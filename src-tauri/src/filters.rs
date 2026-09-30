//! Commandes IPC pour le panneau de filtres global (M6-1, PRD §11, phase 1 :
//! dates + profil Hero + presets — le profil Hero reutilise tel quel le
//! mecanisme `hero_profiles` deja construit (M3-5, D19), donc rien de
//! nouveau ici pour cette dimension. Les 14 autres dimensions du PRD §11
//! (room, buy-in, format, ticket, vitesse, taille de table, position,
//! profondeur, phase, cartes, actions, all-in, tags, resultat de main)
//! restent hors perimetre.
//!
//! Etat persiste par ecran + presets nommes : stockes dans `settings`
//! (`gr-store::Store::get_setting`/`set_setting`, meme mecanisme deja
//! utilise pour `watched_roots`, M3-2). Le contenu JSON n'est pas type cote
//! Rust : ce n'est que de l'etat de confort UI (quel preset de date est
//! actuellement choisi par ecran, liste des presets nommes), jamais relu
//! ni valide par le backend — les types vivent cote TypeScript
//! (`ui/src/lib/filters.ts`), meme principe que le JSON de `watched_roots`
//! dont la forme (`Vec<PathBuf>`) est connue mais pas revalidee ici.

use tauri::State;

use crate::import::ImportState;

fn filter_state_key(screen: &str) -> String {
    format!("filter_state_{screen}")
}

const FILTER_PRESETS_KEY: &str = "filter_presets";

/// Etat de filtre persiste pour `screen` (JSON brut, `None` si jamais
/// enregistre). `screen` est un identifiant libre cote UI (`"home"`,
/// `"results"`, `"tournaments"`, `"hands"`), pas verifie ici.
#[tauri::command]
pub fn get_filter_state(
    screen: String,
    state: State<'_, ImportState>,
) -> Result<Option<String>, String> {
    state
        .store
        .get_setting(&filter_state_key(&screen))
        .map_err(|e| e.to_string())
}

/// Enregistre l'etat de filtre de `screen` (JSON brut, deja serialise cote
/// UI).
#[tauri::command]
pub fn set_filter_state(
    screen: String,
    json: String,
    state: State<'_, ImportState>,
) -> Result<(), String> {
    state
        .store
        .set_setting(&filter_state_key(&screen), &json)
        .map_err(|e| e.to_string())
}

/// Presets de dates nommes (JSON brut, un tableau cote UI), partages entre
/// tous les ecrans (PRD §11 : "les combinaisons de filtres peuvent etre
/// sauvegardees en presets nommes").
#[tauri::command]
pub fn get_filter_presets(state: State<'_, ImportState>) -> Result<Option<String>, String> {
    state
        .store
        .get_setting(FILTER_PRESETS_KEY)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_filter_presets(json: String, state: State<'_, ImportState>) -> Result<(), String> {
    state
        .store
        .set_setting(FILTER_PRESETS_KEY, &json)
        .map_err(|e| e.to_string())
}
