//! Retro-remplissage de colonnes `hand_players` prevues en base depuis
//! `0001_init.sql`/M2-1 mais calculees seulement a partir d'une story
//! ulterieure : `net_chips`/`net_bb` (M6-3), `hand_class` (M7-7), et
//! position/profondeur/flags preflop-postflop/EV all-in (M4-3/M4-4/M5-4,
//! trouve et corrige le 30/09 : confirme par Frederic via `just dev` que ses
//! mains reelles, importees le 28/09 via M2-3 **avant** que M4-3/M4-4
//! n'existent le 29/09, ont `position_group`/`vpip_opp`/etc. a `NULL` — la
//! colonne « Position » de l'ecran Mains affichait des tirets). Meme gap que
//! `net_chips`/`hand_class` avant leurs propres correctifs : une colonne
//! prevue en base des `0001_init.sql` n'est pas calculee des sa creation,
//! seulement a partir de la story qui l'exploite le plus. Relit `hand_raw`
//! (seule source du detail des mains deja importees ; `PotResult`/
//! `hero_cards` ne vivent qu'en memoire pendant le parsing, jamais persistes
//! autrement) et reparse (Winamax uniquement, seule salle supportee en V1,
//! meme convention que `gr_ingest::reparse_import_error`).
//!
//! **`position_group IS NULL` n'est pas toujours un marqueur "jamais
//! traite"** : un siege "assis mais non distribue" (`docs/formats/winamax.md`
//! §4, exclu de la main par construction, 7 cas par fichier dans le corpus
//! reel) a `position_group` a `NULL` a vie, meme fraichement importe —
//! meme acceptation que les autres backfills de ce module (un faux positif
//! occasionnel, jamais une perte de donnees, cf. `backfill_hand_class`).
//!
//! **Note pour une future story de nettoyage** : ces trois retro-
//! remplissages (`backfill_net_chips`, `backfill_hand_class`,
//! `backfill_seat_stats`) reparsent chacun independamment `hand_raw` pour
//! les memes mains anciennes si plusieurs colonnes manquent a la fois —
//! redondant mais sans risque (chaque passe est idempotente et peu
//! frequente, seulement au demarrage). Les consolider en un seul passage
//! serait un refactoring opportuniste hors de ce correctif, pas fait ici
//! (CLAUDE.md §9).

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;
use crate::repo::compute_seat_stats;

/// Tres au-dessus de la taille reelle d'une main (quelques Ko, meme la plus
/// longue multiway du corpus) : marge large plutot qu'une estimation fine,
/// `zstd::bulk::decompress` a juste besoin d'un plafond.
const MAX_HAND_TEXT_BYTES: usize = 1_000_000;

/// Calcule `net_chips`/`net_bb` pour toutes les mains dont `hand_players` ne
/// les a pas encore (`net_chips IS NULL`, marqueur des mains importees avant
/// M6-3). Renvoie le nombre de mains retraitees. Une main dont le texte brut
/// ne reparse plus (jamais observe, mais pas exclu par construction) est
/// ignoree sans faire echouer le reste du lot.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn backfill_net_chips(conn: &mut Connection) -> Result<usize, StoreError> {
    let hand_ids: Vec<i64> = {
        let mut stmt =
            conn.prepare("SELECT DISTINCT hand_id FROM hand_players WHERE net_chips IS NULL")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    if hand_ids.is_empty() {
        return Ok(0);
    }

    let mut processed = 0usize;
    let tx = conn.transaction()?;
    for hand_id in hand_ids {
        let raw: Option<Vec<u8>> = tx
            .query_row(
                "SELECT data FROM hand_raw WHERE hand_id = ?1",
                [hand_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(compressed) = raw else { continue };
        let Ok(decompressed) = zstd::bulk::decompress(&compressed, MAX_HAND_TEXT_BYTES) else {
            continue;
        };
        let Ok(text) = String::from_utf8(decompressed) else {
            continue;
        };
        let Ok(hand) = gr_parser_winamax::WinamaxParser::parse_hand(&text) else {
            continue;
        };

        let mut stmt = tx.prepare(
            "SELECT hp.player_id, p.screen_name FROM hand_players hp
             JOIN players p ON p.id = hp.player_id
             WHERE hp.hand_id = ?1",
        )?;
        let seats: Vec<(i64, String)> = stmt
            .query_map([hand_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        for (player_id, pseudo) in seats {
            let net_chips = gr_stats::compute_net_chips(&hand, &pseudo);
            let net_bb = gr_stats::compute_net_bb(&hand, &pseudo);
            tx.execute(
                "UPDATE hand_players SET net_chips = ?1, net_bb = ?2
                 WHERE hand_id = ?3 AND player_id = ?4",
                params![net_chips, net_bb, hand_id, player_id],
            )?;
        }
        processed += 1;
    }
    tx.commit()?;
    Ok(processed)
}

/// Calcule `hand_players.hand_class` (PRD §13.5, ex. `'AKs'`, `'TT'`,
/// `'Q9o'`) pour les mains dont la ligne Hero ne l'a pas encore
/// (`hand_class IS NULL AND is_hero = 1`, marqueur des mains importees
/// avant M7-7). **Hero uniquement** (meme garde que `hole_cards` dans
/// `repo::insert_hand_players`) : les cartes des adversaires ne sont
/// jamais connues avant un `Shows` en cours de main et ne sont pas
/// persistees pour eux. Renvoie le nombre de mains retraitees.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn backfill_hand_class(conn: &mut Connection) -> Result<usize, StoreError> {
    let hand_ids: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT hand_id FROM hand_players WHERE hand_class IS NULL AND is_hero = 1",
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    if hand_ids.is_empty() {
        return Ok(0);
    }

    let mut processed = 0usize;
    let tx = conn.transaction()?;
    for hand_id in hand_ids {
        let raw: Option<Vec<u8>> = tx
            .query_row(
                "SELECT data FROM hand_raw WHERE hand_id = ?1",
                [hand_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(compressed) = raw else { continue };
        let Ok(decompressed) = zstd::bulk::decompress(&compressed, MAX_HAND_TEXT_BYTES) else {
            continue;
        };
        let Ok(text) = String::from_utf8(decompressed) else {
            continue;
        };
        let Ok(hand) = gr_parser_winamax::WinamaxParser::parse_hand(&text) else {
            continue;
        };

        let hand_class = hand
            .hero_cards
            .map(|(a, b)| gr_stats::compute_hand_class(a, b));
        tx.execute(
            "UPDATE hand_players SET hand_class = ?1 WHERE hand_id = ?2 AND is_hero = 1",
            params![hand_class, hand_id],
        )?;
        processed += 1;
    }
    tx.commit()?;
    Ok(processed)
}

/// Calcule position/profondeur/flags preflop-postflop/EV all-in
/// (`hand_players.position_group IS NULL`, marqueur des mains importees
/// avant que M4-3/M4-4/M5-4 ne calculent ces colonnes — trouve le 30/09,
/// voir la doc de module) pour **tous les sieges** de la main (pas
/// seulement Hero : ces colonnes servent aussi aux adversaires pour les
/// contextes de stats). Reutilise `repo::compute_seat_stats`, exactement
/// la meme fonction que l'import normal (`repo::insert_hand_players`) —
/// aucune divergence possible entre les deux chemins. Renvoie le nombre de
/// mains retraitees.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn backfill_seat_stats(conn: &mut Connection) -> Result<usize, StoreError> {
    let hand_ids: Vec<i64> = {
        let mut stmt =
            conn.prepare("SELECT DISTINCT hand_id FROM hand_players WHERE position_group IS NULL")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    if hand_ids.is_empty() {
        return Ok(0);
    }

    let mut processed = 0usize;
    let tx = conn.transaction()?;
    for hand_id in hand_ids {
        let raw: Option<Vec<u8>> = tx
            .query_row(
                "SELECT data FROM hand_raw WHERE hand_id = ?1",
                [hand_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(compressed) = raw else { continue };
        let Ok(decompressed) = zstd::bulk::decompress(&compressed, MAX_HAND_TEXT_BYTES) else {
            continue;
        };
        let Ok(text) = String::from_utf8(decompressed) else {
            continue;
        };
        let Ok(hand) = gr_parser_winamax::WinamaxParser::parse_hand(&text) else {
            continue;
        };

        let mut stmt = tx.prepare(
            "SELECT hp.player_id, p.screen_name FROM hand_players hp
             JOIN players p ON p.id = hp.player_id
             WHERE hp.hand_id = ?1",
        )?;
        let seats: Vec<(i64, String)> = stmt
            .query_map([hand_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let positions = gr_stats::assign_positions(&hand);
        let allin_event = gr_equity::detect_all_in_event(&hand);

        for (player_id, pseudo) in seats {
            let Some(seat_info) = hand.seats.iter().find(|s| s.pseudo == pseudo) else {
                continue;
            };
            let s = compute_seat_stats(&hand, seat_info, &positions, allin_event.as_ref());
            apply_seat_stats_update(&tx, hand_id, player_id, &s)?;
        }
        processed += 1;
    }
    tx.commit()?;
    Ok(processed)
}

/// Applique un [`SeatStats`](crate::repo::SeatStats) recalcule a la ligne
/// `hand_players` existante — separee de [`backfill_seat_stats`] pour
/// rester sous la limite de lignes de `clippy::pedantic`.
fn apply_seat_stats_update(
    tx: &rusqlite::Transaction<'_>,
    hand_id: i64,
    player_id: i64,
    s: &crate::repo::SeatStats,
) -> Result<(), StoreError> {
    tx.execute(
        "UPDATE hand_players SET
            position = ?1, position_group = ?2, stack_bb = ?3, eff_stack_bb = ?4,
            depth_bucket = ?5, eff_depth_bucket = ?6, preflop_line = ?7,
            vpip_opp = ?8, vpip = ?9, pfr = ?10,
            rfi_opp = ?11, rfi = ?12, limp = ?13, oshove = ?14,
            tb_opp = ?15, tb = ?16, f3b_opp = ?17, f3b = ?18, fb_opp = ?19, fb = ?20,
            ats_opp = ?21, ats = ?22, fsteal_opp = ?23, fsteal = ?24, rsteal = ?25,
            saw_flop = ?26, saw_turn = ?27, saw_river = ?28,
            went_sd = ?29, won_sd = ?30, won_hand = ?31,
            cbf_opp = ?32, cbf = ?33, cbt_opp = ?34, cbt = ?35, fcbf_opp = ?36, fcbf = ?37,
            pf_bets = ?38, pf_raises = ?39, pf_calls = ?40, pf_folds = ?41, pf_checks = ?42,
            allin_ev_diff_chips = ?43
         WHERE hand_id = ?44 AND player_id = ?45",
        params![
            s.position_label,
            s.position_group_label,
            s.stack_bb,
            s.eff_stack_bb,
            s.depth_bucket,
            s.eff_depth_bucket,
            s.preflop_line,
            s.vpip.opp,
            s.vpip.act,
            s.pfr.act,
            s.rfi.opp,
            s.rfi.act,
            s.limp.act,
            s.oshove.act,
            s.three_bet.opp,
            s.three_bet.act,
            s.f3b.opp,
            s.f3b.act,
            s.four_bet.opp,
            s.four_bet.act,
            s.ats.opp,
            s.ats.act,
            s.fsteal.opp,
            s.fsteal.act,
            s.rsteal.act,
            s.saw_flop,
            s.saw_turn,
            s.saw_river,
            s.went_sd,
            s.won_sd,
            s.won_hand,
            s.cbf.opp,
            s.cbf.act,
            s.cbt.opp,
            s.cbt.act,
            s.fcbf.opp,
            s.fcbf.act,
            s.postflop_counts.bets,
            s.postflop_counts.raises,
            s.postflop_counts.calls,
            s.postflop_counts.folds,
            s.postflop_counts.checks,
            s.allin_ev_diff_chips,
            hand_id,
            player_id,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gr_parser_api::Room;
    use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

    use super::*;
    use crate::migrate::run_migrations;
    use crate::repo::{insert_hands, HandInsert};

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn obelisk_hands() -> Vec<(String, gr_core::HandRecord)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
        );
        let text = std::fs::read_to_string(path).expect("real OBELISK hands should be readable");
        let (blocks, _offset) = split_hand_blocks(&text);
        blocks
            .into_iter()
            .map(|block| {
                let hand = WinamaxParser::parse_hand(block).expect("fixture hand should parse");
                (block.to_string(), hand)
            })
            .collect()
    }

    /// Simule une main importee avant M6-3 : `net_chips`/`net_bb` mis a NULL
    /// juste apres l'import normal (qui les remplit deja depuis cette
    /// story), pour tester le retro-remplissage independamment de
    /// l'insertion elle-meme.
    fn clear_net_chips(conn: &Connection) {
        conn.execute(
            "UPDATE hand_players SET net_chips = NULL, net_bb = NULL",
            [],
        )
        .expect("clearing net_chips should succeed");
    }

    #[test]
    fn backfill_is_a_no_op_when_nothing_needs_it() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insertion should succeed");

        let processed = backfill_net_chips(&mut conn).expect("backfill should succeed");
        assert_eq!(processed, 0, "l'import normal calcule deja net_chips");
    }

    #[test]
    fn backfill_recomputes_net_chips_matching_what_a_fresh_import_would_produce() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insertion should succeed");

        let before: Vec<(i64, i64, Option<i64>)> = {
            let mut stmt = conn
                .prepare("SELECT hand_id, player_id, net_chips FROM hand_players ORDER BY hand_id, player_id")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };

        clear_net_chips(&conn);
        let processed = backfill_net_chips(&mut conn).expect("backfill should succeed");
        assert_eq!(processed, hands.len());

        let after: Vec<(i64, i64, Option<i64>)> = {
            let mut stmt = conn
                .prepare("SELECT hand_id, player_id, net_chips FROM hand_players ORDER BY hand_id, player_id")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };

        assert_eq!(
            before, after,
            "le retro-remplissage doit reproduire exactement le calcul de l'import normal"
        );
    }

    /// Simule une main importee avant M7-7 : `hand_class` de la ligne Hero
    /// mis a NULL juste apres l'import normal (qui le remplit deja depuis
    /// cette story), pour tester le retro-remplissage independamment de
    /// l'insertion elle-meme.
    fn clear_hand_class(conn: &Connection) {
        conn.execute(
            "UPDATE hand_players SET hand_class = NULL WHERE is_hero = 1",
            [],
        )
        .expect("clearing hand_class should succeed");
    }

    #[test]
    fn hand_class_backfill_is_a_no_op_when_nothing_needs_it() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insertion should succeed");

        let processed = backfill_hand_class(&mut conn).expect("backfill should succeed");
        assert_eq!(processed, 0, "l'import normal calcule deja hand_class");
    }

    #[test]
    fn hand_class_backfill_recomputes_matching_what_a_fresh_import_would_produce() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insertion should succeed");

        let before: Vec<(i64, Option<String>)> = {
            let mut stmt = conn
                .prepare("SELECT hand_id, hand_class FROM hand_players WHERE is_hero = 1 ORDER BY hand_id")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert!(
            before.iter().any(|(_, class)| class.is_some()),
            "au moins une main du corpus devrait avoir une hand_class Hero calculee"
        );

        clear_hand_class(&conn);
        let processed = backfill_hand_class(&mut conn).expect("backfill should succeed");
        assert_eq!(processed, hands.len());

        let after: Vec<(i64, Option<String>)> = {
            let mut stmt = conn
                .prepare("SELECT hand_id, hand_class FROM hand_players WHERE is_hero = 1 ORDER BY hand_id")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };

        assert_eq!(
            before, after,
            "le retro-remplissage doit reproduire exactement le calcul de l'import normal"
        );
    }

    /// Simule une main importee avant M4-3/M4-4/M5-4 (30/09, le gap trouve
    /// via `just dev` sur les vraies mains de Frederic importees le 28/09) :
    /// remet a NULL les colonnes de `SeatStats` juste apres l'import normal
    /// (qui les remplit deja depuis ces stories), pour tester le
    /// retro-remplissage independamment de l'insertion elle-meme.
    fn clear_seat_stats(conn: &Connection) {
        conn.execute(
            "UPDATE hand_players SET
                position = NULL, position_group = NULL, stack_bb = NULL, eff_stack_bb = NULL,
                depth_bucket = NULL, eff_depth_bucket = NULL, preflop_line = NULL,
                vpip_opp = NULL, vpip = NULL, pfr = NULL,
                rfi_opp = NULL, rfi = NULL, limp = NULL, oshove = NULL,
                tb_opp = NULL, tb = NULL, f3b_opp = NULL, f3b = NULL, fb_opp = NULL, fb = NULL,
                ats_opp = NULL, ats = NULL, fsteal_opp = NULL, fsteal = NULL, rsteal = NULL,
                saw_flop = NULL, saw_turn = NULL, saw_river = NULL,
                went_sd = NULL, won_sd = NULL, won_hand = NULL,
                cbf_opp = NULL, cbf = NULL, cbt_opp = NULL, cbt = NULL, fcbf_opp = NULL, fcbf = NULL,
                pf_bets = NULL, pf_raises = NULL, pf_calls = NULL, pf_folds = NULL, pf_checks = NULL,
                allin_ev_diff_chips = NULL",
            [],
        )
        .expect("clearing seat stats should succeed");
    }

    type SeatStatsSnapshot = (
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<f64>,
    );

    fn seat_stats_snapshot(conn: &Connection) -> Vec<SeatStatsSnapshot> {
        let mut stmt = conn
            .prepare(
                "SELECT hand_id, player_id, position, position_group, vpip_opp, vpip,
                        preflop_line, allin_ev_diff_chips
                 FROM hand_players ORDER BY hand_id, player_id",
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
    }

    /// Main synthetique avec uniquement des sieges distribues (`dealt_in:
    /// true` partout) : contrairement a `obelisk_hands()`, dont le corpus
    /// reel contient 7 sieges "assis mais non distribues" par fichier
    /// (`docs/formats/winamax.md` §4, exclus de la main par construction —
    /// `position_group` y reste `NULL` a vie, meme fraichement importe),
    /// cette main garantit que rien ne devrait avoir besoin d'un
    /// retro-remplissage juste apres son import.
    fn fully_dealt_hand() -> gr_core::HandRecord {
        use gr_core::{ActionKind, ActionRecord, Chips, SeatInfo, Street};
        gr_core::HandRecord {
            room_hand_id: "#seat-stats-1".to_string(),
            tournament_name: "T".to_string(),
            tournament_room_id: "1".to_string(),
            table_name: "T(1)#1".to_string(),
            table_max_seats: 3,
            button_seat: 1,
            level: 1,
            sb: Chips::from_i64(10),
            bb: Chips::from_i64(20),
            ante: Chips::ZERO,
            played_at: 0,
            seats: vec![
                SeatInfo {
                    seat: 1,
                    pseudo: "Hero".to_string(),
                    starting_stack: Chips::from_i64(1000),
                    bounty: None,
                    dealt_in: true,
                },
                SeatInfo {
                    seat: 2,
                    pseudo: "V1".to_string(),
                    starting_stack: Chips::from_i64(1000),
                    bounty: None,
                    dealt_in: true,
                },
                SeatInfo {
                    seat: 3,
                    pseudo: "V2".to_string(),
                    starting_stack: Chips::from_i64(1000),
                    bounty: None,
                    dealt_in: true,
                },
            ],
            actions: vec![
                ActionRecord {
                    street: Street::Preflop,
                    pseudo: "V1".to_string(),
                    kind: ActionKind::PostSmallBlind,
                    amount: Some(Chips::from_i64(10)),
                    to_amount: None,
                    is_all_in: false,
                    pot: None,
                    shown_cards: None,
                    shown_label: None,
                },
                ActionRecord {
                    street: Street::Preflop,
                    pseudo: "V2".to_string(),
                    kind: ActionKind::PostBigBlind,
                    amount: Some(Chips::from_i64(20)),
                    to_amount: None,
                    is_all_in: false,
                    pot: None,
                    shown_cards: None,
                    shown_label: None,
                },
                ActionRecord {
                    street: Street::Preflop,
                    pseudo: "Hero".to_string(),
                    kind: ActionKind::Fold,
                    amount: None,
                    to_amount: None,
                    is_all_in: false,
                    pot: None,
                    shown_cards: None,
                    shown_label: None,
                },
                ActionRecord {
                    street: Street::Preflop,
                    pseudo: "V1".to_string(),
                    kind: ActionKind::Fold,
                    amount: None,
                    to_amount: None,
                    is_all_in: false,
                    pot: None,
                    shown_cards: None,
                    shown_label: None,
                },
            ],
            board: vec![],
            pots: vec![],
            total_pot: Chips::ZERO,
            rake: Chips::ZERO,
            uncalled_excess: Chips::ZERO,
            hero_pseudo: Some("Hero".to_string()),
            hero_cards: None,
            parser_version: "0.0.1".to_string(),
        }
    }

    #[test]
    fn seat_stats_backfill_is_a_no_op_when_nothing_needs_it() {
        let mut conn = migrated_connection();
        let hand = fully_dealt_hand();
        insert_hands(
            &mut conn,
            Room::Winamax,
            &[HandInsert {
                hand: &hand,
                raw_text: "irrelevant",
            }],
        )
        .expect("insertion should succeed");

        let processed = backfill_seat_stats(&mut conn).expect("backfill should succeed");
        assert_eq!(
            processed, 0,
            "l'import normal calcule deja les stats de siege"
        );
    }

    #[test]
    fn seat_stats_backfill_recomputes_matching_what_a_fresh_import_would_produce_for_every_seat() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insertion should succeed");

        let before = seat_stats_snapshot(&conn);
        assert!(
            before.iter().any(|(_, _, position, ..)| position.is_some()),
            "au moins un siege du corpus devrait avoir une position calculee"
        );

        clear_seat_stats(&conn);
        let processed = backfill_seat_stats(&mut conn).expect("backfill should succeed");
        assert_eq!(processed, hands.len());

        let after = seat_stats_snapshot(&conn);
        assert_eq!(
            before, after,
            "le retro-remplissage doit reproduire exactement le calcul de l'import normal, pour tous les sieges (pas seulement Hero)"
        );
    }
}
