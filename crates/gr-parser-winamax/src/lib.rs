#![warn(clippy::pedantic)]

//! Parser du format Winamax (fichiers de mains et de summaries, en anglais — ADR-005).

mod board;
mod datetime;
mod detect;
#[cfg(test)]
mod fuzz_tests;
mod hand;
mod money_format;
#[cfg(test)]
mod snapshot_tests;
mod split;
mod summary;

pub use detect::WinamaxParser;
pub use split::split_hand_blocks;
