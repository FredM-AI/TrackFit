use thiserror::Error;

/// Erreur de parsing d'une carte au format `"Ah"` (rang + couleur).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CardParseError {
    #[error("format de carte invalide : {0:?} (attendu 2 caracteres, ex. \"Ah\")")]
    InvalidLength(String),
    #[error("rang de carte inconnu : {0:?}")]
    UnknownRank(char),
    #[error("couleur de carte inconnue : {0:?}")]
    UnknownSuit(char),
}
