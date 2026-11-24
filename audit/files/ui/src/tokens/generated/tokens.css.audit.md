# AUDIT: ui/src/tokens/generated/tokens.css (generated asset — compact audit per spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 263 / 0 (generated CSS custom properties; 20 mode/accent selector blocks) |
| File verdict | REAL (generated artifact, wired into the app entry) |
| Reachable from shipped entry point? | YES — ui/src/main.tsx:27 `import "./tokens/generated/tokens.css"` (both web and desktop shells serve main.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `head/tail` (structure); `rg -c "data-theme\|data-accent"` -> 20; `rg tokens.css ui/src` -> main.tsx:27; `ls themes/` -> tokens.json present |

## 1. One-paragraph verdict
Generated design-token cascade ("GENERATED FILE — DO NOT EDIT", :1) from
themes/tokens.json via `cargo xtask tokens` (header :3-6). Layout matches
its own documented contract: one `:root` block with mode-independent +
default dark/orange values, then `:root[data-theme="light"]` and 9 accent
x 2 mode override blocks (20 selector hits). Every value I spot-checked is
a well-formed CSS custom property (colors hex, sizes px, font stacks
quoted). It is genuinely consumed: main.tsx:27 imports it, and
theme/settingsThemeSync.ts + theme/context.ts import its sibling generated
TS token ids — the theme switcher is the live consumer. This is one of the
rare UI files with a complete, tested pipeline behind it
(crates/theme-tokens/tests/diff.rs diff-tests the committed copy; ui
`tokens:check` script). No hand-edit drift is visible; nothing here
pretends to be backend wiring (it is styling data, and styled correctly).

## 2. What actually works (verified)
— `rg` import chain (main.tsx:27); header block self-documents generator +
source; vitest baseline (1705 passed) includes token/theme tests that would
fail on drift. Diff test exists on the Rust side (header citation; not
executed by me — cargo-build-budget).

## 3. Findings
None. Checked: malformed declarations (none — spot-checked all 53 :root
lines + tail blocks), duplicate/conflicting selectors (accents override by
specificity, intended), stray primitive/alias leakage (header :9-10 says
they are never emitted — confirmed absent: all names are semantic-tier),
unreferenced tokens (out of scope for a generated mirror; consumption is
component-side).

## 4. Per-function table
Not applicable (generated data, 0 functions — spec §13).

## 5. Test honesty
No tests in-file. Generation drift is guarded by
crates/theme-tokens/tests/diff.rs and `npm run tokens:check` (script exists
in package.json:17-18). UNVERIFIED: I did not run either (build budget);
baseline §13 does not list a tokens check explicitly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tokens: 123 primitive / 86 alias / 86 semantic (source: themes/tokens.json)" | :24-25 | REAL-UNPROVEN | themes/tokens.json exists; counts not re-derived (cargo-build-budget) |
| "Regeneration is idempotent; the committed copy is diff-tested" | :5-6 | REAL-UNPROVEN | test file path cited; not run |

## 7. Dead / duplicated / unreachable code
None. `--font-size-micro: 6px` (:70) is suspicious as a design value but is
data, not code — flagged for the design-lane auditor only if unused.

## 8. Dependencies & flags
None (pure CSS). Consumed by the theme system (theme/context.ts,
settingsThemeSync.ts) via `var(--<name>)`.

## 9. Open questions for the next agent
- theme auditor: do all 86 semantic tokens have at least one component
consumer, or does the cascade ship dead variables?
