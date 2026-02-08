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
    pub today_messages: u64,
    pub today_tool_calls: u64,
    pub today_sessions: u64,
    pub today_tokens: u64,
    pub opus_tokens: u64,
    pub sonnet_tokens: u64,
    pub weekly_daily: Vec<u64>,  // 7 elements, oldest first
    pub weekly_messages: u64,
    pub usage_percent: f64,      // estimated 5h window usage
    pub last5h_tokens: u64,
    pub last_active_date: String,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            today_messages: 0,
            today_tool_calls: 0,
            today_sessions: 0,
            today_tokens: 0,
            opus_tokens: 0,
            sonnet_tokens: 0,
            weekly_daily: vec![0; 7],
            weekly_messages: 0,
            usage_percent: 0.0,
            last5h_tokens: 0,
            last_active_date: String::new(),
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
            poll_interval_secs: 30,
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
    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}

fn format_tray_title(usage: &UsageData) -> String {
    let tok_k = usage.today_tokens / 1000;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if !usage.last_active_date.is_empty() && usage.last_active_date != today {
        let short = format_short_date(&usage.last_active_date);
        format!("🔥 {}msg | {}k tok ({})", usage.today_messages, tok_k, short)
    } else {
        format!("🔥 {}msg | {}k tok", usage.today_messages, tok_k)
    }
}

fn format_short_date(date_str: &str) -> String {
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        date.format("%b %-d").to_string()
    } else {
        date_str.to_string()
    }
}

fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let interval = {
                let state = app.state::<AppState>();
                let interval = state.config.lock().unwrap().poll_interval_secs;
                interval
            };

            // Read local Claude data
            let data = usage::read_local_usage();

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
            let quit = MenuItemBuilder::with_id("quit", "Quit BurnRate").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show)
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
                            .inner_size(420.0, 400.0)
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
                            .inner_size(420.0, 400.0)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running BurnRate");
}
