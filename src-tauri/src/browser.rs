use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Mutex, MutexGuard};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

#[derive(Debug)]
pub struct TabData {
    pub info: TabInfo,
    pub history: Vec<String>,
    pub history_idx: usize,
    pub has_webview: bool,
    /// When back/forward navigation is initiated from the app chrome,
    /// remember the target history index so the next finished load does not
    /// get appended as a fresh navigation.
    pub pending_history_idx: Option<usize>,
    pub popup_block_url: Option<String>,
}

#[derive(Default)]
pub struct BrowserState {
    pub tabs: Mutex<HashMap<String, TabData>>,
    pub tab_order: Mutex<Vec<String>>,
    pub active_tab: Mutex<Option<String>>,
    pub window_size: Mutex<Option<(f64, f64)>>,
    pub overlay_height: Mutex<f64>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn lock_state<'a, T>(mutex: &'a Mutex<T>, name: &str) -> Result<MutexGuard<'a, T>, String> {
    mutex.lock().map_err(|_| {
        format!("browser state lock '{name}' is poisoned; previous operation may have failed")
    })
}

pub fn report_error(message: impl Display) {
    eprintln!("orbit: {message}");
}

pub fn ordered_tab_infos(tabs: &HashMap<String, TabData>, order: &[String]) -> Vec<TabInfo> {
    order
        .iter()
        .filter_map(|id| tabs.get(id).map(|tab| tab.info.clone()))
        .collect()
}

pub fn next_active_after_close(order: &[String], closing_id: &str) -> Option<String> {
    let position = order.iter().position(|id| id == closing_id)?;
    if order.len() <= 1 {
        return None;
    }
    order
        .get(position + 1)
        .or_else(|| position.checked_sub(1).and_then(|idx| order.get(idx)))
        .cloned()
}

impl TabData {
    pub fn new(info: TabInfo) -> Self {
        Self {
            info,
            history: Vec::new(),
            history_idx: 0,
            has_webview: false,
            pending_history_idx: None,
            popup_block_url: None,
        }
    }

    pub fn target_history_url(&self, delta: isize) -> Option<(usize, String)> {
        let current = self.history_idx as isize;
        let next_idx = current.checked_add(delta)?;
        if next_idx < 0 {
            return None;
        }
        let next_idx = next_idx as usize;
        self.history
            .get(next_idx)
            .cloned()
            .map(|url| (next_idx, url))
    }

    pub fn commit_loaded_url(&mut self, url: &str, title: &str) {
        if let Some(target_idx) = self.pending_history_idx.take() {
            if self
                .history
                .get(target_idx)
                .map(|entry| entry == url)
                .unwrap_or(false)
            {
                self.history_idx = target_idx;
            } else {
                self.push_history(url);
            }
        } else if self.history_idx > 0
            && self
                .history
                .get(self.history_idx - 1)
                .map(|entry| entry == url)
                .unwrap_or(false)
        {
            self.history_idx -= 1;
        } else if self.history_idx + 1 < self.history.len()
            && self.history[self.history_idx + 1] == url
        {
            self.history_idx += 1;
        } else {
            self.push_history(url);
        }

        self.info.loading = false;
        self.info.url = url.to_string();
        self.info.title = title.to_string();
        self.sync_nav_flags();
    }

    pub fn mark_loading(&mut self, url: &str, title: &str) {
        self.info.loading = true;
        self.info.url = url.to_string();
        self.info.title = title.to_string();
    }

    pub fn clear_pending_history(&mut self) {
        self.pending_history_idx = None;
    }

    fn push_history(&mut self, url: &str) {
        if self
            .history
            .last()
            .map(|entry| entry == url)
            .unwrap_or(false)
        {
            self.history_idx = self.history.len().saturating_sub(1);
            return;
        }
        let truncate_to = if self.history.is_empty() {
            0
        } else {
            self.history_idx + 1
        };
        self.history.truncate(truncate_to);
        self.history.push(url.to_string());
        self.history_idx = self.history.len().saturating_sub(1);
    }

    fn sync_nav_flags(&mut self) {
        self.info.can_go_back = self.history_idx > 0;
        self.info.can_go_forward = self.history_idx + 1 < self.history.len();
    }
}

pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return search_url(s);
    }
    if has_http_scheme(s) {
        return s.to_string();
    }
    if has_explicit_scheme_with_slashes(s)
        || (has_explicit_scheme(s) && !looks_like_host_or_local_url(s))
    {
        return search_url(s);
    }
    if looks_like_host_or_local_url(s) {
        format!("https://{s}")
    } else {
        search_url(s)
    }
}

fn search_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "https://duckduckgo.com/?q=".to_string();
    }
    format!("https://duckduckgo.com/?q={}", urlencoding::encode(s))
}

fn has_http_scheme(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn has_explicit_scheme_with_slashes(input: &str) -> bool {
    input
        .split_once("://")
        .map(|(scheme, _)| is_valid_scheme(scheme))
        .unwrap_or(false)
}

fn has_explicit_scheme(input: &str) -> bool {
    input
        .split_once(':')
        .map(|(scheme, _)| is_valid_scheme(scheme))
        .unwrap_or(false)
}

fn is_valid_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

fn looks_like_host_or_local_url(input: &str) -> bool {
    !input.is_empty()
        && !input.contains(' ')
        && (input.contains('.') || input.starts_with("localhost") || input.starts_with("[::1]"))
}

pub fn title_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
        .unwrap_or_else(|| "New Tab".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_full_https_passthrough() {
        assert_eq!(normalize_url("https://github.com"), "https://github.com");
    }

    #[test]
    fn test_normalize_https_passthrough_with_uppercase_scheme() {
        assert_eq!(
            normalize_url("HTTP://localhost:3000"),
            "HTTP://localhost:3000"
        );
    }

    #[test]
    fn test_normalize_full_http_passthrough() {
        assert_eq!(
            normalize_url("http://localhost:3000"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn test_normalize_adds_https_to_domain() {
        assert_eq!(normalize_url("github.com"), "https://github.com");
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        assert_eq!(normalize_url("  github.com  "), "https://github.com");
    }

    #[test]
    fn test_normalize_localhost_adds_https() {
        assert_eq!(
            normalize_url("localhost:5173/test"),
            "https://localhost:5173/test"
        );
    }

    #[test]
    fn test_normalize_search_query_with_spaces() {
        assert_eq!(
            normalize_url("how to center a div"),
            "https://duckduckgo.com/?q=how%20to%20center%20a%20div"
        );
    }

    #[test]
    fn test_normalize_single_word_becomes_search() {
        assert_eq!(normalize_url("rust"), "https://duckduckgo.com/?q=rust");
    }

    #[test]
    fn test_normalize_non_http_scheme_becomes_search() {
        assert_eq!(
            normalize_url("ftp://example.com/file.zip"),
            "https://duckduckgo.com/?q=ftp%3A%2F%2Fexample.com%2Ffile.zip"
        );
    }

    #[test]
    fn test_title_from_url_extracts_host() {
        assert_eq!(
            title_from_url("https://www.github.com/user/repo"),
            "github.com"
        );
    }

    #[test]
    fn test_title_from_url_strips_www() {
        assert_eq!(title_from_url("https://www.reddit.com"), "reddit.com");
    }

    #[test]
    fn test_history_back_detection() {
        let info = TabInfo {
            id: "t1".into(),
            url: "https://b.com".into(),
            title: "b.com".into(),
            loading: false,
            can_go_back: true,
            can_go_forward: false,
        };
        let mut tab = TabData::new(info);
        tab.history = vec!["https://a.com".to_string(), "https://b.com".to_string()];
        tab.history_idx = 1;

        tab.pending_history_idx = Some(0);
        tab.commit_loaded_url("https://a.com", "a.com");

        assert_eq!(tab.history_idx, 0);
        assert_eq!(
            tab.history.len(),
            2,
            "Back navigation should not add new entry"
        );
        assert!(tab.info.can_go_forward);
    }

    #[test]
    fn test_history_forward_detection() {
        let info = TabInfo {
            id: "t1".into(),
            url: "https://a.com".into(),
            title: "a.com".into(),
            loading: false,
            can_go_back: false,
            can_go_forward: true,
        };
        let mut tab = TabData::new(info);
        tab.history = vec!["https://a.com".to_string(), "https://b.com".to_string()];
        tab.history_idx = 0;

        tab.pending_history_idx = Some(1);
        tab.commit_loaded_url("https://b.com", "b.com");

        assert_eq!(tab.history_idx, 1);
        assert_eq!(tab.history.len(), 2);
        assert!(tab.info.can_go_back);
        assert!(!tab.info.can_go_forward);
    }

    #[test]
    fn test_new_navigation_truncates_forward_history() {
        let info = TabInfo {
            id: "t1".into(),
            url: "https://a.com".into(),
            title: "a.com".into(),
            loading: false,
            can_go_back: false,
            can_go_forward: true,
        };
        let mut tab = TabData::new(info);
        tab.history = vec![
            "https://a.com".to_string(),
            "https://b.com".to_string(),
            "https://c.com".to_string(),
        ];
        tab.history_idx = 1;

        tab.commit_loaded_url("https://d.com", "d.com");

        assert_eq!(
            tab.history,
            vec![
                "https://a.com".to_string(),
                "https://b.com".to_string(),
                "https://d.com".to_string(),
            ]
        );
        assert_eq!(tab.history_idx, 2);
        assert!(tab.info.can_go_back);
        assert!(!tab.info.can_go_forward);
    }

    #[test]
    fn test_next_active_after_close_prefers_right_neighbor() {
        let order = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(next_active_after_close(&order, "b"), Some("c".to_string()));
    }

    #[test]
    fn test_next_active_after_close_falls_back_left_at_end() {
        let order = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(next_active_after_close(&order, "c"), Some("b".to_string()));
    }

    #[test]
    fn test_next_active_after_close_returns_none_for_last_tab() {
        let order = vec!["a".to_string()];
        assert_eq!(next_active_after_close(&order, "a"), None);
    }
}
