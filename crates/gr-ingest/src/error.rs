use thiserror::Error;

/// Erreur fatale d'import (M2-3). Une main ou un fichier individuel illisible
/// n'en fait pas partie : PAR-15/R-NOPANIC veulent qu'elle soit consignee
/// dans `ImportSummary::failures` sans interrompre le reste de l'import.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error("erreur de stockage: {0}")]
    Store(#[from] gr_store::StoreError),
}
