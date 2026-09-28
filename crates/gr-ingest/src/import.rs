//! Import en masse (M2-3) : decouverte des fichiers, parsing, ecriture par
//! lots dans `gr-store`, progression, annulation cooperative, rapport final.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gr_core::HandRecord;
use gr_parser_api::Room;
use gr_parser_winamax::{split_hand_blocks, WinamaxParser};
use gr_store::{HandInsert, Store};

use crate::error::IngestError;
use crate::scan::discover_hand_files;

/// Jeton d'annulation cooperatif : l'import en cours termine le fichier
/// entame puis s'arrete au prochain fichier des que le jeton est signale.
/// Clonable et partageable (ex. entre une commande Tauri `import_paths` et
/// une commande `cancel_import`).
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Signale l'annulation ; sans effet si l'import est deja termine.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Remet le jeton a l'etat non annule (reutilisation entre deux imports).
    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Etat de progression emis apres chaque fichier traite (PRD : evenement IPC
/// `import://progress`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProgress {
    pub files_total: usize,
    pub files_done: usize,
    pub hands_inserted: usize,
    pub hands_duplicate: usize,
    pub hands_failed: usize,
}

/// Une main (ou un fichier) qui n'a pas pu etre importee, destinee a
/// l'onglet Erreurs d'import (M2-5). PAR-15 : une defaillance isolee ne
/// doit jamais interrompre le reste de l'import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailure {
    pub file: PathBuf,
    pub message: String,
}

/// Rapport final d'un import (PRD §8.4 point 4 ; BACKLOG M2-3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub files_scanned: usize,
    pub files_imported: usize,
    pub hands_inserted: usize,
    pub hands_duplicate: usize,
    pub hands_failed: usize,
    pub failures: Vec<ImportFailure>,
    pub cancelled: bool,
}

#[derive(Debug, Default)]
struct FileImportReport {
    inserted: usize,
    duplicates: usize,
    failed: usize,
    failures: Vec<ImportFailure>,
}

/// Importe toutes les mains trouvees sous `roots` (fichiers ou dossiers,
/// PRD "Selection de dossiers ou de fichiers") dans `store`, par lots
/// transactionnels (voir `gr_store::BATCH_SIZE`). Rappelle `on_progress`
/// apres chaque fichier traite ; s'arrete proprement des que `cancel` est
/// signale, sans interrompre le fichier en cours.
///
/// # Errors
/// Renvoie une [`IngestError`] si l'ecriture en base echoue. Une erreur de
/// *lecture* ou de *parsing* d'un fichier ou d'une main isolee n'interrompt
/// pas l'import : elle est consignee dans `ImportSummary::failures` (PAR-15).
pub fn run_import(
    store: &Store,
    roots: &[PathBuf],
    cancel: &CancelToken,
    mut on_progress: impl FnMut(&ImportProgress),
) -> Result<ImportSummary, IngestError> {
    let files = discover_hand_files(roots);
    let mut summary = ImportSummary {
        files_scanned: files.len(),
        ..ImportSummary::default()
    };
    let mut progress = ImportProgress {
        files_total: files.len(),
        files_done: 0,
        hands_inserted: 0,
        hands_duplicate: 0,
        hands_failed: 0,
    };

    for file in files {
        if cancel.is_cancelled() {
            summary.cancelled = true;
            break;
        }

        let file_report = import_one_file(store, &file)?;
        summary.files_imported += 1;
        summary.hands_inserted += file_report.inserted;
        summary.hands_duplicate += file_report.duplicates;
        summary.hands_failed += file_report.failed;
        summary.failures.extend(file_report.failures);

        progress.files_done += 1;
        progress.hands_inserted = summary.hands_inserted;
        progress.hands_duplicate = summary.hands_duplicate;
        progress.hands_failed = summary.hands_failed;
        on_progress(&progress);
    }

    Ok(summary)
}

fn import_one_file(store: &Store, path: &Path) -> Result<FileImportReport, IngestError> {
    let mut report = FileImportReport::default();

    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            report.failed += 1;
            report.failures.push(ImportFailure {
                file: path.to_path_buf(),
                message: e.to_string(),
            });
            return Ok(report);
        }
    };
    let (blocks, _offset) = split_hand_blocks(&text);

    let mut hands: Vec<HandRecord> = Vec::with_capacity(blocks.len());
    let mut raw_texts: Vec<&str> = Vec::with_capacity(blocks.len());
    for block in blocks {
        match WinamaxParser::parse_hand(block) {
            Ok(hand) => {
                hands.push(hand);
                raw_texts.push(block);
            }
            Err(e) => {
                report.failed += 1;
                report.failures.push(ImportFailure {
                    file: path.to_path_buf(),
                    message: e.to_string(),
                });
            }
        }
    }

    let inserts: Vec<HandInsert<'_>> = hands
        .iter()
        .zip(raw_texts.iter().copied())
        .map(|(hand, raw_text)| HandInsert { hand, raw_text })
        .collect();

    let insert_report = store.insert_hands(Room::Winamax, &inserts)?;
    report.inserted += insert_report.inserted;
    report.duplicates += insert_report.duplicates;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn corpus_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax")
    }

    fn open_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store should open");
        (dir, store)
    }

    #[test]
    fn imports_the_whole_committed_corpus_without_any_failure() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];
        let expected_files = discover_hand_files(&roots).len();
        assert!(expected_files > 0, "the fixtures corpus must not be empty");

        let mut progress_calls = 0;
        let summary = run_import(&store, &roots, &CancelToken::new(), |_| progress_calls += 1)
            .expect("import should not hit a storage error");

        assert_eq!(summary.files_scanned, expected_files);
        assert_eq!(summary.files_imported, expected_files);
        assert_eq!(summary.hands_failed, 0);
        assert!(summary.failures.is_empty());
        assert!(summary.hands_inserted > 0);
        assert_eq!(summary.hands_duplicate, 0);
        assert!(!summary.cancelled);
        assert_eq!(progress_calls, expected_files);
    }

    #[test]
    fn reimporting_the_same_corpus_yields_zero_insertions_and_n_duplicates() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];

        let first = run_import(&store, &roots, &CancelToken::new(), |_| {})
            .expect("first import should succeed");
        let second = run_import(&store, &roots, &CancelToken::new(), |_| {})
            .expect("second import should succeed");

        assert_eq!(second.hands_inserted, 0);
        assert_eq!(second.hands_duplicate, first.hands_inserted);
        assert_eq!(second.hands_failed, 0);
    }

    #[test]
    fn stops_before_the_next_file_once_cancelled() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];
        let expected_files = discover_hand_files(&roots).len();
        assert!(
            expected_files > 1,
            "need at least 2 files to observe an early stop"
        );

        let cancel = CancelToken::new();
        let cancel_after_first_file = cancel.clone();
        let mut calls = 0;
        let summary = run_import(&store, &roots, &cancel, |_| {
            calls += 1;
            if calls == 1 {
                cancel_after_first_file.cancel();
            }
        })
        .expect("import should not hit a storage error");

        assert!(summary.cancelled);
        assert_eq!(summary.files_imported, 1);
        assert!(summary.files_imported < summary.files_scanned);
    }

    #[test]
    fn an_already_cancelled_token_imports_nothing() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];
        let cancel = CancelToken::new();
        cancel.cancel();

        let summary = run_import(&store, &roots, &cancel, |_| {}).expect("import should not error");

        assert!(summary.cancelled);
        assert_eq!(summary.files_imported, 0);
        assert_eq!(summary.hands_inserted, 0);
    }

    /// Valide le CA de perf differe de M2-3 (≥1000 mains/s sur 100k mains
    /// synthetiques), desormais mesurable grace a `gr-synth` (M2-4).
    /// Ignore par defaut (ecrit ~100k mains sur disque, quelques secondes) :
    /// `cargo test -p gr-ingest -- --ignored perf_100k`.
    #[test]
    #[ignore = "ecrit ~100k mains sur disque ; lancer explicitement avec --ignored"]
    fn perf_100k_synthetic_hands_imports_at_least_1000_hands_per_second() {
        let synth_dir = tempfile::tempdir().expect("temp dir for synthetic hands");
        let mut written = 0usize;
        for tournament in gr_synth::SynthCorpus::new(100_000, 0x00C0_FFEE) {
            let path = synth_dir
                .path()
                .join(format!("{}.txt", tournament.file_stem));
            std::fs::write(&path, &tournament.content).expect("write synthetic tournament file");
            written += tournament.hand_count;
        }
        assert_eq!(written, 100_000);

        let (_db_dir, store) = open_store();
        let roots = vec![synth_dir.path().to_path_buf()];

        let start = std::time::Instant::now();
        let summary = run_import(&store, &roots, &CancelToken::new(), |_| {})
            .expect("import of the synthetic corpus should not hit a storage error");
        let elapsed = start.elapsed();

        assert_eq!(summary.hands_failed, 0);
        assert_eq!(summary.hands_inserted, 100_000);

        let hands_per_sec = 100_000.0 / elapsed.as_secs_f64();
        println!("M2-3 perf (differee) : {hands_per_sec:.0} mains/s ({elapsed:?} pour 100k mains)");
        assert!(
            hands_per_sec >= 1000.0,
            "cible PRD/BACKLOG M2-3 : >= 1000 mains/s, mesure {hands_per_sec:.0}"
        );
    }
}
