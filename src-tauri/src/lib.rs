mod import;
mod import_errors;
mod logs;

use std::sync::Arc;

use import::ImportState;
use tauri::Manager;

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

            let store = gr_store::Store::open(&data_dir)?;
            app.manage(ImportState {
                store: Arc::new(store),
                cancel: gr_ingest::CancelToken::new(),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import::import_paths,
            import::cancel_import,
            import_errors::list_import_errors,
            import_errors::reparse_import_error_cmd,
            import_errors::ignore_import_error,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
