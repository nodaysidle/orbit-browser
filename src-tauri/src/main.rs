#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock;
mod browser;
mod db;
mod download;
mod layout;
mod tabs;

use adblock::AdBlocker;
use browser::{lock_state, report_error, BrowserState, TabData, TabInfo};
use db::Db;
use download::download_file;
use layout::{resize_active_webview, resize_active_webview_to, MAX_OVERLAY_HEIGHT};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tabs::{
    close_tab, create_tab, find_in_page, get_active_tab, get_tabs, go_back, go_forward,
    go_home_tab, navigate_tab, reload_tab, reorder_tabs, reset_zoom, set_reader_mode, set_tab_zoom,
    stop_tab, switch_tab,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};

const MENU_ABOUT: &str = "orbit:about";
const MENU_SETTINGS: &str = "orbit:settings";
const MENU_NEW_TAB: &str = "orbit:new-tab";
const MENU_CLOSE_TAB: &str = "orbit:close-tab";
const MENU_FOCUS_ADDRESS: &str = "orbit:focus-address";
const MENU_RELOAD: &str = "orbit:reload";
const MENU_STOP: &str = "orbit:stop";
const MENU_HOME: &str = "orbit:home";
const MENU_BACK: &str = "orbit:back";
const MENU_FORWARD: &str = "orbit:forward";
const MENU_FIND: &str = "orbit:find";
const MENU_ZOOM_IN: &str = "orbit:zoom-in";
const MENU_ZOOM_OUT: &str = "orbit:zoom-out";
const MENU_ACTUAL_SIZE: &str = "orbit:actual-size";
const MENU_ENTER_FULLSCREEN: &str = "orbit:enter-full-screen";
const MENU_SHOW_BOOKMARKS: &str = "orbit:show-bookmarks";
const MENU_SHOW_HISTORY: &str = "orbit:show-history";
const MENU_MINIMIZE: &str = "orbit:minimize";
const MENU_ZOOM_WINDOW: &str = "orbit:zoom-window";

// ── DB Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
fn add_bookmark(
    db: tauri::State<'_, Db>,
    url: String,
    title: String,
) -> Result<db::Bookmark, String> {
    db.add_bookmark(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_bookmarks(db: tauri::State<'_, Db>) -> Result<Vec<db::Bookmark>, String> {
    db.get_bookmarks().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_bookmark(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_bookmark(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_bookmarked(db: tauri::State<'_, Db>, url: String) -> Result<bool, String> {
    db.is_bookmarked(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history(
    db: tauri::State<'_, Db>,
    limit: i64,
    offset: i64,
) -> Result<Vec<db::HistoryEntry>, String> {
    db.get_history(limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_history(
    db: tauri::State<'_, Db>,
    query: String,
) -> Result<Vec<db::HistoryEntry>, String> {
    db.search_history(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(db: tauri::State<'_, Db>) -> Result<(), String> {
    db.clear_history().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_setting(db: tauri::State<'_, Db>, key: String) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(db: tauri::State<'_, Db>, key: String, value: String) -> Result<(), String> {
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn sync_browser_view(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() {
        return Err("invalid browser viewport size".to_string());
    }
    resize_active_webview_to(&app, width, height)
}

#[tauri::command]
fn set_overlay_height(
    app: tauri::AppHandle,
    state: tauri::State<'_, BrowserState>,
    height: f64,
) -> Result<(), String> {
    if !height.is_finite() {
        return Err("invalid overlay height".to_string());
    }
    let height = height.clamp(0.0, MAX_OVERLAY_HEIGHT);
    *lock_state(&state.overlay_height, "overlay_height")? = height;
    resize_active_webview(&app);
    Ok(())
}

fn install_browser_menu(app: &tauri::AppHandle) -> Result<(), String> {
    let about = MenuItem::with_id(app, MENU_ABOUT, "About Orbit", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "Settings…", true, Some("CmdOrCtrl+,"))
        .map_err(|e| e.to_string())?;
    let app_separator = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit Orbit")).map_err(|e| e.to_string())?;
    let app_menu = Submenu::with_items(
        app,
        "Orbit",
        true,
        &[&about, &settings, &app_separator, &quit],
    )
    .map_err(|e| e.to_string())?;

    let new_tab = MenuItem::with_id(app, MENU_NEW_TAB, "New Tab", true, Some("CmdOrCtrl+T"))
        .map_err(|e| e.to_string())?;
    let close_tab = MenuItem::with_id(app, MENU_CLOSE_TAB, "Close Tab", true, Some("CmdOrCtrl+W"))
        .map_err(|e| e.to_string())?;
    let focus_address = MenuItem::with_id(
        app,
        MENU_FOCUS_ADDRESS,
        "Focus Address",
        true,
        Some("CmdOrCtrl+L"),
    )
    .map_err(|e| e.to_string())?;
    let file_menu = Submenu::with_items(app, "File", true, &[&new_tab, &close_tab, &focus_address])
        .map_err(|e| e.to_string())?;

    let undo = PredefinedMenuItem::undo(app, Some("Undo")).map_err(|e| e.to_string())?;
    let redo = PredefinedMenuItem::redo(app, Some("Redo")).map_err(|e| e.to_string())?;
    let edit_separator_1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let cut = PredefinedMenuItem::cut(app, Some("Cut")).map_err(|e| e.to_string())?;
    let copy = PredefinedMenuItem::copy(app, Some("Copy")).map_err(|e| e.to_string())?;
    let paste = PredefinedMenuItem::paste(app, Some("Paste")).map_err(|e| e.to_string())?;
    let edit_separator_2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let select_all =
        PredefinedMenuItem::select_all(app, Some("Select All")).map_err(|e| e.to_string())?;
    let find = MenuItem::with_id(app, MENU_FIND, "Find", true, Some("CmdOrCtrl+F"))
        .map_err(|e| e.to_string())?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &edit_separator_1,
            &cut,
            &copy,
            &paste,
            &edit_separator_2,
            &select_all,
            &find,
        ],
    )
    .map_err(|e| e.to_string())?;

    let back = MenuItem::with_id(app, MENU_BACK, "Back", true, Some("CmdOrCtrl+["))
        .map_err(|e| e.to_string())?;
    let forward = MenuItem::with_id(app, MENU_FORWARD, "Forward", true, Some("CmdOrCtrl+]"))
        .map_err(|e| e.to_string())?;
    let reload = MenuItem::with_id(app, MENU_RELOAD, "Reload", true, Some("CmdOrCtrl+R"))
        .map_err(|e| e.to_string())?;
    let stop = MenuItem::with_id(app, MENU_STOP, "Stop", true, Some("CmdOrCtrl+."))
        .map_err(|e| e.to_string())?;
    let home = MenuItem::with_id(app, MENU_HOME, "Home", true, Some("CmdOrCtrl+Shift+H"))
        .map_err(|e| e.to_string())?;
    let view_separator_1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let zoom_in = MenuItem::with_id(app, MENU_ZOOM_IN, "Zoom In", true, Some("CmdOrCtrl+="))
        .map_err(|e| e.to_string())?;
    let zoom_out = MenuItem::with_id(app, MENU_ZOOM_OUT, "Zoom Out", true, Some("CmdOrCtrl+-"))
        .map_err(|e| e.to_string())?;
    let actual_size = MenuItem::with_id(
        app,
        MENU_ACTUAL_SIZE,
        "Actual Size",
        true,
        Some("CmdOrCtrl+0"),
    )
    .map_err(|e| e.to_string())?;
    let view_separator_2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let enter_fullscreen = MenuItem::with_id(
        app,
        MENU_ENTER_FULLSCREEN,
        "Enter Full Screen",
        true,
        Some("CmdOrCtrl+Control+F"),
    )
    .map_err(|e| e.to_string())?;
    let view_separator_3 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let show_bookmarks = MenuItem::with_id(
        app,
        MENU_SHOW_BOOKMARKS,
        "Show Bookmarks",
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let show_history =
        MenuItem::with_id(app, MENU_SHOW_HISTORY, "Show History", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &back,
            &forward,
            &reload,
            &stop,
            &home,
            &view_separator_1,
            &zoom_in,
            &zoom_out,
            &actual_size,
            &view_separator_2,
            &enter_fullscreen,
            &view_separator_3,
            &show_bookmarks,
            &show_history,
        ],
    )
    .map_err(|e| e.to_string())?;

    let minimize = MenuItem::with_id(app, MENU_MINIMIZE, "Minimize", true, Some("CmdOrCtrl+M"))
        .map_err(|e| e.to_string())?;
    let zoom_window = MenuItem::with_id(app, MENU_ZOOM_WINDOW, "Zoom", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let window_menu = Submenu::with_items(app, "Window", true, &[&minimize, &zoom_window])
        .map_err(|e| e.to_string())?;

    let help_menu = Submenu::new(app, "Help", true).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ],
    )
    .map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    app.on_menu_event(|app, event| {
        let action = match event.id().as_ref() {
            MENU_ABOUT => {
                if let Err(err) = app.emit("orbit-about", ()) {
                    report_error(format_args!("failed to emit about event: {err}"));
                }
                return;
            }
            MENU_SETTINGS => "settings",
            MENU_NEW_TAB => "new-tab",
            MENU_CLOSE_TAB => "close-tab",
            MENU_FOCUS_ADDRESS => "focus-address",
            MENU_RELOAD => "reload",
            MENU_STOP => "stop",
            MENU_HOME => "home",
            MENU_BACK => "back",
            MENU_FORWARD => "forward",
            MENU_FIND => "find",
            MENU_ZOOM_IN => "zoom-in",
            MENU_ZOOM_OUT => "zoom-out",
            MENU_ACTUAL_SIZE => "actual-size",
            MENU_ENTER_FULLSCREEN => {
                if let Some(window) = app.get_webview_window("main") {
                    let next = !window.is_fullscreen().unwrap_or(false);
                    if let Err(err) = window.set_fullscreen(next) {
                        report_error(format_args!("failed to toggle full screen: {err}"));
                    }
                }
                return;
            }
            MENU_SHOW_BOOKMARKS => "show-bookmarks",
            MENU_SHOW_HISTORY => "show-history",
            MENU_MINIMIZE => {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(err) = window.minimize() {
                        report_error(format_args!("failed to minimize window: {err}"));
                    }
                }
                return;
            }
            MENU_ZOOM_WINDOW => {
                if let Some(window) = app.get_webview_window("main") {
                    let result = if window.is_maximized().unwrap_or(false) {
                        window.unmaximize()
                    } else {
                        window.maximize()
                    };
                    if let Err(err) = result {
                        report_error(format_args!("failed to zoom window: {err}"));
                    }
                }
                return;
            }
            _ => return,
        };
        if let Err(err) = app.emit("orbit-shortcut", action) {
            report_error(format_args!("failed to emit native shortcut: {err}"));
        }
    });
    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app data dir unavailable: {e}"))?
                .join("orbit.db");
            let db_dir = db_path
                .parent()
                .ok_or_else(|| "invalid app data path".to_string())?;
            std::fs::create_dir_all(db_dir)
                .map_err(|e| format!("failed to create app data dir: {e}"))?;
            let db =
                Db::open(&db_path).map_err(|e| format!("failed to open Orbit database: {e}"))?;
            app.manage(db);

            let blocker = match app.path().resource_dir() {
                Ok(resource_dir) => {
                    let blocklist_path = resource_dir.join("resources/adblock-patterns.json");
                    if blocklist_path.exists() {
                        AdBlocker::load_from_json(&blocklist_path)
                    } else {
                        AdBlocker::new(vec![])
                    }
                }
                Err(err) => {
                    report_error(format_args!(
                        "resource dir unavailable, ad blocking disabled: {err}"
                    ));
                    AdBlocker::new(vec![])
                }
            };
            app.manage(blocker);

            // Initialize browser state
            let browser_state = BrowserState::new();
            app.manage(browser_state);

            // Restore previous session
            {
                let db = app.state::<Db>();
                match db.load_session() {
                    Ok((urls, active_index)) => {
                        let mut restored_ids = Vec::new();
                        for url in urls {
                            let id =
                                format!("t{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
                            let history = if url.is_empty() {
                                Vec::new()
                            } else {
                                vec![url.clone()]
                            };
                            let info = TabInfo {
                                id: id.clone(),
                                url: url.clone(),
                                title: if url.is_empty() {
                                    "New Tab".to_string()
                                } else {
                                    browser::title_from_url(&url)
                                },
                                loading: false,
                                can_go_back: false,
                                can_go_forward: false,
                            };
                            let browser_state = app.state::<BrowserState>();
                            lock_state(&browser_state.tabs, "tabs")?.insert(
                                id.clone(),
                                TabData {
                                    info,
                                    history,
                                    history_idx: 0,
                                    has_webview: false,
                                    pending_history_idx: None,
                                    popup_block_url: None,
                                },
                            );
                            restored_ids.push(id);
                        }
                        let browser_state = app.state::<BrowserState>();
                        *lock_state(&browser_state.tab_order, "tab_order")? = restored_ids.clone();
                        if let Some(id) = active_index
                            .and_then(|idx| restored_ids.get(idx).cloned())
                            .or_else(|| restored_ids.first().cloned())
                        {
                            *lock_state(&browser_state.active_tab, "active_tab")? = Some(id);
                        }
                    }
                    Err(err) => {
                        report_error(format_args!("failed to restore session: {err}"));
                    }
                }
            }

            install_browser_menu(app.handle())?;

            // Window resize handler — update stored logical size, then reposition webview
            let app_h = app.handle().clone();
            let last_window_sync = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(16)));
            let main = app
                .get_webview_window("main")
                .ok_or_else(|| "main window not found".to_string())?;
            main.on_window_event(move |event| {
                if matches!(
                    event,
                    tauri::WindowEvent::Resized(_)
                        | tauri::WindowEvent::ScaleFactorChanged { .. }
                        | tauri::WindowEvent::Moved(_)
                ) {
                    if let Ok(mut last) = last_window_sync.lock() {
                        if last.elapsed() < Duration::from_millis(16) {
                            return;
                        }
                        *last = Instant::now();
                    }

                    // Refresh the cached logical size from the window's actual state
                    if let Some(win) = app_h.get_webview_window("main") {
                        let state = app_h.state::<BrowserState>();
                        if let Err(err) =
                            layout::update_window_size_from_window(state.inner(), &win)
                        {
                            report_error(format_args!("failed to refresh window size: {err}"));
                        }
                    }
                    resize_active_webview(&app_h);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_tab,
            switch_tab,
            close_tab,
            navigate_tab,
            go_back,
            go_forward,
            reload_tab,
            stop_tab,
            go_home_tab,
            get_tabs,
            get_active_tab,
            reorder_tabs,
            find_in_page,
            set_tab_zoom,
            reset_zoom,
            set_reader_mode,
            add_bookmark,
            get_bookmarks,
            delete_bookmark,
            is_bookmarked,
            get_history,
            search_history,
            clear_history,
            get_setting,
            set_setting,
            sync_browser_view,
            set_overlay_height,
            download_file,
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        report_error(format_args!("tauri runtime error: {err}"));
    }
}
