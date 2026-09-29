//! Flags postflop action/opportunite (PRD §10.2, M4-4). Un cas positif, un
//! negatif et un « pas d'opportunite » par stat (CLAUDE.md §7), construits
//! avec le DSL `hand!{}` + `with_postflop` (M4-2/M4-4).
//!
//! Table 6-max de reference (bouton = siege 1) : P1=BTN, P2=SB, P3=BB,
//! P4=UTG, P5=HJ, P6=CO — la meme que `preflop_flags.rs`.

#[path = "support/postflop.rs"]
mod postflop;
mod support;

use gr_core::Street;
use gr_stats::{
    compute_cbf, compute_cbt, compute_fcbf, compute_postflop_counts, compute_wsd, compute_wtsd,
    compute_wwsf, PostflopActionCounts, StatFlag,
};

const OPP_ACT: StatFlag = StatFlag {
    opp: true,
    act: true,
};
const OPP_NO_ACT: StatFlag = StatFlag {
    opp: true,
    act: false,
};
const NO_OPP: StatFlag = StatFlag {
    opp: false,
    act: false,
};

macro_rules! six_max {
    (preflop: [ $( $action:expr ),* $(,)? ]) => {
        hand! {
            seats: [
                (1, "P1", 1000), (2, "P2", 1000), (3, "P3", 1000),
                (4, "P4", 1000), (5, "P5", 1000), (6, "P6", 1000),
            ],
            button: 1,
            sb: 10, bb: 20, ante: 0,
            preflop: [ $( $action ),* ],
        }
    };
}

// --- CBF -------------------------------------------------------------------

#[test]
fn cbf_is_true_when_the_preflop_raiser_bets_first_on_the_flop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::fold_on("P6", Street::Flop),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_cbf(&hand, "P4"), OPP_ACT);
}

#[test]
fn cbf_is_false_when_the_preflop_raiser_checks_the_flop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
        ],
        &[],
    );
    assert_eq!(compute_cbf(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn cbf_has_no_opportunity_for_a_player_who_did_not_raise_preflop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::fold_on("P6", Street::Flop),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_cbf(&hand, "P6"), NO_OPP);
}

// --- CBT -------------------------------------------------------------------

#[test]
fn cbt_is_true_when_the_flop_c_better_bets_first_on_the_turn() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::call_on("P6", 100, Street::Flop),
            postflop::bet_on("P4", 150, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 420)],
    );
    assert_eq!(compute_cbt(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_cbf(&hand, "P4"), OPP_ACT); // toujours vrai aussi.
}

#[test]
fn cbt_is_false_when_the_flop_c_better_checks_the_turn() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::call_on("P6", 100, Street::Flop),
            postflop::check_on("P4", Street::Turn),
            postflop::check_on("P6", Street::Turn),
        ],
        &[],
    );
    assert_eq!(compute_cbt(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn cbt_has_no_opportunity_for_a_player_who_never_c_bet_the_flop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::call_on("P6", 100, Street::Flop),
            postflop::bet_on("P4", 150, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 420)],
    );
    assert_eq!(compute_cbt(&hand, "P6"), NO_OPP);
}

// --- FCBF --------------------------------------------------------------

#[test]
fn fcbf_is_true_when_facing_the_preflop_raisers_flop_bet_and_folding() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::fold_on("P6", Street::Flop),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_fcbf(&hand, "P6"), OPP_ACT);
}

#[test]
fn fcbf_is_false_when_facing_the_preflop_raisers_flop_bet_and_calling() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::bet_on("P4", 100, Street::Flop),
            postflop::call_on("P6", 100, Street::Flop),
        ],
        &[],
    );
    assert_eq!(compute_fcbf(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn fcbf_has_no_opportunity_when_the_flop_is_checked_through() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
        ],
        &[],
    );
    assert_eq!(compute_fcbf(&hand, "P6"), NO_OPP);
}

#[test]
fn fcbf_has_no_opportunity_facing_a_donk_bet_from_a_non_raiser() {
    // P6 (qui n'a pas ouvert preflop) mise en premier sur le flop : ce
    // n'est pas un c-bet, donc pas d'opportunite FCBF pour P4.
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts"],
        vec![
            postflop::bet_on("P6", 100, Street::Flop),
            postflop::fold_on("P4", Street::Flop),
        ],
        &[("P6", 220)],
    );
    assert_eq!(compute_fcbf(&hand, "P4"), NO_OPP);
}

// --- WTSD --------------------------------------------------------------

#[test]
fn wtsd_is_true_when_the_player_sees_the_flop_and_reaches_showdown() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::check_on("P4", Street::Turn),
            postflop::check_on("P6", Street::Turn),
            postflop::check_on("P4", Street::River),
            postflop::check_on("P6", Street::River),
            postflop::shows("P4"),
            postflop::shows("P6"),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wtsd(&hand, "P4"), OPP_ACT);
}

#[test]
fn wtsd_is_false_when_the_player_sees_the_flop_but_folds_before_showdown() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wtsd(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn wtsd_has_no_opportunity_for_a_player_who_folded_preflop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wtsd(&hand, "P5"), NO_OPP);
}

// --- WSD -----------------------------------------------------------------

#[test]
fn wsd_is_true_when_reaching_showdown_and_winning() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::check_on("P4", Street::Turn),
            postflop::check_on("P6", Street::Turn),
            postflop::check_on("P4", Street::River),
            postflop::check_on("P6", Street::River),
            postflop::shows("P4"),
            postflop::shows("P6"),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wsd(&hand, "P4"), OPP_ACT);
}

#[test]
fn wsd_is_false_when_reaching_showdown_and_losing() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::check_on("P4", Street::Turn),
            postflop::check_on("P6", Street::Turn),
            postflop::check_on("P4", Street::River),
            postflop::check_on("P6", Street::River),
            postflop::shows("P4"),
            postflop::shows("P6"),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wsd(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn wsd_has_no_opportunity_when_the_hand_never_reaches_showdown() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wsd(&hand, "P4"), NO_OPP);
    assert_eq!(compute_wsd(&hand, "P6"), NO_OPP);
}

// --- WWSF ----------------------------------------------------------------

#[test]
fn wwsf_is_true_when_winning_without_even_reaching_showdown() {
    // WWSF ne demande pas l'abattage (a la difference de WSD) : gagner en
    // faisant coucher l'adversaire sur le turn compte aussi.
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wwsf(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_wsd(&hand, "P4"), NO_OPP); // pas d'abattage : WSD ne s'applique pas.
}

#[test]
fn wwsf_is_false_for_the_player_who_saw_the_flop_and_lost() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wwsf(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn wwsf_has_no_opportunity_for_a_player_who_folded_preflop() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P6", Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::fold_on("P6", Street::Turn),
        ],
        &[("P4", 220)],
    );
    assert_eq!(compute_wwsf(&hand, "P5"), NO_OPP);
}

// --- Compteurs postflop (AF/AFQ) ----------------------------------------

#[test]
fn postflop_counts_tally_each_kind_of_action_per_player_across_streets() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![
            postflop::bet_on("P4", 50, Street::Flop),
            postflop::call_on("P6", 50, Street::Flop),
            postflop::bet_on("P4", 100, Street::Turn),
            postflop::raise_on("P6", 300, Street::Turn),
            postflop::call_on("P4", 200, Street::Turn),
            postflop::check_on("P4", Street::River),
            postflop::bet_on("P6", 400, Street::River),
            postflop::fold_on("P4", Street::River),
        ],
        &[("P6", 1500)],
    );

    assert_eq!(
        compute_postflop_counts(&hand, "P4"),
        PostflopActionCounts {
            bets: 2,
            raises: 0,
            calls: 1,
            folds: 1,
            checks: 1,
        }
    );
    assert_eq!(
        compute_postflop_counts(&hand, "P6"),
        PostflopActionCounts {
            bets: 1,
            raises: 1,
            calls: 1,
            folds: 0,
            checks: 0,
        }
    );
    // La relance preflop de P4 (raise) n'est pas comptee : les compteurs
    // sont strictement postflop (bets: 2, pas 3).
}

// --- Cas limites --------------------------------------------------------

#[test]
fn wtsd_and_wwsf_apply_even_without_a_preflop_raiser() {
    // Pot limpe (aucune relance preflop) : CBF/CBT/FCBF ne s'appliquent
    // jamais ici (pas de "dernier relanceur"), mais WTSD/WWSF n'en ont pas
    // besoin — seul "a vu le flop" compte.
    let hand = six_max!(preflop: [
        support::call("P4", 20), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::check("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![
            postflop::check_on("P4", Street::Flop),
            postflop::check_on("P3", Street::Flop),
            postflop::check_on("P4", Street::Turn),
            postflop::check_on("P3", Street::Turn),
            postflop::check_on("P4", Street::River),
            postflop::check_on("P3", Street::River),
            postflop::shows("P4"),
            postflop::shows("P3"),
        ],
        &[("P4", 60)],
    );
    assert_eq!(compute_wtsd(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_wwsf(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_cbf(&hand, "P4"), NO_OPP);
}

#[test]
fn wtsd_applies_to_an_all_in_runout_with_no_postflop_actions() {
    // Les deux joueurs sont all-in preflop : les rues restantes sont
    // distribuees sans aucune action (docs/formats/winamax.md §4.4), donc
    // "a vu le flop" doit se deduire du seul board, pas d'une action
    // enregistree sur cette rue.
    let hand = six_max!(preflop: [
        support::shove("P4", 1000), support::fold("P5"), support::call("P6", 1000),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    let hand = postflop::with_postflop(
        hand,
        &["6h", "5h", "Ts", "7h", "8c"],
        vec![postflop::shows("P4"), postflop::shows("P6")],
        &[("P4", 2000)],
    );
    // Les deux joueurs sont encore en jeu jusqu'a l'abattage (all-in) :
    // WTSD s'applique aux deux, pas seulement au gagnant.
    assert_eq!(compute_wtsd(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_wtsd(&hand, "P6"), OPP_ACT);
}
