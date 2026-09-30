//! Tags de mains (M6-5, PRD §13.4/§13.7, D27) : les 9 tags predefinis sont
//! semes par la migration `0004_predefined_tags.sql`. Ce module ne couvre
//! que la liste et l'application en masse ("selection multiple -> tag en
//! masse", CA de M6-5) : creation de tags libres, note texte par main et
//! filtre par tag restent le perimetre de M7-4 (Tags et notes).

use rusqlite::{params, Connection};

use crate::error::StoreError;

/// Un tag (predefini ou libre, PRD D27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub id: i64,
    pub label_key: String,
    pub is_predefined: bool,
    pub shade: Option<String>,
}

/// Tous les tags connus, predefinis d'abord puis par ordre alphabetique de
/// cle (M6-5 n'affiche que les 9 predefinis, mais cette fonction reste
/// generique pour que M7-4 puisse y ajouter des tags libres sans la
/// reecrire).
///
/// # Errors
/// Renvoie une [`StoreError`] si la lecture SQLite echoue.
pub(crate) fn list_tags(conn: &Connection) -> Result<Vec<TagRow>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, label_key, is_predefined, shade FROM tags
         ORDER BY is_predefined DESC, label_key",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TagRow {
            id: row.get(0)?,
            label_key: row.get(1)?,
            is_predefined: row.get(2)?,
            shade: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

/// Applique `tag_id` a chaque main de `hand_ids` (PRD §13.4 : "selection
/// multiple -> tag en masse"). Idempotent par ligne (`INSERT OR IGNORE` sur
/// la cle primaire `(hand_id, tag_id)`) : re-appliquer le meme tag a une
/// main deja taguee ne duplique rien et n'ecrase pas sa note existante
/// (perimetre note = M7-4, mais autant ne pas la perdre par accident si
/// elle existe deja).
///
/// # Errors
/// Renvoie une [`StoreError`] si l'ecriture SQLite echoue.
pub(crate) fn tag_hands(
    conn: &mut Connection,
    hand_ids: &[i64],
    tag_id: i64,
    now_ms: i64,
) -> Result<(), StoreError> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO hand_tags (hand_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
        )?;
        for &hand_id in hand_ids {
            stmt.execute(params![hand_id, tag_id, now_ms])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gr_parser_api::Room;

    use super::*;
    use crate::migrate::run_migrations;
    use crate::repo::{get_or_create_player_id, get_or_create_room};

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite connection");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        run_migrations(&mut conn).expect("migrations should apply");
        conn
    }

    fn seed_hand(conn: &Connection, room_id: i64, hero_player_id: i64, room_hand_id: &str) -> i64 {
        conn.execute(
            "INSERT INTO hands (room_id, room_hand_id, played_at, parser_version, hero_player_id)
             VALUES (?1, ?2, 0, '0.0.1', ?3)",
            params![room_id, room_hand_id, hero_player_id],
        )
        .expect("insert a hand");
        conn.last_insert_rowid()
    }

    #[test]
    fn list_tags_returns_the_nine_predefined_tags_seeded_by_the_migration() {
        let conn = migrated_connection();
        let tags = list_tags(&conn).expect("list tags");
        assert_eq!(tags.len(), 9, "{tags:?}");
        assert!(tags.iter().all(|t| t.is_predefined));
        assert!(tags.iter().any(|t| t.label_key == "tags.badBeat"));
    }

    #[test]
    fn tag_hands_applies_a_tag_to_every_hand_and_is_idempotent() {
        let mut conn = migrated_connection();
        let room_id = get_or_create_room(&conn, Room::Winamax).expect("room");
        let hero_player_id = get_or_create_player_id(&conn, room_id, "Hero").expect("player");
        let hand1 = seed_hand(&conn, room_id, hero_player_id, "#1");
        let hand2 = seed_hand(&conn, room_id, hero_player_id, "#2");
        let tag_id: i64 = conn
            .query_row(
                "SELECT id FROM tags WHERE label_key = 'tags.badBeat'",
                [],
                |row| row.get(0),
            )
            .expect("bad beat tag should exist");

        tag_hands(&mut conn, &[hand1, hand2], tag_id, 1_000).expect("tag hands");
        // Re-application : ne doit pas echouer ni dupliquer.
        tag_hands(&mut conn, &[hand1, hand2], tag_id, 2_000).expect("re-tag hands");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hand_tags WHERE tag_id = ?1",
                [tag_id],
                |row| row.get(0),
            )
            .expect("count hand_tags");
        assert_eq!(count, 2, "1 ligne par main, pas de doublon");
    }
}
