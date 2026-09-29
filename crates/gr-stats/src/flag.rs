//! Couple opportunite/action commun a toutes les stats (PRD §10.1,
//! CLAUDE.md §6) : chaque stat se resume a « le joueur avait-il l'occasion
//! de le faire, et l'a-t-il fait ? ».

/// Resultat d'une fonction `compute_<code>` : `opp` = le joueur avait
/// l'occasion (l'opportunite) d'effectuer l'action de la stat ; `act` = il
/// l'a effectivement effectuee. `act` est toujours `false` quand `opp` est
/// `false` (aucune opportunite ne peut donner lieu a une action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatFlag {
    pub opp: bool,
    pub act: bool,
}

impl StatFlag {
    pub const NONE: Self = Self {
        opp: false,
        act: false,
    };

    #[must_use]
    pub fn opportunity(act: bool) -> Self {
        Self { opp: true, act }
    }
}
