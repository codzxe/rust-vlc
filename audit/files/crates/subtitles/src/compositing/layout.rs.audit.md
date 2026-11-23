# AUDIT: crates/subtitles/src/compositing/layout.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 462 / 13 prod (Rational::new, scale_u64, gcd, Anchor::new, SafeMargin::percent, inset_px, CueLayout::BOTTOM_CENTER, uniform_scale, gcd_u64, scaled_dim, place_cue, clamp_axis + consts) + 7 tests |
| File verdict | UNREACHABLE (real, tested; crate not wired to shipped apps) — code itself REAL in behavior |
| Reachable from shipped entry point? | NO — chain: compositing/compose.rs:93 `place_cue` ← `SubtitleCompositor::compose` (compose.rs:63), srt/render.rs:304, ass/render.rs; all inside `crates/subtitles`, depended on only by `fuzz/Cargo.toml:50` (non-member) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b compositing::layout` → 7 passed 0 failed (exit 0); `od -c` line 263 (bracket-mangling check); derivation of clamp/scale arithmetic by hand |

## 1. One-paragraph verdict
The placement math is genuinely exact (all u64/rational arithmetic, no floats — the determinism claim holds), the anchor/alignment/baseline model is implemented and honestly tested, and 7/7 tests pass when executed. Two real defects: (1) the documented "cue larger than the safe box centers on the full extent" rule is NOT implemented — cues sized between the safe-box width and `extent - inset` get jammed against the inset edge instead of centered, and the one test that claims to pin the centering passes through the ordinary clamp path by numeric coincidence; (2) `place_cue` re-validates the anchor for the pub-field escape hatch but NOT `reference` — `CueLayout { reference: Some((0,0)) }` (constructible by any consumer via pub fields) divides by zero in `scaled_dim`. Every in-crate builder currently guards zero extents, so the panic is not reachable from parsed files today — it is a contract hole in the file whose stated purpose is "the contract a GPU path implements".

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b compositing::layout` → `test result: ok. 7 passed; 0 failed`. Hand-re-derived: anchor math 1/10·1920=192, 9/10·1080=972; uniform_scale min via cross-multiplication (352,288)→(3840,2160) = 7.5 → 128x32 → 960x240 (matches test); Rational reduction incl. gcd(0,n)=n → 0/1; `u64::clamp` min≤max invariant holds (`max = (box_right-size).max(box_left)` ≥ box_left) — no panic path in clamp_axis.

## 3. Findings
F-001 [S2]
Where: layout.rs:19-21 (model), layout.rs:329-334 (impl)
Code: (verbatim)
```
//! 4. The result is clamped into the [`SafeMargin`] box (5 % title
//!    safe / 10 % action safe, broadcast convention); a cue larger
//!    than the box centers on the full extent instead.
```
```
    let mut pos = origin.clamp(box_left, box_right.saturating_sub(size).max(box_left));
    // The overflow case: the cue is wider than the safe box (or the
    // box is degenerate) — center on the full extent instead.
    if pos + size > extent {
        pos = extent.saturating_sub(size) / 2;
    }
```
What is wrong: the fallback fires only when `pos + size > extent`, but after the clamp `pos ≤ box_right - size` whenever `box_right ≥ size + box_left`, so `pos + size ≤ extent - inset < extent` — the fallback is unreachable for every cue that fits `extent - inset`. A cue LARGER than the safe box but ≤ `extent - inset` is clamped to `pos = box_left = inset` (jammed against the inset edge), not centered. Derived counterexample: extent 480, 5 % inset 24, cue width 440 (box [24,456] is 432 wide — 440 > 432): clamp(·,24,24) → x=24; doc/promised model says center → (480-440)/2 = 20. Code gives 24. Why it matters: wide subtitle bitmaps at small presentation sizes (e.g. long ASS lines on 352x288 CIF: box width ≈ 316, cues 317..334 px) render glued to the left inset instead of centered — user-visible placement error in realistic cases, and the module doc is false. What correct looks like: detect `size > box_right - box_left` (cue exceeds safe box) and center on extent, per the doc.
Confidence: HIGH — Verification: read (full arithmetic derivation above); no executable proof without editing files (cargo-test budget not spent — UNVERIFIED by run).

F-002 [S2]
Where: layout.rs:226-239, layout.rs:250-252, layout.rs:280-284; contrast layout.rs:272-277
Code: (verbatim)
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
What is wrong: `reference == Some((0, 0))` with a valid extent takes the `ew*rh <= rw*eh` branch (0 ≤ 0), yielding `(num, den) = (ew, 0)`, `gcd_u64(ew, 0) = ew` → scale `(1, 0)` → `scaled_dim` divides by zero → PANIC. `place_cue` deliberately re-validates the anchor because `CueLayout`/`Anchor`/`Rational` have pub fields ("pub-field escape hatch: constructors validate, this guards", line 262) — but `reference` gets no such guard. Every current in-crate builder happens to guard zero extents (pgs compose clears the display on `pcs.width == 0 || pcs.height == 0`, pgs/mod.rs:531; DVB DDS adds +1 so min 1, dvb/segment.rs:371-372; ASS returns `ZeroPlayRes`, ass/layout.rs:362-364; CC uses a const), so no parsed file panics today — but any external/future consumer constructing `CueLayout` directly (the documented integration style) hits a panic where the crate's contract promises typed errors, never panics. What correct looks like: validate `reference` in `place_cue` (reject zero axes with `ComposeError::InvalidLayout`) mirroring the anchor guard.
Confidence: HIGH — Verification: read (arithmetic traced: (0,0)→(ew,0)→gcd=ew→(1,0)→/0); in-crate reachability of the panic: none found (all four builders checked).

F-003 [S3]
Where: layout.rs:416-425
Code: (verbatim)
```
    fn cue_larger_than_safe_box_centers_on_extent() {
        // 400 px wide cue in a 480 px frame: 5% box is [24, 456] =
        // 432 px wide — does not fit; center on the full extent:
        // (480-400)/2 = 40.
        let cue = bitmap(400, 16, 16);
        let layout = CueLayout::BOTTOM_CENTER;
        let rect = place_cue(&cue, &layout, (480, 270)).must();
        assert_eq!(rect.x, 40);
    }
```
What is wrong: the comment's arithmetic is false — a 400 px cue DOES fit the 432 px box (`box_right - size = 56 ≥ box_left = 24`), so `x=40` comes from the ordinary clamp path (`x0 = 240-200 = 40 ∈ [24,56]`), not from the centering fallback. Deleting the fallback branch (lines 332-334) would NOT fail this test. The one test named after the centering rule proves nothing about it — which is exactly why F-001 survived.
Confidence: HIGH — Verification: read (both paths re-derived; both yield 40 here — coincidence).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rational::new | 49-58 | REAL | — | gcd(0,n)=n → 0/1; ZeroDenominator typed |
| Rational::scale_u64 | 63-65 | REAL | — | ties-up rounding, u64 domain; overflow needs v ≈ 2^64/num — unrealistic |
| gcd | 68-74 | REAL | — | correct; 0,0→1 |
| Anchor::new | 98-109 | REAL | — | rejects num>den, den==0 |
| SafeMargin::percent/inset_px | 162-177 | REAL | — | (w*pct+50)/100 ties-up |
| uniform_scale | 226-239 | REAL | S2 | (0,0) reference → (1,0) scale (F-002) |
| gcd_u64 | 241-247 | REAL | — | correct |
| scaled_dim | 250-252 | REAL | S2 | divides by sd — panics on sd=0 (F-002) |
| place_cue | 264-317 | REAL | S2 | anchor guarded, reference NOT (F-002); clamp delegates to clamp_axis (F-001) |
| clamp_axis | 321-336 | REAL | S2 | clamp sound (no panic); centering fallback unreachable in the documented range (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| anchor_math_is_pixel_exact_on_1080p | YES | exact Rect asserted, no clamp needed |
| baseline_alignment_pins_the_baseline_row | YES | y=92 vs 68 discriminates Baseline vs Bottom |
| safe_area_clamps_overflow | YES | x=48/96 discriminates TitleSafe vs ActionSafe |
| cue_larger_than_safe_box_centers_on_extent | NO (for the fallback) | F-003: passes via the ordinary clamp; fallback untested |
| scaling_is_uniform_min_ratio | YES | 960x240 + 33x17 pin min-ratio + ties-up rounding |
| zero_extent_is_typed | YES | ZeroExtent shape |
| rounding_is_nearest_ties_up | YES | 1/3·100=33, 1/8·100=13 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a cue larger than the box centers on the full extent instead" | layout.rs:19-21 | FALSE | F-001 derivation: 440-px cue in 480 → x=24, not 20 |
| "exact integer/rational arithmetic only (no floats near a pixel)" | layout.rs:1-3 | REAL | read: all u64 arithmetic, no float in file |
| "never distorted, never exceeding either axis" (scaling) | layout.rs:17-18 | REAL | uniform min-ratio + Rect caps sw/sh at extent (313-314) |
| "Baseline pins the cue's baseline row" | layout.rs:13-14 | REAL | place_cue:302 + test y=92 |
| "pub-field escape hatch: constructors validate, this guards" | layout.rs:262 | PARTLY FALSE | anchor guarded (272-277); reference is NOT (F-002) |

## 7. Dead / duplicated / unreachable code
- `gcd` (u32) vs `gcd_u64` duplication (S4-grade, folded here): two near-identical Euclid loops in one file.
- Whole file unreachable from shipped entry points (header table).

## 8. Dependencies & flags
- crate-internal: `compositing::bitmap::{CueBitmap, CueError}`, `compositing::target::ComposeError`; dev `testkit::Must`. No unsafe, no features, no floats.

## 9. Open questions for the next agent
- compose.rs / bitmap.rs owners: does any code path there construct `CueLayout` with pub-field literals (bypassing builders)? If yes, F-002 becomes reachable and must be re-rated S1.
- If anyone adds an engine→subtitles wiring: F-001 will be user-visible on wide cues at small extents; fix before wiring.
