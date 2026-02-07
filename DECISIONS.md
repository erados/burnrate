# Decisions Log

## D1: Project structure - Single frontend/ directory
Using a single `frontend/` directory that contains both the Svelte app and the `src-tauri/` Rust backend. This is standard Tauri v2 layout.

## D2: Mock data when no API key
When no API key is configured, the app shows mock/simulated data so you can see the UI immediately. Real data kicks in once you add your API key in settings.

## D3: Session usage estimation
The Anthropic Admin API doesn't directly expose "session" usage. We estimate it by scanning `~/.claude/` for recently modified files and counting approximate tokens. This is a rough heuristic — may need refinement based on actual Claude Code log format.

## D4: Keychain for API key storage
Using macOS Keychain via the `keyring` crate. This is the most secure option for local API key storage on macOS.

## D5: Weekly reset on Saturday
Per the spec, weekly limits reset on Saturday. We calculate hours until next Saturday for the countdown.

## D6: Adaptive polling
Default 5min polling. When session usage ≥70% or monthly cost ≥80% of limit, polling increases to 1min automatically.

## D7: No window on launch
App starts as a tray-only app. Clicking the tray icon or "Dashboard" menu item opens the window. This is the expected behavior for a menu bar utility.

## Needs User Input
- **Anthropic Admin API access**: Need to verify the exact API endpoints and response format. The current implementation uses `/v1/organizations/{id}/usage` which may need adjustment.
- **Session limit details**: How exactly does Claude define a "session"? Current implementation uses file modification time heuristic.
