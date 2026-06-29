import type { SfSymbolName } from "./registry";

export type SectionIconName =
  | "permissions"
  | "ai-tagging"
  | "ollama-model"
  | "this-mac"
  | "storage"
  | "privacy"
  | "voice"
  | "setup"
  | "clipboard-panel";

export const SECTION_ICON_SYMBOLS: Record<SectionIconName, SfSymbolName> = {
  permissions: "checkmark.shield",
  "ai-tagging": "tag",
  "ollama-model": "shippingbox",
  "this-mac": "macwindow",
  storage: "externaldrive",
  privacy: "lock",
  voice: "mic.fill",
  setup: "checklist",
  "clipboard-panel": "clipboard",
};
