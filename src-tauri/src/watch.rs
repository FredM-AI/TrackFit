//! Watcher temps reel (M3-2) : (re)demarre `gr_ingest::spawn_watcher` sur les
//! dossiers persistes (`settings.watched_roots`, ecrits a la fin de
//! l'assistant de premier lancement, M3-1) et emet `hands://new` quand de
//! nouvelles mains sont inserees.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use gr_ingest::WatcherHandle;
use gr_store::Store;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::import::ImportState;

const WATCHED_ROOTS_SETTING_KEY: &str = "watched_roots";

/// Poignee du watcher courant, geree par Tauri (`app.manage`). `None` tant
/// qu'aucun dossier n'est encore connu (avant la fin de l'assistant).
#[derive(Default)]
pub struct WatcherState(Mutex<Option<WatcherHandle>>);

#[derive(Debug, Clone, Serialize, TS)]
pub struct HandsNewPayload {
    pub hands_inserted: usize,
}

/// Relit `settings.watched_roots` (vide si jamais ecrit) et demarre le
/// watcher dessus. Appelee une fois au demarrage de l'app.
pub fn start_from_persisted_roots(
    app: &AppHandle,
    store: &Arc<Store>,
    watcher_state: &WatcherState,
) {
    let roots = read_watched_roots(store);
    if !roots.is_empty() {
        respawn(app, Arc::clone(store), roots, watcher_state);
    }
}

fn read_watched_roots(store: &Store) -> Vec<PathBuf> {
    let Ok(Some(json)) = store.get_setting(WATCHED_ROOTS_SETTING_KEY) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&json)
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

fn respawn(app: &AppHandle, store: Arc<Store>, roots: Vec<PathBuf>, watcher_state: &WatcherState) {
    let app_handle = app.clone();
    let handle = gr_ingest::spawn_watcher(store, roots, move |tick| {
        if tick.hands_inserted > 0 {
            let _ = app_handle.emit(
                "hands://new",
                HandsNewPayload {
                    hands_inserted: tick.hands_inserted,
                },
            );
        }
    });
    *watcher_state
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(handle);
}

/// Persiste les dossiers a surveiller en temps reel (M3-1, fin de
/// l'assistant : les dossiers choisis pour l'import initial) et (re)demarre
/// le watcher dessus immediatement, sans attendre un redemarrage de l'app.
#[tauri::command]
pub fn set_watched_roots(
    app: AppHandle,
    state: State<'_, ImportState>,
    watcher_state: State<'_, WatcherState>,
    roots: Vec<String>,
) -> Result<(), String> {
    let json = serde_json::to_string(&roots).map_err(|e| e.to_string())?;
    state
        .store
        .set_setting(WATCHED_ROOTS_SETTING_KEY, &json)
        .map_err(|e| e.to_string())?;

    let paths: Vec<PathBuf> = roots.into_iter().map(PathBuf::from).collect();
    if !paths.is_empty() {
        respawn(&app, Arc::clone(&state.store), paths, &watcher_state);
    }
    Ok(())
}
