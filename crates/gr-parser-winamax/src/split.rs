/// Separateur entre deux mains, et suffixe de fin de fichier (§3, PAR-3).
const HAND_SEPARATOR: &str = "\n\n\n";

/// Decoupe un fichier de mains en blocs **complets** (chacun suivi de `\n\n\n`),
/// et renvoie l'offset (en octets, dans `text`) ou reprendre une lecture
/// incrementale : juste apres le dernier separateur trouve, donc avant tout
/// bloc incomplet (main en cours d'ecriture, PAR-3).
///
/// `text` doit deja etre en fins de ligne LF (voir [`gr_parser_api::strip_bom`]
/// et la normalisation CRLF -> LF faite par l'appelant).
#[must_use]
pub fn split_hand_blocks(text: &str) -> (Vec<&str>, usize) {
    let mut blocks = Vec::new();
    let mut block_start = 0;
    let mut search_from = 0;

    while let Some(rel_idx) = text[search_from..].find(HAND_SEPARATOR) {
        let separator_start = search_from + rel_idx;
        let block = &text[block_start..separator_start];
        if !block.trim().is_empty() {
            blocks.push(block);
        }
        search_from = separator_start + HAND_SEPARATOR.len();
        block_start = search_from;
    }

    (blocks, block_start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn space_ko_fixture() -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/winamax/mtt/space-ko/20260925_VELOCITY - SPACE KO(1174075270)_real_holdem_no-limit.txt");
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn splits_a_complete_file_into_all_its_hands_with_no_remainder() {
        let text = space_ko_fixture();
        let (blocks, offset) = split_hand_blocks(&text);
        assert_eq!(blocks.len(), 117);
        assert_eq!(
            offset,
            text.len(),
            "a well-formed file leaves nothing after the last separator"
        );
        for block in &blocks {
            assert!(block.starts_with("Winamax Poker - "));
        }
    }

    #[test]
    fn truncating_after_the_last_separator_yields_the_same_complete_hands() {
        let text = space_ko_fixture();
        let (_, full_offset) = split_hand_blocks(&text);
        let truncated = &text[..full_offset];
        let (blocks, offset) = split_hand_blocks(truncated);
        assert_eq!(blocks.len(), 117);
        assert_eq!(offset, full_offset);
    }

    /// CA de M1-3 : une troncature a n'importe quelle ligne d'une main en cours
    /// d'ecriture ne doit jamais faire apparaitre cette main dans le resultat,
    /// et l'offset doit toujours designer son tout premier caractere.
    #[test]
    fn truncating_at_every_line_of_an_in_progress_hand_excludes_it() {
        let text = space_ko_fixture();
        let (all_blocks, _) = split_hand_blocks(&text);
        let first_hand = all_blocks[0];
        let second_hand = all_blocks[1];
        let second_hand_start = first_hand.len() + HAND_SEPARATOR.len();

        let lines: Vec<&str> = second_hand.lines().collect();
        assert!(
            lines.len() > 5,
            "the fixture's second hand should have several lines"
        );

        // Truncate after each line of the second hand except its very last one
        // (which would make it complete again).
        for up_to_line in 1..lines.len() {
            let truncated_second_hand_len: usize =
                lines[..up_to_line].iter().map(|l| l.len() + 1).sum(); // +1 pour le '\n'
            let truncated_len = second_hand_start + truncated_second_hand_len;
            let truncated_text = &text[..truncated_len];

            let (blocks, offset) = split_hand_blocks(truncated_text);
            assert_eq!(
                blocks,
                vec![first_hand],
                "truncated after line {up_to_line} of hand 2, only hand 1 should be complete"
            );
            assert_eq!(
                offset, second_hand_start,
                "truncated after line {up_to_line} of hand 2, offset should point right before it"
            );
        }
    }

    /// Cas reel le plus frequent : le fichier est tronque a la toute derniere main
    /// (Winamax est encore en train de l'ecrire).
    #[test]
    fn truncating_at_every_line_of_the_final_hand_excludes_only_it() {
        let text = space_ko_fixture();
        let (all_blocks, full_offset) = split_hand_blocks(&text);
        let last_hand = *all_blocks.last().unwrap();
        let last_hand_start = full_offset - last_hand.len() - HAND_SEPARATOR.len();
        assert_eq!(
            &text[last_hand_start..last_hand_start + last_hand.len()],
            last_hand
        );

        let lines: Vec<&str> = last_hand.lines().collect();
        for up_to_line in 1..lines.len() {
            let truncated_len: usize = lines[..up_to_line].iter().map(|l| l.len() + 1).sum();
            let truncated_text = &text[..last_hand_start + truncated_len];

            let (blocks, offset) = split_hand_blocks(truncated_text);
            assert_eq!(
                blocks.len(),
                all_blocks.len() - 1,
                "truncated after line {up_to_line} of the last hand"
            );
            assert_eq!(offset, last_hand_start);
        }
    }

    #[test]
    fn never_panics_on_empty_or_separator_only_input() {
        assert_eq!(split_hand_blocks(""), (vec![], 0));
        assert_eq!(split_hand_blocks("\n\n\n"), (vec![], 3));
        assert_eq!(split_hand_blocks("\n\n\n\n\n\n"), (vec![], 6));
    }
}
