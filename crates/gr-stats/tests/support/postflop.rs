//! Extension postflop du DSL de test (M4-4). Inclus uniquement par
//! `postflop_flags.rs` via `#[path = "support/postflop.rs"] mod postflop;`
//! (pas par `support/mod.rs` lui-meme) : ses helpers ne seraient jamais
//! utilises par les binaires de test preflop-only (`hand_macro.rs`,
//! `preflop_flags.rs`), ce que `dead_code` (clippy `-D warnings`) detecte
//! binaire par binaire.
//!
//! `hand!{}` reste preflop-only (blindes/antes generes automatiquement) ;
//! les actions postflop se fournissent explicitement, rue par rue, via ces
//! helpers, puis s'ajoutent a une main deja construite avec
//! `with_postflop`.

use gr_core::{ActionKind, ActionRecord, Card, Chips, HandRecord, PotKind, PotResult, Street};

fn postflop_action(street: Street, pseudo: &str, kind: ActionKind) -> ActionRecord {
    ActionRecord {
        street,
        pseudo: pseudo.to_string(),
        kind,
        amount: None,
        to_amount: None,
        is_all_in: false,
        pot: None,
        shown_cards: None,
        shown_label: None,
    }
}

/// Le joueur checke sur `street`.
pub fn check_on(pseudo: &str, street: Street) -> ActionRecord {
    postflop_action(street, pseudo, ActionKind::Check)
}

/// Le joueur se couche sur `street`.
pub fn fold_on(pseudo: &str, street: Street) -> ActionRecord {
    postflop_action(street, pseudo, ActionKind::Fold)
}

/// Le joueur mise `amount` jetons sur `street` (premiere mise de la rue).
pub fn bet_on(pseudo: &str, amount: i64, street: Street) -> ActionRecord {
    let mut action = postflop_action(street, pseudo, ActionKind::Bet);
    action.amount = Some(Chips::from_i64(amount));
    action
}

/// Le joueur suit pour `amount` jetons sur `street`.
pub fn call_on(pseudo: &str, amount: i64, street: Street) -> ActionRecord {
    let mut action = postflop_action(street, pseudo, ActionKind::Call);
    action.amount = Some(Chips::from_i64(amount));
    action
}

/// Le joueur relance a `to` jetons au total sur `street`.
pub fn raise_on(pseudo: &str, to: i64, street: Street) -> ActionRecord {
    let mut action = postflop_action(street, pseudo, ActionKind::Raise);
    action.to_amount = Some(Chips::from_i64(to));
    action
}

/// Le joueur montre ses cartes a l'abattage (`Street::Showdown` ; cartes et
/// libelle non necessaires aux stats de M4-4, laisses vides).
pub fn shows(pseudo: &str) -> ActionRecord {
    postflop_action(Street::Showdown, pseudo, ActionKind::Shows)
}

fn card(spec: &str) -> Card {
    spec.parse()
        .unwrap_or_else(|_| panic!("carte de test invalide : {spec}"))
}

/// Etend une main deja construite par `hand!{}` avec un board et des
/// actions postflop, et (optionnellement) le(s) gagnant(s) du pot — pour
/// `won_pot`/WSD/WWSF (M4-4). `winners` vide laisse `hand.pots` a vide
/// (comme construit par `build_hand`).
pub fn with_postflop(
    mut hand: HandRecord,
    board: &[&str],
    postflop_actions: Vec<ActionRecord>,
    winners: &[(&str, i64)],
) -> HandRecord {
    hand.board = board.iter().map(|spec| card(spec)).collect();
    hand.actions.extend(postflop_actions);
    if !winners.is_empty() {
        let pot_winners: Vec<(String, Chips)> = winners
            .iter()
            .map(|(pseudo, amount)| ((*pseudo).to_string(), Chips::from_i64(*amount)))
            .collect();
        let total: i64 = pot_winners.iter().map(|(_, c)| c.amount()).sum();
        hand.total_pot = Chips::from_i64(total);
        hand.pots = vec![PotResult {
            pot: PotKind::Pot,
            amount: Chips::from_i64(total),
            winners: pot_winners,
        }];
    }
    hand
}
