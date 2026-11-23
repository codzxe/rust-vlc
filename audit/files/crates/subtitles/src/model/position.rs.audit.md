# AUDIT: crates/subtitles/src/model/position.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 449 / 11 (+6 tests) |
| File verdict | REAL (in-crate); crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (crate-level; see segment.rs audit header). In-crate chain: `vtt/settings.rs:109/156`, `srt/render.rs:47`, `ass/parser.rs:729`, `ass/layout.rs:247`, `model/wire.rs:432`, `model/cue.rs` -> `Percent`/`Positioning`/`resolve` here — every subtitle format maps onto this model. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p subtitles --lib model::position` -> 6 passed, 0 failed (exit 0); `sed -n` quote verification; Grep caller sweep (output above). |

## 1. One-paragraph verdict
Clean, exact-integer positioning model. All arithmetic is checked or provably non-overflowing (`percent_to_px`'s u64 intermediate genuinely cannot overflow; `line_number_to_px` uses saturating mul/add then clamp — verified against i64::MIN/MAX tests that pass). The one real gap is a model-level precision decision: `Percent` is integral-only (`u8`, 0..=100), while WebVTT's `line`/`position` syntax legally allows decimals — so a spec-legal `position: 37.5%` is dropped as damage by the VTT mapper instead of represented. Secondarily, the module doc labels its WebVTT semantics "[PROPOSAL]" — WebVTT's automatic `position-align`/`line-align`/`line-snap` defaults are not modeled (self-consistent simplification, admitted).

## 2. What actually works (verified)
— `cargo test -p subtitles --lib model::position` -> 6/6 pass (exit 0): percent bounds (0/100 ok, 101/-1 typed), all-nine alignment code roundtrip + rejects, half-up rounding (50% of 1px = 1, 49% of 1px = 0, 33% of 1000 = 330), saturation at u32::MAX, resolve fallbacks vs explicit (bottom-center default = (960,1080) on 1920x1080; line +2/-1 rows), empty viewport typed, i64::MIN/MAX line clamps.
— Overflow analysis (read): `extent_px as u64 * percent as u64` ≤ 4294967295*100 < 2^39; `.saturating_add(50)`; `/100`; the `> u32::MAX` branch is reachable only via saturation artifacts — the doc comment's invariant claim (line 293-294) is true. `line_number_to_px` (lines 311-318): `i64::saturating_mul` + `saturating_add` + `clamp(0, height_px)` — no panic path.

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/model/position.rs:24-52 (the type), consequence in crates/subtitles/src/vtt/settings.rs:104-112/152-160
Code:
```rust
/// A percentage in `0..=100` (typed-valid by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Percent(u8);
```
and (vtt/settings.rs:107-111)
```rust
        match parse_i32(digits) {
            Some(v) => match Percent::new(v) {
                Ok(p) => Some(LinePlacement::Percent(p)),
                Err(_) => None,
```
What is wrong: WebVTT permits decimal values for `line` and `position` (e.g. `position:37.5%`, `line:-12.5%`). `Percent` can only hold integral 0..=100, so the VTT mapper's `parse_i32` rejects the decimal, and the setting becomes a `bad_setting` damage event: a spec-legal positioning is silently degraded to the format default (bottom-center). The same integral-only ceiling propagates to ASS margins (`ass/layout.rs:247` pre-rounds rational margins to whole percents).
Why it matters: realistic WebVTT files (hand-authored and tool-generated, e.g. with `37.5%`) lose horizontal/vertical placement entirely; at 4K, 0.5% resolution steps would be ~19px — representable precision exists in the source but is thrown away rather than rounded.
What correct looks like: `Percent` as fixed-point (e.g. `u16` in 1/100 % units) or a rational; or an explicit documented "integral percents only, decimals damage-typed" policy in the module doc (currently undocumented).
Confidence: HIGH — Verification: read (type + mapper code); the damage-drop path is settings.rs's behavior, which I traced.

F-002 [S4]
Where: crates/subtitles/src/model/position.rs:5-17
Code:
```rust
//! ## Semantics [PROPOSAL — Lead unifies with 5.L1's parser contract]
```
What is wrong: the module's semantics are explicitly a PROPOSAL: WebVTT's automatic `position-align` (defaults derived from `align`), `line-align`, and `line` snap-to-lines-vs-percent interplay are not modeled — `resolve` (lines 330-359) binds the percent to the alignment anchor unconditionally and treats `line` as percent-of-height or row-count only. Self-consistent and documented as provisional, but the "WebVTT-aligned" claim (line 7) overstates conformance.
Why it matters: cue placement for VTT files with only `align` set (no explicit position) will differ from spec-conformant renderers (which apply position-align defaults). Blast radius limited: mapping happens in vtt/settings.rs which does set alignment explicitly.
What correct looks like: either implement the WebVTT default-derivation rules or downgrade the doc wording from "WebVTT-aligned" to "WebVTT-inspired subset".
Confidence: MEDIUM — Verification: read (spec behavior from knowledge; no WebVTT conformance suite in-tree — UNVERIFIED against a real renderer).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Percent::new / get | 41-51 | REAL | F-001 | integral-only ceiling is the model decision |
| Alignment::{new, code, from_code} | 129-198 | REAL | - | total 9-way both directions; roundtrip tested |
| Positioning::default | 226-236 | REAL | - | bottom-center classic |
| Positioning::new | 240-250 | REAL | - | const |
| Viewport::new | 268-276 | REAL | - | zero extents typed |
| percent_to_px | 296-303 | REAL | - | u64 intermediate, saturating; half-up rounding verified |
| line_number_to_px | 311-318 | REAL | - | saturating mul/add + clamp; i64::MIN/MAX tested |
| Positioning::resolve | 330-359 | REAL | F-002 | fallback-to-natural-edge policy; exact ints only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| percent_bounds | YES | pins 0/100/101/-1 |
| alignment_codes_roundtrip_all_nine | YES | sweeps ALL + 2 rejects |
| percent_to_px_rounds_half_up_and_saturates | YES | pins 4 rounding cases + u32::MAX saturation |
| resolve_fallbacks_and_explicit | YES | pins default (960,1080), explicit-override, +2/-1 rows |
| empty_viewport_is_typed_invalid | YES | both axes |
| line_number_clamps | YES | i64::MIN/MAX + zero line-height degeneracies |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Out-of-range percentages are typed-invalid at construction — a renderer never sees one" | position.rs:19-20 | REAL | Percent::new guard + percent_bounds test |
| "The `u64` intermediate cannot overflow (`u32::MAX × 100 + 50 < 2^40`)" | position.rs:293-294 | REAL | arithmetic check: 4.29e11 < 1.0995e12 |
| "A zero `line_height_px` degenerates to the respective edge" | position.rs:307-309 | REAL | line_number_clamps test (5,0,1080)/(−5,0,1080) |
| "WebVTT-aligned" vocabulary | position.rs:7 | SHALLOW | F-002: subset, PROPOSAL-labeled, no position-align/line-align defaults |

## 7. Dead / duplicated / unreachable code
None dead. `Percent::ZERO/HALF/FULL` consts all used (tests + callers). Crate unreachable from shipped binaries (crate-level, see header).

## 8. Dependencies & flags
std-only, no unsafe, no features. Depends only on `crate::model::error` (ModelError).

## 9. Open questions for the next agent
- Does any doc/matrix row promise fractional-percent positioning fidelity (VTT row)? If yes, F-001 upgrades from model limitation to doc-vs-code contradiction.
- vtt/settings.rs `line` suffix (`start|center|end`) maps to VerticalAlign — check WebVTT `line-align` semantics (start/end refer to the writing direction, not simply top/bottom) when auditing that file.
