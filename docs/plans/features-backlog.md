# Features backlog

Living backlog for product features, fixes, and cross-cutting work. Shipped items are recorded in [CHANGELOG.md](../../CHANGELOG.md); note the release version on the line when an item ships.

Not a feature spec — items with a linked `feature-*.md` keep the full design there; open items without a spec stay detailed in this file.

**Related plans:** [feature-overlay-content-tag-filters.md](feature-overlay-content-tag-filters.md) · [feature-voice-hud-accessibility.md](feature-voice-hud-accessibility.md) · [feature-appearance-theme.md](feature-appearance-theme.md) · [audit-hig.md](audit-hig.md) · [feature-quick-look-preview.md](feature-quick-look-preview.md)

---

## Security hardening

- [x] **Explicit Tauri capabilities for the** `settings` **window** — **0.3.0** — first scoped `settings.json` instead of a shared broad default.
      **0.4.0:** three-window ACL — `main.json`, `settings.json`, `voice_overlay.json` with explicit `commands.allow` via permission sets (`main-window-commands`, `settings-window-commands`, `voice-overlay-events`). Settings keeps config, exclusions, Ollama, accessibility, and history commands; it does **not** get `paste_entry` or `activate_entry`. Voice overlay is events-only. Details: [release-macos-intel-gate.md](release-macos-intel-gate.md) §8.

- [x] **Validate Ollama model names before** `ollama pull` — **0.3.0** — `ollama::validate_model_name` (trim, length ≤ 128, safe character set) before pull and settings persistence.
      **0.4.0:** no rule change; `pull_ollama_model` and related Ollama IPC stay on the settings capability set only (not main or voice overlay). Unit tests cover common names, whitespace trim, empty/too-long names, and rejection of shell-metacharacter injection.

- [x] `cargo audit` **in release workflow** — **0.4.0** — dependency audit step in GitHub Actions before release artifacts ship.

- [x] **Per-window IPC command scoping** — **0.4.0** — sensitive commands limited per window: `paste_entry` / `activate_entry` on `main` only; `clear_history`, `start_ollama_server`, exclusion editing on `settings` only; `voice_overlay` cannot invoke those. Complements the capabilities work above.

---

## Developer toolchain

- [x] **Lefthook pre-commit** — **0.4.0** — [Lefthook](https://github.com/evilmartians/lefthook) git hooks (`lefthook.yml`): parallel jobs per file type (JS/TS, Svelte, CSS, docs, Rust), piped auto-fix pipelines (Oxfmt → Oxlint/Stylelint), `stage_fixed` restaging, `skip_in_ci`. Full gate remains `make check` / CI.

---

## Features

- [x] **Infinite scroll** — **0.4.0** — lazy loading entries on horizontal scroll (`get_entries` with `limit` + `offset`; prefetch on scroll, backfill after local deletes, **Try again** on failed page loads)

- [x] **Overlay content & tag filters** — **0.4.0**
      Server-side tag and content-kind filtering in the clipboard overlay: format chips always, semantic chips when AI tagging is on; DB-wide chip counts; image card meta (dimensions, file size) instead of a generic label; filter chips visually distinct from card metadata tags; static panel height **415 / 450 px** (keyboard hints toggle); Content Kind row (All / Text / Images) temporarily hidden in UI. Spec: [feature-overlay-content-tag-filters.md](feature-overlay-content-tag-filters.md).

- [ ] **Shortcut recorder** (voice + future overlay shortcut)
  - Replace text inputs with a shortcut recorder control
  - Show symbols in the UI; persist a canonical string for Rust
  - States: idle / recording / invalid / conflict
  - `aria-label`: “Shortcut, click to record”; `aria-live="polite"` while recording
  - Keypress commits the shortcut without requiring Enter on Save (System Settings pattern)

- [ ] **Voice transcription improvements** — large HUD accessibility and transcription lifecycle pass; spec: [feature-voice-hud-accessibility.md](feature-voice-hud-accessibility.md)
  - Full screen-reader lifecycle: recording → processing → terminal (success / empty / error / not configured)
  - HUD stays visible during transcription; delayed hide after terminal announcement
  - Global announcer + phase state machine; no live-region spam from audio level
  - Rust `voice-a11y` events, seq, and permissions in capabilities

- [ ] **Custom collections**
  - “Name…” field appears when the user clicks **+** to the right of **Clipboard History** / **Starred** tabs — creates a new user-defined collection tab for grouping clipboard entries
  - Backend already supports assigning entries (`set_entry_collection`); finish the UI so cards can add/move entries (`setEntryCollection` is not wired today)
  - Today: create a collection, switch to it (filters by `collection_id`), delete it — but new collections stay empty until entries are assigned another way (e.g. DB directly)
  - Ship end-to-end grouping: assign/remove entries from cards (or equivalent UX) so collections are usable without manual data fixes

- [ ] **Quick Look preview on Space** — Finder-style full entry preview on selected card (`Space`); deferred from [audit-hig.md](audit-hig.md) item 14. Spec: [feature-quick-look-preview.md](feature-quick-look-preview.md)

- [ ] **Appearance — Light / Dark / Automatic** — native macOS theme switching with cool blue-gray light palette, Settings → Appearance segmented control (Light · Dark · Automatic), CSS token refactor (`--rgb-elevation-tint`, `data-theme`), persistence in `AppSettings`, live sync across overlay / voice HUD / settings. Deferred from [audit-hig.md](audit-hig.md) item 7. Spec: [feature-appearance-theme.md](feature-appearance-theme.md)

---

## Fixes

- [ ] **Production build transparency** — verify and fix on macOS 15+ (known Tauri issue [#13415](https://github.com/tauri-apps/tauri/issues/13415))
