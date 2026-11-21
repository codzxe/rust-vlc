# AUDIT: crates/subtitles/src/model/position.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 449 / 8 fns/methods, 6 type defs, 6 tests |
| File verdict | UNREACHABLE (types used crate-wide; the resolve primitive itself production-dead) |
| Reachable from shipped entry point? | NO — crate not linked by `lumen-cli`/any app (only `fuzz/` + own tests). In-crate: `Percent`/`Alignment`/`Positioning`/`LinePlacement` are live vocabulary (ass/parser.rs:482,531,636,732; ass/layout.rs:279-292; srt/render.rs:193-207; vtt), but `Positioning::resolve`, `ResolvedAnchor`, `Viewport`, `percent_to_px`, `line_number_to_px` have ZERO production callers — only tests (ass/layout.rs:442 is in a test mod; srt/render.rs re-implements the fallbacks). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg "\.resolve\(|ResolvedAnchor|percent_to_px|line_number_to_px|Viewport::new|LinePlacement::" crates/subtitles/src` (callers listed above) · `sed -n` quotes · orchrun log grep: `model::position::tests` 6/6 ok (lines 5421-5426) |

## 1. One-paragraph verdict
The cleanest file audited so far: validated `Percent`, exhaustive 9-point alignment with a roundtripping wire code, saturating/clamping conversion helpers whose overflow comments are arithmetically true, and six real tests that all executed and passed. The one structural defect: the module's advertised product — the relative→absolute `resolve()` primitive — is used by nobody; the actual render paths (srt, compositing) each re-implement the identical fallback semantics in their own arithmetic, leaving `resolve`/`Viewport`/`ResolvedAnchor`/both `*_to_px` helpers as test-only code with a live semantic duplicate. And, as with every file in this crate, nothing here is reachable from a shipped binary.

## 2. What actually works (verified)
— 6/6 unit tests passed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5421-5426 (`model::position::tests::* ... ok`).
— `percent_to_px` overflow comment verified: u32::MAX·100+50 = 429,496,729,550 < 2^40 (1,099,511,627,776); round-half-up pinned by tests (0.5→1, 0.49→0, FULL·u32::MAX→u32::MAX).
— `line_number_to_px` clamp semantics traced for i64::MAX/MIN and zero line-height (test `line_number_clamps` pins all four).
— `Alignment::code`/`from_code` 9/9 roundtrip + 2 negative cases (test `alignment_codes_roundtrip_all_nine`).

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/model/position.rs:320-359
Code:
```
    pub fn resolve(
        &self,
        viewport: Viewport,
        line_height_px: u32,
    ) -> Result<ResolvedAnchor, ModelError> {
        let x_px = match self.position {
            Some(p) => percent_to_px(p, viewport.width_px),
```
What is wrong: `resolve` (with `Viewport`, `ResolvedAnchor`, `percent_to_px`, `line_number_to_px`) has no production caller anywhere in the crate — only tests call it (ass/layout.rs:442, test mod). The live paths duplicate its exact fallback semantics instead of calling it: srt/render.rs:185-215 re-implements the same `position`/`line`/alignment-edge match with `Rational` arithmetic (its comment even admits the copy: "the model's own `line_number_to_px` semantics as a fraction"), and compositing carries its own `Anchor`/`Rational` vocabulary (srt/render.rs:42 imports `Anchor, CueAlignment, CueLayout … Rational` from compositing).
Why it matters: two hand-kept copies of the placement fallback table (px-rounded here, exact-rational there) is a drift magnet — a future fix to one (e.g. WebVTT `line`-auto snap, or a `position` clamp) will not reach the other; and the module doc sells `resolve` as "the relative→absolute primitive" when the product's real primitive lives elsewhere.
What correct looks like: either make `resolve` the single source (have srt/compositing convert its output, or move the rational version here and delete the px one), or delete the px path and say so in the module docs.
Confidence: HIGH — Verification: read + rg caller sweep (zero production callers).

### F-002 [S4]
Where: crates/subtitles/src/model/position.rs:41-46 vs 209
Code:
```
    pub fn new(value: i32) -> Result<Self, ModelError> {
        if !(0..=100).contains(&value) {
            return Err(ModelError::PercentOutOfRange { value });
        }
```
and
```
    /// A text-line count from the top; negative from the bottom.
    LineNumber(i64),
```
What is wrong: two user-facing numeric constructors in the same module take different integer widths (`Percent::new(i32)`, `LinePlacement::LineNumber(i64)`) for no stated reason.
Why it matters: API inconsistency invites call-site casts (`as` truncation risk at the boundary); trivial but real hygiene debt.
What correct looks like: pick one width (i64) for both, or document why they differ.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Percent::new` / `get` / consts | 28-52 | REAL | S4 | Validated 0..=100, typed error; i32-vs-i64 inconsistency F-002. |
| `VerticalAlign` / `HorizontalAlign` | 55-74 | REAL | — | Pure enums. |
| `Alignment::ALL` / `new` | 86-134 | REAL | — | Fixed order documented as the roundtrip sweep. |
| `Alignment::code` | 138-150 | REAL | — | Exhaustive 2-char codes. |
| `Alignment::from_code` | 158-198 | REAL | — | Total lookup, `_ => None`. |
| `Positioning::default` / `new` | 226-251 | REAL | — | Bottom-center default, documented as WebVTT's. |
| `Viewport::new` | 263-277 | REAL (test-only callers) | — | Typed EmptyViewport for zero extents. |
| `percent_to_px` | 295-303 | REAL (test-only) | — | const fn, saturating, half-up; overflow comment verified true. |
| `line_number_to_px` | 310-318 | REAL (test-only) | — | Saturating i64 math, clamped; zero-lh degeneration documented. |
| `Positioning::resolve` | 330-359 | DEAD (production) | S3 | F-001; semantics correct per its tests. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `percent_bounds` (372-383) | YES | 0/100 ok, ±1 typed errors. |
| `alignment_codes_roundtrip_all_nine` (386-392) | YES | 9/9 roundtrip + 2 junk codes. |
| `percent_to_px_rounds_half_up_and_saturates` (395-402) | YES | Pins rounding and u32::MAX saturation. |
| `resolve_fallbacks_and_explicit` (405-434) | YES | Default bottom-center (960,1080), explicit FULL, +2/−1 line rows — all concrete. |
| `empty_viewport_is_typed_invalid` (437-440) | YES | Both zero-extent axes. |
| `line_number_clamps` (443-448) | YES | i64::MAX/MIN and zero line-height. |
All 6 executed: orchrun log:5421-5426, all ok.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "relative→absolute resolution into ResolvedAnchor px" as the crate's placement primitive (lib.rs:62-64) | lib.rs card-5.L2 scope | SHALLOW | `resolve`/`ResolvedAnchor` test-only (F-001); real placement math duplicated in srt/render.rs + compositing. |
| "Semantics [PROPOSAL — Lead unifies with 5.L1's parser contract]" (position.rs:5) | position.rs header | HONEST | Correctly labeled proposal; no false finality claimed. |
| Matrix rows subtitle-srt/vtt/ass "implemented/partial" | docs/feature-matrix.toml:892-916 | UNREACHABLE | Crate not linked into any shipped binary (see file-1 audit). |

## 7. Dead / duplicated / unreachable code
Dead-in-production: `resolve`, `ResolvedAnchor`, `Viewport`, `percent_to_px`, `line_number_to_px` (test-only). Duplicated: the resolve fallback table exists again in srt/render.rs:185-215 (rational) — F-001.

## 8. Dependencies & flags
None (pure std + crate::model::error). No features, no unsafe, no panics (all paths typed or clamped — verified by read).

## 9. Open questions for the next agent
1. srt/render.rs + compositing/layout.rs auditors: confirm the rational fallback copy stays in lockstep with position.rs semantics (F-001) — a semantic diff test between the two would pin it.
2. Whoever owns the 5.L1/5.L2 "unification" [PROPOSAL]: decide which resolve survives.
