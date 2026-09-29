use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::error::StoreError;

/// Une migration versionnee et nommee (`0001_init`, ...), embarquee dans le
/// binaire au moment de la compilation.
struct Migration {
    version: &'static str,
    sql: &'static str,
}

/// Migrations dans l'ordre d'application. R-SCHEMA : on n'en modifie jamais
/// une deja mergee, on en ajoute seulement de nouvelles a la fin.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "0001_init",
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: "0002_hands_hero_player_index",
        sql: include_str!("../migrations/0002_hands_hero_player_index.sql"),
    },
    Migration {
        version: "0003_hand_players_hero_allin_index",
        sql: include_str!("../migrations/0003_hand_players_hero_allin_index.sql"),
    },
];

/// Cree la table de suivi si besoin, puis applique les migrations manquantes
/// dans l'ordre, chacune dans sa propre transaction. Idempotent : relancer sur
/// une base deja a jour n'insere rien de plus.
pub(crate) fn run_migrations(conn: &mut Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            checksum TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        apply_if_missing(conn, migration)?;
    }
    Ok(())
}

fn apply_if_missing(conn: &mut Connection, migration: &Migration) -> Result<(), StoreError> {
    let checksum = checksum_of(migration.sql);
    let applied: Option<String> = conn
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version = ?1",
            [migration.version],
            |row| row.get(0),
        )
        .ok();

    match applied {
        Some(applied_checksum) if applied_checksum == checksum => Ok(()),
        Some(_) => Err(StoreError::MigrationChecksumMismatch {
            version: migration.version.to_string(),
        }),
        None => {
            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (?1, ?2, ?3)",
                params![migration.version, checksum, now_ms()],
            )?;
            tx.commit()?;
            Ok(())
        }
    }
}

/// Empreinte non cryptographique : sert uniquement a detecter qu'une
/// migration deja mergee a ete modifiee par erreur (R-SCHEMA), pas a
/// authentifier quoi que ce soit.
fn checksum_of(sql: &str) -> String {
    let mut hasher = DefaultHasher::new();
    sql.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_memory() -> Connection {
        Connection::open_in_memory().expect("in-memory sqlite connection")
    }

    #[test]
    fn creates_all_tables_from_a_fresh_database() {
        let mut conn = open_memory();
        run_migrations(&mut conn).expect("migrations should apply");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'hands'",
                [],
                |row| row.get(0),
            )
            .expect("hands table should exist");
        assert_eq!(count, 1);
    }

    #[test]
    fn is_idempotent_when_run_twice() {
        let mut conn = open_memory();
        run_migrations(&mut conn).expect("first run should apply migrations");
        run_migrations(&mut conn).expect("second run should be a no-op");

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("schema_migrations should be readable");
        assert_eq!(applied, i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX));
    }

    #[test]
    fn rejects_a_tampered_already_applied_migration() {
        let mut conn = open_memory();
        run_migrations(&mut conn).expect("first run should apply migrations");

        conn.execute(
            "UPDATE schema_migrations SET checksum = 'tampered' WHERE version = '0001_init'",
            [],
        )
        .expect("test setup: tamper with the recorded checksum");

        let err = run_migrations(&mut conn).expect_err("a tampered checksum must be rejected");
        assert!(matches!(
            err,
            StoreError::MigrationChecksumMismatch { version } if version == "0001_init"
        ));
    }
}
