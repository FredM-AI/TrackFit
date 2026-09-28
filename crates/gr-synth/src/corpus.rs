use crate::generate::{generate_tournament, GeneratedTournament};
use crate::rng::Rng;

/// Produit des tournois synthetiques jusqu'a couvrir (environ) `total_hands`
/// mains au total (M2-4). Chaque tournoi est genere a la demande : rien
/// n'est accumule en memoire au-dela du tournoi courant, pour rester leger
/// sur `just synth N=1000000`.
pub struct SynthCorpus {
    rng: Rng,
    remaining: usize,
    tournament_index: u64,
}

impl SynthCorpus {
    #[must_use]
    pub fn new(total_hands: usize, seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            remaining: total_hands,
            tournament_index: 0,
        }
    }
}

impl Iterator for SynthCorpus {
    type Item = GeneratedTournament;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.tournament_index += 1;
        let tournament = generate_tournament(&mut self.rng, self.tournament_index, self.remaining);
        self.remaining = self.remaining.saturating_sub(tournament.hand_count);
        Some(tournament)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_exactly_the_requested_total_of_hands() {
        // Le dernier tournoi est toujours borne par les mains restantes
        // (`generate_tournament` plafonne sur `hands_wanted`) : le total
        // produit correspond donc exactement a la demande, jamais un
        // tournoi de trop ni un manque.
        let total: usize = SynthCorpus::new(1000, 42).map(|t| t.hand_count).sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn produces_more_than_one_tournament_for_a_large_total() {
        let tournaments: Vec<_> = SynthCorpus::new(1000, 42).collect();
        assert!(
            tournaments.len() > 1,
            "1000 mains devraient etre reparties sur plusieurs tournois (50-400/tournoi)"
        );
    }

    #[test]
    fn is_deterministic_for_a_given_seed() {
        let a: Vec<String> = SynthCorpus::new(500, 7).map(|t| t.content).collect();
        let b: Vec<String> = SynthCorpus::new(500, 7).map(|t| t.content).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn produces_a_single_tournament_when_total_hands_is_tiny() {
        let tournaments: Vec<_> = SynthCorpus::new(1, 3).collect();
        assert_eq!(tournaments.len(), 1);
        assert!(tournaments[0].hand_count >= 1);
    }

    /// CA de M2-4 au pied de la lettre : `just synth N=1000000` doit
    /// produire environ 1M de mains parsables a 100%. Ignore par defaut
    /// (generation + parsing de 1M de mains, plusieurs dizaines de
    /// secondes) : `cargo test -p gr-synth --release -- --ignored`.
    #[test]
    #[ignore = "genere et parse 1M de mains ; lancer explicitement avec --ignored"]
    fn one_million_synthetic_hands_are_100_percent_parsable() {
        use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

        let mut total = 0usize;
        let mut failures = 0usize;
        for tournament in SynthCorpus::new(1_000_000, 0xC0FF_EE00_1234_5678) {
            let (blocks, offset) = split_hand_blocks(&tournament.content);
            assert_eq!(offset, tournament.content.len());
            assert_eq!(blocks.len(), tournament.hand_count);
            for block in blocks {
                if WinamaxParser::parse_hand(block).is_err() {
                    failures += 1;
                }
            }
            total += tournament.hand_count;
        }

        assert_eq!(total, 1_000_000);
        assert_eq!(failures, 0);
    }
}
