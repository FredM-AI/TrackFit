//! Import des fichiers summary et rattachement aux tournois (M2-6, PRD §8.5).

use std::path::{Path, PathBuf};

use gr_parser_api::Room;
use gr_parser_winamax::WinamaxParser;
use gr_store::Store;

use crate::import::ImportFailure;
use crate::scan::discover_summary_files;

/// Rapport d'import des summaries (M2-6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SummaryImportSummary {
    pub files_scanned: usize,
    pub files_attached: usize,
    pub files_failed: usize,
    pub failures: Vec<ImportFailure>,
}

/// Parse et rattache tous les fichiers summary trouves sous `roots` (M2-6) :
/// buy-in exact, statut `PROVISIONAL` -> `COMPLETE`, `tournament_entries`/
/// `tournament_bullets`. Le rattachement fonctionne quel que soit l'ordre
/// d'import par rapport aux mains (le tournoi est cree "provisoire" au besoin).
/// Un summary illisible ou imparsable n'interrompt pas le reste (PAR-15) : il
/// est consigne dans `failures`, sans etre persiste dans `import_errors`
/// (portee de M2-5, limitee aux mains).
#[must_use]
pub fn import_summaries(store: &Store, roots: &[PathBuf]) -> SummaryImportSummary {
    let files = discover_summary_files(roots);
    let mut report = SummaryImportSummary {
        files_scanned: files.len(),
        ..SummaryImportSummary::default()
    };

    for path in files {
        match import_one_summary(store, &path) {
            Ok(()) => report.files_attached += 1,
            Err(message) => {
                report.files_failed += 1;
                report.failures.push(ImportFailure {
                    file: path,
                    message,
                });
            }
        }
    }
    report
}

fn import_one_summary(store: &Store, path: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let summary = WinamaxParser::parse_summary(&text).map_err(|e| e.to_string())?;
    store
        .attach_summary(Room::Winamax, &summary)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path as StdPath;

    use gr_store::Store;

    use super::*;

    fn corpus_root() -> PathBuf {
        StdPath::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax")
    }

    fn open_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store should open");
        (dir, store)
    }

    #[test]
    fn attaches_every_summary_of_the_committed_corpus_without_any_failure() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];
        let expected_files = discover_summary_files(&roots).len();
        assert!(expected_files > 0, "the fixtures corpus must not be empty");

        let report = import_summaries(&store, &roots);

        assert_eq!(report.files_scanned, expected_files);
        assert_eq!(report.files_attached, expected_files);
        assert_eq!(report.files_failed, 0);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn reattaching_the_same_corpus_summaries_is_a_no_op() {
        let (_dir, store) = open_store();
        let roots = vec![corpus_root()];

        let _first = import_summaries(&store, &roots);
        let second = import_summaries(&store, &roots);

        assert_eq!(second.files_failed, 0);
        assert!(second.failures.is_empty());
    }
}
