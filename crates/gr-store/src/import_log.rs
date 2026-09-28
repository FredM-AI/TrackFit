//! Suivi des fichiers importes et des erreurs de parsing (M2-5, PRD §13.10,
//! ecran Logs / onglet Erreurs d'import).

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::StoreError;

/// Enregistre (ou met a jour) le fichier importe par son chemin, unique
/// (PRD §15 `import_files.path UNIQUE`). Renvoie son id.
pub(crate) fn upsert_import_file(
    conn: &Connection,
    path: &str,
    kind: &str,
    size: i64,
    mtime: i64,
    updated_at: i64,
) -> Result<i64, StoreError> {
    conn.execute(
        "INSERT INTO import_files (path, kind, size, mtime, status, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'OK', ?5)
         ON CONFLICT(path) DO UPDATE SET
            size = excluded.size,
            mtime = excluded.mtime,
            status = excluded.status,
            updated_at = excluded.updated_at",
        params![path, kind, size, mtime, updated_at],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM import_files WHERE path = ?1",
        [path],
        |row| row.get(0),
    )?)
}

/// Etat de suivi d'un fichier de mains deja connu (M3-2, lecture incrementale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportFileProgress {
    pub file_id: i64,
    pub last_offset: i64,
}

/// Lit le progres de lecture connu pour `path` (`None` si jamais vu :
/// l'appelant part alors d'un offset 0).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn get_import_file_progress(
    conn: &Connection,
    path: &str,
) -> Result<Option<ImportFileProgress>, StoreError> {
    conn.query_row(
        "SELECT id, last_offset FROM import_files WHERE path = ?1",
        [path],
        |row| {
            Ok(ImportFileProgress {
                file_id: row.get(0)?,
                last_offset: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Avance `last_offset`/`size` d'un fichier apres une lecture incrementale
/// reussie (M3-2). Remet `status` a `'OK'` (le fichier vient d'etre lu avec
/// succes, meme s'il avait ete marque `MISSING` auparavant).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn update_import_file_progress(
    conn: &Connection,
    file_id: i64,
    size: i64,
    last_offset: i64,
    updated_at: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE import_files SET size = ?1, last_offset = ?2, status = 'OK', updated_at = ?3
         WHERE id = ?4",
        params![size, last_offset, updated_at, file_id],
    )?;
    Ok(())
}

/// Chemins actuellement suivis comme fichiers de mains presents (M3-2, sert
/// a detecter les renommages/suppressions par difference avec le scan disque
/// courant).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn list_tracked_hand_file_paths(conn: &Connection) -> Result<Vec<String>, StoreError> {
    let mut stmt =
        conn.prepare("SELECT path FROM import_files WHERE kind = 'HANDS' AND status = 'OK'")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Marque un fichier de mains comme introuvable (renomme ou supprime,
/// PRD §8.4 "Robustesse") : les mains deja importees restent en base,
/// mais le fichier n'est plus relu tant qu'il ne reapparait pas au meme
/// chemin (`upsert_import_file` repasse alors son statut a `'OK'`).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn mark_import_file_missing(
    conn: &Connection,
    path: &str,
    updated_at: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE import_files SET status = 'MISSING', updated_at = ?1 WHERE path = ?2",
        params![updated_at, path],
    )?;
    Ok(())
}

/// Une nouvelle erreur d'import a consigner. PAR-15 : une defaillance isolee
/// (une main, un fichier) n'interrompt jamais le reste de l'import, elle est
/// juste enregistree ici pour l'onglet Erreurs d'import.
pub struct NewImportError<'a> {
    pub file_id: i64,
    pub file_offset: Option<i64>,
    pub line_no: Option<i64>,
    pub code: &'a str,
    pub message: &'a str,
    pub raw_excerpt: Option<&'a str>,
    pub parser_version: &'a str,
    pub created_at: i64,
}

pub(crate) fn record_import_error(
    conn: &Connection,
    error: &NewImportError<'_>,
) -> Result<i64, StoreError> {
    conn.execute(
        "INSERT INTO import_errors (
            file_id, file_offset, line_no, code, message, raw_excerpt,
            parser_version, status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'OPEN', ?8)",
        params![
            error.file_id,
            error.file_offset,
            error.line_no,
            error.code,
            error.message,
            error.raw_excerpt,
            error.parser_version,
            error.created_at,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Une ligne de l'onglet Erreurs d'import (PRD §13.10), jointe au chemin de
/// son fichier source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportErrorRow {
    pub id: i64,
    pub file_id: i64,
    pub file_path: String,
    pub file_offset: Option<i64>,
    pub line_no: Option<i64>,
    pub code: String,
    pub message: String,
    pub raw_excerpt: Option<String>,
    pub parser_version: String,
    pub status: String,
    pub created_at: i64,
}

const SELECT_IMPORT_ERROR: &str = "SELECT e.id, e.file_id, f.path, e.file_offset, e.line_no, \
     e.code, e.message, e.raw_excerpt, e.parser_version, e.status, e.created_at \
     FROM import_errors e JOIN import_files f ON f.id = e.file_id";

fn row_to_import_error(row: &Row<'_>) -> rusqlite::Result<ImportErrorRow> {
    Ok(ImportErrorRow {
        id: row.get(0)?,
        file_id: row.get(1)?,
        file_path: row.get(2)?,
        file_offset: row.get(3)?,
        line_no: row.get(4)?,
        code: row.get(5)?,
        message: row.get(6)?,
        raw_excerpt: row.get(7)?,
        parser_version: row.get(8)?,
        status: row.get(9)?,
        created_at: row.get(10)?,
    })
}

/// Liste les erreurs d'import, la plus recente d'abord. `status` filtre
/// optionnellement (ex. `Some("OPEN")` pour l'onglet par defaut).
pub(crate) fn list_import_errors(
    conn: &Connection,
    status: Option<&str>,
) -> Result<Vec<ImportErrorRow>, StoreError> {
    let sql = format!(
        "{SELECT_IMPORT_ERROR} WHERE (?1 IS NULL OR e.status = ?1) \
         ORDER BY e.created_at DESC, e.id DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![status], row_to_import_error)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub(crate) fn get_import_error(
    conn: &Connection,
    id: i64,
) -> Result<Option<ImportErrorRow>, StoreError> {
    let sql = format!("{SELECT_IMPORT_ERROR} WHERE e.id = ?1");
    conn.query_row(&sql, [id], row_to_import_error)
        .optional()
        .map_err(StoreError::from)
}

/// Change le statut d'une erreur (ex. `"IGNORED"` pour l'action Ignorer,
/// `"RESOLVED"` quand un reparse reussit).
pub(crate) fn set_import_error_status(
    conn: &Connection,
    id: i64,
    status: &str,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE import_errors SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

/// Met a jour le code/message/version d'une erreur apres une nouvelle
/// tentative de reparse toujours infructueuse (le statut reste `OPEN`).
pub(crate) fn update_import_error_failure(
    conn: &Connection,
    id: i64,
    code: &str,
    message: &str,
    parser_version: &str,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE import_errors SET code = ?1, message = ?2, parser_version = ?3 WHERE id = ?4",
        params![code, message, parser_version, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn sample_error(file_id: i64) -> NewImportError<'static> {
        NewImportError {
            file_id,
            file_offset: Some(120),
            line_no: Some(7),
            code: "UNKNOWN_LINE",
            message: "ligne inconnue",
            raw_excerpt: Some("Winamax Poker - ...corrompu..."),
            parser_version: "0.0.1",
            created_at: 1_000,
        }
    }

    #[test]
    fn upserting_the_same_path_twice_reuses_the_same_file_id() {
        let conn = migrated_connection();
        let first = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        let second = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 200, 20, 2).unwrap();
        assert_eq!(first, second);

        let size: i64 = conn
            .query_row(
                "SELECT size FROM import_files WHERE id = ?1",
                [first],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(size, 200, "the second upsert should refresh the row");
    }

    #[test]
    fn records_and_lists_an_import_error_with_its_file_path() {
        let conn = migrated_connection();
        let file_id = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        let error_id = record_import_error(&conn, &sample_error(file_id)).unwrap();

        let open_errors = list_import_errors(&conn, Some("OPEN")).unwrap();
        assert_eq!(open_errors.len(), 1);
        assert_eq!(open_errors[0].id, error_id);
        assert_eq!(open_errors[0].file_path, "C:/hands/a.txt");
        assert_eq!(open_errors[0].code, "UNKNOWN_LINE");
        assert_eq!(open_errors[0].status, "OPEN");

        let fetched = get_import_error(&conn, error_id).unwrap().unwrap();
        assert_eq!(fetched, open_errors[0]);
    }

    #[test]
    fn get_import_error_returns_none_for_an_unknown_id() {
        let conn = migrated_connection();
        assert_eq!(get_import_error(&conn, 999).unwrap(), None);
    }

    #[test]
    fn list_import_errors_without_a_status_filter_returns_everything() {
        let conn = migrated_connection();
        let file_id = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        let id = record_import_error(&conn, &sample_error(file_id)).unwrap();
        set_import_error_status(&conn, id, "IGNORED").unwrap();

        assert_eq!(list_import_errors(&conn, Some("OPEN")).unwrap().len(), 0);
        assert_eq!(list_import_errors(&conn, None).unwrap().len(), 1);
    }

    #[test]
    fn set_import_error_status_updates_only_the_status() {
        let conn = migrated_connection();
        let file_id = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        let id = record_import_error(&conn, &sample_error(file_id)).unwrap();

        set_import_error_status(&conn, id, "RESOLVED").unwrap();

        let row = get_import_error(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "RESOLVED");
        assert_eq!(row.code, "UNKNOWN_LINE", "other columns must be untouched");
    }

    #[test]
    fn update_import_error_failure_refreshes_code_message_and_version() {
        let conn = migrated_connection();
        let file_id = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        let id = record_import_error(&conn, &sample_error(file_id)).unwrap();

        update_import_error_failure(&conn, id, "CHIP_MISMATCH", "toujours faux", "0.0.2").unwrap();

        let row = get_import_error(&conn, id).unwrap().unwrap();
        assert_eq!(row.code, "CHIP_MISMATCH");
        assert_eq!(row.message, "toujours faux");
        assert_eq!(row.parser_version, "0.0.2");
        assert_eq!(
            row.status, "OPEN",
            "a failed reparse must not change the status"
        );
    }

    #[test]
    fn get_import_file_progress_is_none_for_an_unknown_path() {
        let conn = migrated_connection();
        assert_eq!(
            get_import_file_progress(&conn, "C:/hands/unknown.txt").unwrap(),
            None
        );
    }

    #[test]
    fn update_import_file_progress_advances_offset_and_size() {
        let conn = migrated_connection();
        let file_id = upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 0, 10, 1).unwrap();

        update_import_file_progress(&conn, file_id, 500, 500, 2).unwrap();

        let progress = get_import_file_progress(&conn, "C:/hands/a.txt")
            .unwrap()
            .unwrap();
        assert_eq!(progress.file_id, file_id);
        assert_eq!(progress.last_offset, 500);
    }

    #[test]
    fn marking_a_file_missing_excludes_it_from_tracked_paths_until_it_reappears() {
        let conn = migrated_connection();
        upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 100, 10, 1).unwrap();
        upsert_import_file(&conn, "C:/hands/b.txt", "HANDS", 100, 10, 1).unwrap();

        mark_import_file_missing(&conn, "C:/hands/a.txt", 2).unwrap();
        assert_eq!(
            list_tracked_hand_file_paths(&conn).unwrap(),
            vec!["C:/hands/b.txt".to_string()]
        );

        // The file reappears at the same path (renamed back, or recreated):
        // a fresh upsert must bring it back to 'OK'.
        upsert_import_file(&conn, "C:/hands/a.txt", "HANDS", 120, 30, 3).unwrap();
        let mut tracked = list_tracked_hand_file_paths(&conn).unwrap();
        tracked.sort();
        assert_eq!(
            tracked,
            vec!["C:/hands/a.txt".to_string(), "C:/hands/b.txt".to_string()]
        );
    }
}
