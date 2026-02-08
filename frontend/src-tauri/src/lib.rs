mod usage;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, Url,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    // From claude.ai web (primary)
    pub session_percent: f64,
    pub session_reset_minutes: i64,
    pub weekly_all_percent: f64,
    pub weekly_sonnet_percent: f64,
    pub monthly_cost: f64,
    pub monthly_limit: f64,
    // From local logs (supplementary)
    pub today_messages: u64,
    pub today_tokens: u64,
    pub opus_tokens: u64,
    pub sonnet_tokens: u64,
    // Status
    pub web_connected: bool,
    pub last_updated: String,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            session_percent: 0.0,
            session_reset_minutes: 0,
            weekly_all_percent: 0.0,
            weekly_sonnet_percent: 0.0,
            monthly_cost: 0.0,
            monthly_limit: 0.0,
            today_messages: 0,
            today_tokens: 0,
            opus_tokens: 0,
            sonnet_tokens: 0,
            web_connected: false,
            last_updated: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub poll_interval_secs: u64,
    pub display_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            display_mode: "all".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebScrapedData {
    #[serde(default)]
    session_percent: f64,
    #[serde(default)]
    session_reset_minutes: i64,
    #[serde(default)]
    weekly_all_percent: f64,
    #[serde(default)]
    weekly_sonnet_percent: f64,
    #[serde(default)]
    monthly_cost: f64,
    #[serde(default)]
    monthly_limit: f64,
    #[serde(default)]
    error: Option<String>,
}

pub struct AppState {
    pub usage: Mutex<UsageData>,
    pub config: Mutex<AppConfig>,
    pub scraper_ready: Mutex<bool>,
}

#[tauri::command]
fn get_usage(state: State<AppState>) -> UsageData {
    state.usage.lock().unwrap().clone()
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}

#[tauri::command]
fn open_claude_login(app: AppHandle) -> Result<(), String> {
    // Show or create the scraper window for login
    if let Some(window) = app.get_webview_window("scraper") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            &app,
            "scraper",
            tauri::WebviewUrl::External(
                Url::parse("https://claude.ai/settings/usage").unwrap(),
            ),
        )
        .title("Login to Claude")
        .inner_size(900.0, 700.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_scraper(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("scraper") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
async fn trigger_scrape(app: AppHandle) -> Result<UsageData, String> {
    do_scrape(&app).await
}

async fn do_scrape(app: &AppHandle) -> Result<UsageData, String> {
    // Get local log data first
    let (msgs, tokens, opus, sonnet) = usage::read_local_usage();

    // Try to scrape from the WebView
    let web_data = scrape_webview(app).await;

    let now = chrono::Utc::now().format("%H:%M:%S").to_string();

    let data = match web_data {
        Ok(scraped) => UsageData {
            session_percent: scraped.session_percent,
            session_reset_minutes: scraped.session_reset_minutes,
            weekly_all_percent: scraped.weekly_all_percent,
            weekly_sonnet_percent: scraped.weekly_sonnet_percent,
            monthly_cost: scraped.monthly_cost,
            monthly_limit: scraped.monthly_limit,
            today_messages: msgs,
            today_tokens: tokens,
            opus_tokens: opus,
            sonnet_tokens: sonnet,
            web_connected: true,
            last_updated: now,
        },
        Err(_) => UsageData {
            session_percent: 0.0,
            session_reset_minutes: 0,
            weekly_all_percent: 0.0,
            weekly_sonnet_percent: 0.0,
            monthly_cost: 0.0,
            monthly_limit: 0.0,
            today_messages: msgs,
            today_tokens: tokens,
            opus_tokens: opus,
            sonnet_tokens: sonnet,
            web_connected: false,
            last_updated: now,
        },
    };

    // Update state
    {
        let state = app.state::<AppState>();
        let mut usage = state.usage.lock().unwrap();
        *usage = data.clone();
    }

    // Update tray
    let title = format_tray_title(&data);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_title(Some(&title));
    }

    // Emit to frontend
    let _ = app.emit("usage-updated", &data);

    Ok(data)
}

async fn scrape_webview(app: &AppHandle) -> Result<WebScrapedData, String> {
    // Ensure scraper window exists (hidden)
    let window = if let Some(w) = app.get_webview_window("scraper") {
        w
    } else {
        let w = tauri::WebviewWindowBuilder::new(
            app,
            "scraper",
            tauri::WebviewUrl::External(
                Url::parse("https://claude.ai/settings/usage").unwrap(),
            ),
        )
        .title("Claude Scraper")
        .inner_size(900.0, 700.0)
        .visible(false)
        .build()
        .map_err(|e| format!("Failed to create scraper window: {}", e))?;
        
        // Wait for initial page load
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
        w
    };

    // Navigate to usage page (in case it's on a different page)
    let _ = window.eval(&format!(
        "if (!window.location.href.includes('/settings/usage')) {{ window.location.href = 'https://claude.ai/settings/usage'; }}"
    ));

    // Wait for content to render
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Inject scraping JS
    let js = usage::scraping_js();
    
    // Use eval to run JS - we'll use a workaround since eval() doesn't return values directly
    // Instead, we'll have the JS post a message back via __TAURI__
    let js_with_callback = format!(
        r#"
        (async function() {{
            const result = (function() {{ {} }})();
            // Store result globally so we can retrieve it
            window.__burnrate_result = result;
        }})();
        "#,
        js
    );
    
    window.eval(&js_with_callback).map_err(|e| format!("JS eval failed: {}", e))?;
    
    // Small delay for JS execution
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Retrieve the result
    let retrieve_js = r#"
        (function() {
            if (window.__burnrate_result) {
                window.__TAURI_INTERNALS__.invoke('receive_scrape_result', { data: window.__burnrate_result });
            }
        })();
    "#;
    window.eval(retrieve_js).map_err(|e| format!("Retrieve eval failed: {}", e))?;
    
    // Wait and check state for result
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Check if we got a result via the scraper_ready flag
    let state = app.state::<AppState>();
    let ready = *state.scraper_ready.lock().unwrap();
    if !ready {
        return Err("Scraper not ready or not logged in".to_string());
    }
    
    // Since we can't easily get return values from eval in Tauri v2,
    // let's use a different approach: use the window's event system
    Err("Use event-based scraping".to_string())
}

fn format_tray_title(usage: &UsageData) -> String {
    if usage.web_connected {
        let cost_str = if usage.monthly_limit > 0.0 {
            format!("${:.0}/${:.0}", usage.monthly_cost, usage.monthly_limit)
        } else {
            format!("${:.0}", usage.monthly_cost)
        };
        format!(
            "⚡{}% | 📅{}% | 💰{}",
            usage.session_percent as i64,
            usage.weekly_all_percent as i64,
            cost_str,
        )
    } else {
        let tok_k = usage.today_tokens / 1000;
        format!("🔥 {}msg | {}k tok", usage.today_messages, tok_k)
    }
}

fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Initial delay to let the app start up
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        loop {
            let interval = {
                let state = app.state::<AppState>();
                let val = state.config.lock().unwrap().poll_interval_secs;
                val
            };

            // Read local data
            let (msgs, tokens, opus, sonnet) = usage::read_local_usage();
            let now = chrono::Utc::now().format("%H:%M:%S").to_string();

            {
                let state = app.state::<AppState>();
                let mut usage = state.usage.lock().unwrap();
                usage.today_messages = msgs;
                usage.today_tokens = tokens;
                usage.opus_tokens = opus;
                usage.sonnet_tokens = sonnet;
                usage.last_updated = now;
            }

            // Try web scraping if scraper window exists
            if let Some(window) = app.get_webview_window("scraper") {
                // Navigate to usage page
                let _ = window.eval(
                    "if (!window.location.href.includes('/settings/usage')) { window.location.href = 'https://claude.ai/settings/usage'; }"
                );
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                // Inject scraping JS and send result back via custom event
                let js = format!(
                    r#"
                    (function() {{
                        try {{
                            const result = (function() {{ {} }})();
                            // Post result to Tauri
                            if (window.__TAURI_INTERNALS__) {{
                                window.__TAURI_INTERNALS__.invoke('receive_scrape_result', {{ data: result }});
                            }}
                        }} catch(e) {{
                            console.error('Scrape error:', e);
                        }}
                    }})();
                    "#,
                    usage::scraping_js()
                );
                let _ = window.eval(&js);
                
                // Wait for result to be processed
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            // Update tray and emit
            {
                let state = app.state::<AppState>();
                let data = state.usage.lock().unwrap().clone();
                let title = format_tray_title(&data);
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_title(Some(&title));
                }
                let _ = app.emit("usage-updated", &data);
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    });
}

#[tauri::command]
fn receive_scrape_result(state: State<AppState>, data: String) -> Result<(), String> {
    let parsed: WebScrapedData = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    
    if parsed.error.is_some() {
        return Err(parsed.error.unwrap());
    }

    let mut usage = state.usage.lock().unwrap();
    usage.session_percent = parsed.session_percent;
    usage.session_reset_minutes = parsed.session_reset_minutes;
    usage.weekly_all_percent = parsed.weekly_all_percent;
    usage.weekly_sonnet_percent = parsed.weekly_sonnet_percent;
    usage.monthly_cost = parsed.monthly_cost;
    usage.monthly_limit = parsed.monthly_limit;
    usage.web_connected = true;
    
    let mut ready = state.scraper_ready.lock().unwrap();
    *ready = true;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            usage: Mutex::new(UsageData::default()),
            config: Mutex::new(AppConfig::default()),
            scraper_ready: Mutex::new(false),
        })
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "Dashboard").build(app)?;
            let login = MenuItemBuilder::with_id("login", "Login to Claude").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit BurnRate").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show)
                .item(&login)
                .separator()
                .item(&quit)
                .build()?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(false)
                .title("🔥 loading...")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let _ = tauri::WebviewWindowBuilder::new(
                                app,
                                "main",
                                tauri::WebviewUrl::App("index.html".into()),
                            )
                            .title("BurnRate")
                            .inner_size(440.0, 480.0)
                            .resizable(false)
                            .build();
                        }
                    }
                    "login" => {
                        if let Some(window) = app.get_webview_window("scraper") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let _ = tauri::WebviewWindowBuilder::new(
                                app,
                                "scraper",
                                tauri::WebviewUrl::External(
                                    Url::parse("https://claude.ai/settings/usage").unwrap(),
                                ),
                            )
                            .title("Login to Claude")
                            .inner_size(900.0, 700.0)
                            .build();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let _ = tauri::WebviewWindowBuilder::new(
                                app,
                                "main",
                                tauri::WebviewUrl::App("index.html".into()),
                            )
                            .title("BurnRate")
                            .inner_size(440.0, 480.0)
                            .resizable(false)
                            .build();
                        }
                    }
                })
                .build(app)?;

            start_polling(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage,
            get_config,
            save_config,
            open_claude_login,
            hide_scraper,
            trigger_scrape,
            receive_scrape_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BurnRate");
}
