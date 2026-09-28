use thiserror::Error;

/// Erreur fatale d'import (M2-3). Une main ou un fichier individuel illisible
/// n'en fait pas partie : PAR-15/R-NOPANIC veulent qu'elle soit consignee
/// dans `ImportSummary::failures` sans interrompre le reste de l'import.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error("erreur de stockage: {0}")]
    Store(#[from] gr_store::StoreError),

    /// M2-5 : action "Reparser" sur une erreur d'import qui n'existe plus.
    #[error("erreur d'import {0} introuvable")]
    ImportErrorNotFound(i64),

    /// M2-5 : l'erreur n'a pas de texte brut conserve (ne devrait pas
    /// arriver, `import_one_file` en fournit toujours un).
    #[error("l'erreur d'import {0} n'a pas de texte brut a reparser")]
    NoRawExcerpt(i64),
}
