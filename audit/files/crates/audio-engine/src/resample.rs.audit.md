# AUDIT: crates/audio-engine/src/resample.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 699 / 7 production items + 12 `#[test]` fns + 1 in-test reference impl |
| File verdict | REAL-UNPROVEN (math genuine and differentially pinned) — **dormant in every shipped binary** (the correction loop is never armed in production) |
| Reachable from shipped entry point? | Object YES, hot path NO — `AudioPipeline::new` (apps/cli/src/pipeline_host.rs:1600, apps/web/src/host.rs:1746) constructs `MicroResampler::new` (audio-engine/src/pipeline.rs:130), but production never arms the drift governor (`arm_drift_governor`/`with_drift_governor` callers: tests only), so `correction` stays `DriftCorrection::ZERO` and `AudioPipeline::mix` takes the `ppm == 0` bypass (pipeline.rs:348-350) — `MicroResampler::process` never runs outside tests. |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:1 |
| Commands run | rg for `MicroResampler`/`arm_drift_governor`/`with_drift_governor` consumers across crates+apps; sed quote verification; manual phase-arithmetic check of the prefix loop (documented below) |

## 1. One-paragraph verdict

This is a real rate-conversion kernel, and a careful one: a ±1000 ppm micro-resampler with an exact i128/i64 rational phase accumulator (no floats on the control path, no cumulative rounding drift), a shared-across-channels linear kernel, cross-quantum phase carry, mid-stream correction renormalization, a bounded output capacity (`in + in/1000 + 1`), and a serious test battery — long-stream exact-ratio counts, closed-form interpolation values, convex-hull bounds, special-value bit-exactness, and a differential bit-identical proof against the verbatim pre-refactor kernel. The hostile question "real rate conversion math?" answers YES for what it is. What it is *not*: a track-rate converter (48 kHz→44.1 kHz is explicitly a different, unimplemented card — honest module docs), and — the finding that matters — **none of this executes in any shipped binary**, because the drift governor that feeds it a non-zero correction is only armed by tests ("the production open path leaves the limb unarmed", pipeline_host.rs:635-637). The flagship "consumer #1 of DriftCorrection" (this file's own header, line 2-3) is a consumer with no producer in production.

## 2. What actually works (verified)

- The phase math, verified by hand: `step = 1e6/(1e6+ppm)` carried as `cursor_abs + frac_num/step_den`; the W5-19 restructured prefix loop runs at most 2 iterations at the ±1000 ppm bound (I re-derived both ppm extremes: den=999_000 → one advance can bump the cursor twice; den=1_001_000 → once; entry cursor is pinned to `base−1` by the exit invariant).
- The capacity bound `out ≤ in + in/1000 + 1` is structurally enforced by the loop-exit conditions and pinned by `out_never_exceeds_the_design_bound` over adversarial lengths (460-474).
- W00 baseline cargo-test.log shows the audio-engine suite (86 tests incl. resample) green in the workspace run — UNVERIFIED by my own execution (ADAPTATION).

## 3. Findings

### F-001 [S2] — The entire correction path is dormant in production: the governor is never armed by any shipped host, so `process` never runs and A/V drift is never corrected

Where: crates/audio-engine/src/resample.rs:191 (the hot path); the arming gap is crates/audio-engine/src/pipeline.rs:155-157 + apps/cli/src/pipeline_host.rs:635-641.

Code (apps/cli/src/pipeline_host.rs:634-637, verbatim):
```rust
    /// Arm the drift governor (the late-arming composition path —
    /// [`AudioLimb::observe_drift`]'s regime note): the production
    /// open path leaves the limb unarmed; a composition that paces
```

Code (crates/audio-engine/src/pipeline.rs:348-350, verbatim):
```rust
        let (planes, out_frames) = if self.drift.ppm() == 0 {
            (self.mixed.as_slice(), frames)
        } else {
```

What is wrong: repo-wide grep: `arm_drift_governor` / `with_drift_governor` are called by `apps/cli/tests/sync_scenarios.rs:93` and nothing else. Both production composition roots (`apps/cli/src/pipeline_host.rs:1600`, `apps/web/src/host.rs:1746`) build with `AudioPipeline::new` (unarmed). The unarmed governor leaves `correction = ZERO`, and `mix` then bypasses the resampler entirely — so the drift-correction loop that card 3.B1 was merged for (and that docs/gates/phase3-exit.md:31 counts toward the "sync … MET (merged)" family) has no actuation in any shipped binary. The reason given is honest — the CLI's decode-ahead pump regime would mistake buffering depth for drift — but the paced production pump it depends on ("once 3.B3's backpressure lands steady-state pacing") never landed.

Why it matters: A/V sync correction is claimed as a completed capability family while its only actuator is unreachable code in every shipped build; the moment a real audio device exists, drift will go uncorrected.

What correct looks like: the wall-bounded pacer (which the tauri shell already runs, apps/desktop-tauri/src/app.rs:42-48) arms the governor; or the gate says "observation real, correction pending pacing regime".

Confidence: HIGH — Verification: read (grep over crates/ + apps/; both composition roots read).

### F-002 [S3] — The module's "no panics / geometry mismatch produces no output" claim is false for ragged planes: a short plane panics in the body loop

Where: crates/audio-engine/src/resample.rs:42-45 (the claim) vs 192-194 (the actual guard) vs 255 (the panic site).

Code (42-45, verbatim):
```rust
//! - No `unsafe`, no floats on the control path, no panics: a
//!   zero-frame quantum is a no-op; a geometry mismatch produces no
//!   output (the pipeline is the only caller and guarantees the
//!   shape — the guard is defensive, not reachable).
```

Code (253-255, verbatim):
```rust
            let idx = (cursor - base_abs) as usize;
            for (plane, in_plane) in self.out.iter_mut().zip(input.iter()) {
                let window = &in_plane[idx..idx + 2];
```

What is wrong: the only geometry guard is `input.len() != self.channels` (192). If a caller passes the right channel count but a plane shorter than `in_frames`, `in_plane[idx..idx + 2]` with `idx+2 ≤ in_frames` panics (slice out of range) — no typed error, no empty output. The doc's "no panics" is contradicted by the code it describes. The sole production caller stages planes to equal length (pipeline alignment check), so this is a latent API-contract lie, not a live crash — but the module was written to brag about panic-freedom, and the claim is checkably false.

Why it matters: §3.1/§3.2 — a documented invariant that the code does not enforce; any future second caller inherits a hidden panic.

What correct looks like: check `input.iter().all(|p| p.len() >= in_frames)` next to the channel-count guard, returning 0 (as documented) on mismatch.

Confidence: HIGH — Verification: read (index arithmetic: `idx ≤ in_frames − 2` inside the body loop ⇒ `idx+2 ≤ in_frames`).

### F-003 [S4] — The differential "reference" is the old kernel kept verbatim in the test module: a regression pin, not an independent oracle

Where: crates/audio-engine/src/resample.rs:495-592 + 594-657.

Code (495-497, verbatim):
```rust
    /// The former single-loop kernel, verbatim (the pre-W5-19
    /// `process` body) — the differential reference for the
    /// tail-prefix / body / exact-tail restructure above.
```

What is wrong: nothing about the test itself — it is a legitimate bit-exactness pin for a performance restructure, and `linear_interpolation_values_are_exact` (374-395) independently validates against a closed form, which covers the shared-conceptual-bug risk. But a reader must not mistake the differential test for proof of correctness of the *algorithm* (the reference encodes the same design decisions); the closed-form and exact-ratio tests are the actual oracles. The test names alone ("bit_identical_to_the_former_single_loop") could be cited as evidence of more than they prove. Hygiene note for the claim ledger.

Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-54 | REAL-UNPROVEN | S3 | F-002 false panic-freedom claim; honest about not being a track-rate converter |
| `PPM_SCALE` const | 60 | REAL-UNPROVEN | — | 1e6 |
| `MicroResampler` (struct) | 70-94 | REAL-UNPROVEN | — | O(channels) state; i128 cursor (~7 y at 44.1 kHz headroom) |
| `new` | 100-111 | REAL-UNPROVEN | — | dormant ZERO correction |
| `correction` | 115-117 | REAL-UNPROVEN | — | accessor |
| `set_correction` | 124-134 | REAL-UNPROVEN | — | renormalizes carry on ratio change; idempotent on same ppm; hand-verified both extremes |
| `reset` | 140-145 | REAL-UNPROVEN | — | fresh phase; correction kept (caller's call) |
| `planes` | 152-154 | REAL-UNPROVEN | — | borrows last output |
| `process` | 191-296 | REAL-UNPROVEN / dormant | S2/S3 | F-001 dormant; F-002 ragged-plane panic; prefix ≤ 2 iters; exact-tail `−0.0` flush documented as pinned bits |
| tests (12) | 308-697 | REAL-UNPROVEN | S4 | §5 below |

## 5. Test honesty (12 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `zero_correction_emits_the_input_count_exactly` (309-319) | YES | exact count + exact values |
| `ratio_is_exact_over_long_streams` (322-344) | YES | 1000 quanta vs closed-form ±1, both ppm signs |
| `phase_carries_exactly_across_quanta` (347-371) | YES | 4096 one-sample quanta; monotone + hull bounds |
| `linear_interpolation_values_are_exact` (374-395) | YES | closed form in f64 at 1e-5 — independent oracle |
| `channel_coherence_by_construction` (398-414) | YES | per-channel scaling relations |
| `correction_change_midstream_renormalizes_the_phase` (417-432) | WEAK-MED | asserts only count ranges (2..=5), not values |
| `reset_starts_a_fresh_phase` (435-446) | YES | first output == first sample |
| `zero_frame_quantum_is_a_no_op` (449-457) | YES | 0 then normal |
| `out_never_exceeds_the_design_bound` (460-474) | YES | capacity contract over 7 lengths |
| `the_stream_never_fabricates_samples_past_the_kernel_window` (477-491) | YES | convex hull over 100 quanta |
| `restructured_kernel_is_bit_identical…` (595-657) | YES (regression) | F-003: reference is the old kernel, not an oracle |
| `special_values_survive_the_exact_tail_identically` (660-697) | YES | NaN/±inf/±0 bit patterns both paths |

No test exercises the resampler through the *production* pipeline with a non-zero correction (that exists in apps/cli/tests/sync_scenarios.rs — test-only arming), which is F-001's point.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "consumer #1 of DriftCorrection — the piece audit M12 named absent" | resample.rs:1-3 | TRUE-but-idle | the consumer exists; no production producer arms it (F-001) |
| "sync … **MET (merged)**" (3.B1 counted) | docs/gates/phase3-exit.md:31 | PARTIAL/FALSE for the correction half | observation runs (scheduler SyncSample, my scheduler.rs audit); actuation dormant (F-001) |
| "out_len ≤ in_len + in_len/1000 + 1 for any \|ppm\| ≤ 1000" | resample.rs:40-41 | REAL-UNPROVEN | structurally enforced + tested; not executed by me |
| "No … panics: … a geometry mismatch produces no output" | resample.rs:42-45 | FALSE | F-002: ragged planes panic |
| "linear is inaudible at ≤1000 ppm [PROPOSAL]" | resample.rs:15-16 | UNVERIFIED (perceptual claim) | no listening test exists or could exist headless |

## 7. Dead / duplicated / unreachable code

`MicroResampler::process` in production: constructed but never invoked with a non-zero correction (F-001). `set_correction`'s renormalization path likewise test-only. No duplication found.

## 8. Dependencies & flags

Depends on `media_clock::DriftCorrection` (the ±1000 ppm limiter lives there — drift.rs is another agent's file). No feature gates; `audio-engine` is an unconditional workspace member; the pipeline wiring is compiled into the CLI binary.

## 9. Open questions for the next agent

- `DriftGovernor` (crates/audio-engine/src/governor.rs) and `DriftCorrection` (media-clock drift.rs) — their correctness bounds the resampler's inputs; both are other agents' files.
- If the tauri shell's wall-bounded pacer is the "paced production" the arming note waits for, why doesn't IT arm the governor? (W14 apps/desktop-tauri.)
- `tests/zero_alloc_resample.rs` + `zero_alloc_quantum.rs` — allocation proofs, test-wave territory.
