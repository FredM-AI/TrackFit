//! Valide le CA de perf de M4-7 (NFR-P6, version SQLite) : rapport a 2
//! dimensions x 10 stats sur 2 M mains en moins de 8 s. Ignore par defaut
//! (seme 2 M lignes `hand_players`, plusieurs minutes) :
//! `cargo test -p gr-analytics --release -- --ignored --nocapture perf_2m`.
//!
//! Le seeding insere directement en SQL brut (pas via `gr-store::insert_hands`,
//! qui recalculerait les flags `gr-stats` pour 2 M mains synthetiques —
//! hors sujet ici : ce test mesure le cout du **rapport**, pas de
//! l'import, deja mesure separement par `gr-ingest`, M2-3/M2-4/M4-3/M4-4).

use gr_analytics::{AnalyticsBackend, Dimension, Measure, ReportRequest, SqliteAnalyticsBackend};
use gr_parser_api::Room;
use gr_store::Store;
use rusqlite::params;

const HAND_COUNT: i64 = 2_000_000;
const CHUNK_SIZE: i64 = 5_000;
const POSITION_GROUPS: [&str; 4] = ["EP", "MP", "LP", "Blinds"];
const DEPTH_BUCKETS: [&str; 8] = [
    "<10", "10-15", "15-25", "25-40", "40-60", "60-80", "80-100", "100+",
];

#[test]
#[ignore = "seme 2M lignes hand_players ; lancer explicitement avec --ignored"]
fn perf_2m_hands_report_by_position_and_depth_stays_under_8_seconds() {
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
            let tx = writer.transaction().expect("open a chunk transaction");
            for i in inserted..end {
                let position_group = POSITION_GROUPS[(i % 4) as usize];
                let depth_bucket = DEPTH_BUCKETS[(i % 8) as usize];
                let vpip = i % 3 != 0;
                let rfi_opp = i % 5 != 0;
                let rfi = rfi_opp && i % 2 == 0;

                tx.execute(
                    "INSERT INTO hands (room_id, room_hand_id, played_at, parser_version, hero_player_id)
                     VALUES (?1, ?2, ?3, '0.0.1', ?4)",
                    params![room_id, format!("#perf-{i}"), i, hero_player_id],
                )
                .expect("insert a synthetic hand");
                let hand_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO hand_players (
                        hand_id, player_id, is_hero, position_group, eff_depth_bucket,
                        vpip_opp, vpip, rfi_opp, rfi
                    ) VALUES (?1, ?2, 1, ?3, ?4, 1, ?5, ?6, ?7)",
                    params![
                        hand_id,
                        hero_player_id,
                        position_group,
                        depth_bucket,
                        i64::from(vpip),
                        i64::from(rfi_opp),
                        i64::from(rfi),
                    ],
                )
                .expect("insert a synthetic hand_players row");
            }
            tx.commit().expect("commit the chunk");
            inserted = end;
        }
    }
    let seed_elapsed = seed_start.elapsed();
    println!("seed : {HAND_COUNT} lignes hand_players en {seed_elapsed:?} (hors mesure NFR-P6)");

    let reader = store.reader().expect("reader connection");
    let backend = SqliteAnalyticsBackend::new(&reader);
    let request = ReportRequest {
        hero_profile_id: profile_id,
        dimensions: vec![Dimension::PositionGroup, Dimension::DepthBucket],
        measures: Measure::ALL[..10].to_vec(),
    };

    let start = std::time::Instant::now();
    let rows = backend.run_report(&request).expect("report should run");
    let elapsed = start.elapsed();
    println!(
        "NFR-P6 (SQLite) : rapport 2 dimensions x {} stats sur {HAND_COUNT} mains en {elapsed:?} ({} lignes de resultat)",
        request.measures.len(),
        rows.len()
    );
    assert!(
        elapsed.as_secs_f64() < 8.0,
        "cible NFR-P6 (version SQLite) : < 8s, mesure {elapsed:?}"
    );
}
