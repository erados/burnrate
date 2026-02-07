use crate::UsageData;
use chrono::{Datelike, Local, Timelike};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AnthropicUsageResponse {
    #[serde(default)]
    data: Vec<UsageBucket>,
}

#[derive(Debug, Deserialize)]
struct UsageBucket {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cost_usd: f64,
    #[serde(default)]
    model: String,
}

/// Fetch usage from the Anthropic Admin API
pub async fn fetch_usage(api_key: &str, org_id: &str) -> Result<UsageData, String> {
    let client = reqwest::Client::new();
    let now = Local::now();

    // Monthly usage - from start of month
    let month_start = now
        .with_day(1)
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let today = now.format("%Y-%m-%d").to_string();

    let url = format!(
        "https://api.anthropic.com/v1/organizations/{}/usage?start_date={}&end_date={}&granularity=day",
        org_id, month_start, today
    );

    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("API error: {}", resp.status()));
    }

    let data: AnthropicUsageResponse = resp.json().await.map_err(|e| e.to_string())?;

    let monthly_cost: f64 = data.data.iter().map(|b| b.cost_usd).sum();

    // Calculate weekly usage (since last Saturday)
    let weekday = now.weekday().num_days_from_sunday();
    let days_since_saturday = if weekday >= 6 { 0 } else { weekday + 1 };
    let weekly_start = now - chrono::Duration::days(days_since_saturday as i64);
    let weekly_start_str = weekly_start.format("%Y-%m-%d").to_string();

    // Weekly query
    let weekly_url = format!(
        "https://api.anthropic.com/v1/organizations/{}/usage?start_date={}&end_date={}&granularity=day",
        org_id, weekly_start_str, today
    );

    let weekly_resp = client
        .get(&weekly_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let weekly_data: AnthropicUsageResponse = if weekly_resp.status().is_success() {
        weekly_resp.json().await.unwrap_or(AnthropicUsageResponse { data: vec![] })
    } else {
        AnthropicUsageResponse { data: vec![] }
    };

    let weekly_total_tokens: u64 = weekly_data
        .data
        .iter()
        .map(|b| b.input_tokens + b.output_tokens)
        .sum();
    let weekly_sonnet_tokens: u64 = weekly_data
        .data
        .iter()
        .filter(|b| b.model.contains("sonnet"))
        .map(|b| b.input_tokens + b.output_tokens)
        .sum();

    // Estimate percentages (these are rough - actual limits vary)
    // Weekly limit ~5M tokens for all models
    let weekly_limit = 5_000_000u64;
    let weekly_all_percent = (weekly_total_tokens as f64 / weekly_limit as f64) * 100.0;
    let weekly_sonnet_percent = (weekly_sonnet_tokens as f64 / weekly_limit as f64) * 100.0;

    // Session is harder to track via API - estimate from recent activity
    let session_percent = estimate_session_usage();

    // Hours until Saturday
    let days_until_sat = (6 - weekday) % 7;
    let weekly_reset_hours = (days_until_sat as i64) * 24 + (24 - now.hour() as i64);

    Ok(UsageData {
        session_percent,
        session_reset_minutes: 277, // ~4.5h default, hard to determine exactly
        weekly_all_percent: weekly_all_percent.min(100.0),
        weekly_sonnet_percent: weekly_sonnet_percent.min(100.0),
        weekly_reset_hours,
        monthly_cost,
        monthly_limit: 50.0,
    })
}

/// Estimate session usage by parsing Claude Code logs
fn estimate_session_usage() -> f64 {
    // Try to read ~/.claude/ logs for session data
    let home = std::env::var("HOME").unwrap_or_default();
    let claude_dir = std::path::Path::new(&home).join(".claude");

    if claude_dir.exists() {
        // Look for recent session files
        if let Ok(entries) = std::fs::read_dir(&claude_dir) {
            let mut recent_tokens = 0u64;
            let now = std::time::SystemTime::now();

            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            // Files modified in last 5 hours
                            if age.as_secs() < 18000 {
                                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                    // Simple heuristic: count approximate tokens
                                    recent_tokens += (content.len() / 4) as u64;
                                }
                            }
                        }
                    }
                }
            }

            // Session limit is roughly 500K tokens
            let session_limit = 500_000u64;
            return ((recent_tokens as f64 / session_limit as f64) * 100.0).min(100.0);
        }
    }

    0.0
}

/// Generate mock usage data for testing
pub fn mock_usage() -> UsageData {
    let now = Local::now();
    let hour = now.hour() as f64;

    // Simulate varying usage throughout the day
    UsageData {
        session_percent: (hour * 2.5) % 30.0 + 5.0,
        session_reset_minutes: (277.0 - hour * 10.0).max(30.0) as i64,
        weekly_all_percent: 1.0 + (hour / 24.0) * 5.0,
        weekly_sonnet_percent: 0.5 + (hour / 24.0) * 2.0,
        weekly_reset_hours: {
            let weekday = now.weekday().num_days_from_sunday();
            let days_until_sat = (6 - weekday) % 7;
            (days_until_sat as i64) * 24 + (24 - now.hour() as i64)
        },
        monthly_cost: 39.37,
        monthly_limit: 50.0,
    }
}
