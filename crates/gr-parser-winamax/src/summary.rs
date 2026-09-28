use gr_core::{Money, TournamentBullet, TournamentSummary};
use gr_parser_api::{ParseError, ParseErrorCode};

use crate::money_format::{parse_euros_to_cents, strip_euro_suffix};

fn err(line_no: usize, line: &str, context: &str) -> ParseError {
    ParseError {
        code: ParseErrorCode::UnknownLine,
        line_no,
        line: line.to_string(),
        context: context.to_string(),
    }
}

fn parse_money_token(s: &str, line: &str, line_no: usize) -> Result<Money, ParseError> {
    let s = strip_euro_suffix(s).ok_or_else(|| err(line_no, line, "suffixe euro attendu"))?;
    parse_euros_to_cents(s)
        .map(Money::from_cents)
        .ok_or_else(|| err(line_no, line, "montant en euros"))
}

/// Decoupe le fichier en blocs (un par entree, §5.1), separes par une ligne
/// vide, en conservant le numero de ligne absolu de chaque ligne (erreurs PAR-15).
fn split_summary_blocks(text: &str) -> Vec<Vec<(usize, &str)>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push((i + 1, trimmed));
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

/// `Buy-In : P€ + B€ + F€` (KO) ou `P€ + F€` (non-KO/freeroll, bounty = 0).
fn parse_buyin_line(line: &str, line_no: usize) -> Result<(Money, Money, Money), ParseError> {
    let rest = line
        .strip_prefix("Buy-In : ")
        .ok_or_else(|| err(line_no, line, "prefixe \"Buy-In : \""))?;
    match rest.split(" + ").collect::<Vec<_>>().as_slice() {
        [prize, bounty, fee] => Ok((
            parse_money_token(prize, line, line_no)?,
            parse_money_token(bounty, line, line_no)?,
            parse_money_token(fee, line, line_no)?,
        )),
        [prize, fee] => Ok((
            parse_money_token(prize, line, line_no)?,
            Money::ZERO,
            parse_money_token(fee, line, line_no)?,
        )),
        _ => Err(err(
            line_no,
            line,
            "nombre de composantes du buy-in inattendu",
        )),
    }
}

/// `Xh `, `Ymin ` et/ou `Zs`, dans cet ordre, tout sous-ensemble etant possible
/// (`"5min 27s"`, `"1h 36min 43s"`, `"1min 0s"`).
fn parse_duration_seconds(s: &str, line: &str, line_no: usize) -> Result<u32, ParseError> {
    // Arithmetique saturante (R-NOPANIC) : une duree fuzzee/adversariale ne
    // doit jamais faire deborder le u32, meme si elle n'a aucun sens reel.
    let mut seconds: u32 = 0;
    let mut rest = s.trim();
    if let Some(idx) = rest.find('h') {
        let hours: u32 = rest[..idx]
            .trim()
            .parse()
            .map_err(|_| err(line_no, line, "heures de duree"))?;
        seconds = seconds.saturating_add(hours.saturating_mul(3600));
        rest = rest[idx + 1..].trim();
    }
    if let Some(idx) = rest.find("min") {
        let minutes: u32 = rest[..idx]
            .trim()
            .parse()
            .map_err(|_| err(line_no, line, "minutes de duree"))?;
        seconds = seconds.saturating_add(minutes.saturating_mul(60));
        rest = rest[idx + 3..].trim();
    }
    if let Some(idx) = rest.find('s') {
        let secs: u32 = rest[..idx]
            .trim()
            .parse()
            .map_err(|_| err(line_no, line, "secondes de duree"))?;
        seconds = seconds.saturating_add(secs);
    }
    Ok(seconds)
}

/// `You finished in <N>th place` : le suffixe ordinal (`"th"`, meme pour 7) est ignore.
fn parse_finish_position(line: &str, line_no: usize) -> Result<u32, ParseError> {
    let rest = line
        .strip_prefix("You finished in ")
        .ok_or_else(|| err(line_no, line, "prefixe \"You finished in \""))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse()
        .map_err(|_| err(line_no, line, "numero de place"))
}

/// `You won X€ + Bounty Y€` / `You won Bounty Y€` / `You won X€` (§5.2 : les
/// trois formes sont observees selon KO/ITM).
fn parse_won_line(line: &str, line_no: usize) -> Result<(Money, Money), ParseError> {
    let rest = line
        .strip_prefix("You won ")
        .ok_or_else(|| err(line_no, line, "prefixe \"You won \""))?;
    if let Some(bounty_str) = rest.strip_prefix("Bounty ") {
        return Ok((Money::ZERO, parse_money_token(bounty_str, line, line_no)?));
    }
    if let Some((prize_str, bounty_str)) = rest.split_once(" + Bounty ") {
        return Ok((
            parse_money_token(prize_str, line, line_no)?,
            parse_money_token(bounty_str, line, line_no)?,
        ));
    }
    Ok((parse_money_token(rest, line, line_no)?, Money::ZERO))
}

struct HeaderFields {
    room_tournament_id: String,
    name: String,
    hero_pseudo: String,
    buyin_prize: Money,
    buyin_bounty: Money,
    buyin_fee: Money,
    mode: String,
    tournament_type: String,
    speed: String,
    flight_id: u32,
}

fn expect_line<'a>(
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
    prev_no: usize,
    what: &str,
) -> Result<(usize, &'a str), ParseError> {
    lines.next().ok_or_else(|| err(prev_no, "", what))
}

fn parse_prefixed<'a>(line: &'a str, line_no: usize, prefix: &str) -> Result<&'a str, ParseError> {
    line.strip_prefix(prefix)
        .ok_or_else(|| err(line_no, line, prefix))
}

/// Un bloc = une entree (§5.1, PAR-12). `entry_no` est la position du bloc
/// dans le fichier (1-based, ordre chronologique) ; le rattachement aux mains
/// du fichier de mains (reapparition du Hero apres elimination) est fait a
/// l'ingestion (M2-6), pas ici.
fn parse_summary_block(
    block: &[(usize, &str)],
    entry_no: u32,
) -> Result<(HeaderFields, TournamentBullet), ParseError> {
    let mut lines = block.iter().copied();
    let (line_no, first) = lines
        .next()
        .ok_or_else(|| err(0, "", "bloc de summary vide"))?;

    let rest = parse_prefixed(first, line_no, "Winamax Poker - Tournament summary : ")?;
    let (late_reg_bust, rest) = match rest.strip_suffix(" - Late Registration") {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let open = rest
        .rfind('(')
        .ok_or_else(|| err(line_no, first, "parenthese ouvrante de l'ID tournoi"))?;
    let close_rel = rest[open..]
        .find(')')
        .ok_or_else(|| err(line_no, first, "parenthese fermante de l'ID tournoi"))?;
    let name = rest[..open].to_string();
    let room_tournament_id = rest[open + 1..open + close_rel].to_string();

    let (line_no, player_line) = expect_line(&mut lines, line_no, "ligne \"Player : \" manquante")?;
    let hero_pseudo = parse_prefixed(player_line, line_no, "Player : ")?.to_string();

    let (line_no, buyin_line) = expect_line(&mut lines, line_no, "ligne \"Buy-In\" manquante")?;
    let (buyin_prize, buyin_bounty, buyin_fee) = parse_buyin_line(buyin_line, line_no)?;

    let (line_no, registered_line) = expect_line(
        &mut lines,
        line_no,
        "ligne \"Registered players\" manquante",
    )?;
    let registered_snapshot: u32 =
        parse_prefixed(registered_line, line_no, "Registered players : ")?
            .parse()
            .map_err(|_| err(line_no, registered_line, "nombre d'inscrits"))?;

    let (line_no, mode_line) = expect_line(&mut lines, line_no, "ligne \"Mode\" manquante")?;
    let mode = parse_prefixed(mode_line, line_no, "Mode : ")?.to_string();

    let (line_no, type_line) = expect_line(&mut lines, line_no, "ligne \"Type\" manquante")?;
    let tournament_type = parse_prefixed(type_line, line_no, "Type : ")?.to_string();

    let (line_no, speed_line) = expect_line(&mut lines, line_no, "ligne \"Speed\" manquante")?;
    let speed = parse_prefixed(speed_line, line_no, "Speed : ")?.to_string();

    let (line_no, flight_line) = expect_line(&mut lines, line_no, "ligne \"Flight ID\" manquante")?;
    let flight_id: u32 = parse_prefixed(flight_line, line_no, "Flight ID : ")?
        .parse()
        .map_err(|_| err(line_no, flight_line, "numero de flight"))?;

    // Levels, Prizepool, Tournament started : non utilises pour l'instant (§5.2, ⚠️).
    let (line_no, _levels_line) = expect_line(&mut lines, line_no, "ligne \"Levels\" manquante")?;
    let (line_no, _prizepool_line) =
        expect_line(&mut lines, line_no, "ligne \"Prizepool\" manquante")?;
    let (line_no, _started_line) = expect_line(
        &mut lines,
        line_no,
        "ligne \"Tournament started\" manquante",
    )?;

    let (line_no, duration_line) =
        expect_line(&mut lines, line_no, "ligne \"You played\" manquante")?;
    let played_seconds = parse_duration_seconds(
        parse_prefixed(duration_line, line_no, "You played ")?,
        duration_line,
        line_no,
    )?;

    let (line_no, finish_line) =
        expect_line(&mut lines, line_no, "ligne \"You finished in\" manquante")?;
    let finish_position = parse_finish_position(finish_line, line_no)?;

    let (prize, bounty) = match lines.next() {
        Some((line_no, won_line)) => parse_won_line(won_line, line_no)?,
        None => (Money::ZERO, Money::ZERO),
    };

    Ok((
        HeaderFields {
            room_tournament_id,
            name,
            hero_pseudo,
            buyin_prize,
            buyin_bounty,
            buyin_fee,
            mode,
            tournament_type,
            speed,
            flight_id,
        },
        TournamentBullet {
            entry_no,
            late_reg_bust,
            finish_position: Some(finish_position),
            played_seconds,
            prize,
            bounty,
            registered_snapshot,
        },
    ))
}

/// Parse un fichier summary complet (PAR-12) : un bloc par entree, separes par
/// une ligne vide. Les champs constants (buy-in, mode, type...) sont pris du
/// **dernier** bloc, comme `Registered players` (§5.2 : instantane le plus a jour).
pub(crate) fn parse_summary(text: &str) -> Result<TournamentSummary, ParseError> {
    let blocks = split_summary_blocks(text);
    let mut header: Option<HeaderFields> = None;
    let mut bullets = Vec::with_capacity(blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        let (block_header, bullet) =
            parse_summary_block(block, u32::try_from(i + 1).unwrap_or(u32::MAX))?;
        header = Some(block_header);
        bullets.push(bullet);
    }

    let header = header.ok_or_else(|| err(0, "", "fichier summary vide"))?;
    Ok(TournamentSummary {
        room_tournament_id: header.room_tournament_id,
        name: header.name,
        hero_pseudo: header.hero_pseudo,
        buyin_prize: header.buyin_prize,
        buyin_bounty: header.buyin_bounty,
        buyin_fee: header.buyin_fee,
        mode: header.mode,
        tournament_type: header.tournament_type,
        speed: header.speed,
        flight_id: header.flight_id,
        bullets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn obelisk_summary() -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit_summary.txt",
        );
        std::fs::read_to_string(path).unwrap()
    }

    /// CA de M1-6 : OBELISK -> 2 entrees, cout 4,00 €, gains 52,19 €, profit
    /// +48,19 €, place 7 (valide sur §5.3 du format).
    #[test]
    fn matches_the_obelisk_worked_example() {
        let summary = parse_summary(&obelisk_summary()).unwrap();
        assert_eq!(summary.room_tournament_id, "1173012730");
        assert_eq!(summary.name, "OBELISK - TRIDENT SPACE KO");
        assert_eq!(summary.hero_pseudo, "Hero");
        assert_eq!(summary.buyin_prize, Money::from_cents(80));
        assert_eq!(summary.buyin_bounty, Money::from_cents(100));
        assert_eq!(summary.buyin_fee, Money::from_cents(20));
        assert_eq!(summary.bullets.len(), 2);

        let entries = u32::try_from(summary.bullets.len()).unwrap();
        let cost: Money = std::iter::repeat_n(
            summary.buyin_prize + summary.buyin_bounty + summary.buyin_fee,
            entries as usize,
        )
        .sum();
        let gains: Money = summary.bullets.iter().map(|b| b.prize + b.bounty).sum();
        let profit = gains - cost;

        assert_eq!(cost, Money::from_cents(400));
        assert_eq!(gains, Money::from_cents(5219));
        assert_eq!(profit, Money::from_cents(4819));
        assert_eq!(summary.bullets.last().unwrap().finish_position, Some(7));

        assert!(summary.bullets[0].late_reg_bust);
        assert_eq!(summary.bullets[0].registered_snapshot, 812);
        assert_eq!(summary.bullets[0].played_seconds, 5 * 60 + 27);
        assert_eq!(summary.bullets[0].prize, Money::ZERO);
        assert_eq!(summary.bullets[0].bounty, Money::ZERO);

        assert!(!summary.bullets[1].late_reg_bust);
        assert_eq!(summary.bullets[1].registered_snapshot, 1141);
        assert_eq!(summary.bullets[1].played_seconds, 3600 + 36 * 60 + 43);
        assert_eq!(summary.bullets[1].prize, Money::from_cents(3479));
        assert_eq!(summary.bullets[1].bounty, Money::from_cents(1740));
    }

    #[test]
    fn parses_the_three_you_won_variants() {
        assert_eq!(
            parse_won_line("You won 30.47\u{20ac}", 1).unwrap(),
            (Money::from_cents(3047), Money::ZERO)
        );
        assert_eq!(
            parse_won_line("You won Bounty 1\u{20ac}", 1).unwrap(),
            (Money::ZERO, Money::from_cents(100))
        );
        assert_eq!(
            parse_won_line("You won 34.79\u{20ac} + Bounty 17.40\u{20ac}", 1).unwrap(),
            (Money::from_cents(3479), Money::from_cents(1740))
        );
    }

    #[test]
    fn parses_a_freeroll_buyin() {
        assert_eq!(
            parse_buyin_line("Buy-In : 0\u{20ac} + 0\u{20ac}", 1).unwrap(),
            (Money::ZERO, Money::ZERO, Money::ZERO)
        );
    }

    #[test]
    fn parses_duration_variants() {
        assert_eq!(
            parse_duration_seconds("5min 27s", "", 1).unwrap(),
            5 * 60 + 27
        );
        assert_eq!(
            parse_duration_seconds("1h 36min 43s", "", 1).unwrap(),
            3600 + 36 * 60 + 43
        );
    }
}
