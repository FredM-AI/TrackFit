//! KPIs de résultats MTT (PRD §9.1, M5-1) : coût, frais, gains, profit,
//! ROI (brut/hors frais/prize pool/bounties), ITM %, ABI. Fonctions pures
//! sur des résultats déjà agrégés (pas d'I/O ici, cohérent avec `gr-core`/
//! `gr-stats`) : le pont vers `tournaments`/`tournament_entries`/
//! `tournament_bullets` (`gr-store`) reste à faire quand un écran en aura
//! besoin (M6) — hors périmètre de cette story (CA : "scénarios chiffrés
//! au centime", pas d'intégration store).
//!
//! **Portée volontairement limitée** à la liste explicite du BACKLOG
//! (coût, frais, prize/bounty, tickets, profit, ROI, ROI hors frais, ROI
//! prize pool/bounties, ITM, ABI, freerolls) : place moyenne/% tables
//! finales/meilleur gain/plus gros tournoi/$-heure (présents dans le
//! tableau PRD §9.1 mais pas dans la description BACKLOG de M5-1) sont
//! différés à une story dédiée (écran Résultats, M6+).

/// Comment valoriser un ticket utilisé ou gagné (PRD §8.6, D20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketValuation {
    /// Un ticket utilisé coûte sa valeur faciale ; un ticket gagné rapporte
    /// sa valeur faciale.
    FaceValue,
    /// Un ticket utilisé coûte 0 € ; un ticket gagné rapporte 0 €.
    Zero,
}

impl TicketValuation {
    fn value(self, face_value_cents: Option<i64>) -> i64 {
        match (self, face_value_cents) {
            (TicketValuation::FaceValue, Some(v)) => v,
            (_, None) | (TicketValuation::Zero, Some(_)) => 0,
        }
    }
}

/// Une entrée (bullet) d'un tournoi (PRD §5.1, `tournament_bullets`) : une
/// ligne par re-entry. Montants en centimes (R-MONEY).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bullet {
    pub buyin_prize_cents: i64,
    pub buyin_bounty_cents: i64,
    pub buyin_fee_cents: i64,
    /// `Some(valeur_faciale)` si ce bullet a été payé avec un ticket (PRD
    /// §8.6) : son coût réel pour les KPIs est alors la valeur du ticket
    /// selon `TicketValuation`, **pas** le buy-in cash du tournoi.
    pub paid_with_ticket_face_value_cents: Option<i64>,
    pub prize_won_cents: i64,
    pub bounty_won_cents: i64,
    /// `Some(valeur_faciale)` si ce bullet a rapporté un ticket (satellite).
    pub ticket_won_face_value_cents: Option<i64>,
}

impl Bullet {
    /// Coût réel de ce bullet pour les KPIs (PRD §9.1/§8.6) : la valeur du
    /// ticket s'il a servi de buy-in, sinon le buy-in cash brut
    /// (prize + bounty + fee).
    fn cost_cents(self, valuation: TicketValuation) -> i64 {
        match self.paid_with_ticket_face_value_cents {
            Some(face_value) => valuation.value(Some(face_value)),
            None => self.buyin_prize_cents + self.buyin_bounty_cents + self.buyin_fee_cents,
        }
    }
}

/// Un tournoi joué par le Hero (PRD §9.1) : un ou plusieurs bullets
/// (re-entries, ordre chronologique).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentResult {
    pub bullets: Vec<Bullet>,
    /// `tournaments.ko_type != 'NONE'` : seuls ces tournois entrent dans le
    /// calcul ROI prize pool / ROI bounties (PRD §9.1).
    pub is_ko: bool,
    /// `tournaments.is_freeroll` : exclu du volume de coût (buy-in nul par
    /// construction) mais compté dans le nombre de tournois et les gains
    /// (PRD §9.1).
    pub is_freeroll: bool,
}

impl TournamentResult {
    fn prize_pool_winnings_cents(&self) -> i64 {
        self.bullets.iter().map(|b| b.prize_won_cents).sum()
    }

    fn any_ticket_won(&self) -> bool {
        self.bullets
            .iter()
            .any(|b| b.ticket_won_face_value_cents.is_some())
    }

    /// ITM (PRD §9.1) : gains prize pool > 0 OU ticket gagné (un bounty
    /// seul, sans part de prize pool, ne compte pas comme ITM).
    fn is_itm(&self) -> bool {
        self.prize_pool_winnings_cents() > 0 || self.any_ticket_won()
    }

    /// Bullets non payés avec un ticket (PRD §9.1, ROI prize pool/bounties) :
    /// un ticket ne se décompose pas en part prize/bounty/fee, donc ces deux
    /// ratios se limitent aux entrées payées cash — simplification
    /// documentée, jamais observable sur le corpus actuel (tickets jamais
    /// rencontrés, `docs/formats/winamax.md` §5.2/§7).
    fn cash_bullets(&self) -> impl Iterator<Item = &Bullet> {
        self.bullets
            .iter()
            .filter(|b| b.paid_with_ticket_face_value_cents.is_none())
    }
}

/// KPIs agrégés sur un ensemble de tournois (PRD §9.1). Montants en
/// centimes (R-MONEY) ; ratios en `f64` (jamais stockés, uniquement
/// dérivés pour l'affichage — même convention que `gr_stats::StatFlag`/
/// `depth_bb`). `None` quand le dénominateur est nul (ex. aucun coût,
/// aucun tournoi KO).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResultsKpis {
    pub tournaments_count: u32,
    pub entries_count: u32,
    pub re_entries_count: u32,
    pub freerolls_count: u32,
    pub cost_cents: i64,
    pub fees_cents: i64,
    pub prize_pool_winnings_cents: i64,
    pub bounty_winnings_cents: i64,
    pub tickets_won_value_cents: i64,
    pub total_winnings_cents: i64,
    pub profit_cents: i64,
    pub roi: Option<f64>,
    pub roi_excluding_fees: Option<f64>,
    pub prize_pool_roi: Option<f64>,
    pub bounty_roi: Option<f64>,
    pub itm_count: u32,
    pub itm_rate: Option<f64>,
    /// Coût moyen par tournoi joué (PRD §9.1/glossaire : "coût moyen par
    /// tournoi (inscription + re-entries)") — divisé par le nombre de
    /// **tournois**, pas d'entrées : une re-entry alourdit le numérateur
    /// d'un tournoi donné, pas le nombre de tournois au dénominateur.
    pub abi_cents: Option<f64>,
}

/// Calcule les KPIs de résultats (PRD §9.1) sur `tournaments`, avec la
/// valorisation `valuation` pour les tickets utilisés/gagnés (PRD §8.6).
#[must_use]
pub fn compute_results_kpis(
    tournaments: &[TournamentResult],
    valuation: TicketValuation,
) -> ResultsKpis {
    let tournaments_count = count(tournaments.len());
    let entries_count = count(tournaments.iter().map(|t| t.bullets.len()).sum());
    let re_entries_count = entries_count.saturating_sub(tournaments_count);
    let freerolls_count = count(tournaments.iter().filter(|t| t.is_freeroll).count());

    let all_bullets = || tournaments.iter().flat_map(|t| &t.bullets);
    let cost_cents: i64 = all_bullets().map(|b| b.cost_cents(valuation)).sum();
    let fees_cents: i64 = all_bullets().map(|b| b.buyin_fee_cents).sum();
    let prize_pool_winnings_cents: i64 = all_bullets().map(|b| b.prize_won_cents).sum();
    let bounty_winnings_cents: i64 = all_bullets().map(|b| b.bounty_won_cents).sum();
    let tickets_won_value_cents: i64 = all_bullets()
        .map(|b| valuation.value(b.ticket_won_face_value_cents))
        .sum();
    let total_winnings_cents =
        prize_pool_winnings_cents + bounty_winnings_cents + tickets_won_value_cents;
    let profit_cents = total_winnings_cents - cost_cents;

    let cost_excluding_fees_cents = cost_cents - fees_cents;
    let winnings_excluding_fees_cents = total_winnings_cents - cost_excluding_fees_cents;

    let ko_cash_bullets = || {
        tournaments
            .iter()
            .filter(|t| t.is_ko)
            .flat_map(TournamentResult::cash_bullets)
    };
    let prize_part_of_cost_cents: i64 = ko_cash_bullets().map(|b| b.buyin_prize_cents).sum();
    let prize_winnings_ko_cents: i64 = ko_cash_bullets().map(|b| b.prize_won_cents).sum();
    let bounty_part_of_cost_cents: i64 = ko_cash_bullets().map(|b| b.buyin_bounty_cents).sum();
    let bounty_winnings_ko_cents: i64 = ko_cash_bullets().map(|b| b.bounty_won_cents).sum();

    let itm_count = count(tournaments.iter().filter(|t| t.is_itm()).count());

    ResultsKpis {
        tournaments_count,
        entries_count,
        re_entries_count,
        freerolls_count,
        cost_cents,
        fees_cents,
        prize_pool_winnings_cents,
        bounty_winnings_cents,
        tickets_won_value_cents,
        total_winnings_cents,
        profit_cents,
        roi: ratio(profit_cents, cost_cents),
        roi_excluding_fees: ratio(winnings_excluding_fees_cents, cost_excluding_fees_cents),
        prize_pool_roi: ratio(
            prize_winnings_ko_cents - prize_part_of_cost_cents,
            prize_part_of_cost_cents,
        ),
        bounty_roi: ratio(
            bounty_winnings_ko_cents - bounty_part_of_cost_cents,
            bounty_part_of_cost_cents,
        ),
        itm_count,
        itm_rate: ratio(i64::from(itm_count), i64::from(tournaments_count)),
        abi_cents: ratio(cost_cents, i64::from(tournaments_count)),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// `numerator / denominator`, `None` si `denominator == 0` (division par
/// zero evitee, R-NOPANIC) plutot qu'un NaN/Inf a propager par erreur.
fn ratio(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(precise_f64(numerator) / precise_f64(denominator))
    }
}

/// Convertit un montant en centimes en `f64` pour un ratio d'affichage
/// (jamais stocke, R-MONEY). Sans perte pour tout montant MTT realiste
/// (tres loin de `2^53`), meme convention que `gr_stats::depth::precise_f64`.
fn precise_f64(amount: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let value = amount as f64;
    value
}
