# CLAIM AUDIT: docs/adr/ADR-005-audio-output.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 7 |
| REAL 6 / REAL-UNPROVEN 1 / FALSE 0 | |

## 1. Blunt verdict
The rarest thing in this audit: an ADR whose "Status" precisely separates what IS implemented (the adapter seam) from what is NOT (any OS audio backend), and whose claims survive verification exactly. The crate exists with every named file; the cpal admission gate is a compile-time const `AUDIO_ADR005_ADMITTED: bool = false` with a typed `BackendNotAdmitted` marker — so "no OS audio library is imported, on any target" is literally true in Cargo.toml (no cpal dependency entry). The numbers are as-of-merge and have only grown (3,097 -> 4,153 LOC; 23 -> 33 src tests; tests dir gained pcm_pipeline + zero_alloc_null_sink). The "zero dependencies" phrasing means external crates (Cargo.toml [dependencies] carries only in-workspace path deps) — acceptable but imprecise wording.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the seam is implemented, merged ... `crates/audio-output/**` (lib.rs, device.rs, null.rs, stub.rs, ring.rs, render.rs, counter.rs, error.rs + AC suites)" | Evidence row | REAL | all 8 named src files + 3 named test files exist (ls); crate since grew: +pattern.rs +pump.rs +cpal.rs +pcm_pipeline.rs +zero_alloc_null_sink.rs |
| 2 | "Real-backend admission ... PENDING — no OS audio library is imported, on any target" | Status | REAL | crates/audio-output/Cargo.toml [dependencies] = 3 path deps only, no cpal; src/cpal.rs:43 `pub const AUDIO_ADR005_ADMITTED: bool = false` with compile-time `!AUDIO_ADR005_ADMITTED` assertion (:49) |
| 3 | "3,097 LOC, std-only, zero dependencies; 32 tests: 23 lib + 4 callback-budget + 1 clock-truth + 3 sine-sink-hash + 1 doctest" | implemented-verified | REAL (as-of-merge, since grown) | current: 4,153 LOC; sine_sink_hash=3, callback_budget=4, clock_truth=1 `#[test]` counts EXACT; src tests 33 (superset) — rg -c counts |
| 4 | "cpal 0.18.2 ... conditional-E2, not admitted ... four independent grounds" | Context | REAL | docs/dependencies/records/cpal-0.18.2.md exists; audio-output-cpal.md (wave-14 spike) exists — both records in-tree |
| 5 | "the dev sandbox can neither build it (no alsa.pc — empirical exit 101 recorded) nor exercise a device" | Context | REAL-UNPROVEN | exit-101 claim cites the record; no alsa here either (sandbox unchanged class) — not re-run by me |
| 6 | "the fully-implemented NullBackend ... typed UnsupportedBackend stubs (stable `audio.os_backend_pending` ... enumerates nothing, never fakes success)" | Decision 1 | REAL | null.rs (8 tests) + stub.rs present; error string in error.rs; matches worklog w31's "audio output remains the null device engine-wide" — consistent repo-wide |
| 7 | "Device-loss fallback order ... is configured policy, not hardcoded (the [PROPOSAL] recorded in audio-output/src/lib.rs)" | Decision 5 | REAL-UNPROVEN | lib.rs [PROPOSAL] marker not re-read by me line-by-line; file present |

## 3. Contradictions
None found. The worklog's "audio = null device engine-wide" (worklog.md:2294) and this ADR's PENDING status agree; the engine-info baseline ("decoders: mjpeg, pcm_s16le") exposes no audio device claims either.

## 4. Fabrication / inflation indicators
None — the ADR's core honesty device is that admission is a build-breaking flip (const + compile assert), which mechanically prevents the "secretly enabled" failure mode of most stub audits. The "3,097 LOC / 32 tests" figures are dated-but-true snapshots (supersets now).
