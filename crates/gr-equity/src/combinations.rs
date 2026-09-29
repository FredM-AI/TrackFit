//! Enumeration de combinaisons de cartes par boucles imbriquees classiques
//! (indices croissants), plutot que le generateur interne de `rs_poker`
//! (`CardIter`, base sur l'instruction materielle PDEP/BMI2). Verifie a la
//! main (recherche M5-3, documentee dans `docs/BACKLOG.md`) : sur un
//! processeur sans BMI2, `CardIter` retombe sur une emulation logicielle de
//! PDEP mesuree a ~70 ns/combinaison — bien trop lent pour la cible de perf
//! du CA (HU preflop < 60 ms). Cette implementation est independante du jeu
//! d'instructions et se repartit facilement entre threads.

use rs_poker::core::Card;

/// Appelle `visit` pour chaque combinaison de `k` cartes parmi `pool`
/// (`k >= 1`) dont le PREMIER indice choisi appartient a `first_indices` —
/// permet de repartir l'enumeration entre threads (un `first_indices` par
/// thread, ex. `(thread_idx..pool.len()).step_by(thread_count)`) sans
/// qu'aucun ne parcoure `pool` en entier ni ne double-compte une
/// combinaison.
pub(crate) fn for_each_combination_with_first_in(
    pool: &[Card],
    k: usize,
    first_indices: impl Iterator<Item = usize>,
    visit: &mut impl FnMut(&[Card]),
) {
    debug_assert!(
        k >= 1,
        "k == 0 n'a pas de \"premier indice\" ; gere a part par l'appelant"
    );
    let mut chosen: Vec<Card> = Vec::with_capacity(k);
    for i in first_indices {
        if i + k > pool.len() {
            continue;
        }
        chosen.clear();
        chosen.push(pool[i]);
        extend_combination(pool, k, i + 1, &mut chosen, visit);
    }
}

#[inline]
fn extend_combination(
    pool: &[Card],
    k: usize,
    start: usize,
    chosen: &mut Vec<Card>,
    visit: &mut impl FnMut(&[Card]),
) {
    if chosen.len() == k {
        visit(chosen);
        return;
    }
    let remaining_needed = k - chosen.len();
    if pool.len() < start + remaining_needed {
        return;
    }
    for i in start..=(pool.len() - remaining_needed) {
        chosen.push(pool[i]);
        extend_combination(pool, k, i + 1, chosen, visit);
        chosen.pop();
    }
}

#[cfg(test)]
mod tests {
    use rs_poker::core::{Suit, Value};

    use super::*;

    fn pool_of(n: usize) -> Vec<Card> {
        Value::values()
            .into_iter()
            .take(n)
            .map(|value| Card::new(value, Suit::Spade))
            .collect()
    }

    #[test]
    fn visits_exactly_n_choose_k_combinations_without_duplicates() {
        let pool = pool_of(10);
        let mut seen = std::collections::HashSet::new();
        let mut count = 0;
        for_each_combination_with_first_in(&pool, 3, 0..pool.len(), &mut |combo| {
            count += 1;
            let mut key: Vec<Card> = combo.to_vec();
            key.sort_by_key(std::string::ToString::to_string);
            assert!(seen.insert(key), "combinaison en double : {combo:?}");
        });
        assert_eq!(count, 120); // C(10,3)
    }

    #[test]
    fn splitting_first_indices_across_two_ranges_covers_the_same_total() {
        let pool = pool_of(8);
        let mut count_a = 0;
        for_each_combination_with_first_in(&pool, 4, (0..pool.len()).step_by(2), &mut |_| {
            count_a += 1;
        });
        let mut count_b = 0;
        for_each_combination_with_first_in(&pool, 4, (1..pool.len()).step_by(2), &mut |_| {
            count_b += 1;
        });
        assert_eq!(count_a + count_b, 70); // C(8,4)
    }

    #[test]
    fn a_single_card_pick_visits_every_card_once() {
        let pool = pool_of(5);
        let mut visited = Vec::new();
        for_each_combination_with_first_in(&pool, 1, 0..pool.len(), &mut |combo| {
            visited.push(combo[0]);
        });
        assert_eq!(visited.len(), 5);
    }
}
