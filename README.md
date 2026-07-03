# Clipglass

A clipboard assistant for macOS: searchable clipboard history, pinned items and
collections, image support with on-device OCR, and voice input — local-first,
with optional AI integrations you configure yourself.

Based on [copyosity](https://github.com/vakovalskii/copyosity) by
[@vakovalskii](https://github.com/vakovalskii) (MIT). Clipglass is an
independent fork with its own identity, update channel, and roadmap.

## Requirements

- macOS 12+ (Apple Silicon or Intel)

## Install

1. Download the DMG for your architecture from
   [Releases](https://github.com/syakubson/clipglass/releases).
2. Drag **Clipglass** to Applications.
3. The app is not notarized (no Apple Developer subscription). On first launch
   macOS will warn you. Either right-click the app → **Open** → **Open**, or run:

   ```sh
   xattr -cr /Applications/Clipglass.app
   ```

4. Grant Accessibility and Microphone permissions when prompted.

Clipglass updates itself from GitHub Releases (updates are signed and verified
by the built-in updater).

## Development

```sh
git clone https://github.com/syakubson/clipglass.git
cd clipglass
npm install
make dev        # run the app in dev mode
make check      # full gate: frontend typecheck/tests/lint + Rust build/clippy/tests
```

## Privacy

- Clipboard history is stored locally in
  `~/Library/Application Support/com.syakubson.clipglass/`.
- Image OCR runs on-device (Apple Vision). Nothing leaves your Mac unless you
  explicitly configure an AI provider.
- Sensitive apps can be excluded from history in Settings.

## License

MIT — see [LICENSE](LICENSE). Original work © Valeriy Kovalsky, fork
© syakubson.
