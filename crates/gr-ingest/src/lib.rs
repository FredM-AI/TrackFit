#![warn(clippy::pedantic)]

//! Pipeline d'import en masse (M2-3) : decouverte des fichiers, parsing,
//! ecriture par lots dans `gr-store`, progression, annulation, rapport final.
//! Rattachement des summaries aux tournois (M2-6). Detection des comptes
//! Winamax locaux pour l'assistant de premier lancement (M3-1).

mod accounts;
mod error;
mod import;
mod scan;
mod summary;

pub use accounts::{detect_winamax_accounts, WinamaxAccount};
pub use error::IngestError;
pub use import::{
    reparse_import_error, run_import, CancelToken, ImportFailure, ImportProgress, ImportSummary,
    ReparseOutcome,
};
pub use scan::{discover_hand_files, discover_summary_files};
pub use summary::{import_summaries, SummaryImportSummary};
