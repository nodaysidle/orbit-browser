#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browser;
mod db;
mod adblock;

use browser::{normalize_url, title_from_url, BrowserState, TabData, TabInfo};
use db::Db;
use adblock::AdBlocker;

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager,
    WebviewUrl, Rect, Position, Size, Window,
};
use tauri::webview::WebviewBuilder;

const CHROME_HEIGHT: f64 = 108.0;

fn get_logical_size(app: &AppHandle) -> (f64, f64) {
    let win = app.get_webview_window("main").expect("main window");
    let size = win.inner_size().unwrap_or_default();
    let scale = win.scale_factor().unwrap_or(1.0);
    (size.width as f64 / scale, size.height as f64 / scale)
}

// ── Tab Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
async fn create_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    url: String,
    make_active: bool,
) -> Result<TabInfo, String> {
    let id = format!("t{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
    let is_blank = url.trim().is_empty() || url.trim() == "about:blank";

    let tab_info = TabInfo {
        id: id.clone(),
        url: if is_blank { String::new() } else { normalize_url(&url) },
        title: "New Tab".to_string(),
        loading: false,
        can_go_back: false,
        can_go_forward: false,
    };

    {
        let mut tabs = state.tabs.lock().unwrap();
        tabs.insert(id.clone(), TabData {
            info: tab_info.clone(),
            history: Vec::new(),
            history_idx: 0,
            has_webview: false,
        });
    }

    if make_active {
        *state.active_tab.lock().unwrap() = Some(id.clone());
    }

    // If URL provided, actually create the webview
    if !is_blank {
        create_webview_for_tab(&app, &state, &id, &normalize_url(&url), make_active)
            .await
            .map_err(|e: String| e)?;
    }

    Ok(tab_info)
}

async fn create_webview_for_tab(
    app: &AppHandle,
    state: &tauri::State<'_, BrowserState>,
    tab_id: &str,
    url: &str,
    visible: bool,
) -> Result<(), String> {
    let webview_url = WebviewUrl::External(
        url.parse().map_err(|e: url::ParseError| e.to_string())?
    );
    let (lw, lh) = get_logical_size(app);
    let main_win: Window<tauri::Wry> = app.get_window("main").ok_or("no main window")?;

    let id_c = tab_id.to_string();
    let app_c = app.clone();
    let app_nav = app.clone();
    let id_nav = tab_id.to_string();

    // Clone blocked domains Arc for the on_navigation closure
    let blocked = app.state::<AdBlocker>().arc_domains();

    let wv = main_win.add_child(
        WebviewBuilder::new(tab_id, webview_url)
            .on_navigation(move |nav_url| {
                // Block ad domains
                if let Some(host) = nav_url.host_str() {
                    if blocked.contains(host) {
                        return false;
                    }
                    for domain in blocked.iter() {
                        if host.ends_with(&format!(".{domain}")) {
                            return false;
                        }
                    }
                }
                // Emit navigation start to update address bar immediately
                let _ = app_nav.emit("tab-navigating", serde_json::json!({
                    "id": id_nav,
                    "url": nav_url.to_string()
                }));
                true
            })
            .on_page_load(move |_wv, payload| {
                use tauri::webview::PageLoadEvent;
                let url_str = payload.url().to_string();
                let browser_state = app_c.state::<BrowserState>();
                let db = app_c.state::<Db>();

                match payload.event() {
                    PageLoadEvent::Started => {
                        let mut tabs = browser_state.tabs.lock().unwrap();
                        if let Some(td) = tabs.get_mut(&id_c) {
                            td.info.loading = true;
                            td.info.url = url_str.clone();
                        }
                        let _ = app_c.emit("tab-loading", serde_json::json!({
                            "id": id_c,
                            "url": url_str
                        }));
                    }
                    PageLoadEvent::Finished => {
                        let title = title_from_url(&url_str);
                        let info = {
                            let mut tabs = browser_state.tabs.lock().unwrap();
                            let Some(td) = tabs.get_mut(&id_c) else { return };
                            // Update history stack
                            if td.history_idx > 0 && td.history.get(td.history_idx - 1).map(|u| u == &url_str).unwrap_or(false) {
                                td.history_idx -= 1;
                            } else if td.history_idx + 1 < td.history.len() && td.history[td.history_idx + 1] == url_str {
                                td.history_idx += 1;
                            } else if td.history.last().map(|u| u != &url_str).unwrap_or(true) {
                                td.history.truncate(td.history_idx + 1);
                                td.history.push(url_str.clone());
                                td.history_idx = td.history.len() - 1;
                            }
                            td.info.loading = false;
                            td.info.url = url_str.clone();
                            td.info.title = title.clone();
                            td.info.can_go_back = td.history_idx > 0;
                            td.info.can_go_forward = td.history_idx + 1 < td.history.len();
                            td.info.clone()
                        };
                        // Persist to history (non-blocking)
                        if !url_str.starts_with("about:") {
                            let _ = db.add_history(&url_str, &title);
                        }
                        let _ = app_c.emit("tab-loaded", &info);
                    }
                }
            }),
        LogicalPosition::new(0.0, CHROME_HEIGHT),
        LogicalSize::new(lw, lh - CHROME_HEIGHT),
    ).map_err(|e| e.to_string())?;

    if !visible {
        let _ = wv.hide();
    }

    // Mark webview as created in state
    let mut tabs = state.tabs.lock().unwrap();
    if let Some(td) = tabs.get_mut(tab_id) {
        td.has_webview = true;
    }

    Ok(())
}

#[tauri::command]
async fn switch_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let (lw, lh) = get_logical_size(&app);
    let tabs = state.tabs.lock().unwrap();

    for id in tabs.keys() {
        if let Some(wv) = app.get_webview(id) {
            if *id == tab_id {
                let _ = wv.set_bounds(Rect {
                    position: Position::Logical(LogicalPosition::new(0.0, CHROME_HEIGHT)),
                    size: Size::Logical(LogicalSize::new(lw, lh - CHROME_HEIGHT)),
                });
                let _ = wv.show();
            } else {
                let _ = wv.hide();
            }
        }
    }
    drop(tabs);

    *state.active_tab.lock().unwrap() = Some(tab_id);
    Ok(())
}

#[tauri::command]
async fn close_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<Option<String>, String> {
    // Close native webview if it exists
    if let Some(wv) = app.get_webview(&tab_id) {
        wv.close().map_err(|e| e.to_string())?;
    }

    let mut tabs = state.tabs.lock().unwrap();
    tabs.remove(&tab_id);

    let mut active = state.active_tab.lock().unwrap();
    if active.as_deref() == Some(&tab_id) {
        let new_id = tabs.keys().next().cloned();
        *active = new_id.clone();
        return Ok(new_id);
    }
    Ok(active.clone())
}

#[tauri::command]
async fn navigate_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    let clean = normalize_url(&url);
    let has_webview = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| t.has_webview).unwrap_or(false);

    if has_webview {
        if let Some(wv) = app.get_webview(&tab_id) {
            wv.navigate(clean.parse().map_err(|e: url::ParseError| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
    } else {
        // Lazy creation — first navigation on a blank tab
        let make_active = state.active_tab.lock().unwrap().as_deref() == Some(tab_id.as_str());
        create_webview_for_tab(&app, &state, &tab_id, &clean, make_active).await?;
    }
    Ok(())
}

#[tauri::command]
async fn go_back(app: AppHandle, state: tauri::State<'_, BrowserState>, tab_id: String) -> Result<(), String> {
    let can = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| t.history_idx > 0).unwrap_or(false);
    if can {
        if let Some(wv) = app.get_webview(&tab_id) {
            let _ = wv.eval("history.back()");
        }
    }
    Ok(())
}

#[tauri::command]
async fn go_forward(app: AppHandle, state: tauri::State<'_, BrowserState>, tab_id: String) -> Result<(), String> {
    let can = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| {
            t.history_idx + 1 < t.history.len()
        }).unwrap_or(false);
    if can {
        if let Some(wv) = app.get_webview(&tab_id) {
            let _ = wv.eval("history.forward()");
        }
    }
    Ok(())
}

#[tauri::command]
async fn reload_tab(app: AppHandle, tab_id: String) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&tab_id) {
        let _ = wv.eval("location.reload()");
    }
    Ok(())
}

#[tauri::command]
fn get_tabs(state: tauri::State<'_, BrowserState>) -> Vec<TabInfo> {
    state.tabs.lock().unwrap().values().map(|td| td.info.clone()).collect()
}

#[tauri::command]
fn get_active_tab(state: tauri::State<'_, BrowserState>) -> Option<String> {
    state.active_tab.lock().unwrap().clone()
}

// ── DB Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
fn add_bookmark(db: tauri::State<'_, Db>, url: String, title: String) -> Result<db::Bookmark, String> {
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
fn get_history(db: tauri::State<'_, Db>, limit: i64, offset: i64) -> Result<Vec<db::HistoryEntry>, String> {
    db.get_history(limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_history(db: tauri::State<'_, Db>, query: String) -> Result<Vec<db::HistoryEntry>, String> {
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

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let db_path = app.path().app_data_dir()
                .expect("app data dir")
                .join("orbit.db");
            std::fs::create_dir_all(db_path.parent().unwrap()).ok();
            let db = Db::open(&db_path).expect("open db");
            app.manage(db);

            // Initialize ad blocker
            let blocklist_path = app.path().resource_dir()
                .expect("resource dir")
                .join("resources/adblock-patterns.json");
            let blocker = if blocklist_path.exists() {
                AdBlocker::load_from_json(&blocklist_path)
            } else {
                AdBlocker::new(vec![])
            };
            app.manage(blocker);

            // Initialize browser state
            app.manage(BrowserState::new());

            // Window resize handler — update active webview bounds
            let app_h = app.handle().clone();
            let main = app.get_webview_window("main").unwrap();
            main.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(size) = event {
                    let scale = app_h.get_webview_window("main")
                        .map(|w| w.scale_factor().unwrap_or(1.0))
                        .unwrap_or(1.0);
                    let lw = size.width as f64 / scale;
                    let lh = size.height as f64 / scale;

                    let state = app_h.state::<BrowserState>();
                    let active = state.active_tab.lock().unwrap().clone();
                    if let Some(id) = active {
                        if let Some(wv) = app_h.get_webview(&id) {
                            let _ = wv.set_bounds(Rect {
                                position: Position::Logical(LogicalPosition::new(0.0, CHROME_HEIGHT)),
                                size: Size::Logical(LogicalSize::new(lw, lh - CHROME_HEIGHT)),
                            });
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_tab, switch_tab, close_tab, navigate_tab,
            go_back, go_forward, reload_tab, get_tabs, get_active_tab,
            add_bookmark, get_bookmarks, delete_bookmark, is_bookmarked,
            get_history, search_history, clear_history,
            get_setting, set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
