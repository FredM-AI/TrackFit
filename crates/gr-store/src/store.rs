use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use gr_parser_api::Room;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::StoreError;
use crate::import_log::{self, ImportErrorRow, NewImportError};
use crate::migrate::run_migrations;
use crate::repo::{self, HandInsert, ImportReport};

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

    /// Insere des mains deja parsees (M2-2), par lots transactionnels de
    /// [`repo::BATCH_SIZE`]. Voir [`repo::insert_hands`] pour le detail
    /// (deduplication, tournois provisoires, `hand_raw` compresse).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si une ecriture SQLite ou la compression du
    /// texte brut echoue.
    pub fn insert_hands(
        &self,
        room: Room,
        hands: &[HandInsert<'_>],
    ) -> Result<ImportReport, StoreError> {
        let mut writer = self.writer();
        repo::insert_hands(&mut writer, room, hands)
    }

    /// Enregistre (ou rafraichit) le fichier importe par son chemin (M2-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn upsert_import_file(
        &self,
        path: &str,
        kind: &str,
        size: i64,
        mtime: i64,
        updated_at: i64,
    ) -> Result<i64, StoreError> {
        import_log::upsert_import_file(&self.writer(), path, kind, size, mtime, updated_at)
    }

    /// Consigne une erreur d'import isolee (M2-5, onglet Erreurs d'import).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn record_import_error(&self, error: &NewImportError<'_>) -> Result<i64, StoreError> {
        import_log::record_import_error(&self.writer(), error)
    }

    /// Liste les erreurs d'import, la plus recente d'abord (M2-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn list_import_errors(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<ImportErrorRow>, StoreError> {
        let reader = self.reader()?;
        import_log::list_import_errors(&reader, status)
    }

    /// Recupere une erreur d'import par id (M2-5, action Reparser/Copier).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn get_import_error(&self, id: i64) -> Result<Option<ImportErrorRow>, StoreError> {
        let reader = self.reader()?;
        import_log::get_import_error(&reader, id)
    }

    /// Change le statut d'une erreur d'import (M2-5, action Ignorer/Reparser).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn set_import_error_status(&self, id: i64, status: &str) -> Result<(), StoreError> {
        import_log::set_import_error_status(&self.writer(), id, status)
    }

    /// Met a jour une erreur d'import apres un reparse toujours infructueux
    /// (M2-5) : le statut reste `OPEN`.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn update_import_error_failure(
        &self,
        id: i64,
        code: &str,
        message: &str,
        parser_version: &str,
    ) -> Result<(), StoreError> {
        import_log::update_import_error_failure(&self.writer(), id, code, message, parser_version)
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
