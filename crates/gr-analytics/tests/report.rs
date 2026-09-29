//! Integration test de `SqliteAnalyticsBackend` (M4-7) : verifie le rapport
//! sur des mains reellement inserees via `gr-store` (pas de fixture SQL a
//! la main) — les colonnes agregees (`vpip_opp`/`vpip`/`rfi_opp`/`rfi`...)
//! sont donc calculees par le vrai pipeline `gr-stats`/M4-3, pas par ce
//! test.

use gr_analytics::{
    AnalyticsBackend, Dimension, Measure, ReportRequest, SqliteAnalyticsBackend, StatCell,
};
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

fn base_hand(room_hand_id: &str, button_seat: u8, actions: Vec<ActionRecord>) -> HandRecord {
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: "1".to_string(),
        table_name: "T(1)#1".to_string(),
        table_max_seats: 3,
        button_seat,
        level: 1,
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
        ante: Chips::ZERO,
        played_at: 0,
        seats: vec![seat(1, "Hero", 500), seat(2, "V1", 500), seat(3, "V2", 500)],
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

/// 3-max, bouton = Hero : Hero (BTN, groupe LP) ouvre, tout le monde se
/// couche. VPIP/PFR/RFI tous vrais pour Hero.
fn hand_hero_opens_from_the_button() -> HandRecord {
    base_hand(
        "#1-1-1",
        1,
        vec![
            post(Street::Preflop, "V1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V2", ActionKind::PostBigBlind, 20),
            raise_to("Hero", 60),
            fold("V1"),
            fold("V2"),
        ],
    )
}

/// 3-max, bouton = V1 : Hero est BB (groupe Blinds), V1 (BTN) ouvre, V2
/// (SB) se couche, Hero suit (VPIP vrai, RFI faux : quelqu'un est deja
/// entre avant sa decision).
fn hand_hero_calls_from_the_big_blind() -> HandRecord {
    base_hand(
        "#1-1-2",
        2,
        vec![
            post(Street::Preflop, "V2", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Hero", ActionKind::PostBigBlind, 20),
            raise_to("V1", 60),
            fold("V2"),
            call("Hero", 40),
        ],
    )
}

#[test]
fn report_groups_vpip_and_rfi_by_position_group() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let hand1 = hand_hero_opens_from_the_button();
    let hand2 = hand_hero_calls_from_the_big_blind();
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
            ],
        )
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    let backend = SqliteAnalyticsBackend::new(&reader);

    let mut rows = backend
        .run_report(&ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![Dimension::PositionGroup],
            measures: vec![Measure::Vpip, Measure::Rfi],
        })
        .expect("report should run");
    rows.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));

    assert_eq!(rows.len(), 2, "un groupe de position par main : {rows:?}");

    let blinds = &rows[0];
    assert_eq!(blinds.dimension_values, vec![Some("Blinds".to_string())]);
    assert_eq!(blinds.cells[0].opportunities, 1); // VPIP opp
    assert_eq!(blinds.cells[0].actions, 1); // VPIP act (call)
    assert_eq!(blinds.cells[1].opportunities, 0); // RFI opp (pas first-in)
    assert_eq!(blinds.cells[1].actions, 0);

    let lp = &rows[1];
    assert_eq!(lp.dimension_values, vec![Some("LP".to_string())]);
    assert_eq!(lp.cells[0].opportunities, 1);
    assert_eq!(lp.cells[0].actions, 1);
    assert_eq!(lp.cells[1].opportunities, 1);
    assert_eq!(lp.cells[1].actions, 1);
}

#[test]
fn report_without_dimensions_returns_a_single_aggregate_row() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let hand1 = hand_hero_opens_from_the_button();
    let hand2 = hand_hero_calls_from_the_big_blind();
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
            ],
        )
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    let backend = SqliteAnalyticsBackend::new(&reader);

    let rows = backend
        .run_report(&ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![],
            measures: vec![Measure::Vpip, Measure::Rfi],
        })
        .expect("report should run");

    assert_eq!(rows.len(), 1);
    assert!(rows[0].dimension_values.is_empty());
    // VPIP : 2 opportunites, 2 actions (Hero a mis volontairement des
    // jetons dans les deux mains).
    assert_eq!(rows[0].cells[0].opportunities, 2);
    assert_eq!(rows[0].cells[0].actions, 2);
    // RFI : 1 seule opportunite (main 1, Hero folded-to en BTN) ; en main
    // 2 il n'est pas first-in (V1 a deja relance), donc pas d'opportunite
    // RFI pour cette main (gr-stats::compute_rfi).
    assert_eq!(rows[0].cells[1].opportunities, 1);
    assert_eq!(rows[0].cells[1].actions, 1);
}

#[test]
fn report_with_no_measures_returns_no_rows() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let reader = store.reader().expect("reader connection");
    let backend = SqliteAnalyticsBackend::new(&reader);

    let rows = backend
        .run_report(&ReportRequest {
            hero_profile_id: 1,
            dimensions: vec![Dimension::PositionGroup],
            measures: vec![],
        })
        .expect("an empty measure list should not error");

    assert!(rows.is_empty());
}

#[test]
fn report_with_no_matching_hands_returns_zero_not_null() {
    // Sans dimension et sans main correspondante, `SUM` sur l'agregat
    // global (une seule ligne, ensemble vide) renvoie `NULL` en SQL, pas
    // `0` : `COALESCE` dans la requete doit l'intercepter plutot que
    // planter (`InvalidColumnType`).
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");

    let reader = store.reader().expect("reader connection");
    let backend = SqliteAnalyticsBackend::new(&reader);

    let rows = backend
        .run_report(&ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![],
            measures: vec![Measure::Vpip],
        })
        .expect("report should run even with zero matching hands");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cells[0], StatCell::default());
}
