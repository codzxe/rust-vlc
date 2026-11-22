# AUDIT: crates/audio-engine/tests/zero_alloc_quantum.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 105 / 1 test + 1 helper |
| File verdict | REAL — a measured zero-allocation proof (counting global allocator), not an argued one |
| Reachable from shipped entry point? | N/A (test binary; runs in `cargo test -p audio-engine` — W00: workspace suite green) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep only (no cargo — not executed by me; W00 ran the suite) |

## 1. One-paragraph verdict

This is what a performance claim should look like: a
`CountingGlobalAlloc` global allocator with a guard window
(:75-84) measures that the whole per-quantum pump path
(`process → mix → take_output → drop`) allocates zero bytes over 512
quanta after one warm-up quantum, and again zero after a geometry
change (:98-104). The module doc states the measurement discipline
(one test per binary, serialized, because the allocator is
process-global). The claim it proves is the one the audio pipeline's
docs make (D-1). One gap: only the happy path is measured — the
pipeline's error paths are known (my audio-pipeline audit F-003) to
drop and re-warm buffers, so a corrupt-stream session is not
zero-alloc; the test does not claim that, but nothing else
characterizes it either.

## 2. What actually works (verified)

— The test constructs the real `AudioPipeline` (not a mock), feeds
real 1470-frame s16 quanta with per-quantum-varying content
(:32-51 — a stale-slot leak would surface elsewhere in value ACs,
per the comment), and measures with the real allocator.
— `serialize_tests()` (:55) + one-test-per-binary avoids the
parallel-runner false positive the module doc warns about.
— Would it fail if the code were broken? YES: any reintroduced
per-quantum allocation (e.g. a `to_vec`, a fresh slot rotation under
full retention, a `Vec::with_capacity` in mix) trips
`guard.assert_zero`. Mutation check (reasoned, not run — UNVERIFIED):
reverting the pooled-slot optimization in `AudioPipeline::mix` to the
old `mem::take` + fresh `Arc` per quantum (the C6b violation the
pipeline docs describe) would allocate once per quantum → 512
allocations → assert fails.

## 3. Findings

### F-001 [S4] The zero-alloc proof covers only the happy path; the error path's re-warm churn is unmeasured
Where: zero_alloc_quantum.rs:74-84 vs audio-engine/src/pipeline.rs:297-303
(the error paths that drop the reused buffers — see the audio-pipeline
audit F-003)

Code (:75-78):
```
    let guard = ALLOC.state().guard();
    for frame in &frames {
        pipeline.process(0, frame).must();
```
What is wrong: every `process` in the measured window succeeds. A
partial-frame payload (typed `audio.partial_frame`) drops the scratch
planes (pipeline.rs:297-299 `?` before the restore), and the next
quantum re-allocates — the zero-alloc contract silently fails on
damaged streams. The test does not lie (it never claims error-path
coverage), but the D-1 claim it evidences is narrower than a reader
would assume.
What correct looks like: one more window measuring
N × (partial-frame error + recovery) and asserting a bounded, known
allocation count (or fixing the error path to restore buffers).
Confidence: HIGH — Verification: read (error path verified in the
audio-pipeline audit).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `quantum_frame` | 32-51 | REAL | — | real s16 payload, per-quantum varying; exact pts arithmetic |
| `audio_quantum_path_is_zero_allocation_after_warm_up` | 53-105 | REAL | S4 | measured proof, happy path only (F-001); geometry-change cold path also proven silent |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `audio_quantum_path_is_zero_allocation_after_warm_up` | YES | counting-allocator guard over 512 real quanta; any allocation fails; the geometry-change window pins the cold path too |

Not executed by me (no cargo); W00 workspace run green (this test is
in the 1 751). UNVERIFIED individually.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after one warm-up quantum … allocates zero bytes — measured … not argued from code reading" | :1-5 | REAL (happy path) | the guard/assert_zero mechanism is real; S4 gap F-001 |
| "runs identically under --test-threads=1 and the default parallel runner" | :10-11 | REAL | `serialize_tests()` lock; single test in binary |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

`audio_engine`, `media_types`, `testkit` (alloc_count, Must). No
feature gates; test-only binary.

## 9. Open questions for the next agent

- Wave 16 (test-quality): `testkit::alloc_count` itself — verify the
  guard counts allocations (not deallocations) and is reset per guard;
  the whole proof rests on it.
