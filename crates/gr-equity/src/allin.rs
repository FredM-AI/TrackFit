//! Detection d'un evenement all-in et calcul de l'EV associee (PRD §10.6,
//! M5-4). Au plus un evenement par main : une fois qu'aucune decision
//! supplementaire n'est possible, la main se conclut automatiquement (les
//! rues restantes sont distribuees sans action, `docs/formats/winamax.md`
//! §4.4) — il ne peut donc pas y avoir un second point de decision apres.
//!
//! **Simplification volontaire, validee avec Frederic (29/09) :** l'equite
//! se calcule une seule fois pour l'ensemble des joueurs impliques et du
//! pot total (`Σ hand.pots[].amount`), pas separement par pot (main pot /
//! side pots). PRD §10.6 demande "au moins un joueur all-in... par pot
//! (side pots compris)" au sens ou l'EV doit couvrir tout ce qui est
//! distribue, mais pas necessairement un calcul distinct par pot : pour un
//! affichage replayer (pas un solveur), un all-in multiway a tapis inegaux
//! (vrais side pots) peut repartir l'EV legerement differemment de ce que
//! donnerait un calcul par pot strict (l'equite "pot principal" d'un
//! joueur a tapis court peut differer de son equite "contre tout le
//! monde"). Avantage en retour : la conservation Σ EV = Σ pots est
//! garantie par construction (les equites d'un seul calcul somment a 1).
//!
//! **Synchrone, pas de file de jobs** (decision documentee dans le
//! BACKLOG) : les evenements all-in sont rares (la plupart des mains n'en
//! ont aucun), et `calculate_equity` est deja rapide (M5-3). R-COMP-1 est
//! satisfaite par construction : le pipeline d'import ne traite que des
//! mains completes (Winamax n'ecrit une main sur disque qu'une fois
//! terminee), jamais une main "en cours".

use std::collections::HashSet;

use gr_core::{ActionKind, Card, HandRecord, SeatInfo, Street};

use crate::{calculate_equity, EquityMethod};

/// Un evenement all-in detecte dans une main (PRD §10.6) : les joueurs
/// impliques (encore en jeu, cartes connues) et, pour chacun, l'ecart en
/// jetons entre ce qu'il a reellement gagne et son EV (positif = a
/// gagne plus que son equite ne le laissait attendre, "chanceux" ; negatif
/// = l'inverse).
#[derive(Debug, Clone, PartialEq)]
pub struct AllInEvent {
    /// `(pseudo, allin_ev_diff_chips)`, un par joueur implique, dans
    /// l'ordre des sieges.
    pub player_diffs: Vec<(String, f64)>,
    pub method: EquityMethod,
}

/// Detail complet (pas seulement l'ecart) d'un evenement all-in, pour un
/// affichage replayer (M7-3) : equite et EV en jetons de chaque joueur
/// implique, plus la rue a laquelle la decision a eu lieu (pour savoir a
/// quel pas de rejeu l'afficher).
#[derive(Debug, Clone, PartialEq)]
pub struct AllInPlayerDetail {
    pub pseudo: String,
    /// Part du pot gagnee en moyenne (`0.0..=1.0`), ties compris.
    pub equity: f64,
    pub ev_chips: f64,
    pub actual_won_chips: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AllInDetails {
    pub street: Street,
    pub method: EquityMethod,
    /// Dans l'ordre des sieges, comme [`AllInEvent::player_diffs`].
    pub players: Vec<AllInPlayerDetail>,
}

/// Detecte un evenement all-in dans `hand` et calcule l'EV de chaque
/// joueur implique (PRD §10.6). `None` si la main n'en a pas (showdown
/// normal, main terminee par un fold avant la river, ou main sans
/// abattage), ou si les cartes d'un joueur implique restent inconnues
/// (jamais observe dans le corpus, mais pas de calcul possible sans elles).
#[must_use]
pub fn detect_all_in_event(hand: &HandRecord) -> Option<AllInEvent> {
    let details = compute_all_in_details(hand)?;
    Some(AllInEvent {
        player_diffs: details
            .players
            .iter()
            .map(|p| {
                #[allow(clippy::cast_precision_loss)]
                let actual_won = p.actual_won_chips as f64;
                (p.pseudo.clone(), actual_won - p.ev_chips)
            })
            .collect(),
        method: details.method,
    })
}

/// Comme [`detect_all_in_event`], mais renvoie le detail complet (equite +
/// EV par joueur, pas seulement l'ecart) — utilise par le replayer (M7-3)
/// pour l'affichage "a chaque all-in, equite de chaque joueur et EV" (PRD
/// §13.7). Meme detection, memes limites (au plus un evenement par main,
/// simplification "un seul calcul d'equite pour tout le pot" documentee en
/// tete de module).
#[must_use]
pub fn compute_all_in_details(hand: &HandRecord) -> Option<AllInDetails> {
    let street = last_active_street(hand)?;
    let known_board_len = known_board_len(street)?;
    // Main terminee par un fold plutot que jusqu'au bout : le board ne
    // serait pas entierement distribue.
    if hand.board.len() < 5 {
        return None;
    }

    let folded: HashSet<&str> = hand
        .actions
        .iter()
        .filter(|a| a.kind == ActionKind::Fold)
        .map(|a| a.pseudo.as_str())
        .collect();

    let mut involved_seats: Vec<&SeatInfo> = hand
        .seats
        .iter()
        .filter(|s| s.dealt_in && !folded.contains(s.pseudo.as_str()))
        .collect();
    involved_seats.sort_by_key(|s| s.seat);

    // Pas de contestation : ne devrait pas arriver si le board est
    // entierement distribue (defensif, R-NOPANIC : abandonner plutot que
    // supposer).
    if involved_seats.len() < 2 {
        return None;
    }

    let mut hole_cards = Vec::with_capacity(involved_seats.len());
    for seat in &involved_seats {
        hole_cards.push(known_hole_cards(hand, &seat.pseudo)?);
    }

    let known_board = &hand.board[..known_board_len];
    let calculation = calculate_equity(&hole_cards, known_board).ok()?;

    #[allow(clippy::cast_precision_loss)]
    let pool_chips = hand.pots.iter().map(|p| p.amount.amount()).sum::<i64>() as f64;

    let players = involved_seats
        .iter()
        .zip(&calculation.players)
        .map(|(seat, equity)| {
            let ev_chips = equity.equity * pool_chips;
            let actual_won_chips = actual_chips_won(hand, &seat.pseudo);
            AllInPlayerDetail {
                pseudo: seat.pseudo.clone(),
                equity: equity.equity,
                ev_chips,
                actual_won_chips,
            }
        })
        .collect();

    Some(AllInDetails {
        street,
        method: calculation.method,
        players,
    })
}

/// Rue jusqu'a laquelle une decision volontaire a ete enregistree en
/// dernier (postes d'ante/blindes exclus, et abattage exclu : ce n'est pas
/// une rue de jeu).
fn last_active_street(hand: &HandRecord) -> Option<Street> {
    hand.actions
        .iter()
        .filter(|a| {
            matches!(
                a.street,
                Street::Preflop | Street::Flop | Street::Turn | Street::River
            )
        })
        .filter(|a| {
            !matches!(
                a.kind,
                ActionKind::PostAnte | ActionKind::PostSmallBlind | ActionKind::PostBigBlind
            )
        })
        .map(|a| a.street)
        .max()
}

/// Nombre de cartes de board connues au moment ou `street` a ete la
/// derniere rue avec une decision (PRD §10.6, "avant la river") : `None`
/// pour `River`/`Showdown`, une decision sur la riviere (ou apres) ne
/// laisse plus rien a tirer, ce n'est pas un evenement all-in au sens du
/// CA.
fn known_board_len(street: Street) -> Option<usize> {
    match street {
        Street::Preflop => Some(0),
        Street::Flop => Some(3),
        Street::Turn => Some(4),
        Street::River | Street::Showdown => None,
    }
}

fn known_hole_cards(hand: &HandRecord, pseudo: &str) -> Option<[Card; 2]> {
    if hand.hero_pseudo.as_deref() == Some(pseudo) {
        return hand.hero_cards.map(|(a, b)| [a, b]);
    }
    hand.actions.iter().find_map(|a| {
        if a.pseudo != pseudo || a.kind != ActionKind::Shows {
            return None;
        }
        match a.shown_cards.as_deref() {
            Some([a, b]) => Some([*a, *b]),
            _ => None,
        }
    })
}

fn actual_chips_won(hand: &HandRecord, pseudo: &str) -> i64 {
    hand.pots
        .iter()
        .flat_map(|pot| &pot.winners)
        .filter(|(winner, _)| winner == pseudo)
        .map(|(_, amount)| amount.amount())
        .sum()
}
