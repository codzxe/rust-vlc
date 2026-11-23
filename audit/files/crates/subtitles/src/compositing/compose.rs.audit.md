# AUDIT: crates/subtitles/src/compositing/compose.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 136 / 4 (compositor struct + new/compose/blend_one + Debug) |
| File verdict | REAL (bounds chain proven, scaling math correct, tests pixel-exact); compositor UNREACHABLE even in-crate from production code — only tests call `compose`; crate not linked by any app |
| Reachable from shipped entry point? | NO. Grep: `SubtitleCompositor::compose` callers = tests/{timing_clock,compositing_alloc,compositing_ac}.rs ONLY; no src/ caller, no renderer adapter exists (lib.rs: "a later card" seam). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `cargo test -p subtitles --test compositing_ac --test compositing_alloc` -> 9 passed, 0 failed (exit 0); `sed -n` quote verification; Grep sweeps (CueBitmap literal constructors, compose callers, as_nanos signature). |

## 1. One-paragraph verdict
The blending math is the real thing, not a stub: nearest-neighbor scaling with exact pixel-center mapping (`floor((2·dx+1)·sw/(2·dw))`, clamped), per-cue source-column map that hoists division out of the pixel loop, a scalar per-pixel over blend `(a·s+(255−a)·d+127)/255` with a==0/a==255 fast paths, and a destination-bounds check per row. The z-order is a lexicographic `(z, insertion index)` sort — deterministic, documented, tested. I traced the entire bounds chain (clip ⊆ placed ⇒ `xs`=0, `xe`≤`dw`; loops empty when dimensions are zero, so the `/ (2·dw)` divisions never see zero; zero-dim bitmaps clip to `None` and are skipped) — it holds for constructor-built cues. The hostile-focus hole: `blend_one` slices `bmp.rgba` with NO use-time stride validation while `CueBitmap` exposes pub fields whose escape hatch IS guarded for layouts (`place_cue` re-validates the anchor) but not for bitmaps — a struct-literal bitmap panics the public `compose` API in every profile. No SIMD anywhere (scalar row loop; the "no per-pixel division" claim is true).

## 2. What actually works (verified)
— `cargo test -p subtitles --test compositing_ac --test compositing_alloc` -> 9/9 pass (exit 0): pixel-exact 10%/90% and top-right-quadrant placement on 1080p, 4K↔CIF scaling both directions (960x240 / 33x17 exact), z-order overlap by tuple not insertion luck, semi-transparent rounding byte-exact, partial-overlap seam, inactive cues draw nothing, and `compose_steady_state_is_zero_allocation` (counting allocator) — the zero-alloc claim in the module docs is proven by a test I ran.
— Bounds-chain proof (read, cross-file): `clip` (target.rs:275-289) returns `Some` only with `x0 = rect.x` (if `rect.x ≥ extent.0` then `x0 ≥ x1` → `None`), so `dst.x == placed.x` always ⇒ `xs = 0`, `xe = dst.width ≤ placed.width = dw` — the `dst.x - placed.x` subtraction (compose.rs:123) cannot underflow; `dst.y == placed.y` likewise. `2·dw`/`2·dh` divisions occur only inside loops that are empty when `dw`/`dh` are 0. `sw`,`sh ≥ 1` at `blend_one` because `scaled_dim(0,·)=0` ⇒ zero-dim bitmap ⇒ zero-dim rect ⇒ `clip` → `None`.
— NN identity at 1:1: `dx=0 → (1·sw)/(2·sw)=0`; formula is standard pixel-center mapping; `sy` per row mirrors it (compose.rs:129).
— `blend_row` (target.rs:236-272): destination bounds checked once per row (`(dst_x + xmap.len())·4 > dst_row.len()` → typed `RowStride`), `a==0` skips, `a==255` copies, alpha forced 255 (opaque video plane).

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/compositing/compose.rs:126-132; the escape hatch at crates/subtitles/src/compositing/bitmap.rs:86-97; the guarded sibling at crates/subtitles/src/compositing/layout.rs:272-277
Code (compose.rs:126-132):
```rust
        let stride = sw as usize * 4;
        for y in dst.y..dst.y + dst.height {
            let dy = (y - placed.y) as u64;
            let sy = (((2 * dy + 1) * sh as u64) / (2 * dh)).min(sh as u64 - 1) as usize;
            let src_row = &bmp.rgba[sy * stride..sy * stride + stride];
```
and the unguarded pub fields (bitmap.rs:86-97):
```rust
pub struct CueBitmap {
    /// Pixel width (never zero).
    pub width: u32,
    ...
    /// `width * height * 4` RGBA8 bytes.
    pub rgba: Vec<u8>,
```
What is wrong: `CueBitmap`'s constructors validate (`from_rgba` rejects stride mismatch, bitmap.rs:116-128) but the struct's pub fields allow `CueBitmap { width: 100, height: 100, baseline: 0, rgba: vec![0; 4] }` — and `blend_one` slices `bmp.rgba` purely on `width/height`, with no use-time check. The crate GUARDS the identical escape-hatch class for layouts — `place_cue` re-validates the pub-field anchor (layout.rs:272-277) and `ComposeError::InvalidLayout`'s doc says "A pub-field-constructed layout failed validation at use time" (target.rs:95-96) — but the bitmap's stride is trusted. `blend_row` checks the destination row length but trusts `xmap`/`src_row` (target.rs:236-249 checks only `dst_row.len()`).
Why it matters: `compose` is the crate's public entry; target.rs:75 promises "no panics on pixel data". A consumer following the crate's own pub-field flexibility pattern gets a slice-out-of-bounds panic in debug AND release. No in-tree trigger exists today (verified: every in-crate producer uses `from_rgba`/`solid` — Grep sweep found zero struct literals outside the constructors), which is why this is S2 not S1.
What correct looks like: a use-time guard in `compose`/`blend_one` (one `rgba.len() != sw*sh*4` check per cue → typed `ComposeError`, mirroring the layout guard), or private fields + accessor.
Confidence: HIGH — Verification: read (panic path is direct slice arithmetic; escape hatch verified by struct definition); not executed (read-only source; no test can be added).
F-002 [S4]
Where: crates/subtitles/src/compositing/compose.rs:49-51 vs 90-99
Code:
```rust
    /// z-ascending (higher z on top; same z: later slice entry on
    /// top). Returns how many cues were drawn.
```
with (compose.rs:94-96) `let Some(dst) = clip(rect, extent) else { continue; };` and (compose.rs:99) `Ok(self.order.len())`
What is wrong: a cue whose placement falls entirely outside the target is skipped (`continue`) but still counted in the return value. "How many cues were drawn" is wrong for off-target cues — a caller using the count to detect "no subtitles this frame" gets a false positive.
What correct looks like: count actual blends in a local, or reword the doc.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/subtitles/src/compositing/compose.rs:120
Code:
```rust
            let sx = ((2 * dx + 1) * sw as u64) / (2 * dw);
```
What is wrong: unchecked u64 multiply — `(2·dx+1) ≤ ~2^33` and `sw ≤ 2^32` can overflow u64 only with both operands near u32::MAX (a ≥3-billion-pixel-wide cue AND target simultaneously — constructible only via F-001's pub-field hatch, since a real 3e9-wide bitmap needs ~36 GB of rgba). Debug-profile overflow panic / release wrap in that absurd case.
What correct looks like: `saturating_mul` (cost-free on this path) or cap placed dims like bitmap.rs:116-122 caps expected lengths.
Confidence: HIGH — Verification: read (range arithmetic).
F-004 [S4]
Where: crates/subtitles/src/compositing/compose.rs:73
Code:
```rust
        let now_ns = now.as_nanos()?.0 as i64;
```
What is wrong: `as_nanos` returns a u64-backed `Nanoseconds` (media-types/src/time.rs:192-197); the `as i64` silently wraps negative for timestamps past i64::MAX nanos (year 2262) instead of surfacing the crate's own `TimeError` discipline. Same cast in bitmap.rs:222 for cue spans. Unrealistic input, silent failure mode.
What correct looks like: `i64::try_from(ns).map_err(...)` into the existing `ComposeError::Time` channel.
Confidence: HIGH — Verification: read + signature check (time.rs:192).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SubtitleCompositor::new / Default / Debug | 20-47 | REAL | - | scratch disclosed in Debug (lengths, not pixels) |
| compose | 63-100 | REAL | F-002 | extent guard, typed time error, (z,idx) sort, per-cue place+clip+blend; count off-by-skipped |
| blend_one | 103-135 | REAL (defensive gap) | F-001/F-003/F-004 | NN map + row loop; bounds chain proven for valid cues |

## 5. Test honesty
No in-file tests. The integration suites exercise this file's public surface directly (all ran green, §2):
| Test (compositing_ac) | Would it fail if code were broken? | Reason |
|---|---|---|
| position_10_90_is_pixel_exact_on_1080p / top_right_quadrant | YES | byte-exact rect + pixel asserts |
| scaling_cif_authored_cue_grows_to_4k / 4k_shrinks_to_cif | YES | exact scaled dims + positions |
| overlap_stacks_by_z_order_not_insertion_luck | YES | swapped insertion orders must yield identical pixels |
| semi_transparent_blends_round_to_nearest | YES | pins the blend formula bytes |
| partial_overlap_has_exact_seam | YES | seam row asserted |
| inactive_cues_draw_nothing | YES | untouched sentinel background |
| compose_steady_state_is_zero_allocation (alloc) | YES | counting allocator asserts zero |
Not covered anywhere: fully-clipped cues (F-002's count), escape-hatch bitmaps (F-001), multi-cue xmap reuse across changed geometry (growth between steady-state frames).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero allocation per frame after warm-up — proven by the counting-allocator AC" | compose.rs:2-4 | REAL | compositing_alloc test ran green (1 passed) |
| "one division per cue column instead of per pixel" (xmap) | compose.rs:26-28 | REAL | compose.rs:118-132: division in map build + per-row sy only |
| "sort_unstable is in-place and allocation-free; the index ... is the explicit stable tie-break" | compose.rs:83-85 | REAL | (u16,u32) tuple sort; Rust sort_unstable allocates nothing |
| "A cue whose placement falls entirely outside the target is skipped, not an error" | compose.rs:61-62 | REAL | clip → None → continue (but see F-002's count) |
| "never panics" / "no panics on pixel data" (crate compositing contract) | target.rs:75, lib.rs | FALSE for pub-field-constructed bitmaps | F-001 |

## 7. Dead / duplicated / unreachable code
`SubtitleCompositor` has ZERO production callers — the "engine glue's job once subtitle frames flow" (compose.rs:6-7) does not exist anywhere in the workspace. Real code, test-only reachability, UNREACHABLE verdict at both crate and call-graph level.

## 8. Dependencies & flags
Uses media-types::time::Timestamp (crate's one allowed dep), crate-internal bitmap/layout/target. No unsafe, no features. `blend_row`/`clip` are pub(crate) — good encapsulation of the raw pixel math.

## 9. Open questions for the next agent
- If/when a renderer adapter lands (renderer-wgpu forwarding rows to `ComposeTarget`), F-001 becomes consumer-reachable: fix before that card. The trait's implementor-side `RowStride` error is the seam to reuse.
- `Rgba8Surface::pixel` reuses `RowStride` as an out-of-bounds error (target.rs:192-201) — a lying error type (`y` field carries x-misses too); cosmetic, but worth renaming when target.rs is audited.
