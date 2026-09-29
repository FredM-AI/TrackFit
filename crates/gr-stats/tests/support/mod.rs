//! DSL de test `hand!{}` (M4-2, `gr-stats/tests/support`) : construction
//! concise de `HandRecord` pour les tests de stats (`gr-stats/tests/*.rs`,
//! CLAUDE.md §7). Les postes d'ante/blindes sont generes automatiquement a
//! partir de `button`/`sb`/`bb`/`ante` et des sieges (reutilise
//! `gr_stats::sb_bb_seats`, M4-1) : seules les actions **volontaires**
//! (fold/call/raise...) sont a fournir dans `preflop`, exactement ce dont
//! les flags de stats (VPIP, PFR... M4-3) ont besoin.
//!
//! Portee volontairement limitee au preflop pour l'instant (M4-3 en a
//! besoin) ; a etendre avec des rues postflop quand M4-4 en aura besoin.

use gr_core::{ActionKind, ActionRecord, Chips, HandRecord, SeatInfo, Street};
use gr_stats::sb_bb_seats;

/// Assemble la main a partir des morceaux collectes par la macro `hand!`
/// (non destinee a etre appelee directement).
#[doc(hidden)]
#[must_use]
pub fn build_hand(
    seats: Vec<SeatInfo>,
    button: u8,
    sb: i64,
    bb: i64,
    ante: i64,
    preflop_actions: Vec<ActionRecord>,
) -> HandRecord {
    let table_max_seats = u8::try_from(seats.len()).unwrap_or(u8::MAX);
    let hero_pseudo = seats
        .iter()
        .find(|s| s.pseudo == "Hero")
        .map(|s| s.pseudo.clone());

    let mut hand = HandRecord {
        room_hand_id: "#1-1-1".to_string(),
        tournament_name: "Test".to_string(),
        tournament_room_id: "1".to_string(),
        table_name: "Test(1)#1".to_string(),
        table_max_seats,
        button_seat: button,
        level: 1,
        sb: Chips::from_i64(sb),
        bb: Chips::from_i64(bb),
        ante: Chips::from_i64(ante),
        played_at: 0,
        seats,
        actions: Vec::new(),
        board: Vec::new(),
        pots: Vec::new(),
        total_pot: Chips::ZERO,
        rake: Chips::ZERO,
        uncalled_excess: Chips::ZERO,
        hero_pseudo,
        hero_cards: None,
        parser_version: "0.0.1".to_string(),
    };

    let mut actions = Vec::new();
    if ante > 0 {
        for seat in &hand.seats {
            actions.push(post_ante(&seat.pseudo, ante));
        }
    }
    if let Some((sb_seat, bb_seat)) = sb_bb_seats(&hand) {
        actions.push(post_sb(&seat_pseudo(&hand, sb_seat), sb));
        actions.push(post_bb(&seat_pseudo(&hand, bb_seat), bb));
    }
    actions.extend(preflop_actions);

    hand.actions = actions;
    hand
}

fn seat_pseudo(hand: &HandRecord, seat: u8) -> String {
    hand.seats
        .iter()
        .find(|s| s.seat == seat)
        .map_or_else(String::new, |s| s.pseudo.clone())
}

fn base_action(pseudo: &str, kind: ActionKind) -> ActionRecord {
    ActionRecord {
        street: Street::Preflop,
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

pub fn post_ante(pseudo: &str, amount: i64) -> ActionRecord {
    let mut action = base_action(pseudo, ActionKind::PostAnte);
    action.amount = Some(Chips::from_i64(amount));
    action
}

pub fn post_sb(pseudo: &str, amount: i64) -> ActionRecord {
    let mut action = base_action(pseudo, ActionKind::PostSmallBlind);
    action.amount = Some(Chips::from_i64(amount));
    action
}

pub fn post_bb(pseudo: &str, amount: i64) -> ActionRecord {
    let mut action = base_action(pseudo, ActionKind::PostBigBlind);
    action.amount = Some(Chips::from_i64(amount));
    action
}

/// Le joueur se couche.
pub fn fold(pseudo: &str) -> ActionRecord {
    base_action(pseudo, ActionKind::Fold)
}

/// Le joueur checke (aucune mise a suivre).
pub fn check(pseudo: &str) -> ActionRecord {
    base_action(pseudo, ActionKind::Check)
}

/// Le joueur complete/suit pour `amount` jetons.
pub fn call(pseudo: &str, amount: i64) -> ActionRecord {
    let mut action = base_action(pseudo, ActionKind::Call);
    action.amount = Some(Chips::from_i64(amount));
    action
}

/// Le joueur relance a `to` jetons au total sur la street.
pub fn raise(pseudo: &str, to: i64) -> ActionRecord {
    let mut action = base_action(pseudo, ActionKind::Raise);
    action.to_amount = Some(Chips::from_i64(to));
    action
}

/// Relance all-in a `to` jetons au total sur la street.
pub fn shove(pseudo: &str, to: i64) -> ActionRecord {
    let mut action = raise(pseudo, to);
    action.is_all_in = true;
    action
}

/// Construction concise d'une `HandRecord` pour les tests de stats (M4-2).
/// Les postes d'ante/blindes sont deduits automatiquement de `button`/
/// `sb`/`bb`/`ante` (voir le commentaire de module) : ne lister dans
/// `preflop` que les actions volontaires.
///
/// ```ignore
/// let hand = hand! {
///     seats: [(1, "Hero", 1000), (2, "Villain", 1000), (3, "P3", 1000)],
///     button: 1,
///     sb: 10, bb: 20, ante: 0,
///     preflop: [support::fold("P3"), support::raise("Hero", 40), support::fold("Villain")],
/// };
/// ```
#[macro_export]
macro_rules! hand {
    (
        seats: [ $( ($seat:expr, $pseudo:expr, $stack:expr) ),* $(,)? ],
        button: $button:expr,
        sb: $sb:expr, bb: $bb:expr, ante: $ante:expr,
        preflop: [ $( $action:expr ),* $(,)? ] $(,)?
    ) => {{
        let seats: Vec<gr_core::SeatInfo> = vec![
            $(
                gr_core::SeatInfo {
                    seat: $seat,
                    pseudo: $pseudo.to_string(),
                    starting_stack: gr_core::Chips::from_i64($stack),
                    bounty: None,
                    dealt_in: true,
                }
            ),*
        ];
        let preflop_actions: Vec<gr_core::ActionRecord> = vec![ $( $action ),* ];
        $crate::support::build_hand(seats, $button, $sb, $bb, $ante, preflop_actions)
    }};
}
