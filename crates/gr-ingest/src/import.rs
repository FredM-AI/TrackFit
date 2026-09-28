//! Import en masse (M2-3) : decouverte des fichiers, parsing, ecriture par
//! lots dans `gr-store`, progression, annulation cooperative, rapport final.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gr_core::HandRecord;
use gr_parser_api::Room;
use gr_parser_winamax::{split_hand_blocks, WinamaxParser};
use gr_store::{HandInsert, NewImportError, Store};

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
    let now = now_ms();
    let path_str = path.to_string_lossy().into_owned();

    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            report.failed += 1;
            let message = e.to_string();
            report.failures.push(ImportFailure {
                file: path.to_path_buf(),
                message: message.clone(),
            });
            let file_id = store.upsert_import_file(&path_str, "HANDS", 0, now, now)?;
            store.record_import_error(&NewImportError {
                file_id,
                file_offset: None,
                line_no: None,
                code: "IO_ERROR",
                message: &message,
                raw_excerpt: None,
                parser_version: env!("CARGO_PKG_VERSION"),
                created_at: now,
            })?;
            return Ok(report);
        }
    };

    let (size, mtime) = file_size_and_mtime(path, now);
    let file_id = store.upsert_import_file(&path_str, "HANDS", size, mtime, now)?;

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
                // Le texte brut de la main fautive (`raw_excerpt`) est ce qui
                // permet a l'action "Reparser" de retenter plus tard, sans
                // avoir a relire le fichier (M2-5).
                store.record_import_error(&NewImportError {
                    file_id,
                    file_offset: None,
                    line_no: i64::try_from(e.line_no).ok(),
                    code: &e.code.to_string(),
                    message: &e.context,
                    raw_excerpt: Some(block),
                    parser_version: env!("CARGO_PKG_VERSION"),
                    created_at: now,
                })?;
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

fn file_size_and_mtime(path: &Path, fallback: i64) -> (i64, i64) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return (0, fallback);
    };
    let size = i64::try_from(metadata.len()).unwrap_or(0);
    let mtime = metadata.modified().ok().map_or(fallback, system_time_to_ms);
    (size, mtime)
}

fn now_ms() -> i64 {
    system_time_to_ms(std::time::SystemTime::now())
}

fn system_time_to_ms(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

/// Resultat d'une tentative de reparse (M2-5, action "Reparser" de l'onglet
/// Erreurs d'import).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparseOutcome {
    /// La main se parse desormais correctement : inseree, erreur resolue.
    Resolved,
    /// Toujours en echec ; la ligne d'erreur est rafraichie avec la raison.
    StillFailing { message: String },
}

/// Reprend le texte brut conserve (`raw_excerpt`) d'une erreur d'import et
/// tente de le reparser (M2-5), typiquement apres correction d'un bug du
/// parser. Insere la main et marque l'erreur `RESOLVED` en cas de succes ;
/// sinon rafraichit son message et la laisse `OPEN`.
///
/// # Errors
/// Renvoie une [`IngestError`] si `error_id` n'existe pas, n'a pas de texte
/// brut conserve, ou si l'ecriture en base echoue.
pub fn reparse_import_error(store: &Store, error_id: i64) -> Result<ReparseOutcome, IngestError> {
    let row = store
        .get_import_error(error_id)?
        .ok_or(IngestError::ImportErrorNotFound(error_id))?;
    let raw_excerpt = row
        .raw_excerpt
        .as_deref()
        .ok_or(IngestError::NoRawExcerpt(error_id))?;

    match WinamaxParser::parse_hand(raw_excerpt) {
        Ok(hand) => {
            let insert = HandInsert {
                hand: &hand,
                raw_text: raw_excerpt,
            };
            store.insert_hands(Room::Winamax, &[insert])?;
            store.set_import_error_status(error_id, "RESOLVED")?;
            Ok(ReparseOutcome::Resolved)
        }
        Err(e) => {
            store.update_import_error_failure(
                error_id,
                &e.code.to_string(),
                &e.context,
                env!("CARGO_PKG_VERSION"),
            )?;
            Ok(ReparseOutcome::StillFailing { message: e.context })
        }
    }
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

    /// Main minimale invalide : `Hero does something invalid` ne correspond
    /// a aucun motif d'action connu (PAR-7) -> `UNKNOWN_LINE` garanti.
    const CORRUPTED_HAND: &str = "Winamax Poker - Tournament \"TEST CORRUPTED\" buyIn: 1\u{20ac} + 0\u{20ac} level: 1 - HandId: #1-1-1 - Holdem no limit (0/10/20) - 2026/01/01 00:00:00 UTC\nTable: 'TEST CORRUPTED(1)#1' 2-max (real money) Seat #1 is the button\nSeat 1: Hero (1000)\nSeat 2: P0002 (1000)\n*** ANTE/BLINDS ***\nHero posts small blind 10\nP0002 posts big blind 20\nDealt to Hero [Ah Kd]\n*** PRE-FLOP ***\nHero does something invalid\nP0002 collected 30 from pot\n*** SUMMARY ***\nTotal pot 30 | No rake\n";

    fn write_single_hand_file(dir: &Path, file_name: &str, hand_text: &str) -> PathBuf {
        let path = dir.join(file_name);
        let content = format!("{}\n\n\n", hand_text.trim_end_matches('\n'));
        std::fs::write(&path, content).expect("write single-hand fixture");
        path
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

    /// CA de M2-5 : une main corrompue injectee apparait dans l'onglet
    /// Erreurs d'import (ici valide au niveau donnees : `import_errors`).
    #[test]
    fn a_corrupted_hand_is_recorded_in_import_errors() {
        let (_db_dir, store) = open_store();
        let files_dir = tempfile::tempdir().expect("temp dir for the fixture file");
        write_single_hand_file(files_dir.path(), "corrupted.txt", CORRUPTED_HAND);

        let summary = run_import(
            &store,
            &[files_dir.path().to_path_buf()],
            &CancelToken::new(),
            |_| {},
        )
        .expect("import should not hit a storage error even with a corrupted hand");

        assert_eq!(summary.hands_failed, 1);
        assert_eq!(summary.hands_inserted, 0);

        let errors = store.list_import_errors(None).expect("list import errors");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "UNKNOWN_LINE");
        assert_eq!(errors[0].status, "OPEN");
        assert!(errors[0].file_path.ends_with("corrupted.txt"));
        assert!(errors[0]
            .raw_excerpt
            .as_deref()
            .expect("raw_excerpt should be kept for a later reparse")
            .contains("does something invalid"));
    }

    /// CA de M2-5 : "Reparser" fonctionne une fois la cause corrigee. On ne
    /// peut pas reellement corriger le parser dans un test ; on simule une
    /// correction du texte source (meme mecanisme cote reparse : relire
    /// `raw_excerpt` et retenter), ce qui exerce le meme chemin de code.
    #[test]
    fn reparse_succeeds_and_resolves_the_error_once_the_raw_text_is_valid() {
        let (_db_dir, store) = open_store();
        let files_dir = tempfile::tempdir().expect("temp dir for the fixture file");
        write_single_hand_file(files_dir.path(), "corrupted.txt", CORRUPTED_HAND);

        run_import(
            &store,
            &[files_dir.path().to_path_buf()],
            &CancelToken::new(),
            |_| {},
        )
        .expect("import should not hit a storage error");
        let errors = store.list_import_errors(Some("OPEN")).expect("list errors");
        assert_eq!(errors.len(), 1);
        let error_id = errors[0].id;

        let corrected_hand =
            CORRUPTED_HAND.replace("Hero does something invalid\n", "Hero folds\n");
        store
            .writer()
            .execute(
                &format!("UPDATE import_errors SET raw_excerpt = ?1 WHERE id = {error_id}"),
                [corrected_hand.as_str()],
            )
            .expect("simulate the underlying issue being fixed");

        let outcome = reparse_import_error(&store, error_id).expect("reparse should not error");
        assert_eq!(outcome, ReparseOutcome::Resolved);

        let refreshed = store
            .get_import_error(error_id)
            .expect("get import error")
            .expect("the error row should still exist");
        assert_eq!(refreshed.status, "RESOLVED");

        let hand_count: i64 = store
            .writer()
            .query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
            .expect("hands should be readable");
        assert_eq!(hand_count, 1, "the reparsed hand should have been inserted");
    }

    #[test]
    fn reparse_updates_the_message_and_keeps_the_error_open_when_still_failing() {
        let (_db_dir, store) = open_store();
        let files_dir = tempfile::tempdir().expect("temp dir for the fixture file");
        write_single_hand_file(files_dir.path(), "corrupted.txt", CORRUPTED_HAND);

        run_import(
            &store,
            &[files_dir.path().to_path_buf()],
            &CancelToken::new(),
            |_| {},
        )
        .expect("import should not hit a storage error");
        let error_id = store.list_import_errors(Some("OPEN")).unwrap()[0].id;

        let outcome = reparse_import_error(&store, error_id).expect("reparse should not error");
        assert!(matches!(outcome, ReparseOutcome::StillFailing { .. }));

        let refreshed = store
            .get_import_error(error_id)
            .unwrap()
            .expect("the error row should still exist");
        assert_eq!(refreshed.status, "OPEN");
    }

    #[test]
    fn reparse_of_an_unknown_error_id_fails() {
        let (_db_dir, store) = open_store();
        let err = reparse_import_error(&store, 999_999).expect_err("id should not exist");
        assert!(matches!(err, IngestError::ImportErrorNotFound(999_999)));
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
