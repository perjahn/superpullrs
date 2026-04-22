use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Debug utility for saving API responses to JSON files
/// Enabled via SUPERPULL_DEBUG_API environment variable
/// Saves to SUPERPULL_DEBUG_DIR (default: /tmp/superpull-debug/)
static RESPONSE_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn save_api_response(service: &str, endpoint: &str, json_str: &str) -> anyhow::Result<()> {
    if env::var("SUPERPULL_DEBUG_API").is_err() {
        return Ok(());
    }

    let debug_dir =
        env::var("SUPERPULL_DEBUG_DIR").unwrap_or_else(|_| "/tmp/superpull-debug/".to_string());

    fs::create_dir_all(&debug_dir)?;

    let debug_path = PathBuf::from(&debug_dir);
    let counter = RESPONSE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let filename = format!(
        "{:04}-{}-{}.json",
        counter,
        service,
        endpoint.replace(['/', '?', '&', '='], "_").replace(":", "")
    );

    let file_path = debug_path.join(filename);

    // Pretty-print JSON if possible, otherwise save as-is
    let formatted_json = match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(parsed) => {
            serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| json_str.to_string())
        }
        Err(_) => json_str.to_string(),
    };

    fs::write(file_path, formatted_json)?;

    Ok(())
}
