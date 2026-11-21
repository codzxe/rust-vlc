# AUDIT: crates/subtitles/src/compositing/compose.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Lines / functions | 136 / 4 production items (struct + Debug + new + compose + blend_one) |
| File verdict | UNREACHABLE (real, acceptance-test-executed library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `SubtitleCompositor` callers, repo-wide (rg): ONLY crate tests — ass/render.rs:867 (inside `#[cfg(test)]`, line 577), tests/timing_clock.rs:72,155,176,212, tests/compositing_alloc.rs, tests/compositing_ac.rs. The "engine glue" that compositing/mod.rs:8 describes ("calls SubtitleCompositor::compose with the clock's now()") does not exist. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg "SubtitleCompositor" (callers above — none outside tests); Read of bitmap.rs CompositedCue::new (no layout validation — 212-234), timing.rs active_at (32-44), target.rs blend_row/clip (236-289); sed verification of all quotes; bounds/arithmetic re-derivation of blend_one; baseline: compositing_ac 8/8 ok (orchrun-test-workspace-nofailfast.log.txt:5569-5582), compositing_alloc ok (5583-5588). |

## 1. One-paragraph verdict
The reference compositor is genuinely implemented and unusually well proven for this repo: 8 pixel-exact acceptance tests (position, top-right quadrant, alpha rounding, z-order stacking, exact overlap seams, CIF↔4K scaling both directions, inactive-draws-nothing) run through the real `compose()` path with hand-computed integer expectations and all pass in the baseline run (log:5569-5582), plus a counting-allocator proof that the steady state allocates nothing (log:5583-5588) — the file's central claim is test-proven, not asserted. The nearest-neighbor sampler and all slice/index arithmetic re-derive as panic-free under the constructor invariants (analysis below). It is dead code in the shipped product — no caller outside the crate's own tests exists — and it inherits the crate's escape-hatch fragility: `CompositedCue::new` does not validate the layout, so one bad cue either panics the frame (zero-extent `reference`, layout.rs F-001) or aborts it (out-of-range anchor propagated by `?` at line 93, discarding every OTHER cue too — all-or-nothing where the damage-ledger discipline demands skip-and-note). The return value lies about its meaning (counts active, doc says drawn) and the ns cast wraps past i64::MAX.

## 2. What actually works (verified)
— Pixel-exact placement/blend/z/scaling: 8/8 compositing_ac tests pass in baseline (orchrun-test-workspace-nofailfast.log.txt:5572-5580), including `overlap_stacks_by_z_order_not_insertion_luck` and `partial_overlap_has_exact_seam`.
— Zero-alloc steady state: `compose_steady_state_is_zero_allocation` passes (log:5583-5588); `order.clear()+push` and `xmap.clear()+push` reuse capacity; `sort_unstable` is allocation-free; `place_cue`/`clip`/`blend_row` allocate nothing (verified by reading all three).
— z-semantics: `order.sort_unstable()` on `(z, index)` — unique total key, deterministic; ascending blend order makes higher-z and later-index overwrite lower ones — matches the doc "higher z on top; same z: later slice entry on top" (49-51) and is pinned by the overlap AC test.
— blend_one bounds (re-derived): `dw>0`/`dh>0` whenever `clip` returns `Some` (a zero width/height rect clips to `None` — target.rs:275-289), so the `/(2*dw)` and `/(2*dh)` divisions are guarded; `xs..xe ≤ dw = xmap.len()`; `sy ≤ sh-1` so `src_row = &rgba[sy*stride .. +stride]` stays inside the `w*h*4` buffer; `dst.x - placed.x` cannot underflow because `place_cue` returns `x ≤ extent` (see layout.rs F-005 for the undocumented-invariant caveat).
— NN sampling: `sx = (2dx+1)*sw / (2dw)` clamped to `sw-1` — pixel-center mapping, exact identity at 1:1, matches the doc (25-28).

## 3. Findings

### F-001 [S3] One invalid cue aborts the whole frame — all-or-nothing `?` propagation contradicts the damage-ledger discipline
Where: crates/subtitles/src/compositing/compose.rs:90-98
Code:
```
        for slot in 0..self.order.len() {
            let (_, i) = self.order[slot];
            let cue = &cues[i as usize];
            let rect = place_cue(&cue.bitmap, &cue.layout, extent)?;
            let Some(dst) = clip(rect, extent) else {
                continue;
            };
            self.blend_one(cue, rect, dst, target)?;
        }
        Ok(self.order.len())
```
What is wrong: `place_cue(..)?` returns `Err(InvalidLayout)` for an out-of-range anchor — one malformed cue discards the entire frame's subtitle render (the error aborts the loop; cues already blended remain half-drawn on the target, the rest are lost). `CompositedCue::new` (bitmap.rs:212-234, verified) performs NO layout validation, so an invalid layout flows through the normal constructor. The crate's own pattern elsewhere (missing object skipped + typed damage in pgs; bad setting → note in vtt) is skip-and-record; here it is abort.
Why it matters: one corrupt cue in a file nukes all subtitles for every frame it is active in — a much worse failure mode than the format parsers' discipline, in the layer that is supposed to be the shared contract.
What correct looks like: `continue` on placement error with a counter/typed note in the return (or a damage out-param), reserving `Err` for target-contract violations (ZeroExtent, RowStride) as the doc (58-62) already implies.
Confidence: HIGH — Verification: read of compose.rs + bitmap.rs:212-234 (no validation) + layout.rs place_cue error paths.

### F-002 [S4] Return value counts ACTIVE cues, not DRAWN cues — doc says "drawn"
Where: crates/subtitles/src/compositing/compose.rs:49-51 and 99
Code:
```
    /// Draw every cue in `cues` active at `now` onto `target`,
    /// z-ascending (higher z on top; same z: later slice entry on
    /// top). Returns how many cues were drawn.
```
```
        Ok(self.order.len())
```
What is wrong: a cue that is active but fully clipped (`clip` → `None`, e.g. a cue whose scaled width/height rounded to zero — `scaled_dim(1, 1, 1000) = 0` — while still "active") is counted in `order.len()` but never drawn. The doc's "how many cues were drawn" is false for that case.
Why it matters: callers using the count for statistics or "did anything render" checks get lied to; the alloc AC (`drawn_total == 480`) only avoids the case because its cues always draw.
What correct looks like: count in a `drawn` variable incremented after `blend_one`, return that.
Confidence: HIGH — Verification: read + trace of the zero-scaled-cue path.

### F-003 [S4] `as i64` nanosecond cast wraps for timestamps past i64::MAX (~year 2262)
Where: crates/subtitles/src/compositing/compose.rs:73
Code:
```
        let now_ns = now.as_nanos()?.0 as i64;
```
What is wrong: `as_nanos()` bounds to u64; values in `(i64::MAX, u64::MAX]` wrap negative on the cast. A negative `now_ns` makes `active_at` false for every normal cue (start ≥ 0 > now) → `compose` returns `Ok(0)` and renders nothing — silent, no error. The same cast exists in `CompositedCue::new` (bitmap.rs:222 — A08's file, cross-noted).
Why it matters: type-system-level silent wrap in the timing path; unrealistic input, but the crate's stated discipline is typed errors for exactly this class (`CueError::Time` exists for span conversions).
What correct looks like: `i64::try_from(ns.0).map_err(ComposeError::Time)` or a checked conversion.
Confidence: HIGH — Verification: read + wrap trace; active_at semantics verified in timing.rs:32-44.

### F-004 [S4] `sw as u64 - 1` underflows on a pub-field-mutated zero-size bitmap
Where: crates/subtitles/src/compositing/compose.rs:120-129
Code:
```
        for dx in 0..dw {
            let sx = ((2 * dx + 1) * sw as u64) / (2 * dw);
            self.xmap.push(sx.min(sw as u64 - 1) as u32);
        }
```
What is wrong: `CueBitmap` documents "Pixel width (never zero)" (bitmap.rs:88-89) and its constructors enforce it, but the fields are `pub` — a mutated `width = 0` bitmap reaches `blend_one` with `dw` from `place_cue`… (in practice a 0-width bitmap errors earlier via `ZeroDimension` on construction, so this needs field mutation post-construction). Then `sw as u64 - 1` wraps (release) → `xmap` fills with garbage → `blend_row` indexes `src_row[si+3]` out of bounds → panic. Same escape-hatch class as layout.rs F-001 — the compositor trusts bitmap invariants it does not re-check, unlike the anchor which place_cue DOES re-check.
Why it matters: contract-by-construction only; a future field-mutating caller (the same pattern the crate's own tests use for layouts — compositing_alloc.rs mutates pub fields) can panic the renderer.
What correct looks like: a debug_assert or an early `CueError`-style check in `compose` (`cue.bitmap.width == 0 || height == 0` → skip+count).
Confidence: MEDIUM — Verification: read; the panic path requires pub-field mutation (constructors reject 0), so it is latent, not reachable through current in-crate callers.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct SubtitleCompositor` | 20-30 | REAL | — | two scratch Vecs, documented |
| `impl Debug` | 32-39 | REAL | — | avoids dumping buffers — good hygiene |
| `SubtitleCompositor::new` | 44-47 | REAL | — | Default; must_use ✓ |
| `SubtitleCompositor::compose` | 63-100 | REAL (flawed) | S3 | F-001 all-or-nothing `?`; F-002 return-value semantics; F-003 ns cast; timing filter + z-sort verified correct |
| `SubtitleCompositor::blend_one` | 103-135 | REAL | S4 | F-004 underflow latent; NN math + bounds verified; xmap rebuilt per cue per frame (documented, CPU O(dw) — acceptable) |

## 5. Test honesty (no tests in-file; the suites that execute this file)
| Test (suite) | Would it fail if the code were broken? | Reason |
|---|---|---|
| compositing_ac::position_10_90_is_pixel_exact_on_1080p (+quadrant variant) | YES | exact corner pixels inside/outside the cue rect, hand-computed |
| compositing_ac::semi_transparent_blends_round_to_nearest | YES | pins the (a*s+(255-a)*d+127)/255 rounding through the real blend |
| compositing_ac::overlap_stacks_by_z_order_not_insertion_luck | YES | z-inversion vs insertion order distinguished |
| compositing_ac::partial_overlap_has_exact_seam | YES | seam pixel equality |
| compositing_ac::scaling_cif_authored_cue_grows_to_4k / shrinks_to_cif | YES | both scale directions with expected pixels |
| compositing_ac::inactive_cues_draw_nothing | YES | timing filter through compose |
| compositing_alloc::compose_steady_state_is_zero_allocation | YES | any steady-state allocation fails the guard (incl. sort/xmap growth) |
| timing_clock (4 tests) | YES | clock-driven cue flips through compose |
| Untested | — | F-001's abort path (no test constructs an invalid layout into compose), F-002's clipped-active case, F-003/F-004 wrap paths |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "zero allocation per frame after warm-up — proven by the counting-allocator AC, the 3.B3/W2-07 discipline" | compose.rs:2-4 | REAL | tests/compositing_alloc.rs green in baseline (orchrun log:5583-5588); scratch-reuse verified by reading |
| "callers pass the decoded cue slice per frame (the engine glue's job once subtitle frames flow)" | compose.rs:6-8 | UNREACHABLE | no caller exists outside crate tests (rg repo-wide); the future-tense phrasing is honest but the contract ships unwired |
| "the engine glue calls SubtitleCompositor::compose with the clock's now()" | compositing/mod.rs:8 (A08's file) | PAPER-ONLY | present-tense claim, zero non-test callers — cross-noted for A08 |
| "Returns how many cues were drawn" | compose.rs:51 | FALSE (edge) | F-002: returns active count incl. fully-clipped cues |
| "A cue whose placement falls entirely outside the target is skipped, not an error (data, not a failure)" | compose.rs:61-62 | REAL | clip→None→continue verified; though place_cue's clamping makes full-outside rare (mostly zero-scaled cues) |

## 7. Dead / duplicated / unreachable code
- The compositor itself is unreachable from shipped entry points (the file's purpose — the engine seam — has no consumer).
- No dead fns; `blend_one`'s `placed`/`dst` parameter pair mirrors `clip`'s contract without duplication.

## 8. Dependencies & flags
- crate-internal layout/target/bitmap + media-types Timestamp — clean.
- No unsafe; no feature gates; no per-pixel allocation. `dyn ComposeTarget` — dynamic dispatch per row (`row_rgba`), one virtual call per row per cue — fine at subtitle rates.

## 9. Open questions for the next agent
- A08 (bitmap.rs): `CompositedCue::new` should validate the layout (anchor AND reference — see my layout.rs F-001 for the (0,0)-reference panic and this file's F-001/F-004 for the trust chain). One constructor check closes three latent paths.
- A08 (compositing/mod.rs:8): the present-tense "engine glue calls" claim needs a future-tense rewrite or the actual wiring.
- A03 (target.rs): `blend_row`'s `RowStride { y: 0 }` hard-codes the row in the error — compose.rs could pass the real `y` down (it has it at line 131).
- Engine-glue owner: when wiring this, decide F-001's policy (skip-and-note vs abort) BEFORE the first corrupted-subtitle bug report.
