# AUDIT: crates/audio-engine/src/channel_map.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 270 / 9 production fns + 7 tests |
| File verdict | REAL — identity + apply are on the shipped audio hot path; the four non-identity constructors are REAL code with test-only callers (production-dead surface) |
| Reachable from shipped entry point? | YES for `identity`/`apply`: `AudioPipeline::new` (audio-engine/src/pipeline.rs:117-119) builds identity maps; `process` applies them every staged frame (pipeline.rs:300). NO for `swap_stereo`/`mono_to_stereo`/`stereo_to_mono`/`from_routes`: only `set_channel_map` test call sites (pipeline.rs:554/575/690) and this file's tests |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Small, correct, panic-free channel routing. Validation is total
(zero-channel and out-of-range indices are typed errors at
construction; plane geometry is typed-checked at apply), the apply
loop is deterministic (source-major, route order), frame counts are
conserved, and output-plane capacity is reused (pinned by test). The
identity map runs in every shipped PCM session; every other map in
the vocabulary (swap, mono upmix, stereo downmix, explicit routes) has
no production caller — no command exposes `set_channel_map`, and the
device-adaptation conversion that would need `mono_to_stereo` is
admitted future work in the pipeline docs.

## 2. What actually works (verified)

— Hot path chain: `AudioPipeline::new` → `ChannelMap::identity(...)`
(pipeline.rs:117-119) → `AudioPipeline::process` →
`map.apply(&planes, &mut staged)` (pipeline.rs:300) — executed for
every audio frame the CLI stages (W00: PCM AVI decodes).
— Hand-verified `apply` (:141-170): in-plane count and equal-length
checks are exhaustive; `out` resized then per-plane `clear` + `resize`
(zero-fill); accumulation `*dst += *sample` in construction order —
no index can escape (all sources validated ≤ in_channels at
construction; `sources[to]` indexes a `sources` vec of len
out_channels).
— Zero channels rejected (:41-43), route indices rejected
(:100-105), wrong plane geometry rejected (:146-152) — all typed.

## 3. Findings

### F-001 [S3] The non-identity map vocabulary is production-dead; mono media plays mono forever
Where: channel_map.rs:54-111 (the constructors), audio-engine/src/pipeline.rs:257-267
(`set_channel_map`, only test callers), audio-engine/src/pipeline.rs:54-56
(the admission)

Code (audio-engine/src/pipeline.rs:54-56):
```
    /// The mixer channel count. v0 pins the device shape to it (the
    /// device-adaptation channel conversion joins with real backends,
    /// ADR-005 era).
```
What is wrong: a mono file opens a 1-channel pipeline on a 1-channel
null-device shape; `mono_to_stereo` (and every other non-identity map)
can never run — `set_channel_map` has zero production callers (Grep:
test sites only), no CLI/GUI command exposes channel routing, and the
device-shape adaptation that would need the upmix is documented as
future work. The constructors are correct and tested; they are a
library surface waiting for a wiring that does not exist.
Why it matters: wave 8's "how does a user hear sound" and any
matrix row claiming channel conversion/downmix: in the shipped build
the only map that ever runs is the identity. (The doc comment here is
honest about it — this is a wiring gap, not a lie.)
What correct looks like: negotiate the device shape at open and route
through `mono_to_stereo`/`stereo_to_mono` when the shapes differ (the
doc's own ADR-005 note), or expose a `set_channel_map` command.
Confidence: HIGH — Verification: read + repo-wide Grep for
`set_channel_map|mono_to_stereo|swap_stereo|stereo_to_mono|from_routes`.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ChannelMap::identity` | 40-49 | REAL | — | hot path; zero rejected |
| `swap_stereo` | 54-60 | REAL | S3 | test-only callers (F-001) |
| `mono_to_stereo` | 63-69 | REAL | S3 | the device-adaptation map that never runs (F-001) |
| `stereo_to_mono` | 74-80 | REAL | S3 | test-only; L+R sum without −3 dB (documented as filter-chain work) |
| `from_routes` | 91-111 | REAL | S3 | validated explicit routes; no production caller |
| `in_channels` / `out_channels` | 114-121 | REAL | — | |
| `sources_of` | 125-130 | REAL | — | out-of-range → `&[]` (graceful) |
| `apply` | 141-170 | REAL | — | exhaustive geometry checks; deterministic accumulation; capacity reuse; frames conserved |

## 5. Test honesty (:173-270)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `identity_is_bit_exact` | YES | exact plane equality |
| `swap_exchanges_the_pair` | YES | exact |
| `mono_to_stereo_duplicates` | YES | exact |
| `stereo_to_mono_sums` | YES | exact sums incl. 1.0 + −1.0 = 0.0 |
| `routes_validate_indices_and_sum_duplicates` | YES | sums, silence for unreferenced outputs, all three typed error codes |
| `apply_rejects_wrong_plane_geometry` | YES | both mismatch shapes |
| `apply_reuses_output_capacity` | YES | pins the no-realloc contract (capacity sum equal across applies) |

All exact-value, no tautologies. Not executed by me (no cargo);
UNVERIFIED per-test (W00 workspace run green).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "unity gain — weighted mixing is the mixer's volume stage" | :5-6 | REAL | apply only accumulates |
| "An output channel with no sources is silence" | :9-10 | REAL | :99 empty Vecs, zero-filled |
| "frame counts never change" | :14-15 | REAL | planes resized to input `frames` |
| "no allocation after the first" | :134-135 | REAL | :153-157 clear+resize; capacity pinned by test |

## 7. Dead / duplicated / unreachable code

`swap_stereo`, `mono_to_stereo`, `stereo_to_mono`, `from_routes`,
`sources_of` (no non-test caller): production-dead (F-001).

## 8. Dependencies & flags

`media_types::EngineError`, `crate::error`. No feature gates; ships in
the CLI binary via audio-engine.

## 9. Open questions for the next agent

- Wave 8 (audio-output): when a real (cpal) backend lands, does the
  null-device shape-pinning (`PipelineSpec.channels` = media channels)
  become a device-open failure for mono media on stereo-only devices?
  The conversion seam exists here, unwired.
