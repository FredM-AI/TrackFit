const RANKS: [char; 13] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A',
];
const SUITS: [char; 4] = ['h', 'd', 'c', 's'];

/// PRNG deterministe minimal (`SplitMix64`) : suffisant pour generer un corpus
/// synthetique reproductible sans ajouter de dependance externe (CLAUDE.md
/// §2.8 : `rand` n'apporte rien ici que ces quelques lignes ne fassent deja).
#[derive(Debug, Clone)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        // Un seed nul degenererait en une suite de zeros : on le decale.
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier uniforme dans `[low, high]` (bornes incluses).
    pub(crate) fn range_u32(&mut self, low: u32, high: u32) -> u32 {
        let span = u64::from(high - low + 1);
        low + u32::try_from(self.next_u64() % span).unwrap_or(0)
    }

    pub(crate) fn pick_rank(&mut self) -> char {
        let idx = usize::try_from(self.next_u64() % 13).unwrap_or(0);
        RANKS[idx]
    }

    pub(crate) fn pick_suit(&mut self) -> char {
        let idx = usize::try_from(self.next_u64() % 4).unwrap_or(0);
        SUITS[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic_for_a_given_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_u32_stays_within_bounds() {
        let mut rng = Rng::new(7);
        for _ in 0..1000 {
            let v = rng.range_u32(2, 7);
            assert!((2..=7).contains(&v));
        }
    }

    #[test]
    fn range_u32_handles_a_degenerate_single_value_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10 {
            assert_eq!(rng.range_u32(5, 5), 5);
        }
    }
}
