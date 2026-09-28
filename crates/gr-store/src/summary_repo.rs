//! Rattachement des summaries aux tournois (M2-6, PRD §8.5) : buy-in exact,
//! statut `PROVISIONAL` -> `COMPLETE`, `tournament_entries` +
//! `tournament_bullets` (une ligne par entree, re-entries comprises).
//! Fonctionne quel que soit l'ordre d'import par rapport aux mains : le
//! tournoi est cree "provisoire" au besoin, comme depuis une main (M2-2).

use gr_core::{Money, TournamentSummary};
use gr_parser_api::Room;
use rusqlite::{params, Connection, Transaction};

use crate::error::StoreError;
use crate::repo::{get_or_create_player_id, get_or_create_room, get_or_create_tournament};

/// Bilan du rattachement d'un summary (M2-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachSummaryReport {
    pub tournament_id: i64,
    pub entries_count: usize,
}

/// Rattache un summary deja parse a son tournoi.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn attach_summary(
    conn: &mut Connection,
    room: Room,
    summary: &TournamentSummary,
) -> Result<AttachSummaryReport, StoreError> {
    let tx = conn.transaction()?;
    let room_id = get_or_create_room(&tx, room)?;
    let tournament_id = get_or_create_tournament(
        &tx,
        room_id,
        &summary.room_tournament_id,
        &summary.name,
        None,
    )?;
    let hero_player_id = get_or_create_player_id(&tx, room_id, &summary.hero_pseudo)?;

    update_tournament_from_summary(&tx, tournament_id, summary)?;
    let entry_id = upsert_tournament_entry(&tx, tournament_id, hero_player_id, summary)?;
    replace_tournament_bullets(&tx, entry_id, summary)?;

    tx.commit()?;
    Ok(AttachSummaryReport {
        tournament_id,
        entries_count: summary.bullets.len(),
    })
}

fn update_tournament_from_summary(
    tx: &Transaction<'_>,
    tournament_id: i64,
    summary: &TournamentSummary,
) -> Result<(), StoreError> {
    let is_freeroll = summary.buyin_prize == Money::ZERO
        && summary.buyin_bounty == Money::ZERO
        && summary.buyin_fee == Money::ZERO;
    // Le champ `Type` du summary ne distingue pas KO/PKO/Mystery/Space (les
    // 4 valent "knockout", docs/formats/winamax.md §5.2) : on ne peut donc
    // fiabiliser que la presence d'un bounty, pas le sous-type precis
    // (R-FORMAT : pas d'heuristique non documentee sur le nom du tournoi).
    let ko_type = if summary.tournament_type == "knockout" {
        "KO"
    } else {
        "NONE"
    };
    let entrants = summary
        .bullets
        .last()
        .map(|b| i64::from(b.registered_snapshot));

    tx.execute(
        "UPDATE tournaments SET
            name = ?1,
            buyin_prize_cents = ?2,
            buyin_bounty_cents = ?3,
            buyin_fee_cents = ?4,
            ko_type = ?5,
            is_freeroll = ?6,
            speed = ?7,
            entrants = ?8,
            status = 'COMPLETE'
         WHERE id = ?9",
        params![
            summary.name,
            summary.buyin_prize.cents(),
            summary.buyin_bounty.cents(),
            summary.buyin_fee.cents(),
            ko_type,
            is_freeroll,
            summary.speed,
            entrants,
            tournament_id,
        ],
    )?;
    Ok(())
}

/// `entries_count` = nombre de blocs ; gains = somme de tous les blocs ;
/// place retenue = celle du dernier bloc (PRD §8.5).
fn upsert_tournament_entry(
    tx: &Transaction<'_>,
    tournament_id: i64,
    player_id: i64,
    summary: &TournamentSummary,
) -> Result<i64, StoreError> {
    let entries_count = i64::try_from(summary.bullets.len()).unwrap_or(0);
    let finish_position = summary
        .bullets
        .last()
        .and_then(|b| b.finish_position)
        .map(i64::from);
    let prize_cents: i64 = summary.bullets.iter().map(|b| b.prize.cents()).sum();
    let bounty_cents: i64 = summary.bullets.iter().map(|b| b.bounty.cents()).sum();

    tx.execute(
        "INSERT INTO tournament_entries (
            tournament_id, player_id, entries_count, finish_position, prize_cents, bounty_cents
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(tournament_id, player_id) DO UPDATE SET
            entries_count = excluded.entries_count,
            finish_position = excluded.finish_position,
            prize_cents = excluded.prize_cents,
            bounty_cents = excluded.bounty_cents",
        params![
            tournament_id,
            player_id,
            entries_count,
            finish_position,
            prize_cents,
            bounty_cents
        ],
    )?;
    Ok(tx.query_row(
        "SELECT id FROM tournament_entries WHERE tournament_id = ?1 AND player_id = ?2",
        params![tournament_id, player_id],
        |row| row.get(0),
    )?)
}

/// Remplace entierement les bullets de cette entree (idempotent : reattacher
/// le meme summary ne duplique jamais les lignes).
fn replace_tournament_bullets(
    tx: &Transaction<'_>,
    entry_id: i64,
    summary: &TournamentSummary,
) -> Result<(), StoreError> {
    tx.execute(
        "DELETE FROM tournament_bullets WHERE entry_id = ?1",
        [entry_id],
    )?;
    for bullet in &summary.bullets {
        tx.execute(
            "INSERT INTO tournament_bullets (
                entry_id, entry_no, late_reg_bust, finish_position, played_seconds,
                prize_cents, bounty_cents, registered_snapshot
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry_id,
                bullet.entry_no,
                bullet.late_reg_bust,
                bullet.finish_position.map(i64::from),
                bullet.played_seconds,
                bullet.prize.cents(),
                bullet.bounty.cents(),
                bullet.registered_snapshot,
            ],
        )?;
    }
    Ok(())
}

/// Marque `INCOMPLETE` tout tournoi encore `PROVISIONAL` dont la derniere
/// main connue remonte a plus de `stale_after_hours` heures avant `now_ms`
/// (PRD §8.5, BACKLOG M2-6). Renvoie le nombre de tournois marques.
/// `now_ms` est un parametre (plutot que l'horloge murale reelle) pour
/// rester testable de maniere deterministe.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn mark_stale_provisional_tournaments_incomplete(
    conn: &Connection,
    now_ms: i64,
    stale_after_hours: i64,
) -> Result<usize, StoreError> {
    let threshold_ms = stale_after_hours.saturating_mul(3_600_000);
    let cutoff = now_ms.saturating_sub(threshold_ms);
    let changed = conn.execute(
        "UPDATE tournaments
         SET status = 'INCOMPLETE'
         WHERE status = 'PROVISIONAL'
           AND (SELECT MAX(h.played_at) FROM hands h WHERE h.tournament_id = tournaments.id) < ?1",
        params![cutoff],
    )?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gr_parser_winamax::WinamaxParser;

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

    fn obelisk_summary() -> TournamentSummary {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit_summary.txt",
        );
        let text = std::fs::read_to_string(path).expect("real OBELISK summary should be readable");
        WinamaxParser::parse_summary(&text).expect("summary fixture should parse")
    }

    fn obelisk_hands() -> Vec<(String, gr_core::HandRecord)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
        );
        let text = std::fs::read_to_string(path).expect("real OBELISK hands should be readable");
        let (blocks, _offset) = gr_parser_winamax::split_hand_blocks(&text);
        blocks
            .into_iter()
            .map(|block| {
                let hand = WinamaxParser::parse_hand(block).expect("fixture hand should parse");
                (block.to_string(), hand)
            })
            .collect()
    }

    fn assert_obelisk_entry_and_bullets(conn: &Connection, tournament_id: i64) {
        let status: String = conn
            .query_row(
                "SELECT status FROM tournaments WHERE id = ?1",
                [tournament_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "COMPLETE");

        let (entries_count, finish_position, prize_cents, bounty_cents): (i64, i64, i64, i64) =
            conn.query_row(
                "SELECT entries_count, finish_position, prize_cents, bounty_cents
                 FROM tournament_entries WHERE tournament_id = ?1",
                [tournament_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(entries_count, 2, "OBELISK a 2 entrees (re-entry)");
        assert_eq!(finish_position, 7);
        assert_eq!(prize_cents, 3479);
        assert_eq!(bounty_cents, 1740);

        let bullet_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tournament_bullets b
                 JOIN tournament_entries e ON e.id = b.entry_id
                 WHERE e.tournament_id = ?1",
                [tournament_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bullet_count, 2);
    }

    #[test]
    fn attaching_a_summary_after_its_hands_completes_the_provisional_tournament() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insert hands first");

        let status_before: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status_before, "PROVISIONAL");

        let report = attach_summary(&mut conn, Room::Winamax, &obelisk_summary())
            .expect("attach summary should succeed");

        assert_eq!(report.entries_count, 2);
        assert_obelisk_entry_and_bullets(&conn, report.tournament_id);
    }

    /// CA de M2-6 : le rattachement fonctionne quel que soit l'ordre d'import.
    #[test]
    fn attaching_a_summary_before_any_hand_creates_a_complete_tournament_directly() {
        let mut conn = migrated_connection();

        let report = attach_summary(&mut conn, Room::Winamax, &obelisk_summary())
            .expect("attach summary should succeed even with no prior hand");

        assert_obelisk_entry_and_bullets(&conn, report.tournament_id);

        // Les mains arrivent ensuite et doivent rejoindre le meme tournoi.
        let hands = obelisk_hands();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insert hands afterwards");

        let tournament_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            tournament_count, 1,
            "les mains doivent rejoindre le tournoi cree par le summary, pas en creer un second"
        );
        let status_after: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            status_after, "COMPLETE",
            "l'arrivee des mains ne doit pas retrograder un tournoi deja complete"
        );
    }

    #[test]
    fn reattaching_the_same_summary_does_not_duplicate_entries_or_bullets() {
        let mut conn = migrated_connection();
        let summary = obelisk_summary();

        attach_summary(&mut conn, Room::Winamax, &summary).expect("first attach");
        let report = attach_summary(&mut conn, Room::Winamax, &summary).expect("second attach");

        let entry_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tournament_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(entry_count, 1);
        assert_obelisk_entry_and_bullets(&conn, report.tournament_id);
    }

    #[test]
    fn marks_a_stale_provisional_tournament_incomplete_after_24h_without_a_new_hand() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let last_played_at = hands.iter().map(|(_, h)| h.played_at).max().unwrap();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insert hands");

        let just_under_24h = last_played_at + 23 * 3_600_000;
        let changed =
            mark_stale_provisional_tournaments_incomplete(&conn, just_under_24h, 24).unwrap();
        assert_eq!(changed, 0);
        let status: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "PROVISIONAL");

        let just_over_24h = last_played_at + 25 * 3_600_000;
        let changed =
            mark_stale_provisional_tournaments_incomplete(&conn, just_over_24h, 24).unwrap();
        assert_eq!(changed, 1);
        let status: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "INCOMPLETE");
    }

    #[test]
    fn does_not_downgrade_a_complete_tournament_even_when_stale() {
        let mut conn = migrated_connection();
        let hands = obelisk_hands();
        let last_played_at = hands.iter().map(|(_, h)| h.played_at).max().unwrap();
        let inserts: Vec<HandInsert<'_>> = hands
            .iter()
            .map(|(raw_text, hand)| HandInsert { hand, raw_text })
            .collect();
        insert_hands(&mut conn, Room::Winamax, &inserts).expect("insert hands");
        attach_summary(&mut conn, Room::Winamax, &obelisk_summary()).expect("attach summary");

        let far_future = last_played_at + 365 * 24 * 3_600_000;
        let changed = mark_stale_provisional_tournaments_incomplete(&conn, far_future, 24).unwrap();
        assert_eq!(changed, 0);
        let status: String = conn
            .query_row("SELECT status FROM tournaments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "COMPLETE");
    }
}
