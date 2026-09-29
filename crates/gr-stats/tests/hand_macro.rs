//! Valide le DSL `hand!{}` lui-meme (M4-2) : les postes d'ante/blindes
//! generes automatiquement, et les actions volontaires fournies telles
//! quelles.

mod support;

use gr_core::{ActionKind, Chips};

#[test]
fn generates_small_and_big_blind_posts_in_order() {
    let hand = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000), (3, "P3", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        preflop: [support::fold("P3"), support::fold("Villain")],
    };

    assert_eq!(hand.button_seat, 1);
    assert_eq!(hand.sb, Chips::from_i64(10));
    assert_eq!(hand.bb, Chips::from_i64(20));

    // Sans ante : posts SB puis BB, avant les actions volontaires.
    assert_eq!(hand.actions.len(), 4);
    assert_eq!(hand.actions[0].pseudo, "Villain"); // siege 2 = SB (BTN=1)
    assert_eq!(hand.actions[0].kind, ActionKind::PostSmallBlind);
    assert_eq!(hand.actions[0].amount, Some(Chips::from_i64(10)));
    assert_eq!(hand.actions[1].pseudo, "P3"); // siege 3 = BB
    assert_eq!(hand.actions[1].kind, ActionKind::PostBigBlind);
    assert_eq!(hand.actions[1].amount, Some(Chips::from_i64(20)));
    assert_eq!(hand.actions[2].pseudo, "P3");
    assert_eq!(hand.actions[2].kind, ActionKind::Fold);
    assert_eq!(hand.actions[3].pseudo, "Villain");
    assert_eq!(hand.actions[3].kind, ActionKind::Fold);
}

#[test]
fn generates_ante_posts_for_every_seat_before_the_blinds() {
    let hand = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 3,
        preflop: [],
    };

    // 2 antes + 2 blindes (heads-up : BTN poste la SB) = 4 postes generes.
    assert_eq!(hand.actions.len(), 4);
    assert!(hand.actions[..2]
        .iter()
        .all(|a| a.kind == ActionKind::PostAnte && a.amount == Some(Chips::from_i64(3))));
    assert_eq!(hand.actions[2].pseudo, "Hero"); // heads-up : le bouton poste la SB.
    assert_eq!(hand.actions[2].kind, ActionKind::PostSmallBlind);
    assert_eq!(hand.actions[3].pseudo, "Villain");
    assert_eq!(hand.actions[3].kind, ActionKind::PostBigBlind);
}

#[test]
fn hero_pseudo_is_detected_when_a_seat_is_named_hero() {
    let with_hero = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        preflop: [],
    };
    assert_eq!(with_hero.hero_pseudo, Some("Hero".to_string()));

    let without_hero = hand! {
        seats: [(1, "P1", 1000), (2, "P2", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        preflop: [],
    };
    assert_eq!(without_hero.hero_pseudo, None);
}

#[test]
fn voluntary_actions_carry_the_right_amounts() {
    let hand = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000), (3, "P3", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        preflop: [
            support::fold("P3"),
            support::raise("Hero", 60),
            support::call("Villain", 60),
        ],
    };

    let raise = &hand.actions[3];
    assert_eq!(raise.pseudo, "Hero");
    assert_eq!(raise.kind, ActionKind::Raise);
    assert_eq!(raise.to_amount, Some(Chips::from_i64(60)));

    let call = &hand.actions[4];
    assert_eq!(call.pseudo, "Villain");
    assert_eq!(call.kind, ActionKind::Call);
    assert_eq!(call.amount, Some(Chips::from_i64(60)));
}

#[test]
fn a_big_blind_can_check_when_no_one_raised() {
    let hand = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        // Heads-up : le bouton (Hero) complete la SB, la BB (Villain) checke.
        preflop: [support::call("Hero", 10), support::check("Villain")],
    };

    let check = hand.actions.last().unwrap();
    assert_eq!(check.pseudo, "Villain");
    assert_eq!(check.kind, ActionKind::Check);
    assert_eq!(check.amount, None);
}

#[test]
fn shove_marks_the_raise_as_all_in() {
    let hand = hand! {
        seats: [(1, "Hero", 1000), (2, "Villain", 1000)],
        button: 1,
        sb: 10, bb: 20, ante: 0,
        preflop: [support::shove("Hero", 1000)],
    };
    let shove_action = hand.actions.last().unwrap();
    assert_eq!(shove_action.kind, ActionKind::Raise);
    assert!(shove_action.is_all_in);
    assert_eq!(shove_action.to_amount, Some(Chips::from_i64(1000)));
}
