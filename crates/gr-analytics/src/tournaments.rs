//! Requetes SQLite pour l'ecran Tournois (PRD §13.3, M6-4, phase 1 : liste +
//! detail en lecture seule uniquement — l'edition des metadonnees (via
//! ticket, vitesse, valeur du ticket gagne) est differee a la V2, decision
//! validee avec Frederic le 30/09). L'entete d'un tournoi (buy-in, format,
//! place, gains...) reutilise [`crate::fetch_hero_tournament_results`] filtre
//! sur un seul `tournament_id` ; ce module n'ajoute que ce que cette fonction
//! ne fournit pas deja : mains jouees + ecart EV all-in agreges par tournoi
//! (pour la liste), et le detail main par main d'un tournoi (pour l'ecran de
//! detail). Meme convention que [`crate::home`] : une `&Connection` brute,
//! pas de dependance a `gr-store`.

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;

/// Nombre de mains jouees et ecart EV all-in cumule (bb) par tournoi, pour le
/// profil Hero (PRD §13.3 : colonnes "mains jouees" et "EV diff" de la liste
/// des tournois). Un seul passage sur `hand_players`/`hands` (pas besoin de
/// l'index partiel `ix_hp_hero_allin` ici : `COUNT(*)` doit de toute facon
/// visiter chaque main d'Hero, pas seulement celles avec un all-in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TournamentAggregates {
    pub tournament_id: i64,
    pub hands_played: i64,
    pub ev_diff_bb: f64,
}

/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_tournament_aggregates(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<TournamentAggregates>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT h.tournament_id,
                COUNT(*),
                COALESCE(SUM(CASE
                    WHEN hp.allin_ev_diff_chips IS NOT NULL AND h.bb > 0
                    THEN hp.allin_ev_diff_chips * 1.0 / h.bb
                    ELSE 0.0
                END), 0.0)
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
         GROUP BY h.tournament_id",
    )?;
    let rows = stmt.query_map(params![hero_profile_id], |row| {
        Ok(TournamentAggregates {
            tournament_id: row.get(0)?,
            hands_played: row.get(1)?,
            ev_diff_bb: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Une main d'un tournoi donne, cote Hero (PRD §13.3, detail d'un tournoi :
/// "chronologie du tapis... jetons et bb par main", "all-in", "liste des
/// mains"). `start_stack`/`stack_bb` donnent directement le tapis en debut de
/// main (`hand_players`, deja calcule a l'import) : pas besoin de le
/// reconstruire depuis `net_chips`, contrairement a la courbe G2 (M6-3) qui
/// n'a que le delta par main pour tout l'historique.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TournamentHandRow {
    pub hand_id: i64,
    pub played_at: i64,
    pub level: i64,
    pub start_stack: Option<i64>,
    pub stack_bb: Option<f64>,
    pub net_chips: Option<i64>,
    pub net_bb: Option<f64>,
    /// `hand_players.allin_ev_diff_chips` deja converti en bb (divise par
    /// `hands.bb`, meme convention que `hero_allin_ev_diff_bb`, M6-2) :
    /// `None` si la main n'a pas d'evenement all-in ou si `bb` est nul.
    pub allin_ev_diff_bb: Option<f64>,
}

/// Mains du profil Hero pour un tournoi donne, triees chronologiquement.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_tournament_hero_hands(
    conn: &Connection,
    hero_profile_id: i64,
    tournament_id: i64,
) -> Result<Vec<TournamentHandRow>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT h.id, h.played_at, h.level, hp.start_stack, hp.stack_bb,
                hp.net_chips, hp.net_bb, hp.allin_ev_diff_chips, h.bb
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND h.tournament_id = ?1
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?2)
         ORDER BY h.played_at, h.id",
    )?;
    let rows = stmt.query_map(params![tournament_id, hero_profile_id], |row| {
        let allin_ev_diff_chips: Option<f64> = row.get(7)?;
        let bb: i64 = row.get(8)?;
        #[allow(clippy::cast_precision_loss)]
        let allin_ev_diff_bb = match (allin_ev_diff_chips, bb) {
            (Some(diff), bb) if bb > 0 => Some(diff / bb as f64),
            _ => None,
        };
        Ok(TournamentHandRow {
            hand_id: row.get(0)?,
            played_at: row.get(1)?,
            level: row.get(2)?,
            start_stack: row.get(3)?,
            stack_bb: row.get(4)?,
            net_chips: row.get(5)?,
            net_bb: row.get(6)?,
            allin_ev_diff_bb,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Un adversaire rencontre au moins une fois dans le tournoi (PRD §13.3 :
/// "adversaires rencontres avec leur classification", UC10). Le badge de
/// classification (`players.auto_label`/`manual_label`) est differe apres
/// M7-5 (moteur de classification pas encore construit) — decision deja
/// notee dans `docs/BACKLOG.md` avant cette story, pas un oubli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentOpponentRow {
    pub player_id: i64,
    pub screen_name: String,
    pub hands_together: i64,
}

/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_tournament_opponents(
    conn: &Connection,
    tournament_id: i64,
) -> Result<Vec<TournamentOpponentRow>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.screen_name, COUNT(*)
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         JOIN players p ON p.id = hp.player_id
         WHERE h.tournament_id = ?1
           AND hp.is_hero = 0
         GROUP BY p.id, p.screen_name
         ORDER BY p.screen_name",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(TournamentOpponentRow {
            player_id: row.get(0)?,
            screen_name: row.get(1)?,
            hands_together: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}
