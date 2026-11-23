# AUDIT: crates/transcode/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 807 / 45 (42 pub constructors, `mint`, `FactsMismatch` Display+Error, 2 tests) |
| File verdict | REAL — UNREACHABLE (whole `transcode` crate has zero workspace dependents) |
| Reachable from shipped entry point? | NO (no Cargo.toml in the repo lists `transcode` as a dependency; `rg transcode apps/` → 0 hits; crate is a workspace member that builds and tests but nothing ships it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/transcode-f17733efc4fd5aa3 error::` -> 2 passed, exit 0; full binary -> 50 passed, exit 0 (binary newer than source); rg Cargo.toml dependent scan; sed verifications |

## 1. One-paragraph verdict
A clean typed-error vocabulary module: 42 one-line constructors minting `EngineError`s at stage `Transcode` with stable dotted codes, one table test pinning every stage/kind/code triple (would fail on any drift), and a second test pinning source-chain and decoder-fault kinds. It does exactly what its docs claim, and the honest-capability refusals (`filter_unimplemented`, copy/codec refusals) are typed refusals, not fake implementations. Defects: every I/O failure (including ENOSPC — the test's own "no space" example) is classified `Inaccessible{PermissionDenied}`, which is a taxonomy lie for non-EACCES errors; two constructors take a diagnostic parameter and silently discard it. The entire crate is dead weight in the shipped product: nothing depends on it.

## 2. What actually works (verified)
— `./target/debug/deps/transcode-f17733efc4fd5aa3 error::` -> `2 passed; 0 failed`; full run `50 passed; 0 failed`, exit 0. The 36-entry table asserts `error.stage == Transcode`, `error.kind == expected`, `error.context.code == expected` per constructor; the io/panic test asserts `Error::source(...).is_some()` and the mismatch leaf renders both descriptors.

## 3. Findings

F-001 [S3]
Where: crates/transcode/src/error.rs:309-339
Code:
```
pub fn sink_write_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "transcode.sink.write-io",
    )
    .with_source(err)
}
```
What is wrong: all three I/O constructors (`sink_write_io`, `sink_seek_io`, `worker_spawn_io`) hard-code `AccessDenied::PermissionDenied` for *any* `io::Error` — including `ENOSPC` (disk full), `EIO`, `ENOMEM` (spawn failure). The crate's own test feeds `std::io::Error::other("no space")` (L761) and thereby pins that a disk-full transcode target is reported as "permission denied". The dotted code and the chained source are accurate, but the `ErrorKind` — the user-facing classification — lies for every non-EACCES cause. The `AccessDenied` enum (media-types/src/error.rs:100-111) has no generic-Io variant, which is the root gap. Why it matters: error-kind-driven UX (retry vs. free-up-space vs. check-permissions hints) misfires on the most common transcode failure (full disk). What correct looks like: map `io::ErrorKind` → variant (EACCES/EPERM → PermissionDenied, ENOSPC → a ResourceLimit/NoSpace kind), or add an Io variant to the taxonomy. Confidence: HIGH — Verification: read + test run (the "no space" case is literally pinned).

F-002 [S4]
Where: crates/transcode/src/error.rs:121-127
Code:
```
pub fn audio_track_ambiguous(found: usize) -> EngineError {
    let _ = found;
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.audio-track-ambiguous",
    )
```
What is wrong: the `found` count (how many audio tracks the demuxer exposed — the datum that makes "ambiguous" actionable) is accepted and discarded. Misleading signature; the caller's information is swallowed. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/transcode/src/error.rs:267-273
Code:
```
pub fn wav_channels_out_of_range(channels: u32) -> EngineError {
    let _ = channels;
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.wav.channels-out-of-range",
    )
```
What is wrong: same as F-002 — the offending channel count is dropped; "out of range" without the value. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 16-18 | REAL | — | uniform stage/kind/code mint |
| graph_decode_required / _forbidden / copy_facts_missing / selection_tracks_mismatch | 22-55 | REAL | — | graph-shape refusals, codes match names |
| filter_unimplemented | 59-64 | REAL | — | honest v1 capability refusal (documented; not a fake filter) |
| filter_stages_over_budget | 67-76 | REAL | — | QueueSize limit, budget+attempted carried |
| limit_zero / limit_over_budget | 79-97 | REAL | — | budget facts carried |
| tracks_over_budget | 100-109 | REAL | — | facts carried |
| audio_track_missing | 112-117 | REAL | — | |
| audio_track_ambiguous | 121-127 | REAL | S4 | F-002 (found discarded) |
| track_not_found / track_not_audio | 130-143 | REAL | — | |
| copy_not_pcm / copy_byte_order | 146-171 | REAL | — | descriptor in FormatInfo |
| copy_facts_mismatch + FactsMismatch Display/Error | 176-206 | REAL | — | both sides rendered (test-pinned) |
| wav_codec_not_pcm / frame_not_audio | 210-228 | REAL | — | |
| wav_conversion_unsupported | 234-244 | REAL | — | "from->to" in FormatInfo |
| wav_format_change / wav_partial_frame | 248-264 | REAL | — | |
| wav_channels_out_of_range | 267-273 | REAL | S4 | F-003 (channels discarded) |
| wav_rate_zero / wav_byte_rate_overflow | 276-292 | REAL | — | |
| wav_size_overflow | 296-305 | REAL | — | RIFF u32 ceiling as budget |
| sink_write_io / sink_seek_io / worker_spawn_io | 309-339 | REAL | S3 | F-001 (all I/O → PermissionDenied) |
| worker_panic / flush_undrained | 344-362 | REAL | — | Decoder{BadState}; panic→error doctrine |
| sink_bad_state | 367-374 | REAL | — | typed stand-in for impossible state |
| mp4_tracks_empty / track_duplicate / config_track_missing / track_index | 378-407 | REAL | — | |
| mp4_codec_unsupported | 413-423 | REAL | — | copy set h264/aac only — honest refusal |
| mp4_timescale_not_native / stamp_missing / dts_non_monotonic | 429-454 | REAL | — | |
| mp4_samples_over_budget / size_overflow / sample_entry_over_budget | 459-494 | REAL | — | ceilings as ResourceLimit |
| tests::constructors_mint_stage_transcode_kinds_and_codes | 503-755 | REAL | — | pins all 36 table triples; would fail on drift |
| tests::io_and_fault_constructors_carry_sources_and_kinds | 759-805 | REAL | — | source-chain + remaining 6 constructors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| constructors_mint_stage_transcode_kinds_and_codes | YES | exact stage/kind/code equality per constructor; kind mismatch or code typo fails |
| io_and_fault_constructors_carry_sources_and_kinds | YES | asserts `Error::source().is_some()`, exact kinds/codes, leaf Display content |
Limitation: the table pins the vocabulary, not that any call site ever fires these errors (coverage of the raising paths lives in the crate's other modules/tests — see job_lifecycle.rs audit).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the crate's whole mint vocabulary, one table, tested in one place" | error.rs:1-3 | REAL | 42 constructors; 36-row table + 6 in second test; ran green |
| "Codes are part of the compatibility surface once a release ships, so additions are additive-only" | error.rs:7-8 | REAL-UNPROVEN | no release ships this crate (zero dependents) — the promise is currently vacuous |
| "folded with the OS error as the chain root" | error.rs:307-308 | REAL | `.with_source(err)` + test asserts source present |

## 7. Dead / duplicated / unreachable code
Nothing dead inside the file. The entire `transcode` crate is UNREACHABLE from shipped entry points: `rg 'transcode\s*=' **/Cargo.toml` → no matches; no app source references it. Workspace membership (members = ["crates/*"]) keeps it compiled and tested only.

## 8. Dependencies & flags
media-types (EngineError/ErrorKind/ErrorContext/PipelineStage, AccessDenied, CorruptDetail, FormatInfo, LimitKind), thiserror unused here (leaf hand-impls Display/Error). No features, no unsafe.

## 9. Open questions for the next agent
- job_lifecycle.rs (file 4): do the lifecycle tests actually drive these error constructors, or only happy paths? (audited next)
- Should `AccessDenied` grow an `Io`/`NoSpace` variant before any consumer ships? Root cause of F-001 lives in media-types/src/error.rs:100-111 (another agent's file).
