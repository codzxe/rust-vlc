# AUDIT: ui/stylelint.config.js
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | ~105 / 1 (config default) + resolver fn |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (dev gate) — consumed by scripts/lint-css.mjs (`npm run lint:css`), stylelint 18.0.0 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; cross-check against audited CSS files (hex-in-comments pattern, BEM classes); attempted `npx stylelint` (failed: no node_modules — exit 2) |

## 1. One-paragraph verdict
A serious, well-reasoned token-discipline gate: raw hex banned, all color-building functions banned (color-mix allowed for token composition), named colors banned, with exactly ONE exemption scoped to the emitter output (tokens.css) and a documented reason why the exemption lives in the config (inline disables would break the byte-for-byte drift guard). The BEM class pattern is calibrated to the real codebase with a [PROPOSAL] to tighten. (An earlier draft of this audit predicted a lint failure from allegedly "corrupted" `[hidden]` selectors — that was a display artifact of the tool output sanitizer and was retracted; the CSS corpus is intact per fixed-string counts.)

## 2. What actually works (verified)
— verified by cross-check: every hand-written CSS file I audited (appshell.css, setting-field.css, icon.css) keeps raw hex inside comments only and BEM classes matching the pattern — the discipline is real in the corpus.
— NOT verified: an actual lint:css run (no node_modules; npx resolution failed exit 2). UNVERIFIED whether the gate currently passes.

## 3. Findings
F-001 [S4]
Where: ui/stylelint.config.js:38-45 (resolver fallback)
What is wrong: `process.argv[1]`-relative fallback resolution is brittle (yarn/pnpm shim layouts), and the whole dance exists only because node_modules is absent in some sandboxes — a packaging smell, not a config defect. Confidence: LOW — Verification: read.

## 4. Per-function table
| Element | Lines | Verdict | Notes |
|---|---|---|---|
| resolveConfigStandardPath | 36-45 | REAL | dual resolution documented |
| rules (token discipline) | 48-58 | REAL | color-no-hex, function-disallowed-list, color-named |
| selector-class-pattern | 60-77 | REAL | BEM-calibrated regex |
| tokens.css override | 80-105 | REAL | single scoped exemption, justified |

## 5. Test honesty
N/A (config). The gate itself is the test; its current pass/fail state is UNVERIFIED (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error" | header L19-20 | REAL | my CSS audits found zero inline disables and zero raw hex outside comments |
| "the ONLY exemption is the scoped tokens.css override" | header L15-16 | REAL | overrides[] contains exactly one entry |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
stylelint-config-standard (resolved at runtime). No env vars read.

## 9. Open questions for the next agent
- Run `npm run lint:css` in an installed environment: does it fail on the 8 corrupted `idden]` selectors? If it passes, stylelint 18 tolerates unmatched `]` in these rule positions — worth documenting either way.
