use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// Extensions that should be downloaded instead of navigated to.
const DOWNLOAD_EXTENSIONS: &[&str] = &[
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "dmg", "pkg", "app", "exe", "msi", "deb", "rpm", "torrent", "iso", "img",
];

static DOWNLOAD_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn download_client() -> &'static reqwest::Client {
    DOWNLOAD_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Orbit download HTTP client should initialize")
    })
}

/// Check if a URL likely points to a downloadable file.
pub fn is_download_url(url: &str) -> bool {
    let lower = url.to_lowercase();

    // Skip URLs with query params or fragments for extension matching
    let path_only = lower
        .split('?')
        .next()
        .unwrap_or(&lower)
        .split('#')
        .next()
        .unwrap_or(&lower);

    // Check Content-Disposition header patterns in URL (rare but possible)
    if lower.contains("download") || lower.contains("attachment") {
        // Only flag if there's also a file extension
        if let Some(ext) = extract_extension(path_only) {
            return DOWNLOAD_EXTENSIONS.contains(&ext.as_str());
        }
    }

    // Check by extension
    if let Some(ext) = extract_extension(path_only) {
        return DOWNLOAD_EXTENSIONS.contains(&ext.as_str());
    }

    false
}

fn extract_extension(path: &str) -> Option<String> {
    // Get the last path segment
    let filename = path.rsplit('/').next().unwrap_or(path);
    if filename.is_empty() {
        return None;
    }
    let (_, ext) = filename.rsplit_once('.')?;
    if ext.is_empty() {
        return None;
    }
    Some(ext.to_lowercase())
}

fn ensure_download_url_allowed(url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid download URL: {e}"))?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!("Unsupported download URL scheme: {scheme}"));
    }
    Ok(parsed)
}

/// Derive a filename from a URL.
pub fn filename_from_url(url: &str) -> String {
    let path_only = url.split('?').next().unwrap_or(url);

    if let Some(filename) = path_only.rsplit('/').next() {
        if !filename.is_empty() && filename.contains('.') {
            let decoded = urlencoding::decode(filename)
                .unwrap_or_else(|_| filename.into())
                .into_owned();
            let sanitized = sanitize_filename(&decoded);
            return sanitized;
        }
    }

    // Fallback: timestamp-based filename
    let fallback = format!(
        "download_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return format!("{fallback}_{host}");
        }
    }
    fallback
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            ch if ch.is_ascii_control() => '_',
            ch => ch,
        })
        .collect::<String>();

    let trimmed = sanitized.trim().trim_matches('.');
    if trimmed.is_empty() {
        "download".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_download_path(downloads_dir: &Path, filename: &str) -> PathBuf {
    let safe_filename = sanitize_filename(filename);
    let mut path = downloads_dir.join(&safe_filename);
    if !path.exists() {
        return path;
    }

    let stem = Path::new(&safe_filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let ext = Path::new(&safe_filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    let mut index: u32 = 1;
    loop {
        let candidate = if ext.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{ext}")
        };
        path = downloads_dir.join(&candidate);
        if !path.exists() {
            return path;
        }

        index = index.saturating_add(1);
        if index > 1_000 {
            return path.with_file_name(format!(
                "{stem}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ));
        }
    }
}

/// Download a file from a URL and save to ~/Downloads.
/// Emits `download-started` and `download-complete` events.
#[tauri::command]
pub async fn download_file(app: AppHandle, url: String) -> Result<String, String> {
    let parsed_url = ensure_download_url_allowed(&url)?;
    let safe_url = parsed_url.to_string();
    let filename = filename_from_url(&safe_url);
    let downloads_dir =
        dirs_downloads().ok_or_else(|| "Could not find Downloads directory".to_string())?;
    let filepath = unique_download_path(&downloads_dir, &filename);
    let filename = filepath
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&filename)
        .to_string();

    // Emit started event
    let _ = app.emit(
        "download-started",
        serde_json::json!({
            "url": safe_url,
            "filename": filename,
        }),
    );

    // Download
    let response = download_client()
        .get(parsed_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }
    let mut file = tokio::fs::File::create(&filepath)
        .await
        .map_err(|e| format!("Failed to create file: {e}"))?;
    let mut response = response;
    let mut downloaded: u64 = 0;
    const MAX_DOWNLOAD_BYTES: u64 = 250 * 1024 * 1024;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download failed: {e}"))?
    {
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > MAX_DOWNLOAD_BYTES {
            let _ = tokio::fs::remove_file(&filepath).await;
            return Err(format!(
                "Download too large (>{}MB)",
                MAX_DOWNLOAD_BYTES / (1024 * 1024)
            ));
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to save file: {e}"))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("Failed to finalize file: {e}"))?;

    let path_str = filepath.to_string_lossy().to_string();

    // Emit complete event
    let _ = app.emit(
        "download-complete",
        serde_json::json!({
            "url": safe_url,
            "filename": filename,
            "path": path_str,
            "size": downloaded,
        }),
    );

    Ok(path_str)
}

fn dirs_downloads() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let dl = PathBuf::from(home).join("Downloads");
        std::fs::create_dir_all(&dl).ok()?;
        return Some(dl);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_download_url_zip() {
        assert!(is_download_url("https://example.com/file.zip"));
    }

    #[test]
    fn test_is_download_url_pdf() {
        assert!(!is_download_url("https://example.com/doc.pdf"));
    }

    #[test]
    fn test_is_download_url_dmg() {
        assert!(is_download_url("https://example.com/app.dmg"));
    }

    #[test]
    fn test_is_download_url_mp4() {
        assert!(!is_download_url("https://example.com/video.mp4"));
    }

    #[test]
    fn test_is_not_download_url_html() {
        assert!(!is_download_url("https://example.com/page.html"));
    }

    #[test]
    fn test_is_not_download_url_no_ext() {
        assert!(!is_download_url("https://example.com/about"));
    }

    #[test]
    fn test_is_download_url_with_query() {
        assert!(is_download_url("https://example.com/file.zip?token=abc"));
    }

    #[test]
    fn test_is_not_download_url_browser_viewable_assets() {
        assert!(!is_download_url("https://example.com/image.png"));
        assert!(!is_download_url("https://example.com/data.json"));
        assert!(!is_download_url("https://example.com/feed.xml"));
    }

    #[test]
    fn test_filename_from_url_simple() {
        assert_eq!(
            filename_from_url("https://example.com/file.zip"),
            "file.zip"
        );
    }

    #[test]
    fn test_filename_from_url_with_query() {
        assert_eq!(
            filename_from_url("https://example.com/file.zip?token=abc"),
            "file.zip"
        );
    }

    #[test]
    fn test_filename_from_url_encoded() {
        assert_eq!(
            filename_from_url("https://example.com/my%20file.pdf"),
            "my file.pdf"
        );
    }

    #[test]
    fn test_rejects_non_http_download_scheme() {
        assert!(ensure_download_url_allowed("file:///tmp/test.zip").is_err());
        assert!(ensure_download_url_allowed("ftp://example.com/test.zip").is_err());
    }
}
