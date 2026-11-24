# AUDIT: ui/eslint.config.js
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 116 / 0 (flat config; 7 config blocks) |
| File verdict | REAL (working config backed by a tested local plugin; gated in CI) |
| Reachable from shipped entry point? | YES as tooling — `npm run lint` (package.json:14) and CI .github/workflows/ui.yml:102-105 run it; not shipped in the bundle (dev config) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `ls ui/tools/eslint-plugin-local/` -> index.js + 3 rules + 3 tests; `cat index.js` -> registers all 3; `rg lint .github/workflows/ui.yml` -> :102-105 `npm run lint` gated; baseline full_gate.txt:59-61 lint-literals FAIL is the RUST-side scan (apps/cli/src/host.rs:1799), NOT this config |

## 1. One-paragraph verdict
A real, maintained flat config, not scaffold decoration: the three
governance rules (i18n-no-literals, boundary-imports, semantic-tokens-only)
live in a promoted, unit-tested package (ui/tools/eslint-plugin-local with
rules/*.test.js wired into vitest), all three are set to "error" for
src/**/*.{ts,tsx}, and CI actually runs `npm run lint`
(.github/workflows/ui.yml:102-105). The honest gaps: two react rules ship
as "warn" (never gate anything), and the per-environment globals blocks are
hand-curated allowlists that silently exempt scripts/ and e2e/ from the
local governance rules entirely.

## 2. What actually works (verified)
— Plugin package exists with tests for every rule (ls + cat). Rule
registration verified in index.js (all 3 imported and exported).
— CI gating verified by rg (ui.yml:102-105). Baseline rc-smoke/web-ui-build
PASS covers build; lint itself not re-run by me (UNVERIFIED — npm-budget;
CI comment claims "0 errors, 1 accepted").

## 3. Findings
F-001 [S3]
Where: ui/eslint.config.js:111-113
Code:
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "react-refresh/only-export-components": "warn",
What is wrong: `exhaustive-deps` at "warn" means stale-closure/effect-drift
bugs (exactly the class that silences event subscriptions in this codebase
— see DiscoveryScreen/MusicMode useEffects) never fail a gate; in a repo
whose full gate is already red in 7 sections, warning output is scroll-
past material. What correct looks like: "error" with a suppression count,
or a CI `-max-warnings 0`. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/eslint.config.js:32-55
Code:
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { console: "readonly", process: "readonly" } },
  },
  {
    // e2e/web/site/*.js — the browser-matrix harness page (card 7.I1,
What is wrong: the local governance rules are only attached to
`src/**/*.{ts,tsx}` (:102) — scripts/ and e2e/ files are parsed for globals
but exempt from i18n/boundary/token discipline. Defensible for tooling, but
the boundary rule (ui/src may not import outside ui/src) has no teeth
anywhere outside src. Confidence: HIGH — Verification: read.

## 4. Per-block table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| ignores | 17-29 | REAL | dist/node_modules/coverage + web-matrix staging (gitignored too) |
| scripts globals | 30-34 | REAL | node flavor |
| e2e site globals | 35-56 | REAL | browser globals enumerated |
| web-matrix-run globals | 57-64 | REAL | +URL |
| l10n-pseudo-probe globals | 65-86 | REAL | documented hygiene fix (P4-GATE) |
| generated output | 89-100 | REAL | reportUnusedDisableDirectives off with ownership note |
| src rules | 101-115 | REAL | 3 local errors + react (F-001 warns) |

## 5. Test honesty
The plugin's rules have vitest suites (rules/*.test.js, wired into
`npm run test` per index.js comment) — included in the 1705-pass baseline.
The config file itself is exercised by every CI lint run; not executed
locally (UNVERIFIED — npm-budget).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "promoted verbatim from this file, unit-tested via vitest" | :4-6 | REAL | rules/*.test.js exist (ls) |
| CI "lint (0 errors, 1 accepted" | ui.yml:8 | REAL-UNPROVEN | CI comment; not re-run here |

## 7. Dead / duplicated / unreachable code
None. Every block targets existing paths (scripts/, e2e/web/site/ verified
present in the workspace).

## 8. Dependencies & flags
@eslint/js, typescript-eslint, react-hooks/react-refresh plugins, local
plugin via relative import. No flag gating.

## 9. Open questions for the next agent
- eslint-plugin-local rules auditor: do the 3 rule implementations actually
  walk JSX text/attributes (i18n), import specifiers (boundary), and style
  props (tokens) — or are they regex toys? Their tests will say.
