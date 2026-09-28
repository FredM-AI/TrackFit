//! Lecture incrementale d'un fichier de mains par offset (M3-2, PRD §8.4
//! point 2) : utilisee par le watcher temps reel plutot que par l'import en
//! masse (`import_one_file`, M2-3), qui relit chaque fichier en entier.
//!
//! Robustesse (PRD §8.4) :
//! - fichier verrouille par Winamax en cours d'ecriture -> quelques
//!   tentatives rapprochees puis abandon pour ce passage (le prochain appel,
//!   declenche au plus tard par le polling de secours 2 s du watcher,
//!   agit comme le reste du backoff) ;
//! - fichier tronque ou reecrit (taille courante < offset connu) -> relecture
//!   complete depuis 0, le dedoublonnage `UNIQUE(room_id, room_hand_id)`
//!   evite les doublons.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use gr_store::{NewImportError, Store};

use crate::error::IngestError;
use crate::import::{file_size_and_mtime, insert_blocks, now_ms, FileImportReport, ImportFailure};

/// Delais entre deux tentatives de lecture d'un fichier verrouille, au sein
/// d'un seul appel. Volontairement court : le polling de secours du watcher
/// (2 s) fournit la suite de l'echelle de backoff du PRD (100 ms -> 2 s) sans
/// bloquer le thread du watcher sur un fichier recalcitrant.
const LOCK_RETRY_DELAYS: &[Duration] = &[Duration::from_millis(100), Duration::from_millis(300)];

/// Lit et importe les octets ajoutes depuis le dernier passage sur `path`
/// (ou depuis le debut si le fichier est inconnu ou a ete tronque). Avance
/// `import_files.last_offset` uniquement jusqu'au dernier bloc de main
/// complet (un bloc incomplet en cours d'ecriture est relu au prochain
/// appel).
///
/// # Errors
/// Renvoie une [`IngestError`] si l'ecriture en base echoue. Une erreur de
/// *lecture* du fichier (verrouille au-dela des tentatives) n'interrompt pas
/// l'appelant : elle est consignee et l'offset n'avance pas (PAR-15).
pub(crate) fn import_incremental_file(
    store: &Store,
    path: &Path,
) -> Result<FileImportReport, IngestError> {
    let mut report = FileImportReport::default();
    let now = now_ms();
    let path_str = path.to_string_lossy().into_owned();

    let known_offset = store
        .get_import_file_progress(&path_str)?
        .map_or(0, |p| p.last_offset);
    let (current_size, mtime) = file_size_and_mtime(path, now);

    if current_size == known_offset && known_offset > 0 {
        return Ok(report);
    }

    // Fichier tronque ou reecrit depuis le debut (jamais observe cote
    // Winamax en pratique, mais la relecture complete est sans risque grace
    // au dedoublonnage).
    let start_offset = if current_size < known_offset {
        0
    } else {
        known_offset
    };

    let text = match read_from_offset_with_retry(path, start_offset) {
        Ok(text) => text,
        Err(e) => {
            let file_id = store.upsert_import_file(&path_str, "HANDS", current_size, mtime, now)?;
            let message = e.to_string();
            report.failed += 1;
            report.failures.push(ImportFailure {
                file: path.to_path_buf(),
                message: message.clone(),
            });
            store.record_import_error(&NewImportError {
                file_id,
                file_offset: Some(start_offset),
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

    let file_id = store.upsert_import_file(&path_str, "HANDS", current_size, mtime, now)?;

    let (blocks, consumed) = gr_parser_winamax::split_hand_blocks(&text);
    let new_offset = start_offset + i64::try_from(consumed).unwrap_or(0);

    let block_report = insert_blocks(store, file_id, path, &blocks, now)?;
    report.inserted += block_report.inserted;
    report.duplicates += block_report.duplicates;
    report.failed += block_report.failed;
    report.failures.extend(block_report.failures);

    store.update_import_file_progress(file_id, current_size, new_offset, now)?;

    Ok(report)
}

fn read_from_offset_with_retry(path: &Path, offset: i64) -> std::io::Result<String> {
    let mut last_err = read_from_offset(path, offset);
    for delay in LOCK_RETRY_DELAYS {
        if last_err.is_ok() {
            break;
        }
        std::thread::sleep(*delay);
        last_err = read_from_offset(path, offset);
    }
    last_err
}

fn read_from_offset(path: &Path, offset: i64) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(u64::try_from(offset).unwrap_or(0)))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store should open");
        (dir, store)
    }

    fn hand(id: &str, pseudo_b: &str) -> String {
        format!(
            "Winamax Poker - Tournament \"T\" buyIn: 1\u{20ac} + 0\u{20ac} level: 1 - HandId: #{id}-1-1 - Holdem no limit (0/10/20) - 2026/01/01 00:00:00 UTC\nTable: 'T(1)#1' 2-max (real money) Seat #1 is the button\nSeat 1: Hero (1000)\nSeat 2: {pseudo_b} (1000)\n*** ANTE/BLINDS ***\nHero posts small blind 10\n{pseudo_b} posts big blind 20\nDealt to Hero [Ah Kd]\n*** PRE-FLOP ***\nHero folds\n{pseudo_b} collected 30 from pot\n*** SUMMARY ***\nTotal pot 30 | No rake\n\n\n"
        )
    }

    #[test]
    fn a_new_file_is_read_from_zero_and_offset_advances_to_the_last_complete_block() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        std::fs::write(&path, hand("1", "P0002")).unwrap();

        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(report.inserted, 1);
        assert_eq!(report.failed, 0);

        let progress = store
            .get_import_file_progress(&path.to_string_lossy())
            .unwrap()
            .unwrap();
        let expected_offset = i64::try_from(hand("1", "P0002").len()).unwrap();
        assert_eq!(progress.last_offset, expected_offset);
    }

    #[test]
    fn appending_a_second_hand_only_imports_the_new_bytes() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        std::fs::write(&path, hand("1", "P0002")).unwrap();
        import_incremental_file(&store, &path).unwrap();

        let mut content = hand("1", "P0002");
        content.push_str(&hand("2", "P0002"));
        std::fs::write(&path, &content).unwrap();

        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(
            report.inserted, 1,
            "only the newly appended hand should be parsed/inserted"
        );
        assert_eq!(report.duplicates, 0);
    }

    #[test]
    fn a_partial_trailing_block_is_not_consumed_until_it_is_complete() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        let full = hand("1", "P0002");
        // Winamax ecrit la main sans le separateur final (encore en cours
        // d'ecriture) : seul le contenu jusqu'au dernier `\n\n\n` doit etre
        // consomme, ici rien.
        let truncated = &full[..full.len() - 2];
        std::fs::write(&path, truncated).unwrap();

        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(report.inserted, 0, "the block is not yet complete");

        // Winamax finit d'ecrire le separateur.
        std::fs::write(&path, &full).unwrap();
        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(report.inserted, 1);
    }

    #[test]
    fn no_change_since_last_pass_does_nothing() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        std::fs::write(&path, hand("1", "P0002")).unwrap();
        import_incremental_file(&store, &path).unwrap();

        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(report.inserted, 0);
        assert_eq!(report.duplicates, 0);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn a_truncated_or_rewritten_file_is_reread_from_the_start_without_duplicating() {
        let (dir, store) = open_store();
        let path = dir.path().join("live.txt");
        let mut content = hand("1", "P0002");
        content.push_str(&hand("2", "P0002"));
        std::fs::write(&path, &content).unwrap();
        import_incremental_file(&store, &path).unwrap();

        // Le fichier est reecrit plus court que l'offset connu (cas
        // defensif : jamais observe cote Winamax, mais couvert par le PRD).
        let shorter = hand("1", "P0002");
        std::fs::write(&path, &shorter).unwrap();

        let report = import_incremental_file(&store, &path).unwrap();
        assert_eq!(report.inserted, 0, "hand 1 was already stored");
        assert_eq!(report.duplicates, 1);
    }
}
