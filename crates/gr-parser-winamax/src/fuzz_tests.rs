//! Fuzz leger (PAR-15/R-NOPANIC) : quelle que soit l'entree, le parser ne doit
//! **jamais** paniquer — juste renvoyer `Ok` ou une `ParseError`. Deux familles
//! de proprietes : entree totalement arbitraire, et mutation d'une main reelle
//! valide (bien plus susceptible de percer loin dans le parsing avant d'echouer).

use std::path::Path;

use proptest::prelude::*;

use crate::{split_hand_blocks, WinamaxParser};

fn obelisk_hand() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
    );
    let text = std::fs::read_to_string(path).unwrap();
    let (blocks, _offset) = split_hand_blocks(&text);
    blocks[0].to_string()
}

fn obelisk_summary() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit_summary.txt",
    );
    std::fs::read_to_string(path).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// N'importe quels octets, passes a `detect` : jamais de panique.
    #[test]
    fn detect_never_panics_on_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let _ = WinamaxParser::detect(&bytes);
    }

    /// N'importe quelle chaine, passee a `parse_hand` : jamais de panique.
    #[test]
    fn parse_hand_never_panics_on_arbitrary_strings(s in ".{0,2000}") {
        let _ = WinamaxParser::parse_hand(&s);
    }

    /// N'importe quelle chaine, passee a `parse_summary` : jamais de panique.
    #[test]
    fn parse_summary_never_panics_on_arbitrary_strings(s in ".{0,2000}") {
        let _ = WinamaxParser::parse_summary(&s);
    }

    /// N'importe quelle chaine, passee au decoupeur de mains : jamais de panique.
    #[test]
    fn split_hand_blocks_never_panics_on_arbitrary_strings(s in ".{0,2000}") {
        let _ = split_hand_blocks(&s);
    }

    /// Troncature d'une vraie main a n'importe quel octet : jamais de panique
    /// (complement du test cible de M1-3, mais sur *toute* la main, pas
    /// seulement les frontieres de ligne).
    #[test]
    fn parse_hand_never_panics_on_a_truncated_real_hand(cut in 0usize..obelisk_hand().len()) {
        let hand = obelisk_hand();
        // `cut` peut tomber au milieu d'un caractere multi-octets (le `€` du
        // buyIn) : on recule jusqu'a la frontiere UTF-8 valide precedente.
        let mut cut = cut;
        while cut > 0 && !hand.is_char_boundary(cut) {
            cut -= 1;
        }
        let _ = WinamaxParser::parse_hand(&hand[..cut]);
    }

    /// Meme principe sur le fichier summary.
    #[test]
    fn parse_summary_never_panics_on_a_truncated_real_summary(cut in 0usize..obelisk_summary().len()) {
        let summary = obelisk_summary();
        let mut cut = cut;
        while cut > 0 && !summary.is_char_boundary(cut) {
            cut -= 1;
        }
        let _ = WinamaxParser::parse_summary(&summary[..cut]);
    }

    /// Remplace un caractere ASCII au hasard d'une vraie main par un autre
    /// caractere arbitraire (accent, chiffre, ponctuation...) : jamais de panique.
    #[test]
    fn parse_hand_never_panics_on_a_corrupted_real_hand(idx in 0usize..200, replacement in any::<char>()) {
        let hand = obelisk_hand();
        let mut chars: Vec<char> = hand.chars().collect();
        if !chars.is_empty() {
            let target = idx % chars.len();
            chars[target] = replacement;
        }
        let corrupted: String = chars.into_iter().collect();
        let _ = WinamaxParser::parse_hand(&corrupted);
    }
}
