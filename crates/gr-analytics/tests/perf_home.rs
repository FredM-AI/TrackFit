//! Valide le CA de perf de M6-2 (NFR-P5) : l'ecran Accueil reste < 1 s a 1 M
//! mains. Ignore par defaut (seme 1 M lignes `hands`/`hand_players`,
//! plusieurs minutes) :
//! `cargo test -p gr-analytics --release -- --ignored --nocapture perf_home`.
//!
//! Mesure les deux requetes qui dominent le cout de
//! `src-tauri::home::get_home_snapshot` (`fetch_hero_tournament_results` et
//! `hero_allin_ev_diff_bb`, appelees deux fois — periode courante et
//! precedente — plus une fois non filtrees pour la courbe G1) et
//! `gr-store::total_session_ms_in_range`. Seeding en SQL brut (pas via
//! `gr-store::insert_hands`, meme raisonnement que `tests/perf.rs`, M4-7 :
//! ce test mesure le cout du **calcul des KPIs**, pas de l'import).

use gr_analytics::{fetch_hero_tournament_results, hero_allin_ev_diff_bb};
use gr_parser_api::Room;
use gr_store::Store;
use rusqlite::params;

const HAND_COUNT: i64 = 1_000_000;
const TOURNAMENT_COUNT: i64 = 5_000;
const HANDS_PER_TOURNAMENT: i64 = HAND_COUNT / TOURNAMENT_COUNT;
const SESSION_COUNT: i64 = 2_000;
const CHUNK_SIZE: i64 = 5_000;
/// Etalees sur ~400 jours : une fenetre de 30 jours (M6-2) ne couvre donc
/// qu'une fraction realiste du total, pas l'integralite du corpus.
const SPAN_MS: i64 = 400 * 24 * 3_600_000;

#[test]
#[ignore = "seme 1M mains ; lancer explicitement avec --ignored"]
fn perf_home_kpis_stay_under_one_second_at_1m_hands() {
    let db_dir = tempfile::tempdir().expect("temp dir for the test database");
    let store = Store::open(db_dir.path()).expect("store should open");
    let profile_id = store
        .create_hero_profile("Hero", true)
        .expect("create hero profile");
    store
        .link_hero_account(profile_id, Room::Winamax, "Hero")
        .expect("link Hero pseudo to the profile");

    let (room_id, hero_player_id): (i64, i64) = {
        let reader = store.reader().expect("reader connection");
        let room_id = reader
            .query_row("SELECT id FROM rooms WHERE code = 'winamax'", [], |row| {
                row.get(0)
            })
            .expect("winamax room should exist");
        let hero_player_id = reader
            .query_row(
                "SELECT id FROM players WHERE screen_name = 'Hero'",
                [],
                |row| row.get(0),
            )
            .expect("hero player should exist");
        (room_id, hero_player_id)
    };

    let seed_start = std::time::Instant::now();
    {
        let mut writer = store.writer();

        // Tournois + entree/bullet Hero (1 bullet chacun, pas de re-entry :
        // hors sujet ici, deja couvert par `tests/home.rs`).
        {
            let tx = writer.transaction().expect("open the tournaments chunk");
            for t in 0..TOURNAMENT_COUNT {
                let started_at = t * (SPAN_MS / TOURNAMENT_COUNT);
                tx.execute(
                    "INSERT INTO tournaments (
                        room_id, room_tournament_id, name, started_at,
                        buyin_prize_cents, buyin_bounty_cents, buyin_fee_cents,
                        ko_type, is_freeroll, status
                    ) VALUES (?1, ?2, 'T', ?3, 180, 0, 20, 'NONE', 0, 'COMPLETE')",
                    params![room_id, format!("perf-{t}"), started_at],
                )
                .expect("insert a synthetic tournament");
                let tournament_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO tournament_entries (
                        tournament_id, player_id, entries_count, finish_position,
                        prize_cents, bounty_cents
                    ) VALUES (?1, ?2, 1, 1, 500, 0)",
                    params![tournament_id, hero_player_id],
                )
                .expect("insert a synthetic tournament entry");
                let entry_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO tournament_bullets (
                        entry_id, entry_no, finish_position, played_seconds,
                        prize_cents, bounty_cents
                    ) VALUES (?1, 1, 1, 600, 500, 0)",
                    params![entry_id],
                )
                .expect("insert a synthetic tournament bullet");
            }
            tx.commit().expect("commit the tournaments chunk");
        }

        // Sessions (table separee, pas liee via hands.session_id ici : hors
        // du chemin mesure, cf. doc de tete de fichier).
        {
            let tx = writer.transaction().expect("open the sessions chunk");
            for s in 0..SESSION_COUNT {
                let started_at = s * (SPAN_MS / SESSION_COUNT);
                tx.execute(
                    "INSERT INTO sessions (hero_profile_id, started_at, ended_at, hands, tournaments, max_tables)
                     VALUES (?1, ?2, ?3, 500, 4, 1)",
                    params![profile_id, started_at, started_at + 3_600_000],
                )
                .expect("insert a synthetic session");
            }
            tx.commit().expect("commit the sessions chunk");
        }

        // Mains + hand_players (1 M, chacune rattachee a un tournoi et
        // etalee dans le temps).
        let mut inserted = 0i64;
        while inserted < HAND_COUNT {
            let end = (inserted + CHUNK_SIZE).min(HAND_COUNT);
            let tx = writer.transaction().expect("open a hands chunk");
            for i in inserted..end {
                let tournament_index = i / HANDS_PER_TOURNAMENT.max(1);
                let tournament_id = tournament_index + 1; // 1-based, insere dans le meme ordre ci-dessus
                let played_at = i * (SPAN_MS / HAND_COUNT);
                let has_allin = i % 20 == 0;
                let allin_diff: Option<f64> =
                    has_allin.then_some(if i % 40 == 0 { 12.5 } else { -8.0 });

                tx.execute(
                    "INSERT INTO hands (room_id, room_hand_id, played_at, parser_version, hero_player_id, tournament_id, bb)
                     VALUES (?1, ?2, ?3, '0.0.1', ?4, ?5, 20)",
                    params![room_id, format!("#perf-{i}"), played_at, hero_player_id, tournament_id],
                )
                .expect("insert a synthetic hand");
                let hand_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO hand_players (hand_id, player_id, is_hero, allin_ev_diff_chips)
                     VALUES (?1, ?2, 1, ?3)",
                    params![hand_id, hero_player_id, allin_diff],
                )
                .expect("insert a synthetic hand_players row");
            }
            tx.commit().expect("commit the hands chunk");
            inserted = end;
        }
    }
    let seed_elapsed = seed_start.elapsed();
    println!("seed : {HAND_COUNT} mains + {TOURNAMENT_COUNT} tournois en {seed_elapsed:?} (hors mesure NFR-P5)");

    let reader = store.reader().expect("reader connection");
    let now_ms = SPAN_MS;
    let current_start = now_ms - 30 * 24 * 3_600_000;
    let previous_start = now_ms - 60 * 24 * 3_600_000;

    let start = std::time::Instant::now();

    // Periode courante (30 derniers jours) + precedente : meme sequence que
    // `src-tauri::home::period_kpis`, appelee deux fois par
    // `get_home_snapshot`.
    for (since, until) in [
        (Some(current_start), Some(now_ms)),
        (Some(previous_start), Some(current_start)),
    ] {
        let results = fetch_hero_tournament_results(&reader, profile_id, since, until, None)
            .expect("fetch_hero_tournament_results should succeed");
        std::hint::black_box(&results);
        let diff_bb = hero_allin_ev_diff_bb(&reader, profile_id, since, until)
            .expect("hero_allin_ev_diff_bb should succeed");
        std::hint::black_box(diff_bb);
        let session_ms = store
            .total_session_ms_in_range(profile_id, since, until)
            .expect("total_session_ms_in_range should succeed");
        std::hint::black_box(session_ms);
    }

    // Courbe G1 : tout l'historique, non filtre.
    let all_results = fetch_hero_tournament_results(&reader, profile_id, None, None, None)
        .expect("unfiltered fetch_hero_tournament_results should succeed");
    assert_eq!(
        i64::try_from(all_results.len()).unwrap_or(-1),
        TOURNAMENT_COUNT
    );

    let elapsed = start.elapsed();
    println!("NFR-P5 : KPIs Accueil (periode courante + precedente + courbe G1) sur {HAND_COUNT} mains en {elapsed:?}");
    assert!(
        elapsed.as_secs_f64() < 1.0,
        "cible NFR-P5 : < 1s, mesure {elapsed:?}"
    );
}
