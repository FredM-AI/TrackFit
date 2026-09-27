#![warn(clippy::pedantic)]

//! Types de domaine partages par tout le workspace Graphite (sans I/O).

mod action;
mod card;
mod error;
mod hand;
mod money;
mod position;
mod seat;
mod street;
mod tournament;

pub use action::{ActionKind, ActionRecord, PotKind};
pub use card::{Card, Rank, Suit};
pub use error::CardParseError;
pub use hand::{HandRecord, PotResult};
pub use money::{Chips, Money};
pub use position::Position;
pub use seat::SeatInfo;
pub use street::Street;
pub use tournament::{KoType, TournamentBullet, TournamentSummary};
