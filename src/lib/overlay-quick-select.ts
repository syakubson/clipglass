/**
 * Quick-select: bare digit keys 1-9 paste the first nine overlay entries.
 * Pure logic only — the keydown wiring lives in routes/+page.svelte.
 */

export type QuickSelectEntry = { id: number; content_type: string };

/** How many leading entries get digit badges. */
export const QUICK_SELECT_MAX = 9;

const PASTEABLE_TYPES = new Set(["text", "image"]);

/**
 * Parse a keydown into a quick-select digit (1-9).
 * Returns null when the key is not a bare digit or focus sits in a text
 * field — in that case the digit must reach the input untouched.
 */
export function quickSelectDigit(
  key: string,
  opts: { searchFocused: boolean; typingInField: boolean; hasModifier: boolean },
): number | null {
  if (opts.searchFocused || opts.typingInField || opts.hasModifier) return null;
  if (key.length !== 1 || key < "1" || key > "9") return null;
  return key.charCodeAt(0) - 48;
}

/**
 * Entry id to activate for a 1-based digit, or null when the slot is empty
 * or the entry's type cannot be pasted into the target app.
 */
export function entryIdForDigit(
  digit: number,
  entries: readonly QuickSelectEntry[],
): number | null {
  if (digit < 1 || digit > QUICK_SELECT_MAX) return null;
  const entry = entries[digit - 1];
  if (!entry || !PASTEABLE_TYPES.has(entry.content_type)) return null;
  return entry.id;
}

/** Badge digit for a 0-based list index, or null beyond the first nine. */
export function badgeDigitForIndex(index: number): number | null {
  return index >= 0 && index < QUICK_SELECT_MAX ? index + 1 : null;
}
