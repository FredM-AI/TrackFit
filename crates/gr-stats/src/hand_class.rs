//! Classe de main de depart au format standard a deux caracteres (+ suffixe
//! suited/offsuit pour une non-paire), ex. `'AKs'`, `'TT'`, `'Q9o'` — PRD
//! §13.5 (grille 13×13), `hand_players.hand_class`, M7-7.

use gr_core::Card;

/// Classe de `(a, b)` : rang le plus fort d'abord (`Rank` est deja `Ord`,
/// l'As est le plus fort), `"XX"` pour une paire, sinon `"XYs"` (memes
/// couleurs) ou `"XYo"` (couleurs differentes).
#[must_use]
pub fn compute_hand_class(a: Card, b: Card) -> String {
    let (high, low) = if a.rank >= b.rank { (a, b) } else { (b, a) };
    if high.rank == low.rank {
        format!("{}{}", high.rank.to_char(), low.rank.to_char())
    } else if high.suit == low.suit {
        format!("{}{}s", high.rank.to_char(), low.rank.to_char())
    } else {
        format!("{}{}o", high.rank.to_char(), low.rank.to_char())
    }
}

#[cfg(test)]
mod tests {
    use gr_core::{Rank, Suit};

    use super::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card::new(rank, suit)
    }

    #[test]
    fn a_pair_has_no_suited_or_offsuit_suffix() {
        let class = compute_hand_class(card(Rank::Ten, Suit::Hearts), card(Rank::Ten, Suit::Clubs));
        assert_eq!(class, "TT");
    }

    #[test]
    fn a_suited_non_pair_gets_the_s_suffix_with_the_higher_rank_first() {
        let class = compute_hand_class(
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
        );
        assert_eq!(class, "AKs");
    }

    #[test]
    fn an_offsuit_non_pair_gets_the_o_suffix_with_the_higher_rank_first_regardless_of_argument_order(
    ) {
        // King donne en premier argument, Two en second : le rang le plus
        // fort doit quand meme sortir en premier dans le libelle.
        let class =
            compute_hand_class(card(Rank::King, Suit::Hearts), card(Rank::Two, Suit::Clubs));
        assert_eq!(class, "K2o");

        // Meme paire de cartes, arguments inverses : meme resultat.
        let class_reversed =
            compute_hand_class(card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts));
        assert_eq!(class_reversed, "K2o");
    }

    #[test]
    fn queen_nine_offsuit_matches_the_prd_example() {
        // PRD §13.5 / schema : exemple cite explicitement ('Q9o').
        let class = compute_hand_class(
            card(Rank::Queen, Suit::Diamonds),
            card(Rank::Nine, Suit::Spades),
        );
        assert_eq!(class, "Q9o");
    }
}
