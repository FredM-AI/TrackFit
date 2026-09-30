//! Commandes IPC pour l'ecran Mains (M6-5, PRD §13.4). Liste paginee (pas de
//! snapshot "tout d'un coup" comme M6-2/M6-3/M6-4 : le volume vise 1M+
//! mains, NFR-P7 impose un premier affichage < 500 ms, donc le frontend ne
//! demande que les pages visibles). "Tag en masse" reutilise les 9 tags
//! predefinis semes par la migration `0004_predefined_tags.sql` : creer un
//! tag libre reste le perimetre de M7-4. Double-clic -> Replayer est gere
//! cote frontend (navigation vers le placeholder `/replayer`, M7-3 pas
//! encore construit) : pas de commande dediee necessaire ici.

use gr_analytics::{count_hero_hands, fetch_hero_hands_page};
use gr_store::TagRow;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::import::ImportState;

#[derive(Debug, Clone, Serialize, TS)]
pub struct HandListRowPayload {
    #[ts(type = "number")]
    pub hand_id: i64,
    #[ts(type = "number")]
    pub played_at: i64,
    pub tournament_name: Option<String>,
    #[ts(type = "number | null")]
    pub level: Option<i64>,
    pub position: Option<String>,
    pub eff_stack_bb: Option<f64>,
    pub hole_cards: Option<String>,
    pub preflop_line: Option<String>,
    pub board: Option<String>,
    pub net_bb: Option<f64>,
    pub allin_ev_diff_bb: Option<f64>,
    pub tag_label_keys: Vec<String>,
}

/// Nombre total de mains du profil Hero actif (pour dimensionner le
/// virtualizer). Commande separee de [`get_hands_page`] : `COUNT(*)` coute
/// ~85 ms a 1M mains (mesure `perf_hands`), pas la peine de le repayer a
/// chaque page demandee pendant le defilement — le frontend l'appelle une
/// seule fois au montage de l'ecran.
#[tauri::command]
pub fn get_hands_count(state: State<'_, ImportState>) -> Result<i64, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(0);
    };
    let reader = state.store.reader().map_err(|e| e.to_string())?;
    count_hero_hands(&reader, profile_id).map_err(|e| e.to_string())
}

/// Page de la liste des mains (PRD §13.4), la plus recente d'abord, scope
/// au profil Hero actif.
#[tauri::command]
pub fn get_hands_page(
    limit: i64,
    offset: i64,
    state: State<'_, ImportState>,
) -> Result<Vec<HandListRowPayload>, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };

    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let rows = fetch_hero_hands_page(&reader, profile_id, limit, offset)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| HandListRowPayload {
            hand_id: r.hand_id,
            played_at: r.played_at,
            tournament_name: r.tournament_name,
            level: r.level,
            position: r.position,
            eff_stack_bb: r.eff_stack_bb,
            hole_cards: r.hole_cards,
            preflop_line: r.preflop_line,
            board: r.board,
            net_bb: r.net_bb,
            allin_ev_diff_bb: r.allin_ev_diff_bb,
            tag_label_keys: r.tag_label_keys,
        })
        .collect();

    Ok(rows)
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct TagPayload {
    #[ts(type = "number")]
    pub id: i64,
    pub label_key: String,
    pub is_predefined: bool,
}

/// Tags connus (M6-5 n'en affiche que les 9 predefinis, mais la commande
/// reste generique pour M7-4).
#[tauri::command]
pub fn list_tags(state: State<'_, ImportState>) -> Result<Vec<TagPayload>, String> {
    let tags: Vec<TagRow> = state.store.list_tags().map_err(|e| e.to_string())?;
    Ok(tags
        .into_iter()
        .map(|t| TagPayload {
            id: t.id,
            label_key: t.label_key,
            is_predefined: t.is_predefined,
        })
        .collect())
}

/// Applique `tag_id` a chaque main de `hand_ids` (PRD §13.4 : "selection
/// multiple -> tag en masse").
#[tauri::command]
pub fn tag_hands(
    hand_ids: Vec<i64>,
    tag_id: i64,
    now_ms: i64,
    state: State<'_, ImportState>,
) -> Result<(), String> {
    state
        .store
        .tag_hands(&hand_ids, tag_id, now_ms)
        .map_err(|e| e.to_string())
}
