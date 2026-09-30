//! CA de M7-6 : `DuckDbAnalyticsBackend` doit renvoyer exactement les memes
//! resultats que `SqliteAnalyticsBackend` pour la meme requete, sur les
//! memes mains (`gr-store` reelle, meme pipeline `gr-stats` qu'en
//! production — pas une fixture SQL a la main, meme discipline que
//! `tests/report.rs`). Feature-gate sur `analytics-duckdb` : ce fichier ne
//! compile pas dans le build par defaut (`just dev`/CI standard).
#![cfg(feature = "analytics-duckdb")]

use gr_analytics::{
    open_duckdb, sync_incremental, AnalyticsBackend, Dimension, DuckDbAnalyticsBackend, Measure,
    ReportRequest, SqliteAnalyticsBackend,
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

fn base_hand(
    room_hand_id: &str,
    button_seat: u8,
    stack: i64,
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
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
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

fn hand_hero_opens_from_the_button() -> HandRecord {
    base_hand(
        "#1-1-1",
        1,
        5_000,
        vec![
            post(Street::Preflop, "V1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V2", ActionKind::PostBigBlind, 20),
            raise_to("Hero", 60),
            fold("V1"),
            fold("V2"),
        ],
    )
}

fn hand_hero_calls_from_the_big_blind() -> HandRecord {
    base_hand(
        "#1-1-2",
        2,
        1_200,
        vec![
            post(Street::Preflop, "V2", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "Hero", ActionKind::PostBigBlind, 20),
            raise_to("V1", 60),
            fold("V2"),
            call("Hero", 40),
        ],
    )
}

fn hand_hero_folds_from_the_small_blind() -> HandRecord {
    base_hand(
        "#1-1-3",
        3,
        300,
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
            raise_to("V2", 60),
            fold("Hero"),
            fold("V1"),
        ],
    )
}

/// Prepare un `Store` avec 3 mains variees (positions/profondeurs
/// differentes) et son `analytics.duckdb` synchronise en entier —
/// reutilise par chaque test d'equivalence.
fn seeded_store_with_synced_duckdb() -> (tempfile::TempDir, Store, duckdb::Connection, i64) {
    let db_dir = tempfile::tempdir().expect("temp dir for the sqlite database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let hand1 = hand_hero_opens_from_the_button();
    let hand2 = hand_hero_calls_from_the_big_blind();
    let hand3 = hand_hero_folds_from_the_small_blind();
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

    let duck_dir = tempfile::tempdir().expect("temp dir for analytics.duckdb");
    let duck = open_duckdb(&duck_dir.path().join("analytics.duckdb")).expect("duckdb should open");
    {
        let reader = store.reader().expect("reader connection");
        sync_incremental(&duck, &reader).expect("sync should succeed");
    }

    (duck_dir, store, duck, profile_id)
}

fn assert_same_report(store: &Store, duck: &duckdb::Connection, request: &ReportRequest) {
    let reader = store.reader().expect("reader connection");
    let sqlite_backend = SqliteAnalyticsBackend::new(&reader);
    let duckdb_backend = DuckDbAnalyticsBackend::new(duck);

    let mut sqlite_rows = sqlite_backend
        .run_report(request)
        .expect("sqlite report should run");
    let mut duckdb_rows = duckdb_backend
        .run_report(request)
        .expect("duckdb report should run");
    sqlite_rows.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));
    duckdb_rows.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));

    assert_eq!(
        sqlite_rows, duckdb_rows,
        "SQLite et DuckDB devraient renvoyer exactement les memes lignes pour {request:?}"
    );
}

#[test]
fn one_dimension_matches_between_backends() {
    let (_duck_dir, store, duck, profile_id) = seeded_store_with_synced_duckdb();
    assert_same_report(
        &store,
        &duck,
        &ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![Dimension::PositionGroup],
            measures: vec![Measure::Vpip, Measure::Pfr, Measure::Rfi],
        },
    );
}

#[test]
fn two_dimensions_and_the_full_measure_set_match_between_backends() {
    let (_duck_dir, store, duck, profile_id) = seeded_store_with_synced_duckdb();
    assert_same_report(
        &store,
        &duck,
        &ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![Dimension::DepthBucket, Dimension::PositionGroup],
            measures: Measure::ALL.to_vec(),
        },
    );
}

#[test]
fn no_dimension_aggregate_matches_between_backends() {
    let (_duck_dir, store, duck, profile_id) = seeded_store_with_synced_duckdb();
    assert_same_report(
        &store,
        &duck,
        &ReportRequest {
            hero_profile_id: profile_id,
            dimensions: vec![],
            measures: vec![Measure::Vpip, Measure::Rfi],
        },
    );
}

#[test]
fn rebuild_reproduces_the_same_data_as_an_incremental_sync() {
    let (_duck_dir, store, duck, profile_id) = seeded_store_with_synced_duckdb();
    let request = ReportRequest {
        hero_profile_id: profile_id,
        dimensions: vec![Dimension::PositionGroup],
        measures: vec![Measure::Vpip],
    };
    let reader = store.reader().expect("reader connection");
    let mut before = DuckDbAnalyticsBackend::new(&duck)
        .run_report(&request)
        .expect("report before rebuild");
    before.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));

    gr_analytics::rebuild(&duck, &reader).expect("rebuild should succeed");

    let mut after = DuckDbAnalyticsBackend::new(&duck)
        .run_report(&request)
        .expect("report after rebuild");
    after.sort_by(|a, b| a.dimension_values.cmp(&b.dimension_values));
    assert_eq!(
        before, after,
        "une reconstruction ne doit rien changer au resultat"
    );
}
