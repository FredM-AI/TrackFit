//! Flags preflop action/opportunite (PRD §10.2, M4-3). Un cas positif, un
//! negatif et un « pas d'opportunite » par stat (CLAUDE.md §7), construits
//! avec le DSL `hand!{}` (M4-2).
//!
//! Table 6-max de reference (bouton = siege 1) : P1=BTN, P2=SB, P3=BB,
//! P4=UTG, P5=HJ, P6=CO. Ordre d'action preflop : P4, P5, P6, P1, P2, P3.

mod support;

use gr_stats::{
    compute_3b, compute_4b, compute_ats, compute_f3b, compute_fsteal, compute_limp, compute_oshove,
    compute_pfr, compute_rfi, compute_rsteal, compute_vpip, preflop_line, StatFlag,
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

// --- VPIP ------------------------------------------------------------

#[test]
fn vpip_is_true_when_the_player_limps() {
    let hand = six_max!(preflop: [
        support::call("P4", 20), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::check("P3"),
    ]);
    assert_eq!(compute_vpip(&hand, "P4"), OPP_ACT);
}

#[test]
fn vpip_is_false_when_the_player_folds() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"),
    ]);
    assert_eq!(compute_vpip(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn vpip_has_no_opportunity_on_a_walk() {
    // Tout le monde se couche devant la BB : elle gagne sans avoir eu la parole.
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"),
    ]);
    assert_eq!(compute_vpip(&hand, "P3"), NO_OPP);
}

// --- PFR ---------------------------------------------------------------

#[test]
fn pfr_is_true_when_the_player_opens() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_pfr(&hand, "P4"), OPP_ACT);
}

#[test]
fn pfr_is_false_when_the_player_only_limps() {
    let hand = six_max!(preflop: [
        support::call("P4", 20), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::check("P3"),
    ]);
    assert_eq!(compute_pfr(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn pfr_has_no_opportunity_on_a_walk() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"),
    ]);
    assert_eq!(compute_pfr(&hand, "P3"), NO_OPP);
}

// --- RFI -----------------------------------------------------------------

#[test]
fn rfi_is_true_when_folded_to_and_the_player_raises() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_rfi(&hand, "P4"), OPP_ACT);
}

#[test]
fn rfi_is_false_when_folded_to_and_the_player_folds() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"),
    ]);
    assert_eq!(compute_rfi(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn rfi_has_no_opportunity_when_someone_already_entered() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_rfi(&hand, "P5"), NO_OPP);
}

// --- LIMP ----------------------------------------------------------------

#[test]
fn limp_is_true_when_folded_to_and_the_player_calls() {
    let hand = six_max!(preflop: [
        support::call("P4", 20), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::check("P3"),
    ]);
    assert_eq!(compute_limp(&hand, "P4"), OPP_ACT);
}

#[test]
fn limp_is_false_when_folded_to_and_the_player_raises() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_limp(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn limp_has_no_opportunity_when_someone_already_entered() {
    let hand = six_max!(preflop: [
        support::call("P4", 20), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::check("P3"),
    ]);
    assert_eq!(compute_limp(&hand, "P5"), NO_OPP);
}

// --- OSHOVE ----------------------------------------------------------------

#[test]
fn oshove_is_true_when_folded_to_and_the_player_shoves() {
    let hand = six_max!(preflop: [
        support::shove("P4", 1000), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_oshove(&hand, "P4"), OPP_ACT);
}

#[test]
fn oshove_is_false_for_a_non_all_in_open_raise() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_oshove(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn oshove_has_no_opportunity_when_someone_already_entered() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_oshove(&hand, "P5"), NO_OPP);
}

// --- 3B --------------------------------------------------------------------

#[test]
fn threebet_is_true_when_facing_exactly_one_raise_and_reraising() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::raise("P6", 180),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_3b(&hand, "P6"), OPP_ACT);
}

#[test]
fn threebet_is_false_when_facing_exactly_one_raise_and_calling() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_3b(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn threebet_has_no_opportunity_for_the_original_raiser() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::call("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_3b(&hand, "P4"), NO_OPP);
}

// --- F3B / 4B ----------------------------------------------------------

#[test]
fn f3b_is_true_when_the_opener_folds_to_a_3bet() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::raise("P6", 180),
        support::fold("P1"), support::fold("P2"), support::fold("P3"), support::fold("P4"),
    ]);
    assert_eq!(compute_f3b(&hand, "P4"), OPP_ACT);
    // Complementaire (PRD §10.2) : meme opportunite, action differente.
    assert_eq!(compute_4b(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn fourbet_is_true_when_the_opener_reraises_the_3bet() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::raise("P6", 180),
        support::fold("P1"), support::fold("P2"), support::fold("P3"), support::raise("P4", 400),
    ]);
    assert_eq!(compute_4b(&hand, "P4"), OPP_ACT);
    assert_eq!(compute_f3b(&hand, "P4"), OPP_NO_ACT);
}

#[test]
fn f3b_and_4b_have_no_opportunity_for_the_3better_itself() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::raise("P6", 180),
        support::fold("P1"), support::fold("P2"), support::fold("P3"), support::fold("P4"),
    ]);
    assert_eq!(compute_f3b(&hand, "P6"), NO_OPP);
    assert_eq!(compute_4b(&hand, "P6"), NO_OPP);
}

// --- ATS -------------------------------------------------------------------

#[test]
fn ats_is_true_when_folded_to_at_the_co_and_the_player_raises() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::raise("P6", 60),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_ats(&hand, "P6"), OPP_ACT);
}

#[test]
fn ats_is_false_when_folded_to_at_the_co_and_the_player_folds() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::raise("P1", 60), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_ats(&hand, "P6"), OPP_NO_ACT);
}

#[test]
fn ats_has_no_opportunity_from_utg() {
    // Folded to en UTG : premiere position, pas une position de steal (PRD §10.2).
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_ats(&hand, "P4"), NO_OPP);
}

// --- FSTEAL / RSTEAL ---------------------------------------------------

#[test]
fn fsteal_and_rsteal_share_the_same_opportunity_for_sb_and_bb() {
    // CO tente un steal ; BTN et SB s'inclinent (fold to steal) ; BB reraise (resteal).
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::raise("P6", 60),
        support::fold("P1"), support::fold("P2"), support::raise("P3", 200),
    ]);
    assert_eq!(compute_fsteal(&hand, "P2"), OPP_ACT); // SB : fold to steal.
    assert_eq!(compute_rsteal(&hand, "P3"), OPP_ACT); // BB : resteal.
                                                      // Complementaires : meme opportunite, action opposee non declenchee.
    assert_eq!(compute_rsteal(&hand, "P2"), OPP_NO_ACT);
    assert_eq!(compute_fsteal(&hand, "P3"), OPP_NO_ACT);
}

#[test]
fn fsteal_and_rsteal_have_no_opportunity_when_the_raiser_is_not_a_steal_position() {
    // UTG ouvre : pas une tentative de steal (PRD §10.2).
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(compute_fsteal(&hand, "P3"), NO_OPP);
    assert_eq!(compute_rsteal(&hand, "P3"), NO_OPP);
}

// --- Ligne preflop synthetique -------------------------------------------

#[test]
fn preflop_line_is_rfi_for_a_lone_open() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(preflop_line(&hand, "P4"), Some("RFI".to_string()));
}

#[test]
fn preflop_line_chains_events_for_an_opener_folding_to_a_3bet() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::fold("P5"), support::raise("P6", 180),
        support::fold("P1"), support::fold("P2"), support::fold("P3"), support::fold("P4"),
    ]);
    assert_eq!(preflop_line(&hand, "P4"), Some("RFI-F3B".to_string()));
}

#[test]
fn preflop_line_is_call_open_for_a_flat_call() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::call("P5", 60), support::fold("P6"),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(preflop_line(&hand, "P5"), Some("CALL-OPEN".to_string()));
}

#[test]
fn preflop_line_is_sqz_for_a_3bet_with_a_caller_behind() {
    let hand = six_max!(preflop: [
        support::raise("P4", 60), support::call("P5", 60), support::raise("P6", 220),
        support::fold("P1"), support::fold("P2"), support::fold("P3"),
    ]);
    assert_eq!(preflop_line(&hand, "P6"), Some("SQZ".to_string()));
}

#[test]
fn preflop_line_is_none_on_a_walk() {
    let hand = six_max!(preflop: [
        support::fold("P4"), support::fold("P5"), support::fold("P6"),
        support::fold("P1"), support::fold("P2"),
    ]);
    assert_eq!(preflop_line(&hand, "P3"), None);
}
