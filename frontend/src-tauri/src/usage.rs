use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;

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
}

fn claude_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let p = PathBuf::from(home).join(".claude");
    if p.exists() { Some(p) } else { None }
}

fn today_str() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn scan_sessions_for_date(target_date: &str) -> Vec<String> {
    let claude = match claude_dir() {
        Some(d) => d,
        None => return vec![],
    };
    let projects = claude.join("projects");
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
                            if !entry.full_path.is_empty() {
                                jsonl_paths.push(entry.full_path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    jsonl_paths
}

fn parse_jsonl_files(paths: &[String]) -> (u64, u64, u64, u64) {
    let mut messages = 0u64;
    let mut total_tokens = 0u64;
    let mut opus_tokens = 0u64;
    let mut sonnet_tokens = 0u64;

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

            match parsed.line_type.as_str() {
                "user" => messages += 1,
                "assistant" => {
                    if let Some(msg) = &parsed.message {
                        if let Some(usage) = &msg.usage {
                            let tokens = usage.output_tokens + usage.input_tokens;
                            total_tokens += tokens;
                            if msg.model.contains("opus") {
                                opus_tokens += tokens;
                            } else if msg.model.contains("sonnet") {
                                sonnet_tokens += tokens;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    (messages, total_tokens, opus_tokens, sonnet_tokens)
}

/// Read local Claude Code log data (supplementary to web scraping)
pub fn read_local_usage() -> (u64, u64, u64, u64) {
    let today = today_str();
    let paths = scan_sessions_for_date(&today);
    if paths.is_empty() {
        return (0, 0, 0, 0);
    }
    parse_jsonl_files(&paths)
}

/// The JS to inject into claude.ai/settings/usage to scrape data.
/// Returns a JSON string with usage info.
pub fn scraping_js() -> &'static str {
    r#"
    (function() {
        try {
            const result = {
                session_percent: 0,
                session_reset_minutes: 0,
                weekly_all_percent: 0,
                weekly_sonnet_percent: 0,
                monthly_cost: 0,
                monthly_limit: 0,
                raw_texts: []
            };

            const t = document.body ? document.body.innerText : '';
            // Save a snippet for debugging
            result.raw_texts = [t.substring(0, 500)];

            // Find ALL "X% used" or "X% 사용됨" occurrences in order
            const pctMatches = [];
            const pctRe = /(\d+(?:\.\d+)?)\s*%\s*(?:used|사용됨)/gi;
            let m;
            while ((m = pctRe.exec(t)) !== null) {
                pctMatches.push(parseFloat(m[1]));
            }

            // Also try bare "X%" near section keywords if above fails
            if (pctMatches.length === 0) {
                // Try finding percentages from aria/progress elements
                const bars = document.querySelectorAll('[role="progressbar"], progress, [aria-valuenow]');
                bars.forEach(b => {
                    const v = b.getAttribute('aria-valuenow') || b.getAttribute('value');
                    if (v) pctMatches.push(parseFloat(v));
                });
            }

            // Assign in order: session, all models, sonnet
            if (pctMatches.length >= 1) result.session_percent = pctMatches[0];
            if (pctMatches.length >= 2) result.weekly_all_percent = pctMatches[1];
            if (pctMatches.length >= 3) result.weekly_sonnet_percent = pctMatches[2];

            // Reset time: "Resets in X hr Y min" or just "X hr Y min"
            const rHM = t.match(/[Rr]esets?\s+in\s+(\d+)\s*h[r]?\s+(\d+)\s*min/);
            if (rHM) {
                result.session_reset_minutes = parseInt(rHM[1]) * 60 + parseInt(rHM[2]);
            } else {
                // Hours only: "Resets in 3 hr"
                const rH = t.match(/[Rr]esets?\s+in\s+(\d+)\s*h[r]?/);
                if (rH) result.session_reset_minutes = parseInt(rH[1]) * 60;
                // Minutes only: "Resets in 45 min"
                const rM = t.match(/[Rr]esets?\s+in\s+(\d+)\s*min/);
                if (rM) result.session_reset_minutes = parseInt(rM[1]);
            }
            // Korean fallback
            if (result.session_reset_minutes === 0) {
                const rK = t.match(/(\d+)\s*시간\s*(\d+)\s*분\s*후/);
                if (rK) result.session_reset_minutes = parseInt(rK[1]) * 60 + parseInt(rK[2]);
            }

            // Extra usage / monthly cost scraping
            // Try multiple patterns for cost and limit

            // Pattern 1: "$XX.XX used" or "US$XX.XX used"
            const costM = t.match(/(?:US)?\$\s*(\d+(?:\.\d+)?)\s*(?:used|사용)/i);
            if (costM) result.monthly_cost = parseFloat(costM[1]);

            // Pattern 2: "$X.XX / $XX.XX" (cost / limit format)
            const slashM = t.match(/\$\s*(\d+(?:\.\d+)?)\s*\/\s*\$\s*(\d+(?:\.\d+)?)/);
            if (slashM) {
                result.monthly_cost = parseFloat(slashM[1]);
                result.monthly_limit = parseFloat(slashM[2]);
            }

            // Pattern 3: "of $XX" or "/ $XX" for limit
            if (result.monthly_limit === 0) {
                const limitM = t.match(/(?:\/|of)\s*(?:US)?\$\s*(\d+(?:\.\d+)?)/i);
                if (limitM) result.monthly_limit = parseFloat(limitM[1]);
            }

            // Pattern 4: Look near "Extra usage" or "extra" section for dollar amounts
            const extraIdx = t.toLowerCase().indexOf('extra usage');
            if (extraIdx !== -1) {
                const extraSection = t.substring(extraIdx, extraIdx + 300);
                const dollarMatches = [...extraSection.matchAll(/\$\s*(\d+(?:\.\d+)?)/g)];
                if (dollarMatches.length >= 2 && result.monthly_cost === 0) {
                    result.monthly_cost = parseFloat(dollarMatches[0][1]);
                    result.monthly_limit = parseFloat(dollarMatches[1][1]);
                } else if (dollarMatches.length >= 1 && result.monthly_limit === 0) {
                    result.monthly_limit = parseFloat(dollarMatches[0][1]);
                }
            }

            // Pattern 5: "Limit: $XX" or "limit of $XX"
            const limitPattern = t.match(/limit(?:\s+of)?\s*:?\s*\$\s*(\d+(?:\.\d+)?)/i);
            if (limitPattern && result.monthly_limit === 0) {
                result.monthly_limit = parseFloat(limitPattern[1]);
            }

            return JSON.stringify(result);
        } catch(e) {
            return JSON.stringify({ error: e.message, session_percent: 0, session_reset_minutes: 0, weekly_all_percent: 0, weekly_sonnet_percent: 0, monthly_cost: 0, monthly_limit: 0 });
        }
    })()
    "#
}
