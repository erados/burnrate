use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub session_percent: f64,
    pub weekly_all_percent: f64,
    pub weekly_sonnet_percent: f64,
}

fn history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".burnrate").join("history.json"))
}

pub fn load_history() -> Vec<HistoryEntry> {
    let path = match history_path() {
        Some(p) => p,
        None => return vec![],
    };
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => vec![],
    }
}

pub fn append_entry(session_percent: f64, weekly_all_percent: f64, weekly_sonnet_percent: f64) {
    let path = match history_path() {
        Some(p) => p,
        None => return,
    };

    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    let mut entries = load_history();
    let now = chrono::Utc::now();

    entries.push(HistoryEntry {
        timestamp: now.to_rfc3339(),
        session_percent,
        weekly_all_percent,
        weekly_sonnet_percent,
    });

    // Prune entries older than 7 days
    let cutoff = now - chrono::Duration::days(7);
    entries.retain(|e| {
        chrono::DateTime::parse_from_rfc3339(&e.timestamp)
            .map(|t| t >= cutoff)
            .unwrap_or(false)
    });

    if let Ok(json) = serde_json::to_string(&entries) {
        let _ = fs::write(&path, json);
    }
}
