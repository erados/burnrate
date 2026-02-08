mod usage;

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, Url,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub session_percent: f64,
    pub session_reset_minutes: i64,
    pub weekly_all_percent: f64,
    pub weekly_sonnet_percent: f64,
    pub monthly_cost: f64,
    pub monthly_limit: f64,
    pub today_messages: u64,
    pub today_tokens: u64,
    pub opus_tokens: u64,
    pub sonnet_tokens: u64,
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
fn hide_scraper(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("scraper") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
fn open_claude_login(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("scraper") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        build_scraper_window(&app, true)?;
    }
    Ok(())
}

#[tauri::command]
async fn trigger_scrape(app: AppHandle) -> Result<UsageData, String> {
    // Ensure scraper window exists
    if app.get_webview_window("scraper").is_none() {
        build_scraper_window(&app, false)?;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }

    if let Some(window) = app.get_webview_window("scraper") {
        // Navigate to usage page
        let _ = window.eval(
            "if (!window.location.href.includes('/settings/usage')) { window.location.href = 'https://claude.ai/settings/usage'; }"
        );
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Inject scraping JS
        let js = build_scrape_inject_js();
        let _ = window.eval(&js);

        // Wait for navigation handler to process
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    let state = app.state::<AppState>();
    let data = state.usage.lock().unwrap().clone();
    Ok(data)
}

/// Build the scraper WebView window with on_navigation handler
fn build_scraper_window(app: &AppHandle, visible: bool) -> Result<(), String> {
    let app_handle = app.clone();

    tauri::WebviewWindowBuilder::new(
        app,
        "scraper",
        tauri::WebviewUrl::External(
            Url::parse("https://claude.ai/settings/usage").unwrap(),
        ),
    )
    .title("Login to Claude")
    .inner_size(900.0, 700.0)
    .visible(visible)
    .on_navigation(move |url| {
        let url_str = url.as_str();

        // Intercept burnrate://result/<base64> URLs
        if url_str.starts_with("burnrate://result/") {
            let encoded = &url_str["burnrate://result/".len()..];
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(json_str) = String::from_utf8(bytes) {
                    if let Ok(scraped) = serde_json::from_str::<WebScrapedData>(&json_str) {
                        if scraped.error.is_none() {
                            let now = chrono::Utc::now().format("%H:%M:%S").to_string();
                            let state = app_handle.state::<AppState>();
                            {
                                let mut usage = state.usage.lock().unwrap();
                                usage.session_percent = scraped.session_percent;
                                usage.session_reset_minutes = scraped.session_reset_minutes;
                                usage.weekly_all_percent = scraped.weekly_all_percent;
                                usage.weekly_sonnet_percent = scraped.weekly_sonnet_percent;
                                usage.monthly_cost = scraped.monthly_cost;
                                usage.monthly_limit = scraped.monthly_limit;
                                usage.web_connected = true;
                                usage.last_updated = now;
                            }

                            // Update tray and emit
                            let data = state.usage.lock().unwrap().clone();
                            let title = format_tray_title(&data);
                            if let Some(tray) = app_handle.tray_by_id("main-tray") {
                                let _ = tray.set_title(Some(&title));
                            }
                            let _ = app_handle.emit("usage-updated", &data);
                        }
                    }
                }
            }
            return false; // Don't navigate to burnrate:// URL
        }

        // After login, if user lands on claude.ai main page, redirect to usage
        // This handles: claude.ai, claude.ai/new, claude.ai/chat*, etc.
        if (url_str == "https://claude.ai/"
            || url_str == "https://claude.ai"
            || url_str.starts_with("https://claude.ai/new")
            || url_str.starts_with("https://claude.ai/chat"))
            && !url_str.contains("/settings/")
        {
            let handle = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if let Some(w) = handle.get_webview_window("scraper") {
                    let _ = w.eval("window.location.href = 'https://claude.ai/settings/usage';");
                }
            });
        }

        // Allow all normal navigation (https, http, about, etc.)
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Build the JS string that scrapes and navigates to burnrate://result/
fn build_scrape_inject_js() -> String {
    format!(
        r#"
        (function() {{
            try {{
                const jsonStr = {scrape_js};
                const encoded = btoa(unescape(encodeURIComponent(jsonStr)));
                window.location.href = 'burnrate://result/' + encoded;
            }} catch(e) {{
                console.error('BurnRate scrape error:', e);
            }}
        }})();
        "#,
        scrape_js = usage::scraping_js()
    )
}

fn format_tray_title(usage: &UsageData) -> String {
    if usage.web_connected {
        let reset_str = if usage.session_reset_minutes <= 0 {
            String::new()
        } else if usage.session_reset_minutes < 60 {
            format!(" {}m", usage.session_reset_minutes)
        } else {
            let hours = usage.session_reset_minutes / 60;
            let mins = usage.session_reset_minutes % 60;
            if mins == 0 {
                format!(" {}h", hours)
            } else {
                format!(" {}h{}m", hours, mins)
            }
        };
        format!(
            "⚡{}%{} | 🔋{}%",
            usage.session_percent as i64,
            reset_str,
            usage.weekly_all_percent as i64,
        )
    } else {
        "🔥 loading...".to_string()
    }
}

fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Initial delay
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        loop {
            let interval = {
                let state = app.state::<AppState>();
                let v = state.config.lock().unwrap().poll_interval_secs;
                v
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

            // Try web scraping - create scraper window if needed
            if app.get_webview_window("scraper").is_none() {
                let _ = build_scraper_window(&app, false);
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            }
            if let Some(window) = app.get_webview_window("scraper") {
                // Navigate to usage page if needed
                let _ = window.eval(
                    "if (!window.location.href.includes('/settings/usage')) { window.location.href = 'https://claude.ai/settings/usage'; }"
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                // Inject scraping JS — result comes back via on_navigation handler
                let js = build_scrape_inject_js();
                let _ = window.eval(&js);

                // Wait for navigation handler to process
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            usage: Mutex::new(UsageData::default()),
            config: Mutex::new(AppConfig::default()),
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
                            .inner_size(440.0, 520.0)
                            .resizable(false)
                            .build();
                        }
                    }
                    "login" => {
                        if let Some(window) = app.get_webview_window("scraper") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let _ = build_scraper_window(app, true);
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
                            .inner_size(440.0, 520.0)
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building BurnRate")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
