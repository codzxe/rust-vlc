# AUDIT: crates/audio-engine/src/channel_map.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 270 / 9 (+7 tests) |
| File verdict | REAL (identity path); the rest of the routing vocabulary is production-unreachable |
| Reachable from shipped entry point? | YES for `identity`+`apply`: apps/cli/src/pipeline_host.rs:660 `self.pipeline.process(0, frame)?` → pipeline.rs:291 `ChannelMap::apply` (identity maps built at pipeline.rs:118); same for apps/web/src/host.rs:230. NO for `swap_stereo`/`mono_to_stereo`/`stereo_to_mono`/`from_routes`/`set_channel_map` — zero non-test callers (rg census). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "ChannelMap\|set_channel_map\|channel_map" apps crates` (exit 0, caller census); `rg -n "channel_map::tests" audit/runs/baseline/cargo-test-workspace.txt` (exit 0, 7/7 ok lines 25-31); sed quote verification |

## 1. One-paragraph verdict
First, the hostile-focus hypothesis is a misfire for this file: channel_map.rs does NOT do VLC-style↔symphonia channel-order conversion — that job lives in the codecs crate (opus/order.rs Vorbis→WAV permutation, audited above). This file is the mixer's unity-gain routing table, and it is correct at what it does: validated constructors, typed rejections for zero-shape and out-of-range routes, an `apply` that checks plane geometry before touching data, deterministic source-major summation, and capacity reuse across quanta (pinned by a test). The identity map runs in every shipped session. The defect is scope, not correctness: the entire *non-identity* vocabulary — the stereo swap, both mono folds, explicit routes, and the `set_channel_map` entry itself — has no caller in any shipped composition; every production session routes identity, so the "channel-shape negotiation" the module doc advertises never negotiates anything in the shipped product.

## 2. What actually works (verified)
— verified by `rg -n "channel_map::tests" audit/runs/baseline/cargo-test-workspace.txt` -> 7/7 ok (identity bit-exactness, swap, both folds, route validation + duplicate summing, plane-geometry rejection, capacity reuse).
— verified by reading the apply path (channel_map.rs:146-175): plane-count and common-length checks precede any indexing; `out` is truncated by `resize_with` when a previous map was wider; accumulation is `+=` over zip — no panic path on the guarded geometry.

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/channel_map.rs:54-107 (vocabulary), crates/audio-engine/src/pipeline.rs:257 (the only setter)
Code (channel_map.rs:54-61, verbatim):
```
    pub fn swap_stereo() -> ChannelMap {
        ChannelMap {
            in_channels: 2,
            out_channels: 2,
            sources: vec![vec![1], vec![0]],
        }
    }
```
What is wrong: not the code — the wiring. `set_channel_map` (pipeline.rs:257) is called only from pipeline.rs's own `#[cfg(test)]` tests (lines 554, 575, 690); no shipped composition (CLI pipeline_host, web host) ever installs a non-identity map. Shipped sessions construct `ChannelMap::identity(spec.channels)` (pipeline.rs:118) and never change it, so `swap_stereo`/`mono_to_stereo`/`stereo_to_mono`/`from_routes` are dead surface in the product — the "mixer's channel-shape negotiation" (module doc, channel_map.rs:3) never happens. A user cannot downmix a 5.1 file to stereo through any shipped path. What correct looks like: either a composition policy (config-driven per docs/07 §2 "A/V sync drift correction"-style mechanism/policy split) installs maps on device-shape mismatch, or the vocabulary is documented as library-surface-not-yet-product-surface. Confidence: HIGH — Verification: read + rg caller census (ran).
F-002 [S4]
Where: crates/audio-engine/src/channel_map.rs:161-164
Code:
```
        for plane in out.iter_mut() {
            plane.clear();
            plane.resize(frames, 0.0);
        }
```
What is wrong: every output plane is zero-filled then (for the shipped identity case) fully overwritten by the single-source copy — one redundant memset of the whole quantum per track per quantum. Correct for maps with silent outputs, wasted work for fully-sourced ones (the only maps production uses). What correct looks like: zero-fill only planes with empty `sources`; initialize sourced planes by extending from the first source. Confidence: HIGH — Verification: read; perf impact UNVERIFIED (cargo-build-budget; the extra memset is O(frames) per plane per quantum at 30 Hz).
F-003 [S4]
Where: crates/audio-engine/src/channel_map.rs:133-138
Code:
```
    pub fn sources_of(&self, to: u32) -> &[u32] {
        match self.sources.get(to as usize) {
            Some(list) => list,
            None => &[],
        }
    }
```
What is wrong: out-of-range `to` silently answers "no sources" instead of a typed error — an index bug at a call site would read as "silence by design" rather than fail loudly. Undocumented behavior. What correct looks like: `Err` (this crate's style everywhere else) or at least a doc line. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `identity` | 40-52 | REAL | — | zero-channel typed refusal; production path |
| `swap_stereo` | 54-61 | REAL-UNPROVEN | S3 (F-001) | correct; test-verified; no production caller |
| `mono_to_stereo` | 63-71 | REAL-UNPROVEN | S3 (F-001) | duplicate; test-verified; no production caller |
| `stereo_to_mono` | 73-80 | REAL-UNPROVEN | S3 (F-001) | L+R sum, −3 dB pan-law explicitly deferred (documented); no production caller |
| `from_routes` | 91-108 | REAL-UNPROVEN | S3 (F-001) | index validation typed; duplicate targets sum; no production caller |
| `in_channels`/`out_channels` | 111-122 | REAL | — | |
| `sources_of` | 133-138 | REAL | S4 (F-003) | silent OOR |
| `apply` | 146-175 | REAL | S4 (F-002) | geometry checks, deterministic accumulation, capacity reuse |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| identity_is_bit_exact | YES | exact vector equality incl. negatives/fractions |
| swap_exchanges_the_pair | YES | distinct per-channel values |
| mono_to_stereo_duplicates / stereo_to_mono_sums | YES | exact sums |
| routes_validate_indices_and_sum_duplicates | YES | sums, silent-output, 3 typed error codes |
| apply_rejects_wrong_plane_geometry | YES | both mismatch shapes |
| apply_reuses_output_capacity | YES | capacity sum equality across two applies (would catch the C6a false-reuse class) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the mixer's channel-shape negotiation" (module purpose) | channel_map.rs:3 | SHALLOW in product | negotiation vocabulary has no shipped caller (F-001) |
| "frame counts never change — only channel shapes do (the sample-count conservation invariant)" | channel_map.rs:10-11 | REAL | apply outputs `frames`-sized planes only; tests |
| Hostile focus: "VLC-style channel order vs symphonia order conversions" | task brief | NOT THIS FILE | order conversion lives in codecs/src/audio/opus/order.rs (audited; S2 bug in its 4-row) — this file is gain routing, order-agnostic |

## 7. Dead / duplicated / unreachable code
The non-identity vocabulary (F-001) — library surface, product-dead. No duplicated blocks. No unsafe. No feature gates.

## 8. Dependencies & flags
`media_types::EngineError`, `crate::error` helpers. Zero external deps, zero alloc beyond the plane buffers `apply` is contracted to reuse.

## 9. Open questions for the next agent
- pipeline.rs owner (already audited by W0x): confirm whether any planned composition (desktop real-device session) installs non-identity maps — until then F-001 stands as product-surface debt.
- If 5.1→stereo downmix ever ships, note stereo_to_mono's missing −3 dB compensation is per-route-summed: two full-scale channels sum to +3 dB overflow territory (f32 domain, so no wrap — but the mixer's volume stage must account for it).
