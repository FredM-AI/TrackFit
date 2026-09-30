//! Requetes SQLite pour l'ecran Accueil (PRD §13.1, M6-2) : resultats de
//! tournois du profil Hero (nourrit `kpis::compute_results_kpis`) et
//! l'ecart EV all-in en bb. Meme convention que [`crate::sqlite`] : une
//! `&Connection` brute, pas de dependance a `gr-store`.

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;
use crate::kpis::{Bullet, TournamentResult};

/// Un resultat de tournoi du profil Hero, avec de quoi l'identifier a
/// l'affichage (PRD §13.1 : "meilleurs et pires tournois") et le pivoter
/// (PRD §13.2, M6-3 phase 2 : vitesse/jour de semaine/heure/mois, et les KPI
/// §9.1 "place moyenne"/"% tables finales" qui ont besoin de `finish_position`
/// et `entrants`).
#[derive(Debug, Clone, PartialEq)]
pub struct TournamentResultRow {
    pub tournament_id: i64,
    pub name: String,
    pub started_at: Option<i64>,
    /// `tournaments.speed` (PRD §9.2, G "vitesse" du pivot complet) : valeur
    /// brute du summary (`turbo`/`semiturbo`/`normal`), `None` si le summary
    /// n'a pas encore ete rattache (tournoi `PROVISIONAL`).
    pub speed: Option<String>,
    /// `tournaments.entrants` (dernier `registered_snapshot` du summary,
    /// M2-6) : nombre d'inscrits, pour le percentile de sortie (G5) et le
    /// KPI "plus gros tournoi" (§9.1).
    pub entrants: Option<i64>,
    /// `tournament_entries.finish_position` (place du dernier bloc, PRD
    /// §8.5) : pour G5 (distribution des places) et les KPI "place
    /// moyenne"/"% tables finales" (§9.1).
    pub finish_position: Option<i64>,
    /// Jour de la semaine UTC de `started_at` (`strftime('%w', ...)` SQLite :
    /// 0 = dimanche .. 6 = samedi), `None` si `started_at` est inconnu.
    /// Calcule cote SQL plutot qu'en Rust pour reutiliser le calendrier
    /// gregorien deja correct de SQLite (mois de longueur variable, annees
    /// bissextiles) sans ajouter de dependance (`chrono`) pour ce seul besoin.
    pub weekday_utc: Option<i64>,
    /// Heure UTC (0-23) de `started_at`, meme provenance que `weekday_utc`.
    pub hour_utc: Option<i64>,
    /// Mois calendaire UTC (1-12) de `started_at`, meme provenance que
    /// `weekday_utc`.
    pub month_utc: Option<i64>,
    pub result: TournamentResult,
}

/// Une ligne brute `tournament_entries` × `tournament_bullets` (colonnes
/// SQL telles quelles, plusieurs booleens de statut sans rapport entre eux
/// — pas un etat a modeliser en enum).
#[allow(clippy::struct_excessive_bools)]
struct TournamentBulletRow {
    tournament_id: i64,
    name: String,
    started_at: Option<i64>,
    speed: Option<String>,
    entrants: Option<i64>,
    finish_position: Option<i64>,
    weekday_utc: Option<i64>,
    hour_utc: Option<i64>,
    month_utc: Option<i64>,
    buyin_prize_cents: i64,
    buyin_bounty_cents: i64,
    buyin_fee_cents: i64,
    is_ko: bool,
    is_freeroll: bool,
    paid_with_ticket: bool,
    used_ticket_value_cents: Option<i64>,
    ticket_won: bool,
    won_ticket_value_cents: Option<i64>,
    entry_no: i64,
    prize_cents: i64,
    bounty_cents: i64,
}

/// Ajoute le bullet de `row` a `result` (PRD §5.1) : ticket utilise
/// attribue au premier bullet, ticket gagne au dernier bullet vu jusqu'ici
/// (les lignes arrivent triees par `entry_no` croissant), faute de
/// granularite par bullet dans `tournament_entries`.
fn push_bullet(result: &mut TournamentResult, row: &TournamentBulletRow) {
    let is_first_bullet = row.entry_no == 1;
    result.bullets.push(Bullet {
        buyin_prize_cents: row.buyin_prize_cents,
        buyin_bounty_cents: row.buyin_bounty_cents,
        buyin_fee_cents: row.buyin_fee_cents,
        paid_with_ticket_face_value_cents: if is_first_bullet && row.paid_with_ticket {
            row.used_ticket_value_cents
        } else {
            None
        },
        prize_won_cents: row.prize_cents,
        bounty_won_cents: row.bounty_cents,
        ticket_won_face_value_cents: None,
    });
    // `e.ticket_won_type_id` est une colonne d'entree (pas de bullet) : elle
    // vaut donc vraie pour CHAQUE ligne de l'entree (jointure repetee), pas
    // seulement la derniere. On efface l'affectation precedente a chaque
    // nouvelle ligne pour que seul le dernier bullet pousse la conserve.
    if row.ticket_won {
        for b in &mut result.bullets {
            b.ticket_won_face_value_cents = None;
        }
        if let Some(last) = result.bullets.last_mut() {
            last.ticket_won_face_value_cents = row.won_ticket_value_cents;
        }
    }
}

/// Resultats de tournois du profil Hero, filtres sur `tournaments.started_at`
/// (`None` = pas de borne). Un bullet (re-entry) par ligne `tournament_bullets`
/// (PRD §5.1) ; le ticket utilise/gagne (rarement renseigne, jamais observe
/// dans le corpus — M5-2) est attribue au premier/dernier bullet, faute de
/// granularite par bullet dans le schema (`tournament_entries` le stocke au
/// niveau de l'entree, pas du bullet).
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_tournament_results(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<Vec<TournamentResultRow>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.started_at, t.buyin_prize_cents, t.buyin_bounty_cents,
                t.buyin_fee_cents, t.ko_type, t.is_freeroll,
                e.paid_with_ticket, ut.face_value_cents,
                e.ticket_won_type_id, wt.face_value_cents,
                b.entry_no, b.prize_cents, b.bounty_cents,
                t.speed, t.entrants, e.finish_position,
                CAST(strftime('%w', t.started_at / 1000, 'unixepoch') AS INTEGER),
                CAST(strftime('%H', t.started_at / 1000, 'unixepoch') AS INTEGER),
                CAST(strftime('%m', t.started_at / 1000, 'unixepoch') AS INTEGER)
         FROM tournament_entries e
         JOIN tournaments t ON t.id = e.tournament_id
         JOIN tournament_bullets b ON b.entry_id = e.id
         LEFT JOIN ticket_types ut ON ut.id = e.ticket_used_type_id
         LEFT JOIN ticket_types wt ON wt.id = e.ticket_won_type_id
         WHERE e.player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
           AND (?2 IS NULL OR t.started_at >= ?2)
           AND (?3 IS NULL OR t.started_at < ?3)
         ORDER BY t.id, b.entry_no",
    )?;

    let rows = stmt.query_map(params![hero_profile_id, since_ms, until_ms], |row| {
        let ko_type: String = row.get(6)?;
        Ok(TournamentBulletRow {
            tournament_id: row.get(0)?,
            name: row.get(1)?,
            started_at: row.get(2)?,
            buyin_prize_cents: row.get(3)?,
            buyin_bounty_cents: row.get(4)?,
            buyin_fee_cents: row.get(5)?,
            is_ko: ko_type != "NONE",
            is_freeroll: row.get(7)?,
            paid_with_ticket: row.get(8)?,
            used_ticket_value_cents: row.get(9)?,
            ticket_won: row.get::<_, Option<i64>>(10)?.is_some(),
            won_ticket_value_cents: row.get(11)?,
            entry_no: row.get(12)?,
            prize_cents: row.get(13)?,
            bounty_cents: row.get(14)?,
            speed: row.get(15)?,
            entrants: row.get(16)?,
            finish_position: row.get(17)?,
            weekday_utc: row.get(18)?,
            hour_utc: row.get(19)?,
            month_utc: row.get(20)?,
        })
    })?;

    let mut results: Vec<TournamentResultRow> = Vec::new();
    for row in rows {
        let row = row?;
        match results.last_mut() {
            Some(current) if current.tournament_id == row.tournament_id => {
                push_bullet(&mut current.result, &row);
            }
            _ => {
                let mut result = TournamentResult {
                    bullets: Vec::new(),
                    is_ko: row.is_ko,
                    is_freeroll: row.is_freeroll,
                };
                push_bullet(&mut result, &row);
                results.push(TournamentResultRow {
                    tournament_id: row.tournament_id,
                    name: row.name.clone(),
                    started_at: row.started_at,
                    speed: row.speed.clone(),
                    entrants: row.entrants,
                    finish_position: row.finish_position,
                    weekday_utc: row.weekday_utc,
                    hour_utc: row.hour_utc,
                    month_utc: row.month_utc,
                    result,
                });
            }
        }
    }

    Ok(results)
}

/// Somme de `hand_players.allin_ev_diff_chips` convertie en bb (divisee par
/// `hands.bb` de chaque main), pour le profil Hero sur `hands.played_at`
/// dans `[since_ms, until_ms)` (`None` = pas de borne). PRD §13.1, carte
/// KPI "Ecart EV all-in (jetons -> bb)".
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn hero_allin_ev_diff_bb(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<f64, AnalyticsError> {
    conn.query_row(
        "SELECT COALESCE(SUM(hp.allin_ev_diff_chips * 1.0 / h.bb), 0.0)
         FROM hand_players hp INDEXED BY ix_hp_hero_allin
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND hp.allin_ev_diff_chips IS NOT NULL
           AND h.bb > 0
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
           AND (?2 IS NULL OR h.played_at >= ?2)
           AND (?3 IS NULL OR h.played_at < ?3)",
        params![hero_profile_id, since_ms, until_ms],
        |row| row.get(0),
    )
    .map_err(AnalyticsError::from)
}
