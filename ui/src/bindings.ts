// Ce fichier est genere par ts-rs (M3-1). Ne pas l'editer a la main.

export type HandsNewPayload = { hands_inserted: number, };

export type ImportProgressPayload = { files_total: number, files_done: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, };

export type ImportSummaryPayload = { files_scanned: number, files_imported: number, hands_inserted: number, hands_duplicate: number, hands_failed: number, summaries_attached: number, summaries_failed: number, cancelled: boolean, };

export type WinamaxAccountPayload = { pseudo: string, history_dir: string, hand_file_count: number, };

