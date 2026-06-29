import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import { SF_SYMBOL_PATHS, SF_SYMBOL_EXPORT_MACOS_MAJOR } from "./registry.ts";
import { SECTION_ICON_SYMBOLS } from "./section-icons.ts";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../../..");

function primaryTextFillHexFromTokens(content: string): string {
  const line = content.match(/^\s*--color-text-primary:\s*#([0-9a-fA-F]{6});/m);
  assert.ok(line, "--color-text-primary #hex not found in tokens.css");
  return line[1].toLowerCase();
}

function normalizePath(path: string): string {
  return path.replace(/\s+/g, " ").trim();
}

function parseSwiftSymbolList(content: string): string[] {
  const block = content.match(/let symbols = \[([\s\S]*?)\]/);
  assert.ok(block, "export-sf-symbols.swift symbols array not found");
  return [...block[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

function chevronPathFromTokensCss(content: string): string {
  const line = content.match(/^\s*--icon-chevron-down:\s*url\("([^"]+)"\);/m);
  assert.ok(line, "--icon-chevron-down not found in tokens.css");
  const decoded = decodeURIComponent(line[1].replace(/^data:image\/svg\+xml,/, ""));
  const pathMatch = decoded.match(/\sd='([^']+)'/);
  assert.ok(pathMatch, "chevron path d attribute not found in tokens.css data URI");
  return pathMatch[1];
}

describe("SF_SYMBOL_EXPORT_MACOS_MAJOR", () => {
  it("matches CI macos-15 pin", () => {
    assert.equal(SF_SYMBOL_EXPORT_MACOS_MAJOR, 15);
  });
});

describe("SF_SYMBOL_PATHS", () => {
  it("every path is non-empty", () => {
    for (const [name, path] of Object.entries(SF_SYMBOL_PATHS)) {
      assert.ok(path.length > 0, `${name} path is empty`);
    }
  });
});

describe("SECTION_ICON_SYMBOLS", () => {
  it("maps every section to a registry symbol", () => {
    for (const [section, symbol] of Object.entries(SECTION_ICON_SYMBOLS)) {
      assert.ok(symbol in SF_SYMBOL_PATHS, `${section} -> ${symbol}`);
    }
  });
});

describe("export-sf-symbols.swift symbol list", () => {
  it("matches registry keys", () => {
    const swift = readFileSync(join(repoRoot, "scripts/export-sf-symbols.swift"), "utf8");
    const swiftSymbols = parseSwiftSymbolList(swift);
    const registrySymbols = Object.keys(SF_SYMBOL_PATHS);
    assert.deepEqual(
      [...swiftSymbols].toSorted(),
      [...registrySymbols].toSorted(),
      "Swift symbol list and registry.ts keys differ",
    );
  });
});

describe("chevron.down sync", () => {
  it("tokens.css data URI path matches registry", () => {
    const tokens = readFileSync(join(repoRoot, "src/lib/styles/tokens.css"), "utf8");
    const fromTokens = chevronPathFromTokensCss(tokens);
    const fromRegistry = SF_SYMBOL_PATHS["chevron.down"];
    assert.equal(
      normalizePath(fromTokens),
      normalizePath(fromRegistry),
      "chevron.down path drift between registry.ts and tokens.css",
    );
  });

  it("tokens.css chevron fill matches --color-text-primary", () => {
    const tokens = readFileSync(join(repoRoot, "src/lib/styles/tokens.css"), "utf8");
    const fillHex = primaryTextFillHexFromTokens(tokens);
    const line = tokens.match(/^\s*--icon-chevron-down:\s*url\("([^"]+)"\);/m);
    assert.ok(line);
    const decoded = decodeURIComponent(line[1].replace(/^data:image\/svg\+xml,/, ""));
    assert.match(decoded, new RegExp(`fill='#${fillHex}'`));
    assert.doesNotMatch(decoded, /currentColor/);
  });

  it("icon-chevron-select-fill aliases --color-text-primary", () => {
    const tokens = readFileSync(join(repoRoot, "src/lib/styles/tokens.css"), "utf8");
    assert.match(tokens, /--icon-chevron-select-fill:\s*var\(--color-text-primary\)/);
  });
});
