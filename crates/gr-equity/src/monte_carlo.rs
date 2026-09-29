//! Equite par Monte Carlo a graine fixe (PRD §10.6, M5-3), pour 4 joueurs
//! ou plus : enumerer tous les tirages devient trop coûteux (ex. 4 mains
//! connues preflop laisse 44 cartes, C(44,5) = 1 086 008 tirages, et la
//! combinatoire grandit vite avec le nombre de joueurs). 200 000 tirages,
//! toujours la meme graine : deux appels avec les memes cartes renvoient
//! exactement le meme resultat (reproductible, y compris pour les tests).

use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_poker::core::Card;

use crate::deck::{remaining_and_bases, score_completed_board};

/// Graine fixe (PRD §10.6, "avec une graine deterministe") : la meme pour
/// tous les calculs, volontairement. Valeur arbitraire, jamais destinee a
/// changer (un changement romprait la reproductibilite des calculs deja
/// affiches/exportes).
const SEED: u64 = 0x4772_6170_6869_7465; // "Graphite" en ASCII/hex.

/// Nombre de tirages (PRD §10.6 : "Monte Carlo a 200 000 tirages au-dela").
pub(crate) const ITERATIONS: u32 = 200_000;

/// Equite estimee de chaque joueur (dans l'ordre de `hole_cards`), par
/// tirage aleatoire (graine fixe) des cartes manquantes du board.
pub(crate) fn monte_carlo_equity(hole_cards: &[[Card; 2]], known_board: &[Card]) -> Vec<f64> {
    let player_count = hole_cards.len();
    let missing_count = 5usize.saturating_sub(known_board.len());
    let (remaining, bases) = remaining_and_bases(hole_cards, known_board);

    let mut rng = StdRng::seed_from_u64(SEED);
    let mut shares = vec![0.0f64; player_count];
    let mut drawn: Vec<Card> = Vec::with_capacity(missing_count);

    for _ in 0..ITERATIONS {
        let mut pool = remaining;
        drawn.clear();
        for _ in 0..missing_count {
            let Some(card) = pool.sample_one(&mut rng) else {
                break;
            };
            pool.remove(card);
            drawn.push(card);
        }
        score_completed_board(&bases, &drawn, &mut shares);
    }

    #[allow(clippy::cast_precision_loss)]
    let iterations = f64::from(ITERATIONS);
    shares.into_iter().map(|total| total / iterations).collect()
}
