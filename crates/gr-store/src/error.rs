use std::path::PathBuf;

use thiserror::Error;

/// Erreur de stockage (ouverture de la base, migrations, pool de connexions).
/// R-NOPANIC : toute defaillance SQLite ou systeme de fichiers est typee ici,
/// jamais de `unwrap()`/`expect()` en dehors des tests.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("impossible de creer le dossier de donnees {path}: {source}")]
    CreateDataDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("erreur de pool de connexions: {0}")]
    Pool(#[from] r2d2::Error),

    /// R-SCHEMA : une migration deja appliquee ne doit jamais etre modifiee.
    /// Cette erreur protege contre une derive silencieuse du schema.
    #[error(
        "la migration {version} est deja appliquee avec un contenu different \
         (une migration mergee ne doit jamais etre modifiee, R-SCHEMA)"
    )]
    MigrationChecksumMismatch { version: String },
}
