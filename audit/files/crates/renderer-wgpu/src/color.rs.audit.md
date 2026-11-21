# AUDIT: crates/renderer-wgpu/src/color.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 176 / 8 units (SurfaceFormat enum + 4 methods, Rgba + 3 consts + 2 methods) + 3 in-file tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the format/pixel vocabulary (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:1317` `SurfaceFormat::Bgra8Srgb` → renderer surface config; `Rgba::BLACK` default background at `params.rs:169`, consumed by the compositor per present). `source_over` itself: NO caller today (Phase-5 overlay primitive, rg-verified). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read of full file; `sed -n` quote verification (96-119); hand-recomputation of every `source_over` test vector (128-over-black → 100/50/25; 128-black-over-white → 127; blend rounding `(t+127)/255`); `rg` callers of `source_over` (0 outside color.rs) and of `Rgba::`/`SurfaceFormat::` (params.rs:169, backend.rs:240-297, pipeline_host.rs:1317); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:4766+` — `color::tests::{wgpu_format_names_are_the_real_enum_variants, swizzles_round_trip_per_pixel, source_over_covers_the_alpha_edges_and_a_middle_case}` all `ok`. |

## 1. One-paragraph verdict
The color vocabulary behind the shipped renderer is small and mostly bulletproof: the two sRGB surface formats, byte-exact BGRA/RGBA swizzles with a round-trip proof, and a rounded 8-bit blend — all three in-file tests ran green in the no-fail-fast baseline, and I recomputed every asserted blend value by hand and they are exact. The one real defect is `source_over`: it implements source-over **only for an opaque `below`** — the output alpha is hard-coded to 255 and `below`'s alpha byte is ignored entirely — while the doc sells it as general "source-over alpha compositing … in 8-bit straight alpha" that the Phase-5 subtitle card can "consume as a proven function". Blend a 50%-red over a *transparent* below and you get `(128, 0, 0, 255)` instead of `(255, 0, 0, 128)`: wrong color, wrong alpha. The test suite "proves" the function only over opaque backgrounds, so the unproven half is exactly the half the doc promises. No caller exists today, which keeps this at S3 instead of S2.

## 2. What actually works (verified)
— All 3 tests executed ok in baseline (`orchrun-test-workspace-nofailfast.log.txt`, renderer_wgpu lib suite): format-name pin, swizzle round-trip + real-swap assert, source-over edges.
— Swizzle math verified: `pack`/`unpack` BGRA byte-reversal is an involution; the `Bgra8Srgb.pack([1,2,3,4]) == [3,2,1,4]` assert pins that it is a real swap, not an identity (150-153).
— Blend rounding recomputed exactly: `t = top·a + bottom·(255−a)`, `(t+127)/255` — 200@128 over black → (200·128+127)/255 = 100 (test asserts 100); 0@128 over white → (255·127+127)/255 = 127 (test asserts 127). Both match; the arithmetic is round-to-nearest and overflow-free in u16 (max 255·255 = 65025 < 65535).
— The vocabulary is live in the shipped lane: `SurfaceFormat::Bgra8Srgb` configures the CLI's headless surface (pipeline_host.rs:1317); `Rgba::BLACK` is the default letterbox background (params.rs:169) the software compositor fills with on every present.
— The sRGB passthrough contract (module docs 4-14: no double conversion, byte-exact at 1:1) is executed by renderer.rs's round-trip AC (`presenter_round_trip_byte_exact_and_reports_the_slot`, baseline ok).

## 3. Findings

### F-001 [S3]
Where: crates/renderer-wgpu/src/color.rs:96-119 (hard-coded alpha at 117; contract at 96-99)
Code:
```
    /// Source-over alpha compositing of `self` over `below` in 8-bit
    /// straight alpha — the subtitle-layer blend primitive (doc 03
    /// §4.11 "the video shader composites"), unit-tested here so the
    /// Phase-5 overlay card consumes a proven function.
    #[must_use = "pure compositing math"]
    pub fn source_over(self, below: Rgba) -> Rgba {
```
and the tail that gives the game away:
```
        Rgba([
            blend(r, below.0[0]),
            blend(g, below.0[1]),
            blend(b, below.0[2]),
            255,
        ])
```
What is wrong: the output alpha is hard-coded `255` and `below.0[3]` is never read. True straight-alpha source-over is `out_a = a + below_a·(255−a)/255`, `out_c = (top·a + below_c·below_a·(255−a)/255)/out_a`. This implementation is that formula only when `below_a == 255`. For any non-opaque `below` the result is wrong in both channels and alpha: e.g. `Rgba([255,0,0,128]).source_over(Rgba::TRANSPARENT)` returns `(128,0,0,255)` where source-over yields `(255,0,0,128)`.
Why it matters: the doc's "unit-tested here so the Phase-5 overlay card consumes a **proven** function" is the defect — the test (`source_over_covers_the_alpha_edges_and_a_middle_case`, 157-175) exercises only opaque `below` (black/white/opaque-blue), so the function is proven for exactly the subset where it is correct and unproven for the class of inputs (transparent/partial backgrounds — a real thing wherever letterbox or transparent surface regions meet a subtitle) where it silently produces wrong pixels. S3 not S2 solely because rg shows zero callers today.
What correct looks like: implement the two-term formula (or `debug_assert_eq!(below.0[3], 255)` + rename to `source_over_opaque` so the contract is in the name), and add the transparent-below and partial-below test vectors.
Confidence: HIGH — Verification: read + hand-recomputation of the counterexample; counterexample not executed (no test harness run — baseline only runs the existing tests, which avoid the case).

### F-002 [S4]
Where: crates/renderer-wgpu/src/color.rs:127-141
Code:
```
    #[test]
    fn wgpu_format_names_are_the_real_enum_variants() {
        // Shape-level pin of the hardware-backend mapping: these are
        // the literal wgpu::TextureFormat variant names (wgpu 26–30;
        // Bgra8UnormSrgb is the canonical swapchain format on
        // Vulkan/GL, Rgba8UnormSrgb on Metal).
```
What is wrong: the test asserts that `wgpu_texture_format()` returns the same string literals that the function body contains — a self-referential pin. It cannot fail unless someone edits one of the two sites, and it proves nothing about wgpu (not a dependency; the names' correctness rests on the comment's claim, not on any compile-time or runtime check against the real enum).
Why it matters: the module doc (21-25) presents this as "the exact wgpu 26/30 enum variant name … pinned by unit test before the dependency exists" — the pin guards against in-repo drift only; when the Lead-gated import lands, a real `match` against `wgpu::TextureFormat::from_str`-equivalent or a typed constant should replace the string, and nothing in the code forces that day to come.
What correct looks like: when wgpu lands, replace `&'static str` with `wgpu::TextureFormat` and delete the string pin; until then, the honest wording is "in-repo drift pin", which the test comment half-admits.
Confidence: HIGH — Verification: read; test self-reference traced.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-25 | REAL | — | sRGB passthrough contract (executed by renderer.rs round-trip AC); point-sampling rationale for skipping gamma conversion is sound for nearest-neighbor. |
| `SurfaceFormat` enum | 32-39 | REAL | — | Two sRGB 8-bit variants; matches the two the shipped lane uses. |
| `wgpu_texture_format` | 44-50 | REAL-UNPROVEN | S4 | String contract pre-dependency; **self-referential pin (F-002)**; names are the real wgpu variants per doc claim — UNVERIFIED against wgpu itself (not a dep). |
| `bytes_per_pixel` | 53-56 | REAL | — | Constant 4; asserted. |
| `pack` / `unpack` | 59-65 / 68-74 | REAL | — | BGRA involution verified; real-swap assert present. |
| `Rgba` consts | 84-88 | REAL | — | BLACK/WHITE used in shipped compositing; TRANSPARENT used by tests. |
| `from_components` | 91-94 | REAL | — | Const constructor, trivial. |
| `source_over` | 100-119 | SHALLOW | S3 | **Correct only for opaque `below`; hard-coded 255 alpha (F-001)**; edge cases (a=0/255) correct; rounding exact; zero callers today. |

## 5. Test honesty (3 tests, all executed ok — baseline renderer_wgpu lib)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `wgpu_format_names_are_the_real_enum_variants` | Only against in-repo drift | Self-referential (F-002): asserts the function's own literals back. Would catch an accidental rename of the mapping, not a wrong mapping. |
| `swizzles_round_trip_per_pixel` | YES | Round-trip + the `[3,2,1,4]` swap assert — a broken swizzle fails both. |
| `source_over_covers_the_alpha_edges_and_a_middle_case` | YES for the opaque-below subset | Edges (a=255, a=0) and three blend vectors recomputed exact. Would NOT catch F-001 — every `below` in it is opaque; the transparent-below case is absent by construction. |
| Mutation check (reasoned): swap `pack`'s BGRA arms | CAUGHT | by the swap assert + round-trip. |
| Mutation check (reasoned): change rounding to `(t)/255` | CAUGHT | 128-over-white would yield 127→127? recomputed: t=32385, /255 = 127.0 → 127 — survives! 200@128/black: t=25600/255 = 100.39 → 100 — survives. **The rounding mode is under-pinned**: truncation vs round-to-nearest differ only for t mod 255 > 127 cases, e.g. top=255, a=128, bottom=0: t=32640, +127 → 32767/255=128 vs 32640/255=128 — same; top=1, a=255? edge. Finding: the test vectors happen to be rounding-invariant — noted, no separate finding (the difference is ≤1 LSB and no vector distinguishes). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the pipeline is a *passthrough* — proven byte-exactly by the headless round-trip AC" | module docs 10-14 | REAL | `renderer::tests::presenter_round_trip_byte_exact_and_reports_the_slot` (baseline ok) asserts readback == input bytes at 1:1. |
| "v0 software compositor uses point sampling … needs no gamma-domain conversion" | module docs 15-20 | REAL | backend.rs nearest-neighbor (its `nearest_math_is_bounded_identity_and_center` test, baseline ok); no gamma math anywhere in the crate (rg). |
| "returns the exact wgpu 26/30 enum variant name … pinned by unit test before the dependency exists" | module docs 21-25 | REAL-UNPROVEN | Names match wgpu's documented variants per the comment's claim (UNVERIFIED against the crate — not a dependency); the pin is in-repo only (F-002). |
| "unit-tested here so the Phase-5 overlay card consumes a proven function" (source_over) | doc at 96-99 | SHALLOW | Proven for opaque `below` only; wrong output for non-opaque `below` (F-001); zero callers today. |

## 7. Dead / duplicated / unreachable code
- `source_over` — no callers (Phase-5 primitive); carries F-001's contract gap.
- `Rgba::TRANSPARENT` — used only by tests today (surfaces with alpha are not configured in v0); harmless constant.
- No duplication.

## 8. Dependencies & flags
- Std-only, no deps beyond the crate's media-types/video-engine edges; no feature gates; no unsafe.
- The wgpu-name mapping is string-level pending the Lead-gated import (crate docs / records/wgpu-30.0.1.md).

## 9. Open questions for the next agent
- When the wgpu import lands, F-002's string seam and F-001's blend must be revisited together — the overlay card's tests should include non-opaque `below` vectors before trusting `source_over`.
- `crates/renderer-wgpu/src/backend.rs` (the SoftwareCompositor that consumes these formats/backgrounds) is unaudited by me — its per-pixel loops are where any swizzle misuse would actually render.
