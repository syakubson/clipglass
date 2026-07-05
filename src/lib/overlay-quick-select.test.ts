import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  badgeDigitForIndex,
  entryIdForDigit,
  QUICK_SELECT_MAX,
  quickSelectDigit,
} from "./overlay-quick-select.ts";

const freeFocus = { searchFocused: false, typingInField: false, hasModifier: false };

describe("quickSelectDigit", () => {
  it("maps bare digit keys 1-9 to their number", () => {
    assert.equal(quickSelectDigit("1", freeFocus), 1);
    assert.equal(quickSelectDigit("9", freeFocus), 9);
  });
  it("rejects 0, letters, multi-char keys and symbols", () => {
    assert.equal(quickSelectDigit("0", freeFocus), null);
    assert.equal(quickSelectDigit("a", freeFocus), null);
    assert.equal(quickSelectDigit("Digit1", freeFocus), null);
    assert.equal(quickSelectDigit("/", freeFocus), null);
  });
  it("yields to search field and text inputs", () => {
    assert.equal(quickSelectDigit("1", { ...freeFocus, searchFocused: true }), null);
    assert.equal(quickSelectDigit("1", { ...freeFocus, typingInField: true }), null);
  });
  it("yields when a modifier is held (Cmd/Ctrl/Alt combos are not ours)", () => {
    assert.equal(quickSelectDigit("1", { ...freeFocus, hasModifier: true }), null);
  });
});

describe("entryIdForDigit", () => {
  const entries = [
    { id: 11, content_type: "text" },
    { id: 22, content_type: "image" },
    { id: 33, content_type: "file" },
  ];
  it("returns the id of the digit's entry (1-based)", () => {
    assert.equal(entryIdForDigit(1, entries), 11);
    assert.equal(entryIdForDigit(2, entries), 22);
  });
  it("returns null for empty slots", () => {
    assert.equal(entryIdForDigit(4, entries), null);
    assert.equal(entryIdForDigit(9, entries), null);
  });
  it("returns null for non-pasteable content types", () => {
    assert.equal(entryIdForDigit(3, entries), null);
  });
  it("returns null out of range", () => {
    assert.equal(entryIdForDigit(0, entries), null);
    assert.equal(entryIdForDigit(QUICK_SELECT_MAX + 1, entries), null);
  });
});

describe("badgeDigitForIndex", () => {
  it("labels the first nine indexes 1-9", () => {
    assert.equal(badgeDigitForIndex(0), 1);
    assert.equal(badgeDigitForIndex(8), 9);
  });
  it("returns null beyond the first nine and for negatives", () => {
    assert.equal(badgeDigitForIndex(9), null);
    assert.equal(badgeDigitForIndex(-1), null);
  });
});
