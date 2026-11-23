# AUDIT: crates/transcode/src/runner.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 1302 / 24 (incl. 6 tests) |
| File verdict | REAL (crate-internal) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — no app/CLI depends on the `transcode` crate at all |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | rg (callers/deps) exit 0/1; sed -n verifications exit 0; baseline citation |

## 1. One-paragraph verdict
The runner is a genuine, carefully-built two-thread pump/worker pipeline: real bounded
queue, real condvar hand-off, real first-set-wins terminal cell, real panic containment,
real WAV/ES/MP4-remux sinks. It is also 100% dead weight in the shipped product: no
member of `apps/` (cli, desktop, desktop-tauri, web, control-server) depends on the
`transcode` crate, and `rg 'Recorder|Remuxer|transcode' apps/*/src` returns nothing. The
crate's own docs are honest that "transcode" means decode-to-PCM-WAV + verbatim stream
copy (no codec encoders exist); the module doc admits the ES sink is "a verbatim byte
dump today". The name of the crate oversells it; the code does not lie.

## 2. What actually works (verified)
— verified by baseline `cargo-test-workspace.txt` L6337-6449: PACKAGE transcode
exit_code=0 (50 unit + 6 job_lifecycle + 7 mp4_remux + 5 record + 5 wav_roundtrip pass);
runner.rs unit tests are inside the 50. Cited, not re-run (cargo-build-budget).
— verified by `rg -n "transcode" apps/cli/Cargo.toml apps/desktop/Cargo.toml
apps/desktop-tauri/Cargo.toml apps/web/Cargo.toml apps/control-server/Cargo.toml` ->
no matches (exit 1): crate never compiled into any shipped binary.

## 3. Findings
F-001 [S2]
Where: crates/transcode/src/runner.rs:137-241 (whole `submit` entry)
Code: `pub fn submit(graph: TranscodeGraph, limits: JobLimits) -> Result<JobHandle, EngineError> {`
What is wrong: `submit` has zero production callers. Callers are only
`crates/transcode/tests/{wav_roundtrip,job_lifecycle,record}.rs` and the `pub use` in
lib.rs:176. No app manifest lists `transcode` as a dependency. Why it matters: the
feature-matrix row `transcode` ("recording, conversion, stream output", lib.rs:3) is
unreachable from every shipped entry point; users cannot invoke any of this. What
correct looks like: a CLI/Tauri command wiring a `TranscodeGraph`, or the row marked
not-shipped. Confidence: HIGH — Verification: read (rg over all app manifests and src).
F-002 [S3]
Where: crates/transcode/src/runner.rs:1109-1115
Code:
```
fn emit_progress_cadence(shared: &JobShared, watermark: &mut u64) {
    let current = shared.snapshot().bytes_written;
```
What is wrong: per-packet mutex acquisition + full `JobStats` copy (`snapshot()` locks
`JobCore` and copies the stats struct) on every packet in three sink loops, plus
`update_stats` (another lock) per packet from both threads — single-mutex contention on
the 2-thread hot path. Why it matters: per-packet lock traffic is the classic
transcode-throughput killer at scale; bounded but unnecessary (a bytes counter read
would do). What correct looks like: local byte accumulation in the loop, one
update+progress check per N packets. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: crates/transcode/src/runner.rs:903-904
Code: `let writer =\n                    WavPcmWriter::new(output, resolved, format.channels as u16, format.rate)?;`
What is wrong: `format.channels as u16` truncating cast — `channels` is u32 (media_types
AudioFormat); a hypothetical >65535-channel frame silently wraps and writes a corrupt
WAV `fmt ` chunk instead of a typed refusal (WavPcmWriter::new only rejects 0 and
>MAX_CHANNELS *after* the truncation, so e.g. 65536 wraps to 0 -> rejected, but 65537
wraps to 1 -> ACCEPTED with wrong channel count). Why it matters: unchecked `as` cast on
parsed input is checklist 6.2; realistic blast radius is low (no decoder emits that),
but it is exactly the class the crate's "every lossy pair is a typed refusal" posture
forbids. What correct looks like: u32->u16 checked conversion with
`wav_channels_out_of_range`. Confidence: MEDIUM — Verification: read (wrap path traced,
not executed).
F-004 [S4]
Where: crates/transcode/src/runner.rs:398-399
Code:
```
// Only this pump closes for teardown after a
// terminal; the worker never closes first.
```
What is wrong: comment is false as written — the worker's exit layer closes the queue
unconditionally (runner.rs:220 `worker_queue.close();`), including before any terminal
in the handoff-None early-return path. The invariant that actually holds is "the worker
only closes after its exit layer, and a terminal is set by then or by the pump's panic
handler". Why it matters: doc/comment trust erosion in the file whose whole value is its
stop-protocol reasoning. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/transcode/src/runner.rs:1037-1047
Code:
```
        TrackSelection::DefaultAudio => {
            let audio: Vec<&Track> = tracks
                .iter()
                .filter(|track| track.kind == MediaKind::Audio)
                .collect();
            match audio.len() {
                0 => Err(audio_track_missing()),
```
What is wrong: `require_audio` parameter is ignored in this arm (and in the `Tracks`
arm only consulted per-track); behavior happens to be right because DefaultAudio implies
audio, but the parameter is dead in one of three arms — a latent trap if the arm is
reused. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/transcode/src/runner.rs:1-4 vs crate name
Code: `//! The pipeline runner (card 7.E1): [`submit`] turns a validated`
What is wrong: nomenclature debt: nothing in the crate transcodes codecs; "transcode" =
PCM-WAV serialization + verbatim copy + remux. Internally documented (lib.rs
"Honest capabilities"), so not fraud — but the crate/feature-matrix name misleads.
Confidence: HIGH — Verification: read (lib.rs:82-131).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| submit | 137-241 | REAL-UNPROVEN (no prod caller) | — | thread spawn, peer teardown on 2nd-spawn failure correct |
| PipelineThreads::interrupt/join | 255-271 | REAL | — | join order pump-first correct |
| build_plan | 300-329 | REAL | — | exhaustive match, typed impossible arms |
| pump_body | 343-429 | REAL | — | budget check, skip-stat, EOS closes queue w/o terminal (worker owns Completed) — sound |
| worker_body | 440-477 | REAL | — | handoff-None -> stop_or_cancel correct |
| run_decode | 480-577 | REAL | — | bounded flush loop (MAX_FLUSH_CALLS), cancel-during-drain -> Cancelled |
| run_copy_wav | 580-645 | REAL | — | frame_bytes division guarded by WavPcmWriter::new channels/rate validation (wav.rs:110-115) |
| run_raw_es | 649-709 | REAL | — | verbatim dump; flush-only finalize |
| run_mp4_remux | 717-831 | REAL | — | config/track cross-check, BE/timescale refusals typed |
| WavEncodeSink::accept | 883-953 | REAL | F-003 | first-frame resolve, mid-stream change refused typed |
| WavEncodeSink::finish | 874-879 | REAL | — | zero-frame source -> empty output (documented honest) |
| TrackHandoff (new/deliver/close/wake/wait) | 970-1022 | REAL | — | poison-tolerant locks, cancel re-check loop |
| select_tracks | 1031-1071 | REAL | F-005 | absent ids delivered-as-absent (documented) |
| check_copy_legality | 1077-1089 | REAL | — | LE family + exact facts match; BE/non-PCM typed refusals |
| terminate_with / stop_or_cancel | 1093-1105 | REAL | — | first-set-wins via OnceLock (job.rs) |
| emit_progress_cadence | 1109-1115 | REAL | F-002 | 64 KiB step, saturating |
| tests (6) | 1143-1301 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| default_audio_selection_is_exactly_one | YES | asserts picked id + typed codes for 0/many audio |
| explicit_selection_enforces_existence_and_audio_where_needed | YES | typed codes per branch |
| multi_track_selection_yields_every_named_track_in_order | YES | order + absent-id drop pinned |
| copy_legality_matrix | YES | 4 refusal codes + full LE family acceptance |
| progress_cadence_fires_on_step_boundaries | YES | boundary -1/+1 around 64 KiB step (relies on absorb-delta stats, verified job.rs:250-252) |
| too_large_maps_to_the_typed_limit | WEAK | constructor reachability smoke, asserts one error code |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "card 7.E1 pipeline runner ... two threads and a JobHandle" | runner.rs:1-4 | REAL | submit:137-241 spawns both threads |
| "ES sink is a verbatim byte dump today" | runner.rs:57-58 | REAL | run_raw_es:649-709 write_all only |
| "first-set-wins terminals ... wait joins both threads" | runner.rs:36-39 | REAL | job.rs:409-421 join protocol |
| feature-matrix `transcode` row shipping "recording, conversion, stream output" | lib.rs:3 (doc 01 §2.5) | UNREACHABLE | no app depends on crate (F-001) |

## 7. Dead / duplicated / unreachable code
Entire file is unreachable from shipped entry points (F-001). No intra-file dead code
found; the four copy-path loops (copy_wav/raw_es/mp4_remux + decode) share a heavily
duplicated `is_stopping`/`Closed`/`Interrupted` triptych pattern (~4x30 lines) —
candidate for a macro/closure, cosmetic.

## 8. Dependencies & flags
std-only; codec-api, media-types, plugin-api, demux (WAV vocab), in-crate queue/job/
wav/mp4mux/convert/graph/limits/cancel. No features in this crate's Cargo.toml gate
this file (see transcode Cargo.toml audit). No unsafe.

## 9. Open questions for the next agent
- Who was supposed to wire card 7.E1/7.E2 into apps/cli (record command)? Check
  planning docs for the missing host card — the crate is finished but orphaned.
- job.rs `snapshot()` returns `self.stats` by copy while `absorb` mutates — verify
  JobStats: Copy consistency there (job.rs owner).
