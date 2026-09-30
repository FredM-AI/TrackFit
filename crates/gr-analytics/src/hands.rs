//! Requetes SQLite pour l'ecran Mains (PRD §13.4, M6-5) : liste virtualisee
//! du profil Hero, paginee cote backend (`LIMIT`/`OFFSET` sur
//! `hands.played_at DESC`, index compose dedie
//! `ix_hands_hero_player_played_at`, migration `0005`) plutot que renvoyee
//! en une fois — a la difference de [`crate::home`]/[`crate::results`]
//! (quelques milliers de tournois, tiennent en memoire), le volume de mains
//! vise 1M+ (NFR-P7 : "premier affichage < 500 ms"), donc le frontend ne
//! charge que les fenetres visibles (react-virtual + pages mises en cache
//! cote `TanStack` Query). Meme convention que [`crate::home`] : une
//! `&Connection` brute, pas de dependance a `gr-store`.

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;

/// `hero_accounts.player_id` du profil (en general un seul pseudo, parfois
/// plusieurs, M3-5). Resolu en Rust plutot que laisse en sous-requete SQL
/// dans les fonctions ci-dessous : avec `hero_player_id IN (SELECT ...)`,
/// SQLite ne peut pas prouver que le resultat reste deja trie par
/// `played_at` (meme avec l'index compose) puisqu'il ne connait pas a
/// l'avance le nombre de valeurs du sous-select, et rajoute un tri complet
/// en `TEMP B-TREE` — mesure a 1M mains : ~2.3s pour la 1ere page contre
/// une cible NFR-P7 de 500ms. Avec un ou plusieurs entiers litteraux lies
/// directement, SQLite reconnait le cas a une seule valeur comme une
/// simple egalite et evite le tri.
fn resolve_hero_player_ids(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<i64>, AnalyticsError> {
    let mut stmt = conn.prepare("SELECT player_id FROM hero_accounts WHERE profile_id = ?1")?;
    let ids = stmt
        .query_map(params![hero_profile_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<i64>>>()?;
    Ok(ids)
}

/// Nombre total de mains du profil Hero (pour dimensionner le virtualizer),
/// filtrable par periode depuis M6-1 (`since_ms`/`until_ms`, `None`/`None`
/// = tout l'historique).
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn count_hero_hands(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<i64, AnalyticsError> {
    let player_ids = resolve_hero_player_ids(conn, hero_profile_id)?;
    if player_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = player_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT COUNT(*) FROM hands
         WHERE hero_player_id IN ({placeholders})
           AND (? IS NULL OR played_at >= ?)
           AND (? IS NULL OR played_at < ?)"
    );
    let params: Vec<Box<dyn rusqlite::ToSql>> = player_ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .chain([
            Box::new(since_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(since_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(until_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(until_ms) as Box<dyn rusqlite::ToSql>,
        ])
        .collect();
    conn.query_row(&sql, rusqlite::params_from_iter(params.iter()), |row| {
        row.get(0)
    })
    .map_err(AnalyticsError::from)
}

/// Une ligne de la liste des mains (PRD §13.4). Colonnes deja calculees a
/// l'import (position, profondeur, ligne preflop, resultat en bb, EV diff) :
/// aucune n'est recalculee ici.
#[derive(Debug, Clone, PartialEq)]
pub struct HandListRow {
    pub hand_id: i64,
    pub played_at: i64,
    pub tournament_name: Option<String>,
    pub level: Option<i64>,
    pub position: Option<String>,
    pub eff_stack_bb: Option<f64>,
    pub hole_cards: Option<String>,
    pub preflop_line: Option<String>,
    pub board: Option<String>,
    pub net_bb: Option<f64>,
    pub allin_ev_diff_bb: Option<f64>,
    /// Cles i18n des tags deja appliques (`tags.label_key`), triees par
    /// libelle. Vide si aucun tag.
    pub tag_label_keys: Vec<String>,
}

/// Page de mains du profil Hero, les plus recentes d'abord (`played_at
/// DESC, hand_id DESC` pour un tri stable). `offset`/`limit` correspondent
/// directement a la fenetre demandee par le virtualizer cote UI.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_hands_page(
    conn: &Connection,
    hero_profile_id: i64,
    limit: i64,
    offset: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<Vec<HandListRow>, AnalyticsError> {
    let player_ids = resolve_hero_player_ids(conn, hero_profile_id)?;
    if player_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Delibrement sans le LEFT JOIN vers hand_tags/tags ici : un GROUP BY
    // sur l'ensemble filtre AVANT la LIMIT forcait SQLite a materialiser et
    // trier les 1M lignes correspondantes avant de ne garder que la page
    // (mesure : 4.6s a 1M mains contre une cible NFR-P7 de 500ms). Les tags
    // de cette seule page sont recuperes par une 2e requete ciblee sur les
    // ids deja limites (`attach_tags`), bien moins couteuse. Voir aussi le
    // commentaire de `resolve_hero_player_ids` : les `player_id` sont lies
    // comme entiers litteraux (pas une sous-requete) pour que SQLite evite
    // un tri complet quand il n'y en a qu'un seul (le cas courant). Le
    // filtre de periode (M6-1) est une condition supplementaire sur la
    // meme colonne indexee `played_at` : verifie par `perf_hands` de ne pas
    // reintroduire le meme probleme de tri.
    let placeholders = player_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT h.id, h.played_at, t.name, h.level,
                hp.position, hp.eff_stack_bb, hp.hole_cards, hp.preflop_line,
                h.board, hp.net_bb, hp.allin_ev_diff_chips, h.bb
         FROM hands h INDEXED BY ix_hands_hero_player_played_at
         JOIN hand_players hp ON hp.hand_id = h.id AND hp.is_hero = 1
         LEFT JOIN tournaments t ON t.id = h.tournament_id
         WHERE h.hero_player_id IN ({placeholders})
           AND (? IS NULL OR h.played_at >= ?)
           AND (? IS NULL OR h.played_at < ?)
         ORDER BY h.played_at DESC, h.id DESC
         LIMIT ? OFFSET ?"
    );
    let mut stmt = conn.prepare(&sql)?;
    let bind_params: Vec<Box<dyn rusqlite::ToSql>> = player_ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .chain([
            Box::new(since_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(since_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(until_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(until_ms) as Box<dyn rusqlite::ToSql>,
            Box::new(limit) as Box<dyn rusqlite::ToSql>,
            Box::new(offset) as Box<dyn rusqlite::ToSql>,
        ])
        .collect();
    let mut rows: Vec<HandListRow> = stmt
        .query_map(rusqlite::params_from_iter(bind_params.iter()), |row| {
            let allin_ev_diff_chips: Option<f64> = row.get(10)?;
            let bb: i64 = row.get(11)?;
            #[allow(clippy::cast_precision_loss)]
            let allin_ev_diff_bb = match (allin_ev_diff_chips, bb) {
                (Some(diff), bb) if bb > 0 => Some(diff / bb as f64),
                _ => None,
            };
            Ok(HandListRow {
                hand_id: row.get(0)?,
                played_at: row.get(1)?,
                tournament_name: row.get(2)?,
                level: row.get(3)?,
                position: row.get(4)?,
                eff_stack_bb: row.get(5)?,
                hole_cards: row.get(6)?,
                preflop_line: row.get(7)?,
                board: row.get(8)?,
                net_bb: row.get(9)?,
                allin_ev_diff_bb,
                tag_label_keys: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)?;

    attach_tags(conn, &mut rows)?;
    Ok(rows)
}

/// Renseigne `tag_label_keys` de chaque ligne de `rows` (deja limitees a une
/// page) via une requete ciblee sur leurs `hand_id`, plutot que de joindre
/// les tags avant la `LIMIT` (voir le commentaire de
/// [`fetch_hero_hands_page`]).
fn attach_tags(conn: &Connection, rows: &mut [HandListRow]) -> Result<(), AnalyticsError> {
    if rows.is_empty() {
        return Ok(());
    }
    let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT ht.hand_id, tg.label_key
         FROM hand_tags ht
         JOIN tags tg ON tg.id = ht.tag_id
         WHERE ht.hand_id IN ({placeholders})
         ORDER BY tg.label_key"
    );
    let mut stmt = conn.prepare(&sql)?;
    let ids: Vec<i64> = rows.iter().map(|r| r.hand_id).collect();
    let tag_rows: Vec<(i64, String)> = stmt
        .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    for row in rows.iter_mut() {
        row.tag_label_keys = tag_rows
            .iter()
            .filter(|(hand_id, _)| *hand_id == row.hand_id)
            .map(|(_, label_key)| label_key.clone())
            .collect();
    }
    Ok(())
}
