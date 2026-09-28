//! Detection des comptes Winamax locaux (M3-1, UC1, PRD §8.1) :
//! `%APPDATA%\winamax\documents\accounts\<PSEUDO>\history\`. Chaque
//! sous-dossier de `accounts_dir` est un pseudo candidat pour le Hero (D19).

use std::path::{Path, PathBuf};

use crate::scan::discover_hand_files;

/// Un compte Winamax local detecte : pseudo (nom du sous-dossier) et dossier
/// d'historiques, avec un decompte indicatif de fichiers de mains deja presents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinamaxAccount {
    pub pseudo: String,
    pub history_dir: PathBuf,
    pub hand_file_count: usize,
}

/// Explore `accounts_dir` (`%APPDATA%\winamax\documents\accounts\`) et
/// renvoie, triee par pseudo, la liste des comptes ayant un sous-dossier
/// d'historiques. La casse de `history` n'est pas garantie (§8.1 ⚠️) : les
/// deux variantes `history`/`History` sont tolerees, ainsi que toute autre
/// casse par comparaison insensible.
#[must_use]
pub fn detect_winamax_accounts(accounts_dir: &Path) -> Vec<WinamaxAccount> {
    let Ok(entries) = std::fs::read_dir(accounts_dir) else {
        return Vec::new();
    };

    let mut accounts: Vec<WinamaxAccount> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let pseudo = path.file_name()?.to_string_lossy().into_owned();
            let history_dir = find_history_dir(&path)?;
            let hand_file_count = discover_hand_files(std::slice::from_ref(&history_dir)).len();
            Some(WinamaxAccount {
                pseudo,
                history_dir,
                hand_file_count,
            })
        })
        .collect();

    accounts.sort_by(|a, b| a.pseudo.cmp(&b.pseudo));
    accounts
}

fn find_history_dir(account_dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(account_dir) else {
        return None;
    };
    entries.flatten().find_map(|entry| {
        let path = entry.path();
        let is_history = path.is_dir()
            && path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("history"));
        is_history.then_some(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(path, "x").expect("write file");
    }

    #[test]
    fn detects_an_account_with_a_lowercase_history_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand = dir.path().join("Hero").join("history").join("a.txt");
        touch(&hand);

        let accounts = detect_winamax_accounts(dir.path());

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].pseudo, "Hero");
        assert_eq!(
            accounts[0].history_dir,
            dir.path().join("Hero").join("history")
        );
        assert_eq!(accounts[0].hand_file_count, 1);
    }

    #[test]
    fn tolerates_an_uppercase_history_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hand = dir.path().join("Hero").join("History").join("a.txt");
        touch(&hand);

        let accounts = detect_winamax_accounts(dir.path());

        assert_eq!(accounts.len(), 1);
        assert_eq!(
            accounts[0].history_dir,
            dir.path().join("Hero").join("History")
        );
    }

    #[test]
    fn skips_an_account_folder_without_any_history_subfolder() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("NoHistory")).expect("create dir");

        assert!(detect_winamax_accounts(dir.path()).is_empty());
    }

    #[test]
    fn returns_an_empty_list_for_a_missing_accounts_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("does-not-exist");
        assert!(detect_winamax_accounts(&missing).is_empty());
    }

    #[test]
    fn sorts_accounts_by_pseudo_and_counts_their_hand_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        touch(&dir.path().join("Zoe").join("history").join("a.txt"));
        touch(&dir.path().join("Zoe").join("history").join("b.txt"));
        touch(&dir.path().join("Alice").join("history").join("a.txt"));

        let accounts = detect_winamax_accounts(dir.path());

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].pseudo, "Alice");
        assert_eq!(accounts[0].hand_file_count, 1);
        assert_eq!(accounts[1].pseudo, "Zoe");
        assert_eq!(accounts[1].hand_file_count, 2);
    }
}
