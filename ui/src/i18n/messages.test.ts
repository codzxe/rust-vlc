// l10n integrity + pseudo-locale unit tests (card 4.H3).
//
// The zero-dep CI gate is `npm run l10n:check` (scripts/l10n-check.mjs);
// these vitest cases cover the runtime side the script cannot reach:
// locale switching, fallback totality, and pseudo-transform contracts.
import { afterEach, describe, expect, it } from "vitest";
import {
  activeLocale,
  isMessageKey,
  messages,
  setActiveLocale,
  useMsg,
  type MessageKey,
} from "./messages";
import { PSEUDO_LOCALE_TAG, toPseudo } from "./pseudo";

const keys = Object.keys(messages) as MessageKey[];

afterEach(() => {
  setActiveLocale("en-US");
});

describe("en-US reference catalog (doc 07 §6.3)", () => {
  it("every entry is a non-empty string (no missing/empty slots)", () => {
    expect(keys.length).toBeGreaterThan(400);
    for (const key of keys) {
      expect(typeof messages[key], key).toBe("string");
      expect(messages[key].length, key).toBeGreaterThan(0);
    }
  });

  it("isMessageKey accepts catalog keys and rejects unknowns", () => {
    expect(isMessageKey("player.controls.play")).toBe(true);
    expect(isMessageKey("not.a.real.key")).toBe(false);
  });
});

describe("pseudo locale (card 4.H3)", () => {
  it("toPseudo wraps, accents, and lengthens deterministically", () => {
    const once = toPseudo("Play");
    expect(toPseudo("Play")).toBe(once); // deterministic
    expect(once.startsWith("[")).toBe(true);
    expect(once.endsWith("]")).toBe(true);
    expect(once.length).toBeGreaterThanOrEqual(Math.ceil("Play".length * 1.5));
    expect(once).not.toContain("Play"); // letters transformed
  });

  it("toPseudo preserves doc 07 §6.2 placeholders and {{ }} escapes verbatim", () => {
    const msg = "{count, plural, one {# item} other {# items}} and {name}";
    const pseudo = toPseudo(msg);
    expect(pseudo).toContain("{count, plural, one {# item} other {# items}}");
    expect(pseudo).toContain("{name}");
    expect(toPseudo("{{literal}}")).toContain("{{literal}}");
  });

  it("setActiveLocale(pseudo) switches useMsg for every key, with en-US fallback intact", () => {
    setActiveLocale(PSEUDO_LOCALE_TAG);
    expect(activeLocale()).toBe(PSEUDO_LOCALE_TAG);
    const m = useMsg();
    for (const key of keys) {
      const p = m(key);
      expect(p.startsWith("["), key).toBe(true);
      expect(p.endsWith("]"), key).toBe(true);
      expect(p, key).not.toBe(messages[key]);
    }
  });

  it("unknown locale tags fall back to the reference catalog (chain stays total)", () => {
    setActiveLocale("de-DE"); // no such catalog in the scaffold
    expect(activeLocale()).toBe("en-US");
    expect(useMsg()("player.controls.play")).toBe("Play");
  });
});
