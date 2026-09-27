use serde::{Deserialize, Serialize};

use crate::card::Card;
use crate::money::Chips;
use crate::street::Street;

/// Type d'action observe dans une main (PAR-7). Les montants et le contexte
/// (all-in, pot vise, cartes montrees) vivent sur `ActionRecord`, pas ici.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionKind {
    PostAnte,
    PostSmallBlind,
    PostBigBlind,
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    Shows,
    Collected,
}

/// Pot vise par une ligne `collected` (§4.5 : `from pot` / `from main pot` / `from side pot K`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotKind {
    /// `collected N from pot` (pas de main/side pot distincts).
    Pot,
    Main,
    Side(u8),
}

/// Une ligne d'action d'une main, dans l'ordre chronologique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub street: Street,
    pub pseudo: String,
    pub kind: ActionKind,
    /// Montant ajoute par l'action (post, call, bet) ou empoche (collected).
    pub amount: Option<Chips>,
    /// Pour une relance : total mise sur la street (`raises X to Y`).
    pub to_amount: Option<Chips>,
    pub is_all_in: bool,
    /// Renseigne uniquement pour `Collected`.
    pub pot: Option<PotKind>,
    /// Renseigne uniquement pour `Shows`.
    pub shown_cards: Option<Vec<Card>>,
    /// Libelle de la main montree (ex. "One pair : Jacks"), uniquement pour `Shows`.
    pub shown_label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Rank, Suit};

    #[test]
    fn serde_roundtrip_preserves_a_raise() {
        let action = ActionRecord {
            street: Street::Preflop,
            pseudo: "Hero".to_string(),
            kind: ActionKind::Raise,
            amount: Some(Chips::from_i64(3_000)),
            to_amount: Some(Chips::from_i64(4_000)),
            is_all_in: false,
            pot: None,
            shown_cards: None,
            shown_label: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        let back: ActionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn shows_carries_cards_and_label() {
        let action = ActionRecord {
            street: Street::Showdown,
            pseudo: "Hero".to_string(),
            kind: ActionKind::Shows,
            amount: None,
            to_amount: None,
            is_all_in: false,
            pot: None,
            shown_cards: Some(vec![
                Card::new(Rank::Ace, Suit::Spades),
                Card::new(Rank::Ace, Suit::Diamonds),
            ]),
            shown_label: Some("One pair : Aces".to_string()),
        };
        assert_eq!(action.shown_cards.as_ref().unwrap().len(), 2);
    }
}
