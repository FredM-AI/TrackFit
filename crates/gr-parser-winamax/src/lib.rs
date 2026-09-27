#![warn(clippy::pedantic)]

//! Parser du format Winamax (fichiers de mains et de summaries, en anglais — ADR-005).

mod detect;
mod split;

pub use detect::WinamaxParser;
pub use split::split_hand_blocks;
