# Clipglass — agent notes

macOS-native clipboard assistant. Tauri 2 (Rust backend) + Svelte 5 (SvelteKit
static). macOS only — NSPanel overlay, CGEvent paste, Apple Vision OCR.

## Commands

- `make dev` — run in dev mode
- `make check` — the full gate (frontend typecheck/tests/lint + Rust
  compile/clippy/fmt/tests). Must be green before any commit.
- Pre-commit hooks: lefthook (oxlint/oxfmt/stylelint auto-fix on staged files).

## Conventions

- Conventional commits.
- Do not add cloud calls or telemetry: local-first; AI features are opt-in and
  user-configured.
- Rust: keep modules focused (one concern per file, as in the existing layout).

## Local AI Onboarding

When working on Ollama onboarding in the app, follow this product rule set:

1. If Ollama is not installed, do not silently install it.
2. Show a clear onboarding state with a download action and short instructions.
3. If Ollama is installed but not running, show that state separately and offer a start/check-again action.
4. If Ollama is installed but the selected model is missing, the app may offer to download the model directly.
5. If both Ollama and the model are ready, show a clear ready state.

Expected user-facing states:

- `Ollama not installed`
- `Ollama installed, server not running`
- `Model not installed`
- `Local AI ready`

Expected actions:

- `Download Ollama`
- `Start Ollama`
- `Download model`
- `Check again`
- `Change model`

Product policy:

- System-level Ollama installation should be explicit and user-approved.
- Model downloads may be initiated from inside the app once Ollama is present.
- The UI should always explain what is missing: runtime, server, or model.
