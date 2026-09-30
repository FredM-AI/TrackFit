//! Rapport prédéfini "Défense de BB par profondeur" (M7-1, PRD §13.5).
//! Contrairement aux 3 autres rapports prédéfinis (M4-7's
//! `AnalyticsBackend::run_report`, dimensions `PositionGroup`/`DepthBucket`
//! suffisent), celui-ci a besoin de filtrer sur la position brute `BB`
//! (pas le groupe `"Blinds"` qui fusionne SB et BB, PRD §10.4) avant de
//! grouper par profondeur — la `AnalyticsBackend` generique ne fait pas de
//! filtre (portee de M4-7, "filtres... hors perimetre"), donc une requete
//! dediee ici plutot que d'etendre l'abstraction generique pour un seul
//! rapport.

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;
use crate::{ReportRow, StatCell};

/// "Défense de BB" (PRD §12.2 : "Fold BB to steal") par tranche de
/// profondeur effective, pour le profil Hero. Fold-to-steal et re-steal
/// partagent la même opportunité (`fsteal_opp`, M4-3) : un joueur en BB qui
/// a eu la parole face à une tentative de vol soit se couche (`fsteal`),
/// soit re-relance (`rsteal`), soit suit/checke.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite échoue.
pub fn fetch_bb_defense_by_depth(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<ReportRow>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT hp.eff_depth_bucket,
                COALESCE(SUM(hp.fsteal_opp), 0), COALESCE(SUM(hp.fsteal), 0),
                COALESCE(SUM(hp.fsteal_opp), 0), COALESCE(SUM(hp.rsteal), 0)
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND hp.position = 'BB'
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
         GROUP BY hp.eff_depth_bucket",
    )?;
    let rows = stmt.query_map(params![hero_profile_id], |row| {
        let depth_bucket: Option<String> = row.get(0)?;
        let fsteal_opp: i64 = row.get(1)?;
        let fsteal: i64 = row.get(2)?;
        let rsteal_opp: i64 = row.get(3)?;
        let rsteal: i64 = row.get(4)?;
        Ok(ReportRow {
            dimension_values: vec![depth_bucket],
            cells: vec![
                StatCell {
                    opportunities: fsteal_opp,
                    actions: fsteal,
                },
                StatCell {
                    opportunities: rsteal_opp,
                    actions: rsteal,
                },
            ],
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}
