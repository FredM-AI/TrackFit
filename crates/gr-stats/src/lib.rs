#![warn(clippy::pedantic)]

//! Moteur de statistiques (PRD §10, M4) : positions, profondeurs de tapis,
//! flags d'action/opportunite par main.

mod depth;
mod position;

pub use depth::{depth_bb, depth_bracket_label, DepthMode, DEFAULT_DEPTH_BRACKETS};
pub use position::{assign_positions, sb_bb_seats};
