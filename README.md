# 🔥 BurnRate

A macOS menu bar app that tracks your Claude Pro usage in real-time — session limits, weekly limits, and extra usage costs at a glance.

![BurnRate Menu Bar](docs/menubar-preview.png)

## Why?

Claude Pro has usage limits that reset on different schedules. BurnRate sits in your menu bar and shows you exactly where you stand — no more guessing if you're about to hit a wall mid-conversation.

## Features

- **Menu bar at a glance**: `⚡27% 3h12m | 🔋7%` — session usage + reset time + weekly usage
- **Dashboard**: Click to see detailed breakdown with progress bars
- **Auto-scraping**: Logs into claude.ai and reads your usage page automatically
- **Session tracking**: Current session usage % with reset countdown
- **Weekly tracking**: All models + Sonnet-only usage
- **Extra usage**: Tracks additional charges beyond Pro plan limits
- **Background polling**: Updates every 60 seconds (configurable)
- **Lightweight**: Native macOS app, minimal resource usage

## Screenshots

| Menu Bar | Dashboard |
|----------|-----------|
| ![menubar](docs/menubar.png) | ![dashboard](docs/dashboard.png) |

## How It Works

BurnRate opens a hidden browser window, logs into your Claude account, and scrapes the usage data from `claude.ai/settings/usage`. No API keys needed — just log in once and it handles the rest.

## Install

### Download
Grab the latest `.dmg` from [Releases](https://github.com/woodiesong/burnrate/releases).

### Build from source
```bash
# Prerequisites: Rust, Node.js
cd frontend
npm install
npx tauri build
```

The built app will be in `frontend/src-tauri/target/release/bundle/macos/BurnRate.app`

## Usage

1. Launch BurnRate — it appears in your menu bar
2. Click the tray icon → **Login to Claude**
3. Sign in with your Claude account
4. Done! Usage data updates automatically

## Menu Bar Format

```
⚡27% 3h12m | 🔋7%
│  │    │       │
│  │    │       └─ Weekly all-models usage
│  │    └───────── Session reset countdown
│  └────────────── Session usage percentage
└───────────────── Session indicator
```

## Stack

- **[Tauri v2](https://tauri.app)** — Native macOS app with minimal footprint
- **[Svelte](https://svelte.dev)** — Reactive dashboard UI
- **Rust** — Backend: polling, tray management, web scraping coordination

## Configuration

Click the tray icon → **Dashboard** → **⚙️ Settings**:
- **Poll interval**: 30s / 1min / 2min / 5min
- **Login/logout**: Manage your Claude connection

## Privacy

- BurnRate runs entirely on your machine
- No data is sent to any external server
- Your Claude session stays in the app's local webview
- No API keys or passwords are stored

## License

MIT

---

Built with ☕ in Brisbane, Australia
