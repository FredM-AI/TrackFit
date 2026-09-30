//! Commande IPC pour l'ecran Resultats (M6-3, PRD §9.2/§13.2). G1 (repris de
//! M6-2), G2 (reel vs EV all-in en bb, §10.6), G3 (ROI par buy-in), G6
//! (volume par jour), G5 (distribution des places, sans la mise en evidence
//! de "la bulle" — bloquee, voir `gr_analytics::results`), le pivot complet
//! (buy-in x KO/non-KO + vitesse + jour de semaine + heure + mois) et les
//! KPI §9.1 restants (place moyenne, % tables finales, meilleur gain, plus
//! gros tournoi). G4 (ROI par format complet KO/PKO/Mystery/Space) reste
//! bloque en permanence (ambiguite du summary Winamax, deja documentee
//! depuis M2-6).

use gr_analytics::{
    compute_additional_kpis, day_of_week_pivot_to_csv, fetch_hero_chip_history,
    fetch_hero_tournament_results, fetch_hero_tournament_volume_by_day,
    finish_percentile_distribution as compute_finish_percentile_distribution, hour_pivot_to_csv,
    lttb, month_pivot_to_csv, pivot_by_buyin_and_ko, pivot_by_day_of_week, pivot_by_hour,
    pivot_by_month, pivot_by_speed, pivot_to_csv, roi_by_buyin, roi_by_buyin_to_csv,
    speed_pivot_to_csv, ResultsKpis,
};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::home::{profit_curve, ProfitCurvePoint};
use crate::import::ImportState;

/// CA de M6-3 : "echantillonnage LTTB au-dela de 5000 points".
const LTTB_THRESHOLD: usize = 5000;

#[derive(Debug, Clone, Serialize, TS)]
pub struct ChipCurvePoint {
    #[ts(type = "number")]
    pub played_at: i64,
    pub cumulative_net_bb: f64,
    pub cumulative_ev_adjusted_net_bb: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct VolumePointPayload {
    #[ts(type = "number")]
    pub day_epoch_ms: i64,
    #[ts(type = "number")]
    pub tournaments_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct PivotRowPayload {
    #[ts(type = "number")]
    pub buyin_min_cents: i64,
    #[ts(type = "number | null")]
    pub buyin_max_cents: Option<i64>,
    pub is_ko: bool,
    #[ts(type = "number")]
    pub tournaments_count: u32,
    #[ts(type = "number")]
    pub cost_cents: i64,
    #[ts(type = "number")]
    pub fees_cents: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    pub roi: Option<f64>,
    pub itm_rate: Option<f64>,
    pub abi_cents: Option<f64>,
}

/// Les 7 colonnes KPI §9.1 deja calculees, partagees par toutes les lignes
/// de pivot phase 2 (evite de les repeter dans 5 structs quasi identiques).
#[derive(Debug, Clone, Serialize, TS)]
pub struct PivotKpisPayload {
    #[ts(type = "number")]
    pub tournaments_count: u32,
    #[ts(type = "number")]
    pub cost_cents: i64,
    #[ts(type = "number")]
    pub fees_cents: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    pub roi: Option<f64>,
    pub itm_rate: Option<f64>,
    pub abi_cents: Option<f64>,
}

impl From<ResultsKpis> for PivotKpisPayload {
    fn from(kpis: ResultsKpis) -> Self {
        Self {
            tournaments_count: kpis.tournaments_count,
            cost_cents: kpis.cost_cents,
            fees_cents: kpis.fees_cents,
            profit_cents: kpis.profit_cents,
            roi: kpis.roi,
            itm_rate: kpis.itm_rate,
            abi_cents: kpis.abi_cents,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct BuyinRoiRowPayload {
    #[ts(type = "number")]
    pub buyin_min_cents: i64,
    #[ts(type = "number | null")]
    pub buyin_max_cents: Option<i64>,
    pub kpis: PivotKpisPayload,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct FinishPercentileBucketPayload {
    #[ts(type = "number")]
    pub floor_percent: u32,
    #[ts(type = "number")]
    pub tournaments_count: u32,
}

/// KPI §9.1 restants (place moyenne, % tables finales, meilleur gain, plus
/// gros tournoi), combles en phase 2 de M6-3 (voir
/// `gr_analytics::compute_additional_kpis`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct AdditionalKpisPayload {
    pub avg_finish_position: Option<f64>,
    pub final_table_rate: Option<f64>,
    #[ts(type = "number")]
    pub best_win_cents: i64,
    #[ts(type = "number | null")]
    pub biggest_tournament_entrants: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SpeedPivotRowPayload {
    pub speed: Option<String>,
    pub kpis: PivotKpisPayload,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DayOfWeekPivotRowPayload {
    #[ts(type = "number")]
    pub weekday: i64,
    pub kpis: PivotKpisPayload,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct HourPivotRowPayload {
    #[ts(type = "number")]
    pub hour: i64,
    pub kpis: PivotKpisPayload,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct MonthPivotRowPayload {
    #[ts(type = "number")]
    pub month: i64,
    pub kpis: PivotKpisPayload,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ResultsSnapshotPayload {
    /// G1, repris de M6-2 (tout l'historique, non filtre par periode).
    pub profit_curve: Vec<ProfitCurvePoint>,
    /// G2 : reel vs ajuste EV all-in, cumule, en bb.
    pub chip_curve: Vec<ChipCurvePoint>,
    /// G6, version "par jour" (semaine/mois/heatmap differes).
    pub volume: Vec<VolumePointPayload>,
    /// Pivot buy-in x KO/non-KO.
    pub pivot: Vec<PivotRowPayload>,
    /// Export CSV du pivot ci-dessus, deja pret (le frontend n'a qu'a le
    /// proposer en telechargement, pas de logique de formatage cote UI).
    pub pivot_csv: String,
    /// G3 : ROI par tranche de buy-in, tous formats confondus.
    pub roi_by_buyin: Vec<BuyinRoiRowPayload>,
    pub roi_by_buyin_csv: String,
    /// G5 : distribution des places de sortie en percentile des inscrits.
    /// Pas de mise en evidence de "la bulle" (PRD §9.2) : bloquee,
    /// `tournaments.paid_places` n'est jamais renseigne (absent du format de
    /// summary Winamax, voir `gr_analytics::results`).
    pub finish_percentile_distribution: Vec<FinishPercentileBucketPayload>,
    pub additional_kpis: AdditionalKpisPayload,
    /// Pivot complet (§13.2) : dimensions supplementaires au pivot buy-in x
    /// KO ci-dessus.
    pub pivot_speed: Vec<SpeedPivotRowPayload>,
    pub pivot_speed_csv: String,
    pub pivot_day_of_week: Vec<DayOfWeekPivotRowPayload>,
    pub pivot_day_of_week_csv: String,
    pub pivot_hour: Vec<HourPivotRowPayload>,
    pub pivot_hour_csv: String,
    pub pivot_month: Vec<MonthPivotRowPayload>,
    pub pivot_month_csv: String,
}

/// Downsample `points` par LTTB en gardant les deux series alignees sur les
/// memes abscisses : LTTB tourne sur des indices (pas sur `played_at`
/// directement) pour choisir quelles lignes garder, puis les deux series
/// (reelle et ajustee EV) sont relues a ces memes indices — sinon LTTB,
/// execute separement sur chaque serie, retiendrait des points differents
/// et casserait la superposition (PRD §9.2 : "cumul... contre le cumul
/// ajuste", un seul graphe).
fn downsample_chip_curve(rows: &[(i64, f64, f64)]) -> Vec<ChipCurvePoint> {
    if rows.len() <= LTTB_THRESHOLD {
        return rows
            .iter()
            .map(|&(played_at, real, ev)| ChipCurvePoint {
                played_at,
                cumulative_net_bb: real,
                cumulative_ev_adjusted_net_bb: ev,
            })
            .collect();
    }

    #[allow(clippy::cast_precision_loss)]
    let indexed: Vec<(f64, f64)> = rows
        .iter()
        .enumerate()
        .map(|(i, &(_, real, _))| (i as f64, real))
        .collect();
    lttb(&indexed, LTTB_THRESHOLD)
        .into_iter()
        .map(|(x, _)| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = x as usize;
            let (played_at, real, ev) = rows[i];
            ChipCurvePoint {
                played_at,
                cumulative_net_bb: real,
                cumulative_ev_adjusted_net_bb: ev,
            }
        })
        .collect()
}

/// Meme principe que [`downsample_chip_curve`] pour G1 (deux series : avec
/// et sans bounties).
fn downsample_profit_curve(points: Vec<ProfitCurvePoint>) -> Vec<ProfitCurvePoint> {
    if points.len() <= LTTB_THRESHOLD {
        return points;
    }

    #[allow(clippy::cast_precision_loss)]
    let indexed: Vec<(f64, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (i as f64, p.cumulative_profit_cents as f64))
        .collect();
    lttb(&indexed, LTTB_THRESHOLD)
        .into_iter()
        .map(|(x, _)| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = x as usize;
            points[i].clone()
        })
        .collect()
}

/// Instantane de l'ecran Resultats (M6-3), scope au profil Hero actif.
#[tauri::command]
pub fn get_results_snapshot(
    state: State<'_, ImportState>,
) -> Result<ResultsSnapshotPayload, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(ResultsSnapshotPayload {
            profit_curve: Vec::new(),
            chip_curve: Vec::new(),
            volume: Vec::new(),
            pivot: Vec::new(),
            pivot_csv: String::new(),
            roi_by_buyin: Vec::new(),
            roi_by_buyin_csv: String::new(),
            finish_percentile_distribution: Vec::new(),
            additional_kpis: AdditionalKpisPayload {
                avg_finish_position: None,
                final_table_rate: None,
                best_win_cents: 0,
                biggest_tournament_entrants: None,
            },
            pivot_speed: Vec::new(),
            pivot_speed_csv: String::new(),
            pivot_day_of_week: Vec::new(),
            pivot_day_of_week_csv: String::new(),
            pivot_hour: Vec::new(),
            pivot_hour_csv: String::new(),
            pivot_month: Vec::new(),
            pivot_month_csv: String::new(),
        });
    };

    let reader = state.store.reader().map_err(|e| e.to_string())?;

    let all_results = fetch_hero_tournament_results(&reader, profile_id, None, None)
        .map_err(|e| e.to_string())?;
    let profit_curve = downsample_profit_curve(profit_curve(&all_results));

    let history = fetch_hero_chip_history(&reader, profile_id).map_err(|e| e.to_string())?;
    let mut cumulative_real = 0.0;
    let mut cumulative_ev = 0.0;
    let chip_rows: Vec<(i64, f64, f64)> = history
        .into_iter()
        .map(|point| {
            cumulative_real += point.net_bb;
            cumulative_ev += point.ev_adjusted_net_bb;
            (point.played_at, cumulative_real, cumulative_ev)
        })
        .collect();
    let chip_curve = downsample_chip_curve(&chip_rows);

    let volume = fetch_hero_tournament_volume_by_day(&reader, profile_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|v| VolumePointPayload {
            day_epoch_ms: v.day_epoch_ms,
            tournaments_count: v.tournaments_count,
        })
        .collect();

    let pivot_rows = pivot_by_buyin_and_ko(&all_results);
    let pivot_csv = pivot_to_csv(&pivot_rows);
    let pivot = pivot_rows
        .into_iter()
        .map(|row| PivotRowPayload {
            buyin_min_cents: row.buyin_min_cents,
            buyin_max_cents: row.buyin_max_cents,
            is_ko: row.is_ko,
            tournaments_count: row.kpis.tournaments_count,
            cost_cents: row.kpis.cost_cents,
            fees_cents: row.kpis.fees_cents,
            profit_cents: row.kpis.profit_cents,
            roi: row.kpis.roi,
            itm_rate: row.kpis.itm_rate,
            abi_cents: row.kpis.abi_cents,
        })
        .collect();

    let g3_rows = roi_by_buyin(&all_results);
    let roi_by_buyin_csv = roi_by_buyin_to_csv(&g3_rows);
    let roi_by_buyin_payload = g3_rows
        .into_iter()
        .map(|row| BuyinRoiRowPayload {
            buyin_min_cents: row.buyin_min_cents,
            buyin_max_cents: row.buyin_max_cents,
            kpis: row.kpis.into(),
        })
        .collect();

    let finish_percentile_distribution = compute_finish_percentile_distribution(&all_results)
        .into_iter()
        .map(|b| FinishPercentileBucketPayload {
            floor_percent: b.floor_percent,
            tournaments_count: b.tournaments_count,
        })
        .collect();

    let additional = compute_additional_kpis(&all_results);
    let additional_kpis = AdditionalKpisPayload {
        avg_finish_position: additional.avg_finish_position,
        final_table_rate: additional.final_table_rate,
        best_win_cents: additional.best_win_cents,
        biggest_tournament_entrants: additional.biggest_tournament_entrants,
    };

    let speed_rows = pivot_by_speed(&all_results);
    let pivot_speed_csv = speed_pivot_to_csv(&speed_rows);
    let pivot_speed = speed_rows
        .into_iter()
        .map(|row| SpeedPivotRowPayload {
            speed: row.speed,
            kpis: row.kpis.into(),
        })
        .collect();

    let day_rows = pivot_by_day_of_week(&all_results);
    let pivot_day_of_week_csv = day_of_week_pivot_to_csv(&day_rows);
    let pivot_day_of_week = day_rows
        .into_iter()
        .map(|row| DayOfWeekPivotRowPayload {
            weekday: row.weekday,
            kpis: row.kpis.into(),
        })
        .collect();

    let hour_rows = pivot_by_hour(&all_results);
    let pivot_hour_csv = hour_pivot_to_csv(&hour_rows);
    let pivot_hour = hour_rows
        .into_iter()
        .map(|row| HourPivotRowPayload {
            hour: row.hour,
            kpis: row.kpis.into(),
        })
        .collect();

    let month_rows = pivot_by_month(&all_results);
    let pivot_month_csv = month_pivot_to_csv(&month_rows);
    let pivot_month = month_rows
        .into_iter()
        .map(|row| MonthPivotRowPayload {
            month: row.month,
            kpis: row.kpis.into(),
        })
        .collect();

    Ok(ResultsSnapshotPayload {
        profit_curve,
        chip_curve,
        volume,
        pivot,
        pivot_csv,
        roi_by_buyin: roi_by_buyin_payload,
        roi_by_buyin_csv,
        finish_percentile_distribution,
        additional_kpis,
        pivot_speed,
        pivot_speed_csv,
        pivot_day_of_week,
        pivot_day_of_week_csv,
        pivot_hour,
        pivot_hour_csv,
        pivot_month,
        pivot_month_csv,
    })
}

#[cfg(test)]
mod tests {
    use super::{downsample_chip_curve, downsample_profit_curve, LTTB_THRESHOLD};
    use crate::home::ProfitCurvePoint;

    #[test]
    fn downsample_chip_curve_keeps_everything_under_the_threshold() {
        #[allow(clippy::cast_precision_loss)]
        let rows: Vec<(i64, f64, f64)> =
            (0..10i64).map(|i| (i, i as f64, i as f64 / 2.0)).collect();
        let curve = downsample_chip_curve(&rows);
        assert_eq!(curve.len(), 10);
        assert_eq!(curve[3].played_at, 3);
        assert!((curve[3].cumulative_net_bb - 3.0).abs() < 1e-9);
        assert!((curve[3].cumulative_ev_adjusted_net_bb - 1.5).abs() < 1e-9);
    }

    #[test]
    fn downsample_chip_curve_reduces_to_the_threshold_and_keeps_both_series_aligned() {
        let rows: Vec<(i64, f64, f64)> = (0..(LTTB_THRESHOLD as i64 + 500))
            .map(|i| (i, f64::from(i as i32), f64::from(i as i32) * 0.9))
            .collect();
        let curve = downsample_chip_curve(&rows);
        assert_eq!(curve.len(), LTTB_THRESHOLD);
        // Les deux series restent coherentes entre elles pour chaque point
        // retenu (ev = 0.9 * reel, comme dans les donnees source).
        for point in &curve {
            let expected_ev = point.cumulative_net_bb * 0.9;
            assert!(
                (point.cumulative_ev_adjusted_net_bb - expected_ev).abs() < 1e-6,
                "{point:?}"
            );
        }
    }

    #[test]
    fn downsample_profit_curve_keeps_everything_under_the_threshold() {
        let points: Vec<ProfitCurvePoint> = (0..10)
            .map(|i| ProfitCurvePoint {
                tournament_id: i,
                started_at: i * 1000,
                cumulative_profit_cents: i * 100,
                cumulative_profit_excluding_bounty_cents: i * 80,
            })
            .collect();
        let curve = downsample_profit_curve(points.clone());
        assert_eq!(curve, points);
    }
}
