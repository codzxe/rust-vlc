// eslint-plugin-local — issue #36 (UI lint hardening).
//
// The three scaffold-local rules that used to live inline in
// ui/eslint.config.js, promoted to a tested package (single source of
// truth). ui/eslint.config.js imports this module and registers it under
// the `local` plugin key, so the rule names below are addressed as
// `local/<name>` exactly as before:
//
//   * i18n-no-literals     — user-visible copy must come from useMsg() +
//                            MessageKey (doc 07 §6.5)
//   * boundary-imports     — src/ modules may not import outside ui/src
//                            (doc 08 §8)
//   * semantic-tokens-only — colors / font-size / radius dimensions must be
//                            var(--semantic-token) in TSX (doc 08 §3.2)
//
// Unit tests live next to each rule (rules/*.test.js, vitest) and are wired
// into `npm run test` via the vitest include glob in vite.config.ts.

import boundaryImports from "./rules/boundary-imports.js";
import i18nNoLiterals from "./rules/i18n-no-literals.js";
import semanticTokensOnly from "./rules/semantic-tokens-only.js";

export default {
  rules: {
    "boundary-imports": boundaryImports,
    "i18n-no-literals": i18nNoLiterals,
    "semantic-tokens-only": semanticTokensOnly,
  },
};
