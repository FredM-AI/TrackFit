//! Profondeur de tapis (PRD §10.3, D24, M4-1).
//!
//! - **bb du joueur** = tapis de depart de la main / big blind.
//! - **bb effectifs** = min(tapis du joueur, plus gros tapis parmi les
//!   adversaires encore en jeu) / big blind — "encore en jeu" au debut de
//!   la main = tout siege distribue autre que le joueur lui-meme (avant
//!   toute action, personne n'a encore pu se coucher).
//! - Tranches (bornes basses incluses, hautes exclues) : configurables,
//!   `DEFAULT_DEPTH_BRACKETS` reprend les valeurs par defaut du PRD.

use gr_core::HandRecord;

/// Bornes par defaut du PRD §10.3 : `<10`, `10-15`, `15-25`, `25-40`,
/// `40-60`, `60-80`, `80-100`, `100+`.
pub const DEFAULT_DEPTH_BRACKETS: &[f64] = &[10.0, 15.0, 25.0, 40.0, 60.0, 80.0, 100.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    /// Tapis du joueur / bb, sans tenir compte des adversaires.
    Player,
    /// min(tapis du joueur, plus gros tapis adverse) / bb (mode par
    /// defaut du PRD).
    Effective,
}

/// Profondeur (en bb) du siege `seat` au debut de `hand`, selon `mode`.
/// `None` si `seat` n'est pas distribue dans cette main, ou si `hand.bb`
/// n'est pas strictement positif (evite une division par zero ; une big
/// blind nulle ne devrait jamais arriver en pratique).
#[must_use]
pub fn depth_bb(hand: &HandRecord, seat: u8, mode: DepthMode) -> Option<f64> {
    if hand.bb.amount() <= 0 {
        return None;
    }
    let bb = precise_f64(hand.bb.amount());

    let player_stack = hand
        .seats
        .iter()
        .find(|s| s.seat == seat && s.dealt_in)
        .map(|s| s.starting_stack.amount())?;

    let stack = match mode {
        DepthMode::Player => player_stack,
        DepthMode::Effective => {
            let largest_opponent = hand
                .seats
                .iter()
                .filter(|s| s.dealt_in && s.seat != seat)
                .map(|s| s.starting_stack.amount())
                .max();
            match largest_opponent {
                Some(opponent_stack) => player_stack.min(opponent_stack),
                // Aucun adversaire distribue (main invalide en pratique,
                // PAR-garanti par le parser) : degrade sur le tapis du joueur.
                None => player_stack,
            }
        }
    };

    Some(precise_f64(stack) / bb)
}

/// Convertit un montant de jetons en `f64` pour un ratio d'affichage
/// (jamais stocke, R-MONEY). Sans perte pour tout tapis de tournoi
/// realiste (tres loin de `2^53`).
fn precise_f64(amount: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let value = amount as f64;
    value
}

/// Etiquette de tranche pour `depth_bb` (PRD §10.3 : bornes basses
/// incluses, hautes exclues). `boundaries` doit etre trie strictement
/// croissant (precondition de l'appelant ; `DEFAULT_DEPTH_BRACKETS` l'est).
/// Renvoie `"?"` si `boundaries` est vide (tranches configurables
/// videes par erreur plutot que de paniquer, R-NOPANIC).
#[must_use]
pub fn depth_bracket_label(depth_bb: f64, boundaries: &[f64]) -> String {
    let (Some(&first), Some(&last)) = (boundaries.first(), boundaries.last()) else {
        return "?".to_string();
    };
    if depth_bb < first {
        return format!("<{}", fmt_bound(first));
    }
    for pair in boundaries.windows(2) {
        let (low, high) = (pair[0], pair[1]);
        if depth_bb >= low && depth_bb < high {
            return format!("{}-{}", fmt_bound(low), fmt_bound(high));
        }
    }
    format!("{}+", fmt_bound(last))
}

fn fmt_bound(v: f64) -> String {
    if v.fract() == 0.0 {
        #[allow(clippy::cast_possible_truncation)]
        let rounded = v as i64;
        rounded.to_string()
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use gr_core::{Chips, SeatInfo};

    use super::*;

    fn seat(n: u8, stack: i64, dealt_in: bool) -> SeatInfo {
        SeatInfo {
            seat: n,
            pseudo: format!("P{n}"),
            starting_stack: Chips::from_i64(stack),
            bounty: None,
            dealt_in,
        }
    }

    fn hand_with(bb: i64, seats: Vec<SeatInfo>) -> HandRecord {
        HandRecord {
            room_hand_id: "#1-1-1".to_string(),
            tournament_name: "T".to_string(),
            tournament_room_id: "1".to_string(),
            table_name: "T(1)#1".to_string(),
            table_max_seats: 9,
            button_seat: 1,
            level: 1,
            sb: Chips::from_i64(bb / 2),
            bb: Chips::from_i64(bb),
            ante: Chips::ZERO,
            played_at: 0,
            seats,
            actions: vec![],
            board: vec![],
            pots: vec![],
            total_pot: Chips::ZERO,
            rake: Chips::ZERO,
            uncalled_excess: Chips::ZERO,
            hero_pseudo: None,
            hero_cards: None,
            parser_version: "0.0.1".to_string(),
        }
    }

    #[test]
    fn player_depth_is_stack_over_bb() {
        let hand = hand_with(20, vec![seat(1, 2000, true), seat(2, 500, true)]);
        assert_eq!(depth_bb(&hand, 1, DepthMode::Player), Some(100.0));
    }

    #[test]
    fn effective_depth_caps_at_the_largest_opponent_stack() {
        let hand = hand_with(
            20,
            vec![seat(1, 2000, true), seat(2, 500, true), seat(3, 300, true)],
        );
        // Hero (2000) face au plus gros adversaire (500) : effectif = 500/20 = 25bb.
        assert_eq!(depth_bb(&hand, 1, DepthMode::Effective), Some(25.0));
    }

    #[test]
    fn effective_depth_is_the_players_own_stack_when_it_is_the_smallest() {
        let hand = hand_with(20, vec![seat(1, 300, true), seat(2, 2000, true)]);
        assert_eq!(depth_bb(&hand, 1, DepthMode::Effective), Some(15.0));
    }

    #[test]
    fn a_seat_not_dealt_in_has_no_depth() {
        let hand = hand_with(20, vec![seat(1, 2000, false)]);
        assert_eq!(depth_bb(&hand, 1, DepthMode::Player), None);
    }

    #[test]
    fn an_unknown_seat_has_no_depth() {
        let hand = hand_with(20, vec![seat(1, 2000, true)]);
        assert_eq!(depth_bb(&hand, 9, DepthMode::Player), None);
    }

    #[test]
    fn a_zero_big_blind_is_rejected_to_avoid_a_division_by_zero() {
        let hand = hand_with(0, vec![seat(1, 2000, true)]);
        assert_eq!(depth_bb(&hand, 1, DepthMode::Player), None);
    }

    #[test]
    fn brackets_use_inclusive_low_and_exclusive_high_bounds() {
        assert_eq!(depth_bracket_label(5.0, DEFAULT_DEPTH_BRACKETS), "<10");
        assert_eq!(depth_bracket_label(10.0, DEFAULT_DEPTH_BRACKETS), "10-15");
        assert_eq!(depth_bracket_label(14.999, DEFAULT_DEPTH_BRACKETS), "10-15");
        assert_eq!(depth_bracket_label(15.0, DEFAULT_DEPTH_BRACKETS), "15-25");
        assert_eq!(depth_bracket_label(100.0, DEFAULT_DEPTH_BRACKETS), "100+");
        assert_eq!(depth_bracket_label(250.0, DEFAULT_DEPTH_BRACKETS), "100+");
    }

    #[test]
    fn brackets_are_configurable() {
        let custom = [20.0, 50.0];
        assert_eq!(depth_bracket_label(10.0, &custom), "<20");
        assert_eq!(depth_bracket_label(30.0, &custom), "20-50");
        assert_eq!(depth_bracket_label(60.0, &custom), "50+");
    }
}
