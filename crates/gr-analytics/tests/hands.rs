//! Integration tests des requetes gr-analytics de l'ecran Mains (M6-5,
//! PRD §13.4) : comptage et pagination de la liste, tags appliques. Meme
//! convention que `tests/home.rs` : un vrai `Store` SQLite.

use gr_analytics::{count_hero_hands, fetch_hero_hands_page};
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

fn base_hand(room_hand_id: &str, played_at: i64) -> HandRecord {
    let pots = vec![pot(30, &[("V1", 30)])];
    let total_pot: i64 = pots.iter().map(|p| p.amount.amount()).sum();
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: "T1".to_string(),
        table_name: "T1#1".to_string(),
        table_max_seats: 2,
        button_seat: 1,
        level: 1,
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
        ante: Chips::ZERO,
        played_at,
        seats: vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
        actions: vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
            base_action(Street::Preflop, "Hero", ActionKind::Fold),
        ],
        board: cards(""),
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

#[test]
fn count_and_page_reflect_the_total_and_return_the_most_recent_hands_first() {
    let (_db_dir, store, profile_id) = open_store();

    let hands: Vec<HandRecord> = (0..5)
        .map(|i| base_hand(&format!("#{i}"), i * 1_000))
        .collect();
    let inserts: Vec<HandInsert<'_>> = hands
        .iter()
        .map(|hand| HandInsert {
            hand,
            raw_text: "irrelevant",
        })
        .collect();
    store
        .insert_hands(Room::Winamax, &inserts)
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    let total = count_hero_hands(&reader, profile_id, None, None).expect("count should succeed");
    assert_eq!(total, 5);

    let page = fetch_hero_hands_page(&reader, profile_id, 2, 0, None, None)
        .expect("page 0 should succeed");
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].played_at, 4_000, "la plus recente d'abord");
    assert_eq!(page[1].played_at, 3_000);
    assert_eq!(page[0].tournament_name.as_deref(), Some("T"));

    let page2 = fetch_hero_hands_page(&reader, profile_id, 2, 4, None, None)
        .expect("page 2 should succeed");
    assert_eq!(page2.len(), 1, "dernier reste : la plus ancienne");
    assert_eq!(page2[0].played_at, 0);
}

#[test]
fn count_and_page_respect_the_since_until_range_from_the_m6_1_filter_panel() {
    let (_db_dir, store, profile_id) = open_store();

    let hands: Vec<HandRecord> = (0..5)
        .map(|i| base_hand(&format!("#{i}"), i * 1_000))
        .collect();
    let inserts: Vec<HandInsert<'_>> = hands
        .iter()
        .map(|hand| HandInsert {
            hand,
            raw_text: "irrelevant",
        })
        .collect();
    store
        .insert_hands(Room::Winamax, &inserts)
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    // [1000, 3000) : mains a 1000 et 2000 seulement (bornes PRD demi-ouvertes).
    let total = count_hero_hands(&reader, profile_id, Some(1_000), Some(3_000))
        .expect("count should succeed");
    assert_eq!(total, 2);

    let page = fetch_hero_hands_page(&reader, profile_id, 10, 0, Some(1_000), Some(3_000))
        .expect("page should succeed");
    let played_ats: Vec<i64> = page.iter().map(|r| r.played_at).collect();
    assert_eq!(played_ats, vec![2_000, 1_000]);
}

#[test]
fn tags_applied_to_a_hand_are_returned_as_their_label_keys() {
    let (_db_dir, store, profile_id) = open_store();

    let hand = base_hand("#1", 1_000);
    store
        .insert_hands(
            Room::Winamax,
            &[HandInsert {
                hand: &hand,
                raw_text: "irrelevant",
            }],
        )
        .expect("insertion should succeed");

    let reader = store.reader().expect("reader connection");
    let hand_id = fetch_hero_hands_page(&reader, profile_id, 1, 0, None, None)
        .expect("page should succeed")[0]
        .hand_id;
    drop(reader);

    let tags = store.list_tags().expect("list tags");
    let bad_beat = tags
        .iter()
        .find(|t| t.label_key == "tags.badBeat")
        .expect("bad beat tag should exist");
    let cooler = tags
        .iter()
        .find(|t| t.label_key == "tags.cooler")
        .expect("cooler tag should exist");
    store
        .tag_hands(&[hand_id], bad_beat.id, 2_000)
        .expect("tag hand");
    store
        .tag_hands(&[hand_id], cooler.id, 3_000)
        .expect("tag hand again with a second tag");

    let reader = store.reader().expect("reader connection");
    let page =
        fetch_hero_hands_page(&reader, profile_id, 1, 0, None, None).expect("page should succeed");
    let mut tag_keys = page[0].tag_label_keys.clone();
    tag_keys.sort();
    assert_eq!(
        tag_keys,
        vec!["tags.badBeat".to_string(), "tags.cooler".to_string()]
    );
}
