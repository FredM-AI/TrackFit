#![warn(clippy::pedantic)]

//! Moteur de statistiques (PRD §10, M4) : positions, profondeurs de tapis,
//! flags d'action/opportunite par main.

mod depth;
mod flag;
mod position;
mod preflop;

pub use depth::{depth_bb, depth_bracket_label, DepthMode, DEFAULT_DEPTH_BRACKETS};
pub use flag::StatFlag;
pub use position::{assign_positions, position_group, sb_bb_seats};
pub use preflop::{
    compute_3b, compute_4b, compute_ats, compute_f3b, compute_fsteal, compute_limp, compute_oshove,
    compute_pfr, compute_rfi, compute_rsteal, compute_vpip, preflop_line,
};
