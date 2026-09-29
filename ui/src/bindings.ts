// Ce fichier est genere par ts-rs (M3-1). Ne pas l'editer a la main.

export type ChipCurvePoint = { played_at: number, cumulative_net_bb: number, cumulative_ev_adjusted_net_bb: number, };

export type HandsNewPayload = { hands_inserted: number, };

export type HeroProfilePayload = { id: number, name: string, is_default: boolean, pseudos: Array<string>, };

export type HomeSnapshotPayload = { current_period: PeriodKpis, previous_period: PeriodKpis, profit_curve: Array<ProfitCurvePoint>, last_session: LastSessionPayload | null, import_status: ImportStatusPayload, };

export type ImportProgressPayload = { files_total: number, files_done: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, };

export type ImportStatusPayload = { watched_roots_count: number, total_hands: number, unresolved_errors_count: number, };

export type ImportSummaryPayload = { files_scanned: number, files_imported: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, summaries_attached: number, summaries_failed: number, cancelled: boolean, };

export type LastSessionPayload = { started_at: number, ended_at: number, hands: number, tournaments: number, profit_cents: number, best_tournament_name: string | null, best_tournament_profit_cents: number | null, worst_tournament_name: string | null, worst_tournament_profit_cents: number | null, };

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
 * Pivot reduit buy-in x KO/non-KO.
 */
pivot: Array<PivotRowPayload>, 
/**
 * Export CSV du pivot ci-dessus, deja pret (le frontend n'a qu'a le
 * proposer en telechargement, pas de logique de formatage cote UI).
 */
pivot_csv: string, };

export type StatusSnapshotPayload = { hands_today: number, last_hand_at: number | null, watcher_run_state: WatcherRunState, };

export type VolumePointPayload = { day_epoch_ms: number, tournaments_count: number, };

export type WatcherRunState = "idle" | "active" | "paused";

export type WinamaxAccountPayload = { pseudo: string, history_dir: string, hand_file_count: number, };

