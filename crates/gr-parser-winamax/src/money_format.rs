/// Parse un montant en euros du format Winamax (`"1"`, `"0.80"`, `"31.41"`) en
/// centimes. Toujours 0 ou 2 decimales dans le corpus observe (§5.2, §8.2-4).
pub(crate) fn parse_euros_to_cents(s: &str) -> Option<i64> {
    let (whole, cents) = match s.split_once('.') {
        Some((whole, cents)) if cents.len() == 2 => (whole, cents),
        None => (s, "00"),
        Some(_) => return None,
    };
    let whole: i64 = whole.parse().ok()?;
    let cents: i64 = cents.parse().ok()?;
    Some(whole * 100 + cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whole_euros() {
        assert_eq!(parse_euros_to_cents("1"), Some(100));
        assert_eq!(parse_euros_to_cents("4"), Some(400));
    }

    #[test]
    fn parses_two_decimals() {
        assert_eq!(parse_euros_to_cents("0.80"), Some(80));
        assert_eq!(parse_euros_to_cents("31.41"), Some(3141));
        assert_eq!(parse_euros_to_cents("18.38"), Some(1838));
    }

    #[test]
    fn rejects_a_single_decimal_digit() {
        // Jamais observe dans le corpus : ne pas inventer une regle non vue (R-FORMAT).
        assert_eq!(parse_euros_to_cents("1.4"), None);
    }
}
