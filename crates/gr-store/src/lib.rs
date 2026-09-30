#![warn(clippy::pedantic)]

//! Stockage SQLite (ADR-001) : resolution du dossier de donnees (ADR-007),
//! migrations numerotees (R-SCHEMA), writer unique + pool de lecture.

mod backfill;
mod error;
mod hero;
mod import_log;
mod migrate;
mod paths;
mod replay;
mod repo;
mod sessions;
mod status;
mod store;
mod summary_repo;
mod tags;
mod ticket_types;

pub use error::StoreError;
pub use hero::HeroProfileRow;
pub use import_log::{ImportErrorRow, ImportFileProgress, NewImportError};
pub use paths::resolve_data_dir;
pub use replay::{
    HandReplay, ReplayAllIn, ReplayAllInPlayer, ReplayPot, ReplaySeatState, ReplaySeatSummary,
    ReplayStep,
};
pub use repo::{HandInsert, ImportReport, BATCH_SIZE};
pub use sessions::LastSessionRow;
pub use store::Store;
pub use summary_repo::AttachSummaryReport;
pub use tags::TagRow;
pub use ticket_types::TicketTypeRow;
