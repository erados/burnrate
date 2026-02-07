# 🔥 BurnRate

macOS menu bar app that monitors your Anthropic API usage and costs in real-time.

## Features

- **Menu bar gauge**: `⚡7% | 📅1% | 💰$39/$50` at a glance
- **Dashboard**: Click to see detailed progress bars for session, weekly, and monthly usage
- **Smart alerts**: Desktop notifications when approaching limits
- **Secure**: API keys stored in macOS Keychain
- **Lightweight**: ~10MB memory, adaptive polling (5min → 1min when usage spikes)

## Usage Layers

| Layer | What | Resets |
|-------|------|--------|
| ⚡ Session | Current session usage % | Every few hours |
| 📅 Weekly | All models / Sonnet usage % | Saturday |
| 💰 Monthly | Dollar cost vs budget | Monthly |

## Development

```bash
cd frontend
npm install
npm run tauri dev
```

## Build

```bash
cd frontend
npm run tauri build
```

## Stack

- **Tauri v2** - Native macOS app with minimal footprint
- **Svelte** - Reactive UI for the dashboard
- **Rust** - Backend polling, keychain integration, tray management

## Configuration

1. Click the tray icon → Settings
2. Enter your Anthropic API key and Organization ID
3. Adjust polling interval and alert thresholds
4. API key is stored securely in macOS Keychain

## License

MIT
