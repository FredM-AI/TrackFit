use std::fmt;

use thiserror::Error;

/// Code d'erreur de parsing, stable et affiche a l'utilisateur (ecran Logs, PRD §13.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorCode {
    /// Une ligne ne correspond a aucun motif connu (PAR-7/PAR-15). R-FORMAT :
    /// jamais de logique speculative, un fixture doit d'abord documenter la ligne.
    UnknownLine,
    /// L'invariant de conservation des jetons est viole (PAR-11).
    ChipMismatch,
}

impl fmt::Display for ParseErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            ParseErrorCode::UnknownLine => "UNKNOWN_LINE",
            ParseErrorCode::ChipMismatch => "CHIP_MISMATCH",
        };
        f.write_str(code)
    }
}

/// Erreur de parsing d'une main ou d'un summary (PAR-15) : jamais de panique,
/// toujours une erreur typee portant le code, la ligne et son contexte.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code} (ligne {line_no}): {line}")]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub line_no: usize,
    pub line: String,
    pub context: String,
}
