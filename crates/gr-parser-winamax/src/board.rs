use gr_core::Card;
use gr_parser_api::{ParseError, ParseErrorCode};

/// Extrait toutes les cartes des groupes `[...]` d'une ligne de street (PAR-8).
/// `*** TURN *** [6h 5h Ts][7h]` a deux groupes ; les concatener donne
/// directement le board complet a jour, sans avoir a suivre un etat precedent.
pub(crate) fn parse_board_line(line: &str, line_no: usize) -> Result<Vec<Card>, ParseError> {
    let mut cards = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        let close = rest[open..].find(']').ok_or_else(|| ParseError {
            code: ParseErrorCode::UnknownLine,
            line_no,
            line: line.to_string(),
            context: "crochet fermant du board".to_string(),
        })?;
        let inner = &rest[open + 1..open + close];
        for token in inner.split_whitespace() {
            let card: Card = token.parse().map_err(|_| ParseError {
                code: ParseErrorCode::UnknownLine,
                line_no,
                line: line.to_string(),
                context: format!("carte invalide \"{token}\""),
            })?;
            cards.push(card);
        }
        rest = &rest[open + close + 1..];
    }
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flop_has_a_single_group() {
        let cards = parse_board_line("*** FLOP *** [6h 5h Ts]", 1).unwrap();
        assert_eq!(cards.len(), 3);
    }

    #[test]
    fn turn_concatenates_both_groups_into_four_cards() {
        let cards = parse_board_line("*** TURN *** [6h 5h Ts][7h]", 1).unwrap();
        assert_eq!(cards.len(), 4);
        assert_eq!(cards[3].to_string(), "7h");
    }

    #[test]
    fn river_concatenates_into_five_cards() {
        let cards = parse_board_line("*** RIVER *** [6h 5h Ts 7h][8c]", 1).unwrap();
        assert_eq!(cards.len(), 5);
        assert_eq!(cards[4].to_string(), "8c");
    }
}
