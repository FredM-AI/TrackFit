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

/// Cree un profil Hero, ou reutilise le profil existant du meme nom
/// (idempotent : l'assistant de premier lancement peut etre rejoue, par ex.
/// apres une interruption entre la creation du profil et le marquage
/// "premier lancement termine", sans violer la contrainte `UNIQUE(name)`).
/// Si `is_default` est vrai, les autres profils existants sont retrogrades
/// (un seul profil par defaut a la fois).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture ou la lecture SQLite echoue.
pub(crate) fn create_hero_profile(
    conn: &Connection,
    name: &str,
    is_default: bool,
) -> Result<i64, StoreError> {
    if is_default {
        conn.execute("UPDATE hero_profiles SET is_default = 0", [])?;
    }
    conn.execute(
        "INSERT INTO hero_profiles (name, is_default) VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET is_default = excluded.is_default",
        params![name, is_default],
    )?;
    conn.query_row(
        "SELECT id FROM hero_profiles WHERE name = ?1",
        [name],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
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

/// Detache le pseudo `screen_name` du profil `profile_id` (M3-5,
/// "modification de profil"). Sans effet si le pseudo n'est pas connu ou
/// n'etait pas rattache a ce profil. Les mains deja importees et leur
/// `hero_player_id` ne sont pas touches ; seul le rattachement au profil
/// change (un prochain `recompute_hero_sessions` re-scope les sessions).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn unlink_hero_account(
    conn: &Connection,
    profile_id: i64,
    room: Room,
    screen_name: &str,
) -> Result<(), StoreError> {
    let room_id = get_or_create_room(conn, room)?;
    conn.execute(
        "DELETE FROM hero_accounts WHERE profile_id = ?1 AND player_id = (
            SELECT id FROM players WHERE room_id = ?2 AND screen_name = ?3
         )",
        params![profile_id, room_id, screen_name],
    )?;
    Ok(())
}

/// Renomme un profil Hero (M3-5, "modification de profil").
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue (ex. nom deja
/// pris par un autre profil, `UNIQUE(name)`).
pub(crate) fn rename_hero_profile(
    conn: &Connection,
    profile_id: i64,
    new_name: &str,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE hero_profiles SET name = ?1 WHERE id = ?2",
        params![new_name, profile_id],
    )?;
    Ok(())
}

/// Pseudos actuellement rattaches a un profil (M3-5, ecran Parametres).
/// Un seul `Room` existe en V1 (`Winamax`) : pas de colonne room distincte
/// pour l'instant, a etendre si un jour une deuxieme room est supportee.
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn list_hero_account_pseudos(
    conn: &Connection,
    profile_id: i64,
) -> Result<Vec<String>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT p.screen_name FROM hero_accounts ha
         JOIN players p ON p.id = ha.player_id
         WHERE ha.profile_id = ?1
         ORDER BY p.screen_name",
    )?;
    let rows = stmt.query_map([profile_id], |row| row.get(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
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
    fn creating_a_profile_with_an_existing_name_reuses_its_id_instead_of_erroring() {
        let conn = migrated_connection();
        let first_id = create_hero_profile(&conn, "Frederic", true).unwrap();
        link_hero_account(&conn, first_id, Room::Winamax, "Fred_W").unwrap();

        let second_id = create_hero_profile(&conn, "Frederic", true).unwrap();
        assert_eq!(
            first_id, second_id,
            "rejouer l'assistant avec le meme nom ne doit pas creer un doublon"
        );

        let profiles = list_hero_profiles(&conn).unwrap();
        assert_eq!(profiles.len(), 1);

        let account_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hero_accounts WHERE profile_id = ?1",
                [first_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            account_count, 1,
            "le pseudo deja rattache au premier passage doit etre conserve"
        );
    }

    #[test]
    fn unlinking_a_pseudo_removes_only_that_account() {
        let conn = migrated_connection();
        let profile_id = create_hero_profile(&conn, "Frederic", true).unwrap();
        link_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").unwrap();
        link_hero_account(&conn, profile_id, Room::Winamax, "FredMTT").unwrap();

        unlink_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").unwrap();

        assert_eq!(
            list_hero_account_pseudos(&conn, profile_id).unwrap(),
            vec!["FredMTT".to_string()]
        );
    }

    #[test]
    fn unlinking_an_unknown_pseudo_is_a_no_op() {
        let conn = migrated_connection();
        let profile_id = create_hero_profile(&conn, "Frederic", true).unwrap();
        link_hero_account(&conn, profile_id, Room::Winamax, "Fred_W").unwrap();

        unlink_hero_account(&conn, profile_id, Room::Winamax, "DoesNotExist").unwrap();

        assert_eq!(
            list_hero_account_pseudos(&conn, profile_id).unwrap(),
            vec!["Fred_W".to_string()]
        );
    }

    #[test]
    fn renaming_a_profile_updates_only_its_name() {
        let conn = migrated_connection();
        let profile_id = create_hero_profile(&conn, "Frederic", true).unwrap();

        rename_hero_profile(&conn, profile_id, "Fred Pro").unwrap();

        let profiles = list_hero_profiles(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Fred Pro");
        assert!(profiles[0].is_default, "renaming must not touch is_default");
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
