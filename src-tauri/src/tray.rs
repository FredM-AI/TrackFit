//! Icone systeme (M3-3) : Ouvrir, Pause/Reprendre l'import, bascule "garder
//! actif en arriere-plan a la fermeture", Quitter. Tant que cette bascule
//! est activee (par defaut, decision Frederic 29/09), fermer la fenetre
//! principale la masque au lieu de quitter l'app (le watcher continue).

use gr_store::Store;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent, Wry};

use crate::import::ImportState;
use crate::watch::{self, WatcherRunState, WatcherState};

const CLOSE_TO_TRAY_SETTING_KEY: &str = "close_to_tray";
const TRAY_NOTICE_SHOWN_SETTING_KEY: &str = "tray_notice_shown";
const TRAY_ID: &str = "main";
const OPEN_ID: &str = "open";
const TOGGLE_IMPORT_ID: &str = "toggle_import";
const CLOSE_TO_TRAY_ID: &str = "close_to_tray";
const QUIT_ID: &str = "quit";

/// `true` par defaut (decision Frederic 29/09) tant que le reglage n'a
/// jamais ete ecrit.
fn is_close_to_tray_enabled(store: &Store) -> bool {
    store
        .get_setting(CLOSE_TO_TRAY_SETTING_KEY)
        .ok()
        .flatten()
        .is_none_or(|v| v == "true")
}

/// `false` par defaut : le petit mot d'explication (idée backlog du 29/09,
/// suite au retour de Frédéric comme quoi rien n'indique que l'app tourne
/// encore) n'a jamais ete acquitte.
fn is_tray_notice_shown(store: &Store) -> bool {
    store
        .get_setting(TRAY_NOTICE_SHOWN_SETTING_KEY)
        .ok()
        .flatten()
        .is_some_and(|v| v == "true")
}

/// Acquitte le mot d'explication affiche au premier masquage (M3-3,
/// composant React `TrayFirstHideNotice`) : ne sera plus jamais reaffiche.
///
/// # Errors
/// Renvoie une erreur si l'ecriture SQLite echoue.
#[tauri::command]
pub fn mark_tray_notice_shown(state: State<'_, ImportState>) -> Result<(), String> {
    state
        .store
        .set_setting(TRAY_NOTICE_SHOWN_SETTING_KEY, "true")
        .map_err(|e| e.to_string())
}

fn build_menu(
    app: &AppHandle,
    run_state: WatcherRunState,
    close_to_tray: bool,
) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, OPEN_ID, "Ouvrir", true, None::<&str>)?;
    let (toggle_label, toggle_enabled) = match run_state {
        WatcherRunState::Active => ("Mettre l'import en pause", true),
        WatcherRunState::Paused => ("Reprendre l'import", true),
        WatcherRunState::Idle => ("Import inactif", false),
    };
    let toggle_import = MenuItem::with_id(
        app,
        TOGGLE_IMPORT_ID,
        toggle_label,
        toggle_enabled,
        None::<&str>,
    )?;
    let close_to_tray_item = CheckMenuItem::with_id(
        app,
        CLOSE_TO_TRAY_ID,
        "Garder actif en arriere-plan a la fermeture",
        true,
        close_to_tray,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quitter", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &open,
            &toggle_import,
            &PredefinedMenuItem::separator(app)?,
            &close_to_tray_item,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

/// Cree l'icone systeme et branche l'interception de fermeture de la
/// fenetre principale. Appelee une fois au demarrage de l'app, une fois
/// `ImportState`/`WatcherState` geres par Tauri.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let watcher_state = app.state::<WatcherState>();
    let import_state = app.state::<ImportState>();
    let close_to_tray = is_close_to_tray_enabled(&import_state.store);
    let menu = build_menu(app, watcher_state.run_state(), close_to_tray)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID).menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN_ID => show_main_window(app),
            TOGGLE_IMPORT_ID => handle_toggle_import(app),
            CLOSE_TO_TRAY_ID => handle_toggle_close_to_tray(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let import_state = app_handle.state::<ImportState>();
                if is_close_to_tray_enabled(&import_state.store) {
                    api.prevent_close();
                    if is_tray_notice_shown(&import_state.store) {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    } else {
                        // Premier masquage : la fenetre reste visible, c'est
                        // `TrayFirstHideNotice` (React) qui la masquera une
                        // fois le mot d'explication acquitte par l'utilisateur
                        // (mark_tray_notice_shown), pour respecter R-I18N
                        // (pas de texte natif en dur hors du systeme i18next).
                        let _ = app_handle.emit("tray://first-hide-notice", ());
                    }
                }
            }
        });
    }

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_toggle_import(app: &AppHandle) {
    let watcher_state = app.state::<WatcherState>();
    let import_state = app.state::<ImportState>();
    match watcher_state.run_state() {
        WatcherRunState::Active => watch::pause(&watcher_state),
        WatcherRunState::Paused => watch::resume(app, &import_state.store, &watcher_state),
        WatcherRunState::Idle => {}
    }
    refresh_menu(app);
}

fn handle_toggle_close_to_tray(app: &AppHandle) {
    let import_state = app.state::<ImportState>();
    let currently_enabled = is_close_to_tray_enabled(&import_state.store);
    let next = if currently_enabled { "false" } else { "true" };
    let _ = import_state
        .store
        .set_setting(CLOSE_TO_TRAY_SETTING_KEY, next);
    refresh_menu(app);
}

fn refresh_menu(app: &AppHandle) {
    let watcher_state = app.state::<WatcherState>();
    let import_state = app.state::<ImportState>();
    let close_to_tray = is_close_to_tray_enabled(&import_state.store);
    let Ok(menu) = build_menu(app, watcher_state.run_state(), close_to_tray) else {
        return;
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_menu(Some(menu));
    }
}
