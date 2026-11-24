# AUDIT: tools/mem-audit/src/walk.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 798 / 18 fns (2 pub) |
| File verdict | REAL — window-bracketed decode-walk driver over the real engine crates; compiled + exercised by baseline |
| Reachable from shipped entry point? | NO — dev tool (`tools/mem-audit` workspace member, binary; release lane = lumen-cli + desktop only). Reachable as `walk::walk_with` from tools/mem-audit/src/main.rs:168,172 and main.rs:218-258 tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | od -c line-149 byte check; rg budget-comparison vs playback-engine; baseline cite |

## 1. One-paragraph verdict
This is a real, carefully-built measurement driver: it brackets the open phase
and the walk phase with the counting allocator's windows, routes demuxed
packets to at most two first-vertical limbs (MJPEG + seeded PCM) behind the
real `codec_api::Decoder` contract, mirrors the engine's shared 1000-consecutive
concealment budget (verified equal to `playback-engine`'s
VIDEO_CONCEALMENT_BUDGET, pipeline.rs:62 "one shared budget for both limbs"),
and reports honest `Refused`/`Fatal` verdicts without fabricating numbers. The
baseline compiled the package and passed its 9 tests, two of which drive this
module (cargo-test-workspace.txt:3278-3294). Two minor defects only: the
unprobed-family fallback misattributes the refusal error, and the AVI limb
selection silently ignores everything but the first MJPEG/PCM track pair.

## 2. What actually works (verified)
— Baseline: audit/runs/baseline/cargo-test-workspace.txt:3278
`Compiling lumen-mem-audit v0.1.0` → :3280 `Running unittests src/main.rs` →
`running 9 tests` … `test result: ok. 9 passed; 0 failed` including
`tests::fixture_walks_twice_with_stable_peak ... ok` (this module's public
surface) and `tests::garbage_file_is_a_typed_refusal ... ok`.
— Reachability chain: main.rs:41 `mod walk;` → main.rs:168-172
`walk::walk_with(path, demux_only)` per class → main.rs:84 `fn main()`.
— Budget-parity check: rg `CONCEALMENT_BUDGET` → walk.rs:80 mirrors
crates/playback-engine/src/pipeline.rs:647 (`pub const
VIDEO_CONCEALMENT_BUDGET: u32 = 1000;`), shared between limbs per
pipeline.rs:62 — walk.rs's single `consecutive` counter (L582) matches.
— Display-channel note: line 149's attribute was od-verified byte-exact
(`#[must_use]`); the Bash channel munges it to `#ust_use]` — established
sandbox artifact (W12-A03/W13-A01 logs), NOT a source defect. No corruption.

## 3. Findings
F-001 [S4]
Where: tools/mem-audit/src/walk.rs:383-391
Code:
    match probe_report {
        Ok(_) => Err(last.unwrap_or_else(|| {
            EngineError::new(
                media_types::PipelineStage::Probe,
                ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("memaudit.container.no_demuxer"),
            )
        })),
        Err(error) => Err(error),
    }
What is wrong: when the probe SUCCEEDS on a format with no audit family (e.g.
flac, mpeg-ts), the walk has already burned 4 constructor attempts (mkv/wav/
ogg/mp4) and returns the LAST demuxer start error (an `demux.mp4.*` line for a
file that is not MP4) rather than the no_demuxer refusal — the error names the
wrong container for a probed file. Why it matters: misattributed refusal lines
in an audit tool whose report is the evidence. What correct looks like: prefer
the explicit `memaudit.container.no_demuxer` (or probe's format) whenever the
probe answered a known-but-unroutable family.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: tools/mem-audit/src/walk.rs:405-436 (open_avi limb selection)
Code:
            (MediaKind::Video, StreamFormat::MotionJpeg { .. }) if video.is_none() => {
                video = Some(make_decode_limb(
                    &track.codec,
                    Box::new(codecs::video::mjpeg::MjpegDecoder::new()),
                    track.id,
                    demux_only,
                )?);
What is wrong: only the FIRST video and FIRST audio track get limbs; multi-track
files (e.g. avi_mjpeg_2pcm with two audio tracks — a committed fixture) drop
the extra track's packets silently. Documented (L599-600 "Packets of unbenched
tracks drop here") and the `tracks` field reports the count, so the coverage
truth is visible — but the decode measurement for multi-track files
under-reports by design. Acceptable for a first-vertical audit; noting the
limit.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Family::name | 64-75 | REAL | — | |
| per_frame_damage | 159-167 | REAL | — | mirrors engine rule (bench pattern) |
| file_uri | 173-210 | REAL | — | typed refusals for not-found/perm/non-file/non-utf8 |
| configure | 216-228 | REAL | — | DecoderConfig with FIRST generation |
| Limb::feed | 251-289 | REAL | — | concealment budget shared, saturating; mirrors engine |
| Limb::drain | 293-299 | REAL | — | flush counts |
| Limb::stats | 302-305 | REAL | — | |
| NullSink::accept | 311-315 | REAL | — | decode-to-null |
| open | 342-393 | REAL | S4 (F-001) | family try-list, fresh streams per attempt |
| open_avi | 397-444 | REAL | S4 (F-002) | strf-harvest + seeded PCM |
| open_counting | 451-477 | REAL | — | mkv/mp4/ogg counting-only |
| open_wav | 484-516 | REAL | — | PCM seeded; others counting-only |
| counting_limb / make_decode_limb / make_seeded_pcm_limb | 519-571 | REAL | — | |
| run_loop | 575-617 | REAL | — | routing, Recovered counted, EndOfStream drains |
| probe_format_of / refused_report | 621-672 | REAL | — | honest zeroed walk phase |
| proc_status_kb / reset_vm_hwm | 677-696 | REAL | — | /proc honesty (None, never zero) |
| walk_with | 702-790 | REAL | — | window discipline verified vs doc |
| walk (cfg(test)) | 794-798 | REAL | — | test-only convenience |

## 5. Test honesty
Module has no inline tests; its consumers are main.rs tests (L212-258):
`fixture_walks_twice_with_stable_peak` asserts `Verdict::Decoded` twice — would
fail if the walk crashed or counters were fabricated;
`garbage_file_is_a_typed_refusal` asserts `Verdict::Refused` — would fail if
the tool faked success. Both passed in baseline (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirror apps/cli's decode-bench … exactly" (L5-11) | walk.rs:5-11 | REAL-UNPROVEN (structure read equal; bench.rs not diffed line-by-line) | read |
| "Inside a window the driver performs NO allocation of its own" (L25-32) | walk.rs:25-32 | REAL | read: counters on stack; formatting after close; the one documented exception quoted in §2 |
| "the same 1000-consecutive budget" (L39-40) | walk.rs:39-40 | REAL | pipeline.rs:647 value + shared-limb semantics (pipeline.rs:62) |

## 7. Dead / duplicated / unreachable code
None dead. `Family::Wav`/`Ogg`/`Mkv` reachable only via the fallback try-list
(probe knows avi/mp4 only) — by design (L336-341).

## 8. Dependencies & flags
Depends on codec_api/demux/media_types/media_probe/plugin_api/input — real
engine crates, not mocks. No feature gates. tools/mem-audit/Cargo.toml audit
separate.

## 9. Open questions for the next agent
- Does `main` classify refused families by the misattributed error (F-001)
  into the wrong class column? (report.rs owner.)
