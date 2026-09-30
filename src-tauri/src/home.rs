//! Commande IPC pour l'ecran Accueil (M6-2, PRD §13.1) : cartes KPI (valeur
//! et variation vs periode precedente, sans sparkline), graphe G1 en format
//! reduit, derniere session, etat de l'import. Perimetre convenu avec
//! Frederic (29/09) : sparklines et "top 3 leaks" differes (historique par
//! periode et moteur de benchmarks/leaks, M7-2, pas encore construits).
//!
//! Periode par defaut : les 30 derniers jours vs les 30 jours precedents
//! (pas de filtre de date reglable, M6-1 differe faute d'ecran consommateur
//! au moment de M6-2). `now_ms` est calcule cote UI (comme `since_ms` dans
//! `status.rs`) : le backend ne connait que l'UTC (R-MONEY).

use gr_analytics::{
    compute_results_kpis, fetch_hero_tournament_results, hero_allin_ev_diff_bb, TicketValuation,
    TournamentResult, TournamentResultRow,
};
use gr_store::LastSessionRow;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::import::ImportState;
use crate::watch::read_watched_roots;

const PERIOD_MS: i64 = 30 * 24 * 3_600_000;

#[derive(Debug, Clone, Copy, Default, Serialize, TS)]
pub struct PeriodKpis {
    #[ts(type = "number")]
    pub tournaments_count: u32,
    #[ts(type = "number")]
    pub cost_cents: i64,
    #[ts(type = "number")]
    pub fees_cents: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    pub roi: Option<f64>,
    pub itm_rate: Option<f64>,
    pub abi_cents: Option<f64>,
    /// Somme de `hand_players.allin_ev_diff_chips` en bb (M5-4).
    pub allin_ev_diff_bb: f64,
    /// `None` si aucune session n'a demarre dans la periode (pas de "$/h"
    /// calculable, pas un vrai zero).
    pub dollars_per_hour_cents: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ProfitCurvePoint {
    #[ts(type = "number")]
    pub tournament_id: i64,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number")]
    pub cumulative_profit_cents: i64,
    /// Meme courbe hors bounties (PRD §13.5/G1 : "en superposition le
    /// profit sans bounties").
    #[ts(type = "number")]
    pub cumulative_profit_excluding_bounty_cents: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct LastSessionPayload {
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number")]
    pub ended_at: i64,
    #[ts(type = "number")]
    pub hands: i64,
    #[ts(type = "number")]
    pub tournaments: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    pub best_tournament_name: Option<String>,
    #[ts(type = "number | null")]
    pub best_tournament_profit_cents: Option<i64>,
    pub worst_tournament_name: Option<String>,
    #[ts(type = "number | null")]
    pub worst_tournament_profit_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ImportStatusPayload {
    #[ts(type = "number")]
    pub watched_roots_count: i64,
    #[ts(type = "number")]
    pub total_hands: i64,
    #[ts(type = "number")]
    pub unresolved_errors_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct HomeSnapshotPayload {
    pub current_period: PeriodKpis,
    pub previous_period: PeriodKpis,
    pub profit_curve: Vec<ProfitCurvePoint>,
    pub last_session: Option<LastSessionPayload>,
    pub import_status: ImportStatusPayload,
}

/// Profit (avec et sans bounties) d'un tournoi (un ou plusieurs bullets),
/// en reutilisant `compute_results_kpis` (M5-1) plutot que de redupliquer
/// ses regles de cout/ticket (PRD §9.1/§8.6).
fn tournament_profit_cents(result: &TournamentResult) -> (i64, i64) {
    let with_bounty =
        compute_results_kpis(std::slice::from_ref(result), TicketValuation::FaceValue).profit_cents;

    let mut without_bounty = result.clone();
    for bullet in &mut without_bounty.bullets {
        bullet.buyin_bounty_cents = 0;
        bullet.bounty_won_cents = 0;
    }
    let excluding_bounty = compute_results_kpis(
        std::slice::from_ref(&without_bounty),
        TicketValuation::FaceValue,
    )
    .profit_cents;

    (with_bounty, excluding_bounty)
}

fn period_kpis(
    reader: &rusqlite::Connection,
    state: &ImportState,
    profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<PeriodKpis, String> {
    let results = fetch_hero_tournament_results(reader, profile_id, since_ms, until_ms)
        .map_err(|e| e.to_string())?;
    let tournament_results: Vec<TournamentResult> = results.into_iter().map(|r| r.result).collect();
    let kpis = compute_results_kpis(&tournament_results, TicketValuation::FaceValue);
    let allin_ev_diff_bb =
        hero_allin_ev_diff_bb(reader, profile_id, since_ms, until_ms).map_err(|e| e.to_string())?;
    let session_ms = state
        .store
        .total_session_ms_in_range(profile_id, since_ms, until_ms)
        .map_err(|e| e.to_string())?;
    #[allow(clippy::cast_precision_loss)]
    let hours = session_ms as f64 / 3_600_000.0;
    #[allow(clippy::cast_precision_loss)]
    let dollars_per_hour_cents = (hours > 0.0).then(|| kpis.profit_cents as f64 / hours);

    Ok(PeriodKpis {
        tournaments_count: kpis.tournaments_count,
        cost_cents: kpis.cost_cents,
        fees_cents: kpis.fees_cents,
        profit_cents: kpis.profit_cents,
        roi: kpis.roi,
        itm_rate: kpis.itm_rate,
        abi_cents: kpis.abi_cents,
        allin_ev_diff_bb,
        dollars_per_hour_cents,
    })
}

/// Courbe de profit cumule (PRD G1, §13.5) : tous les tournois du profil,
/// tries chronologiquement. Les tournois sans `started_at` connu (summary
/// rattachee avant toute main, tres rare en pratique) sont exclus : sans
/// horodatage, impossible de les placer sur l'axe X ou dans une periode.
pub(crate) fn profit_curve(all_results: &[TournamentResultRow]) -> Vec<ProfitCurvePoint> {
    let mut sorted: Vec<&TournamentResultRow> = all_results
        .iter()
        .filter(|r| r.started_at.is_some())
        .collect();
    sorted.sort_by_key(|r| r.started_at);

    let mut cumulative = 0i64;
    let mut cumulative_excluding_bounty = 0i64;
    sorted
        .into_iter()
        .map(|r| {
            let (profit, profit_excluding_bounty) = tournament_profit_cents(&r.result);
            cumulative += profit;
            cumulative_excluding_bounty += profit_excluding_bounty;
            ProfitCurvePoint {
                tournament_id: r.tournament_id,
                started_at: r.started_at.unwrap_or_default(),
                cumulative_profit_cents: cumulative,
                cumulative_profit_excluding_bounty_cents: cumulative_excluding_bounty,
            }
        })
        .collect()
}

fn last_session_payload(
    session: &LastSessionRow,
    all_results: &[TournamentResultRow],
) -> LastSessionPayload {
    let mut profit_cents = 0i64;
    let mut best: Option<(&str, i64)> = None;
    let mut worst: Option<(&str, i64)> = None;
    for row in all_results {
        if !session.tournament_ids.contains(&row.tournament_id) {
            continue;
        }
        let (profit, _) = tournament_profit_cents(&row.result);
        profit_cents += profit;
        if best.is_none_or(|(_, best_profit)| profit > best_profit) {
            best = Some((row.name.as_str(), profit));
        }
        if worst.is_none_or(|(_, worst_profit)| profit < worst_profit) {
            worst = Some((row.name.as_str(), profit));
        }
    }

    LastSessionPayload {
        started_at: session.started_at,
        ended_at: session.ended_at,
        hands: session.hands,
        tournaments: session.tournaments,
        profit_cents,
        best_tournament_name: best.map(|(name, _)| name.to_string()),
        best_tournament_profit_cents: best.map(|(_, profit)| profit),
        worst_tournament_name: worst.map(|(name, _)| name.to_string()),
        worst_tournament_profit_cents: worst.map(|(_, profit)| profit),
    }
}

/// Instantane de l'ecran Accueil (M6-2), scope au profil Hero actif (M3-5).
#[tauri::command]
pub fn get_home_snapshot(
    state: State<'_, ImportState>,
    now_ms: i64,
) -> Result<HomeSnapshotPayload, String> {
    let import_status = ImportStatusPayload {
        watched_roots_count: i64::try_from(read_watched_roots(&state.store).len())
            .unwrap_or(i64::MAX),
        total_hands: state.store.count_all_hands().map_err(|e| e.to_string())?,
        unresolved_errors_count: i64::try_from(
            state
                .store
                .list_import_errors(Some("OPEN"))
                .map_err(|e| e.to_string())?
                .len(),
        )
        .unwrap_or(i64::MAX),
    };

    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        // Aucun profil Hero encore (avant la fin de l'assistant M3-1).
        return Ok(HomeSnapshotPayload {
            current_period: PeriodKpis::default(),
            previous_period: PeriodKpis::default(),
            profit_curve: Vec::new(),
            last_session: None,
            import_status,
        });
    };

    let current_start = now_ms - PERIOD_MS;
    let previous_start = now_ms - 2 * PERIOD_MS;

    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let current_period = period_kpis(
        &reader,
        &state,
        profile_id,
        Some(current_start),
        Some(now_ms),
    )?;
    let previous_period = period_kpis(
        &reader,
        &state,
        profile_id,
        Some(previous_start),
        Some(current_start),
    )?;

    let all_results = fetch_hero_tournament_results(&reader, profile_id, None, None)
        .map_err(|e| e.to_string())?;
    let profit_curve = profit_curve(&all_results);

    let last_session = state
        .store
        .latest_session(profile_id)
        .map_err(|e| e.to_string())?
        .map(|session| last_session_payload(&session, &all_results));

    Ok(HomeSnapshotPayload {
        current_period,
        previous_period,
        profit_curve,
        last_session,
        import_status,
    })
}

#[cfg(test)]
mod tests {
    use gr_analytics::Bullet;
    use gr_store::LastSessionRow;

    use super::{
        last_session_payload, profit_curve, tournament_profit_cents, TournamentResult,
        TournamentResultRow,
    };

    fn bullet(prize_cents: i64, bounty_cents: i64, buyin_prize: i64, buyin_bounty: i64) -> Bullet {
        Bullet {
            buyin_prize_cents: buyin_prize,
            buyin_bounty_cents: buyin_bounty,
            buyin_fee_cents: 0,
            paid_with_ticket_face_value_cents: None,
            prize_won_cents: prize_cents,
            bounty_won_cents: bounty_cents,
            ticket_won_face_value_cents: None,
        }
    }

    fn row(
        tournament_id: i64,
        name: &str,
        started_at: Option<i64>,
        result: TournamentResult,
    ) -> TournamentResultRow {
        TournamentResultRow {
            tournament_id,
            name: name.to_string(),
            started_at,
            speed: None,
            entrants: None,
            finish_position: None,
            weekday_utc: None,
            hour_utc: None,
            month_utc: None,
            result,
        }
    }

    #[test]
    fn tournament_profit_cents_excludes_the_bounty_leg_in_its_second_value() {
        // Buy-in 1,00€ (0,80€ prize + 0,20€ bounty), gagne 3,00€ de prize
        // pool + 0,50€ de bounty : profit total 2,50€, profit sans bounty
        // 2,20€ (3,00 - 0,80).
        let result = TournamentResult {
            bullets: vec![bullet(300, 50, 80, 20)],
            is_ko: true,
            is_freeroll: false,
        };

        let (with_bounty, excluding_bounty) = tournament_profit_cents(&result);
        assert_eq!(with_bounty, 250);
        assert_eq!(excluding_bounty, 220);
    }

    #[test]
    fn profit_curve_accumulates_chronologically_and_skips_tournaments_without_a_timestamp() {
        let t1 = row(
            1,
            "T1",
            Some(1_000),
            TournamentResult {
                bullets: vec![bullet(0, 0, 100, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        ); // profit -100
        let t2 = row(
            2,
            "T2",
            Some(500), // avant t1 chronologiquement, malgre un id plus grand
            TournamentResult {
                bullets: vec![bullet(300, 0, 100, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        ); // profit +200
        let unresolved = row(
            3,
            "T3",
            None, // summary rattachee avant toute main : pas de started_at
            TournamentResult {
                bullets: vec![bullet(9999, 0, 0, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        );

        let curve = profit_curve(&[t1, t2, unresolved]);

        assert_eq!(curve.len(), 2, "le tournoi sans started_at est exclu");
        assert_eq!(curve[0].tournament_id, 2);
        assert_eq!(curve[0].cumulative_profit_cents, 200);
        assert_eq!(curve[1].tournament_id, 1);
        assert_eq!(curve[1].cumulative_profit_cents, 100);
    }

    #[test]
    fn last_session_payload_picks_the_best_and_worst_tournament_by_profit() {
        let session = LastSessionRow {
            started_at: 0,
            ended_at: 1_000,
            hands: 42,
            tournaments: 2,
            max_tables: 1,
            tournament_ids: vec![10, 20, 30],
        };
        let best = row(
            10,
            "Best",
            Some(0),
            TournamentResult {
                bullets: vec![bullet(1000, 0, 100, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        ); // profit +900
        let worst = row(
            20,
            "Worst",
            Some(0),
            TournamentResult {
                bullets: vec![bullet(0, 0, 100, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        ); // profit -100
        let outside_session = row(
            99,
            "Outside",
            Some(0),
            TournamentResult {
                bullets: vec![bullet(5000, 0, 0, 0)],
                is_ko: false,
                is_freeroll: false,
            },
        );

        let payload = last_session_payload(&session, &[best, worst, outside_session]);

        assert_eq!(
            payload.profit_cents, 800,
            "900 - 100, le tournoi 30 est absent des resultats"
        );
        assert_eq!(payload.best_tournament_name, Some("Best".to_string()));
        assert_eq!(payload.best_tournament_profit_cents, Some(900));
        assert_eq!(payload.worst_tournament_name, Some("Worst".to_string()));
        assert_eq!(payload.worst_tournament_profit_cents, Some(-100));
    }
}
