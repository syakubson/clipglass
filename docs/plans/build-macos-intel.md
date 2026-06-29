# macOS Intel build and related improvements

Brief: why code and infrastructure were changed in this set of changes.  
Follow-up and pre-release refinements — in [release-macos-intel-gate.md](release-macos-intel-gate.md).

## Checklist

- [x] **`build-macos.sh`** — pipeline: frontend → Tauri bundle → DMG in `dist/macos/`
- [x] **`macos-target.sh`** — architecture via `MACOS_ARCH=auto | x86_64 | aarch64`
- [x] **Makefile** — `build-macos`, `build-macos-intel`, `build-macos-arm` and mirrored `release-macos-*`
- [x] **Named artifacts** — `Copyosity_0.3.0_x86_64.dmg` etc. in `dist/macos/`
- [x] **`tauri.unsigned.json`** — ad-hoc for local builds; release with Developer ID via `RELEASE_CONFIG=1`
- [x] **`release-macos.sh`** — same build pipeline as local build
- [x] **Makefile `APP_DIR`** — no hardcoded project path
- [x] **`env -u npm_config_devdir`** — stable `npm install` / Tauri build (incl. Cursor)
- [x] **`with-tauri.sh` / `env-rust.sh`** — `cargo` and `tauri` in PATH
- [x] **`.vscode/settings.json`** — same workaround for integrated terminal
- [x] **`.gitignore`** — `/dist` for build artifacts
- [x] **README** — Intel/ARM commands and `dist/macos/` path
- [x] **Frontend toolchain** — SvelteKit / Svelte / Vite update, `cookie` override
- [x] **Clipboard monitoring** — `changeCount`; order: files → raster → text; concealed; ignore Copyosity / excluded apps
- [x] **Image decoders** — jpeg, webp, gif, bmp, tiff via `image` crate for on-disk paths
- [x] **`CaptureContext` / `try_capture_from_clipboard`** — single pasteboard parsing entry point
- [x] **`clipboard_macos.rs`** — pasteboard API, synthetic Cmd+V, remember/restore paste target
- [x] **`clipboard_write.rs`** — `exclude_from_history`, mark as own entry
- [x] **copy vs activate** — split between buffer-only and paste into another app; Enter = `activateEntry`
- [x] **Accessibility** — `check_accessibility` + UI in Settings; Settings window to foreground (`objc2-app-kit`)
- [x] **objc → objc2 migration** — `cocoa` replaced where already in use
- [x] **Frontend** — Enter in feed = paste; Permissions in Settings; card copy/paste model unchanged
- [x] **Voice shortcut** — shared macOS pasteboard logic moved to `clipboard_macos`

---

## 1. Intel build (x86_64)

**Goal:** produce a reproducible `.app` and DMG for Intel Mac, in parallel with Apple Silicon, without tying to a single developer machine.

**What was done:**

- `scripts/build-macos.sh` — unified pipeline: frontend → Tauri bundle → DMG in `dist/macos/`.
- `scripts/macos-target.sh` — architecture via `MACOS_ARCH=auto | x86_64 | aarch64`.
- `Makefile`: `build-macos`, `build-macos-intel`, `build-macos-arm` and mirrored `release-macos-*`.
- Named artifacts in `dist/macos/` (e.g. `Copyosity_0.3.0_x86_64.dmg`).
- `tauri.unsigned.json` — ad-hoc signing for local builds; release with Developer ID via `RELEASE_CONFIG=1` in `release-macos.sh`.
- `release-macos.sh` uses the same build pipeline as local build.

**How to build:** `make build-macos-intel` (Intel) or `make build-macos` / `make build-macos-arm` on the matching machine.

---

## 2. Build infrastructure and dev environment

**Goal:** so Intel/ARM builds and `npm run tauri` work on any machine and in the IDE without manual path setup.

- `APP_DIR ?= $(CURDIR)` in Makefile — no hardcoded project path.
- `env -u npm_config_devdir` for npm — stable `npm install` / Tauri build (incl. Cursor).
- `scripts/with-tauri.sh`, `scripts/env-rust.sh` — `cargo` and `tauri` in PATH.
- `.vscode/settings.json` — same workaround for integrated terminal.
- `.gitignore`: `/dist` — build artifacts directory.
- `README.md` — Intel/ARM commands and `dist/macos/` path.
- SvelteKit / Svelte / Vite update, `cookie` override — current frontend toolchain on a clean clone.

---

## 3. macOS — clipboard and history

**Goal:** catch macOS copies more reliably, show images correctly, and avoid polluting history with the app's own actions.

### Monitoring

- `NSPasteboard.changeCount` — poll only when the clipboard actually changed.
- Read order: **files → raster → text** — when copying an image file in Finder, history gets file pixels, not the utility icon from the pasteboard.
- `image` crate: jpeg, webp, gif, bmp, tiff decoders for on-disk paths; screenshots and Copy Image still via raster API.
- Ignore concealed pasteboard (passwords and hidden content).
- Ignore Copyosity as source and apps from excluded list.
- `CaptureContext`, `try_capture_from_clipboard` modules — single clipboard content parsing entry point.

### Write, copy, and paste

- `clipboard_macos.rs` — pasteboard API, `changeCount`, concealed, synthetic Cmd+V, remember and restore target app before paste.
- `clipboard_write.rs` — write to clipboard with `exclude_from_history` and own-entry mark so card copy doesn't duplicate history.
- `remember_paste_target` / `restore_paste_target` — double-click / Enter paste into the app from which the panel was opened.
- `copy_entry` / `activate_entry` — split between buffer-only and paste into another app.
- Enter in main window — `activateEntry` for text and images.
- `check_accessibility` + UI in Settings — permissions for automatic paste and hotkeys.
- Settings window — correct bring-to-front (`objc2-app-kit`).
- Dependency `cocoa` replaced with `objc2` / `objc2-app-kit` where already in use.

---

## 4. Frontend

- Main feed: Enter = paste selected entry (`activateEntry`) and close panel.
- Settings: Permissions block (Accessibility), hint about re-adding the app in Privacy after a new build.
- Card: single click — copy, double click — paste (this model unchanged).

---

## 5. Voice shortcut

- Transcription still puts text in the clipboard and simulates Cmd+V; shared macOS pasteboard logic moved to `clipboard_macos`.

---

## 6. Affected repository areas

| Area          | Files                                                                                                                                                          |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Build         | `Makefile`, `README.md`, `scripts/build-macos.sh`, `macos-target.sh`, `env-rust.sh`, `with-tauri.sh`, `with-npm.sh`, `release-macos.sh`, `tauri.unsigned.json` |
| Config / deps | `.gitignore`, `.vscode/settings.json`, `package.json`, `package-lock.json`                                                                                     |
| Rust backend  | `clipboard_monitor.rs`, `clipboard_macos.rs`, `clipboard_write.rs`, `commands.rs`, `lib.rs`, `Cargo.toml`                                                      |
| UI            | `+page.svelte`, `settings/+page.svelte`, `ClipboardCard.svelte`                                                                                                |
