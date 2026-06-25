use crate::adblock::AdBlocker;
use crate::browser::{
    lock_state, next_active_after_close, normalize_url, ordered_tab_infos, report_error,
    title_from_url, BrowserState, TabData, TabInfo,
};
use crate::db::Db;
use crate::download::is_download_url;
use crate::layout::{live_webview_bounds, sync_visible_webviews};

use std::collections::{HashMap, HashSet};

use tauri::webview::{NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, Window};

fn save_current_session(app: &AppHandle, state: &BrowserState) {
    let tabs: Vec<TabInfo> = match (
        lock_state(&state.tabs, "tabs"),
        lock_state(&state.tab_order, "tab_order"),
    ) {
        (Ok(tabs), Ok(order)) => ordered_tab_infos(&tabs, &order),
        (Err(err), _) | (_, Err(err)) => {
            report_error(format_args!("failed to save session: {err}"));
            return;
        }
    };
    let active = match lock_state(&state.active_tab, "active_tab") {
        Ok(active) => active.clone(),
        Err(err) => {
            report_error(format_args!("failed to save session: {err}"));
            return;
        }
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let db = app.state::<Db>();
        if let Err(err) = db.save_session(&tabs, active.as_deref()) {
            report_error(format_args!("failed to save session: {err}"));
        }
        if let Err(err) = db.save_active_project_snapshot_from_settings(&tabs, active.as_deref()) {
            report_error(format_args!(
                "failed to save active project snapshot: {err}"
            ));
        }
        if let Err(err) = db.save_detected_project(&tabs, active.as_deref()) {
            report_error(format_args!("failed to save detected project: {err}"));
        }
    });
}

fn tab_exists(state: &BrowserState, tab_id: &str) -> bool {
    match lock_state(&state.tabs, "tabs") {
        Ok(tabs) => tabs.contains_key(tab_id),
        Err(err) => {
            report_error(format_args!("failed to verify tab existence: {err}"));
            false
        }
    }
}

fn is_allowed_navigation_scheme(scheme: &str) -> bool {
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

fn ensure_navigation_allowed(app: &AppHandle, url: &str) -> Result<url::Url, String> {
    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    if !is_allowed_navigation_scheme(parsed.scheme()) {
        return Err(format!(
            "Blocked by Orbit: unsupported URL scheme '{}'",
            parsed.scheme()
        ));
    }
    if app.state::<AdBlocker>().is_blocked(&parsed) {
        return Err(format!("Blocked by Orbit: {}", title_from_url(url)));
    }
    Ok(parsed)
}

#[derive(Clone)]
struct TabStateSnapshot {
    url: String,
    title: String,
    loading: bool,
    pending_history_idx: Option<usize>,
}

fn snapshot_tab_state(tab: &TabData) -> TabStateSnapshot {
    TabStateSnapshot {
        url: tab.info.url.clone(),
        title: tab.info.title.clone(),
        loading: tab.info.loading,
        pending_history_idx: tab.pending_history_idx,
    }
}

fn restore_tab_state(tab: &mut TabData, snapshot: TabStateSnapshot) {
    tab.info.url = snapshot.url;
    tab.info.title = snapshot.title;
    tab.info.loading = snapshot.loading;
    tab.pending_history_idx = snapshot.pending_history_idx;
}

fn emit_blocked_navigation(app: &AppHandle, tab_id: &str, blocked_url: &url::Url) {
    let info = {
        let browser_state = app.state::<BrowserState>();
        let Ok(mut tabs) = lock_state(&browser_state.tabs, "tabs") else {
            report_error("failed to mark blocked tab");
            return;
        };
        let Some(td) = tabs.get_mut(tab_id) else {
            return;
        };
        td.info.loading = false;
        td.info.clone()
    };
    if let Err(err) = app.emit(
        "tab-blocked",
        serde_json::json!({
            "id": tab_id,
            "blockedUrl": blocked_url.to_string(),
            "tab": info,
        }),
    ) {
        report_error(format_args!("failed to emit tab-blocked: {err}"));
    }
}

fn emit_tab_sync(app: &AppHandle, tab_id: &str) {
    let info = {
        let browser_state = app.state::<BrowserState>();
        let Ok(tabs) = lock_state(&browser_state.tabs, "tabs") else {
            report_error("failed to read tab for sync");
            return;
        };
        let Some(tab) = tabs.get(tab_id) else {
            return;
        };
        tab.info.clone()
    };
    if let Err(err) = app.emit("tab-loaded", &info) {
        report_error(format_args!("failed to emit tab sync: {err}"));
    }
}

fn is_likely_embedded_page_event(existing_url: &str, candidate_url: &str) -> bool {
    let Ok(existing) = url::Url::parse(existing_url) else {
        return false;
    };
    let Ok(candidate) = url::Url::parse(candidate_url) else {
        return false;
    };
    let existing_host = existing.host_str().unwrap_or_default();
    let candidate_host = candidate.host_str().unwrap_or_default();
    if existing_host.is_empty() || candidate_host.is_empty() || existing_host == candidate_host {
        return false;
    }

    let candidate_path = candidate.path();
    (existing_host.ends_with("youtube.com")
        && candidate_host == "accounts.youtube.com"
        && candidate_path.contains("RotateCookiesPage"))
        || (existing_host.contains("google.")
            && candidate_host == "ogs.google.com"
            && candidate_path.contains("/widget/"))
}

fn should_ignore_page_event(app: &AppHandle, tab_id: &str, candidate_url: &str) -> bool {
    let browser_state = app.state::<BrowserState>();
    let Ok(tabs) = lock_state(&browser_state.tabs, "tabs") else {
        return false;
    };
    tabs.get(tab_id)
        .map(|tab| is_likely_embedded_page_event(&tab.info.url, candidate_url))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn create_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    url: String,
    make_active: bool,
) -> Result<TabInfo, String> {
    let id = format!("t{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
    let raw_url = url.trim();
    let is_blank = raw_url.is_empty() || raw_url == "about:blank";
    let clean_url = if is_blank {
        String::new()
    } else {
        normalize_url(raw_url)
    };
    if !is_blank {
        ensure_navigation_allowed(&app, &clean_url)?;
    }

    let tab_info = TabInfo {
        id: id.clone(),
        url: clean_url.clone(),
        title: if clean_url.is_empty() {
            "New Tab".to_string()
        } else {
            title_from_url(&clean_url)
        },
        loading: false,
        can_go_back: false,
        can_go_forward: false,
    };

    {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        tabs.insert(id.clone(), TabData::new(tab_info.clone()));
    }
    lock_state(&state.tab_order, "tab_order")?.push(id.clone());

    let previous_active = lock_state(&state.active_tab, "active_tab")?.clone();
    if make_active {
        *lock_state(&state.active_tab, "active_tab")? = Some(id.clone());
    }

    if !is_blank {
        if let Err(err) = create_webview_for_tab(&app, &state, &id, &clean_url, make_active).await {
            lock_state(&state.tabs, "tabs")?.remove(&id);
            lock_state(&state.tab_order, "tab_order")?.retain(|tab_id| tab_id != &id);
            if make_active {
                *lock_state(&state.active_tab, "active_tab")? = previous_active;
                if let Some(active_id) = lock_state(&state.active_tab, "active_tab")?.clone() {
                    let _ = sync_visible_webviews(&app, state.inner(), &active_id);
                }
            }
            return Err(err);
        }
    }

    if make_active {
        sync_visible_webviews(&app, state.inner(), &id)?;
    }

    // Persist session
    save_current_session(&app, state.inner());

    Ok(tab_info)
}

async fn create_webview_for_tab(
    app: &AppHandle,
    state: &tauri::State<'_, BrowserState>,
    tab_id: &str,
    url: &str,
    visible: bool,
) -> Result<(), String> {
    let parsed_url = ensure_navigation_allowed(app, url)?;
    let webview_url = WebviewUrl::External(parsed_url);
    // Always query the live window size for webview creation — cached window_size
    // may be stale if AeroSpace tiled the window between HTML load and first navigation.
    let bounds = live_webview_bounds(app);
    let main_win: Window<tauri::Wry> = app.get_window("main").ok_or("no main window")?;

    let id_c = tab_id.to_string();
    let app_c = app.clone();
    let app_nav = app.clone();
    let id_nav = tab_id.to_string();

    let blocker = app.state::<AdBlocker>().inner().clone();
    let blocker_nav = blocker.clone();
    let blocker_popup = blocker.clone();
    let app_popup = app.clone();
    let id_popup = tab_id.to_string();

    let wv = main_win
        .add_child(
            WebviewBuilder::new(tab_id, webview_url)
                .on_navigation(move |nav_url| {
                    if nav_url.scheme().eq_ignore_ascii_case("about") {
                        return true;
                    }
                    let url_str = nav_url.to_string();
                    if should_ignore_page_event(&app_nav, &id_nav, &url_str) {
                        return true;
                    }
                    {
                        let browser_state = app_nav.state::<BrowserState>();
                        let mut tabs = match lock_state(&browser_state.tabs, "tabs") {
                            Ok(tabs) => tabs,
                            Err(_) => return true,
                        };
                        let Some(td) = tabs.get_mut(&id_nav) else {
                            return false;
                        };
                        if td
                            .popup_block_url
                            .as_deref()
                            .is_some_and(|blocked| blocked == url_str.as_str())
                        {
                            td.popup_block_url = None;
                            drop(tabs);
                            emit_tab_sync(&app_nav, &id_nav);
                            return false;
                        }
                    }
                    if !is_allowed_navigation_scheme(nav_url.scheme()) {
                        emit_blocked_navigation(&app_nav, &id_nav, nav_url);
                        return false;
                    }
                    if blocker_nav.is_blocked(nav_url) {
                        emit_blocked_navigation(&app_nav, &id_nav, nav_url);
                        return false;
                    }
                    // Detect downloadable files from link clicks
                    if is_download_url(&url_str) {
                        let _ = app_nav.emit(
                            "download-detected",
                            serde_json::json!({
                                "url": url_str,
                                "tab_id": id_nav,
                            }),
                        );
                        emit_tab_sync(&app_nav, &id_nav);
                        return false;
                    }
                    let title = title_from_url(&url_str);
                    let browser_state = app_nav.state::<BrowserState>();
                    match lock_state(&browser_state.tabs, "tabs") {
                        Ok(mut tabs) => {
                            if let Some(td) = tabs.get_mut(&id_nav) {
                                td.mark_loading(&url_str, &title);
                            }
                        }
                        Err(err) => {
                            report_error(format_args!("failed to mark tab loading: {err}"));
                        }
                    }
                    if let Err(err) = app_nav.emit(
                        "tab-navigating",
                        serde_json::json!({
                            "id": id_nav.clone(),
                            "url": url_str,
                            "title": title
                        }),
                    ) {
                        report_error(format_args!("failed to emit tab-navigating: {err}"));
                    }
                    true
                })
                .on_new_window(move |url, _features| {
                    if url.scheme().eq_ignore_ascii_case("about") {
                        return NewWindowResponse::Deny;
                    }
                    if !is_allowed_navigation_scheme(url.scheme()) {
                        return NewWindowResponse::Deny;
                    }
                    if blocker_popup.is_blocked(&url) {
                        emit_blocked_navigation(&app_popup, &id_popup, &url);
                        return NewWindowResponse::Deny;
                    }

                    let url_str = url.to_string();
                    {
                        let browser_state = app_popup.state::<BrowserState>();
                        let mut tabs = match lock_state(&browser_state.tabs, "tabs") {
                            Ok(tabs) => tabs,
                            Err(_) => return NewWindowResponse::Deny,
                        };
                        if let Some(td) = tabs.get_mut(&id_popup) {
                            td.popup_block_url = Some(url_str.clone());
                        }
                    }
                    if is_download_url(&url_str) {
                        let _ = app_popup.emit(
                            "download-detected",
                            serde_json::json!({
                                "url": url_str,
                                "tab_id": id_popup,
                            }),
                        );
                        emit_tab_sync(&app_popup, &id_popup);
                        return NewWindowResponse::Deny;
                    }

                    let _ = app_popup.emit(
                        "tab-new-window",
                        serde_json::json!({
                            "tabId": id_popup,
                            "url": url_str,
                        }),
                    );
                    NewWindowResponse::Deny
                })
                .on_page_load(move |wv_handle, payload| {
                    use tauri::webview::PageLoadEvent;
                    let url_str = wv_handle
                        .url()
                        .map(|url| url.to_string())
                        .unwrap_or_else(|_| payload.url().to_string());
                    if should_ignore_page_event(&app_c, &id_c, &url_str) {
                        return;
                    }
                    let browser_state = app_c.state::<BrowserState>();
                    if !tab_exists(browser_state.inner(), &id_c) {
                        return;
                    }
                    let db = app_c.state::<Db>();

                    match payload.event() {
                        PageLoadEvent::Started => {
                            let title = title_from_url(&url_str);
                            match lock_state(&browser_state.tabs, "tabs") {
                                Ok(mut tabs) => {
                                    if let Some(td) = tabs.get_mut(&id_c) {
                                        td.mark_loading(&url_str, &title);
                                    }
                                }
                                Err(err) => {
                                    report_error(format_args!("failed to mark tab loading: {err}"));
                                }
                            }
                            if let Err(err) = app_c.emit(
                                "tab-loading",
                                serde_json::json!({
                                    "id": id_c,
                                    "url": url_str,
                                    "title": title
                                }),
                            ) {
                                report_error(format_args!("failed to emit tab-loading: {err}"));
                            }
                        }
                        PageLoadEvent::Finished => {
                            let title = title_from_url(&url_str);
                            let info = {
                                let Ok(mut tabs) = lock_state(&browser_state.tabs, "tabs") else {
                                    report_error("failed to commit loaded tab");
                                    return;
                                };
                                let Some(td) = tabs.get_mut(&id_c) else {
                                    return;
                                };
                                td.commit_loaded_url(&url_str, &title);
                                td.info.clone()
                            };
                            if !url_str.starts_with("about:") {
                                if let Err(err) = db.add_history(&url_str, &title) {
                                    report_error(format_args!(
                                        "failed to persist history entry: {err}"
                                    ));
                                }
                            }
                            save_current_session(&app_c, browser_state.inner());
                            if let Err(err) = app_c.emit("tab-loaded", &info) {
                                report_error(format_args!("failed to emit tab-loaded: {err}"));
                            }
                            // Emit favicon URL for display
                            if let Some(favicon_url) = favicon_from_url(&url_str) {
                                let _ = app_c.emit(
                                    "tab-favicon",
                                    serde_json::json!({
                                        "id": id_c,
                                        "faviconUrl": favicon_url,
                                    }),
                                );
                            }
                        }
                    }
                }),
            bounds.position,
            bounds.size,
        )
        .map_err(|e| e.to_string())?;

    if !visible {
        wv.hide().map_err(|e| e.to_string())?;
    }

    let mut tabs = lock_state(&state.tabs, "tabs")?;
    if let Some(td) = tabs.get_mut(tab_id) {
        td.has_webview = true;
        td.info.url = url.to_string();
        td.info.title = title_from_url(url);
        td.info.loading = true;
    }

    Ok(())
}

#[tauri::command]
pub async fn switch_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let (url, has_webview) = {
        let tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        (tab.info.url.clone(), tab.has_webview)
    };

    if !url.is_empty() && !has_webview {
        create_webview_for_tab(&app, &state, &tab_id, &url, true).await?;
    }

    *lock_state(&state.active_tab, "active_tab")? = Some(tab_id.clone());
    sync_visible_webviews(&app, state.inner(), &tab_id)?;
    save_current_session(&app, state.inner());
    Ok(())
}

#[tauri::command]
pub async fn close_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<Option<String>, String> {
    if let Some(wv) = app.get_webview(&tab_id) {
        wv.close().map_err(|e| e.to_string())?;
    }

    let next_id = {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let mut order = lock_state(&state.tab_order, "tab_order")?;
        let next_id = next_active_after_close(&order, &tab_id);
        tabs.remove(&tab_id);
        order.retain(|id| id != &tab_id);
        next_id
    };

    let active_id = {
        let mut active = lock_state(&state.active_tab, "active_tab")?;
        if active.as_deref() == Some(&tab_id) {
            *active = next_id;
        }
        active.clone()
    };

    if let Some(active_id) = active_id.as_deref() {
        sync_visible_webviews(&app, state.inner(), active_id)?;
    }

    save_current_session(&app, state.inner());
    Ok(active_id)
}

#[tauri::command]
pub async fn navigate_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    let clean = normalize_url(&url);
    let title = title_from_url(&clean);

    // Check if this is a downloadable file — emit event and let frontend handle it
    if is_download_url(&clean) {
        let _ = app.emit(
            "download-detected",
            serde_json::json!({
                "url": clean,
                "tab_id": tab_id,
            }),
        );
        emit_tab_sync(&app, &tab_id);
        return Ok(());
    }

    let parsed_url = ensure_navigation_allowed(&app, &clean)?;
    let (has_webview, rollback) = {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        let rollback = snapshot_tab_state(tab);
        tab.clear_pending_history();
        tab.info.url = clean.clone();
        tab.info.title = title.clone();
        tab.info.loading = true;
        (tab.has_webview, rollback)
    };

    if has_webview {
        let wv = app
            .get_webview(&tab_id)
            .ok_or_else(|| format!("webview not found: {tab_id}"))?;
        if let Err(err) = wv.navigate(parsed_url) {
            if let Ok(mut tabs) = lock_state(&state.tabs, "tabs") {
                if let Some(tab) = tabs.get_mut(&tab_id) {
                    restore_tab_state(tab, rollback);
                }
            }
            emit_tab_sync(&app, &tab_id);
            return Err(err.to_string());
        }
    } else {
        let make_active =
            lock_state(&state.active_tab, "active_tab")?.as_deref() == Some(tab_id.as_str());
        if let Err(err) = create_webview_for_tab(&app, &state, &tab_id, &clean, make_active).await {
            if let Ok(mut tabs) = lock_state(&state.tabs, "tabs") {
                if let Some(tab) = tabs.get_mut(&tab_id) {
                    restore_tab_state(tab, rollback);
                }
            }
            emit_tab_sync(&app, &tab_id);
            return Err(err);
        }
        if make_active {
            sync_visible_webviews(&app, state.inner(), &tab_id)?;
        }
    }
    if lock_state(&state.active_tab, "active_tab")?.as_deref() == Some(tab_id.as_str()) {
        sync_visible_webviews(&app, state.inner(), &tab_id)?;
    }
    save_current_session(&app, state.inner());
    Ok(())
}

#[tauri::command]
pub async fn go_back(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let (target_idx, target_url, rollback) = {
        let tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        let Some((target_idx, target_url)) = tab.target_history_url(-1) else {
            return Ok(());
        };
        (target_idx, target_url, snapshot_tab_state(tab))
    };

    let webview = app
        .get_webview(&tab_id)
        .ok_or_else(|| format!("webview not found: {tab_id}"))?;
    let parsed = ensure_navigation_allowed(&app, &target_url)?;
    {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        tab.pending_history_idx = Some(target_idx);
        tab.info.url = target_url.clone();
        tab.info.title = title_from_url(&target_url);
        tab.info.loading = true;
    }
    if lock_state(&state.active_tab, "active_tab")?.as_deref() == Some(tab_id.as_str()) {
        sync_visible_webviews(&app, state.inner(), &tab_id)?;
    }
    if let Err(err) = webview.navigate(parsed) {
        if let Ok(mut tabs) = lock_state(&state.tabs, "tabs") {
            if let Some(tab) = tabs.get_mut(&tab_id) {
                restore_tab_state(tab, rollback);
            }
        }
        emit_tab_sync(&app, &tab_id);
        return Err(err.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn go_forward(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let (target_idx, target_url, rollback) = {
        let tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        let Some((target_idx, target_url)) = tab.target_history_url(1) else {
            return Ok(());
        };
        (target_idx, target_url, snapshot_tab_state(tab))
    };

    let webview = app
        .get_webview(&tab_id)
        .ok_or_else(|| format!("webview not found: {tab_id}"))?;
    let parsed = ensure_navigation_allowed(&app, &target_url)?;
    {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        tab.pending_history_idx = Some(target_idx);
        tab.info.url = target_url.clone();
        tab.info.title = title_from_url(&target_url);
        tab.info.loading = true;
    }
    if lock_state(&state.active_tab, "active_tab")?.as_deref() == Some(tab_id.as_str()) {
        sync_visible_webviews(&app, state.inner(), &tab_id)?;
    }
    if let Err(err) = webview.navigate(parsed) {
        if let Ok(mut tabs) = lock_state(&state.tabs, "tabs") {
            if let Some(tab) = tabs.get_mut(&tab_id) {
                restore_tab_state(tab, rollback);
            }
        }
        emit_tab_sync(&app, &tab_id);
        return Err(err.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn reload_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let url = {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        tab.clear_pending_history();
        if tab.info.url.is_empty() {
            return Err(format!("nothing to reload for tab: {tab_id}"));
        }
        tab.info.url.clone()
    };

    let webview = app
        .get_webview(&tab_id)
        .ok_or_else(|| format!("webview not found: {tab_id}"))?;
    let parsed_url = ensure_navigation_allowed(&app, &url)?;
    webview.navigate(parsed_url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let Some(webview) = app.get_webview(&tab_id) else {
        return Ok(());
    };
    webview.eval("window.stop();").map_err(|e| e.to_string())?;

    let info = {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let Some(tab) = tabs.get_mut(&tab_id) else {
            return Ok(());
        };
        tab.clear_pending_history();
        tab.info.loading = false;
        tab.info.clone()
    };
    if let Err(err) = app.emit("tab-loaded", &info) {
        report_error(format_args!("failed to emit tab-loaded after stop: {err}"));
    }
    Ok(())
}

#[tauri::command]
pub async fn go_home_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let info = {
        let mut tabs = lock_state(&state.tabs, "tabs")?;
        let tab = tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("tab not found: {tab_id}"))?;
        tab.clear_pending_history();
        tab.info.url.clear();
        tab.info.title = "New Tab".to_string();
        tab.info.loading = false;
        tab.info.can_go_back = tab.history_idx > 0;
        tab.info.can_go_forward = tab.history_idx + 1 < tab.history.len();
        tab.info.clone()
    };

    if lock_state(&state.active_tab, "active_tab")?.as_deref() == Some(tab_id.as_str()) {
        sync_visible_webviews(&app, state.inner(), &tab_id)?;
    }
    save_current_session(&app, state.inner());
    if let Err(err) = app.emit("tab-loaded", &info) {
        report_error(format_args!(
            "failed to emit tab-loaded after go-home: {err}"
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn get_tabs(state: tauri::State<'_, BrowserState>) -> Result<Vec<TabInfo>, String> {
    let tabs = lock_state(&state.tabs, "tabs")?;
    let order = lock_state(&state.tab_order, "tab_order")?;
    Ok(ordered_tab_infos(&tabs, &order))
}

#[tauri::command]
pub fn get_active_tab(state: tauri::State<'_, BrowserState>) -> Result<Option<String>, String> {
    Ok(lock_state(&state.active_tab, "active_tab")?.clone())
}

#[tauri::command]
pub fn reorder_tabs(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    ordered_ids: Vec<String>,
) -> Result<(), String> {
    {
        let tabs = lock_state(&state.tabs, "tabs")?;
        validate_tab_order(&tabs, &ordered_ids)?;
    }
    *lock_state(&state.tab_order, "tab_order")? = ordered_ids;
    save_current_session(&app, state.inner());
    Ok(())
}

fn validate_tab_order(
    tabs: &HashMap<String, TabData>,
    ordered_ids: &[String],
) -> Result<(), String> {
    let unique_ids: HashSet<&String> = ordered_ids.iter().collect();
    if unique_ids.len() != ordered_ids.len() {
        return Err("invalid tab order: duplicate tab id".to_string());
    }
    if ordered_ids.len() != tabs.len() || ordered_ids.iter().any(|id| !tabs.contains_key(id)) {
        return Err("invalid tab order".to_string());
    }
    Ok(())
}

// ── Favicon ────────────────────────────────────────────────────────────────────

/// Construct a best-effort favicon URL from the page URL.
/// Uses only the page origin to avoid leaking visited hosts to third parties.
fn favicon_from_url(page_url: &str) -> Option<String> {
    let parsed = url::Url::parse(page_url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let origin = parsed.origin().ascii_serialization();
    Some(format!("{origin}/favicon.ico"))
}

#[tauri::command]
pub async fn find_in_page(
    app: AppHandle,
    tab_id: String,
    query: String,
    backwards: bool,
) -> Result<(), String> {
    let webview = app
        .get_webview(&tab_id)
        .ok_or_else(|| format!("webview not found: {tab_id}"))?;
    if query.is_empty() {
        // Clear search highlights
        let _ = webview.eval("window.getSelection().removeAllRanges();");
        return Ok(());
    }
    let safe_query = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let js = format!("window.find({safe_query}, false, {backwards}, true, false, false, false);");
    webview.eval(&js).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Zoom ──────────────────────────────────────────────────────────────────────

const MIN_ZOOM_LEVEL: f64 = 0.5;
const MAX_ZOOM_LEVEL: f64 = 3.0;
const DEFAULT_ZOOM_LEVEL: f64 = 1.0;

#[tauri::command]
pub async fn set_tab_zoom(app: AppHandle, tab_id: String, zoom_level: f64) -> Result<f64, String> {
    let webview = app
        .get_webview(&tab_id)
        .ok_or_else(|| format!("webview not found: {tab_id}"))?;
    let applied = clamp_zoom_level(zoom_level);
    let js = zoom_script(applied);
    webview.eval(&js).map_err(|e| e.to_string())?;
    Ok(applied)
}

#[tauri::command]
pub async fn reset_zoom(app: AppHandle, tab_id: String) -> Result<(), String> {
    let _ = set_tab_zoom(app, tab_id, DEFAULT_ZOOM_LEVEL).await?;
    Ok(())
}

fn clamp_zoom_level(value: f64) -> f64 {
    if !value.is_finite() {
        return DEFAULT_ZOOM_LEVEL;
    }
    (value.clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL) * 10.0).round() / 10.0
}

fn zoom_script(zoom_level: f64) -> String {
    let applied = format!("{:.1}", clamp_zoom_level(zoom_level));
    format!(
        "(() => {{ const orbitZoom = '{applied}'; document.documentElement.style.zoom = orbitZoom; if (document.body) document.body.style.zoom = orbitZoom; }})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::filename_from_url;

    fn test_tab_data(id: &str, url: &str) -> TabData {
        TabData::new(TabInfo {
            id: id.into(),
            url: url.into(),
            title: title_from_url(url),
            loading: false,
            can_go_back: false,
            can_go_forward: false,
        })
    }

    #[test]
    fn test_tab_exists_guards_late_callbacks_after_close() {
        let state = BrowserState::new();
        lock_state(&state.tabs, "tabs")
            .unwrap()
            .insert("t1".into(), test_tab_data("t1", "https://example.com"));

        assert!(tab_exists(&state, "t1"));
        lock_state(&state.tabs, "tabs").unwrap().remove("t1");
        assert!(!tab_exists(&state, "t1"));
    }

    #[test]
    fn test_navigation_scheme_filter_allows_only_http_family() {
        assert!(is_allowed_navigation_scheme("https"));
        assert!(is_allowed_navigation_scheme("HTTP"));
        assert!(!is_allowed_navigation_scheme("file"));
        assert!(!is_allowed_navigation_scheme("javascript"));
    }

    #[test]
    fn test_validate_tab_order_rejects_ghost_entries() {
        let mut tabs = HashMap::new();
        tabs.insert("a".into(), test_tab_data("a", "https://a.example"));
        tabs.insert("b".into(), test_tab_data("b", "https://b.example"));

        assert!(validate_tab_order(&tabs, &["b".into(), "a".into()]).is_ok());
        assert!(validate_tab_order(&tabs, &["b".into(), "ghost".into()]).is_err());
        assert!(validate_tab_order(&tabs, &["b".into()]).is_err());
    }

    #[test]
    fn test_validate_tab_order_rejects_duplicate_entries() {
        let mut tabs = HashMap::new();
        tabs.insert("a".into(), test_tab_data("a", "https://a.example"));
        tabs.insert("b".into(), test_tab_data("b", "https://b.example"));

        let err = validate_tab_order(&tabs, &["a".into(), "a".into()]).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn test_embedded_page_events_do_not_replace_top_level_tab_url() {
        assert!(is_likely_embedded_page_event(
            "https://www.youtube.com/",
            "https://accounts.youtube.com/RotateCookiesPage?origin=https%3A%2F%2Fwww.youtube.com"
        ));
        assert!(is_likely_embedded_page_event(
            "https://www.google.com/",
            "https://ogs.google.com/u/0/widget/app?origin=https%3A%2F%2Fwww.google.com"
        ));
        assert!(!is_likely_embedded_page_event(
            "https://www.youtube.com/",
            "https://www.youtube.com/watch?v=abc"
        ));
        assert!(!is_likely_embedded_page_event(
            "https://example.com/",
            "https://accounts.youtube.com/RotateCookiesPage"
        ));
    }

    #[test]
    fn test_favicon_from_url_basic() {
        let result = favicon_from_url("https://example.com/page").unwrap();
        assert!(result.contains("example.com/favicon.ico"));
        assert!(!result.contains("google.com/s2/favicons"));
    }

    #[test]
    fn test_favicon_from_url_subdomain() {
        let result = favicon_from_url("https://blog.example.com/post").unwrap();
        assert!(result.contains("blog.example.com/favicon.ico"));
    }

    #[test]
    fn test_favicon_from_url_https() {
        let result = favicon_from_url("https://github.com/rust-lang/rust").unwrap();
        assert!(result.contains("github.com/favicon.ico"));
    }

    #[test]
    fn test_zoom_level_is_clamped_and_rounded() {
        assert_eq!(clamp_zoom_level(1.04), 1.0);
        assert_eq!(clamp_zoom_level(1.05), 1.1);
        assert_eq!(clamp_zoom_level(99.0), MAX_ZOOM_LEVEL);
        assert_eq!(clamp_zoom_level(f64::NAN), DEFAULT_ZOOM_LEVEL);
    }

    #[test]
    fn test_favicon_from_url_about_blank_is_none() {
        assert!(favicon_from_url("about:blank").is_none());
    }

    #[test]
    fn test_is_download_url_csv() {
        assert!(!is_download_url("https://data.example.com/export.csv"));
    }

    #[test]
    fn test_is_download_url_json() {
        assert!(!is_download_url("https://api.example.com/data.json"));
    }

    #[test]
    fn test_is_download_url_deb() {
        assert!(is_download_url("https://repo.example.com/package.deb"));
    }

    #[test]
    fn test_is_not_download_url_php() {
        assert!(!is_download_url("https://example.com/page.php"));
    }

    #[test]
    fn test_filename_from_url_github_release() {
        assert_eq!(
            filename_from_url("https://github.com/user/repo/releases/download/v1.0/app.dmg"),
            "app.dmg"
        );
    }

    #[test]
    fn test_filename_from_url_no_extension() {
        let result = filename_from_url("https://example.com/download");
        assert!(result.starts_with("download_"));
    }
}
