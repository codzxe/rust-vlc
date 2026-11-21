# AUDIT: crates/subtitles/src/compositing/target.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-21 |
| Lines / functions | 346 / 15 (+3 test fns) |
| File verdict | UNREACHABLE (real, test-pinned blend math; the R4 seam it defines has NO implementor outside in-crate tests — no renderer-wgpu/video-engine adapter exists; crate unshipped) |
| Reachable from shipped entry point? | NO. In-crate chain: `SubtitleCompositor::compose` (compose.rs:67) → `blend_one` → `clip` + `blend_row` + `ComposeTarget::row_rgba`; callers of `compose`: tests/{timing_clock, compositing_ac, compositing_alloc}.rs only. `rg "ComposeTarget" ` across repo: definitions/re-exports/uses confined to crates/subtitles — **the "renderer adapter (a later card)" never landed**; the sole implementor is the in-crate `Rgba8Surface` used by tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "blend_row\|Rgba8Surface\|ComposeTarget" crates/subtitles` (callers: compose.rs + tests only) · `rg -n "ComposeTarget"` repo-wide (no external implementor) · read compose.rs:85-136 (caller bounds for blend_row inputs) · `rg -n "MAX_BITMAP_BYTES"` (crate's own bitmap cap for contrast) · baseline: `orchrun-test-workspace-nofailfast.log.txt:5357-5359` (target 3/3 ok) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
The blend core is the most honest code in this crate slice: the round-to-nearest over-opaque formula is exactly what the doc says, pinned byte-for-byte by tests that would fail under any formula mutation, bounds are typed (dst row length checked), `clip` is saturating and correct, and the z-order semantics live in the caller (compose.rs:50 "z-ascending, higher z on top, later slice entry wins ties") and are test-pinned. But the file's reason to exist — the R4 `ComposeTarget` seam — is a trait with zero presentation-world implementors: the "renderer adapter (a later card)" the docs promise was never written, so the only `ComposeTarget` in the repository is the in-crate test surface. On top of that: the public `Rgba8Surface::new` has no extent cap (a 65536×65536 request attempts a 16 GiB allocation → capacity-overflow panic/OOM abort, while sibling modules cap bitmaps at 64 MiB), the "associative" blend claim is mathematically wrong for the rounded byte-space operator, `ComposeError::RowStride` is a dumpster variant reused for three unrelated failures with sentinel values, and `blend_row` validates the destination row but indexes the source row unguarded.

## 2. What actually works (verified)
— `compositing::target::tests` 3/3 ok in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5357-5359`; the wider compositing suites (layout 6, bitmap 3, timing, compositing_ac incl. `overlap_stacks_by_z_order_not_insertion_luck`) all green in the same run.
— Blend formula arithmetic verified by hand: a=128, s=255, d=0 → (128·255+127·0+127)/255 = 32767/255 = 128 ✓ matches the test's `[128,128,128,255]`; a=0 skip and a=255 copy arms traced.
— `clip` overflow safety verified: `saturating_add` + `min` + the `x0>=x1` empty test; three-case test pins inside/partial/none.
— Caller bounds for `blend_row` inputs verified at compose.rs:118-132: `xmap` entries ≤ `sw-1` (`.min(sw as u64 - 1)`, line 121), `dst.x as usize` ≤ extent.0 (u32) — the theoretical `(dst_x + xmap.len()) * 4` overflow cannot occur from the only caller.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/compositing/target.rs:166-177
Code:
```rust
    pub fn new(extent: (u32, u32), fill: Rgba) -> Result<Rgba8Surface, ComposeError> {
        if extent.0 == 0 || extent.1 == 0 {
            return Err(ComposeError::ZeroExtent { extent });
        }
        let pixels = vec![[fill.r, fill.g, fill.b, fill.a]; extent.0 as usize * extent.1 as usize]
            .into_flattened();
```
What is wrong: The only input validation on this PUBLIC constructor is the zero check. A `(65536, 65536)` extent asks `vec!` for 2^32 elements × 4 bytes = 16 GiB: depending on the allocator this aborts the process (capacity overflow / OOM) instead of returning the module's typed errors. Sibling paths in the same crate cap their file-influenced allocations — `ass/render.rs:77` `MAX_BITMAP_BYTES = 64 MiB`, dvb/vobsub have their own caps — the reference compositing target has none.
Why it matters: The surface's intended extent is the VIDEO extent (compose.rs passes `target.extent()` through; the docs frame it as the shape "a CPU renderer forwards") — i.e., file-controlled dimensions in the intended wiring. Today's callers are tests with 64×64…3840×2160 extents, so the blast radius is zero — which is the only reason this is S3 and not S1.
What correct looks like: `if extent.0 as u64 * extent.1 as u64 * 4 > MAX_SURFACE_BYTES { return Err(...) }` with the same 64 MiB-class cap the crate uses elsewhere (or a documented dim cap like 16384×16384).
Confidence: HIGH — Verification: read; no run (would abort the sandbox test process — deliberately not run).

### F-002 [S4]
Where: crates/subtitles/src/compositing/target.rs:17-18
Code:
```rust
//! honesty of the software path; linear-light blending belongs to the
//! GPU shader path. Over an opaque background the operation is
//! associative, so z-stacked cues compose identically one-by-one.
```
What is wrong: The rounded byte-space operator is not associative: compositing c2 over black then c1 over the result (a1=a2=128, s1=255, s2=100) yields 153, the mirrored order 114 — rounding also breaks regrouping (one-by-one vs pre-composited can differ by 1 LSB). What is actually true: over an opaque plane each blend is a well-defined color→color map, so a FIXED z order (compose.rs sorts z-ascending, blends bottom-up) gives a deterministic result — that is what the code does and what `overlap_stacks_by_z_order_not_insertion_luck` pins.
Why it matters: The doc justifies the one-by-one semantics with a false mathematical property; a reader "proving" stack correctness from associativity would be misled.
What correct looks like: "z-stacked cues are blended sequentially bottom-up in z order; the result is order-dependent by design and deterministic; rounding may differ by ±1 LSB from a pre-composited stack."
Confidence: HIGH — Verification: hand-computed counterexample (153 vs 114) + read of compose.rs:85-98.
### F-003 [S4]
Where: crates/subtitles/src/compositing/target.rs:189-201, 242-249
Code:
```rust
    /// [`ComposeError::RowStride`] — misused as an out-of-bounds
    /// signal; the coordinates of the AC tests are in-range by
    /// construction, and a miss is a placement defect.
```
```rust
    let end = (dst_x + xmap.len()) * 4;
    if end > dst_row.len() {
        return Err(ComposeError::RowStride {
            y: 0,
```
What is wrong: `RowStride` carries three unrelated failures: a real stride mismatch (its documented purpose), an out-of-bounds `pixel()` read (sentinels `expected: 0, got: usize::MAX`), and a blend-row width overflow (fake `y: 0`). A caller seeing `RowStride` cannot tell a renderer protocol bug from a placement defect from a compositor overrun without decoding sentinel values.
Why it matters: The crate's selling point is typed, diagnostic errors (MP §8); this variant is a diagnostic dead end. The doc's own "misused" admission confirms the smell.
What correct looks like: `ComposeError::OutOfBounds { x, y }` and `ComposeError::BlendOverflow { needed, got }` variants.
Confidence: HIGH — Verification: read.
### F-004 [S4]
Where: crates/subtitles/src/compositing/target.rs:250-252
Code:
```rust
    for (i, &sx) in xmap.iter().enumerate() {
        let si = sx as usize * 4;
        let a = src_row[si + 3] as u32;
```
What is wrong: `blend_row` validates the DESTINATION row length (typed `RowStride` error, lines 242-249) but indexes the SOURCE row unguarded: a `src_row` shorter than `(max(xmap)+1)*4` panics on `src_row[si + 3]` — violating the module's "no panics on pixel data" banner (line 75) for a pub(crate) caller that breaks the xmap≤sw−1 invariant.
Why it matters: Defense-in-depth asymmetry in the one function both reference and future GPU-adapter paths share; today only compose.rs calls it and holds the invariant (verified at compose.rs:121), so this is latent.
What correct looks like: Symmetric typed check: `if (xmap.iter().max().map_or(0, |m| *m as usize + 1)) * 4 > src_row.len() { return Err(...) }`.
Confidence: HIGH — Verification: read + invariant trace at the sole caller.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Rgba` (struct) + `BLACK/WHITE/RED/BLUE` | 25-65 | REAL | — | Fixture palette; RED/BLUE used by AC overlap tests |
| `Rgba::with_alpha` | 67-72 | REAL | — | const, must_use note honest |
| `ComposeError` (enum, 4 variants) | 76-102 | REAL | S4 | RowStride conflation (F-003) |
| `Display for ComposeError` | 104-120 | REAL | — | Correct strings; InvalidLayout drops the source detail in Display |
| `impl Error` / `From<TimeError>` | 122-128 | REAL | — | Trivial; TimeError bridging for the clock read |
| `ComposeTarget` (trait) | 136-147 | REAL | — | The R4 seam; NO implementor outside tests (reachability, header) |
| `Rgba8Surface::new` | 166-177 | REAL | S3 | Unbounded allocation (F-001) |
| `Rgba8Surface::fill` | 180-187 | REAL | — | as_chunks_mut, correct |
| `Rgba8Surface::pixel` | 195-210 | REAL | S4 | OOB → sentinel RowStride (F-003) |
| `ComposeTarget for Rgba8Surface::extent` | 214-216 | REAL | — | Trivial |
| `ComposeTarget for Rgba8Surface::row_rgba` | 218-229 | REAL | — | y checked; slicing arithmetic exact (stride = width*4) |
| `blend_row` | 236-272 | REAL | S4 | Formula correct + test-pinned; dst checked, src not (F-004); a=0 skip / a=255 copy fast paths correct |
| `clip` | 275-289 | REAL | — | Saturating, 3-case test-pinned |
| test `blend_formula_is_round_to_nearest` | 296-315 | REAL | — | Byte-exact; fails under formula/rounding mutation |
| test `clip_bounds_rect_to_extent` | 317-336 | REAL | — | inside/partial/none |
| test `surface_row_contract` | 338-346 | REAL | — | Row length + typed OOB |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| blend_formula_is_round_to_nearest | YES | Exact bytes for three alpha regimes; any formula drift (e.g. `/256`, truncation, no +127) changes them |
| clip_bounds_rect_to_extent | YES | Partial-clip and empty-clip cases pin the arithmetic |
| surface_row_contract | YES | Row length 16 for width 4 + typed OOB at y=2 |
| (cross-file) overlap_stacks_by_z_order… | YES | Z order and tie rules asserted with different z/inorder permutations — would fail if compose sorted by slice order |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Blend formula `(a*s+(255-a)*d+127)/255`, a=0 skip, a=255 copy, dst alpha 255 | target.rs:12-16 | REAL | Test-pinned byte-exact (§2); hand-verified |
| "Over an opaque background the operation is associative, so z-stacked cues compose identically one-by-one" | target.rs:17-18 | FALSE (math) / REAL (behavior) | F-002: counterexample 153≠114; actual bottom-up z semantics correct + pinned |
| "no panics on pixel data" | target.rs:75 | SHALLOW | Holds for all current callers; `blend_row` src indexing panic-capable (F-004), `Rgba8Surface::new` abort-capable (F-001) |
| "a later adapter forwards the rows" (renderer adapter) | target.rs:7-8, lib.rs:21-26 | PAPER-ONLY | No adapter exists; ComposeTarget implemented only by in-crate Rgba8Surface (rg repo-wide) |
| Cargo.toml description: "z-ordered alpha blending onto a renderer-implemented ComposeTarget" | crates/subtitles/Cargo.toml:36 | FALSE (today) | No renderer implements it |
| Matrix row textrender-subtitle-rasterizer evidence: "CueBitmap/CueLayout/place_cue pixel-exact math" | feature-matrix.toml:1124 | REAL (math) / UNREACHABLE (row) | Tests green in baseline; no shipped path reaches any of it |

## 7. Dead / duplicated / unreachable code
- `Rgba::RED`/`BLUE` documented as AC fixtures — used by compositing_ac.rs (verified).
- The `ComposeTarget` trait: production-dead seam (header). `blend_row`/`clip`: reachable only from compose.rs ← tests.
- No duplication in-file; `Rgba` duplicates the MODEL's `Color` (model/style.rs:16-25, same 4×u8 straight alpha) — two identical RGBA8 types in one crate (model::style::Color vs compositing::Rgba); ass/render.rs converts between them (`ass_colour_to_model`, worklog). Cross-module duplication note for the mod owner.

## 8. Dependencies & flags
- Uses `media_types::time::TimeError` (the crate's one declared dep — actually used here, unlike font/mod.rs). No features, no unsafe. `as_chunks_mut`/`into_flattened` — modern std, fine on the pinned toolchain.

## 9. Open questions for the next agent
- compose.rs / mod.rs owner: is the `Rgba` vs `model::style::Color` duplication deliberate (compositing independent of the model) or drift? One conversion function exists in ass/render.rs — should the model own the one true RGBA8?
- If the renderer adapter ever lands (renderer-wgpu implementing ComposeTarget), F-001's extent cap becomes load-bearing (video extent is demux-controlled) — cap before wiring.
