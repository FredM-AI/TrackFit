use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use gr_parser_api::Room;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use gr_core::TournamentSummary;

use crate::error::StoreError;
use crate::hero::{self, HeroProfileRow};
use crate::import_log::{self, ImportErrorRow, ImportFileProgress, NewImportError};
use crate::migrate::run_migrations;
use crate::repo::{self, HandInsert, ImportReport};
use crate::sessions;
use crate::status;
use crate::summary_repo::{self, AttachSummaryReport};

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

    /// Lit le progres de lecture connu d'un fichier de mains (M3-2, lecture
    /// incrementale) ; `None` si le fichier n'a jamais ete vu.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn get_import_file_progress(
        &self,
        path: &str,
    ) -> Result<Option<ImportFileProgress>, StoreError> {
        let reader = self.reader()?;
        import_log::get_import_file_progress(&reader, path)
    }

    /// Avance `last_offset`/`size` d'un fichier apres une lecture
    /// incrementale reussie (M3-2).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn update_import_file_progress(
        &self,
        file_id: i64,
        size: i64,
        last_offset: i64,
        updated_at: i64,
    ) -> Result<(), StoreError> {
        import_log::update_import_file_progress(
            &self.writer(),
            file_id,
            size,
            last_offset,
            updated_at,
        )
    }

    /// Chemins actuellement suivis comme fichiers de mains presents (M3-2).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn list_tracked_hand_file_paths(&self) -> Result<Vec<String>, StoreError> {
        let reader = self.reader()?;
        import_log::list_tracked_hand_file_paths(&reader)
    }

    /// Marque un fichier de mains comme introuvable (renomme/supprime,
    /// M3-2) ; les mains deja importees restent en base.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn mark_import_file_missing(&self, path: &str, updated_at: i64) -> Result<(), StoreError> {
        import_log::mark_import_file_missing(&self.writer(), path, updated_at)
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

    /// Rattache un summary deja parse a son tournoi (M2-6, PRD §8.5) : cree
    /// le tournoi "provisoire" au besoin, met a jour le buy-in exact et le
    /// statut (`COMPLETE`), ecrit `tournament_entries`/`tournament_bullets`.
    /// Fonctionne quel que soit l'ordre d'import par rapport aux mains.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn attach_summary(
        &self,
        room: Room,
        summary: &TournamentSummary,
    ) -> Result<AttachSummaryReport, StoreError> {
        let mut writer = self.writer();
        summary_repo::attach_summary(&mut writer, room, summary)
    }

    /// Marque `INCOMPLETE` tout tournoi encore `PROVISIONAL` dont la
    /// derniere main connue remonte a plus de `stale_after_hours` heures
    /// avant `now_ms` (M2-6, PRD §8.5). Renvoie le nombre de tournois marques.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn mark_stale_provisional_tournaments_incomplete(
        &self,
        now_ms: i64,
        stale_after_hours: i64,
    ) -> Result<usize, StoreError> {
        let writer = self.writer();
        summary_repo::mark_stale_provisional_tournaments_incomplete(
            &writer,
            now_ms,
            stale_after_hours,
        )
    }

    /// Cree un profil Hero (M3-1/M3-5, D19). Si `is_default` est vrai, les
    /// autres profils existants sont retrogrades.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue (ex. nom deja pris).
    pub fn create_hero_profile(&self, name: &str, is_default: bool) -> Result<i64, StoreError> {
        let writer = self.writer();
        hero::create_hero_profile(&writer, name, is_default)
    }

    /// Rattache le pseudo `screen_name` (cree s'il est inconnu) au profil
    /// `profile_id` (M3-1/M3-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn link_hero_account(
        &self,
        profile_id: i64,
        room: Room,
        screen_name: &str,
    ) -> Result<(), StoreError> {
        let writer = self.writer();
        hero::link_hero_account(&writer, profile_id, room, screen_name)
    }

    /// Liste tous les profils Hero, le profil par defaut d'abord.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn list_hero_profiles(&self) -> Result<Vec<HeroProfileRow>, StoreError> {
        let reader = self.reader()?;
        hero::list_hero_profiles(&reader)
    }

    /// Detache un pseudo d'un profil Hero (M3-5). Sans effet si non rattache.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn unlink_hero_account(
        &self,
        profile_id: i64,
        room: Room,
        screen_name: &str,
    ) -> Result<(), StoreError> {
        let writer = self.writer();
        hero::unlink_hero_account(&writer, profile_id, room, screen_name)
    }

    /// Renomme un profil Hero (M3-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn rename_hero_profile(&self, profile_id: i64, new_name: &str) -> Result<(), StoreError> {
        let writer = self.writer();
        hero::rename_hero_profile(&writer, profile_id, new_name)
    }

    /// Pseudos actuellement rattaches a un profil Hero (M3-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn list_hero_account_pseudos(&self, profile_id: i64) -> Result<Vec<String>, StoreError> {
        let reader = self.reader()?;
        hero::list_hero_account_pseudos(&reader, profile_id)
    }

    /// Lit une valeur de `settings` (M3-1 : flag "premier lancement termine").
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let reader = self.reader()?;
        hero::get_setting(&reader, key)
    }

    /// Ecrit (ou remplace) une valeur de `settings`.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn set_setting(&self, key: &str, value_json: &str) -> Result<(), StoreError> {
        let writer = self.writer();
        hero::set_setting(&writer, key, value_json)
    }

    /// Nombre de mains du profil Hero `profile_id` jouees depuis `since_ms`
    /// (epoch ms UTC), pour la barre d'etat "Mains aujourd'hui" (M3-3,
    /// scopee par profil depuis M3-5).
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn count_hero_hands_since(
        &self,
        profile_id: i64,
        since_ms: i64,
    ) -> Result<i64, StoreError> {
        let reader = self.reader()?;
        status::count_hero_hands_since(&reader, profile_id, since_ms)
    }

    /// Horodatage de la derniere main jouee par le profil Hero `profile_id`,
    /// pour la barre d'etat "Derniere main" (M3-3, scopee par profil depuis
    /// M3-5). `None` si aucune main de ce profil n'est encore en base.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si la lecture SQLite echoue.
    pub fn latest_hero_hand_played_at(&self, profile_id: i64) -> Result<Option<i64>, StoreError> {
        let reader = self.reader()?;
        status::latest_hero_hand_played_at(&reader, profile_id)
    }

    /// Regroupe les mains d'Hero en sessions (M3-4, PRD §9.3/H3), avec un
    /// seuil configurable (`settings.session_gap_minutes`, 30 par defaut).
    /// Recalcul integral a chaque appel : correct quelle que soit l'ordre
    /// d'arrivee des mains (import hors ordre, reparse). Renvoie le nombre
    /// de sessions creees ; `0` si aucun profil Hero n'existe encore.
    ///
    /// # Errors
    /// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
    pub fn recompute_hero_sessions(&self) -> Result<usize, StoreError> {
        let mut writer = self.writer();
        sessions::recompute_hero_sessions(&mut writer)
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
