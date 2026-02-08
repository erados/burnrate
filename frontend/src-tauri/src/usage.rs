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

/// Scan all sessions-index.json files for sessions on a given date
fn scan_sessions_for_date(target_date: &str) -> (u64, Vec<String>) {
    let claude = match claude_dir() {
        Some(d) => d,
        None => return (0, vec![]),
    };
    let projects = claude.join("projects");
    let mut session_count = 0u64;
    let mut jsonl_paths = vec![];

    if let Ok(dirs) = std::fs::read_dir(&projects) {
        for dir in dirs.flatten() {
            let idx = dir.path().join("sessions-index.json");
            if let Ok(content) = std::fs::read_to_string(&idx) {
                if let Ok(index) = serde_json::from_str::<SessionsIndex>(&content) {
                    for entry in &index.entries {
                        let created_date = entry.created.get(..10).unwrap_or("");
                        let modified_date = entry.modified.get(..10).unwrap_or("");
                        if created_date == target_date || modified_date == target_date {
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

    let mut target_messages = 0u64;
    let mut target_tool_calls = 0u64;
    let mut target_tokens = 0u64;
    let mut target_model_tokens: HashMap<String, u64> = HashMap::new();
    let mut last_active_date = today.clone();

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

        // Try today first, then find most recent day
        let target_date = if activities.iter().any(|a| a.date == today && a.message_count > 0)
            || model_tokens_list.iter().any(|t| t.date == today && t.tokens_by_model.values().sum::<u64>() > 0)
        {
            today.clone()
        } else {
            // Find most recent date with data
            let mut dates: Vec<&str> = activities.iter().filter(|a| a.message_count > 0).map(|a| a.date.as_str()).collect();
            dates.extend(model_tokens_list.iter().filter(|t| t.tokens_by_model.values().sum::<u64>() > 0).map(|t| t.date.as_str()));
            dates.sort();
            dates.last().unwrap_or(&today.as_str()).to_string()
        };
        last_active_date = target_date.clone();

        if let Some(act) = activities.iter().find(|a| a.date == target_date) {
            target_messages = act.message_count;
            target_tool_calls = act.tool_call_count;
        }
        if let Some(tok) = model_tokens_list.iter().find(|t| t.date == target_date) {
            for (model, count) in &tok.tokens_by_model {
                let key = simplify_model(model);
                *target_model_tokens.entry(key).or_default() += count;
                target_tokens += count;
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

    // Scan JSONL for target date's sessions
    let (target_sessions, target_jsonl_paths) = scan_sessions_for_date(&last_active_date);

    // If stats-cache doesn't have data, use JSONL data
    if target_messages == 0 && !target_jsonl_paths.is_empty() {
        let (msgs, tools, tokens, model_toks) = parse_jsonl_files(&target_jsonl_paths, None);
        target_messages = msgs;
        target_tool_calls = tools;
        target_tokens = tokens;
        target_model_tokens = model_toks;
    }

    // Last 5 hours usage for rate estimate (only relevant for today)
    let last5h_tokens = if last_active_date == today && !target_jsonl_paths.is_empty() {
        let (_, _, tokens, _) = parse_jsonl_files(&target_jsonl_paths, Some(5.0));
        tokens
    } else {
        0
    };

    // Estimated usage percent
    let estimated_cap = 1_000_000u64;
    let usage_percent = ((last5h_tokens as f64 / estimated_cap as f64) * 100.0).min(100.0);

    // Sort weekly tokens by date
    weekly_tokens.sort_by(|a, b| a.0.cmp(&b.0));
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

    let opus_tokens = *target_model_tokens.get("Opus").unwrap_or(&0);
    let sonnet_tokens = *target_model_tokens.get("Sonnet").unwrap_or(&0);

    UsageData {
        today_messages: target_messages,
        today_tool_calls: target_tool_calls,
        today_sessions: target_sessions,
        today_tokens: target_tokens,
        opus_tokens,
        sonnet_tokens,
        weekly_daily,
        weekly_messages,
        usage_percent,
        last5h_tokens,
        last_active_date,
    }
}
