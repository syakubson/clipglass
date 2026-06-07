APP_DIR ?= $(CURDIR)
export PATH := $(HOME)/.cargo/bin:$(PATH)
NPM := env -u npm_config_devdir npm
OLLAMA_MODEL ?= qwen3:4b-instruct-2507-q4_K_M
OLLAMA_DEBUG ?= 1

.PHONY: dev build check install clean-cache clean-cache-aggressive clean-all \
	build-macos build-macos-intel build-macos-arm \
	release-macos release-macos-intel release-macos-arm notarize-wait notarize-info

dev:
	cd $(APP_DIR) && COPYOSITY_OLLAMA_MODEL='$(OLLAMA_MODEL)' COPYOSITY_DEBUG_OLLAMA='$(OLLAMA_DEBUG)' $(NPM) run tauri dev

build:
	cd $(APP_DIR) && COPYOSITY_OLLAMA_MODEL='$(OLLAMA_MODEL)' COPYOSITY_DEBUG_OLLAMA='$(OLLAMA_DEBUG)' $(NPM) run tauri build

check:
	cd $(APP_DIR) && $(NPM) run check && cd src-tauri && cargo check && cargo test

install:
	cd $(APP_DIR) && $(NPM) install

# Drop release artifacts and incremental cache; keep debug/deps for faster rebuilds.
clean-cache:
	@echo "[clean-cache] release builds, incremental cache, frontend output (debug deps kept)"
	cd $(APP_DIR)/src-tauri && cargo clean --release
	find $(APP_DIR)/src-tauri/target -type d -name incremental -exec rm -rf {} + 2>/dev/null || true
	rm -rf $(APP_DIR)/dist $(APP_DIR)/.svelte-kit $(APP_DIR)/build $(APP_DIR)/src-tauri/bundle
	@echo "[clean-cache] done"

# Like clean-cache, plus build-script cache and this app's crate artifacts (~2–3 GB more).
# Third-party rlibs in debug/deps stay — next cargo check relinks the app, not all deps.
clean-cache-aggressive: clean-cache
	@echo "[clean-cache-aggressive] build-script cache + copyosity crate artifacts (third-party deps kept)"
	find $(APP_DIR)/src-tauri/target -type d -path '*/debug/build' -exec rm -rf {} + 2>/dev/null || true
	cd $(APP_DIR)/src-tauri && cargo clean -p copyosity
	@echo "[clean-cache-aggressive] done"

# Remove all generated artifacts; next build starts from scratch.
clean-all:
	@echo "[clean-all] target, node_modules, frontend cache, bundles"
	cd $(APP_DIR)/src-tauri && cargo clean
	rm -rf $(APP_DIR)/node_modules $(APP_DIR)/dist $(APP_DIR)/.svelte-kit $(APP_DIR)/build
	rm -rf $(APP_DIR)/src-tauri/bundle $(APP_DIR)/.tauri
	rm -f $(APP_DIR)/*.dmg
	@echo "[clean-all] done — run 'make install' before dev/build if node_modules was removed"

build-macos:
	cd $(APP_DIR) && MACOS_ARCH=auto ./scripts/build-macos.sh

build-macos-intel:
	cd $(APP_DIR) && MACOS_ARCH=x86_64 ./scripts/build-macos.sh

build-macos-arm:
	cd $(APP_DIR) && MACOS_ARCH=aarch64 ./scripts/build-macos.sh

release-macos:
	cd $(APP_DIR) && MACOS_ARCH=auto KEYCHAIN_PROFILE='AC_PASSWORD' WAIT_FOR_NOTARIZATION=0 ./scripts/release-macos.sh

release-macos-intel:
	cd $(APP_DIR) && MACOS_ARCH=x86_64 KEYCHAIN_PROFILE='AC_PASSWORD' WAIT_FOR_NOTARIZATION=0 ./scripts/release-macos.sh

release-macos-arm:
	cd $(APP_DIR) && MACOS_ARCH=aarch64 KEYCHAIN_PROFILE='AC_PASSWORD' WAIT_FOR_NOTARIZATION=0 ./scripts/release-macos.sh

notarize-info:
	cd $(APP_DIR) && xcrun notarytool info "$$(cat .last_notarization_id)" --keychain-profile AC_PASSWORD

notarize-wait:
	cd $(APP_DIR) && xcrun notarytool wait "$$(cat .last_notarization_id)" --keychain-profile AC_PASSWORD
