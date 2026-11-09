// Stylelint config for the ui/ workspace — CSS token discipline (issue #36,
// doc 08 §3.2/§3.4; the TSX side of the discipline is the
// local/semantic-tokens-only ESLint rule).
//
// Base: stylelint-config-standard. On top of it, hand-written CSS may express
// colors ONLY through semantic tokens:
//   * `color-no-hex`            — no raw #hex anywhere.
//   * `function-disallowed-list`— no color-building functions (rgb()/rgba()/
//     hsl()/hwb()/lab()/lch()/oklab()/oklch()/color()). color-mix() is
//     deliberately ALLOWED: composing var(--token)s (e.g. translucency) is
//     still token discipline; only constructing raw colors is not.
//   * `color-named`             — no named color keywords (transparent is
//     exempt by the rule itself).
//
// Known exceptions are NARROW, INLINE and carry a reason at the use site
// (art-direction values with no semantic token yet — veil gradients, tape
// shadows, white highlight blends). [PROPOSAL] the stricter form — zero raw
// colors, no exceptions — becomes enforceable once the Rust-owned token
// pipeline (themes/tokens.json → cargo xtask tokens) grows semantic tokens
// for the veil/scrim ink, the tape shadow and the white highlight; then the
// inline disables below come off.
//
// Resolving stylelint-config-standard: the canonical location is
// ui/node_modules (real installs, CI). As a fallback for npx-style
// invocations (where stylelint + this config standard share an npx package
// set and ui/node_modules — a shared symlink without stylelint — cannot see
// them), it is resolved as a sibling of the RUNNING stylelint CLI. The
// resolved entry FILE is what `extends` receives (absolute paths are
// supported) so the config standard's own nested extends resolve from ITS
// directory, not ours.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

function resolveConfigStandardPath() {
  try {
    return require.resolve("stylelint-config-standard");
  } catch {
    const requireBesideRunningStylelint = createRequire(process.argv[1]);
    return requireBesideRunningStylelint.resolve("stylelint-config-standard");
  }
}

export default {
  extends: [resolveConfigStandardPath()],
  rules: {
    // --- token discipline (issue #36) --------------------------------------
    "color-no-hex": true,
    "function-disallowed-list": [
      "/^rgb/",
      "/^hsl/",
      "/^hwb/",
      "/^lab/",
      "/^lch/",
      "/^oklab/",
      "/^oklch/",
      "color",
    ],
    "color-named": "never",
  },
  overrides: [
    {
      // The tokens definition file: src/tokens/generated/tokens.css is
      // EMITTER OUTPUT (verbatim copy of themes/generated/tokens.css,
      // produced by the Rust pipeline via ui/scripts/gen-tokens.mjs and
      // drift-checked byte-for-byte by `npm run tokens:check`). Hand-editing
      // an inline stylelint-disable into it would break the drift guard, so
      // the scoped exemption lives HERE instead — this override is the
      // config-level equivalent of the blanket `/* eslint-disable */` the
      // generators emit for ESLint, scoped to the color discipline only:
      // tokens.css is the one file ALLOWED to spell raw colors, because
      // defining them is its entire job.
      files: ["src/tokens/generated/tokens.css"],
      rules: {
        "color-no-hex": null,
        "function-disallowed-list": null,
        "color-named": null,
      },
    },
  ],
};
