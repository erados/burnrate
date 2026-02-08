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

            // Collect all visible text for debugging
            const allText = document.body ? document.body.innerText : '';
            
            // Strategy: find all progress-bar-like elements and their labels
            // The usage page has sections with percentages and progress bars
            
            // Look for percentage patterns like "X% 사용됨" or "X% used"
            const percentRegex = /(\d+(?:\.\d+)?)\s*%\s*(사용됨|used)/gi;
            const percentMatches = [...allText.matchAll(percentRegex)];
            
            // Look for reset time "X시간 Y분 후 재설정" or similar
            const resetRegex = /(\d+)\s*시간\s*(\d+)\s*분\s*후\s*재설정/;
            const resetMatch = allText.match(resetRegex);
            if (resetMatch) {
                result.session_reset_minutes = parseInt(resetMatch[1]) * 60 + parseInt(resetMatch[2]);
            }
            // Also try English pattern
            const resetRegexEn = /(\d+)\s*h(?:ours?)?\s*(\d+)\s*m(?:in(?:utes?)?)?\s*(?:until reset|remaining)/i;
            const resetMatchEn = allText.match(resetRegexEn);
            if (!resetMatch && resetMatchEn) {
                result.session_reset_minutes = parseInt(resetMatchEn[1]) * 60 + parseInt(resetMatchEn[2]);
            }
            // Minutes only
            const resetMinOnly = allText.match(/(\d+)\s*분\s*후\s*재설정/);
            if (!resetMatch && resetMinOnly) {
                result.session_reset_minutes = parseInt(resetMinOnly[1]);
            }
            
            // Look for cost pattern "US$XX.XX 사용" or "$XX.XX used"
            const costRegex = /(?:US)?\$\s*(\d+(?:\.\d+)?)\s*(사용|used)/i;
            const costMatch = allText.match(costRegex);
            if (costMatch) {
                result.monthly_cost = parseFloat(costMatch[1]);
            }
            
            // Look for limit like "US$50" or "/ $50"
            const limitRegex = /(?:\/|of)\s*(?:US)?\$\s*(\d+(?:\.\d+)?)/i;
            const limitMatch = allText.match(limitRegex);
            if (limitMatch) {
                result.monthly_limit = parseFloat(limitMatch[1]);
            }
            // Also try standalone limit pattern
            if (!limitMatch) {
                const standaloneLimitRegex = /(?:US)?\$(\d+(?:\.\d+)?)\s*(?:한도|limit)/i;
                const standaloneMatch = allText.match(standaloneLimitRegex);
                if (standaloneMatch) {
                    result.monthly_limit = parseFloat(standaloneMatch[1]);
                }
            }

            // Try to find sections by looking at the DOM structure
            // Claude.ai usage page typically has sections with headers
            const sections = document.querySelectorAll('[class*="usage"], [class*="section"], [class*="card"], [class*="panel"], section, [role="region"]');
            
            // More robust: find all elements containing percentage text
            const walker = document.createTreeWalker(
                document.body,
                NodeFilter.SHOW_TEXT,
                null,
                false
            );
            
            let textNodes = [];
            let node;
            while (node = walker.nextNode()) {
                const text = node.textContent.trim();
                if (text && (text.includes('%') || text.includes('$') || text.includes('재설정') || text.includes('reset'))) {
                    textNodes.push({
                        text: text,
                        parent: node.parentElement ? node.parentElement.closest('[class]')?.className || '' : ''
                    });
                }
            }
            
            result.raw_texts = textNodes.slice(0, 20);

            // Try to identify sections by their headings
            // Look for heading elements near percentage elements
            const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6, [class*="heading"], [class*="title"], [class*="label"]');
            let currentSection = '';
            let sectionPercents = {};
            
            for (const heading of headings) {
                const headText = heading.textContent.trim().toLowerCase();
                // Find the next sibling or nearby element with a percentage
                const parent = heading.closest('div, section, [class*="card"]');
                if (parent) {
                    const pctMatch = parent.textContent.match(/(\d+(?:\.\d+)?)\s*%/);
                    if (pctMatch) {
                        const pct = parseFloat(pctMatch[1]);
                        if (headText.includes('session') || headText.includes('세션') || headText.includes('current')) {
                            result.session_percent = pct;
                        } else if (headText.includes('all model') || headText.includes('모든 모델') || headText.includes('전체')) {
                            result.weekly_all_percent = pct;
                        } else if (headText.includes('sonnet')) {
                            result.weekly_sonnet_percent = pct;
                        }
                    }
                }
            }

            // Fallback: assign percentages by order if we found them via regex
            if (result.session_percent === 0 && percentMatches.length > 0) {
                // Usually: session %, all models %, sonnet %
                if (percentMatches.length >= 1) result.session_percent = parseFloat(percentMatches[0][1]);
                if (percentMatches.length >= 2) result.weekly_all_percent = parseFloat(percentMatches[1][1]);
                if (percentMatches.length >= 3) result.weekly_sonnet_percent = parseFloat(percentMatches[2][1]);
            }

            return JSON.stringify(result);
        } catch(e) {
            return JSON.stringify({ error: e.message, session_percent: 0, session_reset_minutes: 0, weekly_all_percent: 0, weekly_sonnet_percent: 0, monthly_cost: 0, monthly_limit: 0 });
        }
    })()
    "#
}
