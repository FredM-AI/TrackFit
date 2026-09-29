//! Integration tests des requetes gr-analytics de l'ecran Resultats (M6-3,
//! phase 1) : historique de jetons (G2), volume de tournois par jour (G6),
//! pivot buy-in x KO/non-KO. Meme convention que `tests/home.rs` : un vrai
//! `Store` SQLite, pas de fixture SQL a la main pour les parties qui font
//! des requetes ; le pivot (fonction pure) est teste directement.

use gr_analytics::{
    fetch_hero_chip_history, fetch_hero_tournament_volume_by_day, pivot_by_buyin_and_ko,
    pivot_to_csv, Bullet, TournamentResult, TournamentResultRow,
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
        level: 1,
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

/// Le `TempDir` doit rester vivant aussi longtemps que le `Store` (sinon le
/// dossier de la base est supprime sous ses pieds) : renvoye a l'appelant
/// pour qu'il le garde en vie jusqu'a la fin du test.
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
fn fetch_hero_chip_history_orders_chronologically_and_adjusts_for_a_real_all_in() {
    let (_db_dir, store, profile_id) = open_store();

    // Main 1 (t=1000) : Hero se couche preflop, perd sa BB (pas d'all-in).
    let hand1 = base_hand(
        "#1-1-1",
        "T1",
        1_000,
        vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
        vec![
            post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
            post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
            base_action(Street::Preflop, "Hero", ActionKind::Fold),
        ],
        "",
        vec![pot(30, &[("V1", 30)])],
    );
    // Main 2 (t=2000) : Hero (AA) shove preflop, V1 (72o) suit, Hero gagne
    // tout (1000) alors qu'il n'est pas favori a 100% : diff EV positif.
    let hand2 = base_hand(
        "#1-1-2",
        "T1",
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
    );

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
    let history =
        fetch_hero_chip_history(&reader, profile_id).expect("chip history query should succeed");

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].played_at, 1_000);
    assert!(
        (history[0].net_bb - (-0.5)).abs() < 1e-9,
        "{:?}",
        history[0]
    ); // -10 bb / 20

    assert_eq!(history[1].played_at, 2_000);
    assert!((history[1].net_bb - 25.0).abs() < 1e-9, "{:?}", history[1]); // (1000-500)/20
                                                                          // Hero a gagne alors qu'il n'etait pas favori a 100% : diff EV > 0,
                                                                          // donc la courbe ajustee est en dessous de la courbe reelle.
    assert!(
        history[1].ev_adjusted_net_bb < history[1].net_bb,
        "{:?}",
        history[1]
    );
}

#[test]
fn fetch_hero_tournament_volume_by_day_buckets_tournaments_by_utc_calendar_day() {
    let (_db_dir, store, profile_id) = open_store();
    const DAY_MS: i64 = 86_400_000;

    // Deux tournois du meme jour (T1, T2) et un du lendemain (T3).
    let seeds = [("T1", 1_000), ("T2", 2_000), ("T3", DAY_MS + 3_000)];
    let hands: Vec<HandRecord> = seeds
        .iter()
        .enumerate()
        .map(|(i, (tid, played_at))| {
            base_hand(
                &format!("#{i}-1-1"),
                tid,
                *played_at,
                vec![seat(1, "Hero", 500), seat(2, "V1", 500)],
                vec![
                    post(Street::Preflop, "Hero", ActionKind::PostSmallBlind, 10),
                    post(Street::Preflop, "V1", ActionKind::PostBigBlind, 20),
                ],
                "",
                vec![pot(30, &[("V1", 30)])],
            )
        })
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
    let volume = fetch_hero_tournament_volume_by_day(&reader, profile_id)
        .expect("volume query should succeed");

    assert_eq!(volume.len(), 2, "2 jours distincts : {volume:?}");
    assert_eq!(volume[0].day_epoch_ms, 0);
    assert_eq!(volume[0].tournaments_count, 2, "T1 et T2 le meme jour");
    assert_eq!(volume[1].day_epoch_ms, DAY_MS);
    assert_eq!(volume[1].tournaments_count, 1, "T3 le lendemain");
}

fn cash_bullet(buyin_prize_cents: i64, buyin_fee_cents: i64, prize_won_cents: i64) -> Bullet {
    Bullet {
        buyin_prize_cents,
        buyin_bounty_cents: 0,
        buyin_fee_cents,
        paid_with_ticket_face_value_cents: None,
        prize_won_cents,
        bounty_won_cents: 0,
        ticket_won_face_value_cents: None,
    }
}

fn result_row(tournament_id: i64, is_ko: bool, bullet: Bullet) -> TournamentResultRow {
    TournamentResultRow {
        tournament_id,
        name: "T".to_string(),
        started_at: Some(0),
        result: TournamentResult {
            bullets: vec![bullet],
            is_ko,
            is_freeroll: false,
        },
    }
}

#[test]
fn pivot_groups_by_buyin_bracket_and_ko_and_computes_kpis_per_group() {
    let rows = vec![
        // 1,80€ buy-in (< 2€), non-KO, gagne 5€.
        result_row(1, false, cash_bullet(160, 20, 500)),
        // Autre tournoi < 2€ non-KO, perd tout.
        result_row(2, false, cash_bullet(160, 20, 0)),
        // 9€ buy-in (5-10€), KO, perd tout.
        result_row(3, true, cash_bullet(850, 50, 0)),
    ];

    let pivot = pivot_by_buyin_and_ko(&rows);

    assert_eq!(pivot.len(), 2, "2 groupes non vides : {pivot:?}");

    let small_non_ko = pivot
        .iter()
        .find(|r| r.buyin_min_cents == 0 && !r.is_ko)
        .expect("le groupe < 2€ non-KO devrait exister");
    assert_eq!(small_non_ko.buyin_max_cents, Some(200));
    assert_eq!(small_non_ko.kpis.tournaments_count, 2);
    assert_eq!(small_non_ko.kpis.cost_cents, 360); // 2 x 180
    assert_eq!(small_non_ko.kpis.profit_cents, 140); // 500 - 360

    let mid_ko = pivot
        .iter()
        .find(|r| r.buyin_min_cents == 500 && r.is_ko)
        .expect("le groupe 5-10€ KO devrait exister");
    assert_eq!(mid_ko.buyin_max_cents, Some(1000));
    assert_eq!(mid_ko.kpis.tournaments_count, 1);
    assert_eq!(mid_ko.kpis.profit_cents, -900);
}

#[test]
fn pivot_to_csv_has_a_header_and_one_line_per_row() {
    let rows = vec![result_row(1, false, cash_bullet(160, 20, 500))];
    let pivot = pivot_by_buyin_and_ko(&rows);

    let csv = pivot_to_csv(&pivot);
    let lines: Vec<&str> = csv.lines().collect();

    assert_eq!(lines.len(), 2, "1 en-tete + 1 ligne : {csv}");
    assert!(lines[0].starts_with("buyin_min_cents;"));
    assert!(lines[1].contains(";320;")); // profit_cents = 500 - (160+20)
}
