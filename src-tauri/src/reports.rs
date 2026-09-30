//! Commandes IPC pour l'ecran Rapports (M7-1, PRD §13.5). **Perimetre reduit
//! aux rapports predefinis**, decision validee par Frederic (30/09) :
//! pas de constructeur generique (choix libre de dimensions/mesures,
//! filtres, tri, sauvegarde/duplication/renommage). Des 5 rapports
//! predefinis du PRD, seuls 4 sont livres ici : « Resultats par phase »
//! rejoint M4-5 en V2 (la phase du tournoi n'est jamais calculee).
//!
//! 3 des 4 rapports reutilisent directement `AnalyticsBackend::run_report`
//! (M4-7, deja construit et teste) : aucune nouvelle requete necessaire,
//! juste la bonne combinaison dimension(s)/mesures. Seule « Defense de BB
//! par profondeur » a besoin d'une requete dediee (`gr_analytics::
//! fetch_bb_defense_by_depth`) : elle filtre sur la position brute `BB`,
//! pas le groupe `"Blinds"` (fusion SB+BB), que la `AnalyticsBackend`
//! generique ne sait pas faire (pas de filtres, portee de M4-7).
//!
//! Pas de filtre de periode (M6-1) pour l'instant : ces rapports ne sont
//! pas encore branches sur le panneau de filtres global (`FilterBar` ne
//! reconnait que les 4 ecrans deja branches), differe naturellement avec
//! l'extension du panneau si besoin.
//!
//! `get_starting_hands_grid_report` (M7-7, PRD §13.5) : grille 13×13 des
//! mains de depart, forme differente des 4 rapports ci-dessus (pas de
//! dimensions/mesures choisies), commande dediee.

use gr_analytics::{
    fetch_bb_defense_by_depth, fetch_hand_class_grid, AnalyticsBackend, Dimension, HandClassCell,
    Measure, ReportRequest, ReportRow, SqliteAnalyticsBackend,
};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::import::ImportState;

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReportCellPayload {
    #[ts(type = "number")]
    pub opportunities: i64,
    #[ts(type = "number")]
    pub actions: i64,
    pub percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReportRowPayload {
    pub dimension_values: Vec<Option<String>>,
    pub cells: Vec<ReportCellPayload>,
}

impl From<ReportRow> for ReportRowPayload {
    fn from(row: ReportRow) -> Self {
        Self {
            dimension_values: row.dimension_values,
            cells: row
                .cells
                .into_iter()
                .map(|c| ReportCellPayload {
                    opportunities: c.opportunities,
                    actions: c.actions,
                    percentage: c.percentage(),
                })
                .collect(),
        }
    }
}

fn run(
    state: &State<'_, ImportState>,
    dimensions: Vec<Dimension>,
    measures: Vec<Measure>,
) -> Result<Vec<ReportRowPayload>, String> {
    let Some(hero_profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let backend = SqliteAnalyticsBackend::new(&reader);
    let rows = backend
        .run_report(&ReportRequest {
            hero_profile_id,
            dimensions,
            measures,
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(ReportRowPayload::from).collect())
}

/// « Preflop par position » (PRD §13.5) : VPIP, PFR, RFI, LIMP, OSHOVE,
/// 3-bet, fold-to-3-bet, 4-bet, ATS, par groupe de position (EP/MP/LP/
/// Blinds, PRD §10.4).
#[tauri::command]
pub fn get_preflop_by_position_report(
    state: State<'_, ImportState>,
) -> Result<Vec<ReportRowPayload>, String> {
    run(
        &state,
        vec![Dimension::PositionGroup],
        vec![
            Measure::Vpip,
            Measure::Pfr,
            Measure::Rfi,
            Measure::Limp,
            Measure::Oshove,
            Measure::ThreeBet,
            Measure::FoldToThreeBet,
            Measure::FourBet,
            Measure::Ats,
        ],
    )
}

/// « Open-shove par profondeur × position » (PRD §13.5) : OSHOVE, croise
/// profondeur effective (PRD §10.3, mode par defaut) et groupe de position.
#[tauri::command]
pub fn get_oshove_by_depth_and_position_report(
    state: State<'_, ImportState>,
) -> Result<Vec<ReportRowPayload>, String> {
    run(
        &state,
        vec![Dimension::DepthBucket, Dimension::PositionGroup],
        vec![Measure::Oshove],
    )
}

/// « Postflop (c-bet) » (PRD §13.5) : c-bet flop, c-bet turn, fold au c-bet
/// flop, par groupe de position.
#[tauri::command]
pub fn get_postflop_cbet_report(
    state: State<'_, ImportState>,
) -> Result<Vec<ReportRowPayload>, String> {
    run(
        &state,
        vec![Dimension::PositionGroup],
        vec![Measure::Cbf, Measure::Cbt, Measure::Fcbf],
    )
}

/// « Defense de BB par profondeur » (PRD §12.2 : "Fold BB to steal") :
/// fold-to-steal et re-steal depuis la position brute BB uniquement (pas
/// le groupe "Blinds"), par profondeur effective. Seul des 4 rapports a ne
/// pas passer par `AnalyticsBackend::run_report` (voir la doc de tete de
/// module).
#[tauri::command]
pub fn get_bb_defense_by_depth_report(
    state: State<'_, ImportState>,
) -> Result<Vec<ReportRowPayload>, String> {
    let Some(hero_profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let rows = fetch_bb_defense_by_depth(&reader, hero_profile_id).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(ReportRowPayload::from).collect())
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct HandClassCellPayload {
    pub hand_class: String,
    #[ts(type = "number")]
    pub hands_played: i64,
    pub vpip: ReportCellPayload,
    pub pfr: ReportCellPayload,
    pub avg_net_bb: Option<f64>,
}

impl From<HandClassCell> for HandClassCellPayload {
    fn from(cell: HandClassCell) -> Self {
        Self {
            hand_class: cell.hand_class,
            hands_played: cell.hands_played,
            vpip: ReportCellPayload {
                opportunities: cell.vpip.opportunities,
                actions: cell.vpip.actions,
                percentage: cell.vpip.percentage(),
            },
            pfr: ReportCellPayload {
                opportunities: cell.pfr.opportunities,
                actions: cell.pfr.actions,
                percentage: cell.pfr.percentage(),
            },
            avg_net_bb: cell.avg_net_bb,
        }
    }
}

/// Grille 13×13 des mains de depart (PRD §13.5, M7-7) : frequence, VPIP/PFR
/// et resultat moyen en bb/main par `hand_class`. Pas un rapport comme les
/// 4 autres (forme differente, pas de dimensions/mesures) : commande
/// dediee plutot qu'un passage par `run`.
#[tauri::command]
pub fn get_starting_hands_grid_report(
    state: State<'_, ImportState>,
) -> Result<Vec<HandClassCellPayload>, String> {
    let Some(hero_profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let cells = fetch_hand_class_grid(&reader, hero_profile_id).map_err(|e| e.to_string())?;
    Ok(cells.into_iter().map(HandClassCellPayload::from).collect())
}
