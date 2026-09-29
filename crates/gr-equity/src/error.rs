use thiserror::Error;

/// Erreur de calcul d'equite (PRD §10.6, M5-3). R-NOPANIC : toute entree
/// invalide est typee ici, jamais de `unwrap()`/`expect()` en dehors des
/// tests.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum EquityError {
    #[error("il faut au moins 2 joueurs pour calculer une equite")]
    TooFewPlayers,

    /// Au-dela de 9 joueurs, jamais observe sur Winamax (meme limite que
    /// `gr_stats::assign_positions`, BACKLOG M4-1).
    #[error("plus de 9 joueurs n'est jamais observe sur Winamax")]
    TooManyPlayers,

    #[error("le board ne peut pas depasser 5 cartes, {0} fournies")]
    BoardTooLong(usize),

    #[error("carte en double : {0}")]
    DuplicateCard(gr_core::Card),
}
