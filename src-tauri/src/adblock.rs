use crate::browser::report_error;

use std::collections::HashSet;
use std::sync::Arc;

/// Ad blocker powered by a domain blocklist.
///
/// Exact match is O(1) via HashSet.  Subdomain matching iterates through
/// pre-formatted `.domain` strings and calls `ends_with` — no per-check
/// allocation.  For 30–500 domain blocklists this is fast enough;
/// larger lists should use a suffix trie.
#[derive(Clone)]
pub struct AdBlocker {
    /// Exact-match set (e.g. "doubleclick.net").
    pub blocked_domains: Arc<HashSet<String>>,
    /// Pre-formatted dotted suffixes (e.g. ".doubleclick.net") —
    /// checked against `host.ends_with()` to avoid allocation.
    dotted_domains: Arc<Vec<String>>,
    url_patterns: Arc<Vec<String>>,
}

impl AdBlocker {
    pub fn new(domains: Vec<String>) -> Self {
        Self::with_patterns(domains, vec![])
    }

    pub fn with_patterns(domains: Vec<String>, patterns: Vec<String>) -> Self {
        let domains: Vec<String> = domains
            .into_iter()
            .map(|domain| domain.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect();
        let url_patterns = patterns
            .into_iter()
            .map(|pattern| pattern.trim().to_ascii_lowercase())
            .filter(|pattern| !pattern.is_empty())
            .collect();
        let blocked_domains = Arc::new(domains.iter().cloned().collect());
        let dotted_domains = Arc::new(domains.iter().map(|d| format!(".{d}")).collect());
        Self {
            blocked_domains,
            dotted_domains,
            url_patterns: Arc::new(url_patterns),
        }
    }

    /// Returns `true` if the URL's host matches a blocked domain.
    pub fn is_blocked(&self, url: &url::Url) -> bool {
        let host = match url.host_str().map(|host| host.to_ascii_lowercase()) {
            Some(host) => host,
            None => return false,
        };
        if self.blocked_domains.contains(host.as_str()) {
            return true;
        }
        for dot in self.dotted_domains.iter() {
            if host.ends_with(dot.as_str()) {
                return true;
            }
        }
        let full_url = url.as_str().to_ascii_lowercase();
        for pattern in self.url_patterns.iter() {
            if full_url.contains(pattern.as_str()) {
                return true;
            }
        }
        false
    }

    pub fn load_from_json(json_path: &std::path::Path) -> Self {
        let content = match std::fs::read_to_string(json_path) {
            Ok(content) => content,
            Err(err) => {
                report_error(format_args!(
                    "failed to read adblock list {}: {err}",
                    json_path.display()
                ));
                return Self::new(vec![]);
            }
        };

        let parsed: serde_json::Value = match serde_json::from_str(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                report_error(format_args!(
                    "failed to parse adblock list {}: {err}",
                    json_path.display()
                ));
                return Self::new(vec![]);
            }
        };

        let domains = read_string_array(&parsed, "domains");
        let mut patterns = Vec::new();
        for key in [
            "urlPatterns",
            "youtubePatterns",
            "facebookPatterns",
            "pathPatterns",
        ] {
            patterns.extend(read_string_array(&parsed, key));
        }
        if parsed.get("regexPattern").is_some() {
            report_error("regexPattern is ignored; use literal urlPatterns instead");
        }
        Self::with_patterns(domains, patterns)
    }
}

fn read_string_array(parsed: &serde_json::Value, key: &str) -> Vec<String> {
    parsed[key]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocker() -> AdBlocker {
        AdBlocker::new(vec![
            "doubleclick.net".into(),
            "googleadservices.com".into(),
        ])
    }

    #[test]
    fn test_blocks_exact_domain() {
        let b = blocker();
        let url = url::Url::parse("https://doubleclick.net/ad").unwrap();
        assert!(b.is_blocked(&url));
    }

    #[test]
    fn test_blocks_subdomain() {
        let b = blocker();
        let url = url::Url::parse("https://cdn.doubleclick.net/pixel.gif").unwrap();
        assert!(b.is_blocked(&url));
    }

    #[test]
    fn test_allows_clean_domain() {
        let b = blocker();
        let url = url::Url::parse("https://github.com").unwrap();
        assert!(!b.is_blocked(&url));
    }

    #[test]
    fn test_no_false_positive_on_substring_match() {
        // "notdoubleclick.net" should NOT be blocked just because it contains "doubleclick"
        let b = blocker();
        let url = url::Url::parse("https://notdoubleclick.net").unwrap();
        assert!(!b.is_blocked(&url));
    }

    #[test]
    fn test_load_from_json_loads_domains() {
        // Write a temp JSON file and test loading
        let dir = std::env::temp_dir();
        let path = dir.join("test_adblock.json");
        std::fs::write(&path, r#"{"domains":["ads.test.com","tracker.test.com"]}"#).unwrap();
        let b = AdBlocker::load_from_json(&path);
        let url = url::Url::parse("https://ads.test.com/pixel").unwrap();
        assert!(b.is_blocked(&url));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_blocks_literal_url_patterns() {
        let b = AdBlocker::with_patterns(vec![], vec!["/pagead/".into()]);
        let url = url::Url::parse("https://example.com/pagead/banner.js").unwrap();
        assert!(b.is_blocked(&url));
    }

    #[test]
    fn test_load_from_json_loads_url_patterns() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_adblock_url_patterns.json");
        std::fs::write(&path, r#"{"urlPatterns":["/pagead/"]}"#).unwrap();
        let b = AdBlocker::load_from_json(&path);
        let url = url::Url::parse("https://example.com/pagead/banner.js").unwrap();
        assert!(b.is_blocked(&url));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_json_loads_youtube_patterns() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_adblock_youtube_patterns.json");
        std::fs::write(&path, r#"{"youtubePatterns":["/api/stats/ads"]}"#).unwrap();
        let b = AdBlocker::load_from_json(&path);
        let url = url::Url::parse("https://youtube.com/api/stats/ads?x=1").unwrap();
        assert!(b.is_blocked(&url));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_json_loads_facebook_patterns() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_adblock_facebook_patterns.json");
        std::fs::write(&path, r#"{"facebookPatterns":["facebook.com/tr"]}"#).unwrap();
        let b = AdBlocker::load_from_json(&path);
        let url = url::Url::parse("https://facebook.com/tr?id=123").unwrap();
        assert!(b.is_blocked(&url));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_json_loads_path_patterns() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_adblock_path_patterns.json");
        std::fs::write(&path, r#"{"pathPatterns":["/promo/"]}"#).unwrap();
        let b = AdBlocker::load_from_json(&path);
        let url = url::Url::parse("https://example.com/promo/deal").unwrap();
        assert!(b.is_blocked(&url));
        std::fs::remove_file(&path).ok();
    }
}
