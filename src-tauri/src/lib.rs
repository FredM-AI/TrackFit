#[cfg(test)]
mod bindings_gen;
mod hero_profiles;
mod home;
mod import;
mod import_errors;
mod logs;
mod priority;
mod results;
mod setup;
mod status;
mod tournaments;
mod tray;
mod watch;

use std::sync::Arc;

use import::ImportState;
use tauri::Manager;
use watch::WatcherState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let data_dir = gr_store::resolve_data_dir(&exe_dir);

            let log_guard = logs::init(&data_dir)?;
            app.manage(log_guard);

            let store = Arc::new(gr_store::Store::open(&data_dir)?);
            // PRD §8.5 / BACKLOG M2-6 : un tournoi encore PROVISIONAL sans
            // nouvelle main depuis 24h passe INCOMPLETE, a chaque demarrage.
            store.mark_stale_provisional_tournaments_incomplete(now_ms(), 24)?;

            // M6-3 : rattrapage de hand_players.net_chips/net_bb pour les
            // mains importees avant que ce calcul n'existe (M2-1, jamais
            // fait jusqu'ici). En thread separe (pas dans .setup(), qui
            // bloquerait le demarrage) : ce reparse peut prendre plusieurs
            // dizaines de secondes sur un gros corpus deja importe ; sans
            // effet (quasi instantane) une fois le rattrapage initial fait.
            {
                let store = Arc::clone(&store);
                std::thread::spawn(move || match store.backfill_net_chips() {
                    Ok(0) => {}
                    Ok(n) => log::info!("net_chips retro-rempli pour {n} main(s)"),
                    Err(e) => log::warn!("echec du retro-remplissage net_chips : {e}"),
                });
            }

            app.manage(ImportState {
                store: Arc::clone(&store),
                cancel: gr_ingest::CancelToken::new(),
            });

            let watcher_state = WatcherState::default();
            watch::start_from_persisted_roots(app.handle(), &store, &watcher_state);
            app.manage(watcher_state);

            // PRD §6.2 : ne jamais concurrencer le client Winamax pour le
            // CPU sur cette machine faible (M3-3).
            priority::set_below_normal();
            tray::setup(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import::import_paths,
            import::cancel_import,
            import_errors::list_import_errors,
            import_errors::reparse_import_error_cmd,
            import_errors::ignore_import_error,
            setup::detect_winamax_accounts,
            setup::create_hero_profile_and_accounts,
            setup::is_first_launch_complete,
            setup::mark_first_launch_complete,
            watch::set_watched_roots,
            status::get_status_snapshot,
            tray::mark_tray_notice_shown,
            hero_profiles::list_hero_profiles_cmd,
            hero_profiles::get_active_hero_profile_id,
            hero_profiles::set_active_hero_profile_id,
            hero_profiles::create_hero_profile_cmd,
            hero_profiles::rename_hero_profile_cmd,
            hero_profiles::add_hero_pseudo_cmd,
            hero_profiles::remove_hero_pseudo_cmd,
            home::get_home_snapshot,
            results::get_results_snapshot,
            tournaments::get_tournaments_list,
            tournaments::get_tournament_detail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}
