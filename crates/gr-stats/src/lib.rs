#![warn(clippy::pedantic)]

//! Moteur de statistiques (PRD §10, M4) : positions, profondeurs de tapis,
//! flags d'action/opportunite par main.

mod chips;
mod depth;
mod flag;
mod hand_class;
mod position;
mod postflop;
mod preflop;

pub use chips::{compute_net_bb, compute_net_chips};
pub use depth::{depth_bb, depth_bracket_label, DepthMode, DEFAULT_DEPTH_BRACKETS};
pub use flag::StatFlag;
pub use hand_class::compute_hand_class;
pub use position::{assign_positions, position_group, sb_bb_seats};
pub use postflop::{
    compute_cbf, compute_cbt, compute_fcbf, compute_postflop_counts, compute_wsd, compute_wtsd,
    compute_wwsf, saw_street, went_to_showdown, won_pot, PostflopActionCounts,
};
pub use preflop::{
    compute_3b, compute_4b, compute_ats, compute_f3b, compute_fsteal, compute_limp, compute_oshove,
    compute_pfr, compute_rfi, compute_rsteal, compute_vpip, preflop_line,
};
