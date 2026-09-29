//! Requetes SQLite pour l'ecran Resultats (PRD §9.2/§13.2, M6-3, phase 1) :
//! historique de jetons par main (G2, reel vs EV all-in), volume de
//! tournois par jour (G6) et pivot buy-in x KO/non-KO (version reduite du
//! pivot complet §13.2, differe faute de sous-types KO/PKO/Mystery/Space
//! fiabilisables — `docs/formats/winamax.md`, deja documente depuis M2-6).
//! Meme convention que [`crate::home`] : une `&Connection` brute, pas de
//! dependance a `gr-store`.

use std::fmt::Write as _;

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;
use crate::home::TournamentResultRow;
use crate::kpis::{compute_results_kpis, ResultsKpis, TicketValuation};

/// Un point de la courbe G2 (PRD §9.2) : jetons nets de la main (reel) et
/// ajustes a l'EV all-in (`net_bb - allin_ev_diff_bb`, 0 si la main n'a pas
/// d'evenement all-in, M5-4), en bb. Cumul et echantillonnage LTTB restent a
/// la charge de l'appelant (`src-tauri`, memes conventions que G1/M6-2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipHistoryPoint {
    pub played_at: i64,
    pub net_bb: f64,
    pub ev_adjusted_net_bb: f64,
}

/// Historique chronologique des mains du Hero avec `net_bb` connu (main
/// deja retro-remplie, M6-3 ; les mains pas encore traitees par
/// `Store::backfill_net_chips` sont silencieusement absentes, elles
/// reapparaitront au prochain appel une fois le rattrapage termine).
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_chip_history(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<ChipHistoryPoint>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT h.played_at, hp.net_bb, hp.allin_ev_diff_chips, h.bb
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND hp.net_bb IS NOT NULL
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
         ORDER BY h.played_at, h.id",
    )?;
    let rows = stmt.query_map(params![hero_profile_id], |row| {
        let played_at: i64 = row.get(0)?;
        let net_bb: f64 = row.get(1)?;
        let allin_ev_diff_chips: Option<f64> = row.get(2)?;
        let bb: i64 = row.get(3)?;
        #[allow(clippy::cast_precision_loss)]
        let allin_diff_bb = match (allin_ev_diff_chips, bb) {
            (Some(diff), bb) if bb > 0 => diff / bb as f64,
            _ => 0.0,
        };
        Ok(ChipHistoryPoint {
            played_at,
            net_bb,
            ev_adjusted_net_bb: net_bb - allin_diff_bb,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Un jour (minuit UTC epoch ms) et le nombre de tournois du Hero demarres
/// ce jour-la (G6, PRD §9.2, version "par jour" — semaine/mois/heatmap
/// differes en phase 1 de M6-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TournamentVolumePoint {
    pub day_epoch_ms: i64,
    pub tournaments_count: i64,
}

/// Basee sur `hands`/`tournaments` (pas `tournament_entries`, qui n'existe
/// qu'une fois le summary attache, M2-6) : un tournoi compte des qu'au
/// moins une main d'Hero y a ete importee, meme incomplet.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_tournament_volume_by_day(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<TournamentVolumePoint>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT (t.started_at / 86400000) * 86400000 AS day, COUNT(DISTINCT t.id)
         FROM hands h
         JOIN tournaments t ON t.id = h.tournament_id
         WHERE h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
           AND t.started_at IS NOT NULL
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt.query_map(params![hero_profile_id], |row| {
        Ok(TournamentVolumePoint {
            day_epoch_ms: row.get(0)?,
            tournaments_count: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Tranches de buy-in (PRD §9.2, G3) : `(borne_min_cents, borne_max_cents)`,
/// `None` = pas de borne haute ("100+"). Reutilisees ici pour le pivot
/// (phase 1 de M6-3) faute d'un reglage utilisateur (§11 : "tranches
/// modifiables", differe — pas d'ecran de parametres pour ca encore).
pub const BUYIN_BRACKETS_CENTS: [(i64, Option<i64>); 7] = [
    (0, Some(200)),
    (200, Some(500)),
    (500, Some(1000)),
    (1000, Some(2500)),
    (2500, Some(5000)),
    (5000, Some(10000)),
    (10000, None),
];

fn buyin_bracket_index(buyin_cents: i64) -> usize {
    BUYIN_BRACKETS_CENTS
        .iter()
        .position(|&(min, max)| buyin_cents >= min && max.is_none_or(|max| buyin_cents < max))
        .unwrap_or(BUYIN_BRACKETS_CENTS.len() - 1)
}

/// Une ligne du pivot reduit (buy-in x KO/non-KO, phase 1 de M6-3 — le
/// pivot complet §13.2 (format/vitesse/jour/heure/mois) et les KPI §9.1 pas
/// encore calcules (place moyenne, % tables finales, meilleur gain) sont
/// differes, decision validee avec Frederic avant implementation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotRow {
    pub buyin_min_cents: i64,
    pub buyin_max_cents: Option<i64>,
    pub is_ko: bool,
    pub kpis: ResultsKpis,
}

/// Regroupe `results` par tranche de buy-in (du premier bullet de chaque
/// tournoi, constant sur toute une re-entry) et `is_ko`, puis calcule les
/// KPIs §9.1 deja disponibles (`compute_results_kpis`, M5-1) sur chaque
/// groupe. Ne fait aucune requete : pur, sur des donnees deja recuperees via
/// [`crate::fetch_hero_tournament_results`] (memes conventions que G1, M6-2).
#[must_use]
pub fn pivot_by_buyin_and_ko(results: &[TournamentResultRow]) -> Vec<PivotRow> {
    let mut groups: Vec<Vec<crate::TournamentResult>> =
        vec![Vec::new(); BUYIN_BRACKETS_CENTS.len() * 2];

    for row in results {
        let Some(first_bullet) = row.result.bullets.first() else {
            continue;
        };
        let buyin_cents = first_bullet.buyin_prize_cents
            + first_bullet.buyin_bounty_cents
            + first_bullet.buyin_fee_cents;
        let bracket = buyin_bracket_index(buyin_cents);
        let ko_offset = usize::from(row.result.is_ko);
        groups[bracket * 2 + ko_offset].push(row.result.clone());
    }

    groups
        .into_iter()
        .enumerate()
        .filter(|(_, group)| !group.is_empty())
        .map(|(idx, group)| {
            let (buyin_min_cents, buyin_max_cents) = BUYIN_BRACKETS_CENTS[idx / 2];
            let is_ko = idx % 2 == 1;
            PivotRow {
                buyin_min_cents,
                buyin_max_cents,
                is_ko,
                kpis: compute_results_kpis(&group, TicketValuation::FaceValue),
            }
        })
        .collect()
}

/// Export CSV du pivot (PRD §13.2 : "donnees agregees, pas de mains, donc
/// compatible avec D26" — pas de donnee personnelle a anonymiser). Colonnes
/// dans l'ordre des KPI §9.1 deja calcules ; `;` comme separateur (convention
/// FR, coherente avec un tableur ouvert en France).
#[must_use]
pub fn pivot_to_csv(rows: &[PivotRow]) -> String {
    let mut csv = String::from(
        "buyin_min_cents;buyin_max_cents;is_ko;tournaments;cost_cents;fees_cents;profit_cents;roi;itm_rate;abi_cents\n",
    );
    for row in rows {
        let _ = writeln!(
            csv,
            "{};{};{};{};{};{};{};{};{};{}",
            row.buyin_min_cents,
            row.buyin_max_cents.map_or(String::new(), |v| v.to_string()),
            row.is_ko,
            row.kpis.tournaments_count,
            row.kpis.cost_cents,
            row.kpis.fees_cents,
            row.kpis.profit_cents,
            row.kpis.roi.map_or(String::new(), |v| v.to_string()),
            row.kpis.itm_rate.map_or(String::new(), |v| v.to_string()),
            row.kpis.abi_cents.map_or(String::new(), |v| v.to_string()),
        );
    }
    csv
}
