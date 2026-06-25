use crate::browser::{lock_state, report_error, BrowserState};

use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Position, Rect, Size, WebviewWindow,
};

pub const CHROME_HEIGHT: f64 = 100.0;
pub const MAX_OVERLAY_HEIGHT: f64 = 680.0;

fn logical_size_from_window(win: &WebviewWindow) -> Result<(f64, f64), String> {
    let physical = win.inner_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    Ok((
        physical.width as f64 / scale,
        physical.height as f64 / scale,
    ))
}

fn fallback_logical_size(app: &AppHandle) -> (f64, f64) {
    if let Some(win) = app.get_webview_window("main") {
        match logical_size_from_window(&win) {
            Ok(size) => return size,
            Err(err) => {
                report_error(format_args!("failed to read window size: {err}"));
            }
        }
    }
    (1280.0, 800.0)
}

pub fn update_window_size_from_window(
    state: &BrowserState,
    win: &WebviewWindow,
) -> Result<(), String> {
    let (width, height) = logical_size_from_window(win)?;
    update_window_size(state, width, height)
}

fn get_logical_size(app: &AppHandle, state: &BrowserState) -> (f64, f64) {
    match lock_state(&state.window_size, "window_size") {
        Ok(size) => size.unwrap_or_else(|| fallback_logical_size(app)),
        Err(err) => {
            report_error(format_args!("failed to read cached window size: {err}"));
            fallback_logical_size(app)
        }
    }
}

fn get_overlay_height(state: &BrowserState) -> f64 {
    match lock_state(&state.overlay_height, "overlay_height") {
        Ok(height) => height.clamp(0.0, MAX_OVERLAY_HEIGHT),
        Err(err) => {
            report_error(format_args!("failed to read overlay height: {err}"));
            0.0
        }
    }
}

pub fn update_window_size(state: &BrowserState, width: f64, height: f64) -> Result<(), String> {
    *lock_state(&state.window_size, "window_size")? = Some((width.max(1.0), height.max(1.0)));
    Ok(())
}

pub fn active_webview_bounds(app: &AppHandle, state: &BrowserState) -> Rect {
    let (width, height) = get_logical_size(app, state);
    bounds_for_window_size(width, height, CHROME_HEIGHT + get_overlay_height(state))
}

pub fn live_webview_bounds(app: &AppHandle) -> Rect {
    let (width, height) = fallback_logical_size(app);
    bounds_for_window_size(width, height, CHROME_HEIGHT)
}

fn bounds_for_window_size(width: f64, height: f64, chrome_height: f64) -> Rect {
    let chrome_height = chrome_height.max(0.0);
    let page_height = (height - chrome_height).max(1.0);
    Rect {
        position: Position::Logical(LogicalPosition::new(0.0, chrome_height)),
        size: Size::Logical(LogicalSize::new(width.max(1.0), page_height)),
    }
}

pub fn sync_visible_webviews(
    app: &AppHandle,
    state: &BrowserState,
    active_id: &str,
) -> Result<(), String> {
    let bounds = active_webview_bounds(app, state);
    let (tab_ids, active_has_page) = {
        let tabs = lock_state(&state.tabs, "tabs")?;
        let tab_ids: Vec<String> = tabs.keys().cloned().collect();
        let active_has_page = tabs
            .get(active_id)
            .map(|tab| {
                let url = tab.info.url.to_ascii_lowercase();
                url.starts_with("http://") || url.starts_with("https://")
            })
            .unwrap_or(false);
        (tab_ids, active_has_page)
    };

    for id in tab_ids {
        if let Some(wv) = app.get_webview(&id) {
            if id == active_id && active_has_page {
                wv.set_bounds(bounds).map_err(|e| e.to_string())?;
                wv.show().map_err(|e| e.to_string())?;
            } else {
                wv.hide().map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

pub fn resize_active_webview(app: &AppHandle) {
    let state = app.state::<BrowserState>();
    let bounds = active_webview_bounds(app, state.inner());
    let active = match lock_state(&state.active_tab, "active_tab") {
        Ok(active) => active.clone(),
        Err(err) => {
            report_error(format_args!("failed to read active tab: {err}"));
            None
        }
    };
    if let Some(id) = active {
        if let Some(wv) = app.get_webview(&id) {
            if let Err(err) = wv.set_bounds(bounds) {
                report_error(format_args!("failed to resize active webview: {err}"));
            }
        }
    }
}

pub fn resize_active_webview_to(app: &AppHandle, width: f64, height: f64) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    update_window_size(state.inner(), width, height)?;
    resize_active_webview(app);
    Ok(())
}

pub fn set_active_webview_obscured(app: &AppHandle, obscured: bool) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let active = lock_state(&state.active_tab, "active_tab")?.clone();
    let Some(id) = active else {
        return Ok(());
    };

    let active_has_page = {
        let tabs = lock_state(&state.tabs, "tabs")?;
        tabs.get(&id)
            .map(|tab| {
                let url = tab.info.url.to_ascii_lowercase();
                url.starts_with("http://") || url.starts_with("https://")
            })
            .unwrap_or(false)
    };

    let Some(wv) = app.get_webview(&id) else {
        return Ok(());
    };

    if obscured || !active_has_page {
        wv.hide().map_err(|e| e.to_string())?;
    } else {
        wv.set_bounds(active_webview_bounds(app, state.inner()))
            .map_err(|e| e.to_string())?;
        wv.show().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_bounds_fill_window_below_chrome() {
        let bounds = bounds_for_window_size(1261.0, 1337.0, CHROME_HEIGHT);
        match bounds.size {
            Size::Logical(size) => {
                assert_eq!(size.width, 1261.0);
                assert_eq!(size.height, 1237.0);
            }
            Size::Physical(_) => panic!("browser page bounds should stay in logical units"),
        }
    }

    #[test]
    fn page_bounds_never_collapse() {
        let bounds = bounds_for_window_size(0.0, 10.0, CHROME_HEIGHT);
        match bounds.size {
            Size::Logical(size) => {
                assert_eq!(size.width, 1.0);
                assert_eq!(size.height, 1.0);
            }
            Size::Physical(_) => panic!("browser page bounds should stay in logical units"),
        }
    }
}
