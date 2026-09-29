//! Gestion des profils Hero (M3-5, D19) : creation/renommage, ajout/retrait
//! de pseudo, selection du profil actif (celui utilise par toutes les
//! requetes Hero, `settings.active_hero_profile_id`). Ecran Parametres >
//! Profils Hero.

use gr_parser_api::Room;
use gr_store::Store;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::import::ImportState;

const ACTIVE_HERO_PROFILE_SETTING_KEY: &str = "active_hero_profile_id";

#[derive(Debug, Clone, Serialize, TS)]
pub struct HeroProfilePayload {
    // Voir le commentaire equivalent dans `status.rs` : ts-rs mappe `i64`
    // sur `bigint` par defaut, l'IPC Tauri serialise en JSON (`number` cote
    // JS). Sans risque ici (ids SQLite `INTEGER PRIMARY KEY`, tres loin de
    // `Number.MAX_SAFE_INTEGER`).
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub pseudos: Vec<String>,
}

/// Resout le profil Hero actuellement selectionne : `settings.active_hero_profile_id`
/// si present et valide, sinon le profil par defaut (ou le premier profil
/// existant, `list_hero_profiles` trie deja `is_default DESC, name`). `None`
/// si aucun profil Hero n'existe encore (avant la fin de l'assistant M3-1).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn resolve_active_hero_profile_id(
    store: &Store,
) -> Result<Option<i64>, gr_store::StoreError> {
    let profiles = store.list_hero_profiles()?;
    if profiles.is_empty() {
        return Ok(None);
    }
    if let Some(active_id) = store
        .get_setting(ACTIVE_HERO_PROFILE_SETTING_KEY)?
        .and_then(|v| v.parse::<i64>().ok())
    {
        if profiles.iter().any(|p| p.id == active_id) {
            return Ok(Some(active_id));
        }
    }
    Ok(Some(profiles[0].id))
}

/// Liste tous les profils Hero (M3-5, ecran Parametres), avec leurs pseudos
/// rattaches.
#[tauri::command]
pub fn list_hero_profiles_cmd(
    state: State<'_, ImportState>,
) -> Result<Vec<HeroProfilePayload>, String> {
    let profiles = state
        .store
        .list_hero_profiles()
        .map_err(|e| e.to_string())?;
    profiles
        .into_iter()
        .map(|p| {
            let pseudos = state
                .store
                .list_hero_account_pseudos(p.id)
                .map_err(|e| e.to_string())?;
            Ok(HeroProfilePayload {
                id: p.id,
                name: p.name,
                is_default: p.is_default,
                pseudos,
            })
        })
        .collect()
}

/// Id du profil Hero actuellement selectionne, `None` si aucun profil
/// n'existe encore.
#[tauri::command]
pub fn get_active_hero_profile_id(state: State<'_, ImportState>) -> Result<Option<i64>, String> {
    resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())
}

/// Change le profil Hero actif : toutes les requetes Hero (barre d'etat,
/// sessions...) filtrent desormais par ses `hero_accounts`.
#[tauri::command]
pub fn set_active_hero_profile_id(
    state: State<'_, ImportState>,
    profile_id: i64,
) -> Result<(), String> {
    state
        .store
        .set_setting(ACTIVE_HERO_PROFILE_SETTING_KEY, &profile_id.to_string())
        .map_err(|e| e.to_string())
}

/// Cree un nouveau profil Hero (M3-5, "Creation... de profils") et y
/// rattache les pseudos donnes. N'est jamais cree par defaut (`is_default`
/// reste au premier profil, cree par l'assistant M3-1) ; le selectionner
/// via `set_active_hero_profile_id` si besoin.
#[tauri::command]
pub fn create_hero_profile_cmd(
    state: State<'_, ImportState>,
    name: String,
    pseudos: Vec<String>,
) -> Result<i64, String> {
    let profile_id = state
        .store
        .create_hero_profile(&name, false)
        .map_err(|e| e.to_string())?;
    for pseudo in &pseudos {
        state
            .store
            .link_hero_account(profile_id, Room::Winamax, pseudo)
            .map_err(|e| e.to_string())?;
    }
    Ok(profile_id)
}

/// Renomme un profil Hero (M3-5, "... et modification de profils").
#[tauri::command]
pub fn rename_hero_profile_cmd(
    state: State<'_, ImportState>,
    profile_id: i64,
    new_name: String,
) -> Result<(), String> {
    state
        .store
        .rename_hero_profile(profile_id, &new_name)
        .map_err(|e| e.to_string())
}

/// Rattache un pseudo supplementaire a un profil Hero existant (M3-5).
#[tauri::command]
pub fn add_hero_pseudo_cmd(
    state: State<'_, ImportState>,
    profile_id: i64,
    pseudo: String,
) -> Result<(), String> {
    state
        .store
        .link_hero_account(profile_id, Room::Winamax, &pseudo)
        .map_err(|e| e.to_string())
}

/// Detache un pseudo d'un profil Hero (M3-5).
#[tauri::command]
pub fn remove_hero_pseudo_cmd(
    state: State<'_, ImportState>,
    profile_id: i64,
    pseudo: String,
) -> Result<(), String> {
    state
        .store
        .unlink_hero_account(profile_id, Room::Winamax, &pseudo)
        .map_err(|e| e.to_string())
}
