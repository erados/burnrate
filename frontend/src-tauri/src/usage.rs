use crate::UsageData;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct StatsCache {
    #[serde(default)]
    daily_activity: Vec<DailyActivity>,
    #[serde(default, rename = "dailyActivity")]
    daily_activity_alt: Vec<DailyActivity>,
    #[serde(default)]
    daily_model_tokens: Vec<DailyModelTokens>,
    #[serde(default, rename = "dailyModelTokens")]
    daily_model_tokens_alt: Vec<DailyModelTokens>,
}

#[derive(Debug, Deserialize, Clone)]
struct DailyActivity {
    date: String,
    #[serde(default, rename = "messageCount")]
    message_count: u64,
    #[serde(default, rename = "sessionCount")]
    session_count: u64,
    #[serde(default, rename = "toolCallCount")]
    tool_call_count: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct DailyModelTokens {
    date: String,
    #[serde(default, rename = "tokensByModel")]
    tokens_by_model: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
struct SessionsIndex {
    #[serde(default)]
    entries: Vec<SessionEntry>,
}

#[derive(Debug, Deserialize)]
struct SessionEntry {
    #[serde(default, rename = "fullPath")]
    full_path: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    modified: String,
    #[serde(default, rename = "messageCount")]
    message_count: u64,
}

#[derive(Debug, Deserialize)]
struct JournalLine {
    #[serde(default, rename = "type")]
    line_type: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    message: Option<AssistantMessage>,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    #[serde(default)]
    model: String,
    #[serde(default)]
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct TokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

fn claude_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let p = PathBuf::from(home).join(".claude");
    if p.exists() { Some(p) } else { None }
}

fn read_stats_cache() -> Option<StatsCache> {
    let path = claude_dir()?.join("stats-cache.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn today_str() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Scan all sessions-index.json files for today's sessions
fn scan_today_sessions() -> (u64, Vec<String>) {
    let claude = match claude_dir() {
        Some(d) => d,
        None => return (0, vec![]),
    };
    let projects = claude.join("projects");
    let today = today_str();
    let mut session_count = 0u64;
    let mut jsonl_paths = vec![];

    if let Ok(dirs) = std::fs::read_dir(&projects) {
        for dir in dirs.flatten() {
            let idx = dir.path().join("sessions-index.json");
            if let Ok(content) = std::fs::read_to_string(&idx) {
                if let Ok(index) = serde_json::from_str::<SessionsIndex>(&content) {
                    for entry in &index.entries {
                        // Check if session was active today
                        let created_date = entry.created.get(..10).unwrap_or("");
                        let modified_date = entry.modified.get(..10).unwrap_or("");
                        if created_date == today || modified_date == today {
                            session_count += 1;
                            if !entry.full_path.is_empty() {
                                jsonl_paths.push(entry.full_path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    (session_count, jsonl_paths)
}

/// Parse JSONL files to get token usage and message/tool counts
fn parse_jsonl_files(paths: &[String], since_hours: Option<f64>) -> (u64, u64, u64, HashMap<String, u64>) {
    let mut messages = 0u64;
    let mut tool_calls = 0u64;
    let mut total_tokens = 0u64;
    let mut model_tokens: HashMap<String, u64> = HashMap::new();

    let cutoff = since_hours.map(|h| {
        Utc::now() - chrono::Duration::seconds((h * 3600.0) as i64)
    });

    for path in paths {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let parsed: JournalLine = match serde_json::from_str(line) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Filter by time if cutoff specified
            if let Some(cutoff_time) = &cutoff {
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&parsed.timestamp) {
                    if ts < *cutoff_time {
                        continue;
                    }
                }
            }

            match parsed.line_type.as_str() {
                "user" => messages += 1,
                "assistant" => {
                    messages += 1;
                    if let Some(msg) = &parsed.message {
                        if let Some(usage) = &msg.usage {
                            let tokens = usage.output_tokens + usage.input_tokens;
                            total_tokens += tokens;
                            *model_tokens.entry(simplify_model(&msg.model)).or_default() += tokens;
                        }
                        // Count tool use in content (heuristic: check if content has tool_use type)
                    }
                }
                _ => {}
            }
        }

        // Also count tool calls by looking for tool_use in assistant content
        for line in content.lines() {
            if line.contains("\"type\":\"tool_use\"") || line.contains("\"type\": \"tool_use\"") {
                tool_calls += 1;
            }
        }
    }

    (messages, tool_calls, total_tokens, model_tokens)
}

fn simplify_model(model: &str) -> String {
    if model.contains("opus") {
        "Opus".to_string()
    } else if model.contains("sonnet") {
        "Sonnet".to_string()
    } else if model.contains("haiku") {
        "Haiku".to_string()
    } else if model.is_empty() {
        "Unknown".to_string()
    } else {
        model.to_string()
    }
}

/// Main function to gather all usage data from local Claude files
pub fn read_local_usage() -> UsageData {
    let today = today_str();
    let stats = read_stats_cache();

    // Get today's activity from stats-cache (may be stale)
    let mut today_messages = 0u64;
    let mut today_tool_calls = 0u64;
    let mut today_tokens = 0u64;
    let mut today_model_tokens: HashMap<String, u64> = HashMap::new();

    // Weekly data from stats-cache
    let mut weekly_tokens: Vec<(String, u64)> = vec![];
    let mut weekly_messages = 0u64;

    if let Some(ref stats) = stats {
        let activities = if !stats.daily_activity_alt.is_empty() {
            &stats.daily_activity_alt
        } else {
            &stats.daily_activity
        };
        let model_tokens_list = if !stats.daily_model_tokens_alt.is_empty() {
            &stats.daily_model_tokens_alt
        } else {
            &stats.daily_model_tokens
        };

        // Today from stats cache
        if let Some(act) = activities.iter().find(|a| a.date == today) {
            today_messages = act.message_count;
            today_tool_calls = act.tool_call_count;
        }
        if let Some(tok) = model_tokens_list.iter().find(|t| t.date == today) {
            for (model, count) in &tok.tokens_by_model {
                let key = simplify_model(model);
                *today_model_tokens.entry(key).or_default() += count;
                today_tokens += count;
            }
        }

        // Last 7 days
        let seven_days_ago = (Utc::now() - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
        for tok in model_tokens_list {
            if tok.date >= seven_days_ago {
                let total: u64 = tok.tokens_by_model.values().sum();
                weekly_tokens.push((tok.date.clone(), total));
            }
        }
        for act in activities {
            if act.date >= seven_days_ago {
                weekly_messages += act.message_count;
            }
        }
    }

    // Scan JSONL for today's live data (supplements stale stats-cache)
    let (today_sessions, today_jsonl_paths) = scan_today_sessions();

    // If stats-cache doesn't have today, use JSONL data
    if today_messages == 0 && !today_jsonl_paths.is_empty() {
        let (msgs, tools, tokens, model_toks) = parse_jsonl_files(&today_jsonl_paths, None);
        today_messages = msgs;
        today_tool_calls = tools;
        today_tokens = tokens;
        today_model_tokens = model_toks;
    }

    // Last 5 hours usage for rate estimate
    let (_, _, last5h_tokens, _) = if !today_jsonl_paths.is_empty() {
        parse_jsonl_files(&today_jsonl_paths, Some(5.0))
    } else {
        (0, 0, 0, HashMap::new())
    };

    // Estimated usage percent (rough: assume ~1M tokens/5h session cap)
    let estimated_cap = 1_000_000u64;
    let usage_percent = ((last5h_tokens as f64 / estimated_cap as f64) * 100.0).min(100.0);

    // Sort weekly tokens by date
    weekly_tokens.sort_by(|a, b| a.0.cmp(&b.0));
    // Pad to 7 days
    let mut weekly_daily: Vec<u64> = vec![0; 7];
    let now = Utc::now().date_naive();
    for (date_str, tokens) in &weekly_tokens {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let days_ago = (now - date).num_days();
            if days_ago >= 0 && days_ago < 7 {
                weekly_daily[6 - days_ago as usize] = *tokens;
            }
        }
    }

    // Model breakdown for today
    let opus_tokens = *today_model_tokens.get("Opus").unwrap_or(&0);
    let sonnet_tokens = *today_model_tokens.get("Sonnet").unwrap_or(&0);

    UsageData {
        today_messages,
        today_tool_calls,
        today_sessions,
        today_tokens,
        opus_tokens,
        sonnet_tokens,
        weekly_daily,
        weekly_messages,
        usage_percent,
        last5h_tokens,
    }
}
