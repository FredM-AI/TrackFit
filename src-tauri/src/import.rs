//! Commandes IPC d'import en masse (M2-3) : `import_paths` lance l'import et
//! emet la progression sur `import://progress`, `cancel_import` l'interrompt
//! proprement. Le pipeline lui-meme vit dans `gr-ingest` (testable sans Tauri).

use std::path::PathBuf;
use std::sync::Arc;

use gr_ingest::{run_import, CancelToken, ImportProgress, ImportSummary};
use gr_store::Store;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

/// Etat partage par les commandes d'import (gere par Tauri, `app.manage`).
pub struct ImportState {
    pub store: Arc<Store>,
    pub cancel: CancelToken,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ImportProgressPayload {
    pub files_total: usize,
    pub files_done: usize,
    pub hands_inserted: usize,
    pub hands_duplicate: usize,
    pub hands_failed: usize,
}

impl From<&ImportProgress> for ImportProgressPayload {
    fn from(progress: &ImportProgress) -> Self {
        Self {
            files_total: progress.files_total,
            files_done: progress.files_done,
            hands_inserted: progress.hands_inserted,
            hands_duplicate: progress.hands_duplicate,
            hands_failed: progress.hands_failed,
        }
    }
}

/// Rapport final renvoye au frontend (PRD §8.4 point 4, M2-6 pour les summaries).
#[derive(Debug, Clone, Serialize, TS)]
pub struct ImportSummaryPayload {
    pub files_scanned: usize,
    pub files_imported: usize,
    pub hands_inserted: usize,
    pub hands_duplicate: usize,
    pub hands_failed: usize,
    pub summaries_attached: usize,
    pub summaries_failed: usize,
    pub cancelled: bool,
}

impl From<ImportSummary> for ImportSummaryPayload {
    fn from(summary: ImportSummary) -> Self {
        Self {
            files_scanned: summary.files_scanned,
            files_imported: summary.files_imported,
            hands_inserted: summary.hands_inserted,
            hands_duplicate: summary.hands_duplicate,
            hands_failed: summary.hands_failed,
            summaries_attached: summary.summaries_attached,
            summaries_failed: summary.summaries_failed,
            cancelled: summary.cancelled,
        }
    }
}

/// Lance un import sur les dossiers/fichiers donnes et emet `import://progress`
/// apres chaque fichier traite.
#[tauri::command]
pub fn import_paths(
    app: AppHandle,
    state: State<'_, ImportState>,
    paths: Vec<PathBuf>,
) -> Result<ImportSummaryPayload, String> {
    state.cancel.reset();
    let summary = run_import(&state.store, &paths, &state.cancel, |progress| {
        let _ = app.emit("import://progress", ImportProgressPayload::from(progress));
    })
    .map_err(|e| e.to_string())?;
    Ok(summary.into())
}

/// Demande l'arret d'un import en cours (termine le fichier entame).
#[tauri::command]
pub fn cancel_import(state: State<'_, ImportState>) {
    state.cancel.cancel();
}
