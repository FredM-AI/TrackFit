//! Equite par enumeration exacte des boards restants (PRD §10.6, M5-3),
//! pour 2 ou 3 joueurs. Enumeration par boucles imbriquees ([`combinations`])
//! plutot que le generateur interne de `rs_poker` (trop lent sans BMI2, cf.
//! le commentaire de module de `combinations`), multi-threadee
//! (`std::thread::scope`, aucune dependance supplementaire) : necessaire
//! pour tenir la cible de perf du CA (HU preflop < 60 ms sur le Celeron
//! cible, 4 coeurs).

use rs_poker::core::Card;

use crate::combinations::for_each_combination_with_first_in;
use crate::deck::{remaining_and_bases, score_completed_board};

/// Equite exacte de chaque joueur (dans l'ordre de `hole_cards`), calculee
/// en enumerant tous les tirages possibles des cartes manquantes du board
/// (0 a 5 selon la rue). Repartit les tirages sur les coeurs disponibles.
pub(crate) fn exact_equity(hole_cards: &[[Card; 2]], known_board: &[Card]) -> Vec<f64> {
    let player_count = hole_cards.len();
    let missing_count = 5usize.saturating_sub(known_board.len());
    let (remaining, bases) = remaining_and_bases(hole_cards, known_board);
    let pool: Vec<Card> = remaining.into_iter().collect();

    if missing_count == 0 {
        // Riviere deja jouee : un seul tirage (vide), pas besoin de threads.
        let mut shares = vec![0.0f64; player_count];
        score_completed_board(&bases, &[], &mut shares);
        return shares;
    }

    let thread_count = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);

    let totals: Vec<f64> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..thread_count)
            .map(|thread_idx| {
                let bases = &bases;
                let pool = &pool;
                scope.spawn(move || {
                    let mut shares = vec![0.0f64; player_count];
                    let first_indices = (thread_idx..pool.len()).step_by(thread_count);
                    for_each_combination_with_first_in(
                        pool,
                        missing_count,
                        first_indices,
                        &mut |combo| score_completed_board(bases, combo, &mut shares),
                    );
                    shares
                })
            })
            .collect();

        let mut totals = vec![0.0f64; player_count];
        for handle in handles {
            let shares = handle.join().unwrap_or_else(|_| vec![0.0; player_count]);
            for (total, share) in totals.iter_mut().zip(shares) {
                *total += share;
            }
        }
        totals
    });

    normalize(&pool, missing_count, &totals)
}

/// Normalise les parts accumulees par le nombre total de tirages
/// (`C(pool.len(), missing_count)`), calcule directement plutot que par
/// sommation flottante des parts (evite toute derive d'arrondi sur ~1,7M
/// tirages).
fn normalize(pool: &[Card], missing_count: usize, totals: &[f64]) -> Vec<f64> {
    let total_runouts = binomial(pool.len(), missing_count);
    totals.iter().map(|total| total / total_runouts).collect()
}

#[allow(clippy::cast_precision_loss)]
fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut result = 1.0f64;
    for i in 0..k {
        result *= (n - i) as f64;
        result /= (i + 1) as f64;
    }
    result
}
