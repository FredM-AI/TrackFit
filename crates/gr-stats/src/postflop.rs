//! Flags postflop action/opportunite (PRD §10.2, M4-4) : CBF, CBT, FCBF,
//! WTSD, WSD (W$SD), WWSF, et les compteurs postflop pour AF/AFQ. Meme
//! convention que `preflop` (CLAUDE.md §6), `compute_<code>(hand, pseudo)
//! -> StatFlag`.
//!
//! Comment une main "va a l'abattage" est deduit du parser (§4.4/§4.7 du
//! format Winamax) : la section `*** SHOW DOWN ***` du texte brut n'existe
//! que si la main est reellement revelee (2+ joueurs encore en jeu a la
//! riviere) ; ses actions (`Shows`, `Collected`) sont les seules taguees
//! `Street::Showdown` — un pot remporte sans abattage (tout le monde se
//! couche) garde le `Street` de la derniere rue jouee. Donc : une main est
//! allee a l'abattage si et seulement si `hand.actions` contient une
//! action `Street::Showdown` (`mucks` n'a jamais ete observe dans le
//! corpus, cf. `docs/formats/winamax.md` §4.5 : tout joueur non couche
//! encore en jeu a l'abattage a une ligne `shows`).
//!
//! Tests dans `gr-stats/tests/postflop_flags.rs` (DSL `hand!{}` etendu au
//! postflop, M4-4).

use gr_core::{ActionKind, HandRecord, Street};

use crate::flag::StatFlag;

fn is_dealt_in(hand: &HandRecord, pseudo: &str) -> bool {
    hand.seats.iter().any(|s| s.pseudo == pseudo && s.dealt_in)
}

/// Le joueur a-t-il une relance/mise/action postflop en commun avec `kind`
/// dans le passe, sur une rue strictement avant `street` ? Utilise pour
/// determiner s'il s'est couche avant d'atteindre `street`.
fn folded_before(hand: &HandRecord, pseudo: &str, street: Street) -> bool {
    hand.actions
        .iter()
        .any(|a| a.pseudo == pseudo && a.kind == ActionKind::Fold && a.street < street)
}

/// La main a-t-elle atteint `street` (assez de cartes au board) ? Vrai par
/// definition pour `Preflop`. Pour `Showdown`, verifie qu'un abattage a
/// reellement eu lieu (voir le commentaire de module).
fn board_reached(hand: &HandRecord, street: Street) -> bool {
    match street {
        Street::Preflop => true,
        Street::Flop => hand.board.len() >= 3,
        Street::Turn => hand.board.len() >= 4,
        Street::River => hand.board.len() >= 5,
        Street::Showdown => hand_reached_showdown(hand),
    }
}

fn hand_reached_showdown(hand: &HandRecord) -> bool {
    hand.actions.iter().any(|a| a.street == Street::Showdown)
}

/// Le joueur a-t-il vu la rue `street` (distribue, main atteinte, pas
/// couche avant) ? Base commune a WTSD/WWSF (`street = Flop`) et a
/// CBF/CBT/FCBF.
#[must_use]
pub fn saw_street(hand: &HandRecord, pseudo: &str, street: Street) -> bool {
    is_dealt_in(hand, pseudo) && board_reached(hand, street) && !folded_before(hand, pseudo, street)
}

/// Le joueur est-il alle a l'abattage (main revelee, pas couche avant) ?
#[must_use]
pub fn went_to_showdown(hand: &HandRecord, pseudo: &str) -> bool {
    is_dealt_in(hand, pseudo)
        && hand_reached_showdown(hand)
        && !folded_before(hand, pseudo, Street::Showdown)
}

/// Le joueur a-t-il remporte tout ou partie d'au moins un pot de `hand` ?
#[must_use]
pub fn won_pot(hand: &HandRecord, pseudo: &str) -> bool {
    hand.pots
        .iter()
        .any(|pot| pot.winners.iter().any(|(winner, _)| winner == pseudo))
}

/// WTSD, Went To `ShowDown` (PRD §10.2) : opportunite = a vu le flop ; action
/// = arrive a l'abattage.
#[must_use]
pub fn compute_wtsd(hand: &HandRecord, pseudo: &str) -> StatFlag {
    if !saw_street(hand, pseudo, Street::Flop) {
        return StatFlag::NONE;
    }
    StatFlag::opportunity(went_to_showdown(hand, pseudo))
}

/// WSD, Won $ at `ShowDown` / W$SD (PRD §10.2) : opportunite = arrive a
/// l'abattage ; action = a gagne tout ou partie d'un pot.
#[must_use]
pub fn compute_wsd(hand: &HandRecord, pseudo: &str) -> StatFlag {
    if !went_to_showdown(hand, pseudo) {
        return StatFlag::NONE;
    }
    StatFlag::opportunity(won_pot(hand, pseudo))
}

/// WWSF, Won When Saw Flop (PRD §10.2, complementaire de WTSD) : meme
/// opportunite que WTSD ; action = a gagne tout ou partie d'un pot (avec ou
/// sans abattage).
#[must_use]
pub fn compute_wwsf(hand: &HandRecord, pseudo: &str) -> StatFlag {
    if !saw_street(hand, pseudo, Street::Flop) {
        return StatFlag::NONE;
    }
    StatFlag::opportunity(won_pot(hand, pseudo))
}

/// Une decision volontaire sur une rue postflop donnee, avec le nombre de
/// mises/relances deja intervenues sur **cette rue** avant elle (chaque
/// street redemarre a 0 : pas de report du preflop).
struct StreetStep<'a> {
    pseudo: &'a str,
    kind: ActionKind,
    bets_before: u32,
}

fn street_steps(hand: &HandRecord, street: Street) -> Vec<StreetStep<'_>> {
    let mut bets = 0u32;
    let mut steps = Vec::new();
    for action in &hand.actions {
        if action.street != street {
            continue;
        }
        if !matches!(
            action.kind,
            ActionKind::Bet
                | ActionKind::Raise
                | ActionKind::Call
                | ActionKind::Check
                | ActionKind::Fold
        ) {
            continue;
        }
        steps.push(StreetStep {
            pseudo: &action.pseudo,
            kind: action.kind,
            bets_before: bets,
        });
        if matches!(action.kind, ActionKind::Bet | ActionKind::Raise) {
            bets += 1;
        }
    }
    steps
}

/// Pseudo du dernier relanceur preflop (l'agresseur, potentiel c-betteur),
/// `None` si personne n'a relance preflop (pot limpe/checke).
fn last_preflop_raiser(hand: &HandRecord) -> Option<&str> {
    hand.actions
        .iter()
        .rev()
        .find(|a| a.street == Street::Preflop && a.kind == ActionKind::Raise)
        .map(|a| a.pseudo.as_str())
}

/// CBF, C-bet flop (PRD §10.2) : le joueur est le dernier relanceur
/// preflop, voit le flop, et sa premiere decision sur le flop n'a encore
/// vu aucune mise (premier a agir, ou checke jusqu'a lui) ; action = mise.
#[must_use]
pub fn compute_cbf(hand: &HandRecord, pseudo: &str) -> StatFlag {
    if last_preflop_raiser(hand) != Some(pseudo) {
        return StatFlag::NONE;
    }
    if !saw_street(hand, pseudo, Street::Flop) {
        return StatFlag::NONE;
    }
    let steps = street_steps(hand, Street::Flop);
    let Some(step) = steps.iter().find(|s| s.pseudo == pseudo) else {
        return StatFlag::NONE;
    };
    if step.bets_before != 0 {
        return StatFlag::NONE;
    }
    StatFlag::opportunity(step.kind == ActionKind::Bet)
}

/// CBT, C-bet turn (PRD §10.2, complementaire) : le joueur a c-bet le flop,
/// voit le turn, et sa premiere decision sur le turn n'a encore vu aucune
/// mise ; action = mise.
#[must_use]
pub fn compute_cbt(hand: &HandRecord, pseudo: &str) -> StatFlag {
    if !compute_cbf(hand, pseudo).act {
        return StatFlag::NONE;
    }
    if !saw_street(hand, pseudo, Street::Turn) {
        return StatFlag::NONE;
    }
    let steps = street_steps(hand, Street::Turn);
    let Some(step) = steps.iter().find(|s| s.pseudo == pseudo) else {
        return StatFlag::NONE;
    };
    if step.bets_before != 0 {
        return StatFlag::NONE;
    }
    StatFlag::opportunity(step.kind == ActionKind::Bet)
}

/// FCBF, Fold to c-bet flop (PRD §10.2) : le joueur fait face a la
/// **premiere** mise du flop, et cette mise a ete faite par le dernier
/// relanceur preflop (un c-bet, pas une "donk bet") ; action = fold.
#[must_use]
pub fn compute_fcbf(hand: &HandRecord, pseudo: &str) -> StatFlag {
    let Some(raiser) = last_preflop_raiser(hand) else {
        return StatFlag::NONE;
    };
    if raiser == pseudo {
        return StatFlag::NONE;
    }
    if !saw_street(hand, pseudo, Street::Flop) {
        return StatFlag::NONE;
    }
    let steps = street_steps(hand, Street::Flop);
    let Some(opening_bet) = steps
        .iter()
        .find(|s| s.bets_before == 0 && s.kind == ActionKind::Bet)
    else {
        return StatFlag::NONE;
    };
    if opening_bet.pseudo != raiser {
        return StatFlag::NONE;
    }
    let Some(step) = steps
        .iter()
        .find(|s| s.pseudo == pseudo && s.bets_before == 1)
    else {
        return StatFlag::NONE;
    };
    StatFlag::opportunity(step.kind == ActionKind::Fold)
}

/// Compteurs d'actions postflop d'un joueur (PRD §10.2, AF/AFQ) : simples
/// decomptes, pas un couple opportunite/action — le ratio (AF, AFQ) se
/// calcule en agregeant ces compteurs sur un echantillon de mains, pas sur
/// une seule main.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostflopActionCounts {
    pub bets: u32,
    pub raises: u32,
    pub calls: u32,
    pub folds: u32,
    pub checks: u32,
}

/// Decompte les actions postflop (flop/turn/river) du joueur dans `hand`.
#[must_use]
pub fn compute_postflop_counts(hand: &HandRecord, pseudo: &str) -> PostflopActionCounts {
    let mut counts = PostflopActionCounts::default();
    for action in &hand.actions {
        if action.pseudo != pseudo {
            continue;
        }
        if !matches!(action.street, Street::Flop | Street::Turn | Street::River) {
            continue;
        }
        match action.kind {
            ActionKind::Bet => counts.bets += 1,
            ActionKind::Raise => counts.raises += 1,
            ActionKind::Call => counts.calls += 1,
            ActionKind::Fold => counts.folds += 1,
            ActionKind::Check => counts.checks += 1,
            _ => {}
        }
    }
    counts
}
