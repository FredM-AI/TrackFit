/// Salle de poker dont un fichier a ete detecte (PRD D1 : architecture multi-room par plugins).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Room {
    Winamax,
}

/// Langue du fichier detecte. Constat §1 de `docs/formats/winamax.md` : les fichiers
/// Winamax sont toujours en anglais, meme avec un client en francais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
}

/// Resultat de `RoomParser::detect` : la salle et la langue reconnues dans l'en-tete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Detection {
    pub room: Room,
    pub language: Language,
}
