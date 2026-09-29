//! Insertion des mains parsees (M2-2) : `hands`, `hand_players` (positions,
//! profondeurs et flags de stats preflop calcules par `gr-stats`, M4-3),
//! `actions`, `hand_raw` (zstd) et `tournaments` provisoires (rattaches au
//! summary en M2-6).

use gr_core::{ActionKind, Chips, HandRecord, Street};
use gr_parser_api::Room;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::StoreError;

/// Nombre maximal de mains ecrites dans une seule transaction (PRD §8.4, point 6).
pub const BATCH_SIZE: usize = 500;

/// Une main deja parsee, avec son texte brut d'origine (conserve compresse
/// dans `hand_raw` pour le replayer et un futur reparse, PAR-13).
pub struct HandInsert<'a> {
    pub hand: &'a HandRecord,
    pub raw_text: &'a str,
}

/// Bilan d'une insertion par lot (PRD §8.4, point 4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub inserted: usize,
    pub duplicates: usize,
}

impl ImportReport {
    fn add(&mut self, other: Self) {
        self.inserted += other.inserted;
        self.duplicates += other.duplicates;
    }
}

/// Insere des mains deja parsees, par lots de [`BATCH_SIZE`], chacun dans sa
/// propre transaction. Deduplique sur `UNIQUE(room_id, room_hand_id)` : une
/// main deja presente est ignoree en entier (ni elle, ni ses lignes filles,
/// ne sont reecrites) et comptee dans `duplicates`.
///
/// # Errors
/// Renvoie une [`StoreError`] si une ecriture SQLite ou la compression du
/// texte brut echoue.
pub fn insert_hands(
    conn: &mut Connection,
    room: Room,
    hands: &[HandInsert<'_>],
) -> Result<ImportReport, StoreError> {
    let mut report = ImportReport::default();
    for chunk in hands.chunks(BATCH_SIZE) {
        let tx = conn.transaction()?;
        let room_id = get_or_create_room(&tx, room)?;
        for item in chunk {
            report.add(insert_one_hand(&tx, room_id, item)?);
        }
        tx.commit()?;
    }
    Ok(report)
}

fn insert_one_hand(
    tx: &Transaction<'_>,
    room_id: i64,
    item: &HandInsert<'_>,
) -> Result<ImportReport, StoreError> {
    let hand = item.hand;
    let tournament_id = get_or_create_tournament(
        tx,
        room_id,
        &hand.tournament_room_id,
        &hand.tournament_name,
        Some(hand.played_at),
    )?;
    let hero_player_id = match &hand.hero_pseudo {
        Some(pseudo) => Some(get_or_create_player(tx, room_id, pseudo, hand.played_at)?),
        None => None,
    };
    let board: String = hand.board.iter().map(ToString::to_string).collect();
    let players_dealt =
        i64::try_from(hand.seats.iter().filter(|s| s.dealt_in).count()).unwrap_or(i64::MAX);

    let changes = tx.execute(
        "INSERT OR IGNORE INTO hands (
            room_id, room_hand_id, tournament_id, table_name, table_max_seats,
            players_dealt, button_seat, level, sb, bb, ante, played_at,
            board, total_pot, rake, hero_player_id, parser_version
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        params![
            room_id,
            hand.room_hand_id,
            tournament_id,
            hand.table_name,
            hand.table_max_seats,
            players_dealt,
            hand.button_seat,
            hand.level,
            hand.sb.amount(),
            hand.bb.amount(),
            hand.ante.amount(),
            hand.played_at,
            board,
            hand.total_pot.amount(),
            hand.rake.amount(),
            hero_player_id,
            hand.parser_version,
        ],
    )?;

    if changes == 0 {
        return Ok(ImportReport {
            inserted: 0,
            duplicates: 1,
        });
    }
    let hand_id = tx.last_insert_rowid();

    insert_hand_raw(tx, hand_id, item.raw_text)?;
    insert_hand_players(tx, room_id, hand_id, hand)?;
    insert_actions(tx, room_id, hand_id, hand)?;

    Ok(ImportReport {
        inserted: 1,
        duplicates: 0,
    })
}

fn insert_hand_raw(tx: &Transaction<'_>, hand_id: i64, raw_text: &str) -> Result<(), StoreError> {
    let compressed = zstd::bulk::compress(raw_text.as_bytes(), 3)?;
    tx.execute(
        "INSERT INTO hand_raw (hand_id, codec, data) VALUES (?1, 'zstd', ?2)",
        params![hand_id, compressed],
    )?;
    Ok(())
}

/// Insere `hand_players`, avec les positions, profondeurs et flags de
/// stats preflop calcules par `gr-stats` (PRD §10.2/§10.3/§10.4, M4-3).
/// Un seul `assign_positions` par main (pas par siege) : evite de
/// recalculer les positions de tous les sieges N fois.
fn insert_hand_players(
    tx: &Transaction<'_>,
    room_id: i64,
    hand_id: i64,
    hand: &HandRecord,
) -> Result<(), StoreError> {
    let positions = gr_stats::assign_positions(hand);
    for seat in &hand.seats {
        let player_id = get_or_create_player(tx, room_id, &seat.pseudo, hand.played_at)?;
        let is_hero = hand.hero_pseudo.as_deref() == Some(seat.pseudo.as_str());
        let hole_cards = if is_hero {
            hand.hero_cards.map(|(a, b)| format!("{a}{b}"))
        } else {
            None
        };

        let position = positions.get(&seat.seat).copied();
        let position_label = position.map(|p| p.to_string());
        let position_group_label = position.map(gr_stats::position_group);

        let stack_bb = gr_stats::depth_bb(hand, seat.seat, gr_stats::DepthMode::Player);
        let eff_stack_bb = gr_stats::depth_bb(hand, seat.seat, gr_stats::DepthMode::Effective);
        let depth_bucket =
            stack_bb.map(|d| gr_stats::depth_bracket_label(d, gr_stats::DEFAULT_DEPTH_BRACKETS));
        let eff_depth_bucket = eff_stack_bb
            .map(|d| gr_stats::depth_bracket_label(d, gr_stats::DEFAULT_DEPTH_BRACKETS));

        let preflop_line = gr_stats::preflop_line(hand, &seat.pseudo);

        let vpip = gr_stats::compute_vpip(hand, &seat.pseudo);
        let pfr = gr_stats::compute_pfr(hand, &seat.pseudo);
        let rfi = gr_stats::compute_rfi(hand, &seat.pseudo);
        let limp = gr_stats::compute_limp(hand, &seat.pseudo);
        let oshove = gr_stats::compute_oshove(hand, &seat.pseudo);
        let three_bet = gr_stats::compute_3b(hand, &seat.pseudo);
        let f3b = gr_stats::compute_f3b(hand, &seat.pseudo);
        let four_bet = gr_stats::compute_4b(hand, &seat.pseudo);
        let ats = gr_stats::compute_ats(hand, &seat.pseudo);
        let fsteal = gr_stats::compute_fsteal(hand, &seat.pseudo);
        let rsteal = gr_stats::compute_rsteal(hand, &seat.pseudo);

        tx.execute(
            "INSERT INTO hand_players (
                hand_id, player_id, seat, position, position_group, is_hero, start_stack,
                stack_bb, eff_stack_bb, depth_bucket, eff_depth_bucket, hole_cards, preflop_line,
                vpip_opp, vpip, pfr,
                rfi_opp, rfi, limp, oshove,
                tb_opp, tb, f3b_opp, f3b, fb_opp, fb,
                ats_opp, ats, fsteal_opp, fsteal, rsteal
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16,
                ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26,
                ?27, ?28, ?29, ?30, ?31
            )",
            params![
                hand_id,
                player_id,
                seat.seat,
                position_label,
                position_group_label,
                is_hero,
                seat.starting_stack.amount(),
                stack_bb,
                eff_stack_bb,
                depth_bucket,
                eff_depth_bucket,
                hole_cards,
                preflop_line,
                vpip.opp,
                vpip.act,
                pfr.act,
                rfi.opp,
                rfi.act,
                limp.act,
                oshove.act,
                three_bet.opp,
                three_bet.act,
                f3b.opp,
                f3b.act,
                four_bet.opp,
                four_bet.act,
                ats.opp,
                ats.act,
                fsteal.opp,
                fsteal.act,
                rsteal.act,
            ],
        )?;
    }
    Ok(())
}

fn insert_actions(
    tx: &Transaction<'_>,
    room_id: i64,
    hand_id: i64,
    hand: &HandRecord,
) -> Result<(), StoreError> {
    for (seq, action) in hand.actions.iter().enumerate() {
        let seq = i64::try_from(seq).unwrap_or(i64::MAX);
        let player_id = get_or_create_player(tx, room_id, &action.pseudo, hand.played_at)?;
        tx.execute(
            "INSERT INTO actions (hand_id, seq, street, player_id, kind, amount, to_amount, is_allin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                hand_id,
                seq,
                street_code(action.street),
                player_id,
                action_kind_code(action.kind),
                action.amount.map(Chips::amount),
                action.to_amount.map(Chips::amount),
                action.is_all_in,
            ],
        )?;
    }
    Ok(())
}

/// Prend `&Connection` (pas `&Transaction`) : appelable depuis une
/// transaction existante (deref coercion) ou sans transaction (M3-1).
pub(crate) fn get_or_create_room(conn: &Connection, room: Room) -> Result<i64, StoreError> {
    let (code, name) = match room {
        Room::Winamax => ("winamax", "Winamax"),
    };
    if let Some(id) = conn
        .query_row("SELECT id FROM rooms WHERE code = ?1", [code], |row| {
            row.get(0)
        })
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO rooms (code, name) VALUES (?1, ?2)",
        params![code, name],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Cree le tournoi "provisoire" s'il est inconnu pour `(room_id,
/// room_tournament_id)` (M2-2), sinon renvoie son id sans le modifier (le
/// rattachement du summary, M2-6, met a jour les autres colonnes separement).
/// `started_at` est `None` quand l'appelant n'a pas de main a s'y referer
/// (summary arrive avant toute main, PRD §8.5 : l'ordre d'import est libre).
pub(crate) fn get_or_create_tournament(
    tx: &Transaction<'_>,
    room_id: i64,
    room_tournament_id: &str,
    name: &str,
    started_at: Option<i64>,
) -> Result<i64, StoreError> {
    if let Some(id) = tx
        .query_row(
            "SELECT id FROM tournaments WHERE room_id = ?1 AND room_tournament_id = ?2",
            params![room_id, room_tournament_id],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    tx.execute(
        "INSERT INTO tournaments (room_id, room_tournament_id, name, started_at, status)
         VALUES (?1, ?2, ?3, ?4, 'PROVISIONAL')",
        params![room_id, room_tournament_id, name, started_at],
    )?;
    Ok(tx.last_insert_rowid())
}

/// Cree le joueur s'il est inconnu pour cette salle, sinon elargit la plage
/// `first_seen_at`/`last_seen_at` (§14, prealable a l'auto-classification).
/// N'utiliser qu'avec un horodatage fiable (celui d'une vraie main) : voir
/// [`get_or_create_player_id`] sinon.
pub(crate) fn get_or_create_player(
    tx: &Transaction<'_>,
    room_id: i64,
    screen_name: &str,
    seen_at: i64,
) -> Result<i64, StoreError> {
    tx.execute(
        "INSERT INTO players (room_id, screen_name, first_seen_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(room_id, screen_name) DO UPDATE SET
            first_seen_at = MIN(players.first_seen_at, excluded.first_seen_at),
            last_seen_at = MAX(players.last_seen_at, excluded.last_seen_at)",
        params![room_id, screen_name, seen_at],
    )?;
    get_player_id(tx, room_id, screen_name)
}

/// Cree le joueur s'il est inconnu, sans toucher `first_seen_at`/
/// `last_seen_at` (laisses `NULL`) : pour un appelant qui n'a pas
/// d'horodatage de main fiable (rattachement d'un summary, M2-6, ou
/// creation d'un compte Hero, M3-1). Prend `&Connection` (pas `&Transaction`).
pub(crate) fn get_or_create_player_id(
    conn: &Connection,
    room_id: i64,
    screen_name: &str,
) -> Result<i64, StoreError> {
    conn.execute(
        "INSERT INTO players (room_id, screen_name) VALUES (?1, ?2)
         ON CONFLICT(room_id, screen_name) DO NOTHING",
        params![room_id, screen_name],
    )?;
    get_player_id(conn, room_id, screen_name)
}

fn get_player_id(conn: &Connection, room_id: i64, screen_name: &str) -> Result<i64, StoreError> {
    Ok(conn.query_row(
        "SELECT id FROM players WHERE room_id = ?1 AND screen_name = ?2",
        params![room_id, screen_name],
        |row| row.get(0),
    )?)
}

fn street_code(street: Street) -> &'static str {
    match street {
        Street::Preflop => "PREFLOP",
        Street::Flop => "FLOP",
        Street::Turn => "TURN",
        Street::River => "RIVER",
        Street::Showdown => "SHOWDOWN",
    }
}

fn action_kind_code(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::PostAnte => "POST_ANTE",
        ActionKind::PostSmallBlind => "POST_SB",
        ActionKind::PostBigBlind => "POST_BB",
        ActionKind::Fold => "FOLD",
        ActionKind::Check => "CHECK",
        ActionKind::Call => "CALL",
        ActionKind::Bet => "BET",
        ActionKind::Raise => "RAISE",
        ActionKind::Shows => "SHOWS",
        ActionKind::Collected => "COLLECTED",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

    use super::*;
    use crate::migrate::run_migrations;

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn obelisk_hands() -> Vec<(String, HandRecord)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
        );
        let text = std::fs::read_to_string(path).expect("real OBELISK fixture should be readable");
        let (blocks, _offset) = split_hand_blocks(&text);
        blocks
            .into_iter()
            .map(|block| {
                let hand = WinamaxParser::parse_hand(block).expect("fixture hand should parse");
                (block.to_string(), hand)
            })
            .collect()
    }

    #[test]
    fn inserts_a_real_hand_with_its_players_and_actions() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let (raw_text, hand) = &hands[0];
        let insert = HandInsert { hand, raw_text };

        let report =
            insert_hands(&mut conn, Room::Winamax, &[insert]).expect("insertion should succeed");
        assert_eq!(
            report,
            ImportReport {
                inserted: 1,
                duplicates: 0
            }
        );

        let hand_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
            .expect("hands should be readable");
        assert_eq!(hand_count, 1);

        let player_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM hand_players", [], |row| row.get(0))
            .expect("hand_players should be readable");
        assert_eq!(player_rows, i64::try_from(hand.seats.len()).unwrap());

        let action_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .expect("actions should be readable");
        assert_eq!(action_rows, i64::try_from(hand.actions.len()).unwrap());

        let raw_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM hand_raw", [], |row| row.get(0))
            .expect("hand_raw should be readable");
        assert_eq!(raw_rows, 1);

        let tournament_status: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .expect("a provisional tournament should have been created");
        assert_eq!(tournament_status, "PROVISIONAL");
    }

    #[test]
    fn hand_players_have_preflop_stats_computed_at_insertion() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let (raw_text, hand) = &hands[0];
        let insert = HandInsert { hand, raw_text };
        insert_hands(&mut conn, Room::Winamax, &[insert]).expect("insertion should succeed");

        let hero_pseudo = hand
            .hero_pseudo
            .as_deref()
            .expect("fixture hand should have a hero");
        let player_id: i64 = conn
            .query_row(
                "SELECT id FROM players WHERE screen_name = ?1",
                [hero_pseudo],
                |row| row.get(0),
            )
            .expect("hero player should exist");

        let (position, position_group, vpip_opp, stack_bb): (
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<f64>,
        ) = conn
            .query_row(
                "SELECT position, position_group, vpip_opp, stack_bb
                 FROM hand_players WHERE player_id = ?1",
                [player_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("hero hand_players row should exist");

        assert!(position.is_some(), "position not computed at import (M4-3)");
        assert!(
            position_group.is_some(),
            "position_group not computed at import (M4-3)"
        );
        assert!(vpip_opp.is_some(), "vpip_opp not computed at import (M4-3)");
        assert!(stack_bb.is_some(), "stack_bb not computed at import (M4-3)");
    }

    #[test]
    fn reinserting_the_same_hand_is_deduplicated_and_leaves_no_extra_rows() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let (raw_text, hand) = &hands[0];
        let insert = || HandInsert { hand, raw_text };

        let first = insert_hands(&mut conn, Room::Winamax, &[insert()]).expect("first insert");
        assert_eq!(
            first,
            ImportReport {
                inserted: 1,
                duplicates: 0
            }
        );

        let second = insert_hands(&mut conn, Room::Winamax, &[insert()]).expect("second insert");
        assert_eq!(
            second,
            ImportReport {
                inserted: 0,
                duplicates: 1
            }
        );

        let hand_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
            .expect("hands should be readable");
        assert_eq!(hand_count, 1);

        let action_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .expect("actions should be readable");
        assert_eq!(action_rows, i64::try_from(hand.actions.len()).unwrap());
    }

    #[test]
    fn inserts_every_hand_of_a_batch_spanning_more_than_one_transaction_chunk() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        assert!(
            hands.len() > 1,
            "the fixture must contain at least 2 hands for this test to mean anything"
        );

        // On force plusieurs petits lots (au lieu d'un seul de BATCH_SIZE) pour
        // verifier que `insert_hands` traite correctement plusieurs transactions
        // a la suite, sans perdre ni dupliquer de main.
        let chunk_size = 3.min(hands.len());
        let mut total = ImportReport::default();
        for chunk in hands.chunks(chunk_size) {
            let inserts: Vec<HandInsert<'_>> = chunk
                .iter()
                .map(|(raw_text, hand)| HandInsert { hand, raw_text })
                .collect();
            total.add(insert_hands(&mut conn, Room::Winamax, &inserts).expect("chunk insert"));
        }

        assert_eq!(total.inserted, hands.len());
        assert_eq!(total.duplicates, 0);

        let hand_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
            .expect("hands should be readable");
        assert_eq!(hand_count, i64::try_from(hands.len()).unwrap());
    }

    #[test]
    fn batch_size_matches_the_prd_target_of_500_hands_per_transaction() {
        assert_eq!(BATCH_SIZE, 500);
    }
}
