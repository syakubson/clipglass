# SF Symbols iconography

Copyosity is **macOS-only**. UI icons use **Apple SF Symbols**, baked into SVG paths for the Svelte/WebView frontend. Do not add inline custom stroke SVG for toolbar, settings, or overlay controls.

**Agent checklist — add a new icon**

1. Confirm the symbol exists in the [SF Symbols](https://developer.apple.com/sf-symbols/) app (exact name, e.g. `gearshape` not `gear`).
2. Add the name string to the `symbols` array in `scripts/export-sf-symbols.swift`.
3. On macOS: `make export-sf-symbols` (rewrites `src/lib/sf-symbols/registry.ts`).
4. Use in UI: `<SfSymbol name="…" class="…" />` with an existing semantic class from `src/lib/styles/sf-symbol.css`, or add a token + class if it is a new UI block.
5. Run `make fix-frontend && make check-frontend`.
6. Commit **both** the Swift list change and the regenerated `registry.ts`.

---

## Why not inline SVG?

| Approach                                       | Used here?                                                                                                 |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| SF Symbols via `SfSymbol` + generated registry | **Yes** — default for all interactive UI                                                                   |
| Inline Feather/Lucide/custom stroke SVG        | **No** — removed (HIG audit item 15)                                                                       |
| Native `<select>` chevron                      | CSS data URI from `chevron.down` in `tokens.css` (auto-synced on export; fill from `--color-text-primary`) |

The WebView cannot call `Image(systemName:)` at runtime. Paths are exported once on macOS and shipped in `registry.ts`.

---

## File map

| Path                                  | Role                                                    |
| ------------------------------------- | ------------------------------------------------------- |
| `src/lib/components/SfSymbol.svelte`  | Renders `<svg>` + path from registry                    |
| `src/lib/sf-symbols/registry.ts`      | **Generated** — `SF_SYMBOL_PATHS`, `SfSymbolName` union |
| `src/lib/sf-symbols/section-icons.ts` | Maps settings section keys → symbol names               |
| `src/lib/sf-symbols/index.ts`         | Re-exports                                              |
| `src/lib/styles/sf-symbol.css`        | Semantic size classes (width/height from tokens)        |
| `src/lib/styles/tokens.css`           | `--icon-size-*` tokens                                  |
| `scripts/export-sf-symbols.swift`     | macOS export script (symbol list + optical cap heights) |

`sf-symbol.css` is imported globally from `src/routes/+layout.svelte`.

---

## Using an icon in a component

```svelte
<script lang="ts">
  import SfSymbol from "$lib/components/SfSymbol.svelte";
</script>

<button type="button" aria-label="Open settings">
  <SfSymbol name="gearshape" class="overlay-header-settings-icon" />
</button>
```

**Rules**

- `name` must be a `SfSymbolName` (TypeScript catches typos).
- Set size via a **semantic CSS class**, not inline `width`/`height` on the component.
- Decorative icons: omit `title` (default `aria-hidden="true"`). Labeled controls: put `aria-label` on the **button**, not on the SVG.
- Color: `color` / `fill: currentcolor` on the parent; symbols use `fill="currentColor"`.

**Settings section titles** — use `SectionIcon`, not raw `SfSymbol`:

```svelte
<SectionIcon name="voice" />
```

Add new section keys in `section-icons.ts` only.

---

## Size tokens and classes

Display sizes live in `tokens.css`. Classes in `sf-symbol.css` map one token per UI block.

| Token                                 | Class                                        | Typical use                               |
| ------------------------------------- | -------------------------------------------- | ----------------------------------------- |
| `--icon-size-overlay-header-settings` | `.overlay-header-settings-icon`              | Overlay gear (16px)                       |
| `--icon-size-overlay-header-close`    | `.overlay-header-close-icon`                 | Overlay close (13px)                      |
| `--icon-size-search`                  | `.search-icon`                               | Search magnifying glass                   |
| `--icon-size-search-clear`            | `.search-clear-icon`                         | Search clear `xmark`                      |
| `--icon-size-card-action`             | `.card-action-icon`                          | Card toolbar (paste, retag, star, delete) |
| `--icon-size-card-copied`             | `.card-copied-icon`                          | “Copied” checkmark overlay                |
| `--icon-size-section`                 | `.form-title-icon` / `.form-subsection-icon` | Settings section headers (16px)           |
| `--icon-size-form-btn`                | `.form-btn-icon`                             | Settings primary buttons                  |
| `--icon-size-form-inline`             | `.excluded-list-action-icon`                 | Inline ± in lists                         |
| `--icon-size-chevron`                 | `.chevron-down`                              | Action menu trigger                       |
| `--icon-size-chip`                    | `.format-icon`                               | Tag filter format chips                   |
| `--icon-size-collection-dot`          | (on `.tab-dot` in `CollectionTabs`)          | Collection color dot (7px)                |
| `--icon-size-collection-action`       | `.collection-action-icon`                    | Collection × and +                        |
| `--icon-size-voice-mic`               | `.voice-mic-icon`                            | Voice HUD                                 |

**When to add a new token:** only for a **new semantic block** (e.g. a new toolbar). Otherwise reuse the closest class. Tune size in `tokens.css`, not per component.

Filled glyphs (`xmark`) look heavier than outline glyphs (`gearshape`) at the same pixel box — sizes were tuned optically; match nearby controls when adding icons.

---

## Add a new SF Symbol (detailed)

### 1. Pick the system name

Use the exact SF Symbols name (`.fill` variants are separate entries). Verify in the SF Symbols app at **Regular** weight, 16pt.

### 2. Register in the export script

Edit `scripts/export-sf-symbols.swift` — append to the `symbols` array (keep alphabetical or grouped; order defines registry output).

If the glyph looks too heavy or too light next to neighbors, add a `case` in `targetCapHeight(for:size:)` (see existing `xmark` / `plus` overrides).

### 3. Regenerate the registry (macOS only)

```bash
make export-sf-symbols
```

Requires Xcode/macOS. Writes `src/lib/sf-symbols/registry.ts` (listed in `.oxfmtrc.json` `ignorePatterns` — do not hand-format). Sets `SF_SYMBOL_EXPORT_MACOS_MAJOR` (must match CI `macos-15`). **Never edit `registry.ts` by hand.**

CI job **`sf-symbols`** (and release job of the same name) run `make export-sf-symbols && git diff --exit-code` on **`macos-15`** so path geometry stays stable. Other CI jobs use `macos-latest`.

Non-macOS developers: commit the regenerated file from a Mac machine or CI; `npm run check` uses the committed registry.

### 4. Wire the UI

- Import `SfSymbol`.
- Pick `name` from the updated union.
- Apply the correct semantic class from `sf-symbol.css`.

### 5. Validate

```bash
make fix-frontend && make check-frontend
```

Visually check the icon beside its neighbors (filled vs outline weight).

---

## Add a settings section icon

1. Add a key to `SectionIconName` and `SECTION_ICON_SYMBOLS` in `src/lib/sf-symbols/section-icons.ts`.
2. If the SF Symbol is new, follow **Add a new SF Symbol** above first.
3. Use `<SectionIcon name="your-key" />` in `settings/+page.svelte`.

---

## Exceptions

### Native `<select>` (`.form-select`)

Cannot render `SfSymbol`. Chevron uses `--icon-chevron-down` in `tokens.css` — a data URI built from the exported `chevron.down` path. Fill hex is read from `--color-text-primary` at export time (`--icon-chevron-select-fill` aliases that token for documentation). Opacity 85% — WebKit cannot use `currentColor` in `background-image` SVG on `<select>`. `make export-sf-symbols` rewrites both `registry.ts` and that token line automatically.

### Collection color dot

Not an SF Symbol — CSS circle using `--icon-size-collection-dot`.

---

## Do not

- Add inline `<svg>` icons for product UI (overlay, settings, cards, search).
- Edit `registry.ts` manually.
- Set arbitrary per-component pixel sizes on `SfSymbol` — use tokens.
- Use SF Symbol names that are not in the registry (compile error).
- Assume Linux/Windows can run the export script.

---

## Platform note

Copyosity targets **macOS only** (NSPanel, paste pipeline, bundle IDs). SF Symbols fit that scope. A future Windows port would need a separate icon strategy; do not revert to generic SVG “for portability” unless the product direction changes.

---

## Related

- HIG audit: [docs/plans/audit-hig.md](../plans/audit-hig.md) §15
- Paste pipeline (separate concern): [macos-paste-pipeline.md](macos-paste-pipeline.md)
