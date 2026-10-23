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
const PINNED_SHA256 = "832da9d0a9cb1595f572023ddd3dd65c137adbb59521c3b1d78983f622679745";

async function sha256(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
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
    // dark + orange are the DEFAULTS: their values live in plain :root, so
    // no [data-theme="dark"] / [data-accent="orange"] selector exists.
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
    expect(SEMANTIC_TOKEN_NAMES).toHaveLength(70);
    expect(tokenVar("--color-surface-raised")).toBe("var(--color-surface-raised)");
  });

  it("keeps the semantic layer free of value-named tokens (roles, not values)", () => {
    // doc 08 §3.2: names describe role, never value.
    for (const name of SEMANTIC_TOKEN_NAMES) {
      expect(name).not.toMatch(/--(color-)?(gray|hex|dark|light)-/);
    }
  });
});
