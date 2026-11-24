# AUDIT: crates/theme-tokens/src/generated/tokens.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 247 / 0 (generated constants: 4 meta + 110 semantic + 72 accent-ramp) |
| File verdict | UNREACHABLE (generated data, in sync and honest, zero consumers outside its own crate) |
| Reachable from shipped entry point? | NO — no crate depends on theme-tokens. `rg theme_tokens` over apps/, tests/, ui/ sources: 0 matches; crates/commands/Cargo.toml:29 and crates/settings-schema/Cargo.toml:19 mention theme-tokens in COMMENTS only, not as dependency entries. The token system's live half is the CSS copy (`themes/generated/tokens.css` + ui bridge) — the Rust constants are the dead half. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p theme-tokens --tests` -> 22 lib + 2 diff + 12 pipeline + 2 semantic_lint passed, 0 failed (doctest run blocked by sandbox /tmp StorageFull — environment limit, UNVERIFIED); python3 token-count of themes/tokens.json -> primitive 123 / alias 86 / semantic 86 (header claim exact); rg unit cross-check CSS vs Rust |

## 1. One-paragraph verdict
The file is what it says it is: a faithfully generated, drift-gated constants table. The header's counts are exact (python3 count of themes/tokens.json: 123/86/86 — byte-for-byte the header claim), the drift gate is real and self-proving (`committed_outputs_match_regeneration` re-emits from source and compares byte equality; `the_comparison_detects_drift` mutates one token and proves the comparison is not vacuous — both passed in my run), and internal cross-references are consistent (COLOR_ACCENT == ACCENT_ORANGE_PRIMARY, COLOR_EQ_1..7 == the seven non-default accent primaries, COLOR_RECORD == red primary). But the header's purpose statement — "Semantic-layer constants for engine-side consumption" — is false in practice: not one engine, app, or UI file consumes these constants; no crate depends on theme-tokens at all. The Rust half of the design-token pipeline is dead code with excellent tests.

## 2. What actually works (verified)
— `cargo test -p theme-tokens --tests` -> 38 passed total, 0 failed, incl. `committed_outputs_match_regeneration ... ok` (proves this committed file is byte-identical to a fresh emission from themes/tokens.json) and `the_comparison_detects_drift ... ok` (mutation sensitivity proof).
— python3 JSON count: primitive 123 / alias 86 / semantic 86 == header L21 claim, exact.
— Arithmetic consistency: 186 `pub const` = 4 meta (MODES/DEFAULT_MODE/ACCENTS/DEFAULT_ACCENT) + 72 accent-ramp (9 accents × 8 roles, matches header L15-19) + 110 semantic (86 tokens, 24 of them per-mode expanding to DARK/LIGHT pairs — 86+24=110, consistent).
— Cross-constant consistency (read): COLOR_ACCENT 0xd37918 == ACCENT_ORANGE_PRIMARY; COLOR_EQ_1..7 == RED/YELLOW/GREEN/CYAN/BLUE/VIOLET/PINK PRIMARY; COLOR_RECORD == RED PRIMARY.

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/src/generated/tokens.rs:8-11 (claim) + repo-wide dependency graph (contradiction)
Code: (verbatim, L8-11)
```
//! Semantic-layer constants for engine-side consumption (doc 01 §2.3,
//! doc 08 §3.1). Components and engine code consume SEMANTIC tokens
//! only (doc 08 §3.2) — the primitive and alias layers are never
//! referenced directly (`theme_tokens::lint` checks this).
```
What is wrong: "engine-side consumption" does not exist. No Cargo.toml declares a dependency on theme-tokens (the two manifest hits — crates/commands/Cargo.toml:29, crates/settings-schema/Cargo.toml:19 — are prose comments about preaudit families, not dependency entries); `rg theme_tokens` over apps/, tests/, ui/ sources returns 0 matches; renderer-wgpu and video-engine manifests have no theme reference. The only consumers of these constants are theme-tokens' own tests and its generator. Why it matters: 247 lines of "engine-side" constants with a lint enforcing a consumption rule that no consumer can violate; the file's stated reason to exist is currently unsatisfied (the UI consumes the CSS bridge copy, not these). What correct looks like: either wire engine-side rendering to these constants or reclassify the crate as UI-support/tooling-only and fix the header. Confidence: HIGH — Verification: read + rg (repo-wide, no fabrication).
F-002 [S4]
Where: crates/theme-tokens/src/generated/tokens.rs:47-48, 108-110, 123-125
Code: (verbatim, L108-110 + L123-125)
```
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_CAPTION: f32 = 11.0;
pub const FONT_SIZE_DISPLAY: f32 = 32.0;
pub const MOTION_DURATION_FAST: f32 = 120.0;
pub const MOTION_DURATION_SLOW: f32 = 480.0;
pub const MOTION_DURATION_STEP: f32 = 280.0;
```
What is wrong: the CSS twin carries units (`--motion-duration-fast: 120ms`, `--radius-app: 19px`, `--blur-backdrop: 25px` — themes/generated/tokens.css L81/L93/L29) while the Rust constants are bare `f32` with no unit in the name, type, or doc. A future engine consumer must know px/ms by convention — the classic wrong-units trap (checklist 6.2). What correct looks like: typed wrappers (e.g. `Millis(f32)`, `Px(f32)`) or `_PX`/`_MS` suffixes in the emitter. Confidence: HIGH — Verification: read + rg on tokens.css.

## 4. Per-section inventory (generated constants file — no functions; constant-group table)
| Group | Count | Verified | Notes |
|---|---|---|---|
| Meta (MODES, DEFAULT_MODE, ACCENTS, DEFAULT_ACCENT) | 4 | read | 2 modes, 9 accents; `#[rustfmt::skip]` on ACCENTS (single-line-per-accent layout) |
| Semantic: color | 38 | read + cross-ref | COLOR_ACCENT==orange primary; EQ 1-7==accent primaries; RECORD==red |
| Semantic: font | 12 | read | families as `&[&str]` stacks; sizes 6-32; weights 400-800 |
| Semantic: motion/opacity/radius/space/stroke/blur/shadow | 60 | read | OPACITY 0.14-1.0 sane; RADIUS 1-30; SHADOW_* structs |
| Accent ramps (9 accents × 8 roles) | 72 | read | header L16-19 names all 8 roles; ON_ACCENT pairs plausible (dark text on light accents, white on violet) — WCAG ratios NOT computed (UNVERIFIED) |

## 5. Test honesty
No tests in this file. The guarding tests (run by me):
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| diff.rs `committed_outputs_match_regeneration` | YES | re-parses themes/tokens.json, re-emits, byte-compares against THIS file |
| diff.rs `the_comparison_detects_drift` | YES | mutates one `$value` and proves the emission changes — the equality test cannot be vacuous |
| pipeline.rs (12) / semantic_lint.rs (2) / lib (22) | YES (per their own assertions; not re-read line-by-line — scope: this file) | all passed in my run |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Produced by `cargo xtask tokens` ... Regeneration is idempotent; the committed copy is diff-tested" (L3-6) | header | REAL | diff.rs tests passed; `--check` contract in tokens_gen.rs |
| "Semantic-layer constants for engine-side consumption" (L8) | header | FALSE (no consumer) | F-001: no dependents, rg 0 matches |
| "Accents (9) ... eight emitted roles" (L15-19) | header | REAL | 72 = 9×8 constants present |
| "Source shape: 123 primitive / 86 alias / 86 semantic tokens" (L21) | header | REAL | python3 count of themes/tokens.json: 123/86/86 |

## 7. Dead / duplicated / unreachable code
- The entire Rust-constants output is unreachable from any shipped binary (F-001). Token data exists in four places (tokens.json source, themes/generated/tokens.css, this file, ui/src/tokens/generated/tokens.css bridge) — the CSS triplication is drift-gated; the Rust duplication is gated but unused.

## 8. Dependencies & flags
File depends only on `crate::color::{Rgb8, Shadow}` (crate-internal). No features, no unsafe, no tests in-file. Crate deps: serde_json (emit/parse) per Cargo.toml.

## 9. Open questions for the next agent
- renderer-wgpu/video-engine auditors: confirm no engine path was ever supposed to read these in v1 (doc 08 §3.2 "components consume semantic tokens only" — the components are React/TSX and consume the CSS copy; was a Rust consumer ever planned?).
- If the Rust constants stay dead, diff-gating an unused file is pure maintenance overhead — Lead decision.
- WCAG contrast of ON_ACCENT pairs unverified (I did not compute ratios) — owner: design/figma auditors.
