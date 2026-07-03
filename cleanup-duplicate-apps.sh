#!/bin/bash
# Удаление дублей приложений, найденных на этом ПК (Spotlight, 2026-06-15).
# БЕЗОПАСНО: здесь только лишние сборки/билды и старые версии — не системные приложения.
# Проверь список и раскомментируй (или запусти целиком). Ничего не удаляется без твоего запуска.
set -e

# --- Старая копия всего проекта Clipglass в папке coplys (вероятно опечатка) ---
rm -rf "/Users/v.kovalskii/coplys/clipglass/src-tauri/target/release/bundle/macos/Clipglass.app"

# --- ValeDesk: установленная версия в /Applications дублирует dev-сборку (сносим билд) ---
rm -rf "/Users/v.kovalskii/KVDesk/ValeDesk/src-tauri/target/release/bundle/macos/ValeDesk.app"

# --- Xcode DerivedData / Archives: промежуточные сборки, спокойно удаляются ---
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/Agentic-ffxuvjodolfyqkeumnnvxlqgpjuu/Build/Products/Debug-iphoneos/Agentic.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/Agentic-ffxuvjodolfyqkeumnnvxlqgpjuu/Build/Products/Debug-iphonesimulator/Agentic.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/Archives/2026-02-20/Daisy 20.02.2026, 09.22.xcarchive/Products/Applications/Daisy.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/Daisy-aqqepfzlvfcextadonnluswtupfg/Build/Products/Debug-iphoneos/Daisy.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/Daisy-aqqepfzlvfcextadonnluswtupfg/Build/Products/Debug-iphonesimulator/Daisy.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/Archives/2026-02-20/DaisyWasp 20.02.2026, 16.22.xcarchive/Products/Applications/DaisyWasp.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/Archives/2026-02-20/DaisyWasp 20.02.2026, 16.24.xcarchive/Products/Applications/DaisyWasp.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/DaisyWasp-athfmojpdmnxxrfemhmiwqxixmcq/Build/Products/Release-iphoneos/DaisyWasp.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/topsha-mobile-cxzmglxvicsdhfcyjwfsnqgnskec/Build/Products/debug-iphonesimulator/Topsha.app"
rm -rf "/Users/v.kovalskii/Library/Developer/Xcode/DerivedData/topsha-mobile-ddeteypouwwonehaxtdmczkqlfeq/Build/Products/debug-iphonesimulator/Topsha.app"

# --- Battle.net Agent: старая версия рядом с актуальной ---
rm -rf "/Users/Shared/Battle.net/Agent/Agent.8093/Agent.app"

echo "Готово. Дубли-сборки удалены."

# НЕ ТРОГАЕМ (это не дубли):
#   /Applications/Telegram.app + Telegram.localized — обёртка локализации, один и тот же бандл
#   /Applications/Termius.app  + Termius.localized   — то же самое
#   /System/Applications/Siri.app + /System/Library/CoreServices/Siri.app — системные, под защитой SIP
