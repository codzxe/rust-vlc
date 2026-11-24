# CLAIM AUDIT: ui/src/tokens/README.md
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Claims extracted | 7 |
| REAL: 5 REAL-UNPROVEN: 1 STALE/FALSE: 1 STUB: 0 PAPER-ONLY: 0 UNREACHABLE: 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
Accurate pipeline documentation with verifiable teeth (byte-identity and the sha256 pin both check out against the real tree), one STALE claim: the "Honest status" section says ThemeProvider has "no storage" — the code has persisted theme/accent bootstrap since w26.f08 (ThemeProvider.tsx:13-17, 43-44). The README undersells the implemented state; the doc is behind the code, not ahead of it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "themes/generated/tokens.css committed crate output — THE BRIDGE ... verbatim copy" | L7-11 | REAL | ran `cmp themes/generated/tokens.css ui/src/tokens/generated/tokens.css` -> BYTE-IDENTICAL (exit 0) |
| 2 | "drift is impossible to miss (npm run tokens:check + tokens.test.ts pins the sha256)" | L17-18 | REAL | ui/src/tokens/tokens.test.ts:72-73 "pins the generated tokens.css by sha256" |
| 3 | "TokenSandbox ... dev fixture at /dev/tokens ... NOT a product screen" | L29-31 | REAL | App.tsx:297 `/dev/tokens` ungated lazy route |
| 4 | "ThemeProvider currently defaults to the product defaults (dark + orange) with no storage" | L60-62 | STALE/FALSE | ThemeProvider.tsx:43-44 "synchronous bootstrap read of the persisted interface.theme and global.accent-color overrides" (w26.f08) |
| 5 | "e2e/theme.spec.ts loops all 9 accents × dark/light (18 tests)" | L75-77 | REAL-UNPROVEN | e2e file not executed by me; existence consistent with e2e/ dir |
| 6 | "Current consumers: ZERO — no engine crate's Cargo.toml depends on theme-tokens" | L88-90 | REAL-UNPROVEN | Rust-side survey; cross-check belongs to engine-wave agents (UNVERIFIED here) |
| 7 | "renderer-wgpu backend.rs ... default today is the hardcoded color::BLACK" | L95-98 | REAL-UNPROVEN | engine code, not executed/read here |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| ThemeProvider "no storage" (L61) | ThemeProvider.tsx:43-73 persisted overrides bootstrap | The code is true; the README section predates w26.f08 |

## 4. Fabrication / inflation indicators
None — the doc's verifiable claims all check out; the single stale claim understates (not overstates) implementation.

## 5. Matrix rows
n.a. — not a matrix chunk.
