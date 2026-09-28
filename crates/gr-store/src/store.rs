use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::StoreError;
use crate::migrate::run_migrations;

const DB_FILE_NAME: &str = "graphite.db";

/// Point d'entree du stockage SQLite (ADR-001) : un seul writer proteges par
/// un mutex, un pool de connexions en lecture. Les `PRAGMA` de la PRD §15
/// (`WAL`/`foreign_keys`/`synchronous`) sont appliques a chaque connexion ouverte.
pub struct Store {
    writer: Mutex<Connection>,
    readers: Pool<SqliteConnectionManager>,
}

impl Store {
    /// Ouvre (ou cree) la base dans `data_dir`, applique les `PRAGMA` puis les
    /// migrations en attente (M2-1).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si `data_dir` ne peut pas etre cree, si la
    /// connexion SQLite echoue, ou si une migration deja appliquee a ete
    /// modifiee depuis (R-SCHEMA).
    pub fn open(data_dir: &Path) -> Result<Self, StoreError> {
        std::fs::create_dir_all(data_dir).map_err(|source| StoreError::CreateDataDir {
            path: data_dir.to_path_buf(),
            source,
        })?;
        let db_path = data_dir.join(DB_FILE_NAME);

        let mut writer = Connection::open(&db_path)?;
        apply_pragmas(&writer)?;
        run_migrations(&mut writer)?;

        let manager = SqliteConnectionManager::file(&db_path).with_init(|conn| apply_pragmas(conn));
        let readers = Pool::builder().build(manager)?;

        Ok(Self {
            writer: Mutex::new(writer),
            readers,
        })
    }

    /// Acces exclusif a la connexion d'ecriture (ADR-001 : un seul writer).
    pub fn writer(&self) -> MutexGuard<'_, Connection> {
        // Un mutex empoisonne signale un panic ailleurs, deja interdit par
        // R-NOPANIC : recuperer la garde reste le choix le plus sur ici.
        self.writer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Emprunte une connexion en lecture depuis le pool.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si le pool ne peut fournir aucune connexion
    /// (base verrouillee, pool epuise, connexion invalide).
    pub fn reader(&self) -> Result<PooledConnection<SqliteConnectionManager>, StoreError> {
        Ok(self.readers.get()?)
    }
}

fn apply_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_the_database_file_on_first_launch() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join(DB_FILE_NAME);
        assert!(!db_path.exists());

        let store = Store::open(dir.path()).expect("store should open on first launch");
        assert!(db_path.exists());

        let count: i64 = store
            .writer()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'tournaments'",
                [],
                |row| row.get(0),
            )
            .expect("tournaments table should exist after migration");
        assert_eq!(count, 1);
    }

    #[test]
    fn creates_the_data_dir_when_it_does_not_exist_yet() {
        let dir = tempfile::tempdir().expect("temp dir");
        let nested = dir.path().join("nested").join("data");
        assert!(!nested.exists());

        Store::open(&nested).expect("store should create missing parent directories");
        assert!(nested.join(DB_FILE_NAME).exists());
    }

    #[test]
    fn reopening_an_existing_database_is_a_no_op() {
        let dir = tempfile::tempdir().expect("temp dir");
        Store::open(dir.path()).expect("first open should create and migrate the database");
        Store::open(dir.path()).expect("second open should find an up-to-date schema");
    }

    #[test]
    fn reader_pool_can_read_what_the_writer_committed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store should open");

        store
            .writer()
            .execute(
                "INSERT INTO rooms (id, code, name) VALUES (1, 'winamax', 'Winamax')",
                [],
            )
            .expect("insert via the writer connection");

        let reader = store.reader().expect("a pooled reader connection");
        let name: String = reader
            .query_row("SELECT name FROM rooms WHERE id = 1", [], |row| row.get(0))
            .expect("the reader should see the committed row");
        assert_eq!(name, "Winamax");
    }

    #[test]
    fn foreign_keys_are_enforced_on_every_connection() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store should open");

        let err = store
            .writer()
            .execute(
                "INSERT INTO players (id, room_id, screen_name) VALUES (1, 999, 'Hero')",
                [],
            )
            .expect_err("a missing room_id should violate the foreign key");
        assert!(matches!(
            err,
            rusqlite::Error::SqliteFailure(ref e, _)
                if e.code == rusqlite::ErrorCode::ConstraintViolation
        ));
    }
}
