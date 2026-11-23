# AUDIT: crates/audio-output/src/error.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 96 / 5 constructors + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli (lumen-cli) opens the NullBackend; every constructor has production callers in the crate's reachable paths (ring.rs, null.rs, pump.rs, pattern.rs, counter.rs — call sites listed below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed -n (quotes); rg constructor call sites (16 hits across src); baseline citation cargo-test-workspace.txt:158 (test ok) |

## 1. One-paragraph verdict
Five small, real, typed error constructors, each stamping
PipelineStage::AudioOutput and a stable dotted code, each with production
call sites verified by rg: ring_full (null.rs:193,214), pump_backpressure
(pump.rs:206), partial_frame (pump.rs:190, null.rs:185,252), format_unsupported
(null.rs:82), unsupported (9 call sites with distinct codes: ring_shape_invalid,
pattern_invalid, pump_high_water_invalid, pump_clock_mismatch, sample_rate_invalid,
format_mismatch, render_buffer_unaligned, sink_not_open, device_id_unknown).
No stubs, no swallowed errors, no lies. Two hygiene defects: the requested
format in format_unsupported is stringified into an std::io::Error source
(callers cannot recover channels/rate programmatically), and the in-file test
covers only 2 of the 5 constructors directly (the other three are covered
indirectly by the ring/pump/pattern suites, verified by rg + baseline).

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:158 —
  `test error::tests::errors_carry_stage_and_stable_codes ... ok` (part of
  the crate's 33/33 lib tests, exit_code=0 at line 240).
- Indirect coverage verified: pump.rs tests assert `audio.pump_backpressure`
  and `audio.pump_high_water_invalid`; ring.rs tests assert
  `audio.ring_shape_invalid`; pattern.rs tests assert `audio.pattern_invalid`;
  pcm_pipeline.rs asserts the ResourceLimit kind (lines 217-221).
- Every code string emitted here is asserted somewhere in the crate's suites
  (rg-verified call-site → test-code matches).

## 3. Findings
F-001 [S4]
Where: crates/audio-output/src/error.rs:23-36
Code:
```
pub fn format_unsupported(want: &AudioFormat) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Device {
            which: DeviceSlot::AudioOutput,
            cause: DeviceCause::FormatUnsupported,
        },
        ErrorContext::new("audio.format_unsupported"),
    )
    .with_source(std::io::Error::other(format!(
        "wanted {:?} {}ch @ {}Hz",
        want.sample, want.channels, want.rate
    )))
}
```
What is wrong: the structured AudioFormat is flattened into a display string
carried by an unrelated std::io::Error. A caller (UI, settings validation)
cannot programmatically recover which format was wanted — they must parse a
human string. The typed ErrorKind (Device/FormatUnsupported) is right; the
payload is stringly-typed. What correct looks like: an ErrorKind variant (or
context field) carrying the AudioFormat.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/audio-output/src/error.rs:84-95
Code:
```
    #[test]
    fn errors_carry_stage_and_stable_codes() {
        let e = ring_full(64, 128);
        assert_eq!(e.stage, PipelineStage::AudioOutput);
        assert_eq!(e.context.code, "audio.ring_full");
        assert!(e.to_string().contains("[audio-output]"), "{}", e);
        assert!(e.to_string().contains("audio.ring_full"), "{}", e);

        let e = partial_frame(8, 3);
        assert_eq!(e.context.code, "audio.partial_frame");
        assert!(e.to_string().contains("truncated"), "{}", e);
    }
```
What is wrong: the in-file test covers ring_full and partial_frame only;
unsupported, format_unsupported, and pump_backpressure have no direct
constructor test here (their codes are pinned indirectly in other suites —
pump.rs, pcm_pipeline.rs — verified). If format_unsupported's kind or source
formatting regressed, no test in this file would notice (the null.rs
open-with-bad-format path would catch the code only).
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (file <100 LOC — table optional, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unsupported | 13-19 | REAL | — | generic typed constructor; 9 production codes |
| format_unsupported | 23-36 | REAL | S4 | stringly-typed format detail (F-001) |
| ring_full | 41-51 | REAL | — | ResourceLimit/QueueSize; null.rs:193,214 |
| pump_backpressure | 57-67 | REAL | — | pinned by pump.rs + pcm_pipeline.rs tests |
| partial_frame | 70-78 | REAL | — | Corrupt/Truncated; three call sites |
| tests::errors_carry_stage_and_stable_codes | 84-95 | REAL | S4 | 2 of 5 constructors directly (F-002) |

## 5. Test honesty
The in-file test would fail if the stage stamp, code, or Display rendering
broke for ring_full/partial_frame. It cannot detect breakage in the other
three constructors. Indirect pins exist and pass (baseline). No tautologies.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Typed error constructors for the output path (doc 01 §3.4 taxonomy)" | header:1-2 | REAL | all constructors typed; stage pinned by test |
| "v0 devices speak interleaved f32 only" | :21-22 comment | REAL | null.rs enforces (format_unsupported at null.rs:82) |

## 7. Dead / duplicated / unreachable code
None. All five constructors have live callers (rg-verified, §1).

## 8. Dependencies & flags
Deps: media-types (error taxonomy, AudioFormat), std. No features, no unsafe.

## 9. Open questions for the next agent
- Is there an ErrorKind variant planned to carry structured format data, or
  is the io::Error-source pattern the accepted taxonomy (check media-types
  error.rs owners)? If accepted, downgrade F-001 to noise.
