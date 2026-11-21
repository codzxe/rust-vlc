# AUDIT: crates/transcode/src/runner.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 658 (646–1303 of 1303) / 20 items in table |
| Chunk boundary | part2 = 646–1303, extended 6 lines past the equal split (652) to open at `run_raw_es`'s doc comment instead of mid-loop; lines 1–651 belong to A04 (part1). Flagged in §9. |
| File verdict | REAL (library; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. `rg -l 'transcode' --glob 'Cargo.toml'` → only `crates/transcode/Cargo.toml`; no crate under apps/ or crates/ depends on `transcode`. `apps/cli` has zero references. Chain would need to start at `transcode::submit` (lib.rs:176 export) — no caller outside the crate's own tests (`crates/transcode/src/record.rs:132,231,309` is in-crate and itself caller-less from shipped code). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "transcode" apps/cli/Cargo.toml apps/cli/src/*.rs` (no hits); `rg -l 'transcode' --glob 'Cargo.toml'` (only self); `rg -n "fn validate|duplicate" crates/transcode/src/graph.rs` (duplicates refused upstream, graph.rs:359); `sed -n '250,300p' crates/transcode/src/job.rs` (stats = Mutex-locked JobCore); `sed -n '76,84p' crates/media-types/src/frame.rs` (frames_in); `rg -n "MAX_CHANNELS|channels" crates/transcode/src/wav.rs` (WAV channel validation is 1..=MAX_CHANNELS on u16); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5818–5915 (transcode 50 unit + 23 integration tests pass, incl. all 6 runner::tests). |

## 1. One-paragraph verdict
This half of the runner is genuine, carefully-engineered pipeline code: the raw-ES sink, the MP4-remux worker with its config/track cross-check, the lazy WAV encode sink, and the condvar track hand-off are all implemented, panic-safe via the caller's `catch_unwind`, and covered by 6 in-file unit tests plus 4 integration suites that all pass in baseline (`orchrun-test-workspace-nofailfast.log.txt:5868-5915`). It is also completely dead weight in the shipped product: no crate in the workspace depends on `transcode`, so none of this can be reached from `lumen` or `lumen-tauri`. The two real defects are a silent `as u16` channel truncation at a public host-decoder boundary and swallowed finalize errors on the cancel path that can falsify the crate's own "honest prefix" contract.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5818-5915` → `runner::tests` 6/6 ok, `tests/mp4_remux.rs` 7/7 ok (incl. `mid_remux_cancel_leaves_an_honest_playable_prefix`, `progressive_source_remuxes_and_redemuxes_packet_identical`), `tests/wav_roundtrip.rs` 5/5 ok (incl. `raw_es_copy_dumps_the_payload_verbatim`), `tests/record.rs` 5/5 ok, `tests/job_lifecycle.rs` 6/6 ok. These exercise `run_raw_es`, `run_mp4_remux`, `WavEncodeSink`, and `TrackHandoff` end-to-end through `submit`.

## 3. Findings

### F-001 [S3]
Where: crates/transcode/src/runner.rs:903-904
Code:
```
                let writer =
                    WavPcmWriter::new(output, resolved, format.channels as u16, format.rate)?;
```
What is wrong: `format.channels` is `u32` (media-types frame.rs:60); the `as u16` cast truncates before `WavPcmWriter::new`'s `1..=MAX_CHANNELS` validation (wav.rs:110, which receives the already-truncated u16). A host decoder emitting 65537 channels becomes 1 — in range, accepted, silently wrong channel count written into the `fmt ` header; 65536 becomes 0 — refused with `wav_channels_out_of_range(0)`, a false report of the actual value.
Why it matters: this is a public trait boundary (`Box<dyn Decoder>` supplied by any host); the crate's whole posture is typed refusals for impossible values, and this cast converts an impossible value into either a corrupt output or a lying error. The copy path (A04's `run_copy_wav`, facts.channels:u16) does not have this hole.
What correct looks like: range-check `format.channels` (1..=MAX_CHANNELS as u32) before the cast and return the typed `wav_channels_out_of_range(format.channels)` with the true value.
Confidence: HIGH — Verification: read (frame.rs:60 `pub channels: u32`; wav.rs:110 validates the post-cast u16).

### F-002 [S3]
Where: crates/transcode/src/runner.rs:775-778 (and the same shape at 628, 654, 680, 702, 824)
Code:
```
                // The honest prefix: finalize what landed, never claim
                // completion of a stream the user stopped.
                let _ = muxer.finalize();
                return terminate_with(shared, queue, JobOutcome::Cancelled(shared.snapshot()));
```
What is wrong: on the cancel path the remuxer's `finalize` (which writes the `moov` over the `mdat` that already landed) has its `Err` discarded with `let _`. If it fails (seek/write error on the sink), the partial MP4 has no sample tables at all — unplayable — yet the documented contract above the function (711-716: "the partial file's tables describe exactly the samples that made it") and the outcome say an honest playable prefix exists. Same pattern for `writer.finalize()` (WAV, stale RIFF sizes) and `output.flush()` (raw ES).
Why it matters: the cancel path is explicitly tested for "honest playable prefix" (mp4_remux test), but only against a cooperative sink; against a failing sink the user gets a broken file and zero signal — no stat, no event, no log line. A best-effort finalize is fine; an *invisible* one is not.
What correct looks like: fold the failure into `JobStats` (e.g. a `finalization_failed` flag) or emit a terminal-adjacent event so the host can distinguish "clean cancel prefix" from "cancel with broken output".
Confidence: HIGH — Verification: read; `let _` confirmed at 628, 654, 680, 702, 777, 824 via `rg -n "let _ ="` on the range.

### F-003 [S4]
Where: crates/transcode/src/runner.rs:1109-1115
Code:
```
fn emit_progress_cadence(shared: &JobShared, watermark: &mut u64) {
    let current = shared.snapshot().bytes_written;
```
What is wrong: called once per packet from every sink loop; `snapshot()` locks the `JobCore` Mutex (job.rs:255-256) and `update_stats` locks it again per packet (job.rs:250-251), so pump and worker threads take the same uncontended-then-contended lock 2–3× per packet.
Why it matters: pure per-packet overhead on the only data path; harmless at WAV rates, measurable on high-packet-rate ES dumps.
What correct looks like: track the byte watermark locally from the `written` return values (already in hand at every call site) and only touch `JobShared` when the 64 KiB step is crossed.
Confidence: HIGH — Verification: read of job.rs:250-256.

### F-004 [S4]
Where: crates/transcode/src/runner.rs:1300
Code:
```
        let _: EngineError = limit_over_budget(1, 2); // constructor reachability
```
What is wrong: a test line that constructs an error and drops it, asserting nothing; the comment admits it exists only to keep the constructor referenced.
Why it matters: test-hygiene debt of the exact kind this audit hunts — a reader scanning test names could count it as coverage for the limits path.
What correct looks like: assert the constructed error's code/kind like every other test in the module does.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run_raw_es` | 649-709 | REAL (lib-only) | — | Verbatim byte dump; cancel/EOS/interrupt arms all terminate typed; flush errors swallowed per F-002; no frame claim in stats (correct). Exercised by `raw_es_copy_dumps_the_payload_verbatim` (baseline 5911). |
| `run_mp4_remux` | 717-831 | REAL (lib-only) | — | Config×track cross-check refuses missing track/codec/timescale typed; duplicate config ids are impossible here (refused upstream, graph.rs:359 `mp4_track_duplicate` — verified by `rg`); linear `index.find` per packet is budget-bounded; F-002 swallow on cancel. Exercised by 7 baseline tests (5880-5891). |
| `WavEncodeSink::new` | 852-867 | REAL (lib-only) | — | Trivial constructor. |
| `WavEncodeSink::finish` | 874-879 | REAL (lib-only) | — | Empty-output case returns Ok without touching the sink (documented zero-frames ⇒ empty file). |
| `FrameSink::accept` (WavEncodeSink) | 883-952 | REAL (lib-only) | S3 (F-001) | First-frame lazy writer; mid-stream format change refused typed (`wav_format_change`); torn frame refused (`wav_partial_frame`, via `frames_in` multiple-of check, frame.rs:77-83); scratch reused via `mem::take` (no per-frame alloc); `format.channels as u16` truncation is F-001. |
| `TrackHandoff::new` | 971-980 | REAL (lib-only) | — | Trivial. |
| `TrackHandoff::deliver` | 983-988 | REAL (lib-only) | — | poison-proofed lock, notify_all. |
| `TrackHandoff::close` | 994-999 | REAL (lib-only) | — | Pump-exit-layer wake; correct. |
| `TrackHandoff::wake` | 1004-1006 | REAL (lib-only) | — | notify_all without state change; pairs with token re-check in `wait`. |
| `TrackHandoff::wait` | 1010-1021 | REAL (lib-only) | — | Loop re-checks tracks/closed/token on every wake incl. spurious; no missed-wakeup window (deliver/close both notify under the same mutex order). |
| `HandoffState` | 965-968 | REAL (lib-only) | — | Data only. |
| `select_tracks` | 1031-1071 | REAL (lib-only) | — | DefaultAudio 0/1/many typed; Track existence + audio-kind; Tracks delivers absent ids as absent (remux cross-check owns the refusal — documented); `Tracks`+`require_audio` arm is dead by validation (WAV sinks never see multi-track, graph test `multi_track_selection_is_remux_only`) but defensive. |
| `check_copy_legality` | 1077-1089 | REAL (lib-only) | — | LE-family match, BE typed refusal, non-PCM typed refusal; pure string compare. |
| `terminate_with` | 1093-1097 | REAL (lib-only) | — | Observes queue peaks then first-set-wins terminal. |
| `stop_or_cancel` | 1101-1105 | REAL (lib-only) | — | Correct cancel-vs-terminal discrimination. |
| `emit_progress_cadence` | 1109-1115 | REAL (lib-only) | S4 (F-003) | See F-003. |
| `tests::track` (helper) | 1126-1141 | REAL (lib-only) | — | Fixture builder via testkit `.must()`. |
| `default_audio_selection_is_exactly_one` | 1143-1173 | REAL (lib-only) | — | Asserts picks and typed codes; would fail on regression. |
| `explicit_selection_enforces_existence_and_audio_where_needed` | 1175-1200 | REAL (lib-only) | — | Asserts all four arms with codes. |
| `multi_track_selection_yields_every_named_track_in_order` | 1202-1229 | REAL (lib-only) | — | Order + absent-id semantics pinned. |
| `copy_legality_matrix` | 1231-1266 | REAL (lib-only) | — | Full LE/BE/non-PCM matrix with codes. |
| `progress_cadence_fires_on_step_boundaries` | 1268-1294 | REAL (lib-only) | — | Pins the 64 KiB boundary and no-event-below; would catch an off-by-one in the comparison. |
| `too_large_maps_to_the_typed_limit` | 1296-1301 | REAL (lib-only) | S4 (F-004) | Real assert + F-004 dead line. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| default_audio_selection_is_exactly_one | YES | Asserts picked id and both error codes. |
| explicit_selection_enforces_existence_and_audio_where_needed | YES | Four distinct arms each asserted. |
| multi_track_selection_yields_every_named_track_in_order | YES | Order + absence pinned by id. |
| copy_legality_matrix | YES | Codes asserted per family; mutation of any branch flips a code. |
| progress_cadence_fires_on_step_boundaries | YES | Boundary ±1 behavior asserted both sides. |
| too_large_maps_to_the_typed_limit | Partially | First assert real; last line (F-004) asserts nothing. |
| (file-level) run_raw_es/run_mp4_remux/WavEncodeSink/TrackHandoff | YES | No unit tests here, but 18 integration tests through `submit` exercise them (baseline 5868-5915); e.g. mutating `run_mp4_remux`'s index lookup breaks `progressive_source_remuxes_and_redemuxes_packet_identical`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "card 7.E2: H.264/AAC packets → minimal MP4" | runner.rs:290 comment; crates/transcode/Cargo.toml header ("MP4 remux is the later 7.E2 card" — stale, it landed) | REAL (lib-only) | mp4_remux tests 7/7 baseline; UNREACHABLE from shipped binary (no dependents). |
| "The stop protocol … no deadlock and no lost terminal" | runner.rs:19-39 module docs | REAL (lib-only) | `job_lifecycle` cancellation/completion tests pass (baseline 5874-5879); protocol traced by hand: exit-layer `close` + `OnceLock` first-set-wins verified in `terminate` (job.rs:296+). |
| "the partial file's tables describe exactly the samples that made it" | runner.rs:714-716 | SHALLOW | True only when `finalize` succeeds; failure silently discarded (F-002). |

## 7. Dead / duplicated / unreachable code
Whole crate is unreachable from shipped entry points (see header). In-range: `select_tracks`' `require_audio` arm for `Tracks` (1062-1064) is unreachable by validation; kept as defense. The four sink loops (`run_decode`/`run_copy_wav`/`run_raw_es`/`run_mp4_remux`) share a ~35-line structural skeleton (stop-check / Item / Closed / Interrupted) duplicated four times — deliberate explicitness, noted as duplication.

## 8. Dependencies & flags
No feature flags in crates/transcode/Cargo.toml (no `[features]` section — verified by reading the file). std-only; `unsafe_code = deny` inherited. No cfg-gated paths in part2.

## 9. Open questions for the next agent
- A04 (part1): `run_copy_wav:610` computes `frames = len / frame_bytes` (floor) — a torn final packet reports partial frames as whole; check whether the demux layer guarantees whole-frame packets.
- A03 (graph.rs): confirm the `Mp4Remux` duplicate-id refusal (line 359) also guards the *derived* selection path in `submit` (runner.rs:161-170), not just direct graph construction.
- Whoever owns docs/feature-matrix.toml transcode rows: the matrix claims a convert/record surface no shipped binary can reach — expected matrix verdict UNREACHABLE.
