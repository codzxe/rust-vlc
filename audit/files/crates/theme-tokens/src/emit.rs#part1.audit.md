# AUDIT: crates/theme-tokens/src/emit.rs#part1 (lines 1-405, CSS emission)
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 405 of 814 (chunk part1 = CSS emission; part2 = Rust emission, lines 407-814 → A06) / 12 fns (emit_css, emit_rust, css_document, render_block, ramp_color, ramp_for, role_is_per_mode, validate_mode_pair, css_value, css_num, css_num32, css_shadow_color, css_family, list_with_default, css_header) |
| File verdict | REAL |
| Reachable from shipped entry point? | BUILD-TIME TOOLCHAIN (not in lumen binary): `cargo xtask tokens` → tools/xtask/src/cmd/tokens.rs → tokens_gen bin → emit_css/emit_rust (rg callers verified). OUTPUT IS SHIPPED: themes/generated/tokens.css + ui/src/tokens/generated/tokens.css bridge copy ride the UI bundle. Drift gate tests/diff.rs byte-compares committed outputs against fresh emission in `cargo test` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg emit_css/emit_rust callers (lib.rs, bin/tokens_gen.rs, tests/{pipeline,diff}.rs); sed parse.rs:600-640 (accent_alias_roles) + 640-660 (ramp_from_file vs derive_ramp); sed color.rs:200-285 (AccentRamp field docs, derive_ramp: bright_dark = bright_light); rg tokens.css blocks (:root/data-theme/data-accent) + accent-bright values (#e8851a dark vs #b35f0d light for orange; identical #e66044 both modes for red); ls tools/xtask/src/cmd (tokens.rs); baseline INDEX.md #3/#24 |

## 1. One-paragraph verdict
The CSS emitter is real, deterministic, and — rare in this repo — EXECUTION-
VERIFIED: baseline `cargo test --workspace` (1681 passed, INDEX.md #3)
includes tests/diff.rs, which byte-compares the committed tokens.css against
a fresh `emit_css` run, and `npm run tokens:check` passed (INDEX.md #24).
Duplicate-property detection, unregistered-mode rejection, and the
dark/light-only model check are all defensive and correct. The two findings
are documentation-level: a stale comment describing the pre-w26 accent-bright
model (the code and the color.rs field docs moved on), and a cross-file
positional invariant (ROLE_* constants ↔ parse.rs table order) held together
only by a comment — though the diff gate would catch a silent reorder by
byte drift.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/test-workspace.log.txt (1681
passed / 1 failed / 24 ignored — the failure is apps/desktop zero-alloc,
unrelated): tests/diff.rs ran green ⇒ committed tokens.css IS byte-identical
to fresh emit_css output; pipeline.rs `emission_is_deterministic` and
`emitted_documents_have_the_documented_structure` executed.
— verified by read: determinism mechanics hold — decls sorted per block
(render_block:197), duplicates rejected (198-205), block order = meta order
(modes()/accents() from tokens.json), fixed number forms (css_num/css_num32),
final-newline normalization (`document.pop()` after "}\n\n" — 190-192).
— verified by read: ramp_color role/mode matrix (221-236) covers all 6 roles
× 2 modes with a typed error arm; cross-checked emitted values: orange
bright dark #e8851a vs light #b35f0d (tokens.css:32/118) matches
mined-ramp-from-file (parse.rs:646-649); red bright identical both modes
(tokens.css:146/154) matches derive_ramp's bright_dark = bright_light
(color.rs:278).

## 3. Findings
### F-001 [S4] Stale spec comment: "dark-mode bright/icon equal the primary" — bright does not
Where: crates/theme-tokens/src/emit.rs:214-215
Code:
```
/// The accent-family color for `role` under `mode` (doc 08 §3.3): dark-mode
/// bright/icon equal the primary; light-mode values come from the ramp.
```
What is wrong: The comment describes the pre-w26 model. The code below it
maps `(ROLE_BRIGHT, "dark") => ramp.bright_dark` (line 226), and color.rs's
own field doc says bright_dark is a hover-bright variant whose previous
equality-with-primary was deliberately REMOVED ("dark hover previously
equaled the primary, an invisible hover state", color.rs:222-224). Emitted
proof: orange dark bright #e8851a ≠ primary. Only ICON still equals primary
in dark (line 225).
Why it matters: This comment is the reader's spec for the whole ramp matrix;
it misstates one of six roles and contradicts the sibling crate's docs.
What correct looks like: "dark-mode icon equals the primary; bright is
per-mode (bright_dark/bright_light); strong/on-accent are mode-independent."
Confidence: HIGH — Verification: read + emitted values cross-checked.
### F-002 [S4] ROLE_* constants are a positional mirror of parse.rs's table — invariant held only by a comment
Where: crates/theme-tokens/src/emit.rs:25-34
Code:
```
/// Role order of [`TokenSet::accent_family_css`], matching the
/// `accent_alias_roles` table in `parse.rs`:
/// `color.accent`, `color.accent.bright`, `color.accent.icon`,
/// `color.accent.strong`, `color.accent.tinted`, `color.content.on-accent`.
const ROLE_ACCENT: usize = 0;
```
What is wrong: ramp_color interprets `role` POSITIONALLY against
accent_family_css, whose order comes from parse.rs:606-613's string table.
Reordering that table silently swaps which ramp field each CSS name receives
(bright↔icon etc.) — the compiler cannot catch it.
Why it matters: Mitigated in practice: tests/diff.rs byte-drift would fail
on the next `cargo test` after such a reorder (committed outputs no longer
match), so the safety net is real but indirect — nothing fails AT the
reorder site.
What correct looks like: An enum `AccentRole` shared by parse.rs and emit.rs
(parse builds it, emit matches on it) — order becomes a type, not a comment.
Confidence: HIGH — Verification: read both sites; diff.rs gate read.

## 4. Per-function table (part1)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| emit_css / emit_rust | 62-74 | REAL | — | thin Write wrappers, io errors mapped to TokenError::io |
| css_document | 83-193 | REAL | — | verified via baseline diff gate; unregistered-mode per-mode value → typed error (115-119); final pop() safe (header + ≥1 block always emitted) |
| render_block | 196-212 | REAL | — | sort + windows(2) duplicate check — correct |
| ramp_color | 216-238 | REAL | S4 (F-001 comment) | full role×mode matrix, typed fallthrough error |
| ramp_for | 240-244 | REAL | — | typed error for missing ramp |
| role_is_per_mode | 248-253 | REAL | — | gates mode-block accent entries; unwrap_or(false) honest |
| validate_mode_pair | 257-269 | REAL | — | exactly dark+light enforced — the model's documented constraint |
| css_value | 272-298 | REAL | — | empty font stack rejected (288-290); shadow → 4×px + color |
| css_num / css_num32 | 302-318 | REAL | — | integral-shortest forms; f32 path separate (comment explains why) |
| css_shadow_color | 320-332 | REAL | — | alpha≥1 → hex else rgba |
| css_family | 334-340 | REAL | — | generic keywords unquoted — correct CSS semantics |
| list_with_default | 343-354 | REAL | — | Cow, marks "(default)" |
| css_header | 356-405 | REAL | — | {dm}/{da} templating via replacen; counts from TokenSet |

## 5. Test honesty (tests/diff.rs + tests/pipeline.rs — this crate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| diff.rs byte-equality (all 3 outputs) | YES | any emitter change without repinning committed files fails `cargo test` — executed in baseline #3 |
| pipeline.rs emission_is_deterministic | YES | double-emits and byte-compares |
| emitted_documents_have_the_documented_structure | YES | asserts block layout against doc 08 §3.3 shape |
| accent_ramps_follow_the_contrast_rule | YES | contrast validation of derived ramps |
Mutation check: swap ROLE_BRIGHT/ROLE_ICON constants → emitted css names get
swapped values → diff.rs fails at first differing line. The net holds.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "same TokenSet always renders byte-identical output" (doc 08 §3.4) | header 13-16 | REAL | determinism mechanics verified by read; pipeline test executes it |
| "dark-mode bright/icon equal the primary" | emit.rs:214-215 | FALSE (stale) | F-001: code + color.rs:222-224 + emitted #e8851a |
| "committed copy is diff-tested by crates/theme-tokens/tests/diff.rs" | css_header:365-366 | REAL | test file exists and ran green in baseline #3 |

## 7. Dead / duplicated / unreachable code
None in part1. css_num/css_num32 are near-duplicates by necessity (f32 vs
f64 formatting — documented at 310-312).

## 8. Dependencies & flags
crate-internal (TokenError, color::AccentRamp, model::TokenSet). No feature
flags. Output flows: themes/generated/tokens.css (crate) and
ui/src/tokens/generated/tokens.css (bridge copy via ui/scripts/gen-tokens.mjs)
— the UI consumes only semantic tokens (verified in my onboarding.css audit:
all var() names resolve).

## 9. Open questions for the next agent (part2, A06)
- Does rust_document (part2) reuse ROLE_* the same positional way? Same
  invariant question applies there.
- The derived ramps' bright_dark == bright_light by construction
  (color.rs:278) — is a future divergence intended (the field exists but can
  never differ for derived accents)? If not, bright_dark could collapse into
  bright_light for derived ramps with no behavior change.
