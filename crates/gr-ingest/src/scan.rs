use std::path::{Path, PathBuf};

/// Explore chaque racine (fichier ou dossier, recursivement) et retourne,
/// triee et sans doublon, la liste des fichiers de mains `.txt` trouves.
/// Les fichiers `_summary.txt` sont exclus (rattaches au tournoi via
/// [`crate::import_summaries`], M2-6, docs/formats/winamax.md §2).
#[must_use]
pub fn discover_hand_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    discover(roots, is_hand_file)
}

/// Meme exploration, mais ne retient que les fichiers summary `_summary.txt`
/// (M2-6).
#[must_use]
pub fn discover_summary_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    discover(roots, is_summary_file)
}

fn discover(roots: &[PathBuf], matches: fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        collect(root, matches, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect(path: &Path, matches: fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect(&entry.path(), matches, out);
        }
    } else if matches(path) {
        out.push(path.to_path_buf());
    }
}

fn is_hand_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "txt") && !is_summary_file(path)
}

fn is_summary_file(path: &Path) -> bool {
    path.to_string_lossy().ends_with("_summary.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(path, "").expect("write file");
    }

    #[test]
    fn finds_txt_hand_files_recursively_and_excludes_summaries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand_a = dir.path().join("a.txt");
        let hand_b = dir.path().join("nested").join("b.txt");
        let summary = dir.path().join("a_summary.txt");
        let unrelated = dir.path().join("notes.md");
        for path in [&hand_a, &hand_b, &summary, &unrelated] {
            touch(path);
        }

        let files = discover_hand_files(&[dir.path().to_path_buf()]);

        assert_eq!(files, vec![hand_a, hand_b]);
    }

    #[test]
    fn finds_only_summary_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand = dir.path().join("a.txt");
        let summary = dir.path().join("nested").join("a_summary.txt");
        touch(&hand);
        touch(&summary);

        let files = discover_summary_files(&[dir.path().to_path_buf()]);

        assert_eq!(files, vec![summary]);
    }

    #[test]
    fn accepts_a_single_file_root_directly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand = dir.path().join("only.txt");
        touch(&hand);

        let files = discover_hand_files(std::slice::from_ref(&hand));

        assert_eq!(files, vec![hand]);
    }

    #[test]
    fn deduplicates_a_file_reachable_through_two_roots() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand = dir.path().join("only.txt");
        touch(&hand);

        let files = discover_hand_files(&[dir.path().to_path_buf(), hand.clone()]);

        assert_eq!(files, vec![hand]);
    }

    #[test]
    fn returns_an_empty_list_for_an_empty_or_missing_root() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("does-not-exist");

        assert!(discover_hand_files(&[dir.path().to_path_buf()]).is_empty());
        assert!(discover_hand_files(&[missing]).is_empty());
    }
}
