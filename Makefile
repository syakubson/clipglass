APP_DIR ?= $(CURDIR)
export PATH := $(HOME)/.cargo/bin:$(PATH)
NPM := env -u npm_config_devdir npm
OLLAMA_MODEL ?= qwen3:4b-instruct-2507-q4_K_M
OLLAMA_DEBUG ?= 1

.PHONY: dev build check install build-macos build-macos-intel build-macos-arm \
	release-macos release-macos-intel release-macos-arm notarize-wait notarize-info

dev:
	cd $(APP_DIR) && COPYOSITY_OLLAMA_MODEL='$(OLLAMA_MODEL)' COPYOSITY_DEBUG_OLLAMA='$(OLLAMA_DEBUG)' $(NPM) run tauri dev

build:
	cd $(APP_DIR) && COPYOSITY_OLLAMA_MODEL='$(OLLAMA_MODEL)' COPYOSITY_DEBUG_OLLAMA='$(OLLAMA_DEBUG)' $(NPM) run tauri build

check:
	cd $(APP_DIR) && $(NPM) run check && cd src-tauri && cargo check && cargo test

install:
	cd $(APP_DIR) && $(NPM) install

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
