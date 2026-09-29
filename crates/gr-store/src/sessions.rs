//! Regroupement des mains d'Hero en sessions (M3-4, PRD §9.3/H3) : un
//! recalcul integral a chaque import (masse ou watcher) plutot qu'une mise a
//! jour incrementale. Plus simple, et sans cout mesurable meme sur un corpus
//! de plusieurs dizaines de milliers de mains (tri sur l'index deja present
//! `ix_hands_played_at`). Garantit "recalcul correct lors d'un import hors
//! ordre" (BACKLOG M3-4) par construction : le regroupement ne depend que de
//! l'ordre chronologique final des mains, jamais de leur ordre d'insertion.
//!
//! `hands.hero_player_id` identifie le siege d'Hero sur chaque main (Winamax
//! etiquette toujours son propre siege "Hero", quel que soit le pseudo,
//! docs/formats/winamax.md), mais une main n'appartient a un profil Hero que
//! si ce siege est rattache a ce profil via `hero_accounts` (M3-5, D19) :
//! chaque profil est recalcule independamment, avec son propre decoupage en
//! sessions (des mains rattachees a un autre profil n'y comptent pas).

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

/// Regroupe les mains de **chaque** profil Hero en sessions, avec un seuil
/// d'interruption de `settings.session_gap_minutes` minutes (30 par defaut,
/// PRD §9.3/H3 ; "plus de N minutes" -> un ecart de N minutes pile
/// n'interrompt pas la session). Renvoie le nombre total de sessions creees,
/// tous profils confondus. Sans effet si aucun profil Hero n'existe encore
/// (avant la fin de l'assistant M3-1).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn recompute_hero_sessions(conn: &mut Connection) -> Result<usize, StoreError> {
    let gap_minutes = hero::get_setting(conn, SESSION_GAP_MINUTES_SETTING_KEY)?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_SESSION_GAP_MINUTES);
    let gap_ms = gap_minutes * 60_000;

    let profile_ids = all_hero_profile_ids(conn)?;

    let tx = conn.transaction()?;
    let mut sessions_created = 0usize;
    for hero_profile_id in profile_ids {
        sessions_created += recompute_for_profile(&tx, hero_profile_id, gap_ms)?;
    }
    tx.commit()?;
    Ok(sessions_created)
}

fn recompute_for_profile(
    tx: &Connection,
    hero_profile_id: i64,
    gap_ms: i64,
) -> Result<usize, StoreError> {
    let hands: Vec<HeroHand> = {
        let mut stmt = tx.prepare(
            "SELECT h.id, h.played_at, h.tournament_id, h.table_name FROM hands h
             WHERE h.hero_player_id IN (
               SELECT player_id FROM hero_accounts WHERE profile_id = ?1
             )
             ORDER BY h.played_at, h.id",
        )?;
        let rows = stmt.query_map([hero_profile_id], |row| {
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
        "UPDATE hands SET session_id = NULL WHERE hero_player_id IN (
           SELECT player_id FROM hero_accounts WHERE profile_id = ?1
         )",
        [hero_profile_id],
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
                flush_session(tx, hero_profile_id, &acc)?;
                sessions_created += 1;
            }
        } else if let Some(acc) = current.as_mut() {
            acc.absorb(hand);
        }
    }
    if let Some(acc) = current.take() {
        flush_session(tx, hero_profile_id, &acc)?;
        sessions_created += 1;
    }

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

/// La derniere session (par `started_at`) du profil Hero `hero_profile_id`,
/// avec les tournois qu'elle a touches (PRD §13.1, "derniere session" :
/// duree, tournois, profit, meilleurs et pires tournois — le profit et le
/// classement des tournois restent a calculer par l'appelant via
/// `gr-analytics`, cf. `hands.session_id` deja rattache par
/// [`recompute_hero_sessions`]). `None` si ce profil n'a encore aucune
/// session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSessionRow {
    pub started_at: i64,
    pub ended_at: i64,
    pub hands: i64,
    pub tournaments: i64,
    pub max_tables: i64,
    pub tournament_ids: Vec<i64>,
}

/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn latest_session(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Option<LastSessionRow>, StoreError> {
    let Some((session_id, started_at, ended_at, hands, tournaments, max_tables)) = conn
        .query_row(
            "SELECT id, started_at, ended_at, hands, tournaments, max_tables
             FROM sessions WHERE hero_profile_id = ?1
             ORDER BY started_at DESC LIMIT 1",
            [hero_profile_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };

    let mut stmt = conn.prepare(
        "SELECT DISTINCT tournament_id FROM hands
         WHERE session_id = ?1 AND tournament_id IS NOT NULL",
    )?;
    let tournament_ids = stmt
        .query_map([session_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<i64>>>()?;

    Ok(Some(LastSessionRow {
        started_at,
        ended_at,
        hands,
        tournaments,
        max_tables,
        tournament_ids,
    }))
}

/// Somme des durees de session (`ended_at - started_at`, ms) du profil Hero
/// `hero_profile_id` dont `started_at` tombe dans `[since_ms, until_ms)`
/// (`None` = pas de borne). PRD §13.1, KPI "$/h" (M6-2) : une session est
/// comptee entierement dans la periode ou elle a commence, pas repartie au
/// prorata si elle la chevauche — approximation documentee, dans le meme
/// esprit que la simplification EV all-in de M5-4 (rare en pratique, une
/// session depasse tres rarement une borne de periode de 30 jours).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn total_session_ms_in_range(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<i64, StoreError> {
    conn.query_row(
        "SELECT COALESCE(SUM(ended_at - started_at), 0) FROM sessions
         WHERE hero_profile_id = ?1
           AND (?2 IS NULL OR started_at >= ?2)
           AND (?3 IS NULL OR started_at < ?3)",
        params![hero_profile_id, since_ms, until_ms],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

fn all_hero_profile_ids(conn: &Connection) -> Result<Vec<i64>, StoreError> {
    let mut stmt = conn.prepare("SELECT id FROM hero_profiles")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
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

    /// Cree le siege "Hero" et le rattache immediatement au profil
    /// `profile_id` via `hero_accounts` (M3-5) : sans ce rattachement, les
    /// mains de ce siege ne seraient scopees dans aucun profil.
    fn insert_hero_player(conn: &Connection, room_id: i64, profile_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, 'Hero')",
            [room_id],
        )
        .unwrap();
        let player_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO hero_accounts (profile_id, player_id) VALUES (?1, ?2)",
            params![profile_id, player_id],
        )
        .unwrap();
        player_id
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
        // Pas de profil du tout ici (c'est ce que ce test verifie) : pas
        // d'appel a `insert_hero_player`, qui exigerait un `profile_id`
        // valide pour le rattachement `hero_accounts`.
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, 'Hero')",
            [room_id],
        )
        .unwrap();
        let hero_id = conn.last_insert_rowid();
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");

        let created = recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(created, 0);
        assert!(session_rows(&conn).is_empty());
    }

    #[test]
    fn hands_within_the_default_30_minute_gap_form_a_single_session() {
        let mut conn = migrated_connection();
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);

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
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);

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
        let profile_id = create_default_hero_profile(&conn);
        hero::set_setting(&conn, SESSION_GAP_MINUTES_SETTING_KEY, "10").unwrap();
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", 11 * MINUTE, Some(hero_id), None, "T1");

        recompute_hero_sessions(&mut conn).unwrap();
        assert_eq!(
            session_rows(&conn).len(),
            2,
            "seuil personnalise a 10 min : 11 min d'ecart doit couper"
        );
    }

    /// CA de M3-5 : "toutes les requetes Hero filtrent par `hero_accounts`
    /// du profil selectionne" — deux profils ont chacun leur propre decoupage
    /// en sessions, sans se melanger.
    #[test]
    fn two_hero_profiles_get_independent_sessions() {
        let mut conn = migrated_connection();
        let profile_a = create_default_hero_profile(&conn);
        conn.execute(
            "INSERT INTO hero_profiles (name, is_default) VALUES ('Profil B', 0)",
            [],
        )
        .unwrap();
        let profile_b = conn.last_insert_rowid();

        let room_id = insert_room(&conn);
        let hero_a = insert_hero_player(&conn, room_id, profile_a);
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, 'HeroB')",
            [room_id],
        )
        .unwrap();
        let hero_b = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO hero_accounts (profile_id, player_id) VALUES (?1, ?2)",
            params![profile_b, hero_b],
        )
        .unwrap();

        // Memes horodatages, deux profils distincts : chacun doit voir
        // exactement ses propres mains, pas celles de l'autre.
        insert_hand(&conn, room_id, "a1", 0, Some(hero_a), None, "T1");
        insert_hand(&conn, room_id, "b1", 0, Some(hero_b), None, "T1");
        insert_hand(&conn, room_id, "b2", MINUTE, Some(hero_b), None, "T1");

        recompute_hero_sessions(&mut conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT hero_profile_id, hands FROM sessions ORDER BY hero_profile_id")
            .unwrap();
        let rows: Vec<(i64, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows, vec![(profile_a, 1), (profile_b, 2)]);
    }

    #[test]
    fn latest_session_is_none_without_any_session_yet() {
        let conn = migrated_connection();
        assert_eq!(latest_session(&conn, 1).unwrap(), None);
    }

    #[test]
    fn latest_session_returns_the_most_recent_one_with_its_tournament_ids() {
        let mut conn = migrated_connection();
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);
        let tournament_1 = insert_tournament(&conn, room_id, "T1");
        let tournament_2 = insert_tournament(&conn, room_id, "T2");

        // Deux sessions (60 min d'ecart) : la plus ancienne touche T1, la
        // plus recente touche T1 et T2.
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
            60 * MINUTE,
            Some(hero_id),
            Some(tournament_1),
            "T1",
        );
        insert_hand(
            &conn,
            room_id,
            "h3",
            61 * MINUTE,
            Some(hero_id),
            Some(tournament_2),
            "T2",
        );
        recompute_hero_sessions(&mut conn).unwrap();

        let latest = latest_session(&conn, profile_id)
            .unwrap()
            .expect("a session should exist");
        assert_eq!(latest.started_at, 60 * MINUTE);
        assert_eq!(latest.ended_at, 61 * MINUTE);
        assert_eq!(latest.hands, 2);
        let mut tournament_ids = latest.tournament_ids;
        tournament_ids.sort_unstable();
        let mut expected = vec![tournament_1, tournament_2];
        expected.sort_unstable();
        assert_eq!(tournament_ids, expected);
    }

    #[test]
    fn total_session_ms_in_range_only_counts_sessions_started_in_the_window() {
        let mut conn = migrated_connection();
        let profile_id = create_default_hero_profile(&conn);
        let room_id = insert_room(&conn);
        let hero_id = insert_hero_player(&conn, room_id, profile_id);

        // Session 1 : [0, 10min), demarre avant la fenetre -> exclue.
        insert_hand(&conn, room_id, "h1", 0, Some(hero_id), None, "T1");
        insert_hand(&conn, room_id, "h2", 10 * MINUTE, Some(hero_id), None, "T1");
        // Ecart de 60 min pour forcer une nouvelle session.
        // Session 2 : [100min, 105min), demarre dans la fenetre -> comptee.
        insert_hand(
            &conn,
            room_id,
            "h3",
            100 * MINUTE,
            Some(hero_id),
            None,
            "T1",
        );
        insert_hand(
            &conn,
            room_id,
            "h4",
            105 * MINUTE,
            Some(hero_id),
            None,
            "T1",
        );
        recompute_hero_sessions(&mut conn).unwrap();

        let total = total_session_ms_in_range(&conn, profile_id, Some(50 * MINUTE), None).unwrap();
        assert_eq!(total, 5 * MINUTE);

        let unbounded = total_session_ms_in_range(&conn, profile_id, None, None).unwrap();
        assert_eq!(unbounded, 10 * MINUTE + 5 * MINUTE);
    }
}
