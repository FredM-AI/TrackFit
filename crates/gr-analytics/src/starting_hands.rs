//! Grille 13×13 des mains de depart (PRD §13.5, M7-7) : frequence, VPIP/PFR
//! et resultat moyen en bb/main, une ligne par `hand_class` (ex. `'AKs'`,
//! `'TT'`, `'Q9o'`, `hand_players.hand_class`, calculee par M7-7).
//!
//! Requete dediee plutot qu'une extension du trait generique
//! `AnalyticsBackend` (meme raisonnement que `reports::fetch_bb_defense_by_depth`,
//! M7-1) : la forme du resultat differe de `ReportRow`/`StatCell` (un
//! `AVG(net_bb)` n'est pas un couple opportunite/action) et `hand_class`
//! n'est pas une `Dimension` du trait generique (portee de M4-7).

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;
use crate::StatCell;

/// Une cellule de la grille : frequence (nombre de mains Hero avec cette
/// classe), VPIP/PFR (memes paires opportunite/action que le reste du
/// crate) et resultat moyen en bb/main (`None` si aucune main de cette
/// classe n'a de `net_bb` calcule — ne devrait pas arriver en pratique,
/// `net_bb` est calcule pour toute main importee depuis M6-3, mais `AVG`
/// sur un ensemble vide renvoie `NULL` en SQL).
#[derive(Debug, Clone, PartialEq)]
pub struct HandClassCell {
    pub hand_class: String,
    pub hands_played: i64,
    pub vpip: StatCell,
    pub pfr: StatCell,
    pub avg_net_bb: Option<f64>,
}

/// Grille des mains de depart du profil Hero `hero_profile_id` (PRD §13.5).
/// Une ligne par `hand_class` distincte observee (`hand_players.hand_class
/// IS NOT NULL`) ; `hand_class` reste `NULL` pour une main Hero non encore
/// retraitee (voir `Store::backfill_hand_class`) ou sans `hero_cards`
/// connues (jamais observe en pratique, garde du parseur).
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hand_class_grid(
    conn: &Connection,
    hero_profile_id: i64,
) -> Result<Vec<HandClassCell>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT hp.hand_class, COUNT(*),
                COALESCE(SUM(hp.vpip_opp), 0), COALESCE(SUM(hp.vpip), 0),
                COALESCE(SUM(hp.vpip_opp), 0), COALESCE(SUM(hp.pfr), 0),
                AVG(hp.net_bb)
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND hp.hand_class IS NOT NULL
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
         GROUP BY hp.hand_class",
    )?;
    let rows = stmt.query_map(params![hero_profile_id], |row| {
        Ok(HandClassCell {
            hand_class: row.get(0)?,
            hands_played: row.get(1)?,
            vpip: StatCell {
                opportunities: row.get(2)?,
                actions: row.get(3)?,
            },
            pfr: StatCell {
                opportunities: row.get(4)?,
                actions: row.get(5)?,
            },
            avg_net_bb: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}
