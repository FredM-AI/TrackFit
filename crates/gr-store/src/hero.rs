//! Profils Hero et pseudos rattaches (M3-1/M3-5, D19, PRD §15).

use gr_parser_api::Room;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;
use crate::repo::{get_or_create_player_id, get_or_create_room};

/// Un profil Hero (PRD D19 : regroupe un ou plusieurs pseudos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroProfileRow {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
}

/// Cree un profil Hero. Si `is_default` est vrai, les autres profils
/// existants sont retrogrades (un seul profil par defaut a la fois).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue (ex. `name` deja pris).
pub(crate) fn create_hero_profile(
    conn: &Connection,
    name: &str,
    is_default: bool,
) -> Result<i64, StoreError> {
    if is_default {
        conn.execute("UPDATE hero_profiles SET is_default = 0", [])?;
    }
    conn.execute(
        "INSERT INTO hero_profiles (name, is_default) VALUES (?1, ?2)",
        params![name, is_default],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Rattache le pseudo `screen_name` (cree s'il est inconnu) au profil
/// `profile_id`.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn link_hero_account(
    conn: &Connection,
    profile_id: i64,
    room: Room,
    screen_name: &str,
) -> Result<(), StoreError> {
    // `conn` n'est pas une transaction ici (contrairement a `repo`/`summary_repo`
    // qui l'utilisent depuis une `Transaction`) : `get_or_create_room`/
    // `get_or_create_player_id` acceptent les deux car ils ne prennent qu'une
    // reference a `Connection`.
    let room_id = get_or_create_room(conn, room)?;
    let player_id = get_or_create_player_id(conn, room_id, screen_name)?;
    conn.execute(
        "INSERT INTO hero_accounts (profile_id, player_id) VALUES (?1, ?2)
         ON CONFLICT(profile_id, player_id) DO NOTHING",
        params![profile_id, player_id],
    )?;
    Ok(())
}

/// Liste tous les profils Hero, le profil par defaut d'abord.
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn list_hero_profiles(conn: &Connection) -> Result<Vec<HeroProfileRow>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT id, name, is_default FROM hero_profiles ORDER BY is_default DESC, name")?;
    let rows = stmt.query_map([], |row| {
        Ok(HeroProfileRow {
            id: row.get(0)?,
            name: row.get(1)?,
            is_default: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Lit une valeur de `settings` (M3-1 : flag "premier lancement termine").
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, StoreError> {
    conn.query_row(
        "SELECT value_json FROM settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

/// Ecrit (ou remplace) une valeur de `settings`.
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn set_setting(
    conn: &Connection,
    key: &str,
    value_json: &str,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO settings (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        params![key, value_json],
    )?;
    Ok(())
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

    #[test]
    fn creates_a_profile_and_links_two_pseudos() {
        let conn = migrated_connection();
        let profile_id = create_hero_profile(&conn, "Frederic", true).expect("create profile");
        link_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").expect("link first pseudo");
        link_hero_account(&conn, profile_id, Room::Winamax, "FredMTT").expect("link second pseudo");

        let account_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hero_accounts WHERE profile_id = ?1",
                [profile_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(account_count, 2);
    }

    #[test]
    fn linking_the_same_pseudo_twice_does_not_duplicate_the_account_row() {
        let conn = migrated_connection();
        let profile_id = create_hero_profile(&conn, "Frederic", true).unwrap();
        link_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").unwrap();
        link_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").unwrap();

        let account_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM hero_accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(account_count, 1);
    }

    #[test]
    fn only_one_profile_stays_default() {
        let conn = migrated_connection();
        let first = create_hero_profile(&conn, "Profil A", true).unwrap();
        let second = create_hero_profile(&conn, "Profil B", true).unwrap();

        let profiles = list_hero_profiles(&conn).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(
            profiles[0].id, second,
            "le profil par defaut est liste en premier"
        );
        assert!(profiles[0].is_default);

        let first_is_default: bool = conn
            .query_row(
                "SELECT is_default FROM hero_profiles WHERE id = ?1",
                [first],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!first_is_default, "un seul profil par defaut a la fois");
    }

    #[test]
    fn settings_roundtrip_and_update_in_place() {
        let conn = migrated_connection();
        assert_eq!(get_setting(&conn, "first_launch_completed").unwrap(), None);

        set_setting(&conn, "first_launch_completed", "true").unwrap();
        assert_eq!(
            get_setting(&conn, "first_launch_completed").unwrap(),
            Some("true".to_string())
        );

        set_setting(&conn, "first_launch_completed", "false").unwrap();
        assert_eq!(
            get_setting(&conn, "first_launch_completed").unwrap(),
            Some("false".to_string())
        );
    }
}
