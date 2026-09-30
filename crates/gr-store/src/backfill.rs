//! Retro-remplissage de colonnes `hand_players` prevues en base depuis
//! `0001_init.sql`/M2-1 mais calculees seulement a partir d'une story
//! ulterieure : `net_chips`/`net_bb` (M6-3) et `hand_class` (M7-7). Relit
//! `hand_raw` (seule source du detail des mains deja importees ; `PotResult`/
//! `hero_cards` ne vivent qu'en memoire pendant le parsing, jamais persistes
//! autrement) et reparse (Winamax uniquement, seule salle supportee en V1,
//! meme convention que `gr_ingest::reparse_import_error`).

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;

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
}
