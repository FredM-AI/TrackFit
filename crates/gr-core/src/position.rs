use std::fmt;

use serde::{Deserialize, Serialize};

/// Position a table (PRD §10.4). L'affectation reelle se fait en M4-1 ;
/// ce type ne fait que nommer les positions possibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Position {
    Utg,
    Utg1,
    Utg2,
    Lj,
    Hj,
    Co,
    Btn,
    Sb,
    Bb,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Position::Utg => "UTG",
            Position::Utg1 => "UTG+1",
            Position::Utg2 => "UTG+2",
            Position::Lj => "LJ",
            Position::Hj => "HJ",
            Position::Co => "CO",
            Position::Btn => "BTN",
            Position::Sb => "SB",
            Position::Bb => "BB",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_the_prd_labels() {
        assert_eq!(Position::Utg1.to_string(), "UTG+1");
        assert_eq!(Position::Btn.to_string(), "BTN");
    }
}
