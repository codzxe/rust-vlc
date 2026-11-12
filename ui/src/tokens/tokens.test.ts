// Token snapshot test — card 4.A2 AC (doc 08 §3.4 "validation & hygiene").
//
// Pins the generated token CSS by sha256 and asserts its structure, so any
// drift in themes/tokens.json → themes/generated/tokens.css (Rust side) or a
// forgotten re-run of `npm run tokens:generate` fails here loudly.
//
// Byte-identity with the crate output (themes/generated/tokens.css) is
// enforced by `npm run tokens:check` (scripts/gen-tokens.mjs --check) in CI;
// this file pins the committed ui copy's content + shape.

import { describe, expect, it } from "vitest";
import {
  ACCENTS,
  SEMANTIC_TOKEN_NAMES,
  TOKENS_CSS,
  TOKENS_CSS_SHA256,
  TOKEN_MODES,
  tokenVar,
} from "./generated/tokens";

const cssRaw = TOKENS_CSS;

// Update ONLY together with `cargo xtask tokens` + `npm run tokens:generate`
// in the same commit — schema drift must never slip in silently.
// (w26.f15 WCAG repin 2026-11-12: dark tertiary #6b6b6b→#828282, light tertiary
// #9a9a9a→#767676, light accent-bright #e8851a→#b35f0d — e17 F1/F2, e19 F3;
// deviation documented in themes/tokens.json primitive descriptions. Prior:
// (W25 Figma redesign, 2026-11-11: neutral near-black surfaces, orange
// default accent, Inter face; sunset/note/tape/keycap/visualizer-gradient
// Solstice groups removed; eq band accents added.)
const PINNED_SHA256 = "6edc89dfdbab0f439b081088b2314e70d326c3de2123fbe23e44ae7e33901f03";

async function sha256(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function rootDecl(name: string): string {
  const rootBlock = cssRaw.match(/:root\s*\{([^}]*)\}/)?.[1] ?? "";
  return rootBlock.match(new RegExp(`${name}\\s*:\\s*([^;]+);`))?.[1] ?? "";
}

function lightDecl(name: string): string {
  const lightBlock = cssRaw.match(/:root\[data-theme="light"\]\s*\{([^}]*)\}/)?.[1] ?? "";
  return lightBlock.match(new RegExp(`${name}\\s*:\\s*([^;]+);`))?.[1] ?? "";
}

describe("generated token snapshot (doc 08 §3)", () => {
  it("pins the generated tokens.css by sha256 — update deliberately", async () => {
    const hash = await sha256(cssRaw);
    expect(hash).toBe(PINNED_SHA256);
    expect(TOKENS_CSS_SHA256).toBe(PINNED_SHA256);
  });

  it("emits the documented mode and accent matrix (doc 08 §3.3)", () => {
    expect([...TOKEN_MODES]).toEqual(["dark", "light"]);
    expect(ACCENTS).toHaveLength(9);
    expect([...ACCENTS]).toEqual(
      expect.arrayContaining([
        "orange",
        "red",
        "yellow",
        "green",
        "cyan",
        "blue",
        "violet",
        "pink",
        "gray",
      ]),
    );
    // dark + orange are the DEFAULTS (W25 Figma redesign, 2026-11-11): their
    // values live in plain :root, so no [data-theme="dark"] /
    // [data-accent="orange"] selector exists.
    expect(cssRaw).toContain(':root[data-theme="light"]');
    expect(cssRaw).not.toMatch(/:root\[data-theme="dark"\]/);
    expect(cssRaw).not.toMatch(/:root\[data-accent="orange"\]/);
    // every non-default accent has a dark block AND a light combo block
    for (const accent of ACCENTS.filter((a) => a !== "orange")) {
      expect(cssRaw).toMatch(new RegExp(`:root\\[data-accent="${accent}"\\]`));
      expect(cssRaw).toMatch(
        new RegExp(`:root\\[data-theme="light"\\]\\[data-accent="${accent}"\\]`),
      );
    }
  });

  it("every accent block re-points the core accent tokens (doc 08 §3.2 roles)", () => {
    const rootBlock = cssRaw.match(/:root\s*\{([^}]*)\}/)?.[1] ?? "";
    for (const accent of ACCENTS) {
      // orange (the default) re-points the roles inside plain :root.
      const block =
        accent === "orange"
          ? rootBlock
          : (cssRaw.match(new RegExp(`:root\\[data-accent="${accent}"\\]\\s*\\{([^}]*)\\}`))?.[1] ?? "");
      expect(block.length, `accent block for ${accent}`).toBeGreaterThan(0);
      for (const role of [
        "--color-accent",
        "--color-accent-strong",
        "--color-accent-tinted",
        "--color-content-on-accent",
      ]) {
        expect(block).toContain(role);
      }
    }
  });

  it("the typed accessors mirror the :root block exactly", () => {
    const rootBlock = cssRaw.match(/:root\s*\{([^}]*)\}/)?.[1] ?? "";
    const cssNames = [...rootBlock.matchAll(/--[A-Za-z0-9-]+(?=\s*:)/g)].map((m) => m[0]);
    expect(new Set(cssNames)).toEqual(new Set(SEMANTIC_TOKEN_NAMES));
    expect(SEMANTIC_TOKEN_NAMES).toHaveLength(80);
    expect(tokenVar("--color-surface-raised")).toBe("var(--color-surface-raised)");
  });

  it("keeps the semantic layer free of value-named tokens (roles, not values)", () => {
    // doc 08 §3.2: names describe role, never value.
    for (const name of SEMANTIC_TOKEN_NAMES) {
      expect(name).not.toMatch(/--(color-)?(gray|hex|dark|light)-/);
    }
  });

  it("pins the W25 §1 measured values (figma-fidelity-spec.md, 2026-11-11)", () => {
    // Dark defaults (plain :root).
    expect(rootDecl("--color-surface")).toBe("#090909");
    expect(rootDecl("--color-surface-deep")).toBe("#0e0e0e");
    expect(rootDecl("--color-surface-panel")).toBe("#191919");
    expect(rootDecl("--color-surface-chrome")).toBe("#161616");
    expect(rootDecl("--color-surface-raised")).toBe("#222222");
    expect(rootDecl("--color-surface-hover")).toBe("#262626");
    expect(rootDecl("--color-surface-selected")).toBe("#1f1f1f");
    expect(rootDecl("--color-surface-muted")).toBe("#151515");
    expect(rootDecl("--color-stroke-border")).toBe("#404040");
    expect(rootDecl("--color-content-primary")).toBe("#c6c6c6");
    expect(rootDecl("--color-content-secondary")).toBe("#8f8f8f");
    expect(rootDecl("--color-content-tertiary")).toBe("#828282");
    expect(rootDecl("--color-content-bright")).toBe("#eaeaea");
    // The orange accent family (default accent).
    expect(rootDecl("--color-accent")).toBe("#d37918");
    expect(rootDecl("--color-accent-strong")).toBe("#ab6314");
    expect(rootDecl("--color-accent-tinted")).toBe("#371f06");
    expect(rootDecl("--color-content-on-accent")).toBe("#090909");
    expect(rootDecl("--color-record")).toBe("#e15032");
    // Light mode.
    expect(lightDecl("--color-surface")).toBe("#f0f0f0");
    expect(lightDecl("--color-surface-chrome")).toBe("#f5f5f5");
    expect(lightDecl("--color-surface-panel")).toBe("#efefef");
    expect(lightDecl("--color-content-primary")).toBe("#2b2b2b");
    expect(lightDecl("--color-content-secondary")).toBe("#6e6e6e");
    expect(lightDecl("--color-stroke-border")).toBe("#d9d9d9");
    expect(lightDecl("--color-stroke-icon")).toBe("#9a9a9a");
    expect(lightDecl("--color-accent-bright")).toBe("#b35f0d");
  });

  it("carries the 7 EQ band accents with frame provenance (W25 §1)", () => {
    const eq = [
      "#e15032",
      "#c49e23",
      "#19a234",
      "#18aad3",
      "#185dd3",
      "#8e42ff",
      "#f052b3",
    ];
    eq.forEach((hex, i) => {
      expect(rootDecl(`--color-eq-${i + 1}`)).toBe(hex);
    });
  });

  it("uses the W25 Inter stack and drops every Solstice remnant", () => {
    expect(rootDecl("--font-family-sans")).toBe(
      '"Inter", "-apple-system", "Segoe UI", "Roboto", sans-serif',
    );
    expect(rootDecl("--font-family-time")).toBe(
      '"Inter", "-apple-system", "Segoe UI", "Roboto", sans-serif',
    );
    expect(cssRaw).not.toMatch(/nunito/i);
    for (const gone of [
      "sunset",
      "note",
      "keycap",
      "tape",
      "visualizer",
      "stroke-warm",
      "shadow-note",
    ]) {
      expect(cssRaw).not.toContain(`--color-${gone}`);
      expect(cssRaw).not.toContain(`--gradient-${gone}`);
      expect(cssRaw).not.toContain(`--shadow-${gone}`);
      expect(SEMANTIC_TOKEN_NAMES.join(" ")).not.toContain(gone);
    }
  });
});
