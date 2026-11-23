# AUDIT: crates/subtitles/src/ass/layout.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 584 / 15 (2 enums + AssAlignment impl 5 methods, 2 const blocks, validate, margin_percent, model_positioning, cue_layout, cue_layout_at, 7 tests) |
| File verdict | UNREACHABLE (orphaned: no production caller anywhere — see F-001) |
| Reachable from shipped entry point? | NO, doubly: (a) no workspace member depends on `subtitles` except fuzz/Cargo.toml:50; (b) inside the crate, `cue_layout`/`cue_layout_at` are called only by render.rs fns that themselves have ZERO production callers, and `model_positioning` has ZERO callers at all (parser.rs reimplements it) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | Grep `render_plain_text_cue|render_styled_cue` across crates/subtitles (only render.rs tests); Grep `model_positioning|cue_layout\(|cue_layout_at\(` (only render.rs:427-428,558-559 + own tests); sed quote verifications (exit 0); baseline citation below |

## 1. One-paragraph verdict
The numpad grid, margin validation, and rational-anchor math are correct and well-tested — and the entire file is dead code twice over: no shipped binary links the crate, and within the crate the file's two claimed consumers do not exist (the parser reimplements `model_positioning`'s percent logic locally; the render functions that call `cue_layout`/`cue_layout_at` are themselves never called by anything but their own unit tests). The module doc's central architecture claim — "This is the seam the 5.M2a parser half consumes" — is FALSE. On the hostile-focus questions: margins are anchor insets only (they never constrain line width — the ASS path has no automatic wrapping at all, so every long unwrapped dialogue line overflows the frame), centered alignment silently drops MarginL/R (libass/VSFilter center inside the margin box), and `\pos` coordinates are clamped by the title-safe box and silently degraded to bottom-center when out of frame — both contradicting this file's own "no silent moves for authored coordinates" policy text.

## 2. What actually works (verified)
— Baseline `cargo test -p subtitles` (audit/runs/baseline/cargo-test-workspace.txt:5743-5749): all 7 `ass::layout::tests` pass (`numpad_grid...`, `bad_alignment_is_typed`, `model_positioning_carries_margins_as_rounded_percents`, `cue_layout_builds_exact_rational_anchors`, `middle_alignment_anchors_the_frame_center`, `cue_layout_at_pins_the_pos_anchor_exactly`, `margin_and_play_res_violations_are_typed`), lib total 267 passed / 0 failed. The 1–9 numpad → 3×3 mapping (layout.rs:141-157), round-half-up margin percent (246), exact reduced rationals (20/384=5/96, 258/288=43/48 — test at 473-474), and the typed MarginExceedsPlayRes/ZeroPlayRes refusals (212-238) all check out by read.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/ass/layout.rs:7-12
Code:
```
//! * [`model_positioning`] — the parser/cue-model vocabulary
//!   ([`Positioning`], WebVTT-flavored integer percents). This is the
//!   seam the 5.M2a parser half consumes when it builds
//!   [`crate::model::Cue`]s. Integer percents are LOSSY (a 20 px margin
```
What is wrong: the seam does not exist. Grep proves `model_positioning` has zero callers outside its own tests; parser.rs builds positioning with its own local `to_percent` closure (parser.rs:714-727, "percent = milli / (extent × 1000) × 100, round-half-up") and never imports this function. Likewise `cue_layout`/`cue_layout_at` are called only from render.rs:427-428/558-559 inside `render_plain_text_cue`/`render_styled_cue`, and those two functions have ZERO production callers (grep across crates/subtitles: every call site is a `#[cfg(test)]` body; `AssRenderCue` is constructed only in render.rs tests at 598/1002). ass/mod.rs:36-50 documents this as a "[LEAD SEAM]" to be unioned at merge — the union never produced a bridge. Why it matters: the only live ASS positioning path is the parser's lossy integer-percent one; this file's exact-rational "render path's real output" (line 297) feeds nothing. What correct looks like: either the parser→AssRenderCue bridge, or doc that says "no consumer". Confidence: HIGH — Verification: read (greps, exit 0).

F-002 [S2]
Where: crates/subtitles/src/ass/render.rs:315-319 (cross-file — the consumer of this file's margins) + layout.rs:29-31
Code:
```
/// Break points: the ASS forced break `\N` (the two-byte literal,
/// backslash + `N`); the two-byte literal `\n` (the ASS soft break,
/// honored as a hard break here because this path does no reflow —
/// `WrapStyle` reflow is out of scope, honestly noted); and a bare
/// U+000A (for parsers that pre-normalize `\N` to a real newline).
```
What is wrong: the ASS path implements NO automatic line breaking. `split_lines` (render.rs:323-345) breaks only at `\N`/`\n`/U+000A; bitmap width = max line char count × advance (render.rs:368-379) with no cap against the margin box. libass/VSFilter wrap lines to `PlayResX − MarginL − MarginR` (WrapStyle 0 smart wrap is the default); real-world ASS dialogue relies on this. Consequence through this file: `MarginL`/`MarginR` can never act as wrap constraints — they are anchor insets only — and any long unwrapped line produces a bitmap wider than the frame, which `place_cue` then clamps/centers and crops (`compositing/layout.rs:310-335`, `width: sw.min(extent.0)`). The "margin semantics (documented policy)" section of this file therefore documents a weaker semantic than the one it cites as reference behavior. Confidence: HIGH (code behavior) — Verification: read; libass wrap-width behavior is reference knowledge, not source-verified here (MEDIUM on the reference side).

F-003 [S3]
Where: crates/subtitles/src/ass/layout.rs:318
Code:
```
        AssHorizontal::Center => (Rational::new(1, 2)?, CompHorizontal::Center),
```
What is wrong: centered alignment pins the anchor at frame center, discarding MarginL/MarginR entirely. VSFilter/libass center the text block inside the margin box `[MarginL, W−MarginR]` (VSFilter: center `x = (W − MarginR + MarginL)/2`), so asymmetric margins shift centered cues there but not here. The module doc states the behavior (line 30-31: "ignored for centered — the anchor is the frame center") as plain policy — without the `[HONEST NOTE]` deviation treatment given to the middle-vertical case — while the same header (lines 13-18) sells the margin scaling as "the libass behavior". Confidence: MEDIUM (libass/VSFilter center-in-margin-box policy not source-verifiable in sandbox) / HIGH for what this code does. Verification: read.

F-004 [S3]
Where: crates/subtitles/src/ass/layout.rs:347-351 vs 384 (and parser.rs:706-735 cross-file)
Code:
```
/// the compositing contract: the alignment's anchor point of the cue
/// box binds to `(x_px, y_px)` in SCRIPT coordinates — margins are
/// overridden (the libass behaviour for explicit positions; a clamp
/// would move the cue silently, an error would drop it — neither is
/// honest for authored coordinates).
```
What is wrong: two contradictions. (1) This doc paragraph says authored `\pos` coordinates must not be clamped — but `cue_layout_at` itself sets `margin: SafeMargin::TitleSafe` (line 384), and `place_cue` step 4 (compositing/layout.rs:308-335) clamps EVERY cue into the 5 % title-safe box, so `\pos(5,5)` on a 384×288 script displayed at 1920×1080 moves from (25,19) to (96,54) — a silent move of an authored coordinate. The module header (lines 42-46) admits the title-safe clamp applies "ON TOP of explicit margins" for ordinary cues; the `\pos` paragraph denies clamping in the same file. (2) Out-of-frame `\pos`: `Anchor::new` rejects fractions outside [0,1], so this path returns `Err` (cue dropped); the live parser path (parser.rs:733-735) instead keeps the default bottom-center positioning and bumps `position_overrides_unmappable` — a silent relocation to bottom-center, exactly the "clamp would move the cue silently" outcome the docs refuse. Also `pos: (u32, u32)` (line 360) cannot represent negative coordinates, though `PositionOverride` is i64 milli-px and `parse_decimal_milli("-3.25")` is `Some(-3_250)` (parser.rs:1365) — libass renders negative/off-frame positions (common in typeset files). Confidence: HIGH — Verification: read (place_cue clamp verified in compositing/layout.rs:308-335; parser fallback verified at parser.rs:714-735).

F-005 [S4]
Where: crates/subtitles/src/ass/layout.rs:302-306 and 240-244
Code:
```
/// [`AssRenderError::ZeroPlayRes`] / [`AssRenderError::MarginExceedsPlayRes`]
/// via the shared validation; [`AssRenderError::Cue`] defensively for
/// any rational/anchor construction failure (pre-checked invariants
/// make those unreachable).
```
What is wrong: the "defensive" `?` arms on `Rational::new`/`Anchor::new` are provably dead (validate() guarantees non-zero denominators and margins ≤ axis, so every fraction is in [0,1]), and the same rounding policy exists twice — `margin_percent` here vs parser.rs:714-727 `to_percent` — with the parser copy winning because nothing calls this file (F-001). Duplication + dead arms, no behavior bug. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AssHorizontal`/`AssVertical` | 63-82 | REAL | — | Pure enums. |
| `AssAlignment::{BOTTOM_LEFT..TOP_RIGHT, ALL}` | 89-120 | REAL | — | Correct numpad constants; ALL in numpad order. |
| `AssAlignment::new` | 126-131 | REAL | — | Typed 1..=9 validation. |
| `AssAlignment::get` | 134-137 | REAL | — | Const getter. |
| `AssAlignment::horizontal` | 141-147 | REAL | — | 1/4/7 L, 2/5/8 C, 3/6/9 R — correct. |
| `AssAlignment::vertical` | 151-157 | REAL | — | 1-3 B, 4-6 M, 7-9 T — correct. |
| `AssAlignment::to_model_alignment` | 161-176 | REAL | — | Correct 3×3 projection. |
| `AssMargins::{ASS_DEFAULT, ZERO}` | 193-208 | REAL | — | 10/10/10 matches the common Default style. |
| `validate` | 212-238 | REAL | — | Typed refusal (deliberate deviation: libass clamps oversized margins — documented at line 40-41). |
| `margin_percent` | 245-248 | REAL | — | u64 math, round-half-up, no overflow; dead (no callers). |
| `model_positioning` | 262-294 | REAL (orphaned) | S2 | Correct lossy projection; zero callers (F-001). |
| `cue_layout` | 307-344 | REAL (orphaned) | S3 | Exact rationals correct; center drops L/R margins (F-003); margins never wrap-constrain (F-002). |
| `cue_layout_at` | 357-387 | REAL (orphaned) | S3 | \pos anchor math correct; TitleSafe contradicts its own doc; u32 can't hold negative \pos (F-004). |
| 7 tests | 400-583 | REAL | — | Honest, exact, would fail on regression; none test the (nonexistent) wiring. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| numpad_grid_maps_all_nine_to_the_model_matrix | YES | Sweeps all 9 against expected 3×3 codes. |
| bad_alignment_is_typed | YES | Exact Err equality for 0/10/255. |
| model_positioning_carries_margins_as_rounded_percents | YES | Pins 5/90/95/10 percents + resolved (19,259) px + None fallbacks. |
| cue_layout_builds_exact_rational_anchors | YES | Exact reduced pairs (5,96)/(43,48)/(5,48). |
| middle_alignment_anchors_the_frame_center | YES | Pins the MarginV-ignored policy. |
| cue_layout_at_pins_the_pos_anchor_exactly | YES | Exact (1,2) pair + ZeroPlayRes. |
| margin_and_play_res_violations_are_typed | YES | Exact Err variants + boundary margin==axis Ok. |
All green in baseline. Gap: no test covers \pos outside the frame, negative \pos, or asymmetric centered margins — the exact cases where behavior deviates (F-003/F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This is the seam the 5.M2a parser half consumes" | layout.rs:9-10 | FALSE | parser.rs:714-727 reimplements; zero callers (F-001). |
| "the render path actually feeds crate::compositing" | layout.rs:13-15 | FALSE | render fns have zero production callers (F-001). |
| "the libass behavior" for margin scaling | layout.rs:16-17 | PARTIAL | Scaling matches libass; but center ignores margins (F-003) and there is no wrap box (F-002). |
| "a clamp would move the cue silently ... neither is honest for authored coordinates" | layout.rs:349-351 | FALSE | TitleSafe clamp applies (line 384 + place_cue); parser degrades off-frame \pos to bottom-center (F-004). |
| "9/9 alignment-grid pixel-exact" evidence | docs/feature-matrix.toml:914 | REAL-UNPROVEN | The 9-alignment sweep exists (render.rs:698) and passes (baseline), but exercises an uncalled code path. |

## 7. Dead / duplicated / unreachable code
Everything: whole file is unreachable from shipped binaries (crate unwired), and unreachable even from the crate's own parser (F-001). Duplicated: margin→percent rounding (layout.rs:245-248 vs parser.rs:714-727). Dead: defensive `?` arms documented unreachable (F-005).

## 8. Dependencies & flags
Depends on compositing::layout, model::position, ass::render (error type). No features, no unsafe, no I/O. DEFAULT_PLAY_RES (384,288) matches the libass default; parser.rs:691 cites it but re-derives 384/288 inline (parser.rs:699,701) rather than using the constant.

## 9. Open questions for the next agent
- ass/render.rs auditor: confirm `MAX_BITMAP_BYTES` bound and whether place_cue's `width: sw.min(extent)` crop is actually applied by compose.rs blend (rect smaller than bitmap).
- ass/parser.rs auditor: legacy SSA `\a` alignment encoding (v4 `\a6` etc.) — does the style parse map it onto the 1-9 numpad before AssAlignment::new? Not visible from this file.
- Who was supposed to write the parser→render bridge ("Lead seam", ass/mod.rs:36-50)? It is the single missing wire that makes this entire file + render.rs dead.
