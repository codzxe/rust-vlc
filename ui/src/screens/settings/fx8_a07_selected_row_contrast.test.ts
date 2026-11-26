// FX8-A07 (FIX-PLAN §4 F2.2 tail) — WCAG-AA regression guard for the settings
// selected values-row ink: the ONE contrast exclusion that was fixed in code.
//
// The finding (ledgered by w26.k19 in e2e/a11y.spec.ts ROUTE_EXCLUSIONS): the
// light selected values-row label painted the accent #d37918 on the flipped
// light tint #d8ba9a — 1.75:1, axe serious. The fix (setting-field.css, the
// w26.d2 button-lane light-sweep precedent): light keeps the tinted fill and
// swaps the ink to content-primary; the axe entry is DELETED, so a regression
// fails the per-PR a11y suite (e2e). This unit guard catches the css drift
// one lane earlier, asserting the stylesheet source mapped through the real
// token values (the Button.test.tsx precedent — vitest stubs `.css` imports,
// so the sheet is read via node:fs and the token cascade is evaluated here).
//
// Computed against the committed token set (src/tokens/generated/tokens.ts,
// the generated copy the app ships) with WCAG relative luminance:
//   light content-primary #161616 on the light tints — orange #d8ba9a 9.83:1
//   (the default accent), worst violet #b192e0 6.94:1; every accent >= 4.5:1
//   (WCAG 2.1 AA, normal text). Dark keeps the accent ink; the DEFAULT accent
//   measures 4.79:1 on #371f06 — the a11y suite audits the default accent,
//   and the non-default accents are sampled by the visual matrix only on
//   /onboarding + /settings/global, never on /settings/audio.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ACCENTS, TOKENS_CSS } from "../../tokens/generated/tokens";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const fieldCss = readFileSync(`${dirname}/setting-field.css`, "utf8");

/** One css rule body for a selector (the Button.test.tsx cssRule seam). */
function cssRule(selector: string): string {
  const match = fieldCss.match(
    new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`),
  );
  return match?.[1] ?? "";
}

/** One generated-token block body (`:root[data-theme="light"]` etc.). */
function tokenBlock(selector: string): string {
  const match = TOKENS_CSS.match(
    new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`),
  );
  return match?.[1] ?? "";
}

/** One hex declaration out of a token block. */
function decl(block: string, name: string): string {
  return block.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})`))?.[1] ?? "";
}

/** WCAG 2.x relative luminance + contrast ratio (computed, not hardcoded). */
function srgbToLinear(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}
function luminance(hex: string): number {
  const n = Number.parseInt(hex.slice(1), 16);
  return (
    0.2126 * srgbToLinear((n >> 16) & 0xff) +
    0.7152 * srgbToLinear((n >> 8) & 0xff) +
    0.0722 * srgbToLinear(n & 0xff)
  );
}
function contrast(fg: string, bg: string): number {
  const l1 = luminance(fg);
  const l2 = luminance(bg);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

/** The light ink + the 9 light accent tints, as shipped. */
function lightInkAndTints(): { ink: string; tints: ReadonlyArray<{ accent: string; tint: string }> } {
  const lightBlock = tokenBlock(':root[data-theme="light"]');
  const ink = decl(lightBlock, "--color-content-primary");
  const tints = ACCENTS.map((accent) => ({
    accent,
    // orange is the default: its light tint lives in the plain light block
    tint:
      accent === "orange"
        ? decl(lightBlock, "--color-accent-tinted")
        : decl(tokenBlock(`:root[data-theme="light"][data-accent="${accent}"]`), "--color-accent-tinted"),
  }));
  return { ink, tints };
}

describe("fx8_a07: settings selected values-row ink (WCAG AA, F2.2)", () => {
  it("light mode paints the selected values-row ink as content-primary; dark keeps the accent", () => {
    const light = cssRule(
      ':root[data-theme="light"] .settings-field__control .settings-values__row--selected',
    );
    expect(light, "the light-mode ink override rule must exist").toContain(
      "color: var(--color-content-primary)",
    );
    // The :root[data-theme="light"] prefix out-ranks the base rule
    // ((0,5,0) vs (0,3,0)), so light resolves to content-primary.
    const base = cssRule(".settings-field__control .settings-values__row--selected");
    expect(base).toContain("color: var(--color-accent)");
    expect(base).toContain("background: var(--color-accent-tinted)");
  });

  it("content-primary meets WCAG AA (>=4.5:1) on all 9 light accent tints (orange 9.83:1, worst violet 6.94:1)", () => {
    const { ink, tints } = lightInkAndTints();
    expect(ink, "light content-primary must resolve to the near-black ink").toBe("#161616");
    const ratios = tints.map(({ accent, tint }) => ({
      accent,
      tint,
      ratio: contrast(ink, tint),
    }));
    for (const { accent, tint, ratio } of ratios) {
      expect(
        ratio,
        `light ${accent}: ${ink} on ${tint} = ${ratio.toFixed(2)}:1 must be >= 4.5:1 (WCAG AA)`,
      ).toBeGreaterThanOrEqual(4.5);
    }
    const orange = ratios.find((r) => r.accent === "orange")?.ratio ?? 0;
    expect(orange).toBeCloseTo(9.83, 2);
    const worst = ratios.reduce((min, r) => Math.min(min, r.ratio), Number.POSITIVE_INFINITY);
    expect(worst).toBeCloseTo(6.94, 2); // violet #b192e0 is the tightest tint
  });

  it("the old accent ink measured 1.75:1 on the light tint — the axe-serious finding this fix closes", () => {
    const accent = decl(tokenBlock(":root"), "--color-accent");
    const lightTint = decl(tokenBlock(':root[data-theme="light"]'), "--color-accent-tinted");
    expect(accent).toBe("#d37918");
    expect(lightTint).toBe("#d8ba9a");
    expect(contrast(accent, lightTint)).toBeCloseTo(1.75, 2);
  });

  it("dark default stays AA: the accent ink on tint #371f06 = 4.79:1", () => {
    const accent = decl(tokenBlock(":root"), "--color-accent");
    const darkTint = decl(tokenBlock(":root"), "--color-accent-tinted");
    expect(accent).toBe("#d37918");
    expect(darkTint).toBe("#371f06");
    expect(contrast(accent, darkTint)).toBeCloseTo(4.79, 2);
  });
});
