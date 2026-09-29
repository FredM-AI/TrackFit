//! Types de tickets et métadonnées ticket par tournoi (M5-2, PRD §8.6, D20).
//!
//! **Portée volontairement limitée au backend** : `ticket_types` (table déjà
//! prévue dans `0001_init.sql`) et l'édition des colonnes ticket de
//! `tournament_entries` (déjà prévues elles aussi, `paid_with_ticket`/
//! `ticket_used_type_id`/`ticket_won_type_id`/`ticket_won_count`). Ni le
//! filtre « Via ticket » (PRD §11 : panneau de filtres global, inexistant
//! dans l'UI) ni l'écran d'édition (`Tournaments.tsx` est encore un
//! placeholder, M0-5) ne sont câblés — même raisonnement que les commandes
//! Tauri de M2-5 (Logs), prêtes avant leur consommateur UI. Aucun exemple
//! réel de ticket dans le corpus pour guider un choix d'UI de toute façon
//! (`docs/formats/winamax.md` §5.2/§7).
//!
//! Édite "joué via ticket"/"ticket gagné" **avant** qu'un summary ne soit
//! attaché reste possible : `set_tournament_entry_ticket_used`/`_won`
//! utilisent `ON CONFLICT ... DO UPDATE` en ne touchant **que** leurs
//! propres colonnes, jamais `entries_count`/`finish_position`/
//! `prize_cents`/`bounty_cents` — un `attach_summary` (M2-6) ulterieur ne
//! peut donc pas ecraser une edition manuelle anterieure, et inversement.

use rusqlite::{params, Connection};

use crate::error::StoreError;

/// Un type de ticket (PRD §8.6 : "libellé et valeur, valeur faciale lue ou
/// saisie une fois par type de ticket").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketTypeRow {
    pub id: i64,
    pub room_id: i64,
    pub label: String,
    pub face_value_cents: Option<i64>,
}

/// Crée un type de ticket, ou met à jour sa valeur faciale s'il existe déjà
/// (idempotent sur `UNIQUE(room_id, label)`, même motif que
/// `hero::create_hero_profile`).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'écriture ou la lecture SQLite échoue.
pub(crate) fn create_or_update_ticket_type(
    conn: &Connection,
    room_id: i64,
    label: &str,
    face_value_cents: Option<i64>,
) -> Result<i64, StoreError> {
    conn.execute(
        "INSERT INTO ticket_types (room_id, label, face_value_cents) VALUES (?1, ?2, ?3)
         ON CONFLICT(room_id, label) DO UPDATE SET face_value_cents = excluded.face_value_cents",
        params![room_id, label, face_value_cents],
    )?;
    conn.query_row(
        "SELECT id FROM ticket_types WHERE room_id = ?1 AND label = ?2",
        params![room_id, label],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

/// Liste les types de tickets connus pour `room_id`, par libellé.
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite échoue.
pub(crate) fn list_ticket_types(
    conn: &Connection,
    room_id: i64,
) -> Result<Vec<TicketTypeRow>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, room_id, label, face_value_cents FROM ticket_types
         WHERE room_id = ?1 ORDER BY label",
    )?;
    let rows = stmt.query_map([room_id], |row| {
        Ok(TicketTypeRow {
            id: row.get(0)?,
            room_id: row.get(1)?,
            label: row.get(2)?,
            face_value_cents: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

/// Marque (ou démarque, avec `ticket_type_id: None`) l'entrée
/// `(tournament_id, player_id)` comme payée avec un ticket (PRD §8.6,
/// édition de métadonnée). Crée l'entrée si elle n'existe pas encore (un
/// summary peut ne pas avoir été attaché, M2-6).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'écriture SQLite échoue.
pub(crate) fn set_tournament_entry_ticket_used(
    conn: &Connection,
    tournament_id: i64,
    player_id: i64,
    ticket_type_id: Option<i64>,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO tournament_entries (tournament_id, player_id, paid_with_ticket, ticket_used_type_id)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(tournament_id, player_id) DO UPDATE SET
            paid_with_ticket = excluded.paid_with_ticket,
            ticket_used_type_id = excluded.ticket_used_type_id",
        params![tournament_id, player_id, ticket_type_id.is_some(), ticket_type_id],
    )?;
    Ok(())
}

/// Enregistre (ou efface, avec `ticket_type_id: None`) le ticket gagné par
/// l'entrée `(tournament_id, player_id)` (PRD §8.6, satellites). Crée
/// l'entrée si elle n'existe pas encore, même raison que
/// [`set_tournament_entry_ticket_used`].
///
/// # Errors
/// Renvoie une [`StoreError`] si l'écriture SQLite échoue.
pub(crate) fn set_tournament_entry_ticket_won(
    conn: &Connection,
    tournament_id: i64,
    player_id: i64,
    ticket_type_id: Option<i64>,
    count: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO tournament_entries (tournament_id, player_id, ticket_won_type_id, ticket_won_count)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(tournament_id, player_id) DO UPDATE SET
            ticket_won_type_id = excluded.ticket_won_type_id,
            ticket_won_count = excluded.ticket_won_count",
        params![tournament_id, player_id, ticket_type_id, count],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gr_parser_api::Room;

    use super::*;
    use crate::migrate::run_migrations;
    use crate::repo::{get_or_create_player_id, get_or_create_room, get_or_create_tournament};

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    #[test]
    fn creating_a_ticket_type_twice_updates_its_face_value_instead_of_erroring() {
        let conn = migrated_connection();
        let room_id = get_or_create_room(&conn, Room::Winamax).expect("room");

        let first_id =
            create_or_update_ticket_type(&conn, room_id, "Ticket 5€", Some(500)).expect("create");
        let second_id =
            create_or_update_ticket_type(&conn, room_id, "Ticket 5€", Some(550)).expect("update");
        assert_eq!(first_id, second_id);

        let types = list_ticket_types(&conn, room_id).expect("list");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].face_value_cents, Some(550));
    }

    #[test]
    fn list_ticket_types_orders_by_label() {
        let conn = migrated_connection();
        let room_id = get_or_create_room(&conn, Room::Winamax).expect("room");
        create_or_update_ticket_type(&conn, room_id, "Ticket 20€", Some(2000)).expect("create");
        create_or_update_ticket_type(&conn, room_id, "Ticket 5€", Some(500)).expect("create");

        let types = list_ticket_types(&conn, room_id).expect("list");
        let labels: Vec<&str> = types.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, vec!["Ticket 20€", "Ticket 5€"]);
    }

    fn setup_tournament_and_player(conn: &mut Connection) -> (i64, i64) {
        let room_id = get_or_create_room(conn, Room::Winamax).expect("room");
        let tournament_id = {
            let tx = conn.transaction().expect("transaction");
            let id =
                get_or_create_tournament(&tx, room_id, "12345", "Test", None).expect("tournament");
            tx.commit().expect("commit");
            id
        };
        let player_id = get_or_create_player_id(conn, room_id, "Hero").expect("player");
        (tournament_id, player_id)
    }

    #[test]
    fn marking_an_entry_as_ticket_paid_creates_it_when_no_summary_was_attached_yet() {
        let mut conn = migrated_connection();
        let (tournament_id, player_id) = setup_tournament_and_player(&mut conn);
        let room_id = get_or_create_room(&conn, Room::Winamax).expect("room");
        let ticket_type_id =
            create_or_update_ticket_type(&conn, room_id, "Ticket 5€", Some(500)).expect("create");

        set_tournament_entry_ticket_used(&conn, tournament_id, player_id, Some(ticket_type_id))
            .expect("set ticket used");

        let (paid_with_ticket, used_type_id): (bool, Option<i64>) = conn
            .query_row(
                "SELECT paid_with_ticket, ticket_used_type_id FROM tournament_entries
                 WHERE tournament_id = ?1 AND player_id = ?2",
                params![tournament_id, player_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("entry should have been created");
        assert!(paid_with_ticket);
        assert_eq!(used_type_id, Some(ticket_type_id));
    }

    #[test]
    fn marking_an_entry_as_ticket_paid_does_not_touch_its_result_columns() {
        let mut conn = migrated_connection();
        let (tournament_id, player_id) = setup_tournament_and_player(&mut conn);
        conn.execute(
            "INSERT INTO tournament_entries (tournament_id, player_id, entries_count, finish_position, prize_cents)
             VALUES (?1, ?2, 2, 7, 3000)",
            params![tournament_id, player_id],
        )
        .expect("seed a real result from a summary");

        set_tournament_entry_ticket_used(&conn, tournament_id, player_id, None)
            .expect("set ticket used (cleared)");

        let (entries_count, finish_position, prize_cents): (i64, Option<i64>, i64) = conn
            .query_row(
                "SELECT entries_count, finish_position, prize_cents FROM tournament_entries
                 WHERE tournament_id = ?1 AND player_id = ?2",
                params![tournament_id, player_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("entry should still exist");
        assert_eq!(entries_count, 2);
        assert_eq!(finish_position, Some(7));
        assert_eq!(prize_cents, 3000);
    }

    #[test]
    fn setting_a_ticket_won_records_its_type_and_count() {
        let mut conn = migrated_connection();
        let (tournament_id, player_id) = setup_tournament_and_player(&mut conn);
        let room_id = get_or_create_room(&conn, Room::Winamax).expect("room");
        let ticket_type_id =
            create_or_update_ticket_type(&conn, room_id, "Ticket 20€", Some(2000)).expect("create");

        set_tournament_entry_ticket_won(&conn, tournament_id, player_id, Some(ticket_type_id), 1)
            .expect("set ticket won");

        let (won_type_id, won_count): (Option<i64>, i64) = conn
            .query_row(
                "SELECT ticket_won_type_id, ticket_won_count FROM tournament_entries
                 WHERE tournament_id = ?1 AND player_id = ?2",
                params![tournament_id, player_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("entry should have been created");
        assert_eq!(won_type_id, Some(ticket_type_id));
        assert_eq!(won_count, 1);
    }
}
