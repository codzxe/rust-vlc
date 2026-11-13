// Token snapshot test — card 4.A2 AC (doc 08 §3.4 "validation & hygiene").
//
// Pins the generated token CSS by sha256 and asserts its structure, so any
// drift in themes/tokens.json → themes/generated/tokens.css (Rust side) or a
// forgotten re-run of `npm run tokens:generate` fails here loudly.
//
// Byte-identity with the crate output (themes/generated/tokens.css) is
// enforced by `npm run tokens:check` (scripts/gen-tokens.mjs --check) and by
// crates/theme-tokens/tests/diff.rs (runs under `cargo test --workspace` in
// ci.yml — the gate j18 F3 wired); this file pins the committed ui copy's
// content + shape.

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
// (w26.i19 light WCAG repin 2026-11-12, theme/token lane: light secondary
// #4e4e4e→#3a3a3a, light tertiary #5f5f5f→#474747 (doc/ship split-brain closed
// — h05 F3 + h20 F1), light border #acacac→#9a9a9a (ΔRGB-1-from-chrome
// collision, h05 F5), NEW --color-focus-ring (dark #d37918 / light #824505,
// h05 F1). Same session: dark surface-chrome re-pointed #161616→#1b1b1b
// (gray.775 — Player.svg topbar, w26.i02 ruling text in tokens.json;
// concurrent Lead edit adopted here). Prior: w26.f15 WCAG repin (dark
// tertiary #6b6b6b→#828282, light accent-bright #e8851a→#b35f0d) and the
// W25 Figma redesign, 2026-11-11 (neutral near-black surfaces, orange
// default accent, Inter face; sunset/note/tape/keycap/visualizer-gradient
// Solstice groups removed; eq band accents added.)
// w26.i19 relaunch additions: NEW --color-content-quiet (dark #6b6b6b /
// light #5f5f5f — the frames' quiet band, decorative/inactive scope, i07
// three-wave ask + i13 arbitration: NOT tertiary, #6b6b6b as text is 3.30:1
// on #191919, sub-AA), NEW --color-surface-elevated (dark #1b1b1b panel+2 /
// light #c9c9c9 panel+3 — i09's +2 elevated-panel step ask), and
// --font-size-label 16px→15px (CONTRACT §2 10–15px band, h19/h20 violation,
// a18/i09 15px-token ask).)
// w26.k18 additions: NEW --color-surface-panel-raised (dark #191919 = panel /
// light #dedede — the raised CARD step, k17 P3-4 + i09/i17/i18 standing ask;
// the discovery/convert/media-info `var(--color-surface-panel-raised, …)`
// fallbacks now resolve to a real token) and NEW --font-size-scale-label
// 12px (slider scale labels — the ≈12px frame band i14 asked for; caption
// stays 11px). Counts 122/84/84 → 123/86/86; emit.rs ramp-section split fix
// (j18 F2) changed the generated tokens.rs section layout in the same
// regeneration.
const PINNED_SHA256 =
  "26c55ab2b3193445728537df9cbb878bc77a46b7752714f1ff45752489648a20"; /* w26.k18: +surface.panel-raised, +font.size.scale-label (123/86/86) */

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
    expect(SEMANTIC_TOKEN_NAMES).toHaveLength(86); // w26.k18: +surface.panel-raised, +font.size.scale-label (86 semantic — 123/86/86) // w26.i19+i16 Lead: +content.quiet, +content.accelerator (84 semantic) // w26.i19: +content-quiet +surface-elevated
    expect(tokenVar("--color-surface-raised")).toBe("var(--color-surface-raised)");
    expect(tokenVar("--color-focus-ring")).toBe("var(--color-focus-ring)");
    expect(tokenVar("--color-content-quiet")).toBe("var(--color-content-quiet)");
    expect(tokenVar("--color-surface-elevated")).toBe("var(--color-surface-elevated)");
    expect(tokenVar("--color-surface-panel-raised")).toBe("var(--color-surface-panel-raised)"); // w26.k18 (k17 P3-4)
    expect(tokenVar("--font-size-scale-label")).toBe("var(--font-size-scale-label)"); // w26.k18 (i14)
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
    expect(rootDecl("--color-surface-chrome")).toBe("#1b1b1b"); // w26.i02 gray.775 re-point (Player.svg topbar), adopted w26.i19
    expect(rootDecl("--color-surface-raised")).toBe("#222222");
    expect(rootDecl("--color-surface-hover")).toBe("#262626");
    expect(rootDecl("--color-surface-selected")).toBe("#1f1f1f");
    expect(rootDecl("--color-surface-muted")).toBe("#151515");
    expect(rootDecl("--color-stroke-border")).toBe("#404040");
    expect(rootDecl("--color-content-primary")).toBe("#c6c6c6");
    expect(rootDecl("--color-content-secondary")).toBe("#8f8f8f");
    expect(rootDecl("--color-content-tertiary")).toBe("#828282");
    expect(rootDecl("--color-content-quiet")).toBe("#6b6b6b"); // w26.i19: frames' quiet band, decorative/inactive only (3.30:1 as text — NOT tertiary)
    expect(rootDecl("--color-surface-elevated")).toBe("#1b1b1b"); // w26.i19: panel+2 elevated step (i09 ask)
    expect(rootDecl("--color-surface-panel-raised")).toBe("#191919"); // w26.k18: dark cards = panel (Δ16 on surface, i18 probe; k17 P3-4)
    expect(rootDecl("--font-size-scale-label")).toBe("12px"); // w26.k18: the ≈12px scale-label band (i14 ask; caption stays 11px)
    expect(rootDecl("--font-size-label")).toBe("15px"); // w26.i19: §2 10–15px band (was 16px, h19/h20)
    expect(rootDecl("--color-content-bright")).toBe("#eaeaea");
    // The orange accent family (default accent).
    expect(rootDecl("--color-accent")).toBe("#d37918");
    expect(rootDecl("--color-accent-strong")).toBe("#ab6314");
    expect(rootDecl("--color-accent-tinted")).toBe("#371f06");
    expect(rootDecl("--color-content-on-accent")).toBe("#090909");
    expect(rootDecl("--color-record")).toBe("#e15032");
    // Light mode.
    expect(lightDecl("--color-surface")).toBe("#c2c2c2"); // w26 g03/g05 Lead light-ramp repin (frame warm grays)
    expect(lightDecl("--color-surface-chrome")).toBe("#ababab");
    expect(lightDecl("--color-surface-panel")).toBe("#c6c6c6");
    expect(lightDecl("--color-content-primary")).toBe("#161616");
    expect(lightDecl("--color-content-secondary")).toBe("#3a3a3a"); // w26.i19 WCAG: AA (≥4.5:1) on EVERY light surface incl. chrome (4.95:1)
    expect(lightDecl("--color-content-tertiary")).toBe("#474747"); // w26.i19: split-brain closed — AA on surface/panel/deep/muted/raised
    expect(lightDecl("--color-stroke-border")).toBe("#9a9a9a"); // w26.i19: ΔRGB 17 from chrome (was #acacac, ΔRGB 1 — dead separators)
    expect(lightDecl("--color-stroke-icon")).toBe("#6b6b6b");
    expect(lightDecl("--color-accent-bright")).toBe("#b35f0d"); // fills/hover only — NOT certified as light text (2.5–2.8:1, h20 F2)
    expect(lightDecl("--color-focus-ring")).toBe("#824505"); // w26.i19: WCAG 2.2 3:1 vs every light surface (h05 F1)
    expect(lightDecl("--color-content-quiet")).toBe("#5f5f5f"); // w26.i19: ≥3:1 non-text on surface/panel/deep/muted
    expect(lightDecl("--color-surface-elevated")).toBe("#c9c9c9"); // w26.i19: panel+3 — smallest designated light lift
    expect(lightDecl("--color-surface-panel-raised")).toBe("#dedede"); // w26.k18: light card step runs LIGHTER than panel (Δ28 on surface #c2c2c2, i18 pixel probe)
    expect(rootDecl("--color-focus-ring")).toBe("#d37918"); // dark ring = the accent, unchanged
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
