use std::collections::HashMap;

use gr_core::{
    ActionKind, ActionRecord, Card, Chips, HandRecord, Money, PotKind, PotResult, SeatInfo, Street,
};
use gr_parser_api::{ParseError, ParseErrorCode};

use crate::board::parse_board_line;
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

fn chip_mismatch(context: String) -> ParseError {
    ParseError {
        code: ParseErrorCode::ChipMismatch,
        line_no: 0,
        line: String::new(),
        context,
    }
}

fn parse_chips(s: &str, line: &str, line_no: usize, what: &str) -> Result<Chips, ParseError> {
    s.parse::<i64>()
        .map(Chips::from_i64)
        .map_err(|_| err(line_no, line, what))
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
        dealt_in: false,
    })
}

fn parse_seats(lines: &[(usize, &str)], idx: &mut usize) -> Result<Vec<SeatInfo>, ParseError> {
    let mut seats = Vec::new();
    while let Some(&(line_no, line)) = lines.get(*idx) {
        if !line.starts_with("Seat ") {
            break;
        }
        seats.push(parse_seat_line(line, line_no)?);
        *idx += 1;
    }
    Ok(seats)
}

fn add_contrib(map: &mut HashMap<String, Chips>, pseudo: &str, amount: Chips) {
    let entry = map.entry(pseudo.to_string()).or_insert(Chips::ZERO);
    *entry = *entry + amount;
}

fn base_action(street: Street, is_all_in: bool, kind: ActionKind, pseudo: &str) -> ActionRecord {
    ActionRecord {
        street,
        pseudo: pseudo.to_string(),
        kind,
        amount: None,
        to_amount: None,
        is_all_in,
        pot: None,
        shown_cards: None,
        shown_label: None,
    }
}

/// Verbes de mise simple : `<marker><montant>[ and is all-in]`. Toutes partagent
/// la meme mecanique (montant ajoute litteralement, cumule dans les deux
/// tables de contributions) ; seule l'ante ne compte pas dans la street (§4.1).
const WAGER_VERBS: &[(&str, ActionKind, bool)] = &[
    (" posts ante ", ActionKind::PostAnte, false),
    (" posts small blind ", ActionKind::PostSmallBlind, true),
    (" posts big blind ", ActionKind::PostBigBlind, true),
    (" calls ", ActionKind::Call, true),
    (" bets ", ActionKind::Bet, true),
];

fn try_parse_wager(
    line: &str,
    line_no: usize,
    street_contrib: &mut HashMap<String, Chips>,
    total_contrib: &mut HashMap<String, Chips>,
) -> Option<Result<(ActionKind, String, Chips), ParseError>> {
    for &(marker, kind, counts_toward_street) in WAGER_VERBS {
        if let Some(idx) = line.rfind(marker) {
            let pseudo = &line[..idx];
            let amount_str = &line[idx + marker.len()..];
            return Some(
                parse_chips(amount_str, line, line_no, marker.trim()).map(|amount| {
                    add_contrib(total_contrib, pseudo, amount);
                    if counts_toward_street {
                        add_contrib(street_contrib, pseudo, amount);
                    }
                    (kind, pseudo.to_string(), amount)
                }),
            );
        }
    }
    None
}

/// `P raises X to Y` : X est relatif a la mise de table courante (pas au deja-mise
/// du joueur), donc inutilise ; l'ajout reel = `Y - street_contrib[pseudo]` (§8.2-7).
fn parse_raise(
    line: &str,
    line_no: usize,
    idx: usize,
    marker_len: usize,
    street_contrib: &mut HashMap<String, Chips>,
    total_contrib: &mut HashMap<String, Chips>,
) -> Result<(String, Chips, Chips), ParseError> {
    let pseudo = &line[..idx];
    let rest = &line[idx + marker_len..];
    let (_, to_str) = rest
        .split_once(" to ")
        .ok_or_else(|| err(line_no, line, "marqueur \" to \" de la relance"))?;
    let to_amount = parse_chips(to_str, line, line_no, "montant total de la relance")?;
    let previous = street_contrib.get(pseudo).copied().unwrap_or(Chips::ZERO);
    let added = to_amount - previous;
    add_contrib(total_contrib, pseudo, added);
    street_contrib.insert(pseudo.to_string(), to_amount);
    Ok((pseudo.to_string(), added, to_amount))
}

fn parse_shows(
    line: &str,
    line_no: usize,
    idx: usize,
    marker_len: usize,
) -> Result<(String, Vec<Card>, Option<String>), ParseError> {
    let pseudo = &line[..idx];
    let rest = &line[idx + marker_len..];
    let (cards_str, rest) = rest
        .split_once(']')
        .ok_or_else(|| err(line_no, line, "crochet fermant des cartes montrees"))?;
    let cards = cards_str
        .split_whitespace()
        .map(|t| {
            t.parse::<Card>()
                .map_err(|_| err(line_no, line, "carte montree invalide"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let label = rest
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .map(str::to_string);
    Ok((pseudo.to_string(), cards, label))
}

fn parse_collected(
    line: &str,
    line_no: usize,
    idx: usize,
    marker_len: usize,
) -> Result<(String, Chips, PotKind), ParseError> {
    let pseudo = &line[..idx];
    let rest = &line[idx + marker_len..];
    let (amount_str, pot_str) = rest
        .split_once(" from ")
        .ok_or_else(|| err(line_no, line, "marqueur \" from \" du pot"))?;
    let amount = parse_chips(amount_str, line, line_no, "montant collecte")?;
    let pot = match pot_str {
        "pot" => PotKind::Pot,
        "main pot" => PotKind::Main,
        other => {
            let k = other
                .strip_prefix("side pot ")
                .ok_or_else(|| err(line_no, line, "type de pot inconnu"))?;
            PotKind::Side(
                k.parse()
                    .map_err(|_| err(line_no, line, "numero de side pot"))?,
            )
        }
    };
    Ok((pseudo.to_string(), amount, pot))
}

/// Parse une ligne d'action (PAR-7), en tenant a jour les mises de la street
/// en cours et les contributions totales de la main (PAR-11).
fn parse_action_line(
    line: &str,
    line_no: usize,
    street: Street,
    street_contrib: &mut HashMap<String, Chips>,
    total_contrib: &mut HashMap<String, Chips>,
) -> Result<ActionRecord, ParseError> {
    let (line, is_all_in) = match line.strip_suffix(" and is all-in") {
        Some(rest) => (rest, true),
        None => (line, false),
    };

    if let Some(pseudo) = line.strip_suffix(" folds") {
        return Ok(base_action(street, is_all_in, ActionKind::Fold, pseudo));
    }
    if let Some(pseudo) = line.strip_suffix(" checks") {
        return Ok(base_action(street, is_all_in, ActionKind::Check, pseudo));
    }
    if let Some(result) = try_parse_wager(line, line_no, street_contrib, total_contrib) {
        let (kind, pseudo, amount) = result?;
        let mut a = base_action(street, is_all_in, kind, &pseudo);
        a.amount = Some(amount);
        return Ok(a);
    }
    if let Some(idx) = line.rfind(" raises ") {
        let (pseudo, added, to_amount) = parse_raise(
            line,
            line_no,
            idx,
            " raises ".len(),
            street_contrib,
            total_contrib,
        )?;
        let mut a = base_action(street, is_all_in, ActionKind::Raise, &pseudo);
        a.amount = Some(added);
        a.to_amount = Some(to_amount);
        return Ok(a);
    }
    if let Some(idx) = line.rfind(" shows [") {
        let (pseudo, cards, label) = parse_shows(line, line_no, idx, " shows [".len())?;
        let mut a = base_action(street, is_all_in, ActionKind::Shows, &pseudo);
        a.shown_cards = Some(cards);
        a.shown_label = label;
        return Ok(a);
    }
    if let Some(idx) = line.rfind(" collected ") {
        let (pseudo, amount, pot) = parse_collected(line, line_no, idx, " collected ".len())?;
        let mut a = base_action(street, is_all_in, ActionKind::Collected, &pseudo);
        a.amount = Some(amount);
        a.pot = Some(pot);
        return Ok(a);
    }

    Err(err(line_no, line, "aucun motif d'action connu"))
}

fn parse_dealt_to(line: &str, line_no: usize) -> Result<(String, (Card, Card)), ParseError> {
    let rest = line
        .strip_prefix("Dealt to ")
        .ok_or_else(|| err(line_no, line, "prefixe \"Dealt to \""))?;
    let (pseudo, cards_part) = rest
        .split_once(" [")
        .ok_or_else(|| err(line_no, line, "crochet des cartes du Hero"))?;
    let cards_str = cards_part
        .strip_suffix(']')
        .ok_or_else(|| err(line_no, line, "crochet fermant des cartes du Hero"))?;
    let mut it = cards_str.split_whitespace();
    let c1: Card = it
        .next()
        .ok_or_else(|| err(line_no, line, "premiere carte du Hero"))?
        .parse()
        .map_err(|_| err(line_no, line, "premiere carte du Hero"))?;
    let c2: Card = it
        .next()
        .ok_or_else(|| err(line_no, line, "deuxieme carte du Hero"))?
        .parse()
        .map_err(|_| err(line_no, line, "deuxieme carte du Hero"))?;
    Ok((pseudo.to_string(), (c1, c2)))
}

/// Une section `*** ... ***` du corps de la main (PAR-8). `Preflop` couvre a
/// la fois `ANTE/BLINDS` et `PRE-FLOP` : les blindes deja postees restent
/// comptees dans la street courante, contrairement aux streets suivantes qui
/// ouvrent un nouveau tour de mises (`PostflopStreet` vide `street_contrib`).
enum Section {
    Preflop,
    PostflopStreet(Street, Vec<Card>),
    ShowDown,
    Summary,
}

fn match_section(line: &str, line_no: usize) -> Result<Option<Section>, ParseError> {
    Ok(Some(match line {
        "*** ANTE/BLINDS ***" => Section::Preflop,
        "*** SHOW DOWN ***" => Section::ShowDown,
        "*** SUMMARY ***" => Section::Summary,
        _ if line.starts_with("*** PRE-FLOP ***") => Section::Preflop,
        _ if line.starts_with("*** FLOP ***") => {
            Section::PostflopStreet(Street::Flop, parse_board_line(line, line_no)?)
        }
        _ if line.starts_with("*** TURN ***") => {
            Section::PostflopStreet(Street::Turn, parse_board_line(line, line_no)?)
        }
        _ if line.starts_with("*** RIVER ***") => {
            Section::PostflopStreet(Street::River, parse_board_line(line, line_no)?)
        }
        _ => return Ok(None),
    }))
}

/// `Total pot N | No rake` ou `Total pot N | K rake` (jamais observe non-nul
/// dans le corpus : les MTT Winamax prelevent le rake via le buy-in, pas la main).
fn parse_total_pot_line(line: &str, line_no: usize) -> Result<(Chips, Chips), ParseError> {
    let rest = line
        .strip_prefix("Total pot ")
        .ok_or_else(|| err(line_no, line, "prefixe \"Total pot \""))?;
    let (pot_str, rake_str) = rest
        .split_once(" | ")
        .ok_or_else(|| err(line_no, line, "separateur du rake"))?;
    let pot = parse_chips(pot_str, line, line_no, "montant du pot total")?;
    let rake = if rake_str == "No rake" {
        Chips::ZERO
    } else {
        let n = rake_str
            .strip_suffix(" rake")
            .ok_or_else(|| err(line_no, line, "suffixe de rake"))?;
        parse_chips(n, line, line_no, "montant du rake")?
    };
    Ok((pot, rake))
}

fn record_collected(pots: &mut Vec<PotResult>, pseudo: &str, amount: Chips, pot_kind: PotKind) {
    match pots.iter_mut().find(|p| p.pot == pot_kind) {
        Some(p) => {
            p.amount = p.amount + amount;
            p.winners.push((pseudo.to_string(), amount));
        }
        None => pots.push(PotResult {
            pot: pot_kind,
            amount,
            winners: vec![(pseudo.to_string(), amount)],
        }),
    }
}

/// PAR-11 + §4.6 : verifie Σcontributions = Σcollected = Total pot, et calcule
/// l'excedent non suivi comme l'ecart entre les deux plus fortes contributions
/// totales — une formule arithmetique generale (valable pour tout nombre de
/// side pots) qui ne necessite pas de detecter le texte "side pot".
fn check_conservation_and_uncalled_excess(
    total_contrib: &HashMap<String, Chips>,
    pots: &[PotResult],
    total_pot: Chips,
) -> Result<Chips, ParseError> {
    let sum_contrib: Chips = total_contrib.values().copied().sum();
    let sum_collected: Chips = pots.iter().map(|p| p.amount).sum();
    if sum_contrib != total_pot || sum_collected != total_pot {
        return Err(chip_mismatch(format!(
            "contributions={} collecte={} total_pot={} (PAR-11)",
            sum_contrib.amount(),
            sum_collected.amount(),
            total_pot.amount()
        )));
    }

    let mut contribs: Vec<Chips> = total_contrib.values().copied().collect();
    contribs.sort_unstable_by(|a, b| b.cmp(a));
    Ok(match (contribs.first(), contribs.get(1)) {
        (Some(&max), Some(&second)) => max - second,
        _ => Chips::ZERO,
    })
}

fn parse_prelude(
    lines: &[(usize, &str)],
    idx: &mut usize,
) -> Result<(HeaderFields, TableFields, Vec<SeatInfo>), ParseError> {
    let (header_no, header_line) = *lines.get(*idx).ok_or_else(|| err(0, "", "fichier vide"))?;
    let header = parse_header_line(header_line, header_no)?;
    *idx += 1;

    let (table_no, table_line) = *lines
        .get(*idx)
        .ok_or_else(|| err(header_no, header_line, "ligne de table manquante"))?;
    let table = parse_table_line(table_line, table_no)?;
    *idx += 1;

    let seats = parse_seats(lines, idx)?;
    Ok((header, table, seats))
}

/// Parse une main complete (PAR-4 a PAR-11) : en-tete, table, sieges, actions,
/// streets, board, pots.
pub(crate) fn parse_hand(text: &str) -> Result<HandRecord, ParseError> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .map(str::trim_end)
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .collect();
    let mut idx = 0;

    let (header, table, mut seats) = parse_prelude(&lines, &mut idx)?;
    let mut dealt_in: HashMap<String, bool> =
        seats.iter().map(|s| (s.pseudo.clone(), false)).collect();

    let mut street = Street::Preflop;
    let mut street_contrib: HashMap<String, Chips> = HashMap::new();
    let mut total_contrib: HashMap<String, Chips> = HashMap::new();
    let mut actions: Vec<ActionRecord> = Vec::new();
    let mut board: Vec<Card> = Vec::new();
    let mut pots: Vec<PotResult> = Vec::new();
    let mut hero_pseudo: Option<String> = None;
    let mut hero_cards: Option<(Card, Card)> = None;
    let mut total_pot: Option<Chips> = None;
    let mut rake = Chips::ZERO;
    let mut in_summary = false;

    while let Some(&(line_no, line)) = lines.get(idx) {
        idx += 1;
        if line.is_empty() {
            continue;
        }

        if in_summary {
            if line.starts_with("Total pot ") {
                let (pot, hand_rake) = parse_total_pot_line(line, line_no)?;
                total_pot = Some(pot);
                rake = hand_rake;
            }
            continue;
        }

        if let Some(section) = match_section(line, line_no)? {
            match section {
                Section::Preflop => street = Street::Preflop,
                Section::ShowDown => street = Street::Showdown,
                Section::Summary => in_summary = true,
                Section::PostflopStreet(s, new_board) => {
                    street = s;
                    street_contrib.clear();
                    board = new_board;
                }
            }
            continue;
        }

        if line.starts_with("Dealt to ") {
            let (pseudo, cards) = parse_dealt_to(line, line_no)?;
            dealt_in.insert(pseudo.clone(), true);
            hero_pseudo = Some(pseudo);
            hero_cards = Some(cards);
            continue;
        }

        let action = parse_action_line(
            line,
            line_no,
            street,
            &mut street_contrib,
            &mut total_contrib,
        )?;
        dealt_in.insert(action.pseudo.clone(), true);
        // R-NOPANIC : pas d'expect() ici meme si parse_action_line garantit
        // toujours amount+pot pour un Collected — on ignore silencieusement
        // plutot que de paniquer si cette garantie venait a etre brisee.
        if let (ActionKind::Collected, Some(amount), Some(pot)) =
            (action.kind, action.amount, action.pot)
        {
            record_collected(&mut pots, &action.pseudo, amount, pot);
        }
        actions.push(action);
    }

    let total_pot = total_pot.ok_or_else(|| err(0, "", "ligne \"Total pot\" manquante"))?;
    let uncalled_excess = check_conservation_and_uncalled_excess(&total_contrib, &pots, total_pot)?;

    for seat in &mut seats {
        seat.dealt_in = dealt_in.get(&seat.pseudo).copied().unwrap_or(false);
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
        actions,
        board,
        pots,
        total_pot,
        rake,
        uncalled_excess,
        hero_pseudo,
        hero_cards,
        parser_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBELISK_HAND_1: &str = "Winamax Poker - Tournament \"OBELISK - TRIDENT SPACE KO\" buyIn: 1.80\u{20ac} + 0.20\u{20ac} level: 1 - HandId: #5038051313141678275-1-1790176555 - Holdem no limit (3/10/20) - 2026/09/23 15:15:55 UTC\nTable: 'OBELISK - TRIDENT SPACE KO(1173012730)#0194' 3-max (real money) Seat #3 is the button\nSeat 1: P0001 (500, 1\u{20ac} bounty)\nSeat 2: P0002 (477, 1\u{20ac} bounty)\nSeat 3: Hero (500, 1\u{20ac} bounty)\n*** ANTE/BLINDS ***\nHero posts ante 3\nP0002 posts ante 3\nHero posts small blind 10\nP0002 posts big blind 20\nDealt to Hero [8s Ad]\n*** PRE-FLOP *** \nHero raises 20 to 40\nP0002 calls 20\n*** FLOP *** [6h 5h Ts]\nP0002 checks\nHero checks\n*** TURN *** [6h 5h Ts][7h]\nP0002 checks\nHero checks\n*** RIVER *** [6h 5h Ts 7h][8c]\nP0002 bets 56\nHero calls 56\n*** SHOW DOWN ***\nHero shows [8s Ad] (One pair : 8)\nP0002 shows [3s 9s] (Straight Ten high)\nP0002 collected 198 from pot\n*** SUMMARY ***\nTotal pot 198 | No rake\nBoard: [6h 5h Ts 7h 8c]\nSeat 2: P0002 (big blind) showed [3s 9s] and won 198 with Straight Ten high\nSeat 3: Hero (small blind) (button) showed [8s Ad] and lost with One pair : 8\n";

    const MULTIWAY_ALLIN_HAND: &str = "Winamax Poker - Tournament \"VELOCITY\" buyIn: 1.80\u{20ac} + 0.20\u{20ac} level: 1 - HandId: #1-1-1 - Holdem no limit (40/175/350) - 2026/09/25 00:00:00 UTC\nTable: 'VELOCITY(1)#1' 6-max (real money) Seat #1 is the button\nSeat 1: Hero (20000)\nSeat 2: P0007 (20000)\nSeat 3: P0008 (20000)\nSeat 4: P0009 (20000)\nSeat 5: P0010 (20000)\nSeat 6: P0003 (20000)\n*** ANTE/BLINDS ***\nP0007 posts ante 40\nP0008 posts ante 40\nHero posts ante 40\nP0010 posts ante 40\nP0009 posts ante 40\nP0003 posts ante 40\nP0007 posts small blind 175\nP0008 posts big blind 350\nDealt to Hero [3h Th]\n*** PRE-FLOP *** \nHero folds\nP0010 folds\nP0009 raises 350 to 700\nP0003 raises 1400 to 2100\nP0007 folds\nP0008 folds\nP0009 raises 18220 to 20320 and is all-in\nP0003 calls 15025 and is all-in\n*** FLOP *** [9c 9h 4h]\n*** TURN *** [9c 9h 4h][5d]\n*** RIVER *** [9c 9h 4h 5d][8h]\n*** SHOW DOWN ***\nP0009 shows [Kd Qd] (One pair : 9)\nP0003 shows [Ad Qc] (One pair : 9)\nP0003 collected 35015 from main pot\nP0009 collected 3195 from side pot 1\n*** SUMMARY ***\nTotal pot 38210 | No rake\nBoard: [9c 9h 4h 5d 8h]\n";

    #[test]
    fn parses_a_full_showdown_hand() {
        let hand = parse_hand(OBELISK_HAND_1).unwrap();
        assert_eq!(
            hand.board
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["6h", "5h", "Ts", "7h", "8c"]
        );
        assert_eq!(hand.total_pot, Chips::from_i64(198));
        assert_eq!(hand.rake, Chips::ZERO);
        assert_eq!(hand.uncalled_excess, Chips::ZERO);
        assert_eq!(hand.hero_pseudo, Some("Hero".to_string()));
        assert_eq!(hand.hero_cards.unwrap().0.to_string(), "8s");

        assert_eq!(hand.pots.len(), 1);
        assert_eq!(hand.pots[0].pot, PotKind::Pot);
        assert_eq!(hand.pots[0].amount, Chips::from_i64(198));
        assert_eq!(
            hand.pots[0].winners,
            vec![("P0002".to_string(), Chips::from_i64(198))]
        );

        // Seat 1 (P0001) est assis mais ne poste rien et n'agit jamais : exclu (PAR-6).
        let p0001 = hand.seats.iter().find(|s| s.pseudo == "P0001").unwrap();
        assert!(!p0001.dealt_in);
        assert!(
            hand.seats
                .iter()
                .find(|s| s.pseudo == "Hero")
                .unwrap()
                .dealt_in
        );
        assert!(
            hand.seats
                .iter()
                .find(|s| s.pseudo == "P0002")
                .unwrap()
                .dealt_in
        );

        let raise = hand
            .actions
            .iter()
            .find(|a| a.kind == ActionKind::Raise)
            .unwrap();
        assert_eq!(raise.pseudo, "Hero");
        assert_eq!(raise.amount, Some(Chips::from_i64(30))); // 40 - 10 (small blind deja postee)
        assert_eq!(raise.to_amount, Some(Chips::from_i64(40)));
    }

    #[test]
    fn excludes_seated_but_undealt_players() {
        // P0001 (Seat 1) est assis mais absent de tout posts/action : exclu (PAR-6).
        let hand = parse_hand(OBELISK_HAND_1).unwrap();
        assert_eq!(hand.seats.iter().filter(|s| s.dealt_in).count(), 2);
    }

    #[test]
    fn computes_uncalled_excess_from_a_multiway_all_in_side_pot() {
        let hand = parse_hand(MULTIWAY_ALLIN_HAND).unwrap();
        assert_eq!(hand.total_pot, Chips::from_i64(38210));
        assert_eq!(hand.uncalled_excess, Chips::from_i64(3195));

        assert_eq!(hand.pots.len(), 2);
        let main = hand.pots.iter().find(|p| p.pot == PotKind::Main).unwrap();
        assert_eq!(main.amount, Chips::from_i64(35015));
        let side = hand
            .pots
            .iter()
            .find(|p| p.pot == PotKind::Side(1))
            .unwrap();
        assert_eq!(side.amount, Chips::from_i64(3195));
        assert_eq!(
            side.winners,
            vec![("P0009".to_string(), Chips::from_i64(3195))]
        );
    }

    #[test]
    fn rejects_a_chip_mismatch() {
        let broken = OBELISK_HAND_1.replace("Total pot 198 | No rake", "Total pot 199 | No rake");
        let error = parse_hand(&broken).unwrap_err();
        assert_eq!(error.code, ParseErrorCode::ChipMismatch);
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
