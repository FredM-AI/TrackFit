//! Requetes SQLite pour l'ecran Resultats (PRD §9.2/§13.2, M6-3) :
//! historique de jetons par main (G2, reel vs EV all-in), volume de
//! tournois par jour (G6), ROI par buy-in (G3), distribution des places
//! (G5) et le pivot (buy-in x KO/non-KO, vitesse, jour de semaine, heure,
//! mois). G4 (ROI par format complet KO/PKO/Mystery/Space) reste bloque en
//! permanence : le summary Winamax ne distingue pas les sous-types (`docs/
//! formats/winamax.md`, documente depuis M2-6). La "bulle" de G5 (mise en
//! evidence du bust juste avant les places payees, PRD §9.2) est elle aussi
//! bloquee : `tournaments.paid_places` n'est jamais renseigne, le nombre de
//! places payees etant confirme **absent** du format de summary Winamax
//! (`docs/formats/winamax.md` §5.2 : "❓ places payees | absentes des blocs
//! | ✅ absence") — meme categorie de limite que G4, decouverte pendant
//! cette phase 2. Meme convention que [`crate::home`] : une `&Connection`
//! brute, pas de dependance a `gr-store`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rusqlite::{params, Connection};

use crate::error::AnalyticsError;
use crate::home::TournamentResultRow;
use crate::kpis::{compute_results_kpis, ResultsKpis, TicketValuation};

/// Un point de la courbe G2 (PRD §9.2) : jetons nets de la main (reel) et
/// ajustes a l'EV all-in (`net_bb - allin_ev_diff_bb`, 0 si la main n'a pas
/// d'evenement all-in, M5-4), en bb. Cumul et echantillonnage LTTB restent a
/// la charge de l'appelant (`src-tauri`, memes conventions que G1/M6-2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipHistoryPoint {
    pub played_at: i64,
    pub net_bb: f64,
    pub ev_adjusted_net_bb: f64,
}

/// Historique chronologique des mains du Hero avec `net_bb` connu (main
/// deja retro-remplie, M6-3 ; les mains pas encore traitees par
/// `Store::backfill_net_chips` sont silencieusement absentes, elles
/// reapparaitront au prochain appel une fois le rattrapage termine).
/// Filtrable par periode (M6-1) : `since_ms`/`until_ms`, `None`/`None` =
/// tout l'historique.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_chip_history(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<Vec<ChipHistoryPoint>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT h.played_at, hp.net_bb, hp.allin_ev_diff_chips, h.bb
         FROM hand_players hp
         JOIN hands h ON h.id = hp.hand_id
         WHERE hp.is_hero = 1
           AND hp.net_bb IS NOT NULL
           AND h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
           AND (?2 IS NULL OR h.played_at >= ?2)
           AND (?3 IS NULL OR h.played_at < ?3)
         ORDER BY h.played_at, h.id",
    )?;
    let rows = stmt.query_map(params![hero_profile_id, since_ms, until_ms], |row| {
        let played_at: i64 = row.get(0)?;
        let net_bb: f64 = row.get(1)?;
        let allin_ev_diff_chips: Option<f64> = row.get(2)?;
        let bb: i64 = row.get(3)?;
        #[allow(clippy::cast_precision_loss)]
        let allin_diff_bb = match (allin_ev_diff_chips, bb) {
            (Some(diff), bb) if bb > 0 => diff / bb as f64,
            _ => 0.0,
        };
        Ok(ChipHistoryPoint {
            played_at,
            net_bb,
            ev_adjusted_net_bb: net_bb - allin_diff_bb,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Un jour (minuit UTC epoch ms) et le nombre de tournois du Hero demarres
/// ce jour-la (G6, PRD §9.2, version "par jour" — semaine/mois/heatmap
/// differes en phase 1 de M6-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TournamentVolumePoint {
    pub day_epoch_ms: i64,
    pub tournaments_count: i64,
}

/// Basee sur `hands`/`tournaments` (pas `tournament_entries`, qui n'existe
/// qu'une fois le summary attache, M2-6) : un tournoi compte des qu'au
/// moins une main d'Hero y a ete importee, meme incomplet. Filtrable par
/// periode (M6-1) : `since_ms`/`until_ms`, `None`/`None` = tout l'historique.
///
/// # Errors
/// Renvoie une [`AnalyticsError`] si la lecture SQLite echoue.
pub fn fetch_hero_tournament_volume_by_day(
    conn: &Connection,
    hero_profile_id: i64,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
) -> Result<Vec<TournamentVolumePoint>, AnalyticsError> {
    let mut stmt = conn.prepare(
        "SELECT (t.started_at / 86400000) * 86400000 AS day, COUNT(DISTINCT t.id)
         FROM hands h
         JOIN tournaments t ON t.id = h.tournament_id
         WHERE h.hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)
           AND t.started_at IS NOT NULL
           AND (?2 IS NULL OR t.started_at >= ?2)
           AND (?3 IS NULL OR t.started_at < ?3)
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt.query_map(params![hero_profile_id, since_ms, until_ms], |row| {
        Ok(TournamentVolumePoint {
            day_epoch_ms: row.get(0)?,
            tournaments_count: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AnalyticsError::from)
}

/// Tranches de buy-in (PRD §9.2, G3) : `(borne_min_cents, borne_max_cents)`,
/// `None` = pas de borne haute ("100+"). Reutilisees ici pour le pivot
/// (phase 1 de M6-3) faute d'un reglage utilisateur (§11 : "tranches
/// modifiables", differe — pas d'ecran de parametres pour ca encore).
pub const BUYIN_BRACKETS_CENTS: [(i64, Option<i64>); 7] = [
    (0, Some(200)),
    (200, Some(500)),
    (500, Some(1000)),
    (1000, Some(2500)),
    (2500, Some(5000)),
    (5000, Some(10000)),
    (10000, None),
];

fn buyin_bracket_index(buyin_cents: i64) -> usize {
    BUYIN_BRACKETS_CENTS
        .iter()
        .position(|&(min, max)| buyin_cents >= min && max.is_none_or(|max| buyin_cents < max))
        .unwrap_or(BUYIN_BRACKETS_CENTS.len() - 1)
}

/// Une ligne du pivot reduit (buy-in x KO/non-KO, phase 1 de M6-3 — le
/// pivot complet §13.2 (format/vitesse/jour/heure/mois) et les KPI §9.1 pas
/// encore calcules (place moyenne, % tables finales, meilleur gain) sont
/// differes, decision validee avec Frederic avant implementation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotRow {
    pub buyin_min_cents: i64,
    pub buyin_max_cents: Option<i64>,
    pub is_ko: bool,
    pub kpis: ResultsKpis,
}

/// Regroupe `results` par tranche de buy-in (du premier bullet de chaque
/// tournoi, constant sur toute une re-entry) et `is_ko`, puis calcule les
/// KPIs §9.1 deja disponibles (`compute_results_kpis`, M5-1) sur chaque
/// groupe. Ne fait aucune requete : pur, sur des donnees deja recuperees via
/// [`crate::fetch_hero_tournament_results`] (memes conventions que G1, M6-2).
#[must_use]
pub fn pivot_by_buyin_and_ko(results: &[TournamentResultRow]) -> Vec<PivotRow> {
    let mut groups: Vec<Vec<crate::TournamentResult>> =
        vec![Vec::new(); BUYIN_BRACKETS_CENTS.len() * 2];

    for row in results {
        let Some(first_bullet) = row.result.bullets.first() else {
            continue;
        };
        let buyin_cents = first_bullet.buyin_prize_cents
            + first_bullet.buyin_bounty_cents
            + first_bullet.buyin_fee_cents;
        let bracket = buyin_bracket_index(buyin_cents);
        let ko_offset = usize::from(row.result.is_ko);
        groups[bracket * 2 + ko_offset].push(row.result.clone());
    }

    groups
        .into_iter()
        .enumerate()
        .filter(|(_, group)| !group.is_empty())
        .map(|(idx, group)| {
            let (buyin_min_cents, buyin_max_cents) = BUYIN_BRACKETS_CENTS[idx / 2];
            let is_ko = idx % 2 == 1;
            PivotRow {
                buyin_min_cents,
                buyin_max_cents,
                is_ko,
                kpis: compute_results_kpis(&group, TicketValuation::FaceValue),
            }
        })
        .collect()
}

/// Export CSV du pivot (PRD §13.2 : "donnees agregees, pas de mains, donc
/// compatible avec D26" — pas de donnee personnelle a anonymiser). Colonnes
/// dans l'ordre des KPI §9.1 deja calcules ; `;` comme separateur (convention
/// FR, coherente avec un tableur ouvert en France).
#[must_use]
pub fn pivot_to_csv(rows: &[PivotRow]) -> String {
    let mut csv = String::from(
        "buyin_min_cents;buyin_max_cents;is_ko;tournaments;cost_cents;fees_cents;profit_cents;roi;itm_rate;abi_cents\n",
    );
    for row in rows {
        let _ = writeln!(
            csv,
            "{};{};{};{};{};{};{};{};{};{}",
            row.buyin_min_cents,
            row.buyin_max_cents.map_or(String::new(), |v| v.to_string()),
            row.is_ko,
            row.kpis.tournaments_count,
            row.kpis.cost_cents,
            row.kpis.fees_cents,
            row.kpis.profit_cents,
            row.kpis.roi.map_or(String::new(), |v| v.to_string()),
            row.kpis.itm_rate.map_or(String::new(), |v| v.to_string()),
            row.kpis.abi_cents.map_or(String::new(), |v| v.to_string()),
        );
    }
    csv
}

/// En-tete commun des colonnes KPI (§9.1 deja calculees) partage par tous
/// les exports CSV du pivot phase 2 ; seule la/les colonne(s) de cle de
/// regroupement varient d'une dimension a l'autre.
const KPI_CSV_HEADER: &str =
    "tournaments;cost_cents;fees_cents;profit_cents;roi;itm_rate;abi_cents";

fn kpi_csv_fields(kpis: &ResultsKpis) -> String {
    format!(
        "{};{};{};{};{};{};{}",
        kpis.tournaments_count,
        kpis.cost_cents,
        kpis.fees_cents,
        kpis.profit_cents,
        kpis.roi.map_or(String::new(), |v| v.to_string()),
        kpis.itm_rate.map_or(String::new(), |v| v.to_string()),
        kpis.abi_cents.map_or(String::new(), |v| v.to_string()),
    )
}

/// Une ligne de G3 (PRD §9.2 : "ROI par tranche de buy-in", memes tranches
/// que le pivot phase 1). Fonction pure, memes conventions que
/// [`pivot_by_buyin_and_ko`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuyinRoiRow {
    pub buyin_min_cents: i64,
    pub buyin_max_cents: Option<i64>,
    pub kpis: ResultsKpis,
}

/// G3 (PRD §9.2) : ROI par tranche de buy-in, tous formats confondus
/// (contrairement au pivot phase 1 qui croise en plus KO/non-KO).
#[must_use]
pub fn roi_by_buyin(results: &[TournamentResultRow]) -> Vec<BuyinRoiRow> {
    let mut groups: Vec<Vec<crate::TournamentResult>> =
        vec![Vec::new(); BUYIN_BRACKETS_CENTS.len()];

    for row in results {
        let Some(first_bullet) = row.result.bullets.first() else {
            continue;
        };
        let buyin_cents = first_bullet.buyin_prize_cents
            + first_bullet.buyin_bounty_cents
            + first_bullet.buyin_fee_cents;
        groups[buyin_bracket_index(buyin_cents)].push(row.result.clone());
    }

    groups
        .into_iter()
        .enumerate()
        .filter(|(_, group)| !group.is_empty())
        .map(|(idx, group)| {
            let (buyin_min_cents, buyin_max_cents) = BUYIN_BRACKETS_CENTS[idx];
            BuyinRoiRow {
                buyin_min_cents,
                buyin_max_cents,
                kpis: compute_results_kpis(&group, TicketValuation::FaceValue),
            }
        })
        .collect()
}

/// Export CSV de G3.
#[must_use]
pub fn roi_by_buyin_to_csv(rows: &[BuyinRoiRow]) -> String {
    let mut csv = format!("buyin_min_cents;buyin_max_cents;{KPI_CSV_HEADER}\n");
    for row in rows {
        let _ = writeln!(
            csv,
            "{};{};{}",
            row.buyin_min_cents,
            row.buyin_max_cents.map_or(String::new(), |v| v.to_string()),
            kpi_csv_fields(&row.kpis),
        );
    }
    csv
}

/// Une tranche de percentile de sortie (G5, PRD §9.2) : `floor_percent` va
/// de 0 (meilleures sorties, proches de la victoire) a 90 par pas de 10 (les
/// bustouts les plus precoces). Percentile = `finish_position / entrants`
/// (1 = 1ere place, proche de 100 = sortie la plus precoce).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishPercentileBucket {
    pub floor_percent: u32,
    pub tournaments_count: u32,
}

/// G5 (PRD §9.2) : distribution des places de sortie, en percentile du
/// nombre d'inscrits. Necessite `finish_position` ET `entrants` connus (un
/// tournoi encore `PROVISIONAL`, sans summary rattache, est exclu — meme
/// principe que l'exclusion des tournois sans `started_at` sur G1, M6-2).
/// **La mise en evidence de "la bulle"** (PRD §9.2) reste hors de cette
/// fonction : elle demanderait `tournaments.paid_places`, jamais renseigne
/// (absent du format de summary Winamax, voir la note en tete de module) —
/// limite documentee, pas un oubli.
#[must_use]
pub fn finish_percentile_distribution(
    results: &[TournamentResultRow],
) -> Vec<FinishPercentileBucket> {
    let mut buckets: BTreeMap<u32, u32> = BTreeMap::new();
    for row in results {
        let (Some(finish_position), Some(entrants)) = (row.finish_position, row.entrants) else {
            continue;
        };
        if entrants <= 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let percent = 100.0 * finish_position as f64 / entrants as f64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let floor_percent = ((percent / 10.0).floor() as i64).clamp(0, 9) as u32 * 10;
        *buckets.entry(floor_percent).or_insert(0) += 1;
    }
    buckets
        .into_iter()
        .map(
            |(floor_percent, tournaments_count)| FinishPercentileBucket {
                floor_percent,
                tournaments_count,
            },
        )
        .collect()
}

/// Nombre de sieges par defaut d'une table finale (PRD §9.1 : "Place ≤
/// nombre de sieges de la table finale (9 par defaut, parametrable par
/// format)") : pas d'ecran de parametres pour ce reglage tant que M8-1
/// n'existe pas, valeur fixe assumee (meme principe que d'autres reglages
/// par defaut deja assumes en M6-2/M6-3 phase 1 : periode par defaut,
/// tranches de buy-in).
const FINAL_TABLE_SEATS_DEFAULT: i64 = 9;

/// KPI §9.1 restants (place moyenne, % tables finales, meilleur gain, plus
/// gros tournoi) : differes en M5-1 (portee volontairement limitee, voir
/// `kpis.rs`) puis en phase 1 de M6-3, combles ici. Fonction pure sur des
/// resultats deja recuperes (memes conventions que le pivot).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdditionalResultsKpis {
    pub avg_finish_position: Option<f64>,
    pub final_table_rate: Option<f64>,
    pub best_win_cents: i64,
    pub biggest_tournament_entrants: Option<i64>,
}

#[must_use]
pub fn compute_additional_kpis(results: &[TournamentResultRow]) -> AdditionalResultsKpis {
    let known_finishes: Vec<i64> = results.iter().filter_map(|r| r.finish_position).collect();
    #[allow(clippy::cast_precision_loss)]
    let avg_finish_position = if known_finishes.is_empty() {
        None
    } else {
        Some(known_finishes.iter().sum::<i64>() as f64 / known_finishes.len() as f64)
    };
    #[allow(clippy::cast_precision_loss)]
    let final_table_rate = if known_finishes.is_empty() {
        None
    } else {
        let final_tables = known_finishes
            .iter()
            .filter(|&&p| p <= FINAL_TABLE_SEATS_DEFAULT)
            .count();
        Some(final_tables as f64 / known_finishes.len() as f64)
    };
    let best_win_cents = results
        .iter()
        .map(|r| {
            compute_results_kpis(std::slice::from_ref(&r.result), TicketValuation::FaceValue)
                .total_winnings_cents
        })
        .max()
        .unwrap_or(0);
    let biggest_tournament_entrants = results.iter().filter_map(|r| r.entrants).max();
    AdditionalResultsKpis {
        avg_finish_position,
        final_table_rate,
        best_win_cents,
        biggest_tournament_entrants,
    }
}

/// Regroupe `results` par une cle entiere connue (jour de semaine/heure/mois
/// UTC, deja calcules cote SQL sur `TournamentResultRow`, voir `crate::home`)
/// et calcule les KPI §9.1 sur chaque groupe nettoye. Les tournois sans cle
/// connue (`started_at` absent) sont exclus. Prive : partage par les 3
/// dimensions temporelles du pivot complet ci-dessous (memes conventions,
/// seul le type de la cle differe pour `pivot_by_speed`, gardee separee).
fn group_by_known_key(
    results: &[TournamentResultRow],
    key_fn: impl Fn(&TournamentResultRow) -> Option<i64>,
) -> Vec<(i64, ResultsKpis)> {
    let mut groups: BTreeMap<i64, Vec<crate::TournamentResult>> = BTreeMap::new();
    for row in results {
        if let Some(key) = key_fn(row) {
            groups.entry(key).or_default().push(row.result.clone());
        }
    }
    groups
        .into_iter()
        .map(|(key, group)| {
            (
                key,
                compute_results_kpis(&group, TicketValuation::FaceValue),
            )
        })
        .collect()
}

/// Une ligne du pivot par vitesse (PRD §13.2, M6-3 phase 2). `speed` est la
/// valeur brute du summary (`turbo`/`semiturbo`/`normal`), `None` si le
/// summary n'est pas encore rattache (tournoi `PROVISIONAL`).
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedPivotRow {
    pub speed: Option<String>,
    pub kpis: ResultsKpis,
}

#[must_use]
pub fn pivot_by_speed(results: &[TournamentResultRow]) -> Vec<SpeedPivotRow> {
    let mut groups: BTreeMap<Option<String>, Vec<crate::TournamentResult>> = BTreeMap::new();
    for row in results {
        groups
            .entry(row.speed.clone())
            .or_default()
            .push(row.result.clone());
    }
    groups
        .into_iter()
        .map(|(speed, group)| SpeedPivotRow {
            speed,
            kpis: compute_results_kpis(&group, TicketValuation::FaceValue),
        })
        .collect()
}

/// Export CSV du pivot par vitesse.
#[must_use]
pub fn speed_pivot_to_csv(rows: &[SpeedPivotRow]) -> String {
    let mut csv = format!("speed;{KPI_CSV_HEADER}\n");
    for row in rows {
        let _ = writeln!(
            csv,
            "{};{}",
            row.speed.clone().unwrap_or_default(),
            kpi_csv_fields(&row.kpis),
        );
    }
    csv
}

/// Une ligne du pivot par jour de semaine (PRD §13.2, M6-3 phase 2).
/// `weekday` suit la convention SQLite `strftime('%w', ...)` (UTC) : 0 =
/// dimanche .. 6 = samedi, meme convention que
/// `TournamentResultRow::weekday_utc`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayOfWeekPivotRow {
    pub weekday: i64,
    pub kpis: ResultsKpis,
}

#[must_use]
pub fn pivot_by_day_of_week(results: &[TournamentResultRow]) -> Vec<DayOfWeekPivotRow> {
    group_by_known_key(results, |r| r.weekday_utc)
        .into_iter()
        .map(|(weekday, kpis)| DayOfWeekPivotRow { weekday, kpis })
        .collect()
}

/// Export CSV du pivot par jour de semaine.
#[must_use]
pub fn day_of_week_pivot_to_csv(rows: &[DayOfWeekPivotRow]) -> String {
    let mut csv = format!("weekday;{KPI_CSV_HEADER}\n");
    for row in rows {
        let _ = writeln!(csv, "{};{}", row.weekday, kpi_csv_fields(&row.kpis));
    }
    csv
}

/// Une ligne du pivot par heure UTC (0-23, PRD §13.2, M6-3 phase 2), meme
/// provenance que `TournamentResultRow::hour_utc`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HourPivotRow {
    pub hour: i64,
    pub kpis: ResultsKpis,
}

#[must_use]
pub fn pivot_by_hour(results: &[TournamentResultRow]) -> Vec<HourPivotRow> {
    group_by_known_key(results, |r| r.hour_utc)
        .into_iter()
        .map(|(hour, kpis)| HourPivotRow { hour, kpis })
        .collect()
}

/// Export CSV du pivot par heure.
#[must_use]
pub fn hour_pivot_to_csv(rows: &[HourPivotRow]) -> String {
    let mut csv = format!("hour;{KPI_CSV_HEADER}\n");
    for row in rows {
        let _ = writeln!(csv, "{};{}", row.hour, kpi_csv_fields(&row.kpis));
    }
    csv
}

/// Une ligne du pivot par mois calendaire UTC (1-12, PRD §13.2, M6-3 phase
/// 2), meme provenance que `TournamentResultRow::month_utc`. Regroupe par
/// mois de l'annee (janvier a decembre, toutes annees confondues) plutot que
/// par mois calendaire absolu (`AAAA-MM`) : coherent avec les autres
/// dimensions du pivot complet (jour de semaine, heure), elles aussi
/// recurrentes plutot qu'absolues — le volume par jour absolu existe deja
/// via G6.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthPivotRow {
    pub month: i64,
    pub kpis: ResultsKpis,
}

#[must_use]
pub fn pivot_by_month(results: &[TournamentResultRow]) -> Vec<MonthPivotRow> {
    group_by_known_key(results, |r| r.month_utc)
        .into_iter()
        .map(|(month, kpis)| MonthPivotRow { month, kpis })
        .collect()
}

/// Export CSV du pivot par mois.
#[must_use]
pub fn month_pivot_to_csv(rows: &[MonthPivotRow]) -> String {
    let mut csv = format!("month;{KPI_CSV_HEADER}\n");
    for row in rows {
        let _ = writeln!(csv, "{};{}", row.month, kpi_csv_fields(&row.kpis));
    }
    csv
}
