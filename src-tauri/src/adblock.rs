use std::collections::HashSet;
use std::sync::Arc;

pub struct AdBlocker {
    pub blocked_domains: Arc<HashSet<String>>,
}

impl AdBlocker {
    pub fn new(domains: Vec<String>) -> Self {
        Self {
            blocked_domains: Arc::new(domains.into_iter().collect()),
        }
    }

    pub fn is_blocked(&self, url: &url::Url) -> bool {
        let host = match url.host_str() {
            Some(h) => h,
            None => return false,
        };
        // Exact match
        if self.blocked_domains.contains(host) {
            return true;
        }
        // Subdomain match: host ends with ".blocked_domain"
        for domain in self.blocked_domains.iter() {
            if host.ends_with(&format!(".{domain}")) {
                return true;
            }
        }
        false
    }

    pub fn arc_domains(&self) -> Arc<HashSet<String>> {
        self.blocked_domains.clone()
    }

    pub fn load_from_json(json_path: &std::path::Path) -> Self {
        let content = std::fs::read_to_string(json_path).unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        let domains = parsed["domains"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Self::new(domains)
    }
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
}
