//! Regroupement des mains d'Hero en sessions (M3-4, PRD §9.3/H3) : un
//! recalcul integral a chaque import (masse ou watcher) plutot qu'une mise a
//! jour incrementale. Plus simple, et sans cout mesurable meme sur un corpus
//! de plusieurs dizaines de milliers de mains (tri sur l'index deja present
//! `ix_hands_played_at`). Garantit "recalcul correct lors d'un import hors
//! ordre" (BACKLOG M3-4) par construction : le regroupement ne depend que de
//! l'ordre chronologique final des mains, jamais de leur ordre d'insertion.
//!
//! `hands.hero_player_id` identifie deja le siege d'Hero sur chaque main
//! (Winamax etiquette toujours son propre siege "Hero", quel que soit le
//! pseudo, docs/formats/winamax.md) : aucune jointure via `hero_accounts`
//! n'est necessaire ici. Les sessions sont rattachees au profil Hero par
//! defaut ; M3-5 (profils multi-pseudos) n'est pas encore implemente et un
//! seul profil existe en pratique.

use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;
use crate::hero;

const SESSION_GAP_MINUTES_SETTING_KEY: &str = "session_gap_minutes";
const DEFAULT_SESSION_GAP_MINUTES: i64 = 30;
/// Taille de lot pour les `UPDATE hands SET session_id = ... WHERE id IN
/// (...)` (voir `flush_session`).
const SESSION_UPDATE_CHUNK_SIZE: usize = 500;

struct HeroHand {
    id: i64,
    played_at: i64,
    tournament_id: Option<i64>,
    table_name: Option<String>,
}

#[derive(Default)]
struct SessionAcc {
    started_at: i64,
    ended_at: i64,
    hand_ids: Vec<i64>,
    tournaments: HashSet<i64>,
    // Approximation documentee (BACKLOG M3-4) : nombre de tables distinctes
    // vues pendant la session, pas un vrai calcul de chevauchement temporel
    // (on ne connait que l'instant de chaque main, pas sa duree).
    tables: HashSet<(Option<i64>, String)>,
}

impl SessionAcc {
    fn start(hand: &HeroHand) -> Self {
        let mut acc = Self {
            started_at: hand.played_at,
            ended_at: hand.played_at,
            ..Default::default()
        };
        acc.absorb(hand);
        acc
    }

    fn absorb(&mut self, hand: &HeroHand) {
        self.ended_at = hand.played_at;
        self.hand_ids.push(hand.id);
        if let Some(tournament_id) = hand.tournament_id {
            self.tournaments.insert(tournament_id);
        }
        if let Some(table_name) = &hand.table_name {
            self.tables.insert((hand.tournament_id, table_name.clone()));
        }
    }
}

/// Regroupe toutes les mains d'Hero en sessions, avec un seuil
/// d'interruption de `settings.session_gap_minutes` minutes (30 par defaut,
/// PRD §9.3/H3 ; "plus de N minutes" -> un ecart de N minutes pile
/// n'interrompt pas la session). Renvoie le nombre de sessions creees.
/// Sans effet si aucun profil Hero n'existe encore (avant la fin de
/// l'assistant M3-1).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn recompute_hero_sessions(conn: &mut Connection) -> Result<usize, StoreError> {
    let Some(hero_profile_id) = default_hero_profile_id(conn)? else {
        return Ok(0);
    };
    let gap_minutes = hero::get_setting(conn, SESSION_GAP_MINUTES_SETTING_KEY)?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_SESSION_GAP_MINUTES);
    let gap_ms = gap_minutes * 60_000;

    let tx = conn.transaction()?;

    let hands: Vec<HeroHand> = {
        let mut stmt = tx.prepare(
            "SELECT id, played_at, tournament_id, table_name FROM hands
             WHERE hero_player_id IS NOT NULL ORDER BY played_at, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeroHand {
                id: row.get(0)?,
                played_at: row.get(1)?,
                tournament_id: row.get(2)?,
                table_name: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    // Ordre FK-safe : detacher les mains avant de supprimer les sessions
    // (`hands.session_id` n'a pas de ON DELETE CASCADE, foreign_keys = ON).
    tx.execute(
        "UPDATE hands SET session_id = NULL WHERE hero_player_id IS NOT NULL",
        [],
    )?;
    tx.execute(
        "DELETE FROM sessions WHERE hero_profile_id = ?1",
        [hero_profile_id],
    )?;

    let mut sessions_created = 0usize;
    let mut current: Option<SessionAcc> = None;
    for hand in &hands {
        let starts_new_session = match &current {
            None => true,
            Some(acc) => hand.played_at - acc.ended_at > gap_ms,
        };
        if starts_new_session {
            if let Some(acc) = current.replace(SessionAcc::start(hand)) {
                flush_session(&tx, hero_profile_id, &acc)?;
                sessions_created += 1;
            }
        } else if let Some(acc) = current.as_mut() {
            acc.absorb(hand);
        }
    }
    if let Some(acc) = current.take() {
        flush_session(&tx, hero_profile_id, &acc)?;
        sessions_created += 1;
    }

    tx.commit()?;
    Ok(sessions_created)
}

fn flush_session(
    conn: &Connection,
    hero_profile_id: i64,
    acc: &SessionAcc,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO sessions (hero_profile_id, started_at, ended_at, hands, tournaments, max_tables)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            hero_profile_id,
            acc.started_at,
            acc.ended_at,
            i64::try_from(acc.hand_ids.len()).unwrap_or(i64::MAX),
            i64::try_from(acc.tournaments.len()).unwrap_or(i64::MAX),
            i64::try_from(acc.tables.len()).unwrap_or(i64::MAX),
        ],
    )?;
    let session_id = conn.last_insert_rowid();
    // Un `UPDATE` par lot (`IN (...)`) plutot que par main : mesure a 100k
    // mains synthetiques (perf differee M2-3/M3-4), le cout ligne-par-ligne
    // faisait chuter le debit d'import de ~1430 a ~1330 mains/s.
    for chunk in acc.hand_ids.chunks(SESSION_UPDATE_CHUNK_SIZE) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!("UPDATE hands SET session_id = ? WHERE id IN ({placeholders})");
        let mut stmt = conn.prepare(&sql)?;
        let mut bound_params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(chunk.len() + 1);
        bound_params.push(&session_id);
        for hand_id in chunk {
            bound_params.push(hand_id);
        }
        stmt.execute(bound_params.as_slice())?;
    }
    Ok(())
}

fn default_hero_profile_id(conn: &Connection) -> Result<Option<i64>, StoreError> {
    conn.query_row(
        "SELECT id FROM hero_profiles WHERE is_default = 1 LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn create_default_hero_profile(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO hero_profiles (name, is_default) VALUES ('Hero', 1)",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_room(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO rooms (code, name) VALUES ('WINAMAX', 'Winamax')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_tournament(conn: &Connection, room_id: i64, room_tournament_id: &str) -> i64 {
        conn.execute(
            "INSERT INTO tournaments (room_id, room_tournament_id) VALUES (?1, ?2)",
            params![room_id, room_tournament_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_hero_player(conn: &Connection, room_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, 'Hero')",
            [room_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_hand(
        conn: &Connection,
        room_id: i64,
        room_hand_id: &str,
        played_at: i64,
        hero_player_id: Option<i64>,
        tournament_id: Option<i64>,
        table_name: &str,
    ) {
        conn.execute(
            "INSERT INTO hands (room_id, room_hand_id, played_at, hero_player_id, tournament_id, table_name, parser_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '0.0.1')",
            params![room_id, room_hand_id, played_at, hero_player_id, tournament_id, table_name],
        )
        .unwrap();
    }

    fn session_rows(conn: &Connection) -> Vec<(i64, i64, i64, i64, i64)> {
        let mut stmt = conn
            .prepare(
                "SELECT started_at, ended_at, hands, tournaments, max_tables
                 FROM sessions ORDER BY started_at",
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
    }

    const MINUTE: i64 = 60_000;

    #[test]
    fn does_nothing_when_no_hero_profile_exists_yet() {
        let mut conn = migrated_connection();
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");

        let created = recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(created, 0);
        assert!(session_rows(&conn).is_empty());
    }

    #[test]
    fn hands_within_the_default_30_minute_gap_form_a_single_session() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", 29 * MINUTE, Some(hero_id), None, "T1");

        let created = recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(created, 1);
        let rows = session_rows(&conn);
        assert_eq!(rows, vec![(0, 29 * MINUTE, 2, 0, 1)]);
    }

    #[test]
    fn a_gap_of_exactly_30_minutes_does_not_start_a_new_session() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", 30 * MINUTE, Some(hero_id), None, "T1");

        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(
            session_rows(&conn).len(),
            1,
            "\"plus de\" 30 min exactement ne casse pas la session"
        );
    }

    #[test]
    fn a_gap_of_more_than_30_minutes_starts_a_new_session() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(
            &conn,
            room_id,
            "h2",
            30 * MINUTE + 1,
            Some(hero_id),
            None,
            "T1",
        );

        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(session_rows(&conn).len(), 2);
    }

    #[test]
    fn tournaments_and_tables_are_counted_distinctly() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        let tournament_1 = insert_tournament(&conn, room_id, "T1");
        let tournament_2 = insert_tournament(&conn, room_id, "T2");
        // 2 tournois, 3 tables distinctes, 4 mains, une seule session.
        insert_hand(
            &conn,
            room_id,
            "h1",
            0,
            Some(hero_id),
            Some(tournament_1),
            "T1",
        );
        insert_hand(
            &conn,
            room_id,
            "h2",
            MINUTE,
            Some(hero_id),
            Some(tournament_1),
            "T2",
        );
        insert_hand(
            &conn,
            room_id,
            "h3",
            2 * MINUTE,
            Some(hero_id),
            Some(tournament_2),
            "T1",
        );
        insert_hand(
            &conn,
            room_id,
            "h4",
            3 * MINUTE,
            Some(hero_id),
            Some(tournament_1),
            "T1",
        );

        recompute_hero_sessions(&mut conn).unwrap();
        let rows = session_rows(&conn);
        assert_eq!(rows.len(), 1);
        let (_, _, hands, tournaments, max_tables) = rows[0];
        assert_eq!(hands, 4);
        assert_eq!(tournaments, 2);
        assert_eq!(
            max_tables, 3,
            "T1 du tournoi 1, T2 du tournoi 1, T1 du tournoi 2 : 3 tables distinctes"
        );
    }

    #[test]
    fn a_hand_ignored_by_hero_player_id_is_excluded() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", MINUTE, None, None, "T1");

        recompute_hero_sessions(&mut conn).unwrap();
        let rows = session_rows(&conn);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, 1, "seule la main avec un siege Hero compte");
    }

    #[test]
    fn recomputing_after_an_out_of_order_backfill_matches_the_chronological_result() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);

        // Import "temps reel" : les deux mains les plus recentes d'abord.
        insert_hand(&conn, room_id, "h2", 40 * MINUTE, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h3", 41 * MINUTE, Some(hero_id), None, "T1");
        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(session_rows(&conn).len(), 1);

        // Reparse/backfill : une main plus ancienne, hors de la fenetre de
        // 30 min de la session existante (nouvelle session isolee).
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        recompute_hero_sessions(&mut conn).unwrap();

        let rows = session_rows(&conn);
        assert_eq!(
            rows,
            vec![(0, 0, 1, 0, 1), (40 * MINUTE, 41 * MINUTE, 2, 0, 1)],
            "identique a un import h1, h2, h3 dans l'ordre chronologique"
        );
    }

    #[test]
    fn a_backfilled_hand_filling_the_gap_merges_two_sessions_into_one() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);

        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h3", 60 * MINUTE, Some(hero_id), None, "T1");
        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(
            session_rows(&conn).len(),
            2,
            "60 min d'ecart : deux sessions"
        );

        // La main manquante comble l'ecart : tout devient une seule session.
        insert_hand(&conn, room_id, "h2", 30 * MINUTE, Some(hero_id), None, "T1");
        recompute_hero_sessions(&mut conn).unwrap();

        let rows = session_rows(&conn);
        assert_eq!(rows, vec![(0, 60 * MINUTE, 3, 0, 1)]);
    }

    #[test]
    fn a_custom_gap_setting_is_respected() {
        let mut conn = migrated_connection();
        create_default_hero_profile(&conn);
        hero::set_setting(&conn, SESSION_GAP_MINUTES_SETTING_KEY, "10").unwrap();
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", 11 * MINUTE, Some(hero_id), None, "T1");

        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(
            session_rows(&conn).len(),
            2,
            "seuil personnalise a 10 min : 11 min d'ecart doit couper"
        );
    }
}
