#![warn(clippy::pedantic)]

//! Evaluateur de mains et equite (PRD §10.6, M5-3/M5-4). Fonctions pures
//! (pas d'I/O), comme `gr-core`/`gr-stats`.
//!
//! **R-COMP-1 (rappel) :** ce crate ne sait rien d'une "main en cours" ou
//! "terminee" — c'est a l'appelant (`gr-store`, a l'import) de ne
//! l'invoquer que sur des mains deja completement importees. Satisfait par
//! construction : Winamax n'ecrit une main sur disque qu'une fois
//! terminee, jamais une main "en cours". Aucun calcul d'equite ne doit
//! jamais accompagner une decision en temps reel.
//!
//! L'evaluateur de mains lui-meme (perfect-hash 5/7 cartes) vient du crate
//! `rs_poker` (`default-features = false` : coeur + Hold'em seulement, pas
//! le sous-systeme `arena`/CFR qui tirerait tokio — decision documentee
//! dans `docs/BACKLOG.md`, M5-3). `rs_poker` reste un detail d'implementation
//! interne : l'API publique de ce crate n'expose que des `gr_core::Card`.
//!
//! Deux methodes (PRD §10.6), choisies automatiquement selon le nombre de
//! joueurs :
//! - **2 ou 3 joueurs** : enumeration exacte de tous les tirages de board
//!   restants ([`exact`]), multi-threadee pour tenir la cible de perf du
//!   CA (HU preflop < 60 ms sur le Celeron cible).
//! - **4 joueurs ou plus** : Monte Carlo a 200 000 tirages, graine fixe
//!   ([`monte_carlo`]) — reproductible (memes cartes -> meme resultat).
//!
//! [`detect_all_in_event`] (M5-4) applique tout ca a une main entiere :
//! detecte le (au plus un) evenement all-in et calcule l'EV en jetons de
//! chaque joueur implique, synchrone (voir le commentaire de module
//! d'[`allin`]).

mod allin;
mod combinations;
mod convert;
mod deck;
mod error;
mod exact;
mod monte_carlo;

pub use allin::{detect_all_in_event, AllInEvent};
pub use error::EquityError;

use convert::to_rs_card;

/// Comment l'equite a ete calculee (PRD §10.6) — a afficher a l'utilisateur
/// ("estimation" pour Monte Carlo, PRD §9.1 dans le meme esprit que la
/// mention "estimation" de la methode B des phases de tournoi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquityMethod {
    /// Enumeration exacte de tous les tirages de board restants.
    Exact,
    /// Estimation par tirages aleatoires, graine fixe (PRD §10.6).
    MonteCarlo { iterations: u32 },
}

/// Resultat du calcul d'equite pour un joueur (PRD §10.6) : part du pot
/// gagnee en moyenne sur tous les tirages consideres, ties compris (une
/// egalite a N mains rapporte `1/N` a chacune). `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerEquity {
    pub equity: f64,
}

/// Resultat complet d'un calcul d'equite (PRD §10.6).
#[derive(Debug, Clone, PartialEq)]
pub struct EquityCalculation {
    /// Une entree par joueur, dans l'ordre de `hole_cards` fourni a
    /// [`calculate_equity`].
    pub players: Vec<PlayerEquity>,
    pub method: EquityMethod,
}

/// Calcule l'equite de chaque joueur (PRD §10.6) a partir de ses deux
/// cartes fermees et du board deja connu (0, 3, 4 ou 5 cartes selon la
/// rue). Choisit automatiquement la methode : enumeration exacte pour 2 ou
/// 3 joueurs, Monte Carlo (200 000 tirages, graine fixe) au-dela.
///
/// # Errors
/// Renvoie une [`EquityError`] si moins de 2 ou plus de 9 mains sont
/// fournies, si le board depasse 5 cartes, ou si une carte apparait plus
/// d'une fois (main ou board).
pub fn calculate_equity(
    hole_cards: &[[gr_core::Card; 2]],
    board: &[gr_core::Card],
) -> Result<EquityCalculation, EquityError> {
    validate(hole_cards, board)?;

    let rs_hole: Vec<[rs_poker::core::Card; 2]> = hole_cards
        .iter()
        .map(|[a, b]| [to_rs_card(*a), to_rs_card(*b)])
        .collect();
    let rs_board: Vec<rs_poker::core::Card> = board.iter().copied().map(to_rs_card).collect();

    // PRD §10.6 : enumeration exacte pour 2 ou 3 joueurs, Monte Carlo au-dela.
    let (equities, method) = if hole_cards.len() <= 3 {
        (
            exact::exact_equity(&rs_hole, &rs_board),
            EquityMethod::Exact,
        )
    } else {
        (
            monte_carlo::monte_carlo_equity(&rs_hole, &rs_board),
            EquityMethod::MonteCarlo {
                iterations: monte_carlo::ITERATIONS,
            },
        )
    };

    Ok(EquityCalculation {
        players: equities
            .into_iter()
            .map(|equity| PlayerEquity { equity })
            .collect(),
        method,
    })
}

fn validate(hole_cards: &[[gr_core::Card; 2]], board: &[gr_core::Card]) -> Result<(), EquityError> {
    if hole_cards.len() < 2 {
        return Err(EquityError::TooFewPlayers);
    }
    if hole_cards.len() > 9 {
        return Err(EquityError::TooManyPlayers);
    }
    if board.len() > 5 {
        return Err(EquityError::BoardTooLong(board.len()));
    }

    let mut seen = std::collections::HashSet::new();
    for card in hole_cards.iter().flatten().chain(board) {
        if !seen.insert(*card) {
            return Err(EquityError::DuplicateCard(*card));
        }
    }
    Ok(())
}
