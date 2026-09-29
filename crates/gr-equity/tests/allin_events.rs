//! Detection d'evenement all-in et conservation de l'EV (PRD §10.6, M5-4,
//! CA : "Conservation : Σ EV des joueurs = Σ pots").

use gr_core::{
    ActionKind, ActionRecord, Card, Chips, HandRecord, PotKind, PotResult, SeatInfo, Street,
};
use gr_equity::{detect_all_in_event, EquityMethod};

fn card(spec: &str) -> Card {
    spec.parse()
        .unwrap_or_else(|_| panic!("carte de test invalide : {spec}"))
}

fn cards(specs: &str) -> Vec<Card> {
    specs.split_whitespace().map(card).collect()
}

fn seat(seat_no: u8, pseudo: &str, stack: i64) -> SeatInfo {
    SeatInfo {
        seat: seat_no,
        pseudo: pseudo.to_string(),
        starting_stack: Chips::from_i64(stack),
        bounty: None,
        dealt_in: true,
    }
}

fn base_action(street: Street, pseudo: &str, kind: ActionKind) -> ActionRecord {
    ActionRecord {
        street,
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

fn post(street: Street, pseudo: &str, kind: ActionKind, amount: i64) -> ActionRecord {
    let mut a = base_action(street, pseudo, kind);
    a.amount = Some(Chips::from_i64(amount));
    a
}

fn check(street: Street, pseudo: &str) -> ActionRecord {
    base_action(street, pseudo, ActionKind::Check)
}

fn call(street: Street, pseudo: &str, amount: i64) -> ActionRecord {
    let mut a = base_action(street, pseudo, ActionKind::Call);
    a.amount = Some(Chips::from_i64(amount));
    a
}

fn fold(street: Street, pseudo: &str) -> ActionRecord {
    base_action(street, pseudo, ActionKind::Fold)
}

fn shove(street: Street, pseudo: &str, to: i64) -> ActionRecord {
    let mut a = base_action(street, pseudo, ActionKind::Raise);
    a.to_amount = Some(Chips::from_i64(to));
    a.is_all_in = true;
    a
}

fn shows(pseudo: &str, hole: &str) -> ActionRecord {
    let mut a = base_action(Street::Showdown, pseudo, ActionKind::Shows);
    a.shown_cards = Some(cards(hole));
    a
}

fn base_hand(
    room_hand_id: &str,
    seats: Vec<SeatInfo>,
    actions: Vec<ActionRecord>,
    board: &str,
    pots: Vec<PotResult>,
) -> HandRecord {
    let total_pot: i64 = pots.iter().map(|p| p.amount.amount()).sum();
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
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
        board: cards(board),
        pots,
        total_pot: Chips::from_i64(total_pot),
        rake: Chips::ZERO,
        uncalled_excess: Chips::ZERO,
        hero_pseudo: Some("Hero".to_string()),
        hero_cards: Some((card("Ah"), card("Ad"))),
        parser_version: "0.0.1".to_string(),
    }
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

#[test]
fn heads_up_all_in_preflop_conserves_equity() {
    // Hero (AA) shove preflop, Villain (72o) suit tapis pour tapis ;
    // riviere distribuee sans action (all-in). Hero remporte tout le pot
    // dans cette instance precise.
    let hand = base_hand(
        "#1-1-1",
        vec![seat(1, "Hero", 500), seat(2, "Villain", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain", ActionKind::PostBigBlind, 20),
            shove(Street::Preflop, "Hero", 500),
            call(Street::Preflop, "Villain", 490),
            shows("Hero", "Ah Ad"),
            shows("Villain", "2c 7d"),
        ],
        "Kc Qd Jc Ts 3h",
        vec![pot(1000, &[("Hero", 1000)])],
    );

    let event = detect_all_in_event(&hand).expect("un all-in preflop doit etre detecte");
    assert_eq!(event.method, EquityMethod::Exact);
    assert_eq!(event.player_diffs.len(), 2);

    let total: f64 = event.player_diffs.iter().map(|(_, diff)| diff).sum();
    assert!(total.abs() < 1e-6, "Sigma diffs = {total} (conservation)");

    let hero_diff = event
        .player_diffs
        .iter()
        .find(|(pseudo, _)| pseudo == "Hero")
        .map(|(_, diff)| *diff)
        .expect("Hero doit etre implique");
    // Hero a remporte tout le pot alors qu'AA n'est pas favori a 100% face
    // a 72o preflop : diff positif ("chanceux"), meme si tres favori.
    assert!(hero_diff > 0.0, "hero_diff = {hero_diff}");
}

#[test]
fn no_event_when_the_hand_reaches_a_normal_river_showdown() {
    // Action jusqu'a la riviere (check partout) : pas d'all-in avant la
    // riviere, donc pas d'evenement (PRD §10.6 : "avant la river").
    let hand = base_hand(
        "#1-1-2",
        vec![seat(1, "Hero", 500), seat(2, "Villain", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain", ActionKind::PostBigBlind, 20),
            call(Street::Preflop, "Hero", 10),
            check(Street::Preflop, "Villain"),
            check(Street::Flop, "Villain"),
            check(Street::Flop, "Hero"),
            check(Street::Turn, "Villain"),
            check(Street::Turn, "Hero"),
            check(Street::River, "Villain"),
            check(Street::River, "Hero"),
            shows("Hero", "Ah Ad"),
            shows("Villain", "2c 7d"),
        ],
        "Kc Qd Jc Ts 3h",
        vec![pot(40, &[("Hero", 40)])],
    );

    assert_eq!(detect_all_in_event(&hand), None);
}

#[test]
fn no_event_when_the_hand_ends_by_an_uncontested_fold() {
    let hand = base_hand(
        "#1-1-3",
        vec![seat(1, "Hero", 500), seat(2, "Villain", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain", ActionKind::PostBigBlind, 20),
            fold(Street::Preflop, "Hero"),
        ],
        "",
        vec![pot(20, &[("Villain", 20)])],
    );

    assert_eq!(detect_all_in_event(&hand), None);
}

#[test]
fn no_event_when_a_live_players_cards_are_never_shown() {
    // Meme main que le premier test, mais Villain ne montre pas ses
    // cartes : pas de calcul possible (PRD §10.6, cartes connues requises).
    let hand = base_hand(
        "#1-1-4",
        vec![seat(1, "Hero", 500), seat(2, "Villain", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain", ActionKind::PostBigBlind, 20),
            shove(Street::Preflop, "Hero", 500),
            call(Street::Preflop, "Villain", 490),
        ],
        "Kc Qd Jc Ts 3h",
        vec![pot(1000, &[("Hero", 1000)])],
    );

    assert_eq!(detect_all_in_event(&hand), None);
}

#[test]
fn no_event_when_the_all_in_happens_on_the_river_itself() {
    // Mise/suivi sur la riviere elle-meme : plus rien a tirer, ce n'est
    // pas un evenement all-in au sens du CA ("avant la river").
    let hand = base_hand(
        "#1-1-5",
        vec![seat(1, "Hero", 500), seat(2, "Villain", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain", ActionKind::PostBigBlind, 20),
            call(Street::Preflop, "Hero", 10),
            check(Street::Preflop, "Villain"),
            check(Street::Flop, "Villain"),
            check(Street::Flop, "Hero"),
            check(Street::Turn, "Villain"),
            check(Street::Turn, "Hero"),
            shove(Street::River, "Villain", 480),
            call(Street::River, "Hero", 480),
            shows("Hero", "Ah Ad"),
            shows("Villain", "2c 7d"),
        ],
        "Kc Qd Jc Ts 3h",
        vec![pot(1000, &[("Hero", 1000)])],
    );

    assert_eq!(detect_all_in_event(&hand), None);
}

#[test]
fn three_way_all_in_on_the_turn_conserves_equity() {
    let hand = base_hand(
        "#1-1-6",
        vec![
            seat(1, "Hero", 500),
            seat(2, "Villain1", 500),
            seat(3, "Villain2", 500),
        ],
        vec![
            post(Street::Preflop, "Villain1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Villain2", ActionKind::PostBigBlind, 20),
            call(Street::Preflop, "Hero", 20),
            call(Street::Preflop, "Villain1", 10),
            check(Street::Preflop, "Villain2"),
            check(Street::Flop, "Villain1"),
            check(Street::Flop, "Villain2"),
            check(Street::Flop, "Hero"),
            shove(Street::Turn, "Villain1", 480),
            call(Street::Turn, "Villain2", 480),
            call(Street::Turn, "Hero", 480),
            shows("Hero", "Ah Ad"),
            shows("Villain1", "2c 7d"),
            shows("Villain2", "Ks Kd"),
        ],
        "Kc Qd Jc 3h 9s",
        vec![pot(1500, &[("Hero", 1500)])],
    );

    let event = detect_all_in_event(&hand).expect("un all-in au turn doit etre detecte");
    assert_eq!(event.method, EquityMethod::Exact);
    assert_eq!(event.player_diffs.len(), 3);
    let total: f64 = event.player_diffs.iter().map(|(_, diff)| diff).sum();
    assert!(total.abs() < 1e-6, "Sigma diffs = {total} (conservation)");
}
