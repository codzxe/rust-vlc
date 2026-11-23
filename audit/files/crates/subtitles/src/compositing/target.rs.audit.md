# AUDIT: crates/subtitles/src/compositing/target.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 346 / 12 (Rgba consts+with_alpha, ComposeError Display/Error/From, trait ComposeTarget, Rgba8Surface::{new,fill,pixel}, impl ComposeTarget, blend_row, clip, 3 tests) |
| File verdict | UNREACHABLE (the sink trait has exactly ONE implementation in the whole tree: the in-memory test surface; no shipped consumer) |
| Reachable from shipped entry point? | NO — grep `impl ComposeTarget` across crates/: the sole implementor is `Rgba8Surface` at target.rs:213 ("the test harness"); `SubtitleCompositor::compose` (the only thing that consumes a `dyn ComposeTarget`) is called exclusively from subtitles-crate tests (tests/timing_clock.rs, tests/compositing_ac.rs, tests/compositing_alloc.rs, ass/render.rs tests); no video-engine/renderer-wgpu adapter exists; renderer-wgpu's `subtitle_layer: Option<u64>` (params.rs:159,170) is never set to `Some` nor read |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | Grep `impl ComposeTarget|dyn ComposeTarget` (target.rs:213 + compose.rs:67,108 only); Grep `SubtitleCompositor|compose\(` (all callers in-crate tests); Grep `subtitle_layer` (2 hits); Grep `subtitle` in video-engine/src (1 doc comment only, scheduler.rs:163); sed quote verifications (exit 0); baseline citation below |

## 1. One-paragraph verdict
This is the file that answers the hostile-focus question "where do composited overlays GO": nowhere. `ComposeTarget` — "Where composited subtitle pixels go" — has exactly one implementation in the entire repository, `Rgba8Surface`, which the file's own doc calls "the test harness". Every blend that has ever executed in this codebase painted pixels into a test-only in-memory Vec; no renderer, presenter, video sink, or GPU adapter implements the trait, the "later adapter" the module doc promises was never written, and the shipped renderer's only subtitle hook is an `Option<u64>` that no code ever reads. The blend math itself is correct and byte-exact (round-to-nearest `(a·s+(255−a)·d+127)/255`, pinned by tests), the row contract is validated, and the clipping is sound — competently built plumbing with no house at the end of the pipe.

## 2. What actually works (verified)
— Baseline `cargo test -p subtitles` (audit/runs/baseline/cargo-test-workspace.txt:5827-5829): all 3 `compositing::target::tests` pass (`blend_formula_is_round_to_nearest`, `clip_bounds_rect_to_extent`, `surface_row_contract`); the downstream AC/alloc suites that exercise this file's blend through `SubtitleCompositor` (`compositing_ac`, `compositing_alloc`, `timing_clock`) also pass in the same baseline (267 lib + suites, 0 failed). Blend arithmetic re-derived by hand: a=128,s=255,d=0 → (32640+127)/255 = 128 ✓; a=0 skip ✓; a=255 copy ✓.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/compositing/target.rs:1-8 and 149-154
Code:
```
//! [`ComposeTarget`] is what the renderer implements — the ONE trait
//! this contract needs from the presentation world. It hands out
//! destination rows as straight-alpha RGBA8 slices; the blend math
//! lives here, implemented once. No video-engine or renderer-wgpu
//! edit is part of this card: a later adapter forwards the rows (the
//! same shape renderer-wgpu's software compositor already writes).
```
What is wrong: the "later adapter" does not exist anywhere in the tree. Grep proves the sole `impl ComposeTarget` is `Rgba8Surface` (target.rs:213) — the doc itself labels it "the test harness, and the shape a CPU renderer forwards" (line 150), i.e. test scaffolding. The only consumer of `dyn ComposeTarget` is `SubtitleCompositor::compose` (compose.rs:67,108), and every caller of `compose` is a subtitles-crate test. renderer-wgpu has its own unrelated `Compositor` trait (renderer.rs:45) with no bridge, and its `subtitle_layer: Option<u64>` (params.rs:159) is declared, defaulted `None`, and never read — grep 2 hits total. video-engine's only subtitle mention is a doc comment on `PresentAction::Background` (scheduler.rs:163, "The presenter shows background (+ subtitles if any...)") with zero subtitle code behind it. The compositing/mod.rs:7 header likewise defers to "engine glue, a later card" — never written. Net: composited overlays go into a test-only Vec; the shipped app cannot display a single composited subtitle pixel. Confidence: HIGH — Verification: read (all greps exit 0).

F-002 [S3]
Where: crates/subtitles/src/compositing/target.rs:161-177
Code:
```
    pub fn new(extent: (u32, u32), fill: Rgba) -> Result<Rgba8Surface, ComposeError> {
        if extent.0 == 0 || extent.1 == 0 {
            return Err(ComposeError::ZeroExtent { extent });
        }
        let pixels = vec![[fill.r, fill.g, fill.b, fill.a]; extent.0 as usize * extent.1 as usize]
            .into_flattened();
```
What is wrong: unbounded allocation from the extent — `extent.0 * extent.1 * 4` bytes with no ceiling (a 65536×65536 surface = 17 GiB). Today only tests construct surfaces with small extents, and the trait's real sink (which would hand the compositor a frame-sized target) does not exist, so the blast radius is latent; but this type is simultaneously advertised as "the shape a CPU renderer forwards" (line 150), and any future adapter copying that shape inherits an uncap'd constructor. What correct looks like: a `MAX_SURFACE_BYTES` ceiling with a typed error, mirroring srt/render.rs's MAX_PROBE_PIXELS discipline 20 lines away in the same crate. Confidence: HIGH — Verification: read (allocation not executed).

F-003 [S4]
Where: crates/subtitles/src/compositing/target.rs:189-202
Code:
```
    /// # Errors
    /// [`ComposeError::RowStride`] — misused as an out-of-bounds
    /// signal; the coordinates of the AC tests are in-range by
    /// construction, and a miss is a placement defect.
    pub fn pixel(&self, x: u32, y: u32) -> Result<Rgba, ComposeError> {
        if x >= self.width || y >= self.height {
            return Err(ComposeError::RowStride {
                y,
                expected: 0,
                got: usize::MAX,
```
What is wrong: error-type abuse, self-documented: an out-of-bounds pixel read reports "target row {y} stride mismatch: expected 0 bytes, got 18446744073709551615" — a fabricated stride violation that would send any debugger in the wrong direction. Documented, test-only vocabulary, hence S4. What correct looks like: an `OutOfBounds { x, y }` variant. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/subtitles/src/compositing/target.rs:12-18 and 236-272
Code:
```
//! The destination is an OPAQUE video plane: each touched pixel
//! becomes `(a * s + (255 - a) * d + 127) / 255` per RGB channel with
//! alpha set to 255 (`a = 0` skips, `a = 255` copies). Rounding is to
//! nearest. Byte space, no gamma conversion — the point-sampling
//! honesty of the software path; linear-light blending belongs to the
//! GPU shader path. Over an opaque background the operation is
//! associative, so z-stacked cues compose identically one-by-one.
```
What is wrong: two nits. (1) The opaque-plane invariant is not enforced anywhere — `Rgba8Surface::new((w,h), Rgba{a:0,..})` happily constructs a transparent destination, and `blend_row` then writes alpha 255 on touched pixels while skipping (leaving alpha 0) elsewhere — a half-transparent surface yields inconsistent alpha. (2) The word "associative" is mathematically false for this integer-rounded blend ((A over B) over C ≠ A over (B over C) up to ±1 rounding); what is true is that sequential z-ordered blending is well-defined, which is what the code does. Comment nit only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Rgba::{BLACK,WHITE,RED,BLUE}` | 37-65 | REAL | — | Constants. |
| `Rgba::with_alpha` | 67-72 | REAL | — | Must_use, fine. |
| `ComposeError` Display/Error/From | 104-128 | REAL | — | Complete impls. |
| `trait ComposeTarget` | 136-147 | REAL (unimplemented seam) | S2 | One trait, one test implementor (F-001). |
| `Rgba8Surface::new` | 166-177 | REAL | S3 | Unbounded alloc (F-002). |
| `Rgba8Surface::fill` | 180-187 | REAL | — | Chunked, fine. |
| `Rgba8Surface::pixel` | 195-210 | REAL | S4 | Error-type misuse (F-003). |
| `impl ComposeTarget for Rgba8Surface` | 213-230 | REAL | — | Correct row contract, y bounds checked. |
| `blend_row` | 236-272 | REAL | — | Byte-exact formula; upfront length check; trusts xmap (pub(crate), sole caller clamps at compose.rs:121). |
| `clip` | 275-289 | REAL | — | Saturating, correct empty → None. |
| 3 tests | 296-345 | REAL | — | Honest, exact. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| blend_formula_is_round_to_nearest | YES | Exact bytes for a=128/0/255 cases. |
| clip_bounds_rect_to_extent | YES | Exact rects incl. fully-outside → None. |
| surface_row_contract | YES | Row length + out-of-range rejection. |
All pass in baseline (:5827-5829). None can catch F-001 (a wiring gap is invisible to in-crate tests by construction).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a later adapter forwards the rows (the same shape renderer-wgpu's software compositor already writes)" | target.rs:6-8 | PAPER-ONLY | No adapter exists; renderer-wgpu's compositor is its own trait with no subtitle input (F-001). |
| "Rgba8Surface ... the test harness, and the shape a CPU renderer forwards" | target.rs:149-150 | Half-TRUE | Test harness: yes. Forwarded shape: no forwarder exists. |
| "The presenter shows background (+ subtitles if any — doc 03 §4.10 d)" | video-engine/scheduler.rs:163 | FALSE for this tree | Video engine has zero subtitle code (grep: 1 comment). |
| "engine glue, a later card" (pipeline caller) | compositing/mod.rs:7 | PAPER-ONLY | Never written (F-001). |
| Blend formula/rounding | target.rs:12-18 | REAL | Test-pinned + hand-derived. |

## 7. Dead / duplicated / unreachable code
The entire sink path is dead from the shipped-binary perspective (F-001). `blend_row`/`clip` are pub(crate) with exactly one caller each (compose.rs) — fine. No duplication found.

## 8. Dependencies & flags
Depends on compositing::bitmap/layout + media-types time. No unsafe, no I/O, no features. The `From<TimeError>` conversion is consumed by compose.rs's `now.as_nanos()?`.

## 9. Open questions for the next agent
- compose.rs auditor: the z-tie-break claim ("later cue wins ties") vs vobsub_cues' `z.saturating_add(i)` — do two vobsub batches with enough cues collide in u16 z space (>65535 cues)?
- renderer-wgpu auditor: is the `Compositor` trait's software path structurally able to accept an external row provider, or would the subtitle adapter need renderer.rs surgery?
- Who owns doc 03 §4.10 d — the presenter-background-with-subtitles claim repeated at scheduler.rs:163 should be reconciled or deleted.
