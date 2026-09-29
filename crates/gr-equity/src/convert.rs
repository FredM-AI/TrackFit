//! Conversion `gr_core::Card` <-> `rs_poker::core::Card` (M5-3). `rs_poker`
//! est un detail d'implementation de ce crate : aucun autre crate du
//! workspace ne doit en dependre, seul `gr-equity` connait ce type. La
//! conversion se fait rang par rang/couleur par couleur plutot que par un
//! cast de discriminant : les deux enums de couleur n'ont pas le meme ordre
//! (`gr_core::Suit` = Clubs, Diamonds, Hearts, Spades ; `rs_poker::Suit` =
//! Spade, Club, Heart, Diamond).

use gr_core::{Rank as GrRank, Suit as GrSuit};
use rs_poker::core::{Suit as RsSuit, Value as RsValue};

pub(crate) fn to_rs_card(card: gr_core::Card) -> rs_poker::core::Card {
    rs_poker::core::Card::new(to_rs_value(card.rank), to_rs_suit(card.suit))
}

fn to_rs_value(rank: GrRank) -> RsValue {
    match rank {
        GrRank::Two => RsValue::Two,
        GrRank::Three => RsValue::Three,
        GrRank::Four => RsValue::Four,
        GrRank::Five => RsValue::Five,
        GrRank::Six => RsValue::Six,
        GrRank::Seven => RsValue::Seven,
        GrRank::Eight => RsValue::Eight,
        GrRank::Nine => RsValue::Nine,
        GrRank::Ten => RsValue::Ten,
        GrRank::Jack => RsValue::Jack,
        GrRank::Queen => RsValue::Queen,
        GrRank::King => RsValue::King,
        GrRank::Ace => RsValue::Ace,
    }
}

fn to_rs_suit(suit: GrSuit) -> RsSuit {
    match suit {
        GrSuit::Clubs => RsSuit::Club,
        GrSuit::Diamonds => RsSuit::Diamond,
        GrSuit::Hearts => RsSuit::Heart,
        GrSuit::Spades => RsSuit::Spade,
    }
}

#[cfg(test)]
mod tests {
    use gr_core::{Rank, Suit};
    use rs_poker::core::Rankable;

    use super::*;

    #[test]
    fn every_rank_and_suit_round_trips_through_the_same_display_notation() {
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                let card = gr_core::Card::new(rank, suit);
                let rs_card = to_rs_card(card);
                assert_eq!(rs_card.to_string(), card.to_string());
            }
        }
    }

    #[test]
    fn converted_cards_are_still_rankable() {
        // Une simple verification bout-en-bout : une quinte flush royale
        // convertie doit toujours s'evaluer comme telle.
        let royal_flush = ["As", "Ks", "Qs", "Js", "Ts"]
            .map(|s| s.parse::<gr_core::Card>().expect("valid card"))
            .map(to_rs_card);
        assert_eq!(
            royal_flush.rank().category(),
            rs_poker::core::CoreRank::StraightFlush
        );
    }
}
