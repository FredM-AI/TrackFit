use serde::{Deserialize, Serialize};

/// Rue de jeu d'une main (PAR-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}
