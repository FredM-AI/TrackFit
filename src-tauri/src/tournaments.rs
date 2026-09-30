//! Commandes IPC pour l'ecran Tournois (M6-4, PRD §13.3). Phase 1, validee
//! avec Frederic le 30/09 : liste virtualisee + detail d'un tournoi, tous
//! deux en **lecture seule**. L'edition des metadonnees (via ticket, vitesse,
//! valeur du ticket gagne — PRD §13.3, backend deja pret depuis M5-2 pour la
//! partie ticket) est differee a la V2, decision explicite de Frederic plutot
//! qu'une phase 2 rapprochee comme M6-2/M6-3. Le badge de classification des
//! adversaires (UC10) reste egalement differe : il depend de M7-5 (moteur de
//! classification), pas encore construit — deja note dans `docs/BACKLOG.md`
//! avant cette story.

use gr_analytics::{
    compute_results_kpis, fetch_hero_tournament_aggregates, fetch_hero_tournament_results,
    fetch_tournament_hero_hands, fetch_tournament_opponents, TicketValuation, TournamentResultRow,
};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::hero_profiles::resolve_active_hero_profile_id;
use crate::import::ImportState;

#[derive(Debug, Clone, Serialize, TS)]
pub struct TournamentListRowPayload {
    #[ts(type = "number")]
    pub tournament_id: i64,
    pub name: String,
    #[ts(type = "number | null")]
    pub started_at: Option<i64>,
    pub speed: Option<String>,
    pub is_ko: bool,
    pub is_freeroll: bool,
    pub status: String,
    #[ts(type = "number")]
    pub buyin_prize_cents: i64,
    #[ts(type = "number")]
    pub buyin_bounty_cents: i64,
    #[ts(type = "number")]
    pub buyin_fee_cents: i64,
    pub paid_with_ticket: bool,
    #[ts(type = "number")]
    pub entries_count: u32,
    #[ts(type = "number | null")]
    pub entrants: Option<i64>,
    #[ts(type = "number | null")]
    pub finish_position: Option<i64>,
    #[ts(type = "number")]
    pub prize_cents: i64,
    #[ts(type = "number")]
    pub bounty_cents: i64,
    #[ts(type = "number")]
    pub tickets_won_value_cents: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    #[ts(type = "number")]
    pub hands_played: i64,
    #[ts(type = "number")]
    pub played_seconds: i64,
    pub ev_diff_bb: f64,
}

fn list_row_from(
    row: &TournamentResultRow,
    hands_played: i64,
    ev_diff_bb: f64,
) -> TournamentListRowPayload {
    let kpis = compute_results_kpis(
        std::slice::from_ref(&row.result),
        TicketValuation::FaceValue,
    );
    let first_bullet = row.result.bullets.first();
    TournamentListRowPayload {
        tournament_id: row.tournament_id,
        name: row.name.clone(),
        started_at: row.started_at,
        speed: row.speed.clone(),
        is_ko: row.result.is_ko,
        is_freeroll: row.result.is_freeroll,
        status: row.status.clone(),
        buyin_prize_cents: first_bullet.map_or(0, |b| b.buyin_prize_cents),
        buyin_bounty_cents: first_bullet.map_or(0, |b| b.buyin_bounty_cents),
        buyin_fee_cents: first_bullet.map_or(0, |b| b.buyin_fee_cents),
        paid_with_ticket: row
            .result
            .bullets
            .iter()
            .any(|b| b.paid_with_ticket_face_value_cents.is_some()),
        entries_count: kpis.entries_count,
        entrants: row.entrants,
        finish_position: row.finish_position,
        prize_cents: kpis.prize_pool_winnings_cents,
        bounty_cents: kpis.bounty_winnings_cents,
        tickets_won_value_cents: kpis.tickets_won_value_cents,
        profit_cents: kpis.profit_cents,
        hands_played,
        played_seconds: row.total_played_seconds,
        ev_diff_bb,
    }
}

/// Liste des tournois du profil Hero actif (PRD §13.3), tout l'historique
/// (pas de panneau de filtres reglable tant que M6-1 n'existe pas, meme
/// convention que G1/M6-2). Le frontend virtualise l'affichage
/// (`@tanstack/react-virtual`) plutot que de paginer cote backend : le volume
/// reel (quelques milliers de tournois) tient sans souci en memoire.
#[tauri::command]
pub fn get_tournaments_list(
    state: State<'_, ImportState>,
) -> Result<Vec<TournamentListRowPayload>, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };

    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let results = fetch_hero_tournament_results(&reader, profile_id, None, None, None)
        .map_err(|e| e.to_string())?;
    let aggregates =
        fetch_hero_tournament_aggregates(&reader, profile_id).map_err(|e| e.to_string())?;

    Ok(results
        .iter()
        .map(|row| {
            let (hands_played, ev_diff_bb) = aggregates
                .iter()
                .find(|a| a.tournament_id == row.tournament_id)
                .map_or((0, 0.0), |a| (a.hands_played, a.ev_diff_bb));
            list_row_from(row, hands_played, ev_diff_bb)
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct TournamentStackPointPayload {
    #[ts(type = "number")]
    pub hand_id: i64,
    #[ts(type = "number")]
    pub played_at: i64,
    #[ts(type = "number")]
    pub level: i64,
    #[ts(type = "number | null")]
    pub start_stack: Option<i64>,
    pub stack_bb: Option<f64>,
    #[ts(type = "number | null")]
    pub net_chips: Option<i64>,
    pub net_bb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct TournamentAllInRowPayload {
    #[ts(type = "number")]
    pub hand_id: i64,
    #[ts(type = "number")]
    pub played_at: i64,
    #[ts(type = "number")]
    pub level: i64,
    pub allin_ev_diff_bb: f64,
    pub net_bb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct TournamentOpponentRowPayload {
    #[ts(type = "number")]
    pub player_id: i64,
    pub screen_name: String,
    #[ts(type = "number")]
    pub hands_together: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct TournamentDetailPayload {
    #[ts(type = "number")]
    pub tournament_id: i64,
    pub name: String,
    #[ts(type = "number | null")]
    pub started_at: Option<i64>,
    pub speed: Option<String>,
    pub is_ko: bool,
    pub is_freeroll: bool,
    pub status: String,
    #[ts(type = "number")]
    pub buyin_prize_cents: i64,
    #[ts(type = "number")]
    pub buyin_bounty_cents: i64,
    #[ts(type = "number")]
    pub buyin_fee_cents: i64,
    pub paid_with_ticket: bool,
    #[ts(type = "number")]
    pub entries_count: u32,
    #[ts(type = "number | null")]
    pub entrants: Option<i64>,
    #[ts(type = "number | null")]
    pub finish_position: Option<i64>,
    #[ts(type = "number")]
    pub prize_cents: i64,
    #[ts(type = "number")]
    pub bounty_cents: i64,
    #[ts(type = "number")]
    pub tickets_won_value_cents: i64,
    #[ts(type = "number")]
    pub profit_cents: i64,
    #[ts(type = "number")]
    pub played_seconds: i64,
    /// "Chronologie du tapis du Hero" (PRD §13.3) : tapis (jetons/bb) en
    /// debut de chaque main, dans l'ordre chronologique.
    pub stack_curve: Vec<TournamentStackPointPayload>,
    /// Sous-ensemble de `stack_curve` dont `allin_ev_diff_bb` est connu.
    pub all_ins: Vec<TournamentAllInRowPayload>,
    pub opponents: Vec<TournamentOpponentRowPayload>,
}

/// Detail d'un tournoi (PRD §13.3), scope au profil Hero actif. `None` si
/// aucun profil actif ou si ce tournoi n'a aucune main du Hero (id invalide
/// ou tournoi d'un autre profil).
#[tauri::command]
pub fn get_tournament_detail(
    tournament_id: i64,
    state: State<'_, ImportState>,
) -> Result<Option<TournamentDetailPayload>, String> {
    let Some(profile_id) =
        resolve_active_hero_profile_id(&state.store).map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    let reader = state.store.reader().map_err(|e| e.to_string())?;
    let results =
        fetch_hero_tournament_results(&reader, profile_id, None, None, Some(tournament_id))
            .map_err(|e| e.to_string())?;
    let Some(row) = results.into_iter().next() else {
        return Ok(None);
    };

    let hands = fetch_tournament_hero_hands(&reader, profile_id, tournament_id)
        .map_err(|e| e.to_string())?;
    let opponents =
        fetch_tournament_opponents(&reader, tournament_id).map_err(|e| e.to_string())?;

    let stack_curve: Vec<TournamentStackPointPayload> = hands
        .iter()
        .map(|h| TournamentStackPointPayload {
            hand_id: h.hand_id,
            played_at: h.played_at,
            level: h.level,
            start_stack: h.start_stack,
            stack_bb: h.stack_bb,
            net_chips: h.net_chips,
            net_bb: h.net_bb,
        })
        .collect();
    let all_ins: Vec<TournamentAllInRowPayload> = hands
        .iter()
        .filter_map(|h| {
            h.allin_ev_diff_bb.map(|diff| TournamentAllInRowPayload {
                hand_id: h.hand_id,
                played_at: h.played_at,
                level: h.level,
                allin_ev_diff_bb: diff,
                net_bb: h.net_bb,
            })
        })
        .collect();
    let opponents: Vec<TournamentOpponentRowPayload> = opponents
        .into_iter()
        .map(|o| TournamentOpponentRowPayload {
            player_id: o.player_id,
            screen_name: o.screen_name,
            hands_together: o.hands_together,
        })
        .collect();

    let kpis = compute_results_kpis(
        std::slice::from_ref(&row.result),
        TicketValuation::FaceValue,
    );
    let first_bullet = row.result.bullets.first();

    Ok(Some(TournamentDetailPayload {
        tournament_id: row.tournament_id,
        name: row.name,
        started_at: row.started_at,
        speed: row.speed,
        is_ko: row.result.is_ko,
        is_freeroll: row.result.is_freeroll,
        status: row.status,
        buyin_prize_cents: first_bullet.map_or(0, |b| b.buyin_prize_cents),
        buyin_bounty_cents: first_bullet.map_or(0, |b| b.buyin_bounty_cents),
        buyin_fee_cents: first_bullet.map_or(0, |b| b.buyin_fee_cents),
        paid_with_ticket: row
            .result
            .bullets
            .iter()
            .any(|b| b.paid_with_ticket_face_value_cents.is_some()),
        entries_count: kpis.entries_count,
        entrants: row.entrants,
        finish_position: row.finish_position,
        prize_cents: kpis.prize_pool_winnings_cents,
        bounty_cents: kpis.bounty_winnings_cents,
        tickets_won_value_cents: kpis.tickets_won_value_cents,
        profit_cents: kpis.profit_cents,
        played_seconds: row.total_played_seconds,
        stack_curve,
        all_ins,
        opponents,
    }))
}
