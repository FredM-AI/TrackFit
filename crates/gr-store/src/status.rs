//! Requetes de lecture pour la barre d'etat (M3-3, PRD footer "Import :
//! statut / Mains aujourd'hui / Derniere main"). "Aujourd'hui" est defini
//! par l'appelant (minuit local, calcule cote UI) et passe en epoch ms UTC,
//! Rust ne stockant que des timestamps UTC (R-MONEY).

use rusqlite::Connection;

use crate::error::StoreError;

/// Nombre de mains dont Hero a joue un siege (`hands.hero_player_id`),
/// jouees depuis `since_ms` (epoch ms UTC).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn count_hero_hands_since(conn: &Connection, since_ms: i64) -> Result<i64, StoreError> {
    conn.query_row(
        "SELECT COUNT(*) FROM hands WHERE hero_player_id IS NOT NULL AND played_at >= ?1",
        [since_ms],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

/// Horodatage (`played_at`, epoch ms UTC) de la derniere main jouee par
/// Hero, toutes tables confondues. `None` si aucune main Hero n'est encore
/// en base.
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn latest_hero_hand_played_at(conn: &Connection) -> Result<Option<i64>, StoreError> {
    conn.query_row(
        "SELECT MAX(played_at) FROM hands WHERE hero_player_id IS NOT NULL",
        [],
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

    fn insert_room_and_player(conn: &Connection) -> (i64, i64) {
        conn.execute(
            "INSERT INTO rooms (code, name) VALUES ('WINAMAX', 'Winamax')",
            [],
        )
        .unwrap();
        let room_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO players (room_id, screen_name) VALUES (?1, 'Hero')",
            [room_id],
        )
        .unwrap();
        let player_id = conn.last_insert_rowid();
        (room_id, player_id)
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
    fn counts_only_hero_hands_played_since_the_given_timestamp() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn);
        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 2_000, Some(hero_id));
        insert_hand(&conn, room_id, "h3", 500, Some(hero_id)); // before the cutoff
        insert_hand(&conn, room_id, "h4", 3_000, None); // no Hero seat

        assert_eq!(count_hero_hands_since(&conn, 1_000).unwrap(), 2);
    }

    #[test]
    fn latest_hero_hand_ignores_hands_without_a_hero_seat() {
        let conn = migrated_connection();
        let (room_id, hero_id) = insert_room_and_player(&conn);
        assert_eq!(latest_hero_hand_played_at(&conn).unwrap(), None);

        insert_hand(&conn, room_id, "h1", 1_000, Some(hero_id));
        insert_hand(&conn, room_id, "h2", 5_000, None);
        insert_hand(&conn, room_id, "h3", 3_000, Some(hero_id));

        assert_eq!(latest_hero_hand_played_at(&conn).unwrap(), Some(3_000));
    }
}
