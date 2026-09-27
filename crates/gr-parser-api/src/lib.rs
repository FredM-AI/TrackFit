#![warn(clippy::pedantic)]

//! Contrat commun a tous les parsers de salle (PAR-1). Une salle = un module qui
//! sait detecter ses fichiers et les transformer en types `gr-core`.

mod detection;
mod encoding;
mod error;

pub use detection::{Detection, Language, Room};
pub use encoding::strip_bom;
pub use error::{ParseError, ParseErrorCode};

use gr_core::{HandRecord, TournamentSummary};

/// Parser d'une salle de poker donnee. `detect` sniffe l'en-tete pour identifier
/// la salle et la langue ; `parse_hand`/`parse_summary` transforment le texte
/// (deja decode et debarrasse de son BOM) en types `gr-core`.
pub trait RoomParser {
    fn detect(&self, input: &[u8]) -> Option<Detection>;

    /// # Errors
    /// Renvoie une [`ParseError`] si une ligne ne correspond a aucun motif connu
    /// du format (jamais de panique, PAR-15).
    fn parse_hand(&self, text: &str) -> Result<HandRecord, ParseError>;

    /// # Errors
    /// Renvoie une [`ParseError`] si une ligne ne correspond a aucun motif connu
    /// du format (jamais de panique, PAR-15).
    fn parse_summary(&self, text: &str) -> Result<TournamentSummary, ParseError>;
}
