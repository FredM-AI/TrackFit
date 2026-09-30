//! Etat + synchronisation DuckDB (M7-6, PRD §7.3 point 8/ADR-002). Ce
//! module compile toujours et sa commande [`reconstruct_analytics_index`]
//! est toujours enregistree : pas de branchement conditionnel cote
//! frontend selon la feature cargo `analytics-duckdb` (`just dev` vs
//! `just dev-full`, ADR-004). Sans la feature, [`AnalyticsSyncState::notify`]
//! est un no-op et [`reconstruct_analytics_index`] renvoie une erreur
//! explicite plutot que d'exister ou non selon le build.
//!
//! **Un seul proprietaire de la connexion DuckDB** : un thread dedie recoit
//! les demandes (notification de nouvelles mains, ou reconstruction) par un
//! canal et est seul a toucher la `duckdb::Connection` — evite d'avoir a la
//! rendre `Sync`/de la proteger par un mutex. La reconstruction bloque sur
//! une reponse (canal aller-retour improvise avec `mpsc`, pas de crate
//! "oneshot" pour un seul appel occasionnel) ; une simple notification ne
//! bloque jamais l'appelant.

use tauri::State;

#[cfg(feature = "analytics-duckdb")]
mod enabled {
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    use gr_store::Store;

    pub enum SyncRequest {
        Notify,
        Rebuild(mpsc::Sender<Result<usize, String>>),
    }

    /// Debounce de 5 s (PRD §7.3 point 8) : une notification declenche une
    /// attente ; toute notification suivante dans la fenetre la repousse.
    /// La synchronisation ne s'execute qu'une fois 5 s de silence atteintes,
    /// ou immediatement pour une reconstruction (bouton Parametres, pas de
    /// raison de la retarder).
    const DEBOUNCE: Duration = Duration::from_secs(5);

    pub struct Inner {
        sender: mpsc::Sender<SyncRequest>,
    }

    impl Inner {
        pub fn notify(&self) {
            let _ = self.sender.send(SyncRequest::Notify);
        }

        pub fn rebuild(&self) -> Result<usize, String> {
            let (reply_tx, reply_rx) = mpsc::channel();
            self.sender
                .send(SyncRequest::Rebuild(reply_tx))
                .map_err(|_| "le thread de synchronisation DuckDB s'est arrete".to_string())?;
            reply_rx
                .recv()
                .map_err(|_| "le thread de synchronisation DuckDB n'a pas repondu".to_string())?
        }
    }

    /// Ouvre `analytics.duckdb` dans `data_dir` et demarre le thread de
    /// synchronisation. `None` si l'ouverture echoue (journalise, l'app ne
    /// doit pas refuser de demarrer pour autant).
    pub fn spawn(store: Arc<Store>, data_dir: &std::path::Path) -> Option<Inner> {
        let db_path: PathBuf = data_dir.join("analytics.duckdb");
        let duck = match gr_analytics::open_duckdb(&db_path) {
            Ok(conn) => conn,
            Err(e) => {
                log::warn!("ouverture de analytics.duckdb echouee : {e}");
                return None;
            }
        };

        let (tx, rx) = mpsc::channel::<SyncRequest>();
        std::thread::spawn(move || sync_loop(&store, &duck, &rx));
        Some(Inner { sender: tx })
    }

    fn sync_loop(
        store: &Store,
        duck: &gr_analytics::DuckDbConnection,
        rx: &mpsc::Receiver<SyncRequest>,
    ) {
        loop {
            let Ok(first) = rx.recv() else { return };
            let mut rebuild_reply = None;
            match first {
                SyncRequest::Rebuild(reply) => rebuild_reply = Some(reply),
                SyncRequest::Notify => loop {
                    match rx.recv_timeout(DEBOUNCE) {
                        Ok(SyncRequest::Notify) => {} // repousse la fenetre, reboucle
                        Ok(SyncRequest::Rebuild(reply)) => {
                            rebuild_reply = Some(reply);
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                },
            }

            let result = run_sync(store, duck, rebuild_reply.is_some());
            if let Some(reply) = rebuild_reply {
                let _ = reply.send(result);
            } else if let Err(e) = result {
                log::warn!("echec de synchronisation DuckDB: {e}");
            }
        }
    }

    fn run_sync(
        store: &Store,
        duck: &gr_analytics::DuckDbConnection,
        rebuild: bool,
    ) -> Result<usize, String> {
        let reader = store.reader().map_err(|e| e.to_string())?;
        if rebuild {
            gr_analytics::rebuild(duck, &reader).map_err(|e| e.to_string())
        } else {
            gr_analytics::sync_incremental(duck, &reader).map_err(|e| e.to_string())
        }
    }
}

/// Etat gere par Tauri (`app.manage`), toujours present. `Some` seulement
/// si la feature `analytics-duckdb` est active et que l'ouverture du
/// fichier a reussi.
pub struct AnalyticsSyncState {
    #[cfg(feature = "analytics-duckdb")]
    inner: Option<enabled::Inner>,
}

impl AnalyticsSyncState {
    #[cfg(feature = "analytics-duckdb")]
    pub fn start(store: std::sync::Arc<gr_store::Store>, data_dir: &std::path::Path) -> Self {
        Self {
            inner: enabled::spawn(store, data_dir),
        }
    }

    #[cfg(not(feature = "analytics-duckdb"))]
    pub fn start(_store: std::sync::Arc<gr_store::Store>, _data_dir: &std::path::Path) -> Self {
        Self {}
    }

    /// Signale qu'une synchronisation est utile (nouvelles mains importees).
    /// No-op sans la feature, ou si l'ouverture de `analytics.duckdb` a
    /// echoue au demarrage.
    pub fn notify(&self) {
        #[cfg(feature = "analytics-duckdb")]
        if let Some(inner) = &self.inner {
            inner.notify();
        }
    }
}

/// Reconstruit entierement l'index analytique DuckDB (bouton Parametres,
/// PRD §7.3/ADR-002). Erreur explicite si `analytics-duckdb` n'est pas
/// compilee dans ce build, ou si l'ouverture au demarrage a echoue.
#[tauri::command]
pub fn reconstruct_analytics_index(state: State<'_, AnalyticsSyncState>) -> Result<usize, String> {
    #[cfg(feature = "analytics-duckdb")]
    {
        match &state.inner {
            Some(inner) => inner.rebuild(),
            None => Err("l'index analytique DuckDB n'a pas pu s'ouvrir au demarrage".to_string()),
        }
    }
    #[cfg(not(feature = "analytics-duckdb"))]
    {
        let _ = state;
        Err(
            "cette version n'a pas ete compilee avec le backend DuckDB (analytics-duckdb)"
                .to_string(),
        )
    }
}
