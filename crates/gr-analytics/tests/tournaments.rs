//! Integration tests des requetes gr-analytics de l'ecran Tournois (M6-4,
//! phase 1, PRD §13.3) : agregats par tournoi (mains jouees, ecart EV
//! all-in), mains d'un tournoi (tapis/resultat par main) et adversaires
//! rencontres. Meme convention que `tests/home.rs` : un vrai `Store` SQLite.

use gr_analytics::{
    fetch_hero_tournament_aggregates, fetch_tournament_hero_hands, fetch_tournament_opponents,
};
use gr_core::{
    ActionKind, ActionRecord, Card, Chips, HandRecord, PotKind, PotResult, SeatInfo, Street,
};
use gr_parser_api::Room;
use gr_store::{HandInsert, Store};

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

fn call(street: Street, pseudo: &str, amount: i64) -> ActionRecord {
    let mut a = base_action(street, pseudo, ActionKind::Call);
    a.amount = Some(Chips::from_i64(amount));
    a
}

fn shove(street: Street, pseudo: &str, added: i64, to: i64) -> ActionRecord {
    let mut a = base_action(street, pseudo, ActionKind::Raise);
    a.amount = Some(Chips::from_i64(added));
    a.to_amount = Some(Chips::from_i64(to));
    a.is_all_in = true;
    a
}

fn shows(pseudo: &str, hole: &str) -> ActionRecord {
    let mut a = base_action(Street::Showdown, pseudo, ActionKind::Shows);
    a.shown_cards = Some(cards(hole));
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

#[allow(clippy::too_many_arguments)]
fn base_hand(
    room_hand_id: &str,
    tournament_room_id: &str,
    level: u32,
    played_at: i64,
    seats: Vec<SeatInfo>,
    actions: Vec<ActionRecord>,
    board: &str,
    pots: Vec<PotResult>,
) -> HandRecord {
    let total_pot: i64 = pots.iter().map(|p| p.amount.amount()).sum();
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: tournament_room_id.to_string(),
        table_name: format!("T({tournament_room_id})#1"),
        table_max_seats: u8::try_from(seats.len()).unwrap_or(u8::MAX),
        button_seat: 1,
        level,
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
        ante: Chips::ZERO,
        played_at,
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

fn open_store() -> (tempfile::TempDir, Store, i64) {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");
    (db_dir, store, profile_id)
}

fn insert(store: &Store, hand: &HandRecord) {
    store
        .insert_hands(
            Room::Winamax,
            &[HandInsert {
                hand,
                raw_text: "irrelevant",
            }],
        )
        .expect("insertion should succeed");
}

#[test]
fn fetch_hero_tournament_aggregates_counts_hands_and_sums_ev_diff_per_tournament() {
    let (_db_dir, store, profile_id) = open_store();

    // T1 : 1 main sans all-in.
    insert(
        &store,
        &base_hand(
            "#1-1-1",
            "T1",
            1,
            1_000,
            vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "Hero", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("V1", 30)])],
        ),
    );
    // T2 : 2 mains, dont 1 all-in (Hero AA vs 72o, gagne alors qu'il n'est
    // pas favori a 100% : diff EV garanti non nul, meme scenario que
    // `tests/home.rs`).
    insert(
        &store,
        &base_hand(
            "#2-1-1",
            "T2",
            1,
            2_000,
            vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                shove(Street::Preflop, "Hero", 490, 500),
                call(Street::Preflop, "V1", 480),
                shows("Hero", "Ah Ad"),
                shows("V1", "2c 7d"),
            ],
            "Kc Qd Jc Ts 3h",
            vec![pot(1000, &[("Hero", 1000)])],
        ),
    );
    insert(
        &store,
        &base_hand(
            "#2-1-2",
            "T2",
            1,
            3_000,
            vec![seat(1, "Hero", 1000), seat(2, "V1", 0)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "Hero", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("V1", 30)])],
        ),
    );

    let reader = store.reader().expect("reader connection");
    let aggregates = fetch_hero_tournament_aggregates(&reader, profile_id)
        .expect("aggregates query should succeed");

    assert_eq!(aggregates.len(), 2, "{aggregates:?}");

    let t1_id: i64 = reader
        .query_row(
            "SELECT id FROM tournaments WHERE room_tournament_id = 'T1'",
            [],
            |row| row.get(0),
        )
        .expect("T1 should exist");
    let t2_id: i64 = reader
        .query_row(
            "SELECT id FROM tournaments WHERE room_tournament_id = 'T2'",
            [],
            |row| row.get(0),
        )
        .expect("T2 should exist");

    let t1 = aggregates
        .iter()
        .find(|a| a.tournament_id == t1_id)
        .expect("T1 aggregates should exist");
    assert_eq!(t1.hands_played, 1);
    assert!((t1.ev_diff_bb - 0.0).abs() < 1e-9, "pas d'all-in sur T1");

    let t2 = aggregates
        .iter()
        .find(|a| a.tournament_id == t2_id)
        .expect("T2 aggregates should exist");
    assert_eq!(t2.hands_played, 2, "2 mains sur T2, dont 1 sans all-in");
    assert!(
        t2.ev_diff_bb > 0.0,
        "Hero a gagne sans etre favori a 100% : diff EV positif, {t2:?}"
    );
}

#[test]
fn fetch_tournament_hero_hands_returns_stack_and_result_per_hand_in_chronological_order() {
    let (_db_dir, store, profile_id) = open_store();

    insert(
        &store,
        &base_hand(
            "#1-1-1",
            "T1",
            1,
            2_000,
            vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "Hero", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("V1", 30)])],
        ),
    );
    insert(
        &store,
        &base_hand(
            "#1-1-2",
            "T1",
            2,
            1_000, // volontairement avant la 1ere main inseree : trie par played_at.
            vec![seat(1, "Hero", 490), seat(2, "V1", 510)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "V1", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("Hero", 30)])],
        ),
    );

    let reader = store.reader().expect("reader connection");
    let tournament_id: i64 = reader
        .query_row(
            "SELECT id FROM tournaments WHERE room_tournament_id = 'T1'",
            [],
            |row| row.get(0),
        )
        .expect("T1 should exist");

    let hands = fetch_tournament_hero_hands(&reader, profile_id, tournament_id)
        .expect("hands query should succeed");

    assert_eq!(hands.len(), 2);
    assert_eq!(hands[0].played_at, 1_000, "trie chronologiquement");
    assert_eq!(hands[0].level, 2);
    assert_eq!(hands[0].start_stack, Some(490));
    assert_eq!(hands[1].played_at, 2_000);
    assert_eq!(hands[1].start_stack, Some(500));
}

#[test]
fn fetch_tournament_opponents_lists_distinct_players_met_excluding_hero() {
    let (_db_dir, store, _profile_id) = open_store();

    insert(
        &store,
        &base_hand(
            "#1-1-1",
            "T1",
            1,
            1_000,
            vec![seat(1, "Hero", 500), seat(2, "V1", 500), seat(3, "V2", 500)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "V2", ActionKind::Fold),
                base_action(Street::Preflop, "Hero", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("V1", 30)])],
        ),
    );
    insert(
        &store,
        &base_hand(
            "#1-1-2",
            "T1",
            1,
            2_000,
            vec![seat(1, "Hero", 490), seat(2, "V1", 510)],
            vec![
                post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                base_action(Street::Preflop, "Hero", ActionKind::Fold),
            ],
            "",
            vec![pot(30, &[("V1", 30)])],
        ),
    );

    let reader = store.reader().expect("reader connection");
    let tournament_id: i64 = reader
        .query_row(
            "SELECT id FROM tournaments WHERE room_tournament_id = 'T1'",
            [],
            |row| row.get(0),
        )
        .expect("T1 should exist");

    let opponents =
        fetch_tournament_opponents(&reader, tournament_id).expect("opponents query should succeed");

    assert_eq!(opponents.len(), 2, "{opponents:?}");
    let v1 = opponents
        .iter()
        .find(|o| o.screen_name == "V1")
        .expect("V1 should be listed");
    assert_eq!(v1.hands_together, 2);
    let v2 = opponents
        .iter()
        .find(|o| o.screen_name == "V2")
        .expect("V2 should be listed");
    assert_eq!(v2.hands_together, 1);
    assert!(
        opponents.iter().all(|o| o.screen_name != "Hero"),
        "Hero ne doit pas apparaitre dans ses propres adversaires"
    );
}
