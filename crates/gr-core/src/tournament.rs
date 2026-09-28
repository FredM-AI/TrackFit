use serde::{Deserialize, Serialize};

use crate::money::Money;

/// Type de knockout du tournoi (correspond a `tournaments.ko_type` en base, PRD §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KoType {
    None,
    Ko,
    Pko,
    Mystery,
    Space,
}

/// Une entree du Hero dans le tournoi (un bloc du fichier summary, §5.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TournamentBullet {
    pub entry_no: u32,
    /// Suffixe ` - Late Registration` sur le bloc.
    pub late_reg_bust: bool,
    pub finish_position: Option<u32>,
    pub played_seconds: u32,
    pub prize: Money,
    pub bounty: Money,
    /// `Registered players` au moment de ce bloc (instantane, §5.2).
    pub registered_snapshot: u32,
}

/// Le fichier summary d'un tournoi, une fois toutes ses entrees agregees (§5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TournamentSummary {
    pub room_tournament_id: String,
    pub name: String,
    /// Pseudo du Hero (ligne `Player : <pseudo>`, §5.2), pris du dernier
    /// bloc comme les autres champs constants du header.
    pub hero_pseudo: String,
    pub buyin_prize: Money,
    pub buyin_bounty: Money,
    pub buyin_fee: Money,
    /// `tt` (tournoi) ou `sng` (sit'n'go a table unique, hors perimetre D2).
    pub mode: String,
    /// Valeur brute du champ `Type` (`knockout`, `normal`, `flight`, ...).
    pub tournament_type: String,
    pub speed: String,
    /// `0` = tournoi a un seul flight ; sinon identifie le flight de depart.
    pub flight_id: u32,
    pub bullets: Vec<TournamentBullet>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_preserves_the_summary() {
        let summary = TournamentSummary {
            room_tournament_id: "1173012730".to_string(),
            name: "OBELISK - TRIDENT SPACE KO".to_string(),
            hero_pseudo: "Hero".to_string(),
            buyin_prize: Money::from_cents(80),
            buyin_bounty: Money::from_cents(100),
            buyin_fee: Money::from_cents(20),
            mode: "tt".to_string(),
            tournament_type: "knockout".to_string(),
            speed: "turbo".to_string(),
            flight_id: 0,
            bullets: vec![TournamentBullet {
                entry_no: 1,
                late_reg_bust: true,
                finish_position: Some(737),
                played_seconds: 327,
                prize: Money::ZERO,
                bounty: Money::ZERO,
                registered_snapshot: 812,
            }],
        };
        let json = serde_json::to_string(&summary).unwrap();
        let back: TournamentSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, summary);
    }
}
