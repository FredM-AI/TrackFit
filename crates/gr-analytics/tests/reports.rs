//! Integration test du rapport prédéfini "Défense de BB par profondeur"
//! (M7-1, PRD §13.5) : filtre sur la position brute `BB` (pas le groupe
//! `"Blinds"`), groupé par profondeur effective. Même convention que
//! `tests/report.rs` : un vrai `Store` SQLite, pas de fixture SQL à la main.

use gr_analytics::fetch_bb_defense_by_depth;
use gr_core::{ActionKind, ActionRecord, Chips, HandRecord, SeatInfo, Street};
use gr_parser_api::Room;
use gr_store::{HandInsert, Store};

fn seat(seat_no: u8, pseudo: &str, stack: i64) -> SeatInfo {
    SeatInfo {
        seat: seat_no,
        pseudo: pseudo.to_string(),
        starting_stack: Chips::from_i64(stack),
        bounty: None,
        dealt_in: true,
    }
}

fn action(street: Street, pseudo: &str, kind: ActionKind) -> ActionRecord {
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
    let mut a = action(street, pseudo, kind);
    a.amount = Some(Chips::from_i64(amount));
    a
}

fn raise_to(pseudo: &str, to: i64) -> ActionRecord {
    let mut a = action(Street::Preflop, pseudo, ActionKind::Raise);
    a.to_amount = Some(Chips::from_i64(to));
    a
}

fn call(pseudo: &str, amount: i64) -> ActionRecord {
    post(Street::Preflop, pseudo, ActionKind::Call, amount)
}

fn fold(pseudo: &str) -> ActionRecord {
    action(Street::Preflop, pseudo, ActionKind::Fold)
}

#[allow(clippy::too_many_arguments)]
fn base_hand(
    room_hand_id: &str,
    button_seat: u8,
    stack: i64,
    bb: i64,
    actions: Vec<ActionRecord>,
) -> HandRecord {
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: "1".to_string(),
        table_name: "T(1)#1".to_string(),
        table_max_seats: 3,
        button_seat,
        level: 1,
        sb: Chips::from_i64(bb / 2),
        bb: Chips::from_i64(bb),
        ante: Chips::ZERO,
        played_at: 0,
        seats: vec![
            seat(1, "Hero", stack),
            seat(2, "V1", stack),
            seat(3, "V2", stack),
        ],
        actions,
        board: vec![],
        pots: vec![],
        total_pot: Chips::ZERO,
        rake: Chips::ZERO,
        uncalled_excess: Chips::ZERO,
        hero_pseudo: Some("Hero".to_string()),
        hero_cards: None,
        parser_version: "0.0.1".to_string(),
    }
}

/// 3-max, bouton = V1 (seat 2) : Hero est BB. V1 (BTN) ouvre (tentative de
/// vol), V2 (SB) se couche, Hero se couche aussi -> fsteal vrai.
fn hand_bb_folds_to_steal_short_stack() -> HandRecord {
    base_hand(
        "#1-1-1",
        2,
        200, // 10 bb de profondeur (200/20)
        20,
        vec![
            post(Street::Preflop, "V2", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Hero", ActionKind::PostBigBlind, 20),
            raise_to("V1", 60),
            fold("V2"),
            fold("Hero"),
        ],
    )
}

/// Meme situation, tapis profond : Hero (BB) suit au lieu de se coucher ->
/// fsteal faux, meme opportunite.
fn hand_bb_calls_steal_deep_stack() -> HandRecord {
    base_hand(
        "#1-1-2",
        2,
        2_000, // 100 bb de profondeur (2000/20)
        20,
        vec![
            post(Street::Preflop, "V2", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Hero", ActionKind::PostBigBlind, 20),
            raise_to("V1", 60),
            fold("V2"),
            call("Hero", 40),
        ],
    )
}

/// Hero au bouton (pas en BB) : ne doit pas apparaitre dans le rapport,
/// quelle que soit la situation.
fn hand_hero_not_in_bb() -> HandRecord {
    base_hand(
        "#1-1-3",
        1,
        200,
        20,
        vec![
            post(Street::Preflop, "V1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V2", ActionKind::PostBigBlind, 20),
            raise_to("Hero", 60),
            fold("V1"),
            fold("V2"),
        ],
    )
}

#[test]
fn bb_defense_report_filters_on_raw_bb_position_and_groups_by_depth() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let hand1 = hand_bb_folds_to_steal_short_stack();
    let hand2 = hand_bb_calls_steal_deep_stack();
    let hand3 = hand_hero_not_in_bb();
    store
        .insert_hands(
            Room::Winamax,
            &[
                HandInsert {
                    hand: &hand1,
                    raw_text: "irrelevant",
                },
                HandInsert {
                    hand: &hand2,
                    raw_text: "irrelevant",
                },
                HandInsert {
                    hand: &hand3,
                    raw_text: "irrelevant",
                },
            ],
        )
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    let mut rows =
        fetch_bb_defense_by_depth(&reader, profile_id).expect("report query should succeed");
    rows.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));

    assert_eq!(
        rows.len(),
        2,
        "seulement les 2 mains ou Hero est BB, 2 profondeurs differentes : {rows:?}"
    );
    let total_fsteal_opp: i64 = rows.iter().map(|r| r.cells[0].opportunities).sum();
    let total_fsteal: i64 = rows.iter().map(|r| r.cells[0].actions).sum();
    assert_eq!(total_fsteal_opp, 2, "2 mains ou Hero est BB face a un vol");
    assert_eq!(total_fsteal, 1, "1 seule des 2 est un fold (tapis court)");
}
