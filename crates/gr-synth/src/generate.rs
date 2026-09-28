//! Construction du texte d'un tournoi synthetique, motif par motif, en ne
//! reprenant que des formes deja documentees dans `docs/formats/winamax.md`
//! (R-FORMAT) : en-tete §4.1, table §4.2, sieges §4.3, sections §4.4, pli
//! preflop non suivi de main (`P folds` / `P collected N from pot`) observe
//! sur le corpus reel, resume minimal §4.7 (`Total pot N | No rake`).

use std::fmt::Write as _;

use crate::rng::Rng;

/// Structures de blindes de niveau 1 deja observees (`(3/10/20)`, PAR-4) ou
/// utilisees comme fixture de test du parser (`(40/175/350)`).
const BLIND_STRUCTURES: [(i64, i64, i64); 3] = [(3, 10, 20), (25, 100, 200), (40, 175, 350)];
/// Tailles de table deja documentees (§4.2/§4.3 : 3-max, 6-max, 7-max).
const TABLE_SIZES: [u8; 3] = [3, 6, 7];
/// Tapis de depart deja observes (OBELISK : 500 ; VELOCITY : 20000).
const STARTING_STACKS: [i64; 2] = [500, 20_000];
/// Bornes du nombre de mains par tournoi synthetique.
const HANDS_PER_TOURNAMENT: (u32, u32) = (50, 400);

/// Un tournoi synthetique complet, pret a etre ecrit sur disque.
pub struct GeneratedTournament {
    /// Nom de fichier sans extension, au format documente en §2 (sans les
    /// parentheses autour de l'ID, perdues a l'upload d'apres §2 ⚠️ : on les
    /// garde ici, le parser ne lit que le contenu, jamais le nom de fichier).
    pub file_stem: String,
    pub content: String,
    pub hand_count: usize,
}

/// Genere un tournoi synthetique d'environ `hands_wanted` mains (au plus,
/// jamais moins d'une main), avec une table et un effectif tires
/// deterministement a partir de `rng`.
#[must_use]
pub fn generate_tournament(
    rng: &mut Rng,
    tournament_index: u64,
    hands_wanted: usize,
) -> GeneratedTournament {
    let (min, max) = HANDS_PER_TOURNAMENT;
    let target = usize::try_from(rng.range_u32(min, max)).unwrap_or(50);
    let hand_count = target.min(hands_wanted).max(1);

    let table_max_seats = pick_table_size(rng);
    let seated = u8::try_from(rng.range_u32(2, u32::from(table_max_seats))).unwrap_or(2);
    let (ante, sb, bb) = pick_blind_structure(rng);
    let starting_stack = STARTING_STACKS[usize::try_from(rng.next_u64() % 2).unwrap_or(0)];

    let tournament_name = format!("SYNTH TOURNAMENT {tournament_index:06}");
    let tournament_id = 900_000_000 + tournament_index;
    let table_no = 1_u32;

    let mut content = String::new();
    let hand_count_u64 = u64::try_from(hand_count).unwrap_or(1);
    for hand_no in 1..=hand_count_u64 {
        let button_seat = u8::try_from((hand_no - 1) % u64::from(seated)).unwrap_or(0) + 1;
        let time_of_day =
            u32::try_from((tournament_index.wrapping_mul(1_000_003) + hand_no) % 86_400)
                .unwrap_or(0);
        let hand_text = build_hand(
            rng,
            &tournament_name,
            tournament_id,
            table_no,
            hand_no,
            table_max_seats,
            seated,
            button_seat,
            ante,
            sb,
            bb,
            starting_stack,
            time_of_day,
        );
        // `build_hand` termine chaque ligne par `\n` par simplicite ; le
        // separateur entre mains est *exactement* `\n\n\n` (§3), sans `\n`
        // de plus emprunte a la derniere ligne du contenu.
        content.push_str(hand_text.trim_end_matches('\n'));
        content.push_str("\n\n\n");
    }

    GeneratedTournament {
        file_stem: format!("20260101_{tournament_name}({tournament_id})_real_holdem_no-limit"),
        content,
        hand_count,
    }
}

fn pick_table_size(rng: &mut Rng) -> u8 {
    TABLE_SIZES[usize::try_from(rng.next_u64() % 3).unwrap_or(0)]
}

fn pick_blind_structure(rng: &mut Rng) -> (i64, i64, i64) {
    BLIND_STRUCTURES[usize::try_from(rng.next_u64() % 3).unwrap_or(0)]
}

#[allow(clippy::too_many_arguments)]
fn build_hand(
    rng: &mut Rng,
    tournament_name: &str,
    tournament_id: u64,
    table_no: u32,
    hand_no: u64,
    table_max_seats: u8,
    seated: u8,
    button_seat: u8,
    ante: i64,
    sb: i64,
    bb: i64,
    starting_stack: i64,
    time_of_day_seconds: u32,
) -> String {
    let hh = time_of_day_seconds / 3600;
    let mm = (time_of_day_seconds % 3600) / 60;
    let ss = time_of_day_seconds % 60;

    let (sb_seat, bb_seat) = if seated == 2 {
        (button_seat, next_seat(button_seat, seated))
    } else {
        let sb_seat = next_seat(button_seat, seated);
        (sb_seat, next_seat(sb_seat, seated))
    };
    let pseudo = |seat: u8| -> String {
        if seat == 1 {
            "Hero".to_string()
        } else {
            format!("P{seat:04}")
        }
    };
    let total_pot = i64::from(seated) * ante + sb + bb;
    let z = tournament_index_and_hand_to_z(tournament_id, hand_no);

    let mut text = String::new();
    let _ = writeln!(
        text,
        "Winamax Poker - Tournament \"{tournament_name}\" buyIn: 1\u{20ac} + 0\u{20ac} level: 1 - HandId: #{table_no}-{hand_no}-{z} - Holdem no limit ({ante}/{sb}/{bb}) - 2026/01/01 {hh:02}:{mm:02}:{ss:02} UTC"
    );
    let _ = writeln!(
        text,
        "Table: '{tournament_name}({tournament_id})#{table_no:04}' {table_max_seats}-max (real money) Seat #{button_seat} is the button"
    );
    for seat in 1..=seated {
        let _ = writeln!(text, "Seat {seat}: {} ({starting_stack})", pseudo(seat));
    }
    text.push_str("*** ANTE/BLINDS ***\n");
    for seat in 1..=seated {
        let _ = writeln!(text, "{} posts ante {ante}", pseudo(seat));
    }
    let _ = writeln!(text, "{} posts small blind {sb}", pseudo(sb_seat));
    let _ = writeln!(text, "{} posts big blind {bb}", pseudo(bb_seat));

    let (r1, s1) = (rng.pick_rank(), rng.pick_suit());
    let mut r2 = rng.pick_rank();
    let mut s2 = rng.pick_suit();
    while r1 == r2 && s1 == s2 {
        r2 = rng.pick_rank();
        s2 = rng.pick_suit();
    }
    let _ = writeln!(text, "Dealt to Hero [{r1}{s1} {r2}{s2}]");

    text.push_str("*** PRE-FLOP ***\n");
    let mut seat = next_seat(bb_seat, seated);
    while seat != bb_seat {
        let _ = writeln!(text, "{} folds", pseudo(seat));
        seat = next_seat(seat, seated);
    }
    let _ = writeln!(text, "{} collected {total_pot} from pot", pseudo(bb_seat));

    text.push_str("*** SUMMARY ***\n");
    let _ = writeln!(text, "Total pot {total_pot} | No rake");
    text
}

fn next_seat(seat: u8, seated: u8) -> u8 {
    (seat % seated) + 1
}

fn tournament_index_and_hand_to_z(tournament_id: u64, hand_no: u64) -> u64 {
    tournament_id * 1_000_000 + hand_no
}

#[cfg(test)]
mod tests {
    use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

    use super::*;

    #[test]
    fn every_hand_of_many_generated_tournaments_parses_successfully() {
        // Plusieurs graines/index different pour couvrir les 3 tailles de
        // table, le heads-up (seated == 2) et les 3 structures de blindes.
        for seed in 0..30u64 {
            let mut rng = Rng::new(seed);
            let tournament = generate_tournament(&mut rng, seed + 1, 60);
            let (blocks, offset) = split_hand_blocks(&tournament.content);
            assert_eq!(
                offset,
                tournament.content.len(),
                "seed {seed}: le fichier genere doit se terminer par \\n\\n\\n"
            );
            assert_eq!(blocks.len(), tournament.hand_count);
            for (i, block) in blocks.iter().enumerate() {
                WinamaxParser::parse_hand(block)
                    .unwrap_or_else(|e| panic!("seed {seed}, main {}: {e}", i + 1));
            }
        }
    }

    #[test]
    fn is_deterministic_for_a_given_seed_and_index() {
        let mut rng_a = Rng::new(123);
        let mut rng_b = Rng::new(123);
        let a = generate_tournament(&mut rng_a, 7, 100);
        let b = generate_tournament(&mut rng_b, 7, 100);
        assert_eq!(a.content, b.content);
        assert_eq!(a.file_stem, b.file_stem);
    }

    #[test]
    fn respects_the_hands_wanted_cap_when_it_is_the_smaller_bound() {
        let mut rng = Rng::new(1);
        let tournament = generate_tournament(&mut rng, 1, 5);
        assert_eq!(tournament.hand_count, 5);
        let (blocks, _) = split_hand_blocks(&tournament.content);
        assert_eq!(blocks.len(), 5);
    }

    #[test]
    fn exercises_heads_up_at_least_once_across_many_seeds() {
        let saw_heads_up = (0..50u64).any(|seed| {
            let mut rng = Rng::new(seed);
            let tournament = generate_tournament(&mut rng, seed + 1, 10);
            let (blocks, _) = split_hand_blocks(&tournament.content);
            blocks
                .iter()
                .any(|b| WinamaxParser::parse_hand(b).is_ok_and(|h| h.seats.len() == 2))
        });
        assert!(
            saw_heads_up,
            "aucune main heads-up (seated == 2) generee sur 50 graines"
        );
    }
}
