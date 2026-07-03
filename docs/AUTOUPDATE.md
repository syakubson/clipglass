# Auto-update — how it works & how to release

Clipglass uses `tauri-plugin-updater`. On launch (and via **Settings → Updates**)
the app fetches a manifest from GitHub Releases, and if a newer, **signed**
version exists it downloads and installs it, then relaunches.

- **Update source:** `https://github.com/syakubson/clipglass/releases/latest/download/latest.json`
  (configured in `src-tauri/tauri.conf.json → plugins.updater.endpoints`).
- **Signature:** every update artifact is signed with the updater private key;
  the app verifies it against the public key in `tauri.conf.json → plugins.updater.pubkey`.
  This is **separate** from Apple Developer ID / notarization.

## Keys (one-time, already done)

- Private key: `.tauri/clipglass-updater.key` — **git-ignored, back it up somewhere safe.**
  If lost, you can never ship another auto-update (users must reinstall manually).
- Public key: `.tauri/clipglass-updater.key.pub` — embedded in `tauri.conf.json`.
- For CI, add the private key contents as the repo secret **`TAURI_SIGNING_PRIVATE_KEY`**
  (the key has an empty password). The parked `release.yml.disabled` workflow
  references it; re-enable and adapt it when the signing-free release pipeline lands.

> Auto-update works from Clipglass's first release onward (the updater plugin
> ships in every version).

## Releasing an update (macOS, local)

```bash
# 1. bump version in package.json + src-tauri/tauri.conf.json, commit
# 2. build WITH the signing key so .app.tar.gz(.sig) are produced
export TAURI_SIGNING_PRIVATE_KEY="$(cat .tauri/clipglass-updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri build            # DMG bundling may fail (known) — the .app + .app.tar.gz are still made

# 3. (optional) notarize the .app + DMG with your own Apple Developer ID
# 4. generate the manifest
./scripts/make-latest-json.sh "Release notes here"

# 5. publish: attach the DMG, the *.app.tar.gz, AND latest.json to the release
gh release create vX.Y.Z \
  src-tauri/target/release/bundle/dmg/*.dmg \
  src-tauri/target/release/bundle/macos/*.app.tar.gz \
  latest.json \
  --title "Clipglass X.Y.Z" --notes "..."
```

`latest.json` must be a release asset so the `latest/download/latest.json` URL
resolves. Clipglass is macOS-only for now, so `latest.json` only needs a
`darwin-*` entry.
