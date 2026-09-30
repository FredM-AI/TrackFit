//! Valide le CA de perf de M6-5 (NFR-P7) : premier affichage de la liste
//! des mains < 500 ms a 1 M mains. Ignore par defaut (seme 1 M lignes
//! `hands`/`hand_players`, plusieurs minutes) :
//! `cargo test -p gr-analytics --release -- --ignored --nocapture perf_hands`.
//!
//! Mesure `count_hero_hands` + `fetch_hero_hands_page` (premiere page,
//! meme requete que `src-tauri::hands::get_hands_page`). Seeding en SQL
//! brut (pas via `gr-store::insert_hands`), meme raisonnement que
//! `tests/perf_home.rs` : ce test mesure le cout de la **requete de
//! lecture**, pas de l'import. Pas de tags semes (hors du chemin mesure :
//! `GROUP_CONCAT` sur un `LEFT JOIN` vide a un cout negligeable).

use gr_analytics::{count_hero_hands, fetch_hero_hands_page};
use gr_parser_api::Room;
use gr_store::Store;
use rusqlite::params;

const HAND_COUNT: i64 = 1_000_000;
const CHUNK_SIZE: i64 = 5_000;
const PAGE_SIZE: i64 = 100;
/// Etalees sur ~400 jours, meme convention que `perf_home.rs`.
const SPAN_MS: i64 = 400 * 24 * 3_600_000;

#[test]
#[ignore = "seme 1M mains ; lancer explicitement avec --ignored"]
fn perf_hands_first_page_stays_under_500ms_at_1m_hands() {
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
        let mut inserted = 0i64;
        while inserted < HAND_COUNT {
            let end = (inserted + CHUNK_SIZE).min(HAND_COUNT);
            let tx = writer.transaction().expect("open a hands chunk");
            for i in inserted..end {
                let played_at = i * (SPAN_MS / HAND_COUNT);
                tx.execute(
                    "INSERT INTO hands (room_id, room_hand_id, played_at, level, board, parser_version, hero_player_id, bb)
                     VALUES (?1, ?2, ?3, 1, 'Kc Qd Jc Ts 3h', '0.0.1', ?4, 20)",
                    params![room_id, format!("#perf-{i}"), played_at, hero_player_id],
                )
                .expect("insert a synthetic hand");
                let hand_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO hand_players (
                        hand_id, player_id, is_hero, position, eff_stack_bb,
                        hole_cards, preflop_line, net_bb, allin_ev_diff_chips
                    ) VALUES (?1, ?2, 1, 'BTN', 45.0, 'Ah Ad', 'RFI', 2.5, NULL)",
                    params![hand_id, hero_player_id],
                )
                .expect("insert a synthetic hand_players row");
            }
            tx.commit().expect("commit a hands chunk");
            inserted = end;
        }
    }
    eprintln!(
        "seed : {HAND_COUNT} mains en {:?} (hors mesure NFR-P7)",
        seed_start.elapsed()
    );

    let reader = store.reader().expect("reader connection");
    let start = std::time::Instant::now();

    let total =
        count_hero_hands(&reader, profile_id, None, None).expect("count_hero_hands should succeed");
    let page = fetch_hero_hands_page(&reader, profile_id, PAGE_SIZE, 0, None, None)
        .expect("fetch_hero_hands_page should succeed");

    let elapsed = start.elapsed();
    eprintln!("NFR-P7 : premiere page de la liste des mains sur {HAND_COUNT} mains en {elapsed:?}");

    assert_eq!(total, HAND_COUNT);
    assert_eq!(page.len(), usize::try_from(PAGE_SIZE).unwrap_or(0));
    assert_eq!(page[0].played_at, (HAND_COUNT - 1) * (SPAN_MS / HAND_COUNT));
    assert!(
        elapsed.as_millis() < 500,
        "premier affichage trop lent : {elapsed:?} (cible NFR-P7 < 500 ms)"
    );
}
