//! Requetes de lecture pour la barre d'etat (M3-3, PRD footer "Import :
//! statut / Mains aujourd'hui / Derniere main"). "Aujourd'hui" est defini
//! par l'appelant (minuit local, calcule cote UI) et passe en epoch ms UTC,
//! Rust ne stockant que des timestamps UTC (R-MONEY).
//!
//! Scopees par profil Hero (M3-5, D19) : une main compte pour `profile_id`
//! si son `hero_player_id` est rattache a ce profil via `hero_accounts`,
//! pas simplement "un siege Hero existe" (qui melangerait les profils une
//! fois qu'il y en a plusieurs).

use rusqlite::Connection;

use crate::error::StoreError;

/// Nombre de mains du profil Hero `profile_id` (via `hero_accounts`),
/// jouees depuis `since_ms` (epoch ms UTC).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn count_hero_hands_since(
    conn: &Connection,
    profile_id: i64,
    since_ms: i64,
) -> Result<i64, StoreError> {
    conn.query_row(
        "SELECT COUNT(*) FROM hands
         WHERE played_at >= ?1
           AND hero_player_id IN (
             SELECT player_id FROM hero_accounts WHERE profile_id = ?2
           )",
        rusqlite::params![since_ms, profile_id],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

/// Nombre total de mains importees, tous profils/salles confondus (PRD
/// §13.1, "etat de l'import" : sante globale de l'import, pas une mesure
/// scopee a un profil Hero).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn count_all_hands(conn: &Connection) -> Result<i64, StoreError> {
    conn.query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
        .map_err(StoreError::from)
}

/// Horodatage (`played_at`, epoch ms UTC) de la derniere main jouee par le
/// profil Hero `profile_id`, toutes tables confondues. `None` si aucune
/// main de ce profil n'est encore en base.
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn latest_hero_hand_played_at(
    conn: &Connection,
    profile_id: i64,
) -> Result<Option<i64>, StoreError> {
    conn.query_row(
        "SELECT MAX(played_at) FROM hands
         WHERE hero_player_id IN (
           SELECT player_id FROM hero_accounts WHERE profile_id = ?1
         )",
        [profile_id],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn insert_room_and_player(conn: &Connection, screen_name: &str) -> (i64, i64) {
        conn.execute(
            "INSERT INTO rooms (code, name) VALUES ('WINAMAX', 'Winamax')
             ON CONFLICT(code) DO NOTHING",
            [],
        )
        .unwrap();
        let room_id: i64 = conn
            .query_row("SELECT id FROM rooms WHERE code = 'WINAMAX'", [], |row| {
                row.get(0)
            })
            .unwrap();
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, ?2)",
            rusqlite::params![room_id, screen_name],
        )
        .unwrap();
        let player_id = conn.last_insert_rowid();
        (room_id, player_id)
    }

    fn create_profile_with_player(conn: &Connection, name: &str, player_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO hero_profiles (name, is_default) VALUES (?1, 1)",
            [name],
        )
        .unwrap();
        let profile_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO hero_accounts (profile_id, player_id) VALUES (?1, ?2)",
            rusqlite::params![profile_id, player_id],
        )
        .unwrap();
        profile_id
    }

    fn insert_hand(
        conn: &Connection,
        room_id: i64,
        room_hand_id: &str,
        played_at: i64,
        hero_player_id: Option<i64>,
    ) {
        conn.execute(
            "INSERT INTO hands (room_id, room_hand_id, played_at, hero_player_id, parser_version)
             VALUES (?1, ?2, ?3, ?4, '0.0.1')",
            rusqlite::params![room_id, room_hand_id, played_at, hero_player_id],
        )
        .unwrap();
    }

    #[test]
    fn count_all_hands_ignores_the_hero_profile_scope() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn, "Hero");
        let (_, other_id) = insert_room_and_player(&conn, "OtherHero");
        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 2_000, Some(other_id));
        insert_hand(&conn, room_id, "h3", 3_000, None);

        assert_eq!(count_all_hands(&conn).unwrap(), 3);
    }

    #[test]
    fn counts_only_hero_hands_played_since_the_given_timestamp() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn, "Hero");
        let profile_id = create_profile_with_player(&conn, "Frederic", hero_id);
        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 2_000, Some(hero_id));
        insert_hand(&conn, room_id, "h3", 500, Some(hero_id)); // before the cutoff
        insert_hand(&conn, room_id, "h4", 3_000, None); // no Hero seat

        assert_eq!(count_hero_hands_since(&conn, profile_id, 1_000).unwrap(), 2);
    }

    #[test]
    fn a_hand_played_by_a_pseudo_outside_this_profile_does_not_count() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn, "Hero");
        let (_, other_hero_id) = insert_room_and_player(&conn, "OtherHero");
        let profile_id = create_profile_with_player(&conn, "Frederic", hero_id);
        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 2_000, Some(other_hero_id));

        assert_eq!(count_hero_hands_since(&conn, profile_id, 0).unwrap(), 1);
    }

    #[test]
    fn latest_hero_hand_ignores_hands_without_a_hero_seat() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn, "Hero");
        let profile_id = create_profile_with_player(&conn, "Frederic", hero_id);
        assert_eq!(latest_hero_hand_played_at(&conn, profile_id).unwrap(), None);

        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 5_000, None);
        insert_hand(&conn, room_id, "h3", 3_000, Some(hero_id));

        assert_eq!(
            latest_hero_hand_played_at(&conn, profile_id).unwrap(),
            Some(3_000)
        );
    }
}
