//! Rejeu pas-a-pas d'une main (M7-3, PRD §13.7). Reparse `hand_raw` (meme
//! motif que `backfill::backfill_net_chips` : lecture -> decompression zstd
//! -> `WinamaxParser::parse_hand`) puis reconstruit, pour chaque action de
//! la main dans l'ordre chronologique, l'etat complet de la table (tapis,
//! pot, cartes visibles, board) juste apres cette action. R-COMP-1 est
//! satisfaite par construction : ce module ne lit jamais que des mains deja
//! importees (donc deja terminees, Winamax n'ecrit une main sur disque
//! qu'une fois close).
//!
//! **Simplification volontaire (meme esprit que `gr_equity::allin`) :** le
//! pot affiche a chaque pas (`ReplayStep::pot_total`) est un pot unique
//! cumule (Σ contributions - Σ collectes), pas un decoupage main pot / side
//! pots distincts pendant le jeu. Les side pots reels n'apparaissent qu'au
//! moment de leur collecte (`ActionKind::Collected`, `PotKind::Side(k)`,
//! deja porteurs de cette information) et dans le recapitulatif final
//! ([`HandReplay::final_pots`]).
//!
//! **Correspondance ligne brute (PRD §13.7, "ligne courante surlignee") :**
//! chaque action est associee, en best effort, a son numero de ligne dans le
//! texte brut. L'algorithme marche `raw_text` et `hand.actions` en
//! parallele : pour chaque action, il cherche la premiere ligne non encore
//! consommee qui commence par `"<pseudo> <verbe>"` (le pseudo est inclus
//! entier dans le prefixe recherche, donc jamais ambigu meme quand un pseudo
//! contient un mot d'action, cf. `docs/formats/winamax.md`
//! §"pseudo-contains-action-verb") puis consomme cette ligne pour ne jamais
//! la rematcher. Voir le test `every_action_of_the_real_corpus_finds_a_raw_line`
//! pour la verification de bout en bout.

use std::collections::{HashMap, HashSet};

use gr_core::{ActionKind, ActionRecord, Card, HandRecord, Street};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;

/// Voir `backfill::MAX_HAND_TEXT_BYTES` : meme plafond, meme raison (marge
/// large plutot qu'une estimation fine de la taille reelle d'une main).
const MAX_HAND_TEXT_BYTES: usize = 1_000_000;

/// Etat d'un siege juste apres l'action d'un [`ReplayStep`].
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySeatState {
    pub seat: u8,
    pub pseudo: String,
    pub is_hero: bool,
    pub stack: i64,
    pub folded: bool,
    pub all_in: bool,
    /// Connues des ce pas : Hero des le debut de la main, un villain
    /// seulement a partir de son propre `Shows` (PRD §13.7 : "les cartes
    /// sont visibles pour le Hero et pour les joueurs qui les ont
    /// montrees"). `None` tant qu'inconnues.
    pub cards: Option<[String; 2]>,
}

/// Un pas de rejeu : une action de `hand.actions`, avec l'etat complet de la
/// table juste apres qu'elle a ete appliquee.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayStep {
    pub street: Street,
    pub pseudo: String,
    pub kind: ActionKind,
    /// Montant ajoute par l'action (post/call/bet/relance incrementale) ou
    /// empoche (`Collected`).
    pub amount: Option<i64>,
    /// Pour une relance : total mise sur la street (`raises X to Y`).
    pub to_amount: Option<i64>,
    pub is_all_in: bool,
    /// Libelle de la main montree, seulement pour `Shows`.
    pub shown_label: Option<String>,
    /// Numero de ligne 0-indexe dans le texte brut ; `None` si jamais
    /// observe en pratique sur le corpus reel (voir le test de couverture
    /// en bas de fichier).
    pub raw_line: Option<usize>,
    /// Cartes du board connues a ce pas (longueur 0/3/4/5 selon la street).
    pub board: Vec<String>,
    pub pot_total: i64,
    pub seats: Vec<ReplaySeatState>,
    /// `tapis_avant_action / pot_avant_action`, uniquement pour les
    /// decisions volontaires chiffrees (`Call`/`Bet`/`Raise`) —
    /// simplification documentee : le propre tapis de l'acteur, pas le
    /// "tapis effectif" plafonne par le plus gros adversaire qu'utilise
    /// `gr_stats::depth` pour les stats agregees. `None` si le pot avant
    /// l'action est nul (division evitee, R-NOPANIC).
    pub spr_before: Option<f64>,
    /// `montant_ajoute / pot_avant_action`, uniquement pour `Bet`/`Raise`.
    pub bet_pct_pot: Option<f64>,
}

/// Un pot final distribue (PRD §13.7, "pots / side pots").
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayPot {
    /// `"pot"`, `"main"` ou `"side N"` (voir `PotKind`).
    pub label: String,
    pub amount: i64,
    pub winners: Vec<(String, i64)>,
}

/// Un siege tel qu'assis au debut de la main (PRD §13.7, en-tete de table).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySeatSummary {
    pub seat: u8,
    pub pseudo: String,
    pub is_hero: bool,
    pub starting_stack: i64,
    pub bounty_cents: Option<i64>,
}

/// Detail (equite + EV) d'un joueur au (au plus un) evenement all-in de la
/// main, voir `gr_equity::compute_all_in_details`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayAllInPlayer {
    pub pseudo: String,
    pub equity: f64,
    pub ev_chips: f64,
    pub actual_won_chips: i64,
}

/// L'evenement all-in de la main, s'il y en a un (`gr_equity::allin` : au
/// plus un par main).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayAllIn {
    pub street: Street,
    /// `true` si l'equite vient de la methode Monte Carlo (4+ joueurs) — a
    /// afficher comme "estimation" (PRD §9.1/§10.6).
    pub is_estimated: bool,
    pub players: Vec<ReplayAllInPlayer>,
}

/// Rejeu complet d'une main (M7-3).
#[derive(Debug, Clone, PartialEq)]
pub struct HandReplay {
    pub hand_id: i64,
    pub room_hand_id: String,
    pub tournament_name: String,
    pub table_max_seats: u8,
    pub button_seat: u8,
    pub level: u32,
    pub sb: i64,
    pub bb: i64,
    pub ante: i64,
    pub played_at: i64,
    pub hero_pseudo: Option<String>,
    pub seats: Vec<ReplaySeatSummary>,
    pub steps: Vec<ReplayStep>,
    pub final_pots: Vec<ReplayPot>,
    pub raw_text: String,
    pub all_in: Option<ReplayAllIn>,
    /// Main precedente/suivante du meme joueur (`hands.hero_player_id`),
    /// par ordre chronologique (PRD §13.7 "navigation main precedente /
    /// suivante") — simplification documentee : navigation chronologique
    /// simple, pas necessairement la liste filtree d'origine (aucun contexte
    /// de filtre courant n'est encore transmis par le routing).
    pub prev_hand_id: Option<i64>,
    pub next_hand_id: Option<i64>,
}

/// Charge et rejoue la main `hand_id` (M7-3). `None` si `hand_id` est
/// inconnu, si son texte brut ne reparse plus (jamais observe, mais pas
/// exclu par construction, meme garde que `backfill::backfill_net_chips`).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn get_hand_replay(
    conn: &Connection,
    hand_id: i64,
) -> Result<Option<HandReplay>, StoreError> {
    let raw: Option<Vec<u8>> = conn
        .query_row(
            "SELECT data FROM hand_raw WHERE hand_id = ?1",
            [hand_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(compressed) = raw else {
        return Ok(None);
    };
    let Ok(decompressed) = zstd::bulk::decompress(&compressed, MAX_HAND_TEXT_BYTES) else {
        return Ok(None);
    };
    let Ok(text) = String::from_utf8(decompressed) else {
        return Ok(None);
    };
    let Ok(hand) = gr_parser_winamax::WinamaxParser::parse_hand(&text) else {
        return Ok(None);
    };

    let mut replay = build_hand_replay(hand_id, &hand, &text);
    replay.prev_hand_id = adjacent_hero_hand_id(conn, hand_id, false)?;
    replay.next_hand_id = adjacent_hero_hand_id(conn, hand_id, true)?;
    Ok(Some(replay))
}

/// Main precedente (`forward = false`) ou suivante (`forward = true`) du
/// meme `hands.hero_player_id` que `hand_id`, par ordre chronologique
/// (`played_at`, puis `id` en cas d'egalite). `None` si `hand_id` est
/// inconnu, si sa main n'a pas de `hero_player_id`, ou s'il n'y a pas de
/// main adjacente.
fn adjacent_hero_hand_id(
    conn: &Connection,
    hand_id: i64,
    forward: bool,
) -> Result<Option<i64>, StoreError> {
    let current: Option<(Option<i64>, i64)> = conn
        .query_row(
            "SELECT hero_player_id, played_at FROM hands WHERE id = ?1",
            [hand_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((Some(hero_player_id), played_at)) = current else {
        return Ok(None);
    };

    let sql = if forward {
        "SELECT id FROM hands
         WHERE hero_player_id = ?1 AND (played_at > ?2 OR (played_at = ?2 AND id > ?3))
         ORDER BY played_at ASC, id ASC LIMIT 1"
    } else {
        "SELECT id FROM hands
         WHERE hero_player_id = ?1 AND (played_at < ?2 OR (played_at = ?2 AND id < ?3))
         ORDER BY played_at DESC, id DESC LIMIT 1"
    };

    conn.query_row(sql, params![hero_player_id, played_at, hand_id], |row| {
        row.get(0)
    })
    .optional()
    .map_err(StoreError::from)
}

/// Etat mutable accumule au fil des actions (tapis, pot, cartes connues) —
/// separe de [`build_hand_replay`] pour que chaque pas reste une fonction
/// courte (`clippy::too_many_lines`).
struct ReplayBuilderState<'h> {
    stacks: HashMap<&'h str, i64>,
    folded: HashSet<&'h str>,
    all_in_seats: HashSet<&'h str>,
    known_cards: HashMap<&'h str, [Card; 2]>,
    pot_open: i64,
}

impl<'h> ReplayBuilderState<'h> {
    fn new(hand: &'h HandRecord) -> Self {
        let mut known_cards = HashMap::new();
        if let (Some(hero), Some((a, b))) = (hand.hero_pseudo.as_deref(), hand.hero_cards) {
            known_cards.insert(hero, [a, b]);
        }
        Self {
            stacks: hand
                .seats
                .iter()
                .filter(|s| s.dealt_in)
                .map(|s| (s.pseudo.as_str(), s.starting_stack.amount()))
                .collect(),
            folded: HashSet::new(),
            all_in_seats: HashSet::new(),
            known_cards,
            pot_open: 0,
        }
    }

    /// Applique `action` a l'etat et renvoie le pas de rejeu correspondant.
    fn apply(
        &mut self,
        hand: &'h HandRecord,
        action: &'h ActionRecord,
        raw_line: Option<usize>,
    ) -> ReplayStep {
        let pot_before = self.pot_open;
        let actor_stack_before = self
            .stacks
            .get(action.pseudo.as_str())
            .copied()
            .unwrap_or(0);
        let amount = action.amount.map(gr_core::Chips::amount);
        self.apply_chip_effect(action, amount);
        if action.is_all_in {
            self.all_in_seats.insert(action.pseudo.as_str());
        }

        let board_len = board_len_for_street(action.street).min(hand.board.len());
        let board = hand.board[..board_len]
            .iter()
            .map(ToString::to_string)
            .collect();
        let seats = self.seat_states(hand);

        let is_voluntary_amount_action = matches!(
            action.kind,
            ActionKind::Call | ActionKind::Bet | ActionKind::Raise
        );
        #[allow(clippy::cast_precision_loss)]
        let spr_before = (is_voluntary_amount_action && pot_before > 0)
            .then(|| actor_stack_before as f64 / pot_before as f64);
        #[allow(clippy::cast_precision_loss)]
        let bet_pct_pot = (matches!(action.kind, ActionKind::Bet | ActionKind::Raise)
            && pot_before > 0)
            .then(|| amount.unwrap_or(0) as f64 / pot_before as f64);

        ReplayStep {
            street: action.street,
            pseudo: action.pseudo.clone(),
            kind: action.kind,
            amount,
            to_amount: action.to_amount.map(gr_core::Chips::amount),
            is_all_in: action.is_all_in,
            shown_label: action.shown_label.clone(),
            raw_line,
            board,
            pot_total: self.pot_open,
            seats,
            spr_before,
            bet_pct_pot,
        }
    }

    fn apply_chip_effect(&mut self, action: &'h ActionRecord, amount: Option<i64>) {
        match action.kind {
            ActionKind::PostAnte
            | ActionKind::PostSmallBlind
            | ActionKind::PostBigBlind
            | ActionKind::Call
            | ActionKind::Bet
            | ActionKind::Raise => {
                let amt = amount.unwrap_or(0);
                if let Some(stack) = self.stacks.get_mut(action.pseudo.as_str()) {
                    *stack -= amt;
                }
                self.pot_open += amt;
            }
            ActionKind::Fold => {
                self.folded.insert(action.pseudo.as_str());
            }
            ActionKind::Collected => {
                let amt = amount.unwrap_or(0);
                if let Some(stack) = self.stacks.get_mut(action.pseudo.as_str()) {
                    *stack += amt;
                }
                self.pot_open -= amt;
            }
            ActionKind::Shows => {
                if let Some([a, b]) = action.shown_cards.as_deref() {
                    self.known_cards.insert(action.pseudo.as_str(), [*a, *b]);
                }
            }
            ActionKind::Check => {}
        }
    }

    fn seat_states(&self, hand: &HandRecord) -> Vec<ReplaySeatState> {
        hand.seats
            .iter()
            .filter(|s| s.dealt_in)
            .map(|s| ReplaySeatState {
                seat: s.seat,
                pseudo: s.pseudo.clone(),
                is_hero: hand.hero_pseudo.as_deref() == Some(s.pseudo.as_str()),
                stack: self.stacks.get(s.pseudo.as_str()).copied().unwrap_or(0),
                folded: self.folded.contains(s.pseudo.as_str()),
                all_in: self.all_in_seats.contains(s.pseudo.as_str()),
                cards: self
                    .known_cards
                    .get(s.pseudo.as_str())
                    .map(|[a, b]| [a.to_string(), b.to_string()]),
            })
            .collect()
    }
}

fn build_seats_summary(hand: &HandRecord) -> Vec<ReplaySeatSummary> {
    hand.seats
        .iter()
        .filter(|s| s.dealt_in)
        .map(|s| ReplaySeatSummary {
            seat: s.seat,
            pseudo: s.pseudo.clone(),
            is_hero: hand.hero_pseudo.as_deref() == Some(s.pseudo.as_str()),
            starting_stack: s.starting_stack.amount(),
            bounty_cents: s.bounty.map(gr_core::Money::cents),
        })
        .collect()
}

fn build_final_pots(hand: &HandRecord) -> Vec<ReplayPot> {
    hand.pots
        .iter()
        .map(|p| ReplayPot {
            label: pot_kind_label(p.pot),
            amount: p.amount.amount(),
            winners: p
                .winners
                .iter()
                .map(|(w, a)| (w.clone(), a.amount()))
                .collect(),
        })
        .collect()
}

fn build_all_in(hand: &HandRecord) -> Option<ReplayAllIn> {
    gr_equity::compute_all_in_details(hand).map(|details| ReplayAllIn {
        street: details.street,
        is_estimated: matches!(details.method, gr_equity::EquityMethod::MonteCarlo { .. }),
        players: details
            .players
            .into_iter()
            .map(|p| ReplayAllInPlayer {
                pseudo: p.pseudo,
                equity: p.equity,
                ev_chips: p.ev_chips,
                actual_won_chips: p.actual_won_chips,
            })
            .collect(),
    })
}

/// Fonction pure : reconstruit le rejeu d'une main deja parsee (pas d'I/O,
/// separee de [`get_hand_replay`] pour rester testable sans SQLite).
fn build_hand_replay(hand_id: i64, hand: &HandRecord, raw_text: &str) -> HandReplay {
    let raw_lines = match_action_lines(raw_text, &hand.actions);
    let mut state = ReplayBuilderState::new(hand);
    let steps = hand
        .actions
        .iter()
        .zip(raw_lines)
        .map(|(action, raw_line)| state.apply(hand, action, raw_line))
        .collect();

    HandReplay {
        hand_id,
        room_hand_id: hand.room_hand_id.clone(),
        tournament_name: hand.tournament_name.clone(),
        table_max_seats: hand.table_max_seats,
        button_seat: hand.button_seat,
        level: hand.level,
        sb: hand.sb.amount(),
        bb: hand.bb.amount(),
        ante: hand.ante.amount(),
        played_at: hand.played_at,
        hero_pseudo: hand.hero_pseudo.clone(),
        seats: build_seats_summary(hand),
        steps,
        final_pots: build_final_pots(hand),
        raw_text: raw_text.to_string(),
        all_in: build_all_in(hand),
        prev_hand_id: None,
        next_hand_id: None,
    }
}

/// Nombre de cartes de board connues une fois `street` atteinte (§4.4 :
/// une fois le board complet, `River`/`Showdown` en voient toutes les 5).
fn board_len_for_street(street: Street) -> usize {
    match street {
        Street::Preflop => 0,
        Street::Flop => 3,
        Street::Turn => 4,
        Street::River | Street::Showdown => 5,
    }
}

fn pot_kind_label(kind: gr_core::PotKind) -> String {
    match kind {
        gr_core::PotKind::Pot => "pot".to_string(),
        gr_core::PotKind::Main => "main".to_string(),
        gr_core::PotKind::Side(n) => format!("side {n}"),
    }
}

/// Prefixe (apres le pseudo, avec l'espace separateur) qui identifie sans
/// ambiguite la ligne brute d'une action (§4.4-4.5 de `docs/formats/winamax.md`).
fn verb_prefix(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::PostAnte => "posts ante ",
        ActionKind::PostSmallBlind => "posts small blind ",
        ActionKind::PostBigBlind => "posts big blind ",
        ActionKind::Fold => "folds",
        ActionKind::Check => "checks",
        ActionKind::Call => "calls ",
        ActionKind::Bet => "bets ",
        ActionKind::Raise => "raises ",
        ActionKind::Shows => "shows [",
        ActionKind::Collected => "collected ",
    }
}

/// Associe chaque action de `actions`, dans l'ordre, a son numero de ligne
/// 0-indexe dans `raw_text` (voir la doc de module). Marche les deux en
/// parallele avec un curseur qui n'avance que vers l'avant : une ligne
/// consommee ne peut plus rematcher une action suivante.
fn match_action_lines(raw_text: &str, actions: &[ActionRecord]) -> Vec<Option<usize>> {
    let lines: Vec<&str> = raw_text.lines().collect();
    let mut used = vec![false; lines.len()];
    let mut cursor = 0usize;
    let mut result = Vec::with_capacity(actions.len());

    for action in actions {
        let prefix = format!("{} {}", action.pseudo, verb_prefix(action.kind));
        let mut found = None;
        for (i, line) in lines.iter().enumerate().skip(cursor) {
            if used[i] {
                continue;
            }
            if line.starts_with(&prefix) {
                found = Some(i);
                used[i] = true;
                cursor = i + 1;
                break;
            }
        }
        result.push(found);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gr_core::{Chips, PotKind, PotResult, SeatInfo};
    use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

    use super::*;

    fn seat(seat_no: u8, pseudo: &str, stack: i64) -> SeatInfo {
        SeatInfo {
            seat: seat_no,
            pseudo: pseudo.to_string(),
            starting_stack: Chips::from_i64(stack),
            bounty: None,
            dealt_in: true,
        }
    }

    fn base_action(pseudo: &str, kind: ActionKind) -> ActionRecord {
        ActionRecord {
            street: Street::Preflop,
            pseudo: pseudo.to_string(),
            kind,
            amount: None,
            to_amount: None,
            is_all_in: false,
            pot: None,
            shown_cards: None,
            shown_label: None,
        }
    }

    fn base_hand(
        seats: Vec<SeatInfo>,
        actions: Vec<ActionRecord>,
        pots: Vec<PotResult>,
    ) -> HandRecord {
        HandRecord {
            room_hand_id: "#1-1-1".to_string(),
            tournament_name: "T".to_string(),
            tournament_room_id: "1".to_string(),
            table_name: "T(1)#1".to_string(),
            table_max_seats: u8::try_from(seats.len()).unwrap_or(u8::MAX),
            button_seat: 1,
            level: 1,
            sb: Chips::from_i64(10),
            bb: Chips::from_i64(20),
            ante: Chips::ZERO,
            played_at: 0,
            seats,
            actions,
            board: Vec::new(),
            pots,
            total_pot: Chips::ZERO,
            rake: Chips::ZERO,
            uncalled_excess: Chips::ZERO,
            hero_pseudo: Some("Hero".to_string()),
            hero_cards: None,
            parser_version: "0.0.1".to_string(),
        }
    }

    #[test]
    fn stacks_and_pot_track_each_contribution_and_the_final_collect() {
        let mut sb = base_action("V1", ActionKind::PostSmallBlind);
        sb.amount = Some(Chips::from_i64(10));
        let mut bb = base_action("Hero", ActionKind::PostBigBlind);
        bb.amount = Some(Chips::from_i64(20));
        let fold = base_action("V1", ActionKind::Fold);
        let mut collected = base_action("Hero", ActionKind::Collected);
        collected.amount = Some(Chips::from_i64(30));

        let hand = base_hand(
            vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
            vec![sb, bb, fold, collected],
            vec![PotResult {
                pot: PotKind::Pot,
                amount: Chips::from_i64(30),
                winners: vec![("Hero".to_string(), Chips::from_i64(30))],
            }],
        );

        let replay = build_hand_replay(42, &hand, "raw");
        assert_eq!(replay.steps.len(), 4);

        let step_small_blind = &replay.steps[0];
        assert_eq!(step_small_blind.pot_total, 10);
        let v1_after_small_blind = step_small_blind
            .seats
            .iter()
            .find(|s| s.pseudo == "V1")
            .unwrap();
        assert_eq!(v1_after_small_blind.stack, 990);

        let step_big_blind = &replay.steps[1];
        assert_eq!(step_big_blind.pot_total, 30);
        let hero_after_big_blind = step_big_blind
            .seats
            .iter()
            .find(|s| s.pseudo == "Hero")
            .unwrap();
        assert_eq!(hero_after_big_blind.stack, 980);

        let after_fold = &replay.steps[2];
        assert!(
            after_fold
                .seats
                .iter()
                .find(|s| s.pseudo == "V1")
                .unwrap()
                .folded
        );

        let after_collect = &replay.steps[3];
        assert_eq!(
            after_collect.pot_total, 0,
            "le pot est vide une fois distribue"
        );
        let hero_final = after_collect
            .seats
            .iter()
            .find(|s| s.pseudo == "Hero")
            .unwrap();
        assert_eq!(hero_final.stack, 1010, "980 (apres la bb) + 30 collectes");
    }

    #[test]
    fn hero_cards_are_known_from_the_start_villain_cards_only_after_shows() {
        let ac = "Ac".parse().unwrap();
        let ad: Card = "Ad".parse().unwrap();
        let mut hand = base_hand(
            vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
            vec![base_action("V1", ActionKind::Check)],
            vec![],
        );
        hand.hero_cards = Some((ac, ad));

        let replay = build_hand_replay(1, &hand, "raw");
        let step = &replay.steps[0];
        let hero = step.seats.iter().find(|s| s.pseudo == "Hero").unwrap();
        assert_eq!(hero.cards, Some(["Ac".to_string(), "Ad".to_string()]));
        let v1 = step.seats.iter().find(|s| s.pseudo == "V1").unwrap();
        assert_eq!(v1.cards, None, "cartes de V1 inconnues avant son Shows");

        let mut shows = base_action("V1", ActionKind::Shows);
        shows.shown_cards = Some(vec!["Kh".parse().unwrap(), "Kd".parse().unwrap()]);
        let mut hand2 = hand;
        hand2.actions = vec![shows];
        let replay2 = build_hand_replay(1, &hand2, "raw");
        let v1_after_shows = replay2.steps[0]
            .seats
            .iter()
            .find(|s| s.pseudo == "V1")
            .unwrap();
        assert_eq!(
            v1_after_shows.cards,
            Some(["Kh".to_string(), "Kd".to_string()])
        );
    }

    #[test]
    fn board_reveals_progressively_with_the_street() {
        let cards: Vec<Card> = ["Jd", "9c", "4d", "Kc", "2h"]
            .iter()
            .map(|c| c.parse().unwrap())
            .collect();
        let mut hand = base_hand(
            vec![seat(1, "Hero", 1000), seat(2, "V1", 1000)],
            vec![
                base_action("Hero", ActionKind::Check),
                {
                    let mut a = base_action("Hero", ActionKind::Check);
                    a.street = Street::Flop;
                    a
                },
                {
                    let mut a = base_action("Hero", ActionKind::Check);
                    a.street = Street::Turn;
                    a
                },
                {
                    let mut a = base_action("Hero", ActionKind::Check);
                    a.street = Street::River;
                    a
                },
            ],
            vec![],
        );
        hand.board = cards;

        let replay = build_hand_replay(1, &hand, "raw");
        assert_eq!(replay.steps[0].board.len(), 0, "preflop : board inconnu");
        assert_eq!(replay.steps[1].board.len(), 3, "flop : 3 cartes");
        assert_eq!(replay.steps[2].board.len(), 4, "turn : 4 cartes");
        assert_eq!(replay.steps[3].board.len(), 5, "river : 5 cartes");
    }

    #[test]
    fn spr_and_bet_pct_pot_use_the_pot_before_the_action() {
        let mut sb = base_action("V1", ActionKind::PostSmallBlind);
        sb.amount = Some(Chips::from_i64(10));
        let mut bb = base_action("Hero", ActionKind::PostBigBlind);
        bb.amount = Some(Chips::from_i64(20));
        let mut raise = base_action("V1", ActionKind::Raise);
        raise.amount = Some(Chips::from_i64(50));
        raise.to_amount = Some(Chips::from_i64(60));

        let hand = base_hand(
            vec![seat(1, "Hero", 1000), seat(2, "V1", 990)],
            vec![sb, bb, raise],
            vec![],
        );

        let replay = build_hand_replay(1, &hand, "raw");
        let raise_step = &replay.steps[2];
        // pot avant la relance = 30 (10 + 20) ; tapis de V1 avant = 990 - 10 = 980.
        assert!((raise_step.spr_before.unwrap() - 980.0 / 30.0).abs() < 1e-9);
        assert!((raise_step.bet_pct_pot.unwrap() - 50.0 / 30.0).abs() < 1e-9);

        // Les postes de blindes n'ont pas de SPR/bet-pct-pot (pas des decisions).
        assert_eq!(replay.steps[0].spr_before, None);
        assert_eq!(replay.steps[0].bet_pct_pot, None);
    }

    fn corpus_files() -> Vec<std::path::PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/winamax");
        let mut files = Vec::new();
        collect_real_hand_files(&root, &mut files);
        files
    }

    fn collect_real_hand_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_real_hand_files(&path, out);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with("_real_holdem_no-limit.txt") {
                    out.push(path);
                }
            }
        }
    }

    /// Propriete de conservation (CLAUDE.md §7) : chaque action de chaque
    /// main du corpus reel doit trouver sa ligne brute — sinon le panneau
    /// historique du replayer (PRD §13.7) ne pourrait pas surligner la ligne
    /// courante pour cette main.
    #[test]
    fn every_action_of_the_real_corpus_finds_a_raw_line() {
        let files = corpus_files();
        assert!(
            !files.is_empty(),
            "le corpus de fixtures reelles devrait etre non vide"
        );

        let mut hands_checked = 0usize;
        for path in files {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("lecture de {}: {e}", path.display()));
            let (blocks, _offset) = split_hand_blocks(&text);
            for block in blocks {
                let Ok(hand) = WinamaxParser::parse_hand(block) else {
                    continue;
                };
                let raw_lines = match_action_lines(block, &hand.actions);
                for (action, raw_line) in hand.actions.iter().zip(&raw_lines) {
                    assert!(
                        raw_line.is_some(),
                        "main {} : pas de ligne trouvee pour {:?} de {}",
                        hand.room_hand_id,
                        action.kind,
                        action.pseudo
                    );
                }
                hands_checked += 1;
            }
        }
        assert!(
            hands_checked > 0,
            "au moins une main devrait avoir ete rejouee"
        );
    }
}
