#![warn(clippy::pedantic)]

//! Generateur de tournois synthetiques au format Winamax (M2-4), base
//! exclusivement sur les motifs deja documentes dans
//! `docs/formats/winamax.md` (R-FORMAT) : sert a fabriquer un gros volume
//! de mains 100% parsables pour les tests de charge (`just synth`, M2-3/M8-4).

mod corpus;
mod generate;
mod rng;

pub use corpus::SynthCorpus;
pub use generate::GeneratedTournament;
