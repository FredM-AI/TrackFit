//! `fetch_hand_class_grid` (M7-7, PRD §13.5) : agregation reelle sur des
//! mains inserees via `gr-store` (meme discipline que `tests/report.rs`) —
//! `hand_class`/`net_bb` sont donc calcules par le vrai pipeline
//! `gr-stats`/`gr-store::repo`, pas par ce test.

use gr_analytics::fetch_hand_class_grid;
use gr_core::{
    ActionKind, ActionRecord, Card, Chips, HandRecord, PotKind, PotResult, Rank, SeatInfo, Street,
    Suit,
};
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

/// Relance a `to` jetons au total sur la street, avec le meme `amount`
/// incremental que le vrai parser (`to - street_contrib_precedent`) : ici
/// toujours une premiere relance de la street (`street_contrib` = 0), donc
/// `amount == to`. `compute_net_chips` (M6-3) lit `amount`, pas
/// `to_amount` — les deux doivent etre coherents pour ce test.
fn raise_to(pseudo: &str, to: i64) -> ActionRecord {
    let mut a = action(Street::Preflop, pseudo, ActionKind::Raise);
    a.amount = Some(Chips::from_i64(to));
    a.to_amount = Some(Chips::from_i64(to));
    a
}

fn fold(pseudo: &str) -> ActionRecord {
    action(Street::Preflop, pseudo, ActionKind::Fold)
}

fn base_hand(
    room_hand_id: &str,
    hero_cards: (Card, Card),
    actions: Vec<ActionRecord>,
) -> HandRecord {
    HandRecord {
        room_hand_id: room_hand_id.to_string(),
        tournament_name: "T".to_string(),
        tournament_room_id: "1".to_string(),
        table_name: "T(1)#1".to_string(),
        table_max_seats: 3,
        button_seat: 1,
        level: 1,
        sb: Chips::from_i64(10),
        bb: Chips::from_i64(20),
        ante: Chips::ZERO,
        played_at: 0,
        seats: vec![
            seat(1, "Hero", 2_000),
            seat(2, "V1", 2_000),
            seat(3, "V2", 2_000),
        ],
        actions,
        board: vec![],
        pots: vec![],
        total_pot: Chips::ZERO,
        rake: Chips::ZERO,
        uncalled_excess: Chips::ZERO,
        hero_pseudo: Some("Hero".to_string()),
        hero_cards: Some(hero_cards),
        parser_version: "0.0.1".to_string(),
    }
}

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit)
}

/// Hero (BTN) ouvre avec AA, tout le monde se couche : VPIP/PFR vrais,
/// remporte le pot non conteste (10 + 20 + 60 = 90, net = +30 = +1.5 bb).
fn hand_hero_opens_aa() -> HandRecord {
    let mut hand = base_hand(
        "#1-1-1",
        (card(Rank::Ace, Suit::Spades), card(Rank::Ace, Suit::Hearts)),
        vec![
            post(Street::Preflop, "V1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V2", ActionKind::PostBigBlind, 20),
            raise_to("Hero", 60),
            fold("V1"),
            fold("V2"),
        ],
    );
    hand.pots = vec![PotResult {
        pot: PotKind::Pot,
        amount: Chips::from_i64(90),
        winners: vec![("Hero".to_string(), Chips::from_i64(90))],
    }];
    hand
}

/// Hero (BTN) se couche avec 72o : aucune opportunite VPIP/PFR gagnee
/// (fold direct), perd sa mise (ici aucune, BTN fold sans poster).
fn hand_hero_folds_72o(room_hand_id: &str) -> HandRecord {
    base_hand(
        room_hand_id,
        (
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Two, Suit::Diamonds),
        ),
        vec![
            post(Street::Preflop, "V1", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V2", ActionKind::PostBigBlind, 20),
            fold("Hero"),
            fold("V1"),
        ],
    )
}

#[test]
fn grid_groups_by_hand_class_with_frequency_vpip_pfr_and_average_net_bb() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let hand1 = hand_hero_opens_aa();
    let hand2 = hand_hero_folds_72o("#1-1-2");
    let hand3 = hand_hero_folds_72o("#1-1-3");
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
    let mut grid = fetch_hand_class_grid(&reader, profile_id).expect("grid should run");
    grid.sort_by(|a, b| a.hand_class.cmp(&b.hand_class));

    assert_eq!(grid.len(), 2, "deux hand_class distinctes : {grid:?}");

    let aa = grid.iter().find(|c| c.hand_class == "AA").expect("AA cell");
    assert_eq!(aa.hands_played, 1);
    assert_eq!(aa.vpip.opportunities, 1);
    assert_eq!(aa.vpip.actions, 1);
    assert_eq!(aa.pfr.opportunities, 1);
    assert_eq!(aa.pfr.actions, 1);
    assert_eq!(aa.avg_net_bb, Some(1.5)); // 30 jetons / 20 bb

    let offsuit = grid
        .iter()
        .find(|c| c.hand_class == "72o")
        .expect("72o cell");
    assert_eq!(offsuit.hands_played, 2);
    assert_eq!(offsuit.vpip.opportunities, 2);
    assert_eq!(offsuit.vpip.actions, 0, "fold direct : pas de VPIP");
    assert_eq!(offsuit.pfr.opportunities, 2);
    assert_eq!(offsuit.pfr.actions, 0);
    assert_eq!(
        offsuit.avg_net_bb,
        Some(0.0),
        "Hero ne poste rien en BTN, fold cout 0"
    );
}

#[test]
fn grid_is_empty_without_any_hand() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");

    let reader = store.reader().expect("reader connection");
    let grid = fetch_hand_class_grid(&reader, profile_id).expect("grid should run");
    assert!(grid.is_empty());
}
