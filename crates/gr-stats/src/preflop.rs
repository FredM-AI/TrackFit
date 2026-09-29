//! Flags preflop action/opportunite (PRD §10.2, M4-3) : VPIP, PFR, RFI,
//! LIMP, OSHOVE, 3B, F3B, 4B, ATS, FSTEAL, RSTEAL, et la ligne preflop
//! synthetique. Une fonction pure par stat (CLAUDE.md §6),
//! `compute_<code>(hand, pseudo) -> StatFlag`.
//!
//! Convention partagee par toutes les stats de ce module : le "preflop"
//! exclut les postes d'ante/blindes (PRD §10.2, « le préflop exclut les
//! blindes et antes postées ») ; seules les actions volontaires (fold,
//! check, call, raise) comptent. Un joueur sans aucune action volontaire
//! preflop enregistree n'a jamais eu « la parole » (main gagnee par walk,
//! ou terminee avant que son tour n'arrive) : il n'a d'opportunite pour
//! aucune stat de ce module.
//!
//! Tests dans `gr-stats/tests/preflop_flags.rs` (DSL `hand!{}`, M4-2),
//! comme convenu par CLAUDE.md §7 : au moins un cas positif, un negatif et
//! un « pas d'opportunite » par stat.

use std::collections::HashSet;

use gr_core::{ActionKind, HandRecord, Position, Street};

use crate::flag::StatFlag;
use crate::position::assign_positions;

/// Positions depuis lesquelles une relance « first in » est une tentative
/// de steal (PRD §10.2, ATS/FSTEAL/RSTEAL).
const STEAL_POSITIONS: [Position; 3] = [Position::Co, Position::Btn, Position::Sb];

/// Une decision volontaire preflop, avec l'etat de la main juste avant
/// qu'elle ne soit prise.
struct PreflopStep<'a> {
    pseudo: &'a str,
    kind: ActionKind,
    is_all_in: bool,
    /// Nombre de relances preflop deja intervenues avant cette decision.
    raises_before: u32,
    /// Nombre de joueurs distincts ayant deja mis volontairement des jetons
    /// (call ou raise) avant cette decision.
    entrants_before: u32,
}

/// Reconstruit la chronologie des decisions volontaires preflop de `hand`
/// (postes d'ante/blindes exclus, PRD §10.2).
fn preflop_steps(hand: &HandRecord) -> Vec<PreflopStep<'_>> {
    let mut raises = 0u32;
    let mut entrants: HashSet<&str> = HashSet::new();
    let mut steps = Vec::new();

    for action in &hand.actions {
        if action.street != Street::Preflop {
            continue;
        }
        let is_post = matches!(
            action.kind,
            ActionKind::PostAnte | ActionKind::PostSmallBlind | ActionKind::PostBigBlind
        );
        if is_post {
            continue;
        }

        steps.push(PreflopStep {
            pseudo: &action.pseudo,
            kind: action.kind,
            is_all_in: action.is_all_in,
            raises_before: raises,
            entrants_before: u32::try_from(entrants.len()).unwrap_or(u32::MAX),
        });

        match action.kind {
            ActionKind::Raise => {
                raises += 1;
                entrants.insert(&action.pseudo);
            }
            ActionKind::Call => {
                entrants.insert(&action.pseudo);
            }
            _ => {}
        }
    }

    steps
}

fn first_step<'a, 's>(steps: &'s [PreflopStep<'a>], pseudo: &str) -> Option<&'s PreflopStep<'a>> {
    steps.iter().find(|s| s.pseudo == pseudo)
}

fn position_of(hand: &HandRecord, pseudo: &str) -> Option<Position> {
    let seat = hand.seats.iter().find(|s| s.pseudo == pseudo)?.seat;
    assign_positions(hand).get(&seat).copied()
}

/// Le joueur a-t-il eu « la parole » preflop (opportunite de base commune
/// a toutes les stats de ce module) ? `false` si aucune decision
/// volontaire n'est enregistree pour lui (walk, ou main terminee avant son
/// tour).
fn had_the_floor(steps: &[PreflopStep<'_>], pseudo: &str) -> bool {
    first_step(steps, pseudo).is_some()
}

/// « Folded to » le joueur (PRD §10.2, RFI/LIMP/OSHOVE/ATS) : sa premiere
/// decision volontaire preflop intervient avant toute relance ou tout call
/// (personne n'est encore entre dans le pot).
fn is_first_in(steps: &[PreflopStep<'_>], pseudo: &str) -> bool {
    first_step(steps, pseudo).is_some_and(|s| s.raises_before == 0 && s.entrants_before == 0)
}

/// VPIP (PRD §10.2) : toute main distribuee ou le joueur a eu la parole
/// preflop (walks exclus) ; action = call ou relance (le check de la BB ne
/// compte pas).
#[must_use]
pub fn compute_vpip(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !had_the_floor(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let act = steps
        .iter()
        .any(|s| s.pseudo == pseudo && matches!(s.kind, ActionKind::Call | ActionKind::Raise));
    StatFlag::opportunity(act)
}

/// PFR (PRD §10.2) : meme opportunite que VPIP ; action = au moins une
/// relance preflop (open-shove compris).
#[must_use]
pub fn compute_pfr(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !had_the_floor(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let act = steps
        .iter()
        .any(|s| s.pseudo == pseudo && s.kind == ActionKind::Raise);
    StatFlag::opportunity(act)
}

/// RFI, par position (PRD §10.2) : « folded to » le joueur preflop ;
/// action = relance (y compris all-in).
#[must_use]
pub fn compute_rfi(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !is_first_in(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let act = first_step(&steps, pseudo).is_some_and(|s| s.kind == ActionKind::Raise);
    StatFlag::opportunity(act)
}

/// LIMP (PRD §10.2) : meme opportunite que RFI ; action = complete/call de
/// la BB.
#[must_use]
pub fn compute_limp(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !is_first_in(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let act = first_step(&steps, pseudo).is_some_and(|s| s.kind == ActionKind::Call);
    StatFlag::opportunity(act)
}

/// OSHOVE (PRD §10.2) : meme opportunite que RFI ; action = relance
/// all-in.
#[must_use]
pub fn compute_oshove(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !is_first_in(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let act =
        first_step(&steps, pseudo).is_some_and(|s| s.kind == ActionKind::Raise && s.is_all_in);
    StatFlag::opportunity(act)
}

/// 3B (PRD §10.2) : le joueur fait face a exactement une relance preflop
/// (avec ou sans callers) et a encore la parole ; action = relance.
#[must_use]
pub fn compute_3b(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    let Some(step) = steps
        .iter()
        .find(|s| s.pseudo == pseudo && s.raises_before == 1)
    else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Raise)
}

/// Le joueur a-t-il fait la premiere relance preflop (l'« open ») ?
fn is_the_opener(steps: &[PreflopStep<'_>], pseudo: &str) -> bool {
    first_step(steps, pseudo).is_some_and(|s| s.raises_before == 0 && s.kind == ActionKind::Raise)
}

/// F3B et 4B partagent la meme opportunite (PRD §10.2, « 4-bet
/// (complementaire) ») : le joueur a fait le premier raise preflop
/// (l'open) et fait face a une 3-bet (`raises_before == 2` a sa decision
/// suivante).
fn opener_facing_3bet<'a, 's>(
    steps: &'s [PreflopStep<'a>],
    pseudo: &str,
) -> Option<&'s PreflopStep<'a>> {
    if !is_the_opener(steps, pseudo) {
        return None;
    }
    steps
        .iter()
        .find(|s| s.pseudo == pseudo && s.raises_before == 2)
}

/// F3B (PRD §10.2) : action = fold.
#[must_use]
pub fn compute_f3b(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    let Some(step) = opener_facing_3bet(&steps, pseudo) else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Fold)
}

/// 4B (PRD §10.2, complementaire de F3B) : action = relance.
#[must_use]
pub fn compute_4b(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    let Some(step) = opener_facing_3bet(&steps, pseudo) else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Raise)
}

/// ATS, Attempt To Steal (PRD §10.2) : « folded to » le joueur au CO, BTN
/// ou SB ; action = relance.
#[must_use]
pub fn compute_ats(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    if !is_first_in(&steps, pseudo) {
        return StatFlag::NONE;
    }
    let Some(position) = position_of(hand, pseudo) else {
        return StatFlag::NONE;
    };
    if !STEAL_POSITIONS.contains(&position) {
        return StatFlag::NONE;
    }
    let act = first_step(&steps, pseudo).is_some_and(|s| s.kind == ActionKind::Raise);
    StatFlag::opportunity(act)
}

/// FSTEAL et RSTEAL partagent la meme opportunite (PRD §10.2, « Meme
/// opportunite que FSTEAL ») : le joueur est en SB ou BB, fait face a
/// exactement une relance preflop, et seul le relanceur (une position de
/// steal : CO/BTN/SB) est entre dans le pot (aucun cold caller).
fn steal_defense_step<'a, 's>(
    hand: &HandRecord,
    steps: &'s [PreflopStep<'a>],
    pseudo: &str,
) -> Option<&'s PreflopStep<'a>> {
    let position = position_of(hand, pseudo)?;
    if !matches!(position, Position::Sb | Position::Bb) {
        return None;
    }
    let step = steps
        .iter()
        .find(|s| s.pseudo == pseudo && s.raises_before == 1 && s.entrants_before == 1)?;

    let opener = steps
        .iter()
        .find(|s| s.raises_before == 0 && s.kind == ActionKind::Raise)?;
    let opener_position = position_of(hand, opener.pseudo)?;
    if !STEAL_POSITIONS.contains(&opener_position) {
        return None;
    }
    Some(step)
}

/// FSTEAL (PRD §10.2) : action = fold.
#[must_use]
pub fn compute_fsteal(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    let Some(step) = steal_defense_step(hand, &steps, pseudo) else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Fold)
}

/// RSTEAL (PRD §10.2) : action = relance (3-bet vs steal).
#[must_use]
pub fn compute_rsteal(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let steps = preflop_steps(hand);
    let Some(step) = steal_defense_step(hand, &steps, pseudo) else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Raise)
}

/// Etiquette d'une decision volontaire preflop, pour la ligne synthetique
/// (`preflop_line`). Contextualisee par l'etat de la main au moment de la
/// decision (`raises_before`/`entrants_before`), pas seulement par le type
/// d'action — ex. un call face a une relance est `CALL-OPEN`, pas un LIMP.
fn step_tag(step: &PreflopStep<'_>) -> &'static str {
    match step.kind {
        ActionKind::Raise if step.raises_before == 0 && step.is_all_in => "OSHOVE",
        ActionKind::Raise if step.raises_before == 0 => "RFI",
        ActionKind::Raise if step.raises_before == 1 && step.entrants_before >= 2 => "SQZ",
        ActionKind::Raise if step.raises_before == 1 => "3B",
        ActionKind::Raise if step.raises_before == 2 => "4B",
        ActionKind::Raise => "5B+",
        ActionKind::Call if step.raises_before == 0 => "LIMP",
        ActionKind::Call if step.raises_before == 1 => "CALL-OPEN",
        ActionKind::Call if step.raises_before == 2 => "CALL-3B",
        ActionKind::Call => "CALL-4B+",
        ActionKind::Check => "CHECK",
        ActionKind::Fold if step.raises_before == 2 => "F3B",
        ActionKind::Fold if step.raises_before == 3 => "F4B",
        ActionKind::Fold => "FOLD",
        ActionKind::PostAnte
        | ActionKind::PostSmallBlind
        | ActionKind::PostBigBlind
        | ActionKind::Bet
        | ActionKind::Shows
        | ActionKind::Collected => "?",
    }
}

/// Ligne preflop synthetique du joueur (annexe schema PRD, ex. `'RFI'`,
/// `'RFI-F3B'`, `'CALL-OPEN'`, `'SQZ'`) : ses etiquettes de decision
/// volontaires preflop, dans l'ordre, jointes par `-`. `None` s'il n'a
/// aucune decision volontaire preflop enregistree (walk, ou main terminee
/// avant son tour).
#[must_use]
pub fn preflop_line(hand: &HandRecord, pseudo: &str) -> Option<String> {
    let steps = preflop_steps(hand);
    let tags: Vec<&str> = steps
        .iter()
        .filter(|s| s.pseudo == pseudo)
        .map(step_tag)
        .collect();
    if tags.is_empty() {
        None
    } else {
        Some(tags.join("-"))
    }
}
