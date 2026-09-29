#![warn(clippy::pedantic)]

//! Backend de requetage generique pour les rapports (PRD §13.5, ADR-002) :
//! dimensions, mesures (stats §10.2), filtres → table de resultats.
//! L'implementation SQLite (M4-7) est livree en premier ; `DuckDB` suivra en
//! M6, derriere le meme trait [`AnalyticsBackend`].
//!
//! Portee de M4-7 : les 17 stats action/opportunite de PRD §10.2 (VPIP a
//! WWSF), par position et/ou profondeur, pour un profil Hero. AF/AFQ sont
//! exclues : ce sont des ratios de compteurs (pas un couple opportunite/
//! action, cf. `gr_stats::PostflopActionCounts`), une forme differente de
//! [`StatCell`] — a exposer separement quand un ecran en aura besoin.
//! Filtres, dimensions phase/format/cartes/jour : hors perimetre (aucun
//! ecran ne les consomme encore ; phase notamment differee en V2, M4-5).

mod error;
mod home;
mod kpis;
mod sqlite;

pub use error::AnalyticsError;
pub use home::{fetch_hero_tournament_results, hero_allin_ev_diff_bb, TournamentResultRow};
pub use kpis::{compute_results_kpis, Bullet, ResultsKpis, TicketValuation, TournamentResult};
pub use sqlite::SqliteAnalyticsBackend;

/// Dimension de regroupement d'un rapport (PRD §13.5). Portee a ce que
/// M4-7 exige explicitement (position × profondeur) ; d'autres dimensions
/// suivront avec les stories qui les rendent disponibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// `hand_players.position_group` (EP/MP/LP/Blinds, PRD §10.4, M4-3).
    PositionGroup,
    /// `hand_players.eff_depth_bucket` (profondeur effective, PRD §10.3,
    /// mode par defaut du PRD, M4-3).
    DepthBucket,
}

impl Dimension {
    fn column(self) -> &'static str {
        match self {
            Dimension::PositionGroup => "position_group",
            Dimension::DepthBucket => "eff_depth_bucket",
        }
    }
}

/// Une stat action/opportunite calculable par le rapport (PRD §10.2). Le
/// nom entre parentheses est le sigle PRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Measure {
    /// VPIP.
    Vpip,
    /// PFR.
    Pfr,
    /// RFI.
    Rfi,
    /// LIMP.
    Limp,
    /// OSHOVE.
    Oshove,
    /// 3B.
    ThreeBet,
    /// F3B.
    FoldToThreeBet,
    /// 4B.
    FourBet,
    /// ATS.
    Ats,
    /// FSTEAL.
    Fsteal,
    /// RSTEAL.
    Rsteal,
    /// CBF.
    Cbf,
    /// CBT.
    Cbt,
    /// FCBF.
    Fcbf,
    /// WTSD.
    Wtsd,
    /// WSD (W$SD).
    Wsd,
    /// WWSF.
    Wwsf,
}

impl Measure {
    /// Toutes les stats disponibles (PRD §10.2, hors AF/AFQ), dans l'ordre
    /// de la table §10.2.
    pub const ALL: [Measure; 17] = [
        Measure::Vpip,
        Measure::Pfr,
        Measure::Rfi,
        Measure::Limp,
        Measure::Oshove,
        Measure::ThreeBet,
        Measure::FoldToThreeBet,
        Measure::FourBet,
        Measure::Ats,
        Measure::Fsteal,
        Measure::Rsteal,
        Measure::Cbf,
        Measure::Cbt,
        Measure::Fcbf,
        Measure::Wtsd,
        Measure::Wsd,
        Measure::Wwsf,
    ];

    /// Colonnes `hand_players` (opportunite, action) — memes couples que
    /// `gr-store::repo::insert_hand_players` (M4-3/M4-4) : plusieurs stats
    /// partagent la meme colonne d'opportunite (ex. VPIP/PFR → `vpip_opp`),
    /// une des stats "complementaires" du PRD §10.2.
    fn columns(self) -> (&'static str, &'static str) {
        match self {
            Measure::Vpip => ("vpip_opp", "vpip"),
            Measure::Pfr => ("vpip_opp", "pfr"),
            Measure::Rfi => ("rfi_opp", "rfi"),
            Measure::Limp => ("rfi_opp", "limp"),
            Measure::Oshove => ("rfi_opp", "oshove"),
            Measure::ThreeBet => ("tb_opp", "tb"),
            Measure::FoldToThreeBet => ("f3b_opp", "f3b"),
            Measure::FourBet => ("fb_opp", "fb"),
            Measure::Ats => ("ats_opp", "ats"),
            Measure::Fsteal => ("fsteal_opp", "fsteal"),
            Measure::Rsteal => ("fsteal_opp", "rsteal"),
            Measure::Cbf => ("cbf_opp", "cbf"),
            Measure::Cbt => ("cbt_opp", "cbt"),
            Measure::Fcbf => ("fcbf_opp", "fcbf"),
            Measure::Wtsd => ("saw_flop", "went_sd"),
            Measure::Wsd => ("went_sd", "won_sd"),
            Measure::Wwsf => ("saw_flop", "won_hand"),
        }
    }
}

/// Compte opportunite/action agrege pour une cellule du rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatCell {
    pub opportunities: i64,
    pub actions: i64,
}

impl StatCell {
    /// Pourcentage (PRD §10.1 : "pourcentage avec 1 decimale"). `None` sans
    /// aucune opportunite (division par zero), pour que l'appelant affiche
    /// un tiret plutot qu'un faux `0.0 %`.
    #[must_use]
    pub fn percentage(&self) -> Option<f64> {
        if self.opportunities == 0 {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(100.0 * self.actions as f64 / self.opportunities as f64)
        }
    }
}

/// Une requete de rapport (PRD §13.5) : dimensions (lignes), mesures
/// (colonnes), filtree sur un profil Hero (D19, hors perimetre pour
/// l'instant : pas de filtres additionnels, aucun ecran ne les consomme
/// encore).
#[derive(Debug, Clone)]
pub struct ReportRequest {
    pub hero_profile_id: i64,
    pub dimensions: Vec<Dimension>,
    pub measures: Vec<Measure>,
}

/// Une ligne du tableau de resultats : une valeur par dimension demandee
/// (dans l'ordre de `ReportRequest::dimensions`), une cellule par mesure
/// demandee (dans l'ordre de `ReportRequest::measures`). Une valeur de
/// dimension est `None` si `hand_players` n'a pas pu la calculer pour ce
/// groupe (ex. profondeur indeterminee, M4-3).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportRow {
    pub dimension_values: Vec<Option<String>>,
    pub cells: Vec<StatCell>,
}

/// Backend de requetage generique pour les rapports (PRD §13.5, ADR-002) :
/// SQLite livre en premier (M4-7), `DuckDB` en M6.
pub trait AnalyticsBackend {
    /// # Errors
    /// Renvoie une [`AnalyticsError`] si la requete echoue.
    fn run_report(&self, request: &ReportRequest) -> Result<Vec<ReportRow>, AnalyticsError>;
}
