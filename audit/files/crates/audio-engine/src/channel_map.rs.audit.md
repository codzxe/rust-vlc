# AUDIT: crates/audio-engine/src/channel_map.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 270 / 19 (10 prod incl. struct + 4 constructors + 3 accessors + `apply`; 1 test helper + 8 tests) |
| File verdict | REAL (identity map is shipped-live; the routing constructors are test-only — see F-001) |
| Reachable from shipped entry point? | YES for `identity`/`apply`: `lumen open` → `apps/cli/src/pipeline_host.rs:1600` `AudioPipeline::new(PipelineSpec{..})` → `crates/audio-engine/src/pipeline.rs:118` `ChannelMap::identity(spec.channels)` → `pipeline.rs:300` `map.apply(&planes, &mut staged)` per quantum. Executed by baseline: AVI MJPEG+PCM 60/60 audio frames (INDEX.md contrast lane), `sample-aac.mp4` Playing. NO for `swap_stereo`/`mono_to_stereo`/`stereo_to_mono`/`from_routes`/`set_channel_map`: only callers are pipeline.rs's `#[cfg(test)]` module (lines 554, 575, 690) and this file's tests — the shipped pipeline never replaces the identity map. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "ChannelMap::\|set_channel_map\|from_routes" crates apps` → caller census above; `sed -n '100,130p' crates/audio-engine/src/pipeline.rs` (identity construction, shape validation); `rg -n "channels" crates/demux/src/wav/format.rs` → MAX_CHANNELS = 64 bound (format.rs:41); `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
This module is what it claims: a tiny, correct, allocation-stable channel router with validated construction, typed geometry errors, and honest unit semantics (unity gain, sum-on-duplicate, silence-on-unreferenced). `apply` preserves the frame-count invariant, reuses output capacity, and cannot panic (all indexing behind construction-time validation; `sources_of` uses `get`). The two defects are structural, not computational: the entire "v0 vocabulary" of non-identity routing (swap/upmix/downmix/explicit routes) is public API that no shipped code path ever installs — every shipped frame traverses the identity map, so the module's advertised capability surface is dead weight exercised only by tests; and the constructors accept any `u32` channel count with allocation proportional to it, bounded in practice only by far-away demuxer caps (WAV `MAX_CHANNELS = 64`), not by this API.

## 2. What actually works (verified)
— Identity routing through the shipped engine: baseline INDEX.md — AVI first-vertical `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` (audio passes through `AudioPipeline` → identity map → null ring) and `sample-aac.mp4` Playing; `pipeline.rs:118,300` are the two live call sites.
— Sum/duplicate/silence semantics + validation + capacity-reuse contract: verified by reading the 8 tests (182-269) against the code — every behavior the doc claims (lines 4-15) has an exact assertion (`stereo_to_mono_sums` 0.25+0.5=0.75; unreferenced output → 0.0; capacity sum unchanged across applies at 265-267).
— Panic-freedom of `apply`: plane-count and common-length pre-checked (146-152); accumulation via `zip` of two equal-length planes (164-166); construction-time route-index validation (101-103). No unwrap/index/slice panics found in any reachable path.

## 3. Findings

### F-001 [S3] The non-identity routing vocabulary is dead public API — shipped audio is identity-only
Where: crates/audio-engine/src/channel_map.rs:54-111 (constructors) and crates/audio-engine/src/pipeline.rs:257
Code:
```rust
    /// The stereo swap: 2 in, 2 out, left and right exchanged. A
    /// mis-wired pair is audible in one operation (the corpus's
    /// distinct 440 Hz/660 Hz tone plan exists exactly to catch it).
    pub fn swap_stereo() -> ChannelMap {
```
What is wrong: `swap_stereo`, `mono_to_stereo`, `stereo_to_mono`, `from_routes`, and the pipeline's `set_channel_map` have zero shipped callers (caller census: pipeline.rs test module only). The module doc presents the stereo-pair routing set as "the mixer's channel-shape negotiation" (card 2.C3, doc 01 §2.5) — in the shipped build there is no negotiation: `AudioPipeline::new` installs `identity` and nothing ever replaces it, so a mono AAC/MP3 track plays mono and a stereo track stereo, with no upmix/downmix anywhere in the output chain.
Why it matters: dead capability surface that the docs advertise as live ("stereo pair routing and friends" is the module's first line); a user or matrix row crediting channel routing in the shipped player is crediting test-only code.
What correct looks like: either wire a policy (e.g. mono→stereo when the device opens stereo) or mark the constructors `#[cfg(any(test, feature = "…"))]` and fix the doc to say identity-only in v0.
Confidence: HIGH — Verification: read + full caller census (rg across crates/ and apps/).

### F-002 [S4] No upper bound on channel counts in the public constructors
Where: crates/audio-engine/src/channel_map.rs:40-49 and 91-111
Code:
```rust
        Ok(ChannelMap {
            in_channels: channels,
            out_channels: channels,
            sources: (0..channels).map(|c| vec![c]).collect(),
        })
```
What is wrong: `identity(u32::MAX)` / `from_routes(u32::MAX, …)` allocate `Vec` storage proportional to the raw count (a 4-billion-entry `Vec<Vec<u32>>` is an instant OOM abort). Nothing in this API or in `AudioPipeline::new` (pipeline.rs:114 checks only `channels == 0`) caps the value; the only real-world bound lives two layers away in the WAV demuxer (`MAX_CHANNELS: u16 = 64`, crates/demux/src/wav/format.rs:41) and in decoder-side gates (AAC ≤ 2).
Why it matters: the "unbounded allocation from file-controlled counts" checklist class; today unreachable because every shipped producer of `spec.channels` is itself bounded, but the bound is accidental, not enforced where the allocation happens.
What correct looks like: a `MAX_CHANNELS`-style ceiling (the demuxer's 64 is the precedent) in `identity`/`from_routes`/`AudioPipeline::new` with a typed `audio.channel_count_excessive`.
Confidence: HIGH — Verification: read; WAV bound confirmed at format.rs:41; no other shipped producer exceeds decoder gates.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct ChannelMap` | 26-31 | REAL | — | Private fields; constructed-validated invariant holds (all ctors validate). |
| `identity` | 40-49 | REAL (shipped-live) | S4 (F-002) | Zero-channel typed refusal; `vec![c]` per channel. |
| `swap_stereo` | 54-60 | REAL, test-only | S3 (F-001) | Correct table `[[1],[0]]`. |
| `mono_to_stereo` | 63-69 | REAL, test-only | S3 (F-001) | Duplicate semantics. |
| `stereo_to_mono` | 74-80 | REAL, test-only | S3 (F-001) | L+R sum; −3 dB pan-law honestly deferred in doc (71-74). |
| `from_routes` | 91-111 | REAL, test-only | S3 (F-001), S4 (F-002) | Index validation before any push (101-103); duplicate routes sum by construction. |
| `in_channels` | 113-116 | REAL | — | Trivial accessor. |
| `out_channels` | 118-121 | REAL | — | Trivial accessor. |
| `sources_of` | 125-130 | REAL | — | `get`-based; out-of-range → `&[]` (silent but read-only and documented as empty-set semantics). |
| `apply` | 141-170 | REAL (shipped-live) | — | Geometry checks (146-152); zero-fill then source-major accumulate (deterministic order); `zip` on equal lengths; capacity reuse via clear+resize (154-157). No panic path. |
| `planes` (test helper) | 178-180 | REAL | — | — |
| 8 tests | 182-269 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `identity_is_bit_exact` (183) | YES | Exact `assert_eq!(out, input)` on non-trivial floats. |
| `swap_exchanges_the_pair` (194) | YES | Distinct values per channel. |
| `mono_to_stereo_duplicates` (203) | YES | Both output planes asserted. |
| `stereo_to_mono_sums` (212) | YES | 0.25+0.5 / 1.0−1.0 exact. |
| `routes_validate_indices_and_sum_duplicates` (221) | YES | Sum 1+4=5, unreferenced→0.0, and all three typed refusal codes asserted (236-241). |
| `apply_rejects_wrong_plane_geometry` (245) | YES | Both mismatch shapes → `audio.planar_plane_mismatch`. |
| `apply_reuses_output_capacity` (258) | YES | Capacity-sum equality across two applies — mutating resize-into-fresh-Vec (e.g. `*out = Vec::new()`) fails it. |
Mutation check (reasoned): flipping `sources` order in `swap_stereo` fails 194; changing `+=` to `=` fails 221 (1+4 ≠ 4); dropping the length check at 150 fails 245. The suite has full teeth for everything it covers — the gap is that nothing covers the constructors' absence from the shipped pipeline (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Channel routing: stereo pair routing and friends (card 2.C3 … the mixer's channel-shape negotiation)" | module doc 1-2 | SHALLOW | The machinery exists and is correct, but no shipped path negotiates anything — identity only (F-001; caller census). |
| "frame counts never change — only channel shapes do (the sample-count conservation invariant)" | module doc 13-15 | REAL | `apply` writes `frames`-long planes regardless of shape (153-157); verified in code and tests. |
| "its capacity is reused across calls (no allocation after the first, doc 03 §4.8 c)" | `apply` doc 132-135 | REAL | Test 258-269 asserts capacity stability. |

## 7. Dead / duplicated / unreachable code
- `swap_stereo`/`mono_to_stereo`/`stereo_to_mono`/`from_routes`/`set_channel_map`: dead in shipped builds (F-001).
- No duplication found; no unsafe.

## 8. Dependencies & flags
- No Cargo features gate this module (`audio-engine` has no codec features); no external deps beyond `media_types` + `testkit` (dev). Compiled into every shipped build including web.

## 9. Open questions for the next agent
- Pipeline owner (W07-A01/A02, pipeline.rs): is the missing channel-map policy (mono upmix at the device) a recorded card/PROPOSAL, or genuinely forgotten? The null device makes it inaudible today; a real backend makes it user-visible.
- If a real audio backend lands: F-001's dead constructors become the upmix/downmix seam — re-audit then.
