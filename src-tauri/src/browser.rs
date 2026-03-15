use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

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
}

#[derive(Default)]
pub struct BrowserState {
    pub tabs: Mutex<HashMap<String, TabData>>,
    pub active_tab: Mutex<Option<String>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else if s.contains('.') && !s.contains(' ') && !s.is_empty() {
        format!("https://{s}")
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(s))
    }
}

pub fn title_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
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
    fn test_normalize_full_http_passthrough() {
        assert_eq!(normalize_url("http://localhost:3000"), "http://localhost:3000");
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
    fn test_normalize_search_query_with_spaces() {
        assert_eq!(
            normalize_url("how to center a div"),
            "https://duckduckgo.com/?q=how%20to%20center%20a%20div"
        );
    }

    #[test]
    fn test_normalize_single_word_becomes_search() {
        assert_eq!(
            normalize_url("rust"),
            "https://duckduckgo.com/?q=rust"
        );
    }

    #[test]
    fn test_title_from_url_extracts_host() {
        assert_eq!(title_from_url("https://www.github.com/user/repo"), "github.com");
    }

    #[test]
    fn test_title_from_url_strips_www() {
        assert_eq!(title_from_url("https://www.reddit.com"), "reddit.com");
    }

    #[test]
    fn test_history_back_detection() {
        // Simulate having visited a.com then b.com (idx=1)
        let mut history = vec!["https://a.com".to_string(), "https://b.com".to_string()];
        let mut idx: usize = 1;

        // User navigates "back" — page loads a.com again
        let new_url = "https://a.com";
        if idx > 0 && history[idx - 1] == new_url {
            idx -= 1;  // Detected as back navigation
        } else {
            history.truncate(idx + 1);
            history.push(new_url.to_string());
            idx = history.len() - 1;
        }

        assert_eq!(idx, 0);
        assert_eq!(history.len(), 2, "Back navigation should not add new entry");
    }
}
