mod usage;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub session_percent: f64,
    pub session_reset_minutes: i64,
    pub weekly_all_percent: f64,
    pub weekly_sonnet_percent: f64,
    pub weekly_reset_hours: i64,
    pub monthly_cost: f64,
    pub monthly_limit: f64,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            session_percent: 0.0,
            session_reset_minutes: 0,
            weekly_all_percent: 0.0,
            weekly_sonnet_percent: 0.0,
            weekly_reset_hours: 0,
            monthly_cost: 0.0,
            monthly_limit: 50.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_key: String,
    pub org_id: String,
    pub poll_interval_secs: u64,
    pub monthly_limit: f64,
    pub session_alert_threshold: f64,
    pub monthly_alert_threshold: f64,
    pub display_mode: String, // "all" | "critical"
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            org_id: String::new(),
            poll_interval_secs: 300,
            monthly_limit: 50.0,
            session_alert_threshold: 80.0,
            monthly_alert_threshold: 90.0,
            display_mode: "all".to_string(),
        }
    }
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
    // Store API key in keychain
    if !config.api_key.is_empty() {
        let entry = keyring::Entry::new("burnrate", "anthropic-api-key")
            .map_err(|e| e.to_string())?;
        entry.set_password(&config.api_key).map_err(|e| e.to_string())?;
    }

    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}

#[tauri::command]
fn load_api_key() -> Result<String, String> {
    let entry = keyring::Entry::new("burnrate", "anthropic-api-key")
        .map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

fn format_tray_title(usage: &UsageData, mode: &str) -> String {
    match mode {
        "critical" => {
            // Show only the most critical metric
            let monthly_pct = (usage.monthly_cost / usage.monthly_limit * 100.0) as i64;
            if usage.session_percent >= 80.0 {
                format!("⚡{}%", usage.session_percent as i64)
            } else if monthly_pct >= 80 {
                format!("💰${:.0}/${:.0}", usage.monthly_cost, usage.monthly_limit)
            } else if usage.weekly_all_percent >= 50.0 {
                format!("📅{}%", usage.weekly_all_percent as i64)
            } else {
                format!("⚡{}%", usage.session_percent as i64)
            }
        }
        _ => {
            format!(
                "⚡{}% | 📅{}% | 💰${:.0}/${:.0}",
                usage.session_percent as i64,
                usage.weekly_all_percent as i64,
                usage.monthly_cost,
                usage.monthly_limit
            )
        }
    }
}

fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let interval = {
                let state = app.state::<AppState>();
                let config = state.config.lock().unwrap();
                let usage = state.usage.lock().unwrap();

                // Update tray title
                let title = format_tray_title(&usage, &config.display_mode);
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_title(Some(&title));
                }

                config.poll_interval_secs
            };

            // Fetch usage data
            let (api_key, org_id) = {
                let state = app.state::<AppState>();
                let config = state.config.lock().unwrap();
                (config.api_key.clone(), config.org_id.clone())
            };

            let new_usage = if !api_key.is_empty() && !org_id.is_empty() {
                usage::fetch_usage(&api_key, &org_id).await.ok()
            } else {
                Some(usage::mock_usage())
            };

            if let Some(data) = new_usage {
                let state = app.state::<AppState>();

                // Check thresholds before updating
                {
                    let config = state.config.lock().unwrap();
                    let old = state.usage.lock().unwrap();

                    if data.session_percent >= config.session_alert_threshold
                        && old.session_percent < config.session_alert_threshold
                    {
                        let _ = app.emit("notification", serde_json::json!({
                            "title": "BurnRate ⚡",
                            "body": format!("Session usage at {}% — slow down!", data.session_percent as i64)
                        }));
                    }

                    let monthly_pct = data.monthly_cost / config.monthly_limit * 100.0;
                    let old_monthly_pct = old.monthly_cost / config.monthly_limit * 100.0;
                    if monthly_pct >= config.monthly_alert_threshold
                        && old_monthly_pct < config.monthly_alert_threshold
                    {
                        let _ = app.emit("notification", serde_json::json!({
                            "title": "BurnRate 💰",
                            "body": format!("Monthly cost ${:.2} — limit approaching!", data.monthly_cost)
                        }));
                    }
                }

                // Update stored usage
                {
                    let mut usage = state.usage.lock().unwrap();
                    *usage = data.clone();
                }

                // Update tray
                {
                    let config = state.config.lock().unwrap();
                    let title = format_tray_title(&data, &config.display_mode);
                    if let Some(tray) = app.tray_by_id("main-tray") {
                        let _ = tray.set_title(Some(&title));
                    }
                }

                // Emit to frontend
                let _ = app.emit("usage-updated", &data);
            }

            // Adaptive polling: faster when usage is high
            let sleep_secs = {
                let state = app.state::<AppState>();
                let usage = state.usage.lock().unwrap();
                if usage.session_percent >= 70.0 || usage.monthly_cost / usage.monthly_limit >= 0.8
                {
                    60 // 1 minute when usage is high
                } else {
                    interval
                }
            };

            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
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
            // Create tray menu
            let show = MenuItemBuilder::with_id("show", "Dashboard").build(app)?;
            let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit BurnRate").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show)
                .item(&settings)
                .separator()
                .item(&quit)
                .build()?;

            // Build tray icon
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .title("⚡0% | 📅0% | 💰$0/$50")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" | "settings" => {
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
                            .inner_size(400.0, 520.0)
                            .resizable(false)
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
                            .inner_size(400.0, 520.0)
                            .resizable(false)
                            .build();
                        }
                    }
                })
                .build(app)?;

            // Start background polling
            start_polling(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage,
            get_config,
            save_config,
            load_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BurnRate");
}
