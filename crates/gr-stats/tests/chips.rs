//! `compute_net_chips`/`compute_net_bb` (PRD §9.1/§13.4, M6-3). Helpers
//! locaux distincts du DSL `hand!{}` partage (`tests/support/mod.rs`) : ce
//! DSL ne renseigne que `to_amount` sur une relance (suffisant pour les
//! flags preflop/postflop, M4-2), alors que `compute_net_chips` a besoin du
//! meme `amount` incremental que produit le vrai parser
//! (`gr-parser-winamax::parse_action_line`, PAR-11).

use gr_core::{ActionKind, ActionRecord, Chips, HandRecord, PotKind, PotResult, SeatInfo, Street};
use gr_stats::{compute_net_bb, compute_net_chips};

fn seat(seat_no: u8, pseudo: &str, stack: i64) -> SeatInfo {
    SeatInfo {
        seat: seat_no,
        pseudo: pseudo.to_string(),
        starting_stack: Chips::from_i64(stack),
        bounty: None,
        dealt_in: true,
    }
}

fn base_action(pseudo: &str, kind: ActionKind) -> ActionRecord {
    ActionRecord {
        street: Street::Preflop,
        pseudo: pseudo.to_string(),
        kind,
        amount: None,
        to_amount: None,
        is_all_in: false,
        pot: None,
        shown_cards: None,
        shown_label: None,
    }
}

fn post(pseudo: &str, kind: ActionKind, amount: i64) -> ActionRecord {
    let mut a = base_action(pseudo, kind);
    a.amount = Some(Chips::from_i64(amount));
    a
}

fn call(pseudo: &str, amount: i64) -> ActionRecord {
    post(pseudo, ActionKind::Call, amount)
}

fn fold(pseudo: &str) -> ActionRecord {
    base_action(pseudo, ActionKind::Fold)
}

/// Relance a `to` jetons au total sur la street, avec le meme `amount`
/// incremental que le vrai parser (`to - street_contrib_precedent`) : ici
/// toujours une premiere relance de la street (`street_contrib` = 0).
fn raise_from_zero(pseudo: &str, to: i64) -> ActionRecord {
    let mut a = base_action(pseudo, ActionKind::Raise);
    a.amount = Some(Chips::from_i64(to));
    a.to_amount = Some(Chips::from_i64(to));
    a
}

fn pot(amount: i64, winners: &[(&str, i64)]) -> PotResult {
    PotResult {
        pot: PotKind::Pot,
        amount: Chips::from_i64(amount),
        winners: winners
            .iter()
            .map(|(name, share)| ((*name).to_string(), Chips::from_i64(*share)))
            .collect(),
    }
}

fn base_hand(seats: Vec<SeatInfo>, actions: Vec<ActionRecord>, pots: Vec<PotResult>) -> HandRecord {
    HandRecord {
        room_hand_id: "#1-1-1".to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: "1".to_string(),
        table_name: "T(1)#1".to_string(),
        table_max_seats: u8::try_from(seats.len()).unwrap_or(u8::MAX),
        button_seat: 1,
        level: 1,
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
        ante: Chips::ZERO,
        played_at: 0,
        seats,
        actions,
        board: Vec::new(),
        pots,
        total_pot: Chips::ZERO,
        rake: Chips::ZERO,
        uncalled_excess: Chips::ZERO,
        hero_pseudo: Some("Hero".to_string()),
        hero_cards: None,
        parser_version: "0.0.1".to_string(),
    }
}

#[test]
fn net_chips_of_the_winner_is_winnings_minus_their_own_contribution() {
    // BTN Hero raises to 60, SB (V1) folds, BB (V2) calls 40 de plus (deja
    // 20 postes) ; Hero remporte tout (130 = 10 + 60 + 60).
    let hand = base_hand(
        vec![
            seat(1, "Hero", 1000),
            seat(2, "V1", 1000),
            seat(3, "V2", 1000),
        ],
        vec![
            post("V1", ActionKind::PostSmallBlind, 10),
            post("V2", ActionKind::PostBigBlind, 20),
            raise_from_zero("Hero", 60),
            fold("V1"),
            call("V2", 40),
        ],
        vec![pot(130, &[("Hero", 130)])],
    );

    assert_eq!(compute_net_chips(&hand, "Hero"), 70); // 130 - 60
    assert_eq!(compute_net_chips(&hand, "V1"), -10); // 0 - 10
    assert_eq!(compute_net_chips(&hand, "V2"), -60); // 0 - 60
}

#[test]
fn net_chips_sum_to_zero_across_all_players_without_rake() {
    let hand = base_hand(
        vec![
            seat(1, "Hero", 1000),
            seat(2, "V1", 1000),
            seat(3, "V2", 1000),
        ],
        vec![
            post("V1", ActionKind::PostSmallBlind, 10),
            post("V2", ActionKind::PostBigBlind, 20),
            raise_from_zero("Hero", 60),
            fold("V1"),
            call("V2", 40),
        ],
        vec![pot(130, &[("Hero", 130)])],
    );

    let total: i64 = ["Hero", "V1", "V2"]
        .iter()
        .map(|p| compute_net_chips(&hand, p))
        .sum();
    assert_eq!(total, 0, "conservation : Σ net_chips = 0 (pas de rake)");
}

#[test]
fn a_split_pot_gives_each_winner_exactly_their_own_share() {
    let hand = base_hand(
        vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
        vec![
            post("Hero", ActionKind::PostSmallBlind, 10),
            post("V1", ActionKind::PostBigBlind, 20),
            call("Hero", 10),
        ],
        vec![pot(40, &[("Hero", 20), ("V1", 20)])],
    );

    assert_eq!(compute_net_chips(&hand, "Hero"), 0); // 20 gagnes - 20 mises
    assert_eq!(compute_net_chips(&hand, "V1"), 0); // 20 gagnes - 20 mises
}

#[test]
fn a_player_who_never_acted_or_won_nets_zero() {
    let hand = base_hand(
        vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
        vec![
            post("Hero", ActionKind::PostSmallBlind, 10),
            post("V1", ActionKind::PostBigBlind, 20),
            fold("Hero"),
        ],
        vec![pot(30, &[("V1", 30)])],
    );

    assert_eq!(compute_net_chips(&hand, "Ghost"), 0);
}

#[test]
fn net_bb_divides_net_chips_by_the_big_blind() {
    let hand = base_hand(
        vec![
            seat(1, "Hero", 1000),
            seat(2, "V1", 1000),
            seat(3, "V2", 1000),
        ],
        vec![
            post("V1", ActionKind::PostSmallBlind, 10),
            post("V2", ActionKind::PostBigBlind, 20),
            raise_from_zero("Hero", 60),
            fold("V1"),
            call("V2", 40),
        ],
        vec![pot(130, &[("Hero", 130)])],
    );

    assert!((compute_net_bb(&hand, "Hero").unwrap() - 3.5).abs() < 1e-9); // 70 / 20
}

#[test]
fn net_chips_sum_to_the_negative_rake_on_every_real_obelisk_hand() {
    // Propriete de conservation (CLAUDE.md §7) sur du vrai texte Winamax,
    // pas seulement des mains construites a la main : Σ net_chips de tous
    // les sieges distribues doit valoir exactement -rake (0 sur ce corpus,
    // le rake MTT Winamax est preleve via le buy-in, jamais sur la main).
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
    );
    let text = std::fs::read_to_string(&path).expect("real OBELISK hands should be readable");
    let (blocks, _offset) = gr_parser_winamax::split_hand_blocks(&text);
    assert!(!blocks.is_empty(), "fixture should contain hands");

    for block in blocks {
        let hand =
            gr_parser_winamax::WinamaxParser::parse_hand(block).expect("fixture hand should parse");
        let total: i64 = hand
            .seats
            .iter()
            .filter(|s| s.dealt_in)
            .map(|s| compute_net_chips(&hand, &s.pseudo))
            .sum();
        assert_eq!(
            total,
            -hand.rake.amount(),
            "main {} : Σ net_chips devrait valoir -rake",
            hand.room_hand_id
        );
    }
}

#[test]
fn net_bb_is_none_when_the_big_blind_is_zero() {
    let mut hand = base_hand(
        vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
        vec![call("Hero", 10)],
        vec![pot(10, &[("Hero", 10)])],
    );
    hand.bb = Chips::ZERO;

    assert_eq!(compute_net_bb(&hand, "Hero"), None);
}
