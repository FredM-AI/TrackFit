#![warn(clippy::pedantic)]

//! Stockage SQLite (ADR-001) : resolution du dossier de donnees (ADR-007),
//! migrations numerotees (R-SCHEMA), writer unique + pool de lecture.

mod error;
mod import_log;
mod migrate;
mod paths;
mod repo;
mod store;

pub use error::StoreError;
pub use import_log::{ImportErrorRow, NewImportError};
pub use paths::resolve_data_dir;
pub use repo::{HandInsert, ImportReport, BATCH_SIZE};
pub use store::Store;
