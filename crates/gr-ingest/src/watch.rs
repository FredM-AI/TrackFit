//! Watcher temps reel (M3-2, PRD §8.4) : notification `notify` (native,
//! `ReadDirectoryChangesW` sous Windows) pour un reveil quasi immediat,
//! plus un polling de secours toutes les 2 s (fichiers manques par
//! `notify`, ex. montages reseau, ou evenements rates). Les deux mecanismes
//! declenchent le meme passage : une redecouverte des fichiers de mains
//! sous `roots`, suivie d'une lecture incrementale par fichier
//! (`import_incremental_file`, M3-2).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use gr_store::Store;

use crate::import::now_ms;
use crate::incremental::import_incremental_file;
use crate::scan::discover_hand_files;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Resultat d'un passage du watcher (un reveil notify ou un tick de
/// polling), rapporte a l'appelant pour emettre `hands://new` cote UI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchTick {
    pub hands_inserted: usize,
    pub hands_duplicate: usize,
    pub hands_failed: usize,
}

/// Poignee du watcher : l'arreter (`stop`) ou la laisser sortir de portee
/// (le `Drop` arrete proprement le thread) coupe la surveillance.
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    // Conserve uniquement pour garder le watcher natif vivant tant que ce
    // handle existe (son callback envoie sur `tx`, capture par la closure).
    _fs_watcher: Option<RecommendedWatcher>,
}

impl WatcherHandle {
    /// Arrete le watcher et attend la fin du thread de fond. Sans effet si
    /// deja arrete.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Demarre le watcher temps reel sur `roots` (dossiers d'historiques
/// Winamax). Appelle `on_tick` apres chaque passage, y compris quand il n'y
/// a rien de nouveau (compteurs a 0) : l'appelant (`src-tauri`) decide s'il
/// emet `hands://new` uniquement quand `hands_inserted > 0`.
#[must_use]
pub fn spawn_watcher(
    store: Arc<Store>,
    roots: Vec<PathBuf>,
    mut on_tick: impl FnMut(&WatchTick) + Send + 'static,
) -> WatcherHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<()>();

    // Le watcher natif n'est qu'un signal de reveil : une erreur (dossier
    // absent, permissions) ne doit jamais empecher le watcher de tourner,
    // le polling de secours reste actif dans tous les cas (PRD §8.4).
    let fs_watcher = build_fs_watcher(&roots, tx);

    let thread = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let tick = scan_once(&store, &roots);
                on_tick(&tick);

                // Reveil sur le premier evenement notify recu, ou au plus
                // tard apres 2 s (polling de secours). Un burst d'evenements
                // notify (plusieurs fichiers ecrits d'affilee) est absorbe
                // par le prochain passage complet plutot que de relancer un
                // scan par evenement.
                match rx.recv_timeout(POLL_INTERVAL) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                while rx.try_recv().is_ok() {}
            }
        })
    };

    WatcherHandle {
        stop,
        thread: Some(thread),
        _fs_watcher: fs_watcher,
    }
}

fn build_fs_watcher(roots: &[PathBuf], tx: mpsc::Sender<()>) -> Option<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |_event: notify::Result<notify::Event>| {
        let _ = tx.send(());
    })
    .ok()?;
    for root in roots {
        // Un dossier absent au demarrage (ex. compte pas encore joue) ne
        // doit pas empecher les autres d'etre surveilles.
        let _ = watcher.watch(root, RecursiveMode::Recursive);
    }
    Some(watcher)
}

fn scan_once(store: &Store, roots: &[PathBuf]) -> WatchTick {
    let mut tick = WatchTick::default();

    let discovered = discover_hand_files(roots);
    mark_missing_files(store, &discovered);

    for file in discovered {
        if let Ok(report) = import_incremental_file(store, &file) {
            tick.hands_inserted += report.inserted;
            tick.hands_duplicate += report.duplicates;
            tick.hands_failed += report.failed;
        }
        // Une erreur de store (SQLite) est transitoire dans ce contexte
        // (fichier verrouille cote lecture est deja gere par
        // `import_incremental_file` sans faire remonter d'Err) ; le passage
        // suivant reessaiera naturellement.
    }

    tick
}

/// Marque `MISSING` les fichiers suivis qui n'apparaissent plus dans le
/// scan disque courant (renommage ou suppression, PRD §8.4 "Robustesse").
/// Les mains deja importees restent en base.
fn mark_missing_files(store: &Store, discovered: &[PathBuf]) {
    let Ok(tracked) = store.list_tracked_hand_file_paths() else {
        return;
    };
    let discovered_set: HashSet<String> = discovered
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let now = now_ms();
    for path in tracked {
        if !discovered_set.contains(&path) {
            let _ = store.mark_import_file_missing(&path, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;

    fn open_store() -> (tempfile::TempDir, Arc<Store>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Arc::new(Store::open(dir.path()).expect("store should open"));
        (dir, store)
    }

    fn hand(id: &str) -> String {
        format!(
            "Winamax Poker - Tournament \"T\" buyIn: 1\u{20ac} + 0\u{20ac} level: 1 - HandId: #{id}-1-1 - Holdem no limit (0/10/20) - 2026/01/01 00:00:00 UTC\nTable: 'T(1)#1' 2-max (real money) Seat #1 is the button\nSeat 1: Hero (1000)\nSeat 2: P0002 (1000)\n*** ANTE/BLINDS ***\nHero posts small blind 10\nP0002 posts big blind 20\nDealt to Hero [Ah Kd]\n*** PRE-FLOP ***\nHero folds\nP0002 collected 30 from pot\n*** SUMMARY ***\nTotal pot 30 | No rake\n\n\n"
        )
    }

    /// Attend jusqu'a `timeout` qu'une condition devienne vraie, en
    /// sondant regulierement (evite un `sleep` fixe fragile dans un test
    /// qui depend d'un thread de fond).
    fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        condition()
    }

    #[test]
    fn a_hand_written_before_the_watcher_starts_is_picked_up_on_the_first_pass() {
        let (dir, store) = open_store();
        std::fs::write(dir.path().join("live.txt"), hand("1")).unwrap();

        let inserted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inserted_clone = Arc::clone(&inserted);
        let mut handle = spawn_watcher(
            Arc::clone(&store),
            vec![dir.path().to_path_buf()],
            move |tick| {
                inserted_clone.fetch_add(tick.hands_inserted, Ordering::Relaxed);
            },
        );

        let seen = wait_until(Duration::from_secs(5), || {
            inserted.load(Ordering::Relaxed) >= 1
        });
        handle.stop();

        assert!(seen, "the pre-existing hand should be imported");
    }

    #[test]
    fn a_hand_appended_while_the_watcher_is_running_is_detected() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        std::fs::write(&path, "").unwrap();

        let inserted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inserted_clone = Arc::clone(&inserted);
        let mut handle = spawn_watcher(
            Arc::clone(&store),
            vec![dir.path().to_path_buf()],
            move |tick| {
                inserted_clone.fetch_add(tick.hands_inserted, Ordering::Relaxed);
            },
        );

        // Laisse le premier passage (fichier vide) se produire, puis simule
        // Winamax qui ecrit une main.
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(&path, hand("1")).unwrap();

        let seen = wait_until(Duration::from_secs(5), || {
            inserted.load(Ordering::Relaxed) >= 1
        });
        handle.stop();

        assert!(
            seen,
            "the appended hand should be detected by notify or the 2s poll fallback"
        );
    }

    #[test]
    fn stopping_the_watcher_joins_the_background_thread() {
        let (dir, store) = open_store();
        let mut handle = spawn_watcher(Arc::clone(&store), vec![dir.path().to_path_buf()], |_| {});
        handle.stop();
        assert!(
            handle.thread.is_none(),
            "stop() must join and clear the thread"
        );
    }

    #[test]
    fn a_renamed_file_is_marked_missing_but_its_hands_stay_in_the_database() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        let old_path_str = path.to_string_lossy().into_owned();
        std::fs::write(&path, hand("1")).unwrap();

        let mut handle = spawn_watcher(Arc::clone(&store), vec![dir.path().to_path_buf()], |_| {});
        let seen = wait_until(Duration::from_secs(5), || {
            store
                .list_tracked_hand_file_paths()
                .is_ok_and(|paths| paths.contains(&old_path_str))
        });
        assert!(seen, "the file should be tracked after the first pass");

        std::fs::rename(&path, dir.path().join("renamed.txt")).unwrap();

        let missing = wait_until(Duration::from_secs(5), || {
            store
                .list_tracked_hand_file_paths()
                .is_ok_and(|paths| !paths.contains(&old_path_str))
        });
        handle.stop();

        assert!(
            missing,
            "the old path should no longer be tracked as present"
        );
    }

    fn hand_for(file_idx: usize, hand_idx: usize) -> String {
        // Identifiant tout-numerique unique par (fichier, main), 2 + 6
        // chiffres, pour ne jamais collisionner avec le dedoublonnage
        // `UNIQUE(room_id, room_hand_id)` entre les 12 "tables" simulees.
        hand(&format!("{file_idx:02}{hand_idx:06}"))
    }

    /// Resultat d'un scenario a N fichiers ecrits en parallele (PRD §8.4/
    /// BACKLOG M3-2 CA) : une latence ecriture -> visible en base par main.
    struct ConcurrentWriteReport {
        latencies: Vec<Duration>,
        duplicates: usize,
        failed: usize,
        expected_hands: usize,
    }

    /// Simule `num_files` "tables" ecrivant chacune une main toutes les
    /// `hand_interval`, pendant `total_duration`, et mesure pour chaque main
    /// le delai entre son ecriture sur disque et le moment ou le watcher l'a
    /// rendue visible en base (`import_files.last_offset` a depasse l'octet
    /// de fin de cette main pour ce fichier). Sert au test de fumee rapide
    /// et au test CA complet ci-dessous (seuls les parametres different).
    fn run_concurrent_write_scenario(
        store: &Arc<Store>,
        dir: &std::path::Path,
        num_files: usize,
        hand_interval: Duration,
        total_duration: Duration,
    ) -> ConcurrentWriteReport {
        let duplicates = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let failed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut handle = {
            let duplicates = Arc::clone(&duplicates);
            let failed = Arc::clone(&failed);
            spawn_watcher(Arc::clone(store), vec![dir.to_path_buf()], move |tick| {
                duplicates.fetch_add(tick.hands_duplicate, Ordering::Relaxed);
                failed.fetch_add(tick.hands_failed, Ordering::Relaxed);
            })
        };

        let latencies: Arc<Mutex<Vec<Duration>>> = Arc::new(Mutex::new(Vec::new()));
        let writers: Vec<_> = (0..num_files)
            .map(|file_idx| {
                let store = Arc::clone(store);
                let path = dir.join(format!("table_{file_idx:02}.txt"));
                let latencies = Arc::clone(&latencies);
                std::thread::spawn(move || {
                    use std::io::Write;

                    std::fs::write(&path, "").unwrap();
                    let path_str = path.to_string_lossy().into_owned();
                    let scenario_start = std::time::Instant::now();
                    let mut expected_offset: i64 = 0;
                    let mut hand_idx = 0usize;
                    while scenario_start.elapsed() < total_duration {
                        let text = hand_for(file_idx, hand_idx);
                        // Vrai append (comme Winamax, qui n'a aucune raison
                        // de tronquer/reecrire un historique de mains) :
                        // un `fs::write` du fichier entier serait racy vis a
                        // vis du watcher, qui verrait alors une taille <
                        // `last_offset` en cours d'ecriture et declencherait
                        // a tort le chemin "fichier tronque".
                        std::fs::OpenOptions::new()
                            .append(true)
                            .open(&path)
                            .unwrap()
                            .write_all(text.as_bytes())
                            .unwrap();
                        expected_offset += i64::try_from(text.len()).unwrap();
                        let write_instant = std::time::Instant::now();

                        let visible = wait_until(Duration::from_secs(30), || {
                            store
                                .get_import_file_progress(&path_str)
                                .ok()
                                .flatten()
                                .is_some_and(|p| p.last_offset >= expected_offset)
                        });
                        if visible {
                            latencies.lock().unwrap().push(write_instant.elapsed());
                        }

                        hand_idx += 1;
                        std::thread::sleep(hand_interval);
                    }
                    hand_idx
                })
            })
            .collect();

        let expected_hands: usize = writers.into_iter().map(|w| w.join().unwrap()).sum();
        handle.stop();

        let latencies = Arc::try_unwrap(latencies).unwrap().into_inner().unwrap();
        ConcurrentWriteReport {
            latencies,
            duplicates: duplicates.load(Ordering::Relaxed),
            failed: failed.load(Ordering::Relaxed),
            expected_hands,
        }
    }

    fn assert_ca(report: &ConcurrentWriteReport, label: &str) {
        assert_eq!(
            report.latencies.len(),
            report.expected_hands,
            "{label}: chaque main ecrite doit devenir visible (0 perte)"
        );
        assert_eq!(report.duplicates, 0, "{label}: 0 doublon attendu");
        assert_eq!(report.failed, 0, "{label}: 0 echec de parsing attendu");

        let mut sorted = report.latencies.clone();
        sorted.sort();
        let p95_idx = (sorted.len() * 95 / 100).min(sorted.len() - 1);
        let p95 = sorted[p95_idx];
        println!(
            "{label}: {} mains, p95 = {p95:?} (max {:?})",
            sorted.len(),
            sorted.last().unwrap()
        );
        assert!(
            p95 < Duration::from_secs(2),
            "{label}: p95 doit etre < 2s, mesure {p95:?}"
        );
    }

    #[test]
    fn watcher_keeps_up_with_a_few_files_written_rapidly() {
        let (_db_dir, store) = open_store();
        let hands_dir = tempfile::tempdir().expect("temp dir for hand files");
        let report = run_concurrent_write_scenario(
            &store,
            hands_dir.path(),
            3,
            Duration::from_millis(200),
            Duration::from_secs(3),
        );
        assert_ca(&report, "smoke (3 fichiers, 200ms, 3s)");
    }

    #[test]
    #[ignore = "CA complet BACKLOG M3-2 : 12 fichiers, 1 main/3s, 10 minutes reelles ; lancer explicitement avec --ignored --release"]
    fn watcher_meets_the_m3_2_latency_ca_over_12_files_for_10_minutes() {
        let (_db_dir, store) = open_store();
        let hands_dir = tempfile::tempdir().expect("temp dir for hand files");
        let report = run_concurrent_write_scenario(
            &store,
            hands_dir.path(),
            12,
            Duration::from_secs(3),
            Duration::from_secs(600),
        );
        assert_ca(&report, "CA M3-2 (12 fichiers, 3s, 10 min)");
    }
}
