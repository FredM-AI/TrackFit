//! Implementation `DuckDB` de [`AnalyticsBackend`] (PRD §7.3/§13.5, ADR-002,
//! M7-6). Derriere la feature cargo `analytics-duckdb` (crate `duckdb`,
//! feature `bundled` : `DuckDB` vendored, aucun appel reseau a l'execution,
//! R-NET).
//!
//! **Perimetre (decision Frederic, 30/09) : `DuckDB` ne sert que ce trait**
//! (les 4 rapports predefinis de l'ecran Rapports, M7-1) — pas les pivots/
//! KPIs de Resultats/Accueil, qui ont leurs propres requetes SQLite dediees
//! (`gr_analytics::results`/`kpis`/`home`) jamais passees par
//! `AnalyticsBackend`. Etendre `DuckDB` a ces ecrans est note pour la V2
//! (`docs/V2.md`).
//!
//! **Simplification documentee : `f_hand_player` ne contient que les
//! lignes Hero** (`hand_players.is_hero = 1`) — c'est tout ce que
//! [`crate::sqlite::SqliteAnalyticsBackend`] lit aujourd'hui (meme filtre),
//! donc aucune perte fonctionnelle, et ca reduit fortement le volume
//! synchronise (aide a atteindre la cible ×3 de NFR-P6). Meme esprit que
//! les simplifications documentees de `gr_equity::allin`/`gr_store::replay`.
//!
//! **Resolution du profil Hero a la requete, pas au sync** : `f_hand_player`
//! garde `hero_player_id` (jamais `hero_profile_id`), et une petite table
//! `hero_accounts` (copie de `hero_accounts` SQLite, resynchronisee en
//! entier a chaque sync — quelques lignes, cout negligeable) permet de
//! resoudre `hero_player_id -> profile_id` au moment de la requete, exactement
//! comme `SqliteAnalyticsBackend`. Sans ca, un rattachement de pseudo a un
//! profil apres le sync rendrait le cache `DuckDB` silencieusement faux tant
//! qu'aucune main nouvelle ne force un resync.
//!
//! **Fichier reconstructible, jamais sauvegarde** (ADR-002) : pas de
//! systeme de migration comme `gr-store` — le DDL est un simple
//! `CREATE TABLE IF NOT EXISTS` applique a chaque ouverture, et
//! [`rebuild`] repart de zero a la demande (bouton "Reconstruire").

use std::path::Path;

use duckdb::Connection;

use crate::error::AnalyticsError;
use crate::{AnalyticsBackend, ReportRequest, ReportRow, StatCell};

/// Colonnes opportunite/action distinctes utilisees par au moins une
/// [`crate::Measure`] (voir `Measure::columns` dans `lib.rs`) — source
/// unique pour le DDL, le `SELECT` de synchronisation et l'insertion, pour
/// ne pas desynchroniser trois listes ecrites a la main.
const STAT_COLUMNS: [&str; 28] = [
    "vpip_opp",
    "vpip",
    "pfr",
    "rfi_opp",
    "rfi",
    "limp",
    "oshove",
    "tb_opp",
    "tb",
    "f3b_opp",
    "f3b",
    "fb_opp",
    "fb",
    "ats_opp",
    "ats",
    "fsteal_opp",
    "fsteal",
    "rsteal",
    "cbf_opp",
    "cbf",
    "cbt_opp",
    "cbt",
    "fcbf_opp",
    "fcbf",
    "saw_flop",
    "went_sd",
    "won_sd",
    "won_hand",
];

/// Ouvre (ou cree) `analytics.duckdb` et applique le DDL idempotent.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si l'ouverture ou le DDL echoue.
pub fn open(path: &Path) -> Result<Connection, AnalyticsError> {
    let conn = Connection::open(path)?;
    let stat_columns_ddl: String = STAT_COLUMNS
        .iter()
        .map(|c| format!("{c} BIGINT"))
        .collect::<Vec<_>>()
        .join(", ");
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS sync_state (key VARCHAR PRIMARY KEY, value BIGINT NOT NULL);
         CREATE TABLE IF NOT EXISTS hero_accounts (profile_id BIGINT NOT NULL, player_id BIGINT NOT NULL);
         CREATE TABLE IF NOT EXISTS f_hand_player (
             hand_id BIGINT PRIMARY KEY,
             hero_player_id BIGINT NOT NULL,
             position_group VARCHAR,
             eff_depth_bucket VARCHAR,
             {stat_columns_ddl}
         );"
    ))?;
    Ok(conn)
}

/// Curseur de synchronisation persiste dans `analytics.duckdb` lui-meme
/// (le fichier n'est jamais sauvegarde : le reconstruire remet aussi le
/// curseur a zero, sans dependance a `gr-store::settings`).
fn read_cursor(duck: &Connection) -> Result<i64, AnalyticsError> {
    duck.query_row(
        "SELECT value FROM sync_state WHERE key = 'hand_id_cursor'",
        [],
        |row| row.get(0),
    )
    .or_else(|err| match err {
        duckdb::Error::QueryReturnedNoRows => Ok(0),
        other => Err(other),
    })
    .map_err(AnalyticsError::from)
}

fn write_cursor(duck: &Connection, cursor: i64) -> Result<(), AnalyticsError> {
    duck.execute("DELETE FROM sync_state WHERE key = 'hand_id_cursor'", [])?;
    duck.execute(
        "INSERT INTO sync_state (key, value) VALUES ('hand_id_cursor', ?1)",
        duckdb::params![cursor],
    )?;
    Ok(())
}

/// Resynchronise `hero_accounts` en entier (petite table, quelques lignes
/// au plus — pas la peine d'un curseur incremental).
fn resync_hero_accounts(
    duck: &Connection,
    sqlite: &rusqlite::Connection,
) -> Result<(), AnalyticsError> {
    duck.execute("DELETE FROM hero_accounts", [])?;
    let mut stmt = sqlite.prepare("SELECT profile_id, player_id FROM hero_accounts")?;
    let rows: Vec<(i64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut appender = duck.appender("hero_accounts")?;
    for (profile_id, player_id) in rows {
        appender.append_row(duckdb::params![profile_id, player_id])?;
    }
    appender.flush()?;
    Ok(())
}

/// Une ligne source `hand_players`/`hands` a synchroniser :
/// `(hand_id, hero_player_id, position_group, eff_depth_bucket, stat_columns)`.
type SyncRow = (i64, i64, Option<String>, Option<String>, Vec<Option<i64>>);

/// Synchronise `f_hand_player` depuis SQLite (curseur `hands.id`, ADR-002),
/// et resynchronise `hero_accounts` en entier. Renvoie le nombre de lignes
/// synchronisees. Sans effet (renvoie `0`) si tout est deja a jour.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite ou l'ecriture `DuckDB`
/// echoue.
pub fn sync_incremental(
    duck: &Connection,
    sqlite: &rusqlite::Connection,
) -> Result<usize, AnalyticsError> {
    resync_hero_accounts(duck, sqlite)?;

    let cursor = read_cursor(duck)?;
    let select_columns = STAT_COLUMNS
        .iter()
        .map(|c| format!("hp.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT hp.hand_id, h.hero_player_id, hp.position_group, hp.eff_depth_bucket, {select_columns}
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1 AND hp.hand_id > ?1
         ORDER BY hp.hand_id"
    );
    let mut stmt = sqlite.prepare(&sql)?;
    let stat_count = STAT_COLUMNS.len();
    let rows: Vec<SyncRow> = stmt
        .query_map([cursor], |row| {
            let mut stats = Vec::with_capacity(stat_count);
            for i in 0..stat_count {
                stats.push(row.get::<_, Option<i64>>(4 + i)?);
            }
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, stats))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if rows.is_empty() {
        return Ok(0);
    }

    let mut max_hand_id = cursor;
    {
        let mut appender = duck.appender("f_hand_player")?;
        for (hand_id, hero_player_id, position_group, eff_depth_bucket, stats) in &rows {
            let mut params: Vec<&dyn duckdb::types::ToSql> =
                vec![hand_id, hero_player_id, position_group, eff_depth_bucket];
            for stat in stats {
                params.push(stat);
            }
            appender.append_row(params.as_slice())?;
            max_hand_id = max_hand_id.max(*hand_id);
        }
        appender.flush()?;
    }
    write_cursor(duck, max_hand_id)?;

    Ok(rows.len())
}

/// Reconstruit entierement l'index analytique (bouton "Reconstruire",
/// PRD §7.3/ADR-002) : vide `f_hand_player`/le curseur puis resynchronise
/// depuis le debut.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite ou l'ecriture `DuckDB`
/// echoue.
pub fn rebuild(duck: &Connection, sqlite: &rusqlite::Connection) -> Result<usize, AnalyticsError> {
    duck.execute("DELETE FROM f_hand_player", [])?;
    duck.execute("DELETE FROM sync_state", [])?;
    sync_incremental(duck, sqlite)
}

/// Implementation `DuckDB` du trait generique (PRD §13.5, ADR-002). Emprunte
/// une `&Connection` deja ouverte et synchronisee, meme convention que
/// [`crate::sqlite::SqliteAnalyticsBackend`].
pub struct DuckDbAnalyticsBackend<'a> {
    conn: &'a Connection,
}

impl<'a> DuckDbAnalyticsBackend<'a> {
    #[must_use]
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl AnalyticsBackend for DuckDbAnalyticsBackend<'_> {
    fn run_report(&self, request: &ReportRequest) -> Result<Vec<ReportRow>, AnalyticsError> {
        if request.measures.is_empty() {
            return Ok(Vec::new());
        }

        let dim_columns: Vec<&str> = request.dimensions.iter().map(|d| d.column()).collect();
        let dim_count = dim_columns.len();
        let measure_count = request.measures.len();

        let select_dims: Vec<String> = dim_columns.iter().map(|c| format!("fp.{c}")).collect();
        let select_measures: Vec<String> = request
            .measures
            .iter()
            .enumerate()
            .map(|(i, measure)| {
                let (opp, act) = measure.columns();
                format!(
                    "COALESCE(SUM(fp.{opp}), 0) AS opp_{i}, COALESCE(SUM(fp.{act}), 0) AS act_{i}"
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
            "SELECT {select_clause} FROM f_hand_player fp \
             WHERE fp.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)\
             {group_by_clause}"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(duckdb::params![request.hero_profile_id], |row| {
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
