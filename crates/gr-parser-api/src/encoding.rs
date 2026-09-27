/// Retire un BOM UTF-8 en tete de fichier, s'il est present (PAR-2 : "tolerer un
/// BOM ou du CRLF par robustesse" — aucun fixture n'en a jusqu'ici, mais Winamax
/// pourrait en ecrire un sur une autre machine/version).
#[must_use]
pub fn strip_bom(bytes: &[u8]) -> &[u8] {
    const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
    bytes.strip_prefix(&UTF8_BOM).unwrap_or(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_a_leading_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"Winamax Poker");
        assert_eq!(strip_bom(&bytes), b"Winamax Poker");
    }

    #[test]
    fn leaves_bom_less_input_untouched() {
        assert_eq!(strip_bom(b"Winamax Poker"), b"Winamax Poker");
    }
}
