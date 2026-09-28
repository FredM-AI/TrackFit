//! Commandes IPC de l'onglet Erreurs d'import (M2-5) : lister, reparser,
//! ignorer. Aucun ecran ne les consomme encore (ecran Logs reel differe a
//! M6) ; le pipeline et les commandes sont prets et testes cote Rust.

use gr_ingest::{reparse_import_error, ReparseOutcome};
use gr_store::ImportErrorRow;
use serde::Serialize;
use tauri::State;

use crate::import::ImportState;

/// Une ligne de l'onglet Erreurs d'import, telle qu'envoyee au frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ImportErrorPayload {
    id: i64,
    file_path: String,
    line_no: Option<i64>,
    code: String,
    message: String,
    raw_excerpt: Option<String>,
    status: String,
    created_at: i64,
}

impl From<ImportErrorRow> for ImportErrorPayload {
    fn from(row: ImportErrorRow) -> Self {
        Self {
            id: row.id,
            file_path: row.file_path,
            line_no: row.line_no,
            code: row.code,
            message: row.message,
            raw_excerpt: row.raw_excerpt,
            status: row.status,
            created_at: row.created_at,
        }
    }
}

/// Liste les erreurs d'import (`status = None` pour tout, `Some("OPEN")`
/// pour l'onglet par defaut).
#[tauri::command]
pub fn list_import_errors(
    state: State<'_, ImportState>,
    status: Option<String>,
) -> Result<Vec<ImportErrorPayload>, String> {
    state
        .store
        .list_import_errors(status.as_deref())
        .map(|rows| rows.into_iter().map(ImportErrorPayload::from).collect())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum ReparseOutcomePayload {
    Resolved,
    StillFailing { message: String },
}

impl From<ReparseOutcome> for ReparseOutcomePayload {
    fn from(outcome: ReparseOutcome) -> Self {
        match outcome {
            ReparseOutcome::Resolved => Self::Resolved,
            ReparseOutcome::StillFailing { message } => Self::StillFailing { message },
        }
    }
}

/// Action "Reparser" : relit le texte brut conserve et retente le parsing.
#[tauri::command]
pub fn reparse_import_error_cmd(
    state: State<'_, ImportState>,
    error_id: i64,
) -> Result<ReparseOutcomePayload, String> {
    reparse_import_error(&state.store, error_id)
        .map(ReparseOutcomePayload::from)
        .map_err(|e| e.to_string())
}

/// Action "Ignorer" : marque l'erreur comme traitee sans reparser.
#[tauri::command]
pub fn ignore_import_error(state: State<'_, ImportState>, error_id: i64) -> Result<(), String> {
    state
        .store
        .set_import_error_status(error_id, "IGNORED")
        .map_err(|e| e.to_string())
}
