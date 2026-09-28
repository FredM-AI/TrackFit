#![warn(clippy::pedantic)]

//! Stockage SQLite (ADR-001) : resolution du dossier de donnees (ADR-007),
//! migrations numerotees (R-SCHEMA), writer unique + pool de lecture.

mod error;
mod migrate;
mod paths;
mod store;

pub use error::StoreError;
pub use paths::resolve_data_dir;
pub use store::Store;
