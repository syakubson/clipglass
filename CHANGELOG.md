# Changelog

All notable changes to Copyosity are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Frontend dependencies** — `@sveltejs/kit` 2.63, `svelte` 5.56.2, `svelte-check` 4.6.
- **Tauri stack** — synced npm (`@tauri-apps/api` 2.11, `@tauri-apps/cli` 2.11) and Rust (`tauri` 2.11) versions; `tauri-plugin-opener` 2.5.4, `global-shortcut` 2.3.2, `sql` 2.4.0.

### Security

- **Tauri 2.11** — upstream IPC ACL hardening: custom commands from remote origins are subject to capability checks even without an `AppManifest`.

## [0.4.0] - 2026-06-06

### Added

- **macOS Intel (x86_64) builds** — `make build-macos-intel`, `make release-macos-intel`, and arch-specific DMG names in `dist/macos/` (for example `Copyosity_0.4.0_x86_64.dmg`).
- **Build tooling** — `scripts/build-macos.sh`, `macos-target.sh`, `env-rust.sh`, `with-npm.sh`, `with-tauri.sh`; Makefile targets for Intel, Apple Silicon, and native arch.
- **macOS clipboard integration** (`clipboard_macos.rs`) — `NSPasteboard.changeCount`, concealed-pasteboard check, paste-target remember/restore, synthetic Cmd+V, and live Accessibility probing via **objc2** (replaces legacy `objc`/`cocoa`).
- **Unified clipboard writes** (`clipboard_write.rs`) with explicit **Copy** and **Paste** modes; copy, activate, paste, and voice flows share one code path.
- **Clipboard monitor** — capture image files copied from Finder (png, jpg, jpeg, gif) with a ~20 MB size cap; ignore the app's own pasteboard writes.
- **Per-window Tauri capabilities** — separate permission sets for `main`, `settings`, and `voice_overlay` instead of a single default capability.
- **Ollama model name validation** before `ollama pull` to block malformed or unsafe model strings.
- **GitHub Actions release workflow** — `cargo audit`, `npm run check`, and `cargo test` on tagged releases.
- **Unit tests** — `validate_model_name`, `should_ignore_capture`, clipboard monitor helpers, and settings partial-update coverage; 42 → 53 tests.
- **README** — Apple Silicon vs Intel install table and dual-architecture DMG guidance.

### Changed

- **Clipboard monitor (macOS)** — consult `changeCount` before reading the pasteboard; skip capture when the content hash is unchanged even if `changeCount` increased.
- **History UI** — `clipboard-changed` is emitted only for genuinely new entries (re-copying identical content no longer re-triggers the UI).
- **Paste pipeline** — shared `paste_text_into_target` with paste-target restore; Enter in the main UI calls `activateEntry` (same path as double-click) for text and images.
- **Accessibility UX in Settings** — `check_accessibility({ prompt })` separates silent checks from the macOS trust dialog; one prompt per Settings visit, plus **Request** and paste-attempt flows; no re-prompt loop after Deny or **Open System Settings** while Settings stays open.
- **Settings window** — native title bar instead of overlay style (draggable again); custom header drag region retained.
- **Voice transcription** — uses the shared paste write path and restores the paste target before Cmd+V.
- **macOS release builds** — `release-macos.sh` delegates to `build-macos.sh` instead of duplicating inline build logic.
- **Makefile** — portable `APP_DIR` (`CURDIR`); `make check` runs `cargo test`.

### Fixed

- **Voice overlay** — pre-created NSPanel with non-activating behavior so showing the overlay no longer steals focus from the target app.
- **Voice overlay** — audio level meter uses a logarithmic dB scale for quiet laptop mics.
- **Image history** — backfill `image_data` on duplicate entries that were stored before full-size image support.
- **Finder images** — image files copied from Finder are captured and stored correctly.
- **Database tests** — `update_settings` partial updates no longer wipe Whisper/voice/mic fields.
- **Accessibility status** — live AX probe; UI clears stale “verified” state when rights are revoked; **Recheck** confirms when access is still valid.
- **Settings** — clearer Accessibility guidance after rebuild/reinstall when double-click paste stops working.

### Security

- Sensitive IPC commands scoped per window (`settings` cannot call paste commands; `voice_overlay` cannot call `clear_history` or `start_ollama_server`).
- `cargo audit` runs in the release CI pipeline.

## [0.3.0] - 2026-04-10

### Added

- **Voice transcription** — dictate from the voice overlay into the active app.
- **Model pull progress** — non-blocking Ollama download via REST API with live progress in Settings.
- **Accessibility permission check** in Settings with explicit request action.
- Panel visible over fullscreen apps on all Spaces.

### Changed

- Wider Settings window; auto-save model before tagging test; spinner and loading hints in Settings.
- Ollama lookup searches common install paths when running from a `.app` bundle.

### Fixed

- Image copy used thumbnail instead of original resolution.
- Model presets, pull error handling, and unload-model button behavior.
- Tagging test timeout (60 s for cold model load); status refresh after save.
- Quit button uses `std::process::exit` to bypass `prevent_exit`.
- Settings window opener permission via Tauri capabilities.

## [0.2.1]

### Fixed

- Paste reliability, copy button behavior, scroll position reset, and ghost windows after hide.

## [0.2.0]

### Added

- NSPanel-based main window (no focus stealing).
- Dedicated Settings window.
- Security hardening and updated app icons.
