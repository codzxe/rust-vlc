# AUDIT: crates/playback-engine/tests/track_switch_alloc.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 593 / 1 `#[test]` (whole-binary, counting allocator) + fakes (SharedNow, TallyConsumer, MarkerDemuxer, ContextDecoder, ContextProvider) + harness |
| File verdict | REAL — a sharp, well-instrumented allocation proof; the cleanest file in my set |
| Reachable from shipped entry point? | N/A (test binary). Subject: the engine's per-session decoder cache — the same engine API as track_switch.rs (see its F-004: zero shipped callers of `set_audio_track`). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read (full file); Grep `#[test]` = 1, no `#[ignore]`/cfg/env gates; cross-check of the engine stats fields (`decoders_built`/`decoders_reused` — same fields asserted in track_switch.rs case 9); sed verification of quoted ranges. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The counting-allocator proof for card 3.A3's decoder cache, and the
detector design is genuinely sharp: the fake decoders allocate ONE
256 KiB block at construction and never again, so a soak of 41
alternating audio switches has exactly two shapes — cache works (3
builds total, ~6 KiB per-switch churn) or cache broken (a 256 KiB
context re-allocated every switch). Both oracles are asserted
independently and they agree with a THIRD oracle (the engine's own
`decoders_reused == 40`), and the churn bound is pinned 8× below one
context so a broken cache cannot hide. The discipline is right:
whole-binary serialization, one reused output buffer, pre-sized
snapshot vector, warm-up window excluded, bounds documented with
their measured values and a "re-measure, don't silently weaken"
rule, per-switch frame-delivery proof (the soak cannot pass
vacuously), and an instrument-liveness tripwire (allocator must have
seen calls). Only hygiene findings: another copy of the fake family
(SharedNow/provider duplicated from track_switch.rs), and the
demuxer's `reanchor` is dead surface here.

## 2. What actually works (verified)

— Ungated single `#[test]` with its own `#[global_allocator]` → ran in W00's `cargo test --workspace` (1751 pass).
— Triple oracle for the cache: (a) provider build counts exactly 1 per track (:506-520); (b) engine `decoders_built == 3` / `decoders_reused == 40` (:522-527); (c) allocator churn bytes ≤ 32 KiB per switch against a 256 KiB context (:565-575) — a teardown-and-rebuild cache fails all three, the churn bound by 8×.
— Leak detection proper: creep over the 36-switch measured window ≤ 1 KiB (pinned from a measured 0; :542-550) and band ≤ 16 KiB (:533-541) — a one-slot-per-switch retention fails the creep bound.
— Liveness, three layers: per-switch `frames_after > frames_before` (:493-498, the delivery-is-the-completion-fact proof), terminal within 20 polls or `unreachable!` (:452), `Playing` held across every switch (:499), and the allocator-saw-work tripwire (:588-592) so a broken instrument cannot read as a pass.
— Honest bounds discipline: every constant carries its measured value and margin in a comment (:100-120), and the header demands re-pinning over silent weakening (:54-58).
— Mutation reasoning (UNVERIFIED, no cargo): make the cache evict on retire → build counts and `decoders_reused` fail AND churn bytes blow 32 KiB; retain one queue slot per switch → creep fails; per-switch Vec growth in bookkeeping → band/churn-calls fail; switch that never completes → `unreachable!` at :452.

## 3. Findings

### F-001 [S4] The fake family is copy-pasted from track_switch.rs (third site of the shared-fakes pattern)

Where: crates/playback-engine/tests/track_switch_alloc.rs:127-133, 249-289
Code:
```rust
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}
```
What is wrong: `SharedNow` is character-identical to track_switch.rs:81-87; `ContextProvider` is track_switch.rs's `CountingProvider` minus the shapes map; `marker_packet`/`track`/`base`/`at_millis` are the same builders; `soak_queues`/`soak_budget` are re-declared copies of track_switch.rs's `tiny_queues`/`tiny_budget` (:333-353 says so itself). The header points at `replacement_alloc.rs` as the family origin — the same handful of fakes now lives in at least three integration binaries. A fix to the fake's semantics (e.g. the poison-recovery idiom or the budget math) must be applied N times; the `tiny_budget` comment in track_switch.rs (:414-424) explains subtle salvage reasoning that this copy silently relies on.
Why it matters: §3.5 duplication; drift between the copies makes cross-suite comparisons (the alloc suites exist to be compared) unreliable.
What correct looks like: a `tests/common/` module (the video-engine suite's pattern) or testkit-hosted fakes.
Confidence: HIGH — Verification: read of both files (side-by-side).

### F-002 [S4] `MarkerDemuxer::reanchor` is dead surface in this test — and silently rewinds to position 0

Where: crates/playback-engine/tests/track_switch_alloc.rs:184-188
Code:
```rust
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.issued = 0;
        self.remaining = PACKETS_PER_TRACK * self.tracks.len();
        Ok(target)
    }
```
What is wrong: track switches do not reanchor (track_switch.rs case 10 pins "the demuxer never moved" as the expected behavior), and this test issues no seeks — so `reanchor` is never called. If the engine's switch flow ever began calling reanchor, this fake would silently rewind the stream to position 0 on every switch: the packet budget would never exhaust (making the "no EOS inside the soak window" design at :96-98 accidentally true for the wrong reason) and the fill-from-current-position property would be untested here with no signal.
Why it matters: a defensive fake whose behavior would mask a semantic change in the engine's switch flow; dead surface plus a trap.
What correct looks like: make reanchor a loud refusal (the fake's `seek` already refuses with "not-the-reanchor-path" at :172-178) or assert it is never called during a switch (a counter checked at the end).
Confidence: MEDIUM — Verification: read (no seek ops in the test; the switch op set is `set_audio_track` only; reanchor's call sites are engine-internal, and track_switch.rs's case-10 oracle documents switches as non-reanchoring — the residual uncertainty is whether some engine path calls reanchor on limb rewire, which I could not fully exclude statically).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ALLOC` global | 60-62 | REAL | — | the process-global instrument |
| `SharedNow` | 127-133 | REAL | S4 | F-001 duplicate |
| `TallyConsumer` | 137-146 | REAL | — | non-retaining sink — retention would be the test's own growth (honest design) |
| `MarkerDemuxer` | 150-189 | REAL | S4 | round-robin synthetic packets, zero per-packet alloc; `reanchor` dead + rewinding (F-002) |
| `ContextDecoder` | 196-247 | REAL | — | the detector: one 256 KiB block at construction, zero after; `#[allow(dead_code)] context` is the point, not a smell |
| `ContextProvider` | 251-289 | REAL | S4 | F-001 duplicate of CountingProvider |
| builders (`base`/`at_millis`/`track`/`marker_packet`) | 295-331 | REAL | S4 | F-001 duplicates; static payloads (Arc bump, no alloc) verified |
| `soak_queues`/`soak_budget` | 335-353 | REAL | S4 | copies of track_switch.rs's tiny_queues/tiny_budget |
| `Harness::new` | 362-409 | REAL | — | same frozen throttle clock pattern as track_switch.rs (its F-005 applies here too — tick throttling never exercised) |
| `pump_to_playing` | 411-423 | REAL | — | bounded, loud |
| `settle_switch` | 428-453 | REAL | — | bounded 20 polls; reused out-buffer (D-1) |
| test `decoder_contexts_survive_switches_without_reallocation` | 464-593 | REAL | — | the triple-oracle proof; window arithmetic asserted (:531) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| decoder_contexts_survive_switches_without_reallocation | YES | cache eviction fails build counts + `decoders_reused` + the 8×-margin churn bound; retention fails creep; vacuous run fails per-switch delivery proof + instrument tripwire |

Ungated → ran in the W00 baseline.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the per-session cache parks decoder instances, so the multi-MB decode context … survives every switch" | header :1-7 | REAL (engine-level) | the triple oracle; the motivating REAL codec case (MJPEG DecodeContext) is asserted only via the 256 KiB stand-in — the real-codec version is the alloc suite's `replacement_alloc.rs`/engine suites' territory |
| "Both oracles are asserted … a broken cache cannot hide under it" | :35-37 | REAL | build counts + churn bytes with 8× margin |
| "The constants are pinned from measurement (D-6)" | :54-58 | REAL-UNPROVEN | the measured values are recorded in comments; I could not re-run to confirm the measurements (no cargo) — but the bounds themselves are asserted every run |

## 7. Dead / duplicated / unreachable code

- F-001 (fake family duplication), F-002 (`reanchor` dead surface).
- The tested API (`set_audio_track`) has zero shipped callers — cross-reference track_switch.rs F-004 (not re-derived here).

## 8. Dependencies & flags

No feature flags, no env knobs; runs in the default merge-gate test set (W00 baseline). Same dev-dep set as track_switch.rs plus `testkit::alloc_count`.

## 9. Open questions for the next agent

- Whether any engine path calls `Demuxer::reanchor` during a track switch (F-002's residual uncertainty) — a definitive answer lives in pipeline.rs/session.rs rewire logic (already audited by W03-A01/W03 wave; worth a one-line cross-check).
