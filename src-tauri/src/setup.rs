//! Commandes IPC de l'assistant de premier lancement (M3-1, UC1) : detection
//! des comptes Winamax locaux, creation du profil Hero, statut "premier
//! lancement termine". L'import initial reutilise la commande `import_paths`
//! existante (M2-3) sur le(s) dossier(s) d'historiques choisis.

use gr_parser_api::Room;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::import::ImportState;

const FIRST_LAUNCH_SETTING_KEY: &str = "first_launch_completed";

/// Un compte Winamax local detecte, envoye au frontend (M3-1).
#[derive(Debug, Clone, Serialize, TS)]
pub struct WinamaxAccountPayload {
    pub pseudo: String,
    pub history_dir: String,
    pub hand_file_count: usize,
}

impl From<gr_ingest::WinamaxAccount> for WinamaxAccountPayload {
    fn from(account: gr_ingest::WinamaxAccount) -> Self {
        Self {
            pseudo: account.pseudo,
            history_dir: account.history_dir.to_string_lossy().into_owned(),
            hand_file_count: account.hand_file_count,
        }
    }
}

/// Detecte les comptes Winamax locaux sous
/// `%APPDATA%\winamax\documents\accounts\` (PRD §8.1). Renvoie une liste
/// vide (pas une erreur) si `%APPDATA%` est absente ou si le dossier
/// Winamax n'existe pas encore : l'assistant degrade proprement vers la
/// selection manuelle plutot que d'afficher une erreur.
#[tauri::command]
pub fn detect_winamax_accounts() -> Vec<WinamaxAccountPayload> {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let accounts_dir = std::path::PathBuf::from(appdata)
        .join("winamax")
        .join("documents")
        .join("accounts");
    gr_ingest::detect_winamax_accounts(&accounts_dir)
        .into_iter()
        .map(WinamaxAccountPayload::from)
        .collect()
}

/// Cree le profil Hero par defaut et y rattache les pseudos choisis
/// (M3-1/M3-5, D19). Renvoie l'id du profil cree.
#[tauri::command]
pub fn create_hero_profile_and_accounts(
    state: State<'_, ImportState>,
    name: String,
    pseudos: Vec<String>,
) -> Result<i64, String> {
    let profile_id = state
        .store
        .create_hero_profile(&name, true)
        .map_err(|e| e.to_string())?;
    for pseudo in &pseudos {
        state
            .store
            .link_hero_account(profile_id, Room::Winamax, pseudo)
            .map_err(|e| e.to_string())?;
    }
    Ok(profile_id)
}

/// `true` si l'assistant de premier lancement a deja ete complete.
#[tauri::command]
pub fn is_first_launch_complete(state: State<'_, ImportState>) -> Result<bool, String> {
    state
        .store
        .get_setting(FIRST_LAUNCH_SETTING_KEY)
        .map(|value| value.as_deref() == Some("true"))
        .map_err(|e| e.to_string())
}

/// Marque l'assistant de premier lancement comme termine (n'est plus
/// reaffiche au demarrage suivant).
#[tauri::command]
pub fn mark_first_launch_complete(state: State<'_, ImportState>) -> Result<(), String> {
    state
        .store
        .set_setting(FIRST_LAUNCH_SETTING_KEY, "true")
        .map_err(|e| e.to_string())
}
