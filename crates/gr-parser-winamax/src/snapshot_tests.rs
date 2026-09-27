//! Un snapshot insta par fichier de mains du corpus (CLAUDE.md §7) : chaque
//! main est decoupee puis parsee (en-tete/table/sieges, PAR-4/5/6 ; actions,
//! board et pots restent vides jusqu'a M1-5) et comparee a la reference validee.

use std::path::{Path, PathBuf};

use crate::{split_hand_blocks, WinamaxParser};

fn hand_files() -> Vec<PathBuf> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax");
    let mut files = Vec::new();
    collect(&fixtures_dir, &mut files);
    files.sort();
    files
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "txt")
            && !path.to_string_lossy().ends_with("_summary.txt")
        {
            out.push(path);
        }
    }
}

#[test]
fn snapshot_every_corpus_hand_file() {
    let files = hand_files();
    assert!(
        !files.is_empty(),
        "no hand file found under fixtures/winamax"
    );

    for path in files {
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
        let (blocks, _offset) = split_hand_blocks(&text);
        assert!(!blocks.is_empty(), "{path:?} contains no hand block");

        let hands: Vec<_> = blocks
            .iter()
            .enumerate()
            .map(|(i, block)| {
                WinamaxParser::parse_hand(block)
                    .unwrap_or_else(|e| panic!("{path:?}, hand {}: {e}", i + 1))
            })
            .collect();

        let snapshot_name: String = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        insta::assert_debug_snapshot!(snapshot_name, hands);
    }
}
