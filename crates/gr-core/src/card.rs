use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::CardParseError;

/// Rang d'une carte, ordonne du plus faible (`Two`) au plus fort (`Ace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    #[must_use]
    pub fn to_char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }

    fn from_char(c: char) -> Result<Self, CardParseError> {
        match c {
            '2' => Ok(Rank::Two),
            '3' => Ok(Rank::Three),
            '4' => Ok(Rank::Four),
            '5' => Ok(Rank::Five),
            '6' => Ok(Rank::Six),
            '7' => Ok(Rank::Seven),
            '8' => Ok(Rank::Eight),
            '9' => Ok(Rank::Nine),
            'T' | 't' => Ok(Rank::Ten),
            'J' | 'j' => Ok(Rank::Jack),
            'Q' | 'q' => Ok(Rank::Queen),
            'K' | 'k' => Ok(Rank::King),
            'A' | 'a' => Ok(Rank::Ace),
            other => Err(CardParseError::UnknownRank(other)),
        }
    }
}

/// Couleur d'une carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    #[must_use]
    pub fn to_char(self) -> char {
        match self {
            Suit::Clubs => 'c',
            Suit::Diamonds => 'd',
            Suit::Hearts => 'h',
            Suit::Spades => 's',
        }
    }

    fn from_char(c: char) -> Result<Self, CardParseError> {
        match c {
            'c' | 'C' => Ok(Suit::Clubs),
            'd' | 'D' => Ok(Suit::Diamonds),
            'h' | 'H' => Ok(Suit::Hearts),
            's' | 'S' => Ok(Suit::Spades),
            other => Err(CardParseError::UnknownSuit(other)),
        }
    }
}

/// Une carte, affichee et parsee au format Winamax `"Ah"` (rang puis couleur).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    #[must_use]
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank.to_char(), self.suit.to_char())
    }
}

impl FromStr for Card {
    type Err = CardParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chars: Vec<char> = s.chars().collect();
        let [rank_char, suit_char] = chars[..] else {
            return Err(CardParseError::InvalidLength(s.to_string()));
        };
        Ok(Card::new(
            Rank::from_char(rank_char)?,
            Suit::from_char(suit_char)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ace_of_hearts() {
        let card: Card = "Ah".parse().unwrap();
        assert_eq!(card, Card::new(Rank::Ace, Suit::Hearts));
    }

    #[test]
    fn parses_ten_of_spades() {
        let card: Card = "Ts".parse().unwrap();
        assert_eq!(card, Card::new(Rank::Ten, Suit::Spades));
    }

    #[test]
    fn displays_back_to_the_same_notation() {
        for text in ["Ah", "Kd", "Ts", "2c", "9h"] {
            let card: Card = text.parse().unwrap();
            assert_eq!(card.to_string(), text);
        }
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            "Ahh".parse::<Card>(),
            Err(CardParseError::InvalidLength("Ahh".to_string()))
        );
        assert_eq!(
            "A".parse::<Card>(),
            Err(CardParseError::InvalidLength("A".to_string()))
        );
    }

    #[test]
    fn rejects_unknown_rank_or_suit() {
        assert_eq!("Xh".parse::<Card>(), Err(CardParseError::UnknownRank('X')));
        assert_eq!("Ax".parse::<Card>(), Err(CardParseError::UnknownSuit('x')));
    }

    #[test]
    fn ranks_are_ordered_low_to_high() {
        assert!(Rank::Two < Rank::Ten);
        assert!(Rank::Ten < Rank::Ace);
        assert!(Rank::King < Rank::Ace);
    }
}
