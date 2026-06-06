# Changelog

All notable changes to Copyosity are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Unit test coverage** — `validate_model_name` (Ollama model security gate), `should_ignore_capture` (skip own clipboard writes), and clipboard monitor helpers (`hash_bytes`, `hash_raster_image`, `is_image_path`, `should_skip_source`); 11 new tests (53 total).

## [0.4.0] - 2026-06-06

### Added

- **macOS Intel (x86_64) builds** — `make build-macos-intel`, `make release-macos-intel`, and arch-specific DMG names in `dist/macos/` (for example `Copyosity_0.4.0_x86_64.dmg`).
- **Unified clipboard write module** (`clipboard_write.rs`) with explicit **Copy** and **Paste** modes; copy, activate, paste, and voice flows share one code path.
- **Per-window Tauri capabilities** — separate permission sets for `main`, `settings`, and `voice_overlay` instead of a single default capability.
- **Ollama model name validation** before `ollama pull` (allowlist/regex) to block malformed or unsafe model strings.
- **GitHub Actions release workflow** — `cargo audit`, `npm run check`, and `cargo test` on tagged releases.
- **Build tooling** — `scripts/build-macos.sh`, `macos-target.sh`, `env-rust.sh`, `with-npm.sh`, `with-tauri.sh`; Makefile targets for Intel, Apple Silicon, and native arch.

### Changed

- **Clipboard history deduplication** — `content_hash` is based on content only (no `changeCount` suffix); repeated copies of the same text/image no longer create duplicate entries.
- **Clipboard monitor** — 300 ms poll interval; `changeCount` is a change trigger only, not part of the content hash.
- **Paste pipeline** — shared `paste_text_into_target`; Enter in the main UI uses `activateEntry` (same path as double-click paste).
- **Accessibility UX** — `check_accessibility({ prompt })` separates silent status checks from the macOS trust dialog; prompt on first Settings visit per window session, on **Request**, and when paste is attempted without rights; no re-prompt loop after Deny or **Open System Settings** while Settings stays open.
- **macOS release builds** — `release-macos.sh` reuses `build-macos.sh` (arch-aware paths, `RELEASE_CONFIG`, shared DMG packaging) instead of a duplicate inline build.
- **macOS clipboard layer** — migrated `clipboard_macos.rs` and Accessibility checks from legacy `objc` to **objc2**.
- **Makefile** — portable `APP_DIR` (`CURDIR`), `make check` runs `cargo test`, arch-specific release targets.

### Fixed

- **Voice overlay** — no longer steals focus from the target app; audio level meter displays correctly.
- **Image capture** — copies and stores the full image instead of a thumbnail; backfill for images already in history.
- **Finder file paths** — improved handling of image files copied from Finder.
- **Database tests** — `update_settings` partial updates no longer wipe Whisper/voice/mic fields.
- **Settings** — clearer Accessibility guidance after rebuild/reinstall when double-click paste stops working.
- **Accessibility status** — live AX API probe instead of cached `AXIsProcessTrusted` / handle-only checks; UI updates correctly after rights are revoked in System Settings.
- **Settings accessibility UI** — green “verified” message clears when access is no longer granted; **Recheck** gives explicit success feedback when access is still valid.
- **Settings window** — draggable again (native title bar restored; header drag region retained).

### Security

- Sensitive IPC commands scoped per window (`settings` cannot call paste commands; `voice_overlay` cannot call `clear_history` or `start_ollama_server`).
- `cargo audit` runs in the release CI pipeline.

### Performance

- Single `get_frontmost_app()` call per `file_list` batch in the clipboard monitor.
- ~20 MB size limit for image file encoding.

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
