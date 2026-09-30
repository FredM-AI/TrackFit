//! Commande IPC pour l'ecran Replayer (M7-3, PRD §13.7). Une seule commande
//! ([`get_hand_replay`]) renvoie le rejeu complet d'une main : elle ne
//! depend d'aucun profil Hero actif (le `hand_id` suffit, transmis depuis
//! l'ecran Mains via double-clic) — contrairement aux autres ecrans qui
//! filtrent par `resolve_active_hero_profile_id`.
//!
//! Les enums de domaine (`gr_core::Street`/`ActionKind`) ne derivent pas
//! `ts_rs::TS` (types `gr-core`, purs, sans dependance IPC) : converties ici
//! en `String` (camelCase) pour le payload, meme choix que le reste de
//! `src-tauri` (jamais de tuple brut expose a `ts-rs`, cf. `results.rs`).

use gr_core::{ActionKind, Street};
use gr_store::{
    HandReplay, ReplayAllIn, ReplayPot, ReplaySeatState, ReplaySeatSummary, ReplayStep,
};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::import::ImportState;

fn street_label(street: Street) -> &'static str {
    match street {
        Street::Preflop => "preflop",
        Street::Flop => "flop",
        Street::Turn => "turn",
        Street::River => "river",
        Street::Showdown => "showdown",
    }
}

fn action_kind_label(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::PostAnte => "postAnte",
        ActionKind::PostSmallBlind => "postSmallBlind",
        ActionKind::PostBigBlind => "postBigBlind",
        ActionKind::Fold => "fold",
        ActionKind::Check => "check",
        ActionKind::Call => "call",
        ActionKind::Bet => "bet",
        ActionKind::Raise => "raise",
        ActionKind::Shows => "shows",
        ActionKind::Collected => "collected",
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplaySeatSummaryPayload {
    #[ts(type = "number")]
    pub seat: i64,
    pub pseudo: String,
    pub is_hero: bool,
    #[ts(type = "number")]
    pub starting_stack: i64,
    #[ts(type = "number | null")]
    pub bounty_cents: Option<i64>,
}

impl From<ReplaySeatSummary> for ReplaySeatSummaryPayload {
    fn from(s: ReplaySeatSummary) -> Self {
        Self {
            seat: i64::from(s.seat),
            pseudo: s.pseudo,
            is_hero: s.is_hero,
            starting_stack: s.starting_stack,
            bounty_cents: s.bounty_cents,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplaySeatStatePayload {
    #[ts(type = "number")]
    pub seat: i64,
    pub pseudo: String,
    pub is_hero: bool,
    #[ts(type = "number")]
    pub stack: i64,
    pub folded: bool,
    pub all_in: bool,
    pub cards: Option<[String; 2]>,
}

impl From<ReplaySeatState> for ReplaySeatStatePayload {
    fn from(s: ReplaySeatState) -> Self {
        Self {
            seat: i64::from(s.seat),
            pseudo: s.pseudo,
            is_hero: s.is_hero,
            stack: s.stack,
            folded: s.folded,
            all_in: s.all_in,
            cards: s.cards,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplayStepPayload {
    pub street: String,
    pub pseudo: String,
    pub kind: String,
    #[ts(type = "number | null")]
    pub amount: Option<i64>,
    #[ts(type = "number | null")]
    pub to_amount: Option<i64>,
    pub is_all_in: bool,
    pub shown_label: Option<String>,
    #[ts(type = "number | null")]
    pub raw_line: Option<i64>,
    pub board: Vec<String>,
    #[ts(type = "number")]
    pub pot_total: i64,
    pub seats: Vec<ReplaySeatStatePayload>,
    pub spr_before: Option<f64>,
    pub bet_pct_pot: Option<f64>,
}

impl From<ReplayStep> for ReplayStepPayload {
    fn from(s: ReplayStep) -> Self {
        Self {
            street: street_label(s.street).to_string(),
            pseudo: s.pseudo,
            kind: action_kind_label(s.kind).to_string(),
            amount: s.amount,
            to_amount: s.to_amount,
            is_all_in: s.is_all_in,
            shown_label: s.shown_label,
            raw_line: s.raw_line.map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
            board: s.board,
            pot_total: s.pot_total,
            seats: s
                .seats
                .into_iter()
                .map(ReplaySeatStatePayload::from)
                .collect(),
            spr_before: s.spr_before,
            bet_pct_pot: s.bet_pct_pot,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplayPotWinnerPayload {
    pub pseudo: String,
    #[ts(type = "number")]
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplayPotPayload {
    pub label: String,
    #[ts(type = "number")]
    pub amount: i64,
    pub winners: Vec<ReplayPotWinnerPayload>,
}

impl From<ReplayPot> for ReplayPotPayload {
    fn from(p: ReplayPot) -> Self {
        Self {
            label: p.label,
            amount: p.amount,
            winners: p
                .winners
                .into_iter()
                .map(|(pseudo, amount)| ReplayPotWinnerPayload { pseudo, amount })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplayAllInPlayerPayload {
    pub pseudo: String,
    pub equity: f64,
    pub ev_chips: f64,
    #[ts(type = "number")]
    pub actual_won_chips: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ReplayAllInPayload {
    pub street: String,
    pub is_estimated: bool,
    pub players: Vec<ReplayAllInPlayerPayload>,
}

impl From<ReplayAllIn> for ReplayAllInPayload {
    fn from(a: ReplayAllIn) -> Self {
        Self {
            street: street_label(a.street).to_string(),
            is_estimated: a.is_estimated,
            players: a
                .players
                .into_iter()
                .map(|p| ReplayAllInPlayerPayload {
                    pseudo: p.pseudo,
                    equity: p.equity,
                    ev_chips: p.ev_chips,
                    actual_won_chips: p.actual_won_chips,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct HandReplayPayload {
    #[ts(type = "number")]
    pub hand_id: i64,
    pub room_hand_id: String,
    pub tournament_name: String,
    #[ts(type = "number")]
    pub table_max_seats: i64,
    #[ts(type = "number")]
    pub button_seat: i64,
    #[ts(type = "number")]
    pub level: i64,
    #[ts(type = "number")]
    pub sb: i64,
    #[ts(type = "number")]
    pub bb: i64,
    #[ts(type = "number")]
    pub ante: i64,
    #[ts(type = "number")]
    pub played_at: i64,
    pub hero_pseudo: Option<String>,
    pub seats: Vec<ReplaySeatSummaryPayload>,
    pub steps: Vec<ReplayStepPayload>,
    pub final_pots: Vec<ReplayPotPayload>,
    pub raw_text: String,
    pub all_in: Option<ReplayAllInPayload>,
    #[ts(type = "number | null")]
    pub prev_hand_id: Option<i64>,
    #[ts(type = "number | null")]
    pub next_hand_id: Option<i64>,
}

impl From<HandReplay> for HandReplayPayload {
    fn from(r: HandReplay) -> Self {
        Self {
            hand_id: r.hand_id,
            room_hand_id: r.room_hand_id,
            tournament_name: r.tournament_name,
            table_max_seats: i64::from(r.table_max_seats),
            button_seat: i64::from(r.button_seat),
            level: i64::from(r.level),
            sb: r.sb,
            bb: r.bb,
            ante: r.ante,
            played_at: r.played_at,
            hero_pseudo: r.hero_pseudo,
            seats: r
                .seats
                .into_iter()
                .map(ReplaySeatSummaryPayload::from)
                .collect(),
            steps: r.steps.into_iter().map(ReplayStepPayload::from).collect(),
            final_pots: r
                .final_pots
                .into_iter()
                .map(ReplayPotPayload::from)
                .collect(),
            raw_text: r.raw_text,
            all_in: r.all_in.map(ReplayAllInPayload::from),
            prev_hand_id: r.prev_hand_id,
            next_hand_id: r.next_hand_id,
        }
    }
}

/// Rejeu complet de la main `hand_id` (M7-3). `None` si la main est
/// inconnue ou si son texte brut ne reparse plus (jamais observe en
/// pratique, meme garde que le retro-remplissage `net_chips`, M6-3).
#[tauri::command]
pub fn get_hand_replay(
    hand_id: i64,
    state: State<'_, ImportState>,
) -> Result<Option<HandReplayPayload>, String> {
    state
        .store
        .get_hand_replay(hand_id)
        .map(|r| r.map(HandReplayPayload::from))
        .map_err(|e| e.to_string())
}
