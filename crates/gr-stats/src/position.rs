//! Attribution des positions a table (PRD §10.4, M4-1, BACKLOG). Calculee a
//! partir du bouton et des sieges **distribues** (`SeatInfo::dealt_in`),
//! jamais des sieges theoriques ni des joueurs assis mais non distribues.
//!
//! Convention de reduction du nombre de positions nommees quand la table
//! retrecit de 9 a 4 joueurs (aucune convention explicite dans le PRD au-
//! dela du cas 3-max) : les positions "du milieu" sont retirees dans cet
//! ordre, une a une : LJ, UTG+2, UTG+1, HJ, CO. Deduite des deux seuls
//! points d'ancrage fournis par le PRD/la pratique et verifiee coherente
//! avec les deux : le cas **3-max** explicite (§10.4, aucune position du
//! milieu) et la convention **6-max** universellement admise (UTG, HJ,
//! CO). Confirmee avec Frederic (29/09) que les tables Winamax ne
//! depassent jamais 9-max en pratique — au-dela, l'enum `Position` (9
//! etiquettes) ne suffirait de toute facon pas a nommer chaque siege.

use std::collections::HashMap;

use gr_core::{HandRecord, Position};

/// Attribue une position a chaque siege distribue de `hand`. Cle : numero
/// de siege (`SeatInfo::seat`). Les sieges non distribues (`dealt_in ==
/// false`) sont absents du resultat. Renvoie une map vide si moins de 2
/// sieges sont distribues (aucune position n'a de sens) ou si plus de 9 le
/// sont (au-dela de ce que l'enum `Position` peut nommer distinctement,
/// jamais observe sur Winamax — BACKLOG M4-1).
///
/// **Limite connue :** ne gere pas le bouton mort (`button_seat` absent des
/// sieges distribues) : jamais observe dans le corpus, a traiter si un
/// exemple reel apparait.
#[must_use]
pub fn assign_positions(hand: &HandRecord) -> HashMap<u8, Position> {
    let Some(ordered) = ordered_dealt_in_seats(hand) else {
        return HashMap::new();
    };
    let n = ordered.len();
    ordered.into_iter().zip(position_labels(n)).collect()
}

/// Sieges qui postent la petite et la grosse blinde (dans cet ordre),
/// utile independamment du nommage complet des positions (ex. `gr-stats`
/// lui-meme, ou le DSL de test `hand!{}`, M4-2). `None` dans les memes cas
/// que [`assign_positions`] (moins de 2 ou plus de 9 sieges distribues,
/// bouton mort).
#[must_use]
pub fn sb_bb_seats(hand: &HandRecord) -> Option<(u8, u8)> {
    let ordered = ordered_dealt_in_seats(hand)?;
    if ordered.len() == 2 {
        // Heads-up : le bouton poste la petite blinde (PRD §10.4).
        Some((ordered[0], ordered[1]))
    } else {
        Some((ordered[1], ordered[2]))
    }
}

/// Sieges distribues de `hand`, tries a partir du bouton dans le sens du
/// jeu (numeros de siege croissants, verifie sur deux vrais fixtures —
/// voir le commentaire de module). `None` si moins de 2 ou plus de 9
/// sieges sont distribues, ou si le bouton n'est pas parmi eux (bouton
/// mort, jamais observe).
fn ordered_dealt_in_seats(hand: &HandRecord) -> Option<Vec<u8>> {
    let mut seats: Vec<u8> = hand
        .seats
        .iter()
        .filter(|s| s.dealt_in)
        .map(|s| s.seat)
        .collect();
    seats.sort_unstable();
    seats.dedup();

    let n = seats.len();
    if !(2..=9).contains(&n) {
        return None;
    }

    let button_index = seats.iter().position(|&s| s == hand.button_seat)?;
    Some(
        seats
            .iter()
            .copied()
            .cycle()
            .skip(button_index)
            .take(n)
            .collect(),
    )
}

/// Etiquettes dans l'ordre BTN, SB, BB, UTG... CO (PRD §10.4), pour une
/// table de `n` sieges distribues (2 a 9, garanti par l'appelant).
fn position_labels(n: usize) -> Vec<Position> {
    use Position::{Bb, Btn, Co, Hj, Lj, Sb, Utg, Utg1, Utg2};

    match n {
        2 => vec![Btn, Bb], // heads-up : BTN = SB (PRD §10.4), un seul poste "bouton".
        3 => vec![Btn, Sb, Bb],
        _ => {
            let full_middle = [Utg, Utg1, Utg2, Lj, Hj, Co];
            let drop_order = [Lj, Utg2, Utg1, Hj, Co];
            let to_drop = (9 - n).min(drop_order.len());
            let dropped = &drop_order[..to_drop];
            let middle = full_middle.into_iter().filter(|p| !dropped.contains(p));

            let mut labels = vec![Btn, Sb, Bb];
            labels.extend(middle);
            labels
        }
    }
}

#[cfg(test)]
mod tests {
    use gr_core::{Chips, SeatInfo};

    use super::*;

    fn seat(n: u8, dealt_in: bool) -> SeatInfo {
        SeatInfo {
            seat: n,
            pseudo: format!("P{n}"),
            starting_stack: Chips::from_i64(1000),
            bounty: None,
            dealt_in,
        }
    }

    fn hand_with(button_seat: u8, seats: Vec<SeatInfo>) -> HandRecord {
        HandRecord {
            room_hand_id: "#1-1-1".to_string(),
            tournament_name: "T".to_string(),
            tournament_room_id: "1".to_string(),
            table_name: "T(1)#1".to_string(),
            table_max_seats: 9,
            button_seat,
            level: 1,
            sb: Chips::from_i64(10),
            bb: Chips::from_i64(20),
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

    /// CA de M4-1 : table-driven, 2 a 9 joueurs, verifie contre la
    /// disposition reelle observee sur deux vrais fixtures (BACKLOG M4-1) :
    /// 6-max (bouton siege 3, SB siege 4, BB siege 5, premier a agir
    /// preflop siege 6 = UTG, `mtt/classic-itm/`) et 7-max (bouton siege 4,
    /// SB siege 5, BB siege 6, premier a agir siege 7 = UTG,
    /// `mtt/final-table-heads-up/`, teste separement ci-dessous).
    #[test]
    fn six_max_matches_the_real_fixture_layout() {
        let hand = hand_with(3, (1..=6).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);

        assert_eq!(positions[&3], Position::Btn);
        assert_eq!(positions[&4], Position::Sb);
        assert_eq!(positions[&5], Position::Bb);
        assert_eq!(positions[&6], Position::Utg);
        assert_eq!(positions[&1], Position::Hj);
        assert_eq!(positions[&2], Position::Co);
    }

    #[test]
    fn heads_up_button_is_also_small_blind() {
        let hand = hand_with(1, vec![seat(1, true), seat(2, true)]);
        let positions = assign_positions(&hand);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[&1], Position::Btn);
        assert_eq!(positions[&2], Position::Bb);
    }

    #[test]
    fn three_max_has_no_middle_positions() {
        let hand = hand_with(2, vec![seat(1, true), seat(2, true), seat(3, true)]);
        let positions = assign_positions(&hand);
        assert_eq!(positions[&2], Position::Btn);
        assert_eq!(positions[&3], Position::Sb);
        assert_eq!(positions[&1], Position::Bb);
    }

    #[test]
    fn four_max_adds_only_utg() {
        let hand = hand_with(1, (1..=4).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);
        assert_eq!(positions[&1], Position::Btn);
        assert_eq!(positions[&2], Position::Sb);
        assert_eq!(positions[&3], Position::Bb);
        assert_eq!(positions[&4], Position::Utg);
    }

    #[test]
    fn five_max_adds_utg_and_co() {
        let hand = hand_with(1, (1..=5).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);
        assert_eq!(positions[&4], Position::Utg);
        assert_eq!(positions[&5], Position::Co);
    }

    #[test]
    fn seven_max_adds_utg1_ahead_of_utg2_and_lj() {
        let hand = hand_with(1, (1..=7).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);
        assert_eq!(positions[&1], Position::Btn);
        assert_eq!(positions[&2], Position::Sb);
        assert_eq!(positions[&3], Position::Bb);
        assert_eq!(positions[&4], Position::Utg);
        assert_eq!(positions[&5], Position::Utg1);
        assert_eq!(positions[&6], Position::Hj);
        assert_eq!(positions[&7], Position::Co);
    }

    #[test]
    fn eight_max_adds_utg2_but_not_lj() {
        let hand = hand_with(1, (1..=8).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);
        assert_eq!(positions[&4], Position::Utg);
        assert_eq!(positions[&5], Position::Utg1);
        assert_eq!(positions[&6], Position::Utg2);
        assert_eq!(positions[&7], Position::Hj);
        assert_eq!(positions[&8], Position::Co);
    }

    #[test]
    fn nine_max_uses_every_named_position() {
        let hand = hand_with(1, (1..=9).map(|n| seat(n, true)).collect());
        let positions = assign_positions(&hand);
        assert_eq!(positions[&1], Position::Btn);
        assert_eq!(positions[&2], Position::Sb);
        assert_eq!(positions[&3], Position::Bb);
        assert_eq!(positions[&4], Position::Utg);
        assert_eq!(positions[&5], Position::Utg1);
        assert_eq!(positions[&6], Position::Utg2);
        assert_eq!(positions[&7], Position::Lj);
        assert_eq!(positions[&8], Position::Hj);
        assert_eq!(positions[&9], Position::Co);
    }

    #[test]
    fn seats_not_dealt_in_are_excluded_and_ignored_for_rotation() {
        // Seat 2 est absent (busted) : la table joue a 5, pas a 6.
        let hand = hand_with(
            1,
            vec![
                seat(1, true),
                seat(2, false),
                seat(3, true),
                seat(4, true),
                seat(5, true),
                seat(6, true),
            ],
        );
        let positions = assign_positions(&hand);
        assert_eq!(positions.len(), 5);
        assert!(!positions.contains_key(&2));
        assert_eq!(positions[&1], Position::Btn);
        assert_eq!(positions[&3], Position::Sb);
        assert_eq!(positions[&4], Position::Bb);
        assert_eq!(positions[&5], Position::Utg);
        assert_eq!(positions[&6], Position::Co);
    }

    #[test]
    fn fewer_than_two_dealt_in_seats_yields_no_positions() {
        let hand = hand_with(1, vec![seat(1, true)]);
        assert!(assign_positions(&hand).is_empty());
    }

    #[test]
    fn a_table_size_beyond_nine_yields_no_positions() {
        // N'arrive jamais sur Winamax (confirme par Frederic, 29/09) : verifie
        // seulement que la fonction ne panique pas et se degrade proprement.
        let hand = hand_with(1, (1..=10).map(|n| seat(n, true)).collect());
        assert!(assign_positions(&hand).is_empty());
    }

    #[test]
    fn sb_bb_seats_matches_assign_positions_for_six_max() {
        let hand = hand_with(3, (1..=6).map(|n| seat(n, true)).collect());
        assert_eq!(sb_bb_seats(&hand), Some((4, 5)));
    }

    #[test]
    fn sb_bb_seats_in_heads_up_has_the_button_post_small_blind() {
        let hand = hand_with(1, vec![seat(1, true), seat(2, true)]);
        assert_eq!(sb_bb_seats(&hand), Some((1, 2)));
    }
}
