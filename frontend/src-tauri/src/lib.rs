mod history;
mod usage;

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, Url,
};

fn log(msg: &str) {
    use std::io::Write;
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] [BurnRate] {}\n", ts, msg);
    if let Some(home) = dirs::home_dir() {
        let path = home.join("burnrate-debug.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

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
    pub failed_polls: Mutex<u32>,
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
fn get_history() -> Vec<history::HistoryEntry> {
    history::load_history()
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

/// Wake a hidden scraper WebView so macOS doesn't suspend its WebProcess.
/// Moves offscreen, shows at 1x1, waits for wake, then caller can eval().
fn wake_scraper_window(window: &tauri::WebviewWindow) {
    log("Waking scraper window: moving offscreen (full size) + show");
    // Use a reasonable size so macOS actually renders the page (1x1 causes blank content)
    let _ = window.set_size(tauri::PhysicalSize::new(900u32, 700u32));
    let _ = window.set_position(tauri::PhysicalPosition::new(-20000i32, -20000i32));
    let _ = window.show();
}

/// Hide the scraper window again after scraping.
fn sleep_scraper_window(window: &tauri::WebviewWindow) {
    log("Hiding scraper window again");
    let _ = window.hide();
}

#[tauri::command]
async fn trigger_scrape(app: AppHandle) -> Result<UsageData, String> {
    if app.get_webview_window("scraper").is_none() {
        build_scraper_window(&app, false)?;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }

    if let Some(window) = app.get_webview_window("scraper") {
        wake_scraper_window(&window);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let _ = window.eval(
            "if (!window.location.href.includes('/settings/usage')) { window.location.href = 'https://claude.ai/settings/usage'; }"
        );
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let js = build_scrape_inject_js();
        let _ = window.eval(&js);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        sleep_scraper_window(&window);
    }

    let state = app.state::<AppState>();
    let data = state.usage.lock().unwrap().clone();
    Ok(data)
}

/// Build the scraper WebView window with on_navigation handler
fn build_scraper_window(app: &AppHandle, visible: bool) -> Result<(), String> {
    let app_handle = app.clone();
    log(&format!("Building scraper window, visible={}", visible));

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
            log("on_navigation: received burnrate://result/");
            let encoded = &url_str["burnrate://result/".len()..];
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(json_str) = String::from_utf8(bytes) {
                    log(&format!("Decoded JSON: {}", &json_str[..json_str.len().min(300)]));
                    if let Ok(scraped) = serde_json::from_str::<WebScrapedData>(&json_str) {
                        if scraped.error.is_none() {
                            log(&format!(
                                "Parse success: session={}%, weekly={}%, reset={}min",
                                scraped.session_percent, scraped.weekly_all_percent, scraped.session_reset_minutes
                            ));
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
                            // Reset failed polls on success
                            {
                                let mut fp = state.failed_polls.lock().unwrap();
                                *fp = 0;
                            }

                            let data = state.usage.lock().unwrap().clone();
                            let failed = *state.failed_polls.lock().unwrap();
                            let title = format_tray_title(&data, failed);
                            if let Some(tray) = app_handle.tray_by_id("main-tray") {
                                let _ = tray.set_title(Some(&title));
                            }
                            let _ = app_handle.emit("usage-updated", &data);
                            history::append_entry(
                                scraped.session_percent,
                                scraped.weekly_all_percent,
                                scraped.weekly_sonnet_percent,
                            );
                        } else {
                            log(&format!("Scrape returned error: {:?}", scraped.error));
                        }
                    } else {
                        log("Failed to parse JSON from scraper");
                    }
                }
            }
            return false;
        }

        // After login, redirect to usage page
        if (url_str == "https://claude.ai/"
            || url_str == "https://claude.ai"
            || url_str.starts_with("https://claude.ai/new")
            || url_str.starts_with("https://claude.ai/chat"))
            && !url_str.contains("/settings/")
        {
            log("User landed on main page, redirecting to usage...");
            let handle = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if let Some(w) = handle.get_webview_window("scraper") {
                    let _ = w.eval("window.location.href = 'https://claude.ai/settings/usage';");
                }
            });
        }

        // When landing on usage page, auto-inject scraping JS after delay
        if url_str.contains("/settings/usage") {
            log("Landed on usage page, will auto-inject scraping JS...");
            let handle = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if let Some(w) = handle.get_webview_window("scraper") {
                    let js = build_scrape_inject_js_static();
                    let _ = w.eval(&js);
                }
            });
        }

        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Prevent scraper window from being destroyed on close — hide instead
    if let Some(w) = app.get_webview_window("scraper") {
        let window = w.clone();
        w.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                log("Scraper window close intercepted — hidden instead");
            }
        });
    }

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

/// Same as build_scrape_inject_js but can be called from any context
fn build_scrape_inject_js_static() -> String {
    build_scrape_inject_js()
}

fn format_tray_title(usage: &UsageData, failed_polls: u32) -> String {
    if failed_polls >= 3 {
        return "⚠️ Login required".to_string();
    }
    if usage.web_connected {
        let reset_str = if usage.session_reset_minutes <= 0 {
            String::new()
        } else if usage.session_reset_minutes < 60 {
            format!("{}m", usage.session_reset_minutes)
        } else {
            let hours = usage.session_reset_minutes / 60;
            let mins = usage.session_reset_minutes % 60;
            if mins == 0 {
                format!("{}h", hours)
            } else {
                format!("{}h{}m", hours, mins)
            }
        };

        // When session is at 100% and extra usage is active, show cost info
        if usage.session_percent >= 100.0 && usage.monthly_cost > 0.0 && usage.monthly_limit > 0.0 {
            let remaining = usage.monthly_limit - usage.monthly_cost;
            let remaining_str = if remaining >= 0.0 {
                format!("${:.0}left", remaining)
            } else {
                format!("-${:.0}over", -remaining)
            };
            if reset_str.is_empty() {
                format!(
                    "⚡100% 💰{} 🔋{}%",
                    remaining_str,
                    usage.weekly_all_percent as i64,
                )
            } else {
                format!(
                    "⚡100%({}) 💰{} 🔋{}%",
                    reset_str,
                    remaining_str,
                    usage.weekly_all_percent as i64,
                )
            }
        } else if reset_str.is_empty() {
            format!(
                "⚡{}% 🔋{}%",
                usage.session_percent as i64,
                usage.weekly_all_percent as i64,
            )
        } else {
            format!(
                "⚡{}%({}) 🔋{}%",
                usage.session_percent as i64,
                reset_str,
                usage.weekly_all_percent as i64,
            )
        }
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

            log("Poll start");

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
                // Check if we've ever connected — if not, show the window for login
                let ever_connected = app.state::<AppState>().usage.lock().unwrap().web_connected;
                let show = !ever_connected;
                log(&format!("Scraper window missing, creating (visible={})...", show));
                let _ = build_scraper_window(&app, show);
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            }

            // Record web_connected before scrape to detect if on_navigation fires
            let was_updated = {
                let state = app.state::<AppState>();
                let v = state.usage.lock().unwrap().last_updated.clone();
                v
            };

            if let Some(window) = app.get_webview_window("scraper") {
                // Check if window is visible (user might be logging in)
                let is_visible = window.is_visible().unwrap_or(false);
                let is_connected = app.state::<AppState>().usage.lock().unwrap().web_connected;

                if !is_connected && is_visible {
                    // Window is visible and not connected = user is logging in, don't interfere
                    log("Scraper visible but not connected — waiting for user login");
                } else {
                    // Either connected (do regular scrape) or hidden (wake + scrape)
                    if !is_visible {
                        wake_scraper_window(&window);
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }

                    // Force navigate to usage page
                    log("Force navigating to usage page");
                    let _ = window.eval(
                        "window.location.href = 'https://claude.ai/settings/usage';"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

                    log("Injecting scraping JS");
                    let js = build_scrape_inject_js();
                    let _ = window.eval(&js);

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                    // Hide if it was hidden before
                    if !is_visible {
                        sleep_scraper_window(&window);
                    }
                }
            }

            // Check if scrape succeeded by comparing last_updated
            {
                let state = app.state::<AppState>();
                let current_updated = state.usage.lock().unwrap().last_updated.clone();
                if current_updated == was_updated {
                    // No update happened — increment failed polls
                    let mut fp = state.failed_polls.lock().unwrap();
                    *fp += 1;
                    log(&format!("Scrape did not update data, failed_polls={}", *fp));
                }
            }

            // Update tray and emit
            {
                let state = app.state::<AppState>();
                let data = state.usage.lock().unwrap().clone();
                let failed = *state.failed_polls.lock().unwrap();
                let title = format_tray_title(&data, failed);
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
            failed_polls: Mutex::new(0),
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

            log("BurnRate started, beginning polling");
            start_polling(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage,
            get_config,
            save_config,
            get_history,
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
