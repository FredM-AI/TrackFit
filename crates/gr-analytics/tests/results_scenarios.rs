//! Scénarios de KPIs de résultats chiffrés au centime (PRD §9.1, M5-1),
//! y compris re-entry, ticket utilisé/gagné et PKO (CA de la story).

use gr_analytics::{compute_results_kpis, Bullet, TicketValuation, TournamentResult};

fn cash_bullet(
    buyin_prize_cents: i64,
    buyin_bounty_cents: i64,
    buyin_fee_cents: i64,
    prize_won_cents: i64,
    bounty_won_cents: i64,
) -> Bullet {
    Bullet {
        buyin_prize_cents,
        buyin_bounty_cents,
        buyin_fee_cents,
        paid_with_ticket_face_value_cents: None,
        prize_won_cents,
        bounty_won_cents,
        ticket_won_face_value_cents: None,
    }
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_simple_non_ko_tournament_computes_cost_profit_and_both_rois() {
    // Buy-in 2,00€ (1,80€ prize + 0,20€ fee), gagne 5,00€ de prize pool.
    let tournaments = [TournamentResult {
        bullets: vec![cash_bullet(180, 0, 20, 500, 0)],
        is_ko: false,
        is_freeroll: false,
    }];

    let kpis = compute_results_kpis(&tournaments, TicketValuation::FaceValue);

    assert_eq!(kpis.tournaments_count, 1);
    assert_eq!(kpis.entries_count, 1);
    assert_eq!(kpis.re_entries_count, 0);
    assert_eq!(kpis.cost_cents, 200);
    assert_eq!(kpis.fees_cents, 20);
    assert_eq!(kpis.prize_pool_winnings_cents, 500);
    assert_eq!(kpis.bounty_winnings_cents, 0);
    assert_eq!(kpis.tickets_won_value_cents, 0);
    assert_eq!(kpis.total_winnings_cents, 500);
    assert_eq!(kpis.profit_cents, 300);
    assert!(approx_eq(kpis.roi.expect("cost > 0"), 300.0 / 200.0));
    assert!(approx_eq(
        kpis.roi_excluding_fees.expect("cost hors frais > 0"),
        (500.0 - 180.0) / 180.0
    ));
    assert_eq!(kpis.prize_pool_roi, None); // pas KO
    assert_eq!(kpis.bounty_roi, None);
    assert_eq!(kpis.itm_count, 1);
    assert_eq!(kpis.itm_rate, Some(1.0));
    assert_eq!(kpis.abi_cents, Some(200.0));
}

#[test]
fn a_re_entry_is_two_bullets_of_the_same_tournament() {
    // Meme tournoi (une seule ligne dans `tournaments`), 2 bullets : le
    // premier perdu, le second gagnant.
    let tournaments = [TournamentResult {
        bullets: vec![
            cash_bullet(180, 0, 20, 0, 0),
            cash_bullet(180, 0, 20, 1000, 0),
        ],
        is_ko: false,
        is_freeroll: false,
    }];

    let kpis = compute_results_kpis(&tournaments, TicketValuation::FaceValue);

    assert_eq!(kpis.tournaments_count, 1);
    assert_eq!(kpis.entries_count, 2);
    assert_eq!(kpis.re_entries_count, 1);
    assert_eq!(kpis.cost_cents, 400); // 2 x 2,00€
    assert_eq!(kpis.fees_cents, 40);
    assert_eq!(kpis.prize_pool_winnings_cents, 1000);
    assert_eq!(kpis.profit_cents, 600);
    assert!(approx_eq(kpis.roi.expect("cost > 0"), 600.0 / 400.0));
    // Toujours 1 seul tournoi ITM (pas 2), et l'ABI divise par le nombre de
    // tournois, pas par le nombre d'entrees.
    assert_eq!(kpis.itm_count, 1);
    assert_eq!(kpis.abi_cents, Some(400.0));
}

#[test]
fn a_ticket_used_as_buy_in_costs_its_face_value_not_the_cash_buy_in() {
    // Ticket de 5,00€ utilise comme buy-in d'un tournoi dont le buy-in cash
    // aurait ete 6,00€ (jamais paye en cash) ; perdu (aucun gain).
    let ticket_bullet = Bullet {
        buyin_prize_cents: 550,
        buyin_bounty_cents: 0,
        buyin_fee_cents: 50,
        paid_with_ticket_face_value_cents: Some(500),
        prize_won_cents: 0,
        bounty_won_cents: 0,
        ticket_won_face_value_cents: None,
    };
    let tournaments = [TournamentResult {
        bullets: vec![ticket_bullet],
        is_ko: false,
        is_freeroll: false,
    }];

    let face_value = compute_results_kpis(&tournaments, TicketValuation::FaceValue);
    assert_eq!(face_value.cost_cents, 500); // valeur faciale du ticket, pas 600
    assert_eq!(face_value.profit_cents, -500);

    let zero = compute_results_kpis(&tournaments, TicketValuation::Zero);
    assert_eq!(zero.cost_cents, 0);
    assert_eq!(zero.profit_cents, 0);
}

#[test]
fn a_ticket_won_counts_as_itm_regardless_of_its_valuation() {
    // Satellite : aucun gain cash, mais un ticket de 20,00€ gagne.
    let tournaments = [TournamentResult {
        bullets: vec![Bullet {
            buyin_prize_cents: 90,
            buyin_bounty_cents: 0,
            buyin_fee_cents: 10,
            paid_with_ticket_face_value_cents: None,
            prize_won_cents: 0,
            bounty_won_cents: 0,
            ticket_won_face_value_cents: Some(2000),
        }],
        is_ko: false,
        is_freeroll: false,
    }];

    let face_value = compute_results_kpis(&tournaments, TicketValuation::FaceValue);
    assert_eq!(face_value.tickets_won_value_cents, 2000);
    assert_eq!(face_value.total_winnings_cents, 2000);
    assert_eq!(face_value.profit_cents, 1900);
    assert_eq!(face_value.itm_count, 1);

    let zero = compute_results_kpis(&tournaments, TicketValuation::Zero);
    assert_eq!(zero.tickets_won_value_cents, 0);
    assert_eq!(zero.total_winnings_cents, 0);
    assert_eq!(zero.profit_cents, -100);
    // ITM depend du statut "ticket gagne", pas de sa valorisation (PRD §9.1).
    assert_eq!(zero.itm_count, 1);
}

#[test]
fn a_pko_bounty_only_win_is_not_itm_but_has_a_positive_bounty_roi() {
    // KO : buy-in 2,00€ (0,80€ prize + 1,00€ bounty + 0,20€ fee). Bulle
    // ratee (aucun gain de prize pool) mais un bounty de 1,50€ empoche.
    let tournaments = [TournamentResult {
        bullets: vec![cash_bullet(80, 100, 20, 0, 150)],
        is_ko: true,
        is_freeroll: false,
    }];

    let kpis = compute_results_kpis(&tournaments, TicketValuation::FaceValue);

    assert_eq!(kpis.cost_cents, 200);
    assert_eq!(kpis.bounty_winnings_cents, 150);
    assert_eq!(kpis.total_winnings_cents, 150);
    assert_eq!(kpis.profit_cents, -50);
    assert!(approx_eq(kpis.roi.expect("cost > 0"), -50.0 / 200.0));
    // Part prize du cout = 80, gains prize = 0 -> ROI prize pool = -100 %.
    assert!(approx_eq(
        kpis.prize_pool_roi.expect("tournoi KO"),
        -80.0 / 80.0
    ));
    // Part bounty du cout = 100, gains bounty = 150 -> ROI bounties = +50 %.
    assert!(approx_eq(
        kpis.bounty_roi.expect("tournoi KO"),
        50.0 / 100.0
    ));
    // Un bounty seul, sans part de prize pool, ne compte pas comme ITM.
    assert_eq!(kpis.itm_count, 0);
    assert_eq!(kpis.itm_rate, Some(0.0));
}

#[test]
fn a_freeroll_has_no_cost_and_is_excluded_from_roi_but_counted_in_winnings() {
    let tournaments = [TournamentResult {
        bullets: vec![cash_bullet(0, 0, 0, 300, 0)],
        is_ko: false,
        is_freeroll: true,
    }];

    let kpis = compute_results_kpis(&tournaments, TicketValuation::FaceValue);

    assert_eq!(kpis.freerolls_count, 1);
    assert_eq!(kpis.cost_cents, 0);
    assert_eq!(kpis.prize_pool_winnings_cents, 300);
    assert_eq!(kpis.profit_cents, 300);
    // "Exclus du ROI (cout 0)" (PRD §9.1) : division par zero -> None,
    // pas un ROI infini.
    assert_eq!(kpis.roi, None);
    assert_eq!(kpis.itm_count, 1);
}

#[test]
fn a_freeroll_mixed_with_a_paid_tournament_still_lets_roi_use_the_paid_cost_only() {
    let tournaments = [
        TournamentResult {
            bullets: vec![cash_bullet(180, 0, 20, 0, 0)],
            is_ko: false,
            is_freeroll: false,
        },
        TournamentResult {
            bullets: vec![cash_bullet(0, 0, 0, 300, 0)],
            is_ko: false,
            is_freeroll: true,
        },
    ];

    let kpis = compute_results_kpis(&tournaments, TicketValuation::FaceValue);

    assert_eq!(kpis.tournaments_count, 2);
    assert_eq!(kpis.freerolls_count, 1);
    assert_eq!(kpis.cost_cents, 200); // le freeroll n'ajoute rien au cout
    assert_eq!(kpis.total_winnings_cents, 300);
    assert_eq!(kpis.profit_cents, 100);
    assert!(approx_eq(kpis.roi.expect("cost > 0"), 100.0 / 200.0));
    assert_eq!(kpis.itm_count, 1); // seul le freeroll est ITM ici
    assert_eq!(kpis.itm_rate, Some(0.5));
}
