# AUDIT: crates/transcode/src/record.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 311 / 7 (Recorder::new/select/target/extra_data/record, Remuxer::new/track/remux) |
| File verdict | UNREACHABLE (real assembly code; no shipped app depends on the transcode crate) |
| Reachable from shipped entry point? | NO — `rg transcode apps/cli apps/desktop` → no Cargo.toml dep, no source use; crate is a workspace member (built/tested in CI) but no binary ships it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p transcode --test record` → 5 passed/0 failed; `cargo test -p transcode --doc record` → 1 passed (no_run example compiles); rg hunts |

## 1. One-paragraph verdict
This file is the thin, honest one-call assembly layer for record-to-WAV and remux-to-MP4 (cards 7.E2, issue #44): builders that pack a `TranscodeGraph` and hand it to `submit`. All the real machinery (bounded queues, cancel tokens, honest-partial finalization, codec cross-checks) lives in runner.rs/wav.rs/mp4mux.rs and is out of scope here. What this file does, it does correctly: I verified the one non-trivial claim — `Remuxer::remux` passes an "inert" `TrackSelection::DefaultAudio` because `submit` derives the real selection from the MP4 config list — against runner.rs:162-171, where `SinkSpec::Mp4Remux` is indeed re-derived into `TrackSelection::Tracks(ids)`; a naive reader would otherwise suspect a video-track-killing bug that is not there. The integration suite is real (5 tests: bit-exact re-demux+re-decode of recorded output, lossless widening, typed factory refusal, typed no-audio refusal, mid-record cancel leaving a playable prefix) and I ran it green. The crate's limitation is shipping: nothing depends on it — the consumer card (7.E3 conversion UI) has not landed, so no shipped binary can record or remux.

## 2. What actually works (verified)
— `cargo test -p transcode --test record` → `test result: ok. 5 passed; 0 failed` (own run, 0.01s). `cargo test -p transcode --doc record` → 1 passed (the `no_run` API example compiles). Cross-verified the inert-selection comment against runner.rs (quote below).

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/record.rs:212 (pub fn record) / :291 (pub fn remux)
Code:
```
    pub fn record(self, limits: JobLimits) -> Result<JobHandle, EngineError> {
```
What is wrong: not a code defect — a wiring gap. `rg 'transcode' apps/cli/Cargo.toml apps/cli/src apps/desktop` finds nothing; no shipped entry point (lumen-cli, desktop) can construct a `Recorder`/`Remuxer`. The card program is honest about the sequencing (7.E3 "Conversion UI + … transcode wiring" is a separate, later card; doc 13 wave program L733-734) and no feature-matrix row claims shipped recording, so this is dead-code-in-shipped-binary, not a false claim. What correct looks like: the 7.E3 consumer landing, or the crate being marked library-only in reachability docs. Confidence: HIGH — Verification: ran rg (exit 1 on apps/* hunts).

F-002 [S4]
Where: crates/transcode/src/record.rs:299-304
Code:
```
                // Inert default: the sink's config list derives the
                // real selection inside `submit` (the sink's docs).
                selection: TrackSelection::DefaultAudio,
                extra_data: None,
```
What is wrong: a `DefaultAudio` value that is *documented as meaningless* is threaded through a public struct field as a landmine — any future refactor that stops overriding the selection in `submit` (or a host that builds `TranscodeGraph` directly by copying this pattern) silently drops every video track from remux. The derivation override is verified real today (runner.rs:162-171), but encoding "ignore this field" as a plausible-looking value instead of making the field an `Option`/`#[doc(hidden)]` is defensive-coding debt. Confidence: HIGH — Verification: read + cross-checked runner.rs; mp4_remux integration tests pass with video tracks.

## 4. Per-function table (file is 311 LOC — table included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Recorder::new | 156-169 | REAL | — | defaults: DefaultAudio + WavPcmTarget::Source |
| Recorder::select | 174-178 | REAL | — | builder |
| Recorder::target | 185-189 | REAL | — | legality checked downstream (WavPcmTarget) |
| Recorder::extra_data | 196-200 | REAL | — | out-of-band decoder config passthrough |
| Recorder::record | 212-232 | REAL | S3 | graph assembly + submit; see F-001 (unreachable in shipped) |
| Remuxer::new | 260-266 | REAL | — | — |
| Remuxer::track | 273-277 | REAL | — | order = output track index |
| Remuxer::remux | 291-310 | REAL | S4 | inert DefaultAudio (F-002), derivation verified in runner.rs |

## 5. Test honesty (tests/record.rs — 5 tests, ran green; the file itself has no unit tests)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| recorder_output_redemuxes_and_redecodes_bitexactly | YES | recorded WAV re-demuxed + re-decoded, byte-exact — the card's AC |
| recorder_widening_target_records_losslessly | YES | target-flavor widening verified lossless |
| recorder_surfaces_the_factory_refusal_typed | YES | missing-decoder job fails typed |
| recorder_without_audio_fails_typed | YES | zero-audio-track refusal |
| mid_record_cancel_leaves_an_honest_playable_prefix | YES | cancel → RIFF/data sizes describe exactly the written samples |
(mp4_remux.rs, 744 LOC, covers the Remuxer half — outside this file's scope; not audited here.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the sink's config list derives the real selection inside `submit`" | record.rs:300-301 | REAL | runner.rs:162-171 re-derives `Tracks(ids)` (quoted in §1) |
| "a partial recording is a playable prefix, never a lie about its own length" | record.rs:33-35 | REAL | mid_record_cancel test (ran) |
| "Live capture is per-source, not a mid-pipeline tap … an honestly-recorded gap" | record.rs:21-28 | REAL (honest) | no tap trait exists; no code claims otherwise |
| "MP4 remux (H.264/AAC stream copy only, no transcode claim)" | doc 13 L733 | REAL | Remuxer sets `decode: None` (record.rs:305) |

## 7. Dead / duplicated / unreachable code
No dead code inside the file. The crate as a whole is unreachable from shipped apps (F-001).

## 8. Dependencies & flags
Deps: media-types, plugin-api (Demuxer), crate-internal graph/job/limits/mp4mux/runner. No feature gates, no unsafe, no I/O in this file.

## 9. Open questions for the next agent
- tests/mp4_remux.rs (744 LOC) and runner.rs deserve their own audits — this file's verdict says nothing about the WAV writer or MP4 muxer internals.
- When 7.E3 lands, re-run the reachability answer for this file.
