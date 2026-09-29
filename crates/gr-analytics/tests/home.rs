//! Integration tests des requetes gr-analytics de l'ecran Accueil (M6-2,
//! PRD §13.1) : resultats de tournois du Hero et ecart EV all-in en bb, sur
//! un vrai `Store` SQLite (pas de fixture SQL a la main), meme convention
//! que `tests/report.rs`.

use gr_analytics::{fetch_hero_tournament_results, hero_allin_ev_diff_bb};
use gr_core::{
    ActionKind, ActionRecord, Card, Chips, HandRecord, Money, PotKind, PotResult, SeatInfo, Street,
    TournamentBullet, TournamentSummary,
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
    played_at: i64,
    bb: i64,
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
        level: 1,
        sb: Chips::from_i64(bb / 2),
        bb: Chips::from_i64(bb),
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

fn summary(
    tournament_room_id: &str,
    tournament_type: &str,
    bullets: Vec<TournamentBullet>,
) -> TournamentSummary {
    TournamentSummary {
        room_tournament_id: tournament_room_id.to_string(),
        name: "T".to_string(),
        hero_pseudo: "Hero".to_string(),
        buyin_prize: Money::from_cents(450),
        buyin_bounty: Money::ZERO,
        buyin_fee: Money::from_cents(50),
        mode: "tt".to_string(),
        tournament_type: tournament_type.to_string(),
        speed: "normal".to_string(),
        flight_id: 0,
        bullets,
    }
}

fn bullet(entry_no: u32, finish_position: u32, prize_cents: i64) -> TournamentBullet {
    TournamentBullet {
        entry_no,
        late_reg_bust: false,
        finish_position: Some(finish_position),
        played_seconds: 600,
        prize: Money::from_cents(prize_cents),
        bounty: Money::ZERO,
        registered_snapshot: 100,
    }
}

/// Une main minimale, juste pour que `get_or_create_tournament` fixe
/// `started_at` (M6-2 filtre `fetch_hero_tournament_results` dessus ;
/// `attach_summary` seul ne le renseigne jamais, PRD §8.5).
fn seed_tournament_started_at(store: &Store, tournament_room_id: &str, played_at: i64) {
    let hand = base_hand(
        &format!("#{tournament_room_id}-1-1"),
        tournament_room_id,
        played_at,
        20,
        vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
        ],
        "",
        vec![pot(30, &[("V1", 30)])],
    );
    store
        .insert_hands(
            Room::Winamax,
            &[HandInsert {
                hand: &hand,
                raw_text: "irrelevant",
            }],
        )
        .expect("seed hand should insert");
}

#[test]
fn fetch_hero_tournament_results_aggregates_bullets_of_a_reentry_tournament() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    seed_tournament_started_at(&store, "T1", 1_000);
    store
        .attach_summary(
            Room::Winamax,
            &summary(
                "T1",
                "knockout",
                vec![bullet(1, 500, 0), bullet(2, 3, 2000)],
            ),
        )
        .expect("attach summary should succeed");

    let reader = store.reader().expect("reader connection");
    let results = fetch_hero_tournament_results(&reader, profile_id, None, None)
        .expect("query should succeed");

    assert_eq!(results.len(), 1, "un seul tournoi : {results:?}");
    let t1 = &results[0];
    assert_eq!(t1.started_at, Some(1_000));
    assert!(t1.result.is_ko, "tournament_type=knockout -> is_ko");
    assert!(!t1.result.is_freeroll);
    // 2 bullets (re-entry), chacun avec son propre buy-in et ses propres
    // gains (PRD §5.1 : une ligne `tournament_bullets` par entree).
    assert_eq!(t1.result.bullets.len(), 2);
    assert_eq!(t1.result.bullets[0].buyin_prize_cents, 450);
    assert_eq!(t1.result.bullets[0].buyin_fee_cents, 50);
    assert_eq!(t1.result.bullets[0].prize_won_cents, 0);
    assert_eq!(t1.result.bullets[1].prize_won_cents, 2000);

    // Le resultat alimente directement `compute_results_kpis` (M5-1) sans
    // transformation supplementaire.
    let kpis = gr_analytics::compute_results_kpis(
        std::slice::from_ref(&t1.result),
        gr_analytics::TicketValuation::FaceValue,
    );
    assert_eq!(kpis.tournaments_count, 1);
    assert_eq!(kpis.re_entries_count, 1);
    assert_eq!(kpis.cost_cents, 1000); // 2 x (450+50)
    assert_eq!(kpis.prize_pool_winnings_cents, 2000);
    assert_eq!(kpis.profit_cents, 1000);
}

#[test]
fn fetch_hero_tournament_results_filters_on_the_started_at_range() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    seed_tournament_started_at(&store, "T-OLD", 1_000);
    store
        .attach_summary(
            Room::Winamax,
            &summary("T-OLD", "normal", vec![bullet(1, 1, 100)]),
        )
        .expect("attach summary should succeed");
    seed_tournament_started_at(&store, "T-NEW", 5_000);
    store
        .attach_summary(
            Room::Winamax,
            &summary("T-NEW", "normal", vec![bullet(1, 1, 200)]),
        )
        .expect("attach summary should succeed");

    let reader = store.reader().expect("reader connection");
    let results = fetch_hero_tournament_results(&reader, profile_id, Some(2_000), None)
        .expect("query should succeed");

    assert_eq!(
        results.len(),
        1,
        "seul T-NEW est apres la borne : {results:?}"
    );
    assert_eq!(results[0].started_at, Some(5_000));
}

#[test]
fn fetch_hero_tournament_results_attributes_ticket_use_to_the_first_bullet_and_ticket_win_to_the_last(
) {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");
    seed_tournament_started_at(&store, "T1", 1_000);
    let report = store
        .attach_summary(
            Room::Winamax,
            &summary("T1", "normal", vec![bullet(1, 50, 0), bullet(2, 1, 500)]),
        )
        .expect("attach summary should succeed");

    let used_type = store
        .create_or_update_ticket_type(Room::Winamax, "Ticket 5€", Some(500))
        .expect("create ticket type");
    let won_type = store
        .create_or_update_ticket_type(Room::Winamax, "Ticket 10€", Some(1000))
        .expect("create ticket type");
    let reader = store.reader().expect("reader connection");
    let hero_player_id: i64 = reader
        .query_row(
            "SELECT player_id FROM tournament_entries WHERE tournament_id = ?1",
            [report.tournament_id],
            |row| row.get(0),
        )
        .expect("hero entry should exist");
    store
        .set_tournament_entry_ticket_used(report.tournament_id, hero_player_id, Some(used_type))
        .expect("set ticket used");
    store
        .set_tournament_entry_ticket_won(report.tournament_id, hero_player_id, Some(won_type), 1)
        .expect("set ticket won");

    let results = fetch_hero_tournament_results(&reader, profile_id, None, None)
        .expect("query should succeed");

    assert_eq!(results.len(), 1);
    let bullets = &results[0].result.bullets;
    assert_eq!(bullets.len(), 2);
    assert_eq!(
        bullets[0].paid_with_ticket_face_value_cents,
        Some(500),
        "le ticket utilise est attribue au premier bullet"
    );
    assert_eq!(bullets[1].paid_with_ticket_face_value_cents, None);
    assert_eq!(bullets[0].ticket_won_face_value_cents, None);
    assert_eq!(
        bullets[1].ticket_won_face_value_cents,
        Some(1000),
        "le ticket gagne est attribue au dernier bullet"
    );
}

#[test]
fn hero_allin_ev_diff_bb_is_nonzero_after_a_real_preflop_allin_and_respects_the_date_range() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    // Hero (AA) shove preflop, Villain (72o) suit tapis pour tapis ; riviere
    // distribuee sans action. Hero gagne le pot alors qu'il n'est pas favori
    // a 100 % : diff positif garanti (meme scenario que
    // gr-equity/tests/allin_events.rs::heads_up_all_in_preflop_conserves_equity).
    let hand = base_hand(
        "#1-1-1",
        "T1",
        10_000,
        20,
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
    let diff_bb =
        hero_allin_ev_diff_bb(&reader, profile_id, None, None).expect("query should succeed");
    assert!(
        diff_bb > 0.0,
        "diff_bb = {diff_bb} (Hero a gagne, chanceux)"
    );

    // Hors de la fenetre [11_000, ..) : la main n'est plus comptee.
    let diff_bb_out_of_range = hero_allin_ev_diff_bb(&reader, profile_id, Some(11_000), None)
        .expect("query should succeed");
    assert_eq!(diff_bb_out_of_range, 0.0);
}
