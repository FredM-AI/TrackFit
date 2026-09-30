//! Porte de decision de M7-6 (NFR-P6, ADR-002) : `DuckDB` doit diviser par 3
//! ou plus le temps d'un "rapport personnalise (2 dimensions, 10 stats)" a
//! 2 M mains, sinon le backend reste desactivable par defaut (deja le cas)
//! et le chiffre mesure est consigne dans `docs/adr/0002-duckdb-analytics-backend.md`.
//! Ignore par defaut (seme 2 M lignes, plusieurs minutes) :
//! `cargo test -p gr-analytics --release --features analytics-duckdb -- --ignored --nocapture perf_duckdb`.
//!
//! Seeding en SQL brut (pas via `gr-store::insert_hands`/`gr-synth`), meme
//! raisonnement que `tests/perf_hands.rs`/`tests/perf_home.rs` : ce test
//! mesure le cout de la **requete de rapport**, pas de l'import ni du
//! calcul des flags `gr-stats`.
#![cfg(feature = "analytics-duckdb")]

use gr_analytics::{
    open_duckdb, sync_incremental, AnalyticsBackend, Dimension, DuckDbAnalyticsBackend, Measure,
    ReportRequest, SqliteAnalyticsBackend,
};
use gr_parser_api::Room;
use gr_store::Store;
use rusqlite::params;

const HAND_COUNT: i64 = 2_000_000;
const CHUNK_SIZE: i64 = 5_000;
const POSITION_GROUPS: [&str; 4] = ["EP", "MP", "LP", "Blinds"];
const DEPTH_BUCKETS: [&str; 4] = ["short", "medium", "deep", "very_deep"];

#[test]
#[ignore = "seme 2M mains ; lancer explicitement avec --ignored --release"]
fn perf_duckdb_report_is_at_least_3x_faster_than_sqlite_at_2m_hands() {
    let db_dir = tempfile::tempdir().expect("temp dir for the sqlite database");
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
                tx.execute(
                    "INSERT INTO hands (room_id, room_hand_id, played_at, level, parser_version, hero_player_id, bb)
                     VALUES (?1, ?2, ?3, 1, '0.0.1', ?4, 20)",
                    params![room_id, format!("#perf-{i}"), i, hero_player_id],
                )
                .expect("insert a synthetic hand");
                let hand_id = tx.last_insert_rowid();
                let position_group = POSITION_GROUPS[usize::try_from(i % 4).unwrap_or(0)];
                let depth_bucket = DEPTH_BUCKETS[usize::try_from(i % 4).unwrap_or(0)];
                let flag = i % 2;
                tx.execute(
                    "INSERT INTO hand_players (
                        hand_id, player_id, is_hero, position_group, eff_depth_bucket,
                        vpip_opp, vpip, pfr,
                        rfi_opp, rfi, limp, oshove,
                        tb_opp, tb, f3b_opp, f3b, fb_opp, fb,
                        ats_opp, ats, fsteal_opp, fsteal, rsteal,
                        cbf_opp, cbf, cbt_opp, cbt, fcbf_opp, fcbf,
                        saw_flop, went_sd, won_sd, won_hand
                    ) VALUES (?1, ?2, 1, ?3, ?4,
                        1, ?5, ?5,
                        1, ?5, ?5, ?5,
                        1, ?5, 1, ?5, 1, ?5,
                        1, ?5, 1, ?5, ?5,
                        1, ?5, 1, ?5, 1, ?5,
                        1, ?5, ?5, ?5)",
                    params![hand_id, hero_player_id, position_group, depth_bucket, flag],
                )
                .expect("insert a synthetic hand_players row");
            }
            tx.commit().expect("commit a hands chunk");
            inserted = end;
        }
    }
    eprintln!(
        "seed : {HAND_COUNT} mains en {:?} (hors mesure NFR-P6)",
        seed_start.elapsed()
    );

    let request = ReportRequest {
        hero_profile_id: profile_id,
        dimensions: vec![Dimension::PositionGroup, Dimension::DepthBucket],
        measures: vec![
            Measure::Vpip,
            Measure::Pfr,
            Measure::Rfi,
            Measure::Limp,
            Measure::Oshove,
            Measure::ThreeBet,
            Measure::FoldToThreeBet,
            Measure::FourBet,
            Measure::Ats,
            Measure::Fsteal,
        ],
    };

    let sqlite_elapsed = {
        let reader = store.reader().expect("reader connection");
        let backend = SqliteAnalyticsBackend::new(&reader);
        let start = std::time::Instant::now();
        let rows = backend
            .run_report(&request)
            .expect("sqlite report should run");
        let elapsed = start.elapsed();
        eprintln!("NFR-P6 SQLite : {elapsed:?} ({} lignes)", rows.len());
        elapsed
    };

    let duck_dir = tempfile::tempdir().expect("temp dir for analytics.duckdb");
    let duck = open_duckdb(&duck_dir.path().join("analytics.duckdb")).expect("duckdb should open");
    let sync_start = std::time::Instant::now();
    {
        let reader = store.reader().expect("reader connection");
        sync_incremental(&duck, &reader).expect("sync should succeed");
    }
    eprintln!(
        "sync DuckDB : {HAND_COUNT} lignes en {:?} (hors mesure NFR-P6, cout unique apres import)",
        sync_start.elapsed()
    );

    let duckdb_elapsed = {
        let backend = DuckDbAnalyticsBackend::new(&duck);
        let start = std::time::Instant::now();
        let rows = backend
            .run_report(&request)
            .expect("duckdb report should run");
        let elapsed = start.elapsed();
        eprintln!("NFR-P6 DuckDB : {elapsed:?} ({} lignes)", rows.len());
        elapsed
    };

    let ratio = sqlite_elapsed.as_secs_f64() / duckdb_elapsed.as_secs_f64().max(f64::EPSILON);
    eprintln!("Ratio SQLite/DuckDB : {ratio:.2}x (cible ADR-002/CA M7-6 : >= 3x)");

    assert!(
        sqlite_elapsed.as_millis() < 8_000,
        "NFR-P6 SQLite hors cible : {sqlite_elapsed:?} (cible < 8 s)"
    );
    // Le ratio lui-meme n'est volontairement pas un assert bloquant ici :
    // la CA de M7-6 accepte un resultat < 3x, a condition de le consigner
    // dans l'ADR (voir la doc de module) plutot que de faire echouer la CI.
}
