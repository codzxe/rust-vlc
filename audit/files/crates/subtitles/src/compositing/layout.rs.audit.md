# AUDIT: crates/subtitles/src/compositing/layout.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Lines / functions | 462 / 17 production items (+7 test fns) |
| File verdict | UNREACHABLE (real, test-executed library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `place_cue` production callers: compositing/compose.rs:93 (`SubtitleCompositor::compose`) and nothing else; `SubtitleCompositor` itself is called ONLY by crate tests (ass/render.rs:867 inside `#[cfg(test)]`, tests/timing_clock.rs, tests/compositing_alloc.rs, tests/compositing_ac.rs — rg over the whole repo). The `subtitles` crate's only repo-wide dependent is fuzz/Cargo.toml:50 (not shipped). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | rg "place_cue\|CueLayout\|SafeMargin\|SubtitleCompositor" (callers above); sed/awk/od verbatim verification (od -c for `#[`-lines — the Bash tool's display eats bracket-letter pairs; file bytes confirmed); hand re-derivation of every test's arithmetic; Read of bitmap.rs CueBitmap (pub fields), target.rs clip/blend_row; baseline: orchrun-test-workspace-nofailfast.log.txt:5350-5356 (all 7 layout tests ok), 5583-5588 (zero-alloc AC ok). |

## 1. One-paragraph verdict
This is the most rigorously engineered file in my assignment: exact u32/u64 rational arithmetic with a proven no-overflow rounding primitive, typed errors on every constructor, an anchor re-validation guard against the pub-field escape hatch, and 7 tests whose expected pixels I re-derived by hand and could not break — all green in the baseline run (log:5350-5356). It is nonetheless dead code in the shipped product: its only production consumer (`SubtitleCompositor`) is itself consumed by nothing but the crate's own tests — the "engine glue" that compositing/mod.rs describes does not exist. One latent panic survives the discipline: a hand-built `CueLayout` with `reference: Some((0, 0))` drives `uniform_scale` to a `1/0` scale and `scaled_dim` into a division by zero, because `place_cue` guards the anchor escape hatch but not the reference one. The safe-box "center on extent" branch is advertised by a test whose comment arithmetic is wrong and which passes without ever executing that branch.

## 2. What actually works (verified)
— Placement math: re-derived `anchor_math_is_pixel_exact_on_1080p` (192,972 → rect (128,956,128,32)), `safe_area_clamps_overflow` (48/470, ActionSafe 96), `baseline_alignment_pins_the_baseline_row` (92 vs 68), `scaling_is_uniform_min_ratio` (min-ratio 15/2 → 960×240 at y=1704; 11/120 → 33×17), `rounding_is_nearest_ties_up` (33, 13) — all match the code and pass in baseline (log:5350-5356).
— `Rational::scale_u64` overflow safety: worst case `(2^32-1)*(2^32-1) + 2^31` < `2^64-1` — no overflow for any u32×u64(extent) product; re-derived, not assumed.
— `gcd(0, n)` returns n → `Rational::new(0, den)` reduces to `0/1` ✓; `gcd_u64` is standard Euclid.
— `clamp_axis` never panics: `origin.clamp(min, max)` with `max = box_right.saturating_sub(size).max(box_left) ≥ min` ✓; all u64 paths saturate.
— `Anchor::new` enforces `[0,1]` per axis with a typed error; `place_cue` re-validates (escape-hatch guard) — verified at 262-277.
— Zero-alloc claim downstream: tests/compositing_alloc.rs `compose_steady_state_is_zero_allocation` passes in baseline (log:5583-5588) — the AC cited by compose.rs's doc is real and green.

## 3. Findings

### F-001 [S2] `place_cue` panics (division by zero) on `CueLayout { reference: Some((0, 0)) }` — the reference escape hatch is unguarded
Where: crates/subtitles/src/compositing/layout.rs:226-239 + 249-252 + 279-283
Code:
```
    let (num, den) = if ew * rh <= rw * eh {
        (ew, rw)
    } else {
        (eh, rh)
    };
    let g = gcd_u64(num, den);
    (num / g, den / g)
```
```
fn scaled_dim(dim: u32, sn: u64, sd: u64) -> u64 {
    (dim as u64 * sn + sd / 2) / sd
}
```
What is wrong: with `reference == (0, 0)` and a non-empty extent, `ew*rh = 0 <= rw*eh = 0` takes the first arm → `(num, den) = (ew, 0)`; `gcd_u64(ew, 0) = ew` → scale `(1, 0)`; `scaled_dim(dim, 1, 0)` then divides by `sd = 0` → panic. `place_cue` explicitly guards the OTHER pub-field escape hatch (anchor, lines 272-277, doc line 262: "pub-field escape hatch: constructors validate, this guards") but `CueLayout` has all-pub fields and NO constructor — nothing anywhere validates `reference`, and single-axis zero references are silently mishandled too (`(w, 0)` silently picks the y-scale, `(0, h)` the x-scale, instead of an error).
Why it matters: this is the compositing CONTRACT file — the module doc's four-step model claims a pure, error-typed function; a panic here is exactly the class the crate's "data, never a panic" discipline (bitmap.rs CueError doc, MP §8) forbids. Reachable by any future engine-glue caller constructing layouts by field (the documented pattern — BOTTOM_CENTER is a const, everything else is hand-assembled, see tests/compositing_alloc.rs:37-40 mutating pub fields).
What correct looks like: `place_cue` validates `reference` non-zero (typed `ComposeError::InvalidLayout`), or `uniform_scale` returns an error/`(1,1)` on a zero axis; `CueLayout::new` constructor that validates.
Confidence: HIGH — Verification: read + full arithmetic trace of the (0,0) path (gcd_u64(n, 0) = n confirmed at 241-247).

### F-002 [S3] The "center on extent" branch is untested — `cue_larger_than_safe_box_centers_on_extent` passes through the ordinary clamp path and its comment arithmetic is wrong
Where: crates/subtitles/src/compositing/layout.rs:416-425 (test) and 330-334 (the branch)
Code (test):
```
    fn cue_larger_than_safe_box_centers_on_extent() {
        // 400 px wide cue in a 480 px frame: 5% box is [24, 456] =
        // 432 px wide — does not fit; center on the full extent:
        // (480-400)/2 = 40.
```
Code (branch):
```
    if pos + size > extent {
        pos = extent.saturating_sub(size) / 2;
    }
```
What is wrong: 432 px (the safe box) is WIDER than the 400 px cue — it fits; the comment's "does not fit" is false. The expected x=40 is produced by the ordinary path: anchor u=1/2 of 480 = 240, Center alignment rx = 200, x0 = 40, `clamp(40, 24, max(456-400, 24)=56)` = 40, and `40+400 = 440 ≤ 480` so the recenter branch never runs. The branch only fires when `size > extent - inset` (e.g. 400 px in a 420 px frame). Mutation test by reasoning: deleting lines 331-334 entirely leaves this suite green — the branch has zero coverage.
Why it matters: untested branch in the placement contract; a regression in the overflow-recenter logic (subtitles hanging off-frame) would ship silently.
What correct looks like: a real overflow case (cue width > extent-inset) with a hand-derived centered expectation, e.g. 400 px cue in a 420 px frame → x = 10.
Confidence: HIGH — Verification: read + re-derivation of both paths.

### F-003 [S4] `HorizontalAlign::End` and `VerticalAlign::Top` arms have no test coverage through `place_cue`
Where: crates/subtitles/src/compositing/layout.rs:294-304
Code:
```
    let rx = match layout.alignment.horizontal {
        HorizontalAlign::Start => 0,
        HorizontalAlign::Center => sw / 2,
        HorizontalAlign::End => sw,
    };
```
What is wrong: the in-file tests exercise Start (safe_area_clamps_overflow), Center (anchor_math/scaling), Baseline and Bottom (baseline test) — never `End` (rx = sw) nor `Top` (ry = 0). `Top` is used in production by pgs/layout_for and dvb (right-aligned ASS `\an` uses End) — those paths are only covered indirectly, if at all, in other modules' tests.
Why it matters: the alignment table is the core of the contract; two of seven arms are mutation-invisible in this file.
What correct looks like: one 3×3 alignment sweep test (like the ASS 9-grid) at this layer.
Confidence: HIGH — Verification: read + coverage map of the 7 tests.

### F-004 [S4] Safe-box clamping is the DEFAULT for every layout — silent repositioning of absolute-position formats
Where: crates/subtitles/src/compositing/layout.rs:150-158, 200-201
Code:
```
pub enum SafeMargin {
    /// 5 % title-safe inset (subtitles default).
    #[default]
    TitleSafe,
```
What is wrong (design debt, cross-file blast radius): `TitleSafe` is the default margin and there is no "exact/no-clamp" variant. PGS (pgs/mod.rs layout_for:665), DVB (dvb/mod.rs) and ASS (ass/layout.rs:341,384 — its own doc admits "the compositor's SafeMargin::TitleSafe clamp still applies ON TOP") all clamp absolute author positions into the 5% box. VLC/libass/ffmpeg render these formats at their authored coordinates. The S2 for the PGS instance is filed in my pgs/mod.rs#part2 audit (F-001); recorded here because the root cause is this enum's missing variant.
Why it matters: one enum variant (`Exact`) would fix three formats at once.
Confidence: HIGH — Verification: read of all three call sites.
What correct looks like: `SafeMargin::Exact` (no inset) used by bitmap/absolute-position builders; text formats keep TitleSafe.

### F-005 [S4] `place_cue` returns x/y guaranteed ≤ extent — an undocumented invariant compose.rs depends on for memory safety
Where: crates/subtitles/src/compositing/layout.rs:310-316 and compositing/compose.rs:123-124
Code:
```
    let rect = Rect {
        x: clamp_axis(x0, sw, extent.0 as u64, layout.margin, true),
```
```
        let xs = (dst.x - placed.x) as usize;
        let xe = (dst.x + dst.width - placed.x) as usize;
```
What is wrong: compose.rs subtracts `dst.x - placed.x` in u32; this cannot underflow ONLY because `clamp_axis` provably returns `pos ≤ extent` (both the clamp and the recenter paths bound it). That cross-module invariant is written nowhere — neither file documents it, and a future change to clamp_axis (e.g. returning the raw origin on some path) turns compose.rs into a debug-panic / release-wrap with out-of-bounds slicing downstream.
Why it matters: memory-safety-by-coincidence across the crate's central seam.
What correct looks like: a doc comment on `place_cue`'s contract ("returned rect is within `extent` on both axes") plus a debug_assert in compose.rs.
Confidence: HIGH — Verification: read of both sides; clamp bounds re-derived (pos ≤ box_right ≤ extent; recenter ≤ extent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Rational` | 31-37 | REAL | — | pub num/den u32 |
| `Rational::ZERO` / `ONE` | 40-43 | REAL | — | pre-reduced consts |
| `Rational::new` | 45-58 | REAL | — | typed ZeroDenominator; gcd reduction; `new(0,n)` → 0/1 verified |
| `Rational::scale_u64` | 60-65 | REAL | — | round-nearest-ties-up; overflow-free for u32×u64-extent (bound re-derived) |
| `gcd` | 68-74 | REAL | — | handles (0,n) → n; (0,0) → 1 (unreachable: den≠0 checked) |
| `struct Anchor` + `BOTTOM_CENTER` | 76-91 | REAL | — | u=1/2, v=9/10 |
| `Anchor::new` | 93-109 | REAL | — | [0,1] per axis, typed error; `num > den` (== allowed = 100%) |
| `HorizontalAlign` | 112-122 | REAL | S4 | End arm untested (F-003) |
| `VerticalAlign` | 125-139 | REAL | S4 | Top arm untested (F-003); Baseline semantics defined |
| `CueAlignment` | 141-148 | REAL | — | plain data |
| `SafeMargin` + `percent` + `inset_px` | 150-177 | REAL | S4 | F-004: no Exact variant; inset math ties-up, verified |
| `Rect` | 180-191 | REAL | — | plain data |
| `CueLayout` + `BOTTOM_CENTER` | 193-221 | REAL | S4 | all-pub fields, no validating constructor (root of F-001) |
| `uniform_scale` | 226-239 | REAL (flawed) | S2 | F-001: (0,0) reference → 1/0 scale; single-axis zeros silently pick the wrong scale |
| `gcd_u64` | 241-247 | REAL | — | Euclid; (n,0)=n feeds F-001 |
| `scaled_dim` | 249-252 | REAL (flawed) | S2 | division by sd=0 reachable via F-001 |
| `place_cue` | 263-317 | REAL (flawed) | S2 | F-001 (no reference guard), F-005 (undocumented ≤extent contract); anchor guard ✓; saturating_sub ✓; must_use ✓ |
| `clamp_axis` | 321-336 | REAL | S3 | correct bounds math; centering branch (332-334) has zero test coverage (F-002) |
| tests (7) | 338-462 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| anchor_math_is_pixel_exact_on_1080p | YES | full Rect equality, hand-re-derived |
| baseline_alignment_pins_the_baseline_row | YES | two alignments contrasted (92 vs 68) |
| safe_area_clamps_overflow | YES | clamp boundary + ActionSafe contrast |
| cue_larger_than_safe_box_centers_on_extent | **NO** for its named subject | F-002: expected value comes from the ordinary clamp path; deleting the recenter branch leaves it green |
| scaling_is_uniform_min_ratio | YES | up+down scaling with exact dims and clamped y |
| zero_extent_is_typed | YES | error variant pinned |
| rounding_is_nearest_ties_up | YES | 12.5→13 pins tie direction |
| Untested | — | recenter branch (F-002), End/Top alignments (F-003), zero-dimension reference (F-001 — would panic today, so a test would need the fix first) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "exact integer/rational arithmetic only (no floats near a pixel; MP §8 determinism)" | layout.rs:1-3 | REAL | all math integer/rational; verified scale_u64 rounding |
| "The result is clamped into the SafeMargin box … a cue larger than the box centers on the full extent instead" | layout.rs:19-21 | REAL-UNPROVEN | clamp half verified by tests; the "centers" half is untested (F-002) |
| "pub-field escape hatch: constructors validate, this guards" (place_cue doc) | layout.rs:261-262 | FALSE (half) | the anchor is guarded; the reference escape hatch is NOT (F-001) |
| Compositing contract is "the 5.L1 contract" wired to the engine | crate Cargo.toml description, compositing/mod.rs:8 | UNREACHABLE | no caller of SubtitleCompositor outside crate tests (rg, whole repo); docs/formats/sub-pgs.md:36-39 admits R-27 renderer overlay missing |
| zero-alloc steady state "proven by the counting-allocator AC" | compose.rs:3-4 (cites this file's consumer) | REAL | tests/compositing_alloc.rs passes in baseline (orchrun log:5583-5588) |

## 7. Dead / duplicated / unreachable code
- The whole file (with the crate) is unreachable from shipped entry points.
- No dead items found: `Rational::ZERO` is used by 5 modules (cc/render.rs:82-83, dvb/mod.rs:935-936, vobsub/mod.rs:391-392, pgs/mod.rs:657-658), `Rational::ONE` by srt/render.rs:189,212 (rg-verified).
- Duplicated gcd (u32/u64 pair) — trivial, acceptable.

## 8. Dependencies & flags
- Depends only on crate-internal `bitmap`/`target` types + media-types — consistent with the zero-external-dep stance.
- No unsafe, no feature flags, no allocations in place_cue (pure) ✓.

## 9. Open questions for the next agent
- A08 (compositing/mod.rs, bitmap.rs): `CompositedCue::new` validates anchor via CueLayout construction? If it does NOT (CueLayout has no constructor), F-001's exposure widens — every `CompositedCue` hand-built with a mutated layout can panic the compositor.
- A03 (target.rs): `blend_row`'s `RowStride { y: 0 }` hard-codes y=0 — error carries no real row; minor, but it lies in diagnostics.
- Engine glue owner (future card): when wiring `SubtitleCompositor`, add the `SafeMargin::Exact` variant (F-004) before PGS/DVB/ASS ships, or edge-positioned subs will visibly jump.
- W16 (tests wave): add the recenter-branch test (F-002) and the End/Top sweep (F-003).
