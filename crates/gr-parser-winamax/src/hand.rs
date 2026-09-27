use gr_core::{Chips, HandRecord, Money, SeatInfo};
use gr_parser_api::{ParseError, ParseErrorCode};

use crate::datetime::parse_utc_datetime;
use crate::money_format::parse_euros_to_cents;

fn err(line_no: usize, line: &str, context: &str) -> ParseError {
    ParseError {
        code: ParseErrorCode::UnknownLine,
        line_no,
        line: line.to_string(),
        context: context.to_string(),
    }
}

struct HeaderFields {
    tournament_name: String,
    room_hand_id: String,
    level: u32,
    ante: Chips,
    sb: Chips,
    bb: Chips,
    played_at: i64,
}

/// PAR-4 : `Winamax Poker - Tournament "<nom>" buyIn: ... level: N - HandId: #X-Y-Z - Holdem no limit (ante/SB/BB) - AAAA/MM/JJ HH:MM:SS UTC`.
fn parse_header_line(line: &str, line_no: usize) -> Result<HeaderFields, ParseError> {
    let rest = line
        .strip_prefix("Winamax Poker - Tournament \"")
        .ok_or_else(|| err(line_no, line, "prefixe d'en-tete attendu"))?;
    let (tournament_name, rest) = rest
        .split_once('"')
        .ok_or_else(|| err(line_no, line, "guillemet fermant du nom de tournoi"))?;

    let (_, rest) = rest
        .split_once("level: ")
        .ok_or_else(|| err(line_no, line, "marqueur \"level: \""))?;
    let (level_str, rest) = rest
        .split_once(" - HandId: ")
        .ok_or_else(|| err(line_no, line, "marqueur \"HandId: \""))?;
    let level: u32 = level_str
        .trim()
        .parse()
        .map_err(|_| err(line_no, line, "numero de level"))?;

    let (room_hand_id, rest) = rest
        .split_once(" - Holdem no limit (")
        .ok_or_else(|| err(line_no, line, "marqueur \"Holdem no limit (\""))?;

    let (blinds, rest) = rest
        .split_once(") - ")
        .ok_or_else(|| err(line_no, line, "parenthese fermante des blindes"))?;
    let mut blind_parts = blinds.split('/');
    let mut next_chips = |what: &'static str| -> Result<Chips, ParseError> {
        blind_parts
            .next()
            .ok_or_else(|| err(line_no, line, what))?
            .parse::<i64>()
            .map(Chips::from_i64)
            .map_err(|_| err(line_no, line, what))
    };
    let ante = next_chips("ante")?;
    let sb = next_chips("small blind")?;
    let bb = next_chips("big blind")?;

    let date_str = rest
        .strip_suffix(" UTC")
        .ok_or_else(|| err(line_no, line, "suffixe \" UTC\""))?;
    let played_at =
        parse_utc_datetime(date_str).ok_or_else(|| err(line_no, line, "date de la main"))?;

    Ok(HeaderFields {
        tournament_name: tournament_name.to_string(),
        room_hand_id: room_hand_id.to_string(),
        level,
        ante,
        sb,
        bb,
        played_at,
    })
}

struct TableFields {
    table_name: String,
    tournament_room_id: String,
    table_max_seats: u8,
    button_seat: u8,
}

/// PAR-5 : `Table: '<nom>(<ID>)#<table>' <N>-max (real money) Seat #<B> is the button`.
fn parse_table_line(line: &str, line_no: usize) -> Result<TableFields, ParseError> {
    let rest = line
        .strip_prefix("Table: '")
        .ok_or_else(|| err(line_no, line, "prefixe de la ligne de table"))?;
    let (table_name, rest) = rest
        .split_once('\'')
        .ok_or_else(|| err(line_no, line, "guillemet fermant du nom de table"))?;

    let open = table_name
        .rfind('(')
        .ok_or_else(|| err(line_no, line, "parenthese ouvrante de l'ID tournoi"))?;
    let close_rel = table_name[open..]
        .find(')')
        .ok_or_else(|| err(line_no, line, "parenthese fermante de l'ID tournoi"))?;
    let tournament_room_id = table_name[open + 1..open + close_rel].to_string();

    let (max_str, rest) = rest
        .trim_start()
        .split_once("-max")
        .ok_or_else(|| err(line_no, line, "suffixe \"-max\""))?;
    let table_max_seats: u8 = max_str
        .trim()
        .parse()
        .map_err(|_| err(line_no, line, "taille de table"))?;

    let (_, rest) = rest
        .split_once("Seat #")
        .ok_or_else(|| err(line_no, line, "marqueur \"Seat #\" du bouton"))?;
    let (button_str, _) = rest
        .split_once(' ')
        .ok_or_else(|| err(line_no, line, "numero de siege du bouton"))?;
    let button_seat: u8 = button_str
        .parse()
        .map_err(|_| err(line_no, line, "numero de siege du bouton"))?;

    Ok(TableFields {
        table_name: table_name.to_string(),
        tournament_room_id,
        table_max_seats,
        button_seat,
    })
}

/// PAR-6 : `Seat <N>: <pseudo> (<tapis>[, <bounty>€ bounty])`. Le pseudo peut
/// contenir espaces, points, tirets, underscores, chiffres et parentheses
/// (§8.3) : on cherche la derniere parenthese ouvrante, pas la premiere.
fn parse_seat_line(line: &str, line_no: usize) -> Result<SeatInfo, ParseError> {
    let rest = line
        .strip_prefix("Seat ")
        .ok_or_else(|| err(line_no, line, "prefixe \"Seat \""))?;
    let (seat_str, rest) = rest
        .split_once(": ")
        .ok_or_else(|| err(line_no, line, "separateur du numero de siege"))?;
    let seat: u8 = seat_str
        .parse()
        .map_err(|_| err(line_no, line, "numero de siege"))?;

    let open = rest
        .rfind('(')
        .ok_or_else(|| err(line_no, line, "parenthese ouvrante du tapis"))?;
    let pseudo = rest[..open].trim_end().to_string();
    let inner = rest[open + 1..]
        .strip_suffix(')')
        .ok_or_else(|| err(line_no, line, "parenthese fermante du tapis"))?;

    let (stack_str, bounty) = match inner.split_once(", ") {
        Some((stack_str, bounty_part)) => {
            let bounty_str = bounty_part
                .strip_suffix("\u{20ac} bounty")
                .ok_or_else(|| err(line_no, line, "suffixe de bounty"))?;
            let cents = parse_euros_to_cents(bounty_str)
                .ok_or_else(|| err(line_no, line, "montant du bounty"))?;
            (stack_str, Some(Money::from_cents(cents)))
        }
        None => (inner, None),
    };
    let starting_stack: i64 = stack_str
        .parse()
        .map_err(|_| err(line_no, line, "tapis de depart"))?;

    Ok(SeatInfo {
        seat,
        pseudo,
        starting_stack: Chips::from_i64(starting_stack),
        bounty,
        dealt_in: true,
    })
}

/// Parse l'en-tete, la table et les sieges d'une main (PAR-4/5/6). Les actions,
/// le board et les pots restent vides : ils arrivent en M1-5.
pub(crate) fn parse_hand(text: &str) -> Result<HandRecord, ParseError> {
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .enumerate()
        .map(|(i, l)| (i + 1, l));

    let (header_no, header_line) = lines.next().ok_or_else(|| err(0, "", "fichier vide"))?;
    let header = parse_header_line(header_line, header_no)?;

    let (table_no, table_line) = lines
        .next()
        .ok_or_else(|| err(header_no, header_line, "ligne de table manquante"))?;
    let table = parse_table_line(table_line, table_no)?;

    let mut seats = Vec::new();
    for (line_no, line) in lines {
        if !line.starts_with("Seat ") {
            break;
        }
        seats.push(parse_seat_line(line, line_no)?);
    }

    Ok(HandRecord {
        room_hand_id: header.room_hand_id,
        tournament_name: header.tournament_name,
        tournament_room_id: table.tournament_room_id,
        table_name: table.table_name,
        table_max_seats: table.table_max_seats,
        button_seat: table.button_seat,
        level: header.level,
        sb: header.sb,
        bb: header.bb,
        ante: header.ante,
        played_at: header.played_at,
        seats,
        actions: vec![],
        board: vec![],
        pots: vec![],
        total_pot: Chips::ZERO,
        rake: Chips::ZERO,
        hero_pseudo: None,
        hero_cards: None,
        parser_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBELISK_HAND: &str = "Winamax Poker - Tournament \"OBELISK - TRIDENT SPACE KO\" buyIn: 1.80\u{20ac} + 0.20\u{20ac} level: 1 - HandId: #5038051313141678275-16-1790176801 - Holdem no limit (3/10/20) - 2026/09/23 15:20:01 UTC\nTable: 'OBELISK - TRIDENT SPACE KO(1173012730)#0194' 3-max (real money) Seat #3 is the button\nSeat 1: AK-47-7.62mm (500, 1\u{20ac} bounty)\nSeat 2: CeBoLLuSSS (45494, 31.41\u{20ac} bounty)\nSeat 3: Hero (500)\n*** ANTE/BLINDS ***\n";

    #[test]
    fn parses_header_table_and_seats_of_a_real_hand() {
        let hand = parse_hand(OBELISK_HAND).unwrap();
        assert_eq!(hand.room_hand_id, "#5038051313141678275-16-1790176801");
        assert_eq!(hand.tournament_name, "OBELISK - TRIDENT SPACE KO");
        assert_eq!(hand.tournament_room_id, "1173012730");
        assert_eq!(hand.table_max_seats, 3);
        assert_eq!(hand.button_seat, 3);
        assert_eq!(hand.level, 1);
        assert_eq!(hand.ante, Chips::from_i64(3));
        assert_eq!(hand.sb, Chips::from_i64(10));
        assert_eq!(hand.bb, Chips::from_i64(20));
        assert_eq!(hand.played_at, 1_790_176_801_000);

        assert_eq!(hand.seats.len(), 3);
        assert_eq!(hand.seats[0].pseudo, "AK-47-7.62mm");
        assert_eq!(hand.seats[0].bounty, Some(Money::from_cents(100)));
        assert_eq!(hand.seats[1].pseudo, "CeBoLLuSSS");
        assert_eq!(hand.seats[1].bounty, Some(Money::from_cents(3141)));
        assert_eq!(hand.seats[2].pseudo, "Hero");
        assert_eq!(hand.seats[2].bounty, None);
        assert_eq!(hand.seats[2].starting_stack, Chips::from_i64(500));
    }

    #[test]
    fn seat_pseudo_can_contain_spaces_dots_dashes_and_underscores() {
        let seat =
            parse_seat_line("Seat 2: Marie-Claire.99_x (20000, 1\u{20ac} bounty)", 3).unwrap();
        assert_eq!(seat.pseudo, "Marie-Claire.99_x");
        let seat = parse_seat_line("Seat 1: Jean Dupont (20000)", 3).unwrap();
        assert_eq!(seat.pseudo, "Jean Dupont");
    }
}
