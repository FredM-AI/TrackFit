//! Implementation SQLite de [`AnalyticsBackend`] (PRD §13.5/ADR-002, M4-7).
//! Agrege directement `hand_players`/`hands` (pas de compteurs
//! incrementaux : `player_stat_counters` est differe en V2, M4-6) — un
//! `SUM`/`GROUP BY` par requete, acceptable pour la cible SQLite de
//! NFR-P6 (< 8 s, 2 dimensions × 10 stats, 2 M mains).

use rusqlite::Connection;

use crate::error::AnalyticsError;
use crate::{AnalyticsBackend, ReportRequest, ReportRow, StatCell};

/// Emprunte une `&Connection` (pas de dependance a `gr-store` : ce crate
/// reste utilisable avec n'importe quelle connexion SQLite migree, y
/// compris en test, cf. `gr-store::Store::reader`).
pub struct SqliteAnalyticsBackend<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAnalyticsBackend<'a> {
    #[must_use]
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl AnalyticsBackend for SqliteAnalyticsBackend<'_> {
    fn run_report(&self, request: &ReportRequest) -> Result<Vec<ReportRow>, AnalyticsError> {
        if request.measures.is_empty() {
            return Ok(Vec::new());
        }

        let dim_columns: Vec<&str> = request.dimensions.iter().map(|d| d.column()).collect();
        let dim_count = dim_columns.len();
        let measure_count = request.measures.len();

        let select_dims: Vec<String> = dim_columns.iter().map(|c| format!("hp.{c}")).collect();
        let select_measures: Vec<String> = request
            .measures
            .iter()
            .enumerate()
            .map(|(i, measure)| {
                let (opp, act) = measure.columns();
                // `COALESCE(..., 0)` : `SUM` d'une colonne entierement
                // `NULL` (aucune ligne, ou stat pas encore calculee pour ce
                // groupe) renvoie `NULL` en SQL, pas `0`.
                format!(
                    "COALESCE(SUM(hp.{opp}), 0) AS opp_{i}, COALESCE(SUM(hp.{act}), 0) AS act_{i}"
                )
            })
            .collect();
        let select_clause = select_dims
            .iter()
            .cloned()
            .chain(select_measures)
            .collect::<Vec<_>>()
            .join(", ");
        let group_by_clause = if dim_columns.is_empty() {
            String::new()
        } else {
            format!(" GROUP BY {}", select_dims.join(", "))
        };

        let sql = format!(
            "SELECT {select_clause} FROM hand_players hp \
             JOIN hands h ON h.id = hp.hand_id \
             WHERE hp.is_hero = 1 \
               AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)\
             {group_by_clause}"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([request.hero_profile_id], |row| {
            let mut dimension_values = Vec::with_capacity(dim_count);
            for i in 0..dim_count {
                dimension_values.push(row.get::<_, Option<String>>(i)?);
            }
            let mut cells = Vec::with_capacity(measure_count);
            for i in 0..measure_count {
                let opportunities: i64 = row.get(dim_count + i * 2)?;
                let actions: i64 = row.get(dim_count + i * 2 + 1)?;
                cells.push(StatCell {
                    opportunities,
                    actions,
                });
            }
            Ok(ReportRow {
                dimension_values,
                cells,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AnalyticsError::from)
    }
}
