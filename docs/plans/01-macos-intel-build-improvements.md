# macOS Intel build and related improvements

Кратко: зачем меняли код и инфраструктуру в этом наборе правок.

---

## 1. Сборка под Intel (x86_64)

**Цель:** получить воспроизводимую `.app` и DMG для Intel Mac, параллельно с Apple Silicon, без привязки к одной машине разработчика.

**Что сделано:**

- `scripts/build-macos.sh` — единый pipeline: frontend → Tauri bundle → DMG в `dist/macos/`.
- `scripts/macos-target.sh` — архитектура через `MACOS_ARCH=auto | x86_64 | aarch64`.
- `Makefile`: `build-macos`, `build-macos-intel`, `build-macos-arm` и зеркальные `release-macos-*`.
- Именованные артефакты в `dist/macos/` (например `Copyosity_0.3.0_x86_64.dmg`).
- `tauri.unsigned.json` — ad-hoc подпись для локальных сборок; release с Developer ID через `RELEASE_CONFIG=1` в `release-macos.sh`.
- `release-macos.sh` использует тот же build pipeline, что и локальная сборка.

**Как собрать:** `make build-macos-intel` (Intel) или `make build-macos` / `make build-macos-arm` на соответствующей машине.

---

## 2. Инфраструктура сборки и dev-окружения

**Цель:** чтобы Intel/ARM сборки и `npm run tauri` работали на любой машине и в IDE без ручной настройки путей.

- `APP_DIR ?= $(CURDIR)` в Makefile — не hardcoded путь к проекту.
- `env -u npm_config_devdir` для npm — стабильный `npm install` / Tauri build (в т.ч. Cursor).
- `scripts/with-tauri.sh`, `scripts/env-rust.sh` — `cargo` и `tauri` в PATH.
- `.vscode/settings.json` — тот же workaround для integrated terminal.
- `.gitignore`: `/dist` — каталог артефактов сборки.
- `README.md` — команды Intel/ARM и путь `dist/macos/`.
- Обновление SvelteKit / Svelte / Vite, override `cookie` — актуальный frontend toolchain на чистом clone.

---

## 3. macOS — буфер обмена и история

**Цель:** надёжнее ловить копирование на macOS, корректно показывать картинки, не засорять историю действиями самого приложения.

### Мониторинг

- `NSPasteboard.changeCount` — опрос только когда буфер реально менялся.
- Порядок чтения: **файлы → raster → текст** — при копировании image-файла в Finder в историю попадают пиксели файла, а не служебная иконка с pasteboard.
- `image` crate: декодеры jpeg, webp, gif, bmp, tiff для путей с диска; скриншоты и «Copy Image» по-прежнему через raster API.
- Игнор concealed pasteboard (пароли и скрытый контент).
- Игнор источника Copyosity и приложений из excluded list.
- Модули `CaptureContext`, `try_capture_from_clipboard` — единая точка разбора содержимого буфера.

### Запись, копирование и вставка

- `clipboard_macos.rs` — pasteboard API, `changeCount`, concealed, синтетический Cmd+V, запоминание и восстановление целевого приложения перед вставкой.
- `clipboard_write.rs` — запись в буфер с `exclude_from_history` и пометкой «своя» запись, чтобы copy из карточки не дублировал историю.
- `remember_paste_target` / `restore_paste_target` — double-click / Enter вставляют в приложение, из которого открыли панель.
- `copy_entry` / `activate_entry` — разделение «только в буфер» и «вставить в другое приложение».
- Enter в главном окне — `activateEntry` для текста и изображений.
- `check_accessibility` + UI в Settings — права для автоматической вставки и горячих клавиш.
- Окно Settings — корректный вывод на передний план (`objc2-app-kit`).
- Зависимость `cocoa` заменена на `objc2` / `objc2-app-kit` там, где это уже используется.

---

## 4. Frontend

- Главная лента: Enter = вставка выбранной записи (`activateEntry`) и закрытие панели.
- Settings: блок Permissions (Accessibility), подсказка про повторное добавление приложения в Privacy после новой сборки.
- Карточка: один клик — copy, двойной — paste (без смены этой модели).

---

## 5. Voice shortcut

- Транскрипция по-прежнему кладёт текст в буфер и имитирует Cmd+V; общая macOS-логика pasteboard вынесена в `clipboard_macos`.

---

## 6. Затронутые области репозитория

| Область | Файлы |
|---------|--------|
| Сборка | `Makefile`, `README.md`, `scripts/build-macos.sh`, `macos-target.sh`, `env-rust.sh`, `with-tauri.sh`, `with-npm.sh`, `release-macos.sh`, `tauri.unsigned.json` |
| Конфиг / deps | `.gitignore`, `.vscode/settings.json`, `package.json`, `package-lock.json` |
| Rust backend | `clipboard_monitor.rs`, `clipboard_macos.rs`, `clipboard_write.rs`, `commands.rs`, `lib.rs`, `Cargo.toml` |
| UI | `+page.svelte`, `settings/+page.svelte`, `ClipboardCard.svelte` |
