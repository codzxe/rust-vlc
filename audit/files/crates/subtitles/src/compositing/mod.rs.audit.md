# AUDIT: crates/subtitles/src/compositing/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 37 / 0 prod (facade: 5 pub mods, 4 re-export groups) |
| File verdict | REAL-UNPROVEN in product — the overlay pipeline behind this facade is REAL and executed byte-exact by me, but NO renderer implements the seam and no shipped entry point links the crate |
| Reachable from shipped entry point? | NO — `ComposeTarget` has exactly ONE implementor repo-wide: the crate's own `Rgba8Surface` (target.rs:213); rg over crates/ + apps/ finds no other; crate unlinked (parser.rs F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rustc probe (audit/runs/probe_comp.rs vs prebuilt rlibs, exit 0 — blend/z/timing outputs below); rg 'impl ComposeTarget' (exit 0: 1 hit); rg matrix compositing row (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
Hostile-focus answer: this is REAL alpha blending, not boxes. I compiled a probe against the
prebuilt rlib and drove the full pipeline — timing filter, placement, clip, scale, blend:
a=128 red over white produced exactly (255,127,127,255), the over-operator value
`(128·s + 127·d + 127)/255`; red z=1 stacked over blue z=0 left red on top; a two-cue
semi-transparent-over-opaque overlap produced byte-exact (128,0,127,255); the half-open
timing filter dropped the cue at t=end. The facade's module map matches reality (every named
child item exists), and the compositing matrix row claims "partial", not "implemented" —
honest. The one structural defect: the `ComposeTarget` seam this whole contract exists to
serve has no implementor outside the crate — the "later adapter" that would forward rows to
renderer-wgpu never landed, so the pipeline's only destination ever exercised is its own test
surface.

## 2. What actually works (verified)
— probe_comp (exit 0): `drawn=1`, pixels `(3,3) r=255 g=127 b=127 a=255` (red a=128 over
  white — formula-exact), `(4,3) r=0 g=0 b=255` (opaque copy), untouched white elsewhere;
  `at 2s drawn=0` (half-open end); z-order red-over-blue → `(255,0,0)`; overlap blend
  red a=128 over opaque blue → `(128,0,127,255)` — exactly `(128·255+127)/255=128` /
  `(127·255+127)/255=127`. Placement: 2×1 cue bottom-center on 8×4 → row 3, x=3 — correct.
— Baseline: compositing unit tests 17/17 (cargo-test-workspace.txt:5817-5833),
  compositing_ac 8/8 incl. `semi_transparent_blends_round_to_nearest`,
  `overlap_stacks_by_z_order_not_insertion_luck`, `partial_overlap_has_exact_seam`
  (6039-6052), compositing_alloc `compose_steady_state_is_zero_allocation` (6056),
  roundtrip matrix 5/5 (6095).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/compositing/target.rs:213 (the only impl) vs crates/subtitles/src/compositing/mod.rs:4-11
Code: (verbatim, mod.rs pipeline doc)
```
//! Pipeline shape: a caller (engine glue, a later card) holds decoded
//! cues as [`CompositedCue`]s — an RGBA8 [`CueBitmap`] carrying its
```
and (verbatim, target.rs:6-8)
```
//! this contract needs from the presentation world. It hands out
//! destination rows as straight-alpha RGBA8 slices; the blend math
//! lives here, implemented once. No video-engine or renderer-wgpu
```
What is wrong: `rg 'impl ComposeTarget'` over the entire workspace returns exactly one hit —
the in-crate reference surface. The "engine glue / later adapter" caller the docs describe
does not exist; renderer-wgpu and video-engine contain zero references to ComposeTarget.
The overlay pipeline is real, tested math whose only destination ever exercised is its own
test harness — a one-sided seam (same pattern as font/mod.rs F-002). The docs label the
adapter as future work (honest) and the matrix row says "partial"
(docs/feature-matrix.toml:1123), so this is dead-seam debt, not a false claim.
What correct looks like: a renderer-wgpu ComposeTarget adapter, or the seam marked
explicitly unwired in the module map. Confidence: HIGH — Verification: ran rg.
Zero-findings justification for the rest — checked and how:
- Blend math: executed byte-exact (three alpha cases + stacking, §2) against target.rs:250-270.
- Nearest-neighbor scaling map `xmap[dx] = floor((2dx+1)·sw/(2dw))` (compose.rs:26-28,119-122):
  identity at 1:1 verified algebraically; min(sw−1) clamp present both axes.
- Z tie-break: sort on (z, index) then bottom-up draw (compose.rs:78-98) — later-index-on-top
  confirmed by code order and the baseline z-order AC.
- Scratch reuse: order/xmap clear-then-push (compose.rs:77-85,118-122) — zero-alloc pinned by
  the counting-allocator AC (baseline 6056).
- Re-exports (mod.rs:31-37) vs child pub items: verified present (rg).
- clip() saturation: saturating_add before min (target.rs:278-279) — no overflow path.

## 4. Per-function table
Not applicable (0 functions; facade under 100 LOC). Children audited for this verdict:
target.rs (read fully — blend/clip/surface REAL), compose.rs (read fully — driver REAL),
timing.rs (read fully — REAL), bitmap.rs/layout.rs (API + key functions + 17 baseline tests).

## 5. Test honesty
No tests in this file. Child/AC suites (17 unit + 8 AC + 1 alloc + 5 roundtrip) assert exact
pixel values and seams — they would fail on any blend/placement/timing regression; my probe
reproduces their central claims independently.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the compositor filters by the half-open [t0,t1) interval, sorts by z, computes each cue's destination rectangle and blends" | mod.rs:8-11 | REAL | executed end-to-end (probe_comp) |
| "the over-opaque blend" `(a*s+(255-a)*d+127)/255` | target.rs:13-14 | REAL | executed byte-exact: (255,127,127) and (128,0,127) |
| "Over an opaque background the operation is associative" | target.rs:17-18 | REAL | affine-composition argument: dst' = a·s+(1−a)·dst ignores dst α — grouping-independent |
| "zero allocation per frame after warm-up" | compose.rs:2-4 | REAL (test-pinned) | baseline compose_steady_state_is_zero_allocation ok |
| "No video-engine or renderer-wgpu edit is part of this card: a later adapter forwards the rows" | target.rs:6-8 | REAL (admitted gap) — but the adapter still absent | rg: no ComposeTarget impl outside crate (F-001) |
| matrix compositing row "partial" ×3 platforms | docs/feature-matrix.toml:1123 | REAL | consistent with the unwired seam — the honest status, unlike the subtitle-format rows' "implemented" |

## 7. Dead / duplicated / unreachable code
The `ComposeTarget` trait + reference surface are reachable only from tests/probes (F-001).
No dead items in the facade; all re-exports live.

## 8. Dependencies & flags
media-types (Timestamp/TimeSpan — the ONLY external dep of the crate); std otherwise; no
unsafe; no features. Child modules: bitmap/compose/layout/target/timing — all present with
real content (verified by read + baseline).

## 9. Open questions for the next agent
- layout.rs and bitmap.rs were only partially read (API + key fns): their safe-area and
  rational-anchor math is pinned by 9 baseline tests + the 176-cue roundtrip matrix, but not
  independently audited by me.
- If an adapter ever lands, re-run probe_comp against the real renderer surface; the
  byte-exact expectations are in §2.
