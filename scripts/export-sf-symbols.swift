#!/usr/bin/env swift
// Exports SF Symbol vector paths to src/lib/sf-symbols/registry.ts (macOS only).
//
// Vector extraction uses undocumented AppKit selectors on NSImageRep (`vectorGlyph`,
// `CGPath`). Apple provides no public API to export SF Symbol SVG paths; this is the
// standard maintainer-only approach for macOS apps that bake symbols into WebViews.
// CI `sf-symbols` job pins `macos-15` and re-exports on every push so geometry drift is caught early.
import AppKit
import CoreGraphics
import Foundation

let exportSize: CGFloat = 16
let symbols = [
    "magnifyingglass",
    "xmark",
    "gearshape",
    "star",
    "star.fill",
    "arrow.down.doc",
    "arrow.triangle.2.circlepath",
    "mic.fill",
    "checkmark",
    "checkmark.shield",
    "lock",
    "externaldrive",
    "clipboard",
    "tag",
    "photo",
    "chevron.down",
    "plus.app",
    "minus",
    "plus",
    "checklist",
    "macwindow",
    "shippingbox",
]

private let vectorGlyphSelector = Selector(("vectorGlyph"))
private let cgPathSelector = Selector(("CGPath"))

/// Cap height in export points at `exportSize` (16). Most symbols use 13pt; filled
/// dismiss glyphs need a lower cap to match outline symbols like gearshape optically.
/// Manual overrides (xmark 10.5pt, plus 13pt) were visually verified against overlay
/// controls — add a case here when a new glyph reads too heavy or too light.
func targetCapHeight(for name: String, size: CGFloat) -> CGFloat {
    let capPoints: CGFloat
    switch name {
    case "xmark":
        capPoints = 10.5
    case "plus":
        // Plus bars read lighter than filled xmark at the same cap.
        capPoints = 13.0
    default:
        capPoints = 13.0
    }
    return size * (capPoints / exportSize)
}

func pathToSvg(_ path: CGPath, name: String, size: CGFloat) -> String {
    // Scale to a shared cap height and center in the em square.
    // Filling the bbox (size / max(w,h)) upscales compact filled glyphs like xmark;
    // native-only coords leave wide symbols clipped in a 16×16 viewBox.
    let bounds = path.boundingBox
    let minX = bounds.minX
    let minY = bounds.minY
    let width = max(bounds.width, .leastNonzeroMagnitude)
    let height = max(bounds.height, .leastNonzeroMagnitude)
    let capHeight = targetCapHeight(for: name, size: size)
    let scale = capHeight / max(width, height)
    let offsetX = (size - width * scale) / 2 - minX * scale
    let offsetY = (size - height * scale) / 2 - minY * scale

    func mapX(_ x: CGFloat) -> CGFloat { x * scale + offsetX }
    func mapY(_ y: CGFloat) -> CGFloat { y * scale + offsetY }

    var d = ""
    path.applyWithBlock { element in
        let pts = element.pointee.points
        switch element.pointee.type {
        case .moveToPoint:
            d += String(format: "M%.2f %.2f ", mapX(pts[0].x), mapY(pts[0].y))
        case .addLineToPoint:
            d += String(format: "L%.2f %.2f ", mapX(pts[0].x), mapY(pts[0].y))
        case .addQuadCurveToPoint:
            d += String(
                format: "Q%.2f %.2f %.2f %.2f ",
                mapX(pts[0].x), mapY(pts[0].y), mapX(pts[1].x), mapY(pts[1].y)
            )
        case .addCurveToPoint:
            d += String(
                format: "C%.2f %.2f %.2f %.2f %.2f %.2f ",
                mapX(pts[0].x), mapY(pts[0].y),
                mapX(pts[1].x), mapY(pts[1].y),
                mapX(pts[2].x), mapY(pts[2].y)
            )
        case .closeSubpath:
            d += "Z "
        @unknown default:
            break
        }
    }
    return d.trimmingCharacters(in: .whitespaces)
}

/// Verifies undocumented vector export still works on this macOS build before processing
/// the full symbol list.
func preflightVectorExport(rep: NSImageRep) -> Bool {
    guard rep.responds(to: vectorGlyphSelector) else {
        fputs(
            "preflight failed: NSImageRep does not respond to vectorGlyph on this macOS build\n",
            stderr
        )
        return false
    }
    guard let glyphUnmanaged = rep.perform(vectorGlyphSelector) else {
        fputs("preflight failed: vectorGlyph returned nil\n", stderr)
        return false
    }
    let glyph = glyphUnmanaged.takeUnretainedValue()
    guard glyph.responds(to: cgPathSelector) else {
        fputs("preflight failed: vector glyph does not respond to CGPath\n", stderr)
        return false
    }
    guard let pathUnmanaged = glyph.perform(cgPathSelector) else {
        fputs("preflight failed: CGPath returned nil\n", stderr)
        return false
    }
    let path = pathUnmanaged.takeUnretainedValue() as! CGPath
    guard !path.isEmpty else {
        fputs("preflight failed: CGPath missing or empty\n", stderr)
        return false
    }
    return true
}

/// Extracts vector paths via private AppKit selectors (`vectorGlyph`, `CGPath`).
func exportPath(name: String, size: CGFloat) -> String? {
    guard let img = NSImage(systemSymbolName: name, accessibilityDescription: nil) else {
        fputs("missing symbol: \(name)\n", stderr)
        return nil
    }
    let config = NSImage.SymbolConfiguration(pointSize: size, weight: .regular)
    guard let configured = img.withSymbolConfiguration(config),
          let imageRep = configured.representations.first
    else {
        fputs("no bitmap representation for: \(name)\n", stderr)
        return nil
    }

    guard imageRep.responds(to: vectorGlyphSelector) else {
        fputs("no vectorGlyph for: \(name)\n", stderr)
        return nil
    }
    guard let glyphUnmanaged = imageRep.perform(vectorGlyphSelector) else {
        fputs("vectorGlyph returned nil for: \(name)\n", stderr)
        return nil
    }
    let glyph = glyphUnmanaged.takeUnretainedValue()
    guard glyph.responds(to: cgPathSelector) else {
        fputs("no CGPath for: \(name)\n", stderr)
        return nil
    }
    guard let pathUnmanaged = glyph.perform(cgPathSelector) else {
        fputs("CGPath returned nil for: \(name)\n", stderr)
        return nil
    }
    let path = pathUnmanaged.takeUnretainedValue() as! CGPath
    if path.isEmpty {
        fputs("empty path for: \(name)\n", stderr)
        return nil
    }

    return pathToSvg(path, name: name, size: size)
}

/// Repo root: scripts/export-sf-symbols.swift → parent is scripts/, grandparent is repo root.
func repoRootURL() -> URL {
    let scriptPath = URL(fileURLWithPath: CommandLine.arguments[0]).resolvingSymlinksInPath()
    return scriptPath.deletingLastPathComponent().deletingLastPathComponent()
}

/// Keep in sync with `runs-on: macos-15` on the `sf-symbols` job in `.github/workflows/ci.yml` and `release.yml`.
let exportMacOSMajor = 15

/// Reads `--color-text-primary: #rrggbb` — canonical palette hex for chevron data URIs.
func parsePrimaryTextFillHex(tokensURL: URL) throws -> String {
    let content = try String(contentsOf: tokensURL, encoding: .utf8)
    guard let line = content.split(separator: "\n").first(where: { $0.contains("--color-text-primary:") }) else {
        fputs("--color-text-primary not found in tokens.css\n", stderr)
        exit(1)
    }
    guard let hashIndex = line.firstIndex(of: "#") else {
        fputs("--color-text-primary has no #hex in tokens.css\n", stderr)
        exit(1)
    }
    let hexStart = line.index(after: hashIndex)
    let hexEnd = line.index(hexStart, offsetBy: 6, limitedBy: line.endIndex) ?? line.endIndex
    let hex = String(line[hexStart ..< hexEnd]).lowercased()
    guard hex.count == 6, hex.allSatisfy(\.isHexDigit) else {
        fputs("--color-text-primary must be #rrggbb in tokens.css\n", stderr)
        exit(1)
    }
    return hex
}

func svgChevronDataUri(path: String, fillHex: String) -> String {
    // WebKit/Tauri cannot resolve currentColor inside background-image data URIs on <select>;
    // fill hex is read from --color-text-primary in tokens.css at export time.
    let svg =
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'>"
        + "<path fill='#\(fillHex)' fill-opacity='0.85' d='\(path)'/></svg>"
    let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_.!~*'()"))
    let encoded = svg.addingPercentEncoding(withAllowedCharacters: allowed) ?? svg
    return "data:image/svg+xml,\(encoded)"
}

func syncChevronInTokens(path: String, fillHex: String, tokensURL: URL) throws {
    var content = try String(contentsOf: tokensURL, encoding: .utf8)
    let dataUri = svgChevronDataUri(path: path, fillHex: fillHex)
    let newLine = "  --icon-chevron-down: url(\"\(dataUri)\");"
    let pattern = "  --icon-chevron-down: url\\(\"[^\"]*\"\\);"
    guard let range = content.range(of: pattern, options: .regularExpression) else {
        fputs("Could not find --icon-chevron-down in tokens.css\n", stderr)
        exit(1)
    }
    content.replaceSubrange(range, with: newLine)
    try content.write(to: tokensURL, atomically: true, encoding: .utf8)
}

// --- Main ---

let root = repoRootURL()
let outURL = root.appendingPathComponent("src/lib/sf-symbols/registry.ts")
let tokensURL = root.appendingPathComponent("src/lib/styles/tokens.css")
let fillHex = try parsePrimaryTextFillHex(tokensURL: tokensURL)
let hostMacOS = ProcessInfo.processInfo.operatingSystemVersion
let hostVersionLabel = "\(hostMacOS.majorVersion).\(hostMacOS.minorVersion).\(hostMacOS.patchVersion)"

var lines: [String] = [
    "// Generated by scripts/export-sf-symbols.swift — do not edit by hand.",
    "// Re-run on macOS after changing the symbol list.",
    "// Host: macOS \(hostVersionLabel). Canonical geometry: macOS \(exportMacOSMajor) (CI pins macos-\(exportMacOSMajor)).",
    "",
    "export const SF_SYMBOL_VIEWBOX = \(Int(exportSize));",
    "",
    "/** macOS major version for canonical path geometry (must match CI `macos-\(exportMacOSMajor)`). */",
    "export const SF_SYMBOL_EXPORT_MACOS_MAJOR = \(exportMacOSMajor);",
    "",
    "export type SfSymbolName =",
]

for (index, name) in symbols.enumerated() {
    let suffix = index == symbols.count - 1 ? ";" : ""
    lines.append("  | \"\(name)\"\(suffix)")
}

lines.append("")
lines.append("export const SF_SYMBOL_PATHS: Record<SfSymbolName, string> = {")

var chevronPathForTokens: String?

guard let probe = NSImage(systemSymbolName: "gearshape", accessibilityDescription: nil),
      let probeConfigured = probe.withSymbolConfiguration(
          NSImage.SymbolConfiguration(pointSize: exportSize, weight: .regular)
      ),
      let probeRep = probeConfigured.representations.first,
      preflightVectorExport(rep: probeRep)
else {
    fputs(
        "Cannot export SF Symbol vectors on this macOS build — vectorGlyph/CGPath unavailable. Pin CI to macos-15 or update scripts/export-sf-symbols.swift.\n",
        stderr
    )
    exit(1)
}

for name in symbols {
    guard let exported = exportPath(name: name, size: exportSize) else {
        exit(1)
    }
    if name == "chevron.down" {
        chevronPathForTokens = exported
    }
    let escaped = exported.replacingOccurrences(of: "\\", with: "\\\\")
        .replacingOccurrences(of: "\"", with: "\\\"")
    lines.append("  \"\(name)\": \"\(escaped)\",")
}

lines.append("};")
lines.append("")

guard let chevronPath = chevronPathForTokens else {
    fputs("chevron.down missing from symbol list\n", stderr)
    exit(1)
}

let output = lines.joined(separator: "\n")
try FileManager.default.createDirectory(
    at: outURL.deletingLastPathComponent(),
    withIntermediateDirectories: true
)
try output.write(to: outURL, atomically: true, encoding: .utf8)
try syncChevronInTokens(path: chevronPath, fillHex: fillHex, tokensURL: tokensURL)
print("Wrote \(outURL.path) (\(symbols.count) symbols, host macOS \(hostVersionLabel), canonical major \(exportMacOSMajor))")
print("Synced --icon-chevron-down in \(tokensURL.path) (fill #\(fillHex) from --color-text-primary)")
