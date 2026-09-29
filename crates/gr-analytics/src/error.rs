use thiserror::Error;

/// Erreur de requetage d'un `AnalyticsBackend` (PRD §13.5). R-NOPANIC :
/// toute defaillance SQLite est typee ici, jamais de `unwrap()`/`expect()`
/// en dehors des tests.
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
