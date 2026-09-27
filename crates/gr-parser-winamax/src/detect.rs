use gr_core::HandRecord;
use gr_parser_api::{strip_bom, Detection, Language, ParseError, Room};

use crate::hand;

/// Signature commune aux fichiers de mains (`Winamax Poker - Tournament "..."`)
/// et de summary (`Winamax Poker - Tournament summary : ...`), §4.1/§5.1.
const SIGNATURE: &str = "Winamax Poker - Tournament";

/// Parser du format Winamax (fichiers en anglais meme avec un client FR, ADR-005).
#[derive(Debug, Default, Clone, Copy)]
pub struct WinamaxParser;

impl WinamaxParser {
    /// Detecte si `input` est un fichier Winamax, en tolerant un BOM UTF-8 en tete (PAR-2).
    #[must_use]
    pub fn detect(input: &[u8]) -> Option<Detection> {
        let bytes = strip_bom(input);
        let text = std::str::from_utf8(bytes).ok()?;
        let first_line = text.lines().next()?;
        if first_line.starts_with(SIGNATURE) {
            Some(Detection {
                room: Room::Winamax,
                language: Language::English,
            })
        } else {
            None
        }
    }

    /// Parse une main (PAR-4/5/6). Les actions, le board et les pots restent
    /// vides jusqu'a M1-5.
    ///
    /// # Errors
    /// Renvoie une [`ParseError`] si l'en-tete, la table ou un siege ne
    /// correspond a aucun motif connu (PAR-15).
    pub fn parse_hand(text: &str) -> Result<HandRecord, ParseError> {
        hand::parse_hand(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn detects_a_hand_file_header() {
        let header = "Winamax Poker - Tournament \"VELOCITY\" buyIn: 1.80\u{20ac} + 0.20\u{20ac} level: 1 - HandId: #1-2-3 - Holdem no limit (10/20) - 2026/09/25 00:00:00 UTC\n";
        let detection = WinamaxParser::detect(header.as_bytes()).unwrap();
        assert_eq!(detection.room, Room::Winamax);
        assert_eq!(detection.language, Language::English);
    }

    #[test]
    fn detects_a_summary_file_header() {
        let header = "Winamax Poker - Tournament summary : VELOCITY(1174075270)\n";
        assert!(WinamaxParser::detect(header.as_bytes()).is_some());
    }

    #[test]
    fn tolerates_a_leading_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"Winamax Poker - Tournament \"VELOCITY\"\n");
        assert!(WinamaxParser::detect(&bytes).is_some());
    }

    #[test]
    fn rejects_unrelated_content() {
        assert!(WinamaxParser::detect(b"hello world").is_none());
        assert!(WinamaxParser::detect(b"").is_none());
    }

    fn collect_txt_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_txt_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "txt") {
                out.push(path);
            }
        }
    }

    /// CA de M1-2 : les fichiers du corpus committe sont detectes `winamax` a 100 %.
    #[test]
    fn detects_every_committed_fixture_as_winamax() {
        let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax");
        let mut files = Vec::new();
        collect_txt_files(&fixtures_dir, &mut files);
        assert!(!files.is_empty(), "no fixture found under {fixtures_dir:?}");

        for path in files {
            let bytes = std::fs::read(&path).unwrap();
            let detection = WinamaxParser::detect(&bytes);
            assert!(detection.is_some(), "failed to detect {path:?} as winamax");
            assert_eq!(detection.unwrap().room, Room::Winamax);
        }
    }
}
