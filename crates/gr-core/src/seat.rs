use serde::{Deserialize, Serialize};

use crate::money::{Chips, Money};

/// Un siege a la table au debut de la main (PAR-6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatInfo {
    pub seat: u8,
    pub pseudo: String,
    pub starting_stack: Chips,
    /// Bounty courant du joueur, si le tournoi est un KO (`, X€ bounty`).
    pub bounty: Option<Money>,
    /// `false` si le joueur est assis mais n'a recu aucune carte (exclu de la main).
    pub dealt_in: bool,
}
