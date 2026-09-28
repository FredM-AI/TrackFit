//! Logs applicatifs (M2-5, PRD D32, architecture `tracing`/`tracing-subscriber`/`tracing-appender`) :
//! fichier tournant quotidien dans `<data_dir>/logs/`, retention glissante de
//! 30 jours purgee au demarrage.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tracing_appender::non_blocking::WorkerGuard;

const RETENTION_DAYS: u64 = 30;
const LOG_FILE_PREFIX: &str = "graphite.log";

#[must_use]
pub fn log_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

/// Initialise le subscriber `tracing` global (fichier tournant quotidien) et
/// purge les fichiers de plus de 30 jours. A appeler une seule fois, au
/// demarrage ; le `WorkerGuard` renvoye doit rester vivant tant que
/// l'application tourne (il vide le tampon non-bloquant a sa destruction).
///
/// # Errors
/// Renvoie une erreur E/S si `<data_dir>/logs/` ne peut pas etre cree ou
/// parcouru.
pub fn init(data_dir: &Path) -> std::io::Result<WorkerGuard> {
    let dir = log_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    prune_old_logs(&dir, RETENTION_DAYS, SystemTime::now())?;

    let appender = tracing_appender::rolling::daily(&dir, LOG_FILE_PREFIX);
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    let subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();
    // Deja installe (ex. tests unitaires) : on ignore plutot que paniquer.
    let _ = tracing::subscriber::set_global_default(subscriber);

    Ok(guard)
}

/// Supprime les fichiers de `dir` dont la derniere modification remonte a
/// plus de `retention_days` jours. `now` est un parametre (plutot que
/// `SystemTime::now()` en dur) pour rester testable de maniere deterministe.
///
/// # Errors
/// Renvoie une erreur E/S si `dir` existe mais ne peut pas etre parcouru
/// (un dossier absent n'est pas une erreur : rien a purger).
pub fn prune_old_logs(dir: &Path, retention_days: u64, now: SystemTime) -> std::io::Result<usize> {
    let cutoff = now
        .checked_sub(Duration::from_secs(
            retention_days.saturating_mul(24 * 3600),
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };

    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified < cutoff && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch_with_age(path: &Path, age: Duration) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .expect("create log fixture file");
        let mtime = SystemTime::now()
            .checked_sub(age)
            .expect("age should not underflow SystemTime");
        file.set_modified(mtime).expect("set mtime");
    }

    #[test]
    fn removes_only_files_older_than_the_retention_window() {
        let dir = tempfile::tempdir().expect("temp dir");
        let old = dir.path().join("graphite.log.2020-01-01");
        let recent = dir.path().join("graphite.log.2026-09-28");
        touch_with_age(&old, Duration::from_secs(40 * 24 * 3600));
        touch_with_age(&recent, Duration::from_secs(2 * 24 * 3600));

        let removed = prune_old_logs(dir.path(), RETENTION_DAYS, SystemTime::now())
            .expect("prune should succeed");

        assert_eq!(removed, 1);
        assert!(!old.exists());
        assert!(recent.exists());
    }

    #[test]
    fn does_nothing_on_a_missing_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("does-not-exist");
        let removed = prune_old_logs(&missing, RETENTION_DAYS, SystemTime::now())
            .expect("a missing directory should not be an error");
        assert_eq!(removed, 0);
    }

    #[test]
    fn keeps_everything_when_nothing_is_old_enough() {
        let dir = tempfile::tempdir().expect("temp dir");
        let recent = dir.path().join("graphite.log.today");
        touch_with_age(&recent, Duration::from_secs(3600));

        let removed = prune_old_logs(dir.path(), RETENTION_DAYS, SystemTime::now())
            .expect("prune should succeed");

        assert_eq!(removed, 0);
        assert!(recent.exists());
    }

    #[test]
    fn log_dir_is_a_logs_subfolder_of_the_data_dir() {
        let data_dir = Path::new("C:/graphite-data");
        assert_eq!(log_dir(data_dir), data_dir.join("logs"));
    }
}
