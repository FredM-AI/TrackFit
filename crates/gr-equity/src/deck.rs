//! Petits utilitaires partages par `exact`/`monte_carlo` (M5-3) : le jeu de
//! 52 cartes, et les accumulateurs de base (main + board deja connu) par
//! joueur.

use rs_poker::core::{Card, CardBitSet, Rank, SevenCardAccum, Suit, Value};

/// Nombre maximal de joueurs (meme limite que `gr_stats::assign_positions`,
/// BACKLOG M4-1 : jamais observe au-dela sur Winamax). Taille fixe du
/// tableau de rangs utilise par `score_completed_board` pour rester sans
/// allocation dans la boucle chaude (jusqu'a ~1,7M appels par calcul exact).
const MAX_PLAYERS: usize = 9;

pub(crate) fn full_deck() -> CardBitSet {
    let mut set = CardBitSet::new();
    for suit in Suit::suits() {
        for value in Value::values() {
            set.insert(Card::new(value, suit));
        }
    }
    set
}

/// Jeu restant (52 cartes moins les mains et le board deja connus) et, pour
/// chaque joueur, un accumulateur pre-rempli avec sa main + le board deja
/// connu (n'attend plus que les cartes manquantes du board).
pub(crate) fn remaining_and_bases(
    hole_cards: &[[Card; 2]],
    known_board: &[Card],
) -> (CardBitSet, Vec<SevenCardAccum>) {
    let mut remaining = full_deck();
    for pair in hole_cards {
        for card in pair {
            remaining.remove(*card);
        }
    }
    for card in known_board {
        remaining.remove(*card);
    }

    let bases = hole_cards
        .iter()
        .map(|pair| {
            let mut acc = SevenCardAccum::new();
            for card in pair {
                acc.add(*card);
            }
            for card in known_board {
                acc.add(*card);
            }
            acc
        })
        .collect();

    (remaining, bases)
}

/// Part du pot (0.0 a 1.0) gagnee par chaque joueur pour un tirage complet
/// des cartes manquantes du board : la meilleure main empoche 1 si elle est
/// seule, sinon le pot est partage a parts egales entre les mains a
/// egalite (PRD §10.6).
///
/// Le tableau de rangs est de taille fixe ([`MAX_PLAYERS`]) : ni lui ni
/// `missing` (une slice, jamais reallouee) n'allouent sur le tas — cette
/// fonction tourne jusqu'a ~1,7M fois par calcul exact.
///
/// # Panics
/// Si `bases.len()` depasse [`MAX_PLAYERS`] — n'arrive jamais en pratique,
/// la validation en amont (`EquityError::TooManyPlayers`) l'empeche.
#[inline]
pub(crate) fn score_completed_board(
    bases: &[SevenCardAccum],
    missing: &[Card],
    shares: &mut [f64],
) {
    debug_assert!(bases.len() <= MAX_PLAYERS);
    let mut ranks: [Option<Rank>; MAX_PLAYERS] = [None; MAX_PLAYERS];
    for (slot, base) in ranks.iter_mut().zip(bases) {
        let mut acc = *base;
        for card in missing {
            acc.add(*card);
        }
        *slot = Some(acc.rank());
    }

    // Vide seulement si `bases` l'est, ce que la validation en amont
    // interdit (>= 2 joueurs) ; ne rien faire plutot que paniquer si ce
    // n'etait pas le cas (R-NOPANIC).
    let Some(best) = ranks.iter().flatten().copied().max() else {
        return;
    };
    let winners = ranks.iter().flatten().filter(|r| **r == best).count();
    #[allow(clippy::cast_precision_loss)]
    let share = 1.0 / winners as f64;
    for (slot, rank) in shares.iter_mut().zip(ranks.iter().flatten()) {
        if *rank == best {
            *slot += share;
        }
    }
}
