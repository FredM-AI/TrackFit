//! Un snapshot insta par fichier de mains et par fichier summary du corpus
//! (CLAUDE.md §7) : chaque fichier est parse integralement et compare a la
//! reference validee.

use std::path::{Path, PathBuf};

use crate::{split_hand_blocks, WinamaxParser};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "txt") {
            out.push(path);
        }
    }
}

fn hand_files() -> Vec<PathBuf> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax");
    let mut files = Vec::new();
    collect(&fixtures_dir, &mut files);
    files.retain(|p| !p.to_string_lossy().ends_with("_summary.txt"));
    files.sort();
    files
}

fn summary_files() -> Vec<PathBuf> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax");
    let mut files = Vec::new();
    collect(&fixtures_dir, &mut files);
    files.retain(|p| p.to_string_lossy().ends_with("_summary.txt"));
    files.sort();
    files
}

fn snapshot_name(path: &Path) -> String {
    path.file_stem()
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
        .collect()
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

        insta::assert_debug_snapshot!(snapshot_name(&path), hands);
    }
}

#[test]
fn snapshot_every_corpus_summary_file() {
    let files = summary_files();
    assert!(
        !files.is_empty(),
        "no summary file found under fixtures/winamax"
    );

    for path in files {
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
        let summary =
            WinamaxParser::parse_summary(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        insta::assert_debug_snapshot!(snapshot_name(&path), summary);
    }
}
