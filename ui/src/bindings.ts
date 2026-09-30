// Ce fichier est genere par ts-rs (M3-1). Ne pas l'editer a la main.

export type AdditionalKpisPayload = { avg_finish_position: number | null, final_table_rate: number | null, best_win_cents: number, biggest_tournament_entrants: number | null, };

export type BuyinRoiRowPayload = { buyin_min_cents: number, buyin_max_cents: number | null, kpis: PivotKpisPayload, };

export type ChipCurvePoint = { played_at: number, cumulative_net_bb: number, cumulative_ev_adjusted_net_bb: number, };

export type DayOfWeekPivotRowPayload = { weekday: number, kpis: PivotKpisPayload, };

export type FinishPercentileBucketPayload = { floor_percent: number, tournaments_count: number, };

export type HandsNewPayload = { hands_inserted: number, };

export type HeroProfilePayload = { id: number, name: string, is_default: boolean, pseudos: Array<string>, };

export type HomeSnapshotPayload = { current_period: PeriodKpis, previous_period: PeriodKpis, profit_curve: Array<ProfitCurvePoint>, last_session: LastSessionPayload | null, import_status: ImportStatusPayload, };

export type HourPivotRowPayload = { hour: number, kpis: PivotKpisPayload, };

export type ImportProgressPayload = { files_total: number, files_done: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, };

export type ImportStatusPayload = { watched_roots_count: number, total_hands: number, unresolved_errors_count: number, };

export type ImportSummaryPayload = { files_scanned: number, files_imported: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, summaries_attached: number, summaries_failed: number, cancelled: boolean, };

export type LastSessionPayload = { started_at: number, ended_at: number, hands: number, tournaments: number, profit_cents: number, best_tournament_name: string | null, best_tournament_profit_cents: number | null, worst_tournament_name: string | null, worst_tournament_profit_cents: number | null, };

export type MonthPivotRowPayload = { month: number, kpis: PivotKpisPayload, };

export type PeriodKpis = { tournaments_count: number, cost_cents: number, fees_cents: number, profit_cents: number, roi: number | null, itm_rate: number | null, abi_cents: number | null, 
/**
 * Somme de `hand_players.allin_ev_diff_chips` en bb (M5-4).
 */
allin_ev_diff_bb: number, 
/**
 * `None` si aucune session n'a demarre dans la periode (pas de "$/h"
 * calculable, pas un vrai zero).
 */
dollars_per_hour_cents: number | null, };

export type PivotKpisPayload = { tournaments_count: number, cost_cents: number, fees_cents: number, profit_cents: number, roi: number | null, itm_rate: number | null, abi_cents: number | null, };

export type PivotRowPayload = { buyin_min_cents: number, buyin_max_cents: number | null, is_ko: boolean, tournaments_count: number, cost_cents: number, fees_cents: number, profit_cents: number, roi: number | null, itm_rate: number | null, abi_cents: number | null, };

export type ProfitCurvePoint = { tournament_id: number, started_at: number, cumulative_profit_cents: number, 
/**
 * Meme courbe hors bounties (PRD §13.5/G1 : "en superposition le
 * profit sans bounties").
 */
cumulative_profit_excluding_bounty_cents: number, };

export type ResultsSnapshotPayload = { 
/**
 * G1, repris de M6-2 (tout l'historique, non filtre par periode).
 */
profit_curve: Array<ProfitCurvePoint>, 
/**
 * G2 : reel vs ajuste EV all-in, cumule, en bb.
 */
chip_curve: Array<ChipCurvePoint>, 
/**
 * G6, version "par jour" (semaine/mois/heatmap differes).
 */
volume: Array<VolumePointPayload>, 
/**
 * Pivot buy-in x KO/non-KO.
 */
pivot: Array<PivotRowPayload>, 
/**
 * Export CSV du pivot ci-dessus, deja pret (le frontend n'a qu'a le
 * proposer en telechargement, pas de logique de formatage cote UI).
 */
pivot_csv: string, 
/**
 * G3 : ROI par tranche de buy-in, tous formats confondus.
 */
roi_by_buyin: Array<BuyinRoiRowPayload>, roi_by_buyin_csv: string, 
/**
 * G5 : distribution des places de sortie en percentile des inscrits.
 * Pas de mise en evidence de "la bulle" (PRD §9.2) : bloquee,
 * `tournaments.paid_places` n'est jamais renseigne (absent du format de
 * summary Winamax, voir `gr_analytics::results`).
 */
finish_percentile_distribution: Array<FinishPercentileBucketPayload>, additional_kpis: AdditionalKpisPayload, 
/**
 * Pivot complet (§13.2) : dimensions supplementaires au pivot buy-in x
 * KO ci-dessus.
 */
pivot_speed: Array<SpeedPivotRowPayload>, pivot_speed_csv: string, pivot_day_of_week: Array<DayOfWeekPivotRowPayload>, pivot_day_of_week_csv: string, pivot_hour: Array<HourPivotRowPayload>, pivot_hour_csv: string, pivot_month: Array<MonthPivotRowPayload>, pivot_month_csv: string, };

export type SpeedPivotRowPayload = { speed: string | null, kpis: PivotKpisPayload, };

export type StatusSnapshotPayload = { hands_today: number, last_hand_at: number | null, watcher_run_state: WatcherRunState, };

export type VolumePointPayload = { day_epoch_ms: number, tournaments_count: number, };

export type WatcherRunState = "idle" | "active" | "paused";

export type WinamaxAccountPayload = { pseudo: string, history_dir: string, hand_file_count: number, };

