#![warn(clippy::pedantic)]

//! Pipeline d'import en masse (M2-3) : decouverte des fichiers, parsing,
//! ecriture par lots dans `gr-store`, progression, annulation, rapport final.

mod error;
mod import;
mod scan;

pub use error::IngestError;
pub use import::{run_import, CancelToken, ImportFailure, ImportProgress, ImportSummary};
pub use scan::discover_hand_files;
