#![warn(clippy::pedantic)]

//! Parser du format Winamax (fichiers de mains et de summaries, en anglais — ADR-005).

mod detect;

pub use detect::WinamaxParser;
