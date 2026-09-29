//! Commande IPC pour la barre d'etat (M3-3, PRD footer) : mains d'Hero
//! aujourd'hui, derniere main jouee, statut du watcher temps reel.

use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::import::ImportState;
use crate::watch::{WatcherRunState, WatcherState};

#[derive(Debug, Clone, Serialize, TS)]
pub struct StatusSnapshotPayload {
    // ts-rs mappe `i64` sur `bigint` par defaut, mais l'IPC Tauri serialise
    // en JSON (un `number` JS cote runtime) : sans cette annotation, le type
    // TS genere mentirait sur la vraie forme de la valeur recue. Sans risque
    // de perte de precision ici (compteurs de mains, epoch ms bien en
    // dessous de Number.MAX_SAFE_INTEGER).
    #[ts(type = "number")]
    pub hands_today: i64,
    #[ts(type = "number | null")]
    pub last_hand_at: Option<i64>,
    pub watcher_run_state: WatcherRunState,
}

/// Instantane pour la barre d'etat, scope au profil Hero actif (M3-5).
/// `since_ms` (minuit local, calcule cote UI en JS) delimite "aujourd'hui" :
/// le backend ne connait que l'UTC (R-MONEY) et ignore le fuseau horaire de
/// l'utilisateur.
#[tauri::command]
pub fn get_status_snapshot(
    state: State<'_, ImportState>,
    watcher_state: State<'_, WatcherState>,
    since_ms: i64,
) -> Result<StatusSnapshotPayload, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        // Aucun profil Hero encore (avant la fin de l'assistant M3-1).
        return Ok(StatusSnapshotPayload {
            hands_today: 0,
            last_hand_at: None,
            watcher_run_state: watcher_state.run_state(),
        });
    };
    let hands_today = state
        .store
        .count_hero_hands_since(profile_id, since_ms)
        .map_err(|e| e.to_string())?;
    let last_hand_at = state
        .store
        .latest_hero_hand_played_at(profile_id)
        .map_err(|e| e.to_string())?;
    Ok(StatusSnapshotPayload {
        hands_today,
        last_hand_at,
        watcher_run_state: watcher_state.run_state(),
    })
}
