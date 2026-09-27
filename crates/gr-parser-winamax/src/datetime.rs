//! Conversion d'une date `"AAAA/MM/JJ HH:MM:SS"` (deja debarrassee de son
//! suffixe ` UTC`) en epoch UTC millisecondes (R-MONEY), sans dependance externe.

/// Jours ecoules entre `1970-01-01` et la date civile donnee (algorithme
/// classique de Howard Hinnant, valide sur le calendrier gregorien proleptique).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Parse `"AAAA/MM/JJ HH:MM:SS"` en epoch UTC millisecondes, ou `None` si le
/// format ne correspond pas exactement (PAR-4).
pub(crate) fn parse_utc_datetime(s: &str) -> Option<i64> {
    let (date, time) = s.split_once(' ')?;
    let mut date_parts = date.split('/');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() {
        return None;
    }

    let mut time_parts = time.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;
    if time_parts.next().is_some() {
        return None;
    }

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(seconds * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_real_handid_unix_timestamp() {
        // docs/formats/winamax.md §4.1 : "Timestamp du HandId = date de l'en-tete
        // sur 445/445 mains", verifie sur fixtures/winamax/mtt/space-ko-3max-itm-reentry/
        // (HandId #...-16-1790176801, en-tete "2026/09/23 15:20:01 UTC").
        assert_eq!(
            parse_utc_datetime("2026/09/23 15:20:01"),
            Some(1_790_176_801_000)
        );
    }

    #[test]
    fn round_trips_the_unix_epoch() {
        assert_eq!(parse_utc_datetime("1970/01/01 00:00:00"), Some(0));
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(parse_utc_datetime("not a date"), None);
        assert_eq!(parse_utc_datetime("2026/09/23"), None);
    }
}
