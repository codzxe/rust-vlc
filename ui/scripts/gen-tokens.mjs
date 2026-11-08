#!/usr/bin/env node
// gen-tokens.mjs — card 4.A2, doc 08 §3.1 UI side of the token pipeline.
//
// Pipeline shape (decision recorded in src/tokens/README.md):
//
//   themes/tokens.json  (Rust-owned source of truth, card 1.B4 — READ-ONLY)
//     └▶ cargo xtask tokens  (crates/theme-tokens `tokens-gen` — unchanged)
//          └▶ themes/generated/tokens.css   (committed crate output = the bridge)
//               └▶ THIS SCRIPT  (node, zero deps)
//                    ├▶ ui/src/tokens/generated/tokens.css  (verbatim copy)
//                    └▶ ui/src/tokens/generated/tokens.ts   (typed accessors)
//
// `node scripts/gen-tokens.mjs`         regenerate (idempotent, sorted output)
// `node scripts/gen-tokens.mjs --check` exit 1 on drift vs the committed copies
//
// The script NEVER touches the Rust side. If themes/generated/tokens.css is
// stale, run `cargo xtask tokens` first (crate 1.B4 owns that).

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "..");
const crateCssPath = resolve(uiRoot, "..", "themes", "generated", "tokens.css");
const outCssPath = resolve(uiRoot, "src", "tokens", "generated", "tokens.css");
const outTsPath = resolve(uiRoot, "src", "tokens", "generated", "tokens.ts");

// Doc 08 §3.3: dark is the default mode, yellow the default accent (Solstice
// design language, 2026-11-08 — Keyvox gold). These are product constants
// (product.toml brand fields) — the non-default values are parsed from the
// CSS selectors below.
const DEFAULT_MODE = "dark";
const DEFAULT_ACCENT = "yellow";

async function main() {
  const check = process.argv.includes("--check");
  const css = await readFile(crateCssPath, "utf8");

  const declRe = /--[A-Za-z0-9-]+(?=\s*:)/g;
  const blockRe = /(:root(?:\[[^\]]+\])*)\s*\{([^}]*)\}/g;

  const semantic = [];
  const modes = new Set([DEFAULT_MODE]);
  const accents = new Set([DEFAULT_ACCENT]);

  let match;
  while ((match = blockRe.exec(css)) !== null) {
    const selector = match[1];
    const body = match[2];
    const theme = selector.match(/\[data-theme="([^"]+)"\]/);
    const accent = selector.match(/\[data-accent="([^"]+)"\]/);
    if (theme) modes.add(theme[1]);
    if (accent) accents.add(accent[1]);
    if (selector === ":root") {
      for (const name of body.match(declRe) ?? []) {
        if (!semantic.includes(name)) semantic.push(name);
      }
    }
  }
  semantic.sort();

  if (semantic.length === 0) {
    console.error(`gen-tokens: no :root declarations found in ${crateCssPath}`);
    process.exit(1);
  }

  const sha256 = createHash("sha256").update(css, "utf8").digest("hex");
  const modeList = [...modes].sort();
  const accentList = [...accents].sort();

  const ts = renderTs({ sha256, modeList, accentList, semantic, css });

  if (check) {
    const [oldCss, oldTs] = await Promise.all([
      readFile(outCssPath, "utf8").catch(() => null),
      readFile(outTsPath, "utf8").catch(() => null),
    ]);
    let drift = false;
    if (oldCss !== css) {
      console.error(
        `gen-tokens --check: ${outCssPath} differs from ${crateCssPath} — run \`npm run tokens:generate\``,
      );
      drift = true;
    }
    if (oldTs !== ts) {
      console.error(`gen-tokens --check: ${outTsPath} is stale — run \`npm run tokens:generate\``);
      drift = true;
    }
    if (drift) process.exit(1);
    console.log(
      `gen-tokens --check: OK (${semantic.length} semantic tokens, ${modeList.length} modes, ${accentList.length} accents, sha256 ${sha256.slice(0, 12)}…)`,
    );
    return;
  }

  await mkdir(dirname(outCssPath), { recursive: true });
  await writeFile(outCssPath, css, "utf8");
  await writeFile(outTsPath, ts, "utf8");
  console.log(
    `gen-tokens: wrote ${semantic.length} semantic tokens, ${modeList.length} modes, ${accentList.length} accents (sha256 ${sha256.slice(0, 12)}…)`,
  );
}

function renderTs({ sha256, modeList, accentList, semantic, css }) {
  const q = (s) => `"${s}"`;
  return `// GENERATED FILE — DO NOT EDIT.
//
// Produced by ui/scripts/gen-tokens.mjs (card 4.A2) from
// themes/generated/tokens.css — the committed output of the Rust
// crates/theme-tokens pipeline (card 1.B4, \`cargo xtask tokens\`).
// Regenerate with \`npm run tokens:generate\` in ui/. Drift is caught by
// src/tokens/tokens.test.ts (pinned sha256 + structure) and
// \`npm run tokens:check\`.
//
// Components consume SEMANTIC tokens only, via \`var(--…)\` in CSS or
// \`tokenVar("…")\` for inline styles (doc 08 §3.2). Raw hex/rgb/px values
// are rejected by the local/semantic-tokens-only lint rule.

/** SHA-256 of themes/generated/tokens.css at generation time. */
export const TOKENS_CSS_SHA256 = ${q(sha256)};

/** The full generated token CSS — same bytes as tokens.css, carried here so
 * tests can pin structure + hash without the filesystem (vitest stubs raw
 * .css imports). */
export const TOKENS_CSS = ${JSON.stringify(css)};

/** Theming modes (doc 08 §3.3). \`dark\` is the product default. */
export type TokenMode = ${modeList.map(q).join(" | ")};
export const TOKEN_MODES: readonly TokenMode[] = [${modeList.map(q).join(", ")}];

/** The 9 accent selections (doc 08 §3.3). \`orange\` is the product default. */
export type AccentId = ${accentList.map(q).join(" | ")};
export const ACCENTS: readonly AccentId[] = [${accentList.map(q).join(", ")}];

/** Every semantic CSS custom property name emitted in the \`:root\` block. */
export type SemanticTokenName = ${semantic.map(q).join(" | ")};
export const SEMANTIC_TOKEN_NAMES: readonly SemanticTokenName[] = [
${semantic.map((n) => `  ${q(n)},`).join("\n")}
];

/** Build a \`var(--…)\` reference for a semantic token (doc 08 §3.2). */
export function tokenVar(name: SemanticTokenName): string {
  return \`var(\${name})\`;
}
`;
}

main().catch((err) => {
  console.error(`gen-tokens: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});
