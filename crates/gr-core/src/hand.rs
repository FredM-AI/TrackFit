use serde::{Deserialize, Serialize};

use crate::action::{ActionRecord, PotKind};
use crate::card::Card;
use crate::money::Chips;
use crate::seat::SeatInfo;

/// Un pot dispute et ses gagnants (§4.5-4.7 : pots partages, main/side pots).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PotResult {
    pub pot: PotKind,
    pub amount: Chips,
    pub winners: Vec<String>,
}

/// Une main complete, telle qu'extraite d'un fichier d'historique Winamax (PAR-4 a PAR-10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandRecord {
    /// Cle d'unicite complete `#X-Y-Z` (PAR-4). X = table, Y = n° sur la table, Z = timestamp.
    pub room_hand_id: String,
    pub tournament_name: String,
    /// ID du tournoi, lu dans le nom de table `<nom>(<ID>)#<table>` (§4.2).
    pub tournament_room_id: String,
    pub table_name: String,
    pub table_max_seats: u8,
    pub button_seat: u8,
    pub level: u32,
    pub sb: Chips,
    pub bb: Chips,
    pub ante: Chips,
    /// Horodatage UTC en millisecondes epoch (R-MONEY).
    pub played_at: i64,
    pub seats: Vec<SeatInfo>,
    pub actions: Vec<ActionRecord>,
    pub board: Vec<Card>,
    pub pots: Vec<PotResult>,
    pub total_pot: Chips,
    pub rake: Chips,
    /// Pseudo du Hero, identifie par la ligne `Dealt to <pseudo>` (seul joueur dont les cartes sont connues avant l'abattage).
    pub hero_pseudo: Option<String>,
    pub hero_cards: Option<(Card, Card)>,
    /// Version semver du parser qui a produit cet enregistrement (PAR-13).
    pub parser_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Rank, Suit};

    fn sample_hand() -> HandRecord {
        HandRecord {
            room_hand_id: "#5038051313141678275-16-1790176801".to_string(),
            tournament_name: "OBELISK - TRIDENT SPACE KO".to_string(),
            tournament_room_id: "1173012730".to_string(),
            table_name: "OBELISK - TRIDENT SPACE KO(1173012730)#0194".to_string(),
            table_max_seats: 3,
            button_seat: 3,
            level: 1,
            sb: Chips::from_i64(10),
            bb: Chips::from_i64(20),
            ante: Chips::from_i64(3),
            played_at: 1_790_176_801_000,
            seats: vec![],
            actions: vec![],
            board: vec![],
            pots: vec![],
            total_pot: Chips::ZERO,
            rake: Chips::ZERO,
            hero_pseudo: Some("Hero".to_string()),
            hero_cards: Some((
                Card::new(Rank::Ace, Suit::Spades),
                Card::new(Rank::Ace, Suit::Diamonds),
            )),
            parser_version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn serde_roundtrip_preserves_the_hand() {
        let hand = sample_hand();
        let json = serde_json::to_string(&hand).unwrap();
        let back: HandRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, hand);
    }
}
