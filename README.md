# 🔥 BurnRate

[![GitHub release](https://img.shields.io/github/v/release/erados/burnrate)](https://github.com/erados/burnrate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon-black?logo=apple)](https://github.com/erados/burnrate/releases)

A macOS menu bar app that tracks your Claude Pro/Max usage in real-time — so you never get rate-limited by surprise.

<!-- TODO: Add demo GIF here
<p align="center">
  <img src="docs/demo.gif" alt="BurnRate Demo" width="600">
</p>
-->

<p align="center">
  <img src="docs/menubar.png" alt="BurnRate Menu Bar" width="400">
</p>

<p align="center">
  <img src="docs/dashboard.png" alt="BurnRate Dashboard" width="360">
</p>

## Why?

Claude Pro and Max have usage limits that reset on different schedules. You're mid-conversation, deep in thought — then suddenly rate-limited. BurnRate sits in your menu bar so you always know where you stand.

## Quick Start

1. Download `BurnRate_0.1.0_aarch64.dmg` from [Releases](https://github.com/erados/burnrate/releases)
2. Open the DMG and drag to Applications
3. Right-click → Open (not notarized yet)
4. Sign in to Claude when prompted
5. Done! Check your menu bar ⚡

## Features

- **Menu bar at a glance**: `⚡41% 3h01m | 🔋11%` — session usage, reset countdown, weekly usage
- **Dashboard**: Click to see detailed breakdown with visual progress bars
- **Session tracking**: Current session usage % with countdown to reset
- **Weekly tracking**: All models + Sonnet-specific usage
- **Extra usage**: Monthly spend vs limit (e.g. `$39.37 / $50.00`) — Max plan
- **Background polling**: Auto-updates every 60 seconds
- **Zero config**: Just log in to Claude once — no API keys needed
- **Lightweight**: Native macOS app via Tauri, minimal CPU/memory
- **Works with Claude Pro and Max** plans

## How It Works

BurnRate opens a hidden browser window, authenticates with your Claude account, and reads usage data from `claude.ai/settings/usage`. The window stays offscreen — you never see it.

1. Launch BurnRate → appears in menu bar
2. First launch: Claude login window appears → sign in once
3. Done! Usage data auto-refreshes in the background

## Menu Bar Format

```
⚡41% 3h01m | 🔋11%
│  │    │       │
│  │    │       └─ Weekly all-models usage
│  │    └───────── Time until session reset
│  └────────────── Session usage percentage
└───────────────── Session indicator

After 3 failed polls:
⚠️ Login required      ← Click tray → Login to Claude
```

## Dashboard

| Card | Shows |
|------|-------|
| **Session Limit** | Usage %, reset countdown, visual bar |
| **Weekly (All Models)** | Combined usage across all Claude models |
| **Weekly (Sonnet)** | Sonnet-specific usage tracking |
| **Extra Usage** | Monthly spend vs limit (e.g. $39.37 / $50.00) |

## Settings

Click tray icon → Dashboard → ⚙️ Settings:

- **Poll interval**: 30s / 1min / 2min / 5min
- **Login to Claude**: Opens visible login window
- **Logout**: Clears session, stops polling

## Build from Source

```bash
# Prerequisites: Rust, Node.js 18+
cd frontend
npm install
npx tauri build
```

Output: `frontend/src-tauri/target/release/bundle/macos/BurnRate.app`

## Privacy & Security

- **100% local** — no external servers, no telemetry
- **No API keys** — uses browser session cookie only
- **No stored passwords** — auth handled by Claude's own login
- Claude session lives in an isolated macOS WebView (WKWebView)
- ⚠️ This app scrapes claude.ai — technically against Anthropic's ToS. Use at your own risk.

## Tech Stack

- **[Tauri v2](https://tauri.app)** — Rust + native WebView, ~5MB bundle
- **[Svelte](https://svelte.dev)** — Dashboard UI
- **Rust** — Polling loop, tray management, scraping coordination
- **Custom URL scheme** — `burnrate://result/<base64>` for WebView → Rust IPC

## Known Issues

- Intel Mac build not yet available (ARM/Apple Silicon only)
- Login cookies may not persist after macOS restart (WKWebView limitation)
- Window briefly appears during initial login flow
- Tested on Claude Max. Pro users: please [open an issue](https://github.com/erados/burnrate/issues) if anything looks off!

## Contributing

Issues and PRs welcome! If you'd like to contribute:

1. Fork the repo
2. Create a feature branch
3. Submit a PR

## License

MIT

---

Built with ☕ in Brisbane, Australia
