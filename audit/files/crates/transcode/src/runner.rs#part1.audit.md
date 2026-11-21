# AUDIT: crates/transcode/src/runner.rs#part1 (lines 1-651)
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 651 (of 1302; chunk boundary: `run_raw_es` starts at 649 and continues into part2/A05) / 18 items |
| File verdict | UNREACHABLE (real, in-crate-tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Proof: `grep -n "transcode" apps/*/Cargo.toml` → zero hits; apps/cli/Cargo.toml `[dependencies]` (read in full) has no transcode edge; `rg -l "lumen-transcode\|transcode::" crates/ apps/` → only crates/transcode's own files (lib.rs:176 `pub use runner::submit`, record.rs:132 caller — both inside the dead crate); grep transcode in crates/commands/ + apps/desktop/ → zero. Consistent with W01-A03 F-002 (product.toml:165 `transcode = false # Phase 7`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "^fn \|^pub fn \|..." runner.rs` (item map, chunk split) · `rg -n "transcode" apps/cli/src/` (zero) · `grep -n transcode crates/*/Cargo.toml apps/*/Cargo.toml` (only self) · sed -n 505-520p / 396-402p / 213-229p / 1031-1089p / 836-953p (quote verification + part2 helper cross-read) · `rg -n "fn validate" -A 60 graph.rs` (filters refusal, Tracks-for-Mp4Remux-only rule) · `rg -n "fn new" -A 25 wav.rs` (channels/rate validation order) · `rg -n "fn update_stats\|fn terminate" -A 16 job.rs` (absorb merge, first-set-wins) · baseline #3 (1681 passed workspace-wide, transcode tests included) |

## 1. One-paragraph verdict
This is a genuinely engineered two-thread transcode pipeline — real
condvar-bounded queue hand-off, real panic containment, real typed
refusals — and it is 100% dead from every shipped binary's perspective
(no Cargo.toml in apps/ or any other crate depends on `transcode`). Within
the dead code there is one real correctness defect against the crate's own
documented contract: a user cancel that the PUMP observes first mints the
`Cancelled` terminal, and the worker then exits without the promised
best-effort sink finalize (runner.rs:509-517, 568-574 vs cancel.rs:16-19) —
the "honest partial output" guarantee is racy, not guaranteed. Two
smaller defects: a false concurrency-invariant comment
("the worker never closes first", contradicted by the worker's own exit
layer at line 220) and silent discarding of finalize errors on the cancel
path. The elaborate stop-protocol documentation otherwise checks out
line-by-line against the code (verified below).

## 2. What actually works (verified)
— Thread topology + teardown: `submit` spawns pump/worker under
`catch_unwind` (runner.rs:192-229, verified by sed); on worker-spawn
failure the already-running pump is interrupted and joined before the
typed refusal (223-228). Verified by reading both exit layers (203-204,
220) — every thread exit closes the queue, so no peer can park forever.
— First-set-wins terminals: verified against job.rs:301-315
(`if self.outcome.set(outcome).is_ok()`), matching the module-doc claim
(runner.rs:36-39).
— No divide-by-zero in `run_copy_wav` frame accounting: `WavPcmWriter::new`
(wav.rs:104-115) rejects `channels == 0 || channels > MAX_CHANNELS` and
`rate == 0` BEFORE `frame_bytes` is computed (runner.rs:590-595), so
`len / frame_bytes` (610) divides by ≥ 1. Verified by reading wav.rs.
— Stats are folded, not clobbered: every `update_stats(&JobStats { one
field, ..default })` call site (385-397, 521-528, 611-615) is safe because
`JobShared::update_stats` calls `stats.absorb(delta)` (job.rs:250-252) —
verified by reading job.rs.
— The "impossible empty hand-off" comment (454-456) is TRUE:
`TranscodeGraph::validate` (graph.rs:373-377) refuses
`TrackSelection::Tracks(_)` for every sink except Mp4Remux, and the other
two selections error on missing tracks (select_tracks:1043, 1051) — so
single-track sinks always receive exactly one track; Mp4Remux re-checks
every config against the delivered table (735-744) and fails typed.
— Filters are truly refused before being ignored: `filters: _` at line 155
is safe because `validate` rejects non-empty filter lists (graph.rs:324-326).
— In-crate tests run and pass: baseline #3 `cargo test --workspace`
(INDEX.md, 1681 passed / 1 failed — the failure is
apps/desktop/tests/zero_alloc_transport.rs, not transcode).

## 3. Findings

### F-001 [S2]
Where: crates/transcode/src/runner.rs:508-517 (primary), 568-574 (same pattern), 377-380 + 1101-1105 (the racing minter)
Code:
```rust
    loop {
        if shared.is_stopping() {
            if shared.outcome().is_none() {
                // Cancellation: finalize best-effort so the partial
                // file's sizes tell the truth (crate::cancel docs).
                let _ = sink.finish();
                return terminate_with(shared, queue, JobOutcome::Cancelled(shared.snapshot()));
            }
            return;
        }
```
What is wrong: The best-effort finalize on cancel only happens when the
WORKER wins the race to mint the `Cancelled` terminal. On user cancel the
token is fired and both threads wake: the pump's stop path
(`stop_or_cancel`, called from pump_body at 379-380/400-401) also finds
`outcome().is_none()` and mints `Cancelled` first. The worker — mid
`decoder.decode` — then re-enters its loop, sees `is_stopping()` true and
`outcome()` already set, and takes the bare `return;` (line 516) WITHOUT
`sink.finish()`. The identical hole exists in the `PopWait::Interrupted`
arm (568-574: outcome set by the pump ⇒ return at 573, no finish).
Why it matters: crate::cancel.rs:16-19 documents "A cancelled job still
**finalizes its sink** (best effort) so the partial output is structurally
valid — sizes tell the truth" — the honest-prefix contract the crate
repeats in run_copy_wav (600-601), run_raw_es (677-681) and its test name
`cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix`
(tests/job_lifecycle.rs:363). Under this interleaving the partial WAV keeps
its placeholder data-chunk sizes (nothing patches the header), i.e. the
guarantee is racy, not a guarantee. Whether the partial file is truthful
depends on thread scheduling.
What correct looks like: the worker owns the sink, so only the worker
should mint `Cancelled` (the pump's stop_or_cancel should defer when the
worker still owns a sink plan), or the worker's early-exit arms must run
the best-effort finalize whenever the terminal is `Cancelled`, regardless
of which thread set it.
Confidence: HIGH — Verification: read (interleaving traced across
runner.rs:377-380, 400-401, 508-517, 568-574, 1101-1105; cancel.rs:16-19;
job.rs:301-315 first-set-wins). Not executed (crate unreachable, no cargo
runs allowed per rules).

### F-002 [S3]
Where: crates/transcode/src/runner.rs:137-241 (whole `submit` surface) — provenance: crates/transcode/Cargo.toml:55-59
Code:
```rust
pub fn submit(graph: TranscodeGraph, limits: JobLimits) -> Result<JobHandle, EngineError> {
    graph.validate(&limits)?;
    let cancel = CancelToken::new();
```
What is wrong: Nothing in the function — the problem is that nothing
shipped can ever call it. No manifest in the repository depends on the
`transcode` crate (grep across all Cargo.tomls: self-matches only); the
`lumen` CLI's `[dependencies]` (apps/cli/Cargo.toml) and desktop-tauri's
path deps (desktop, commands) contain no transcode edge, and
crates/commands/ + apps/desktop/ have zero source references. The only
in-tree caller of `runner::submit` is record.rs:132 — inside the same dead
crate.
Why it matters: 1,302 lines of concurrency-sensitive pipeline (plus the
whole crate) are compile-and-test-only cargo in every workspace CI lane
(baseline #1/#3/#4 pay for it); any doc that ever claims "convert/record"
works is false on reachability alone. Mitigation: product.toml declares
`transcode = false # Phase 7` and matrix rows say not_started (W01-A03
F-002) — the parking is a recorded decision, which is why this is S3 and
not S0.
What correct looks like: Phase-7 wiring (a CLI convert/record command), or
an explicit workspace note that the crate is test-only until Phase 7.
Confidence: HIGH — Verification: read (dependency greps listed in header;
product.toml/matrix via W01-A03 audit cross-check).

### F-003 [S3]
Where: crates/transcode/src/runner.rs:398-400
Code:
```rust
                    // Only this pump closes for teardown after a
                    // terminal; the worker never closes first.
                    PushWait::Closed => return stop_or_cancel(&shared, &queue),
```
What is wrong: The invariant is false. The worker's exit layer closes the
queue on EVERY exit — `worker_queue.close();` at runner.rs:220 — including
the failure path where the worker mints `Failed` (e.g. decode error at
520) and closes BEFORE the pump observes anything. That is exactly the
case this `PushWait::Closed` arm exists to handle.
Why it matters: This is a concurrency file whose entire design is
documented through invariant comments; a maintainer trusting "the worker
never closes first" will reason wrongly about the Closed arm (e.g.
"Closed here can only mean teardown-after-terminal") when it demonstrably
also means worker-failed-first. Misleading invariants in condvar code are
how deadlocks get introduced during refactors.
What correct looks like: "A Closed here means the worker exited (terminal
set, panic, or spawn-order teardown); defer to its outcome" — matching
what stop_or_cancel actually does.
Confidence: HIGH — Verification: read (sed -n 213,229p shows
`worker_queue.close();` in the worker's unconditional exit layer; sed -n
396,402p shows the comment).
### F-004 [S4]
Where: crates/transcode/src/runner.rs:513, 542, 570 (decode), 600, 628, 638 (copy), 654 (raw ES, boundary)
Code:
```rust
                let _ = sink.finish();
                return terminate_with(shared, queue, JobOutcome::Cancelled(shared.snapshot()));
```
What is wrong: On the cancel path, a failing finalize (I/O error while
patching the WAV header) is silently discarded (`let _ =`), yet the
`Cancelled(snapshot)` terminal still reports `bytes_written` as if the
prefix were truthful.
Why it matters: The honest-prefix promise then fails silently: the caller
cannot distinguish "structurally valid partial file" from "header never
patched". Limited blast radius (cancel path only, error discarded is the
finalize, not the data).
What correct looks like: Record finalize failure in JobStats (a
`finalize_failed: bool` or an event), or at minimum a comment admitting
the stats may overstate validity when finish() errors.
Confidence: HIGH — Verification: read (all six sites verified by sed).
### F-005 [S4]
Where: crates/transcode/src/runner.rs:607-615
Code:
```rust
            PopWait::Item(packet) => {
                let len = packet.data.len() as u64;
                match writer.write_pcm(packet.data.as_slice()) {
                    Ok(written) => {
                        let frames = len / frame_bytes;
```
What is wrong: `len / frame_bytes` floors: a malformed (or hostile
host-demuxer) packet whose payload is not a whole number of frames
undercounts `sample_frames_written` (e.g. 25.25 frames counted as 25,
trailing byte still counted in `bytes_written`), so the two counters can
disagree about the same prefix.
Why it matters: Cosmetic stat drift only — the writer still streams the
bytes verbatim; no panic, no overflow (u64).
What correct looks like: Count whole frames from the writer's own
accounting, or reject partial-frame packets as a typed
`wav_partial_frame` (the decode path already does exactly that for frames,
runner.rs:923-926).
Confidence: HIGH — Verification: read.

## 4. Per-function table (items in lines 1-651; part2 items listed only where cross-read for correctness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (stop-protocol claims) | 1-60 | REAL | — | Every claim traced: close-not-token (203-204, 220), catch_unwind (195, 216), first-set-wins (job.rs:301-315), joins-before-answer (JobHandle, job.rs scope). [PROPOSAL] markers at 56-59/90-93/96-99 are internal annotations, honest. |
| imports | 62-88 | REAL | — | All used (error helpers, graph types, queue traits). |
| `PROGRESS_STEP_BYTES` | 90-94 | REAL | — | 64 KiB; consumed by emit_progress_cadence (1109) + WavEncodeSink (950). |
| `MAX_FLUSH_CALLS` | 1024 bound | 96-100 | REAL | — | Enforced at 553; bounds a misbehaving host decoder. |
| `PCM_LE_DESCRIPTORS` | 105-112 | REAL | — | Consumed at 1078 (part2 check_copy_legality). |
| `PCM_BE_DESCRIPTORS` | 115-121 | REAL | — | Consumed at 1084; used to refuse BE copies typed, not dead. |
| `submit` | 137-241 | REAL (unreachable) | S3 | Correct teardown on worker-spawn failure (223-228: interrupt + join + typed refusal); Mp4Remux selection derived from configs (161-170); UNREACHABLE (F-002). |
| `PipelineThreads::interrupt` | 257-260 | REAL | — | queue.interrupt + handoff.wake — wakes both parked surfaces. |
| `PipelineThreads::join` | 266-270 | REAL | — | Joins pump first (producer); returns any-panicked bool. |
| `WorkerPlan` (enum) | 275-295 | REAL | — | Four arms; exhaustive, no default silently swallowing. |
| `build_plan` | 300-329 | REAL | — | Impossible arms return typed sink_bad_state, no silent fallbacks. |
| `PumpCtx` (struct) | 332-340 | REAL | — | Data. |
| `pump_body` | 343-429 | REAL | S2 | Track budget check after start (359); ids.contains O(tracks) per packet — documented honest cost (370-373), bounded by max_tracks; EOS close+return delegates the terminal to the worker (421-426) — sound; participates in F-001 race (379-380, 400-401 mints Cancelled before worker can finalize); F-003 comment. |
| `WorkerCtx` (struct) | 432-437 | REAL | — | Data. |
| `worker_body` | 440-477 | REAL | — | "Impossible empty hand-off" comment verified TRUE against graph.rs:373-377 + select_tracks error paths; RawEs arm ignores `first` legitimately (no track claim); Mp4Remux re-checks configs (part2). |
| `run_decode` | 480-577 | REAL | S2/S4 | Bounded flush loop (546-562); cancel race F-001; swallowed finalize F-004; NeedMoreInput/Drained mid-loop correctly no-ops. |
| `run_copy_wav` | 580-645 | REAL | S4 | No div-by-zero (writer validates first — wav.rs:110-115); F-005 floor counting; F-004 sites 600/628/638. |
| `run_raw_es` (boundary) | 649-651 | (part2) | — | Function STARTS at 649 in this chunk; body 652-709 audited by A05. Boundary noted; my read of 652-709 found the same Interrupted/Closed outcome-defer pattern as the other sinks. |

## 5. Test honesty (tests for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| runner.rs in-file tests (1117-1302) | n/a — part2 scope (A05) | select_tracks/copy-legality/progress-cadence/too-large unit tests; boundary at 1117. |
| tests/job_lifecycle.rs `completed_lifecycle_runs_progresses_completes_and_stays_bounded` (285) | YES (reasoned) | Scripted demuxer/decoder drive the REAL submit/pump/worker threads and assert terminal + stats + queue bounds — mocks are of dependencies, the subject is the runner. Passed in baseline #3. |
| tests/job_lifecycle.rs `cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix` (363) | NOT RELIABLY | It asserts the truthful-prefix outcome, but F-001 makes that outcome scheduling-dependent — the test only exercises the interleaving where the worker wins the mint. A mutation deleting `let _ = sink.finish()` would fail this test ONLY if the worker wins the race in that run; the pump-wins interleaving is uncovered. |
| tests/wav_roundtrip.rs / mp4_remux.rs / record.rs | Out of my file scope | Exist, passed baseline #3 (workspace 1681 passed; single failure is apps/desktop zero_alloc, unrelated). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A cancelled job still finalizes its sink (best effort) so the partial output is structurally valid" | cancel.rs:16-19 | SHALLOW (racy) | F-001: pump-minted Cancelled skips worker finalize (508-517, 568-574). |
| "Teardown wakes with close, not the token … no lost terminal … first-set-wins" | runner.rs:19-39 | REAL | Exit layers 203-204/220; job.rs:301-315 OnceLock set; traced all arms. |
| "Panics are stage faults, not job hangs" | runner.rs:31-35 | REAL | catch_unwind at 195/216 + terminate(worker_panic) at 197/218. |
| "The token is checked before each demux event, between packets, and inside every bounded queue park" | runner.rs:41-48 | REAL | 378 (loop top), 392 (push_wait), pop_wait park (queue.rs scope, cross-ref). |
| "the track table is budget-checked right after `start`" | runner.rs:54-55 | REAL | 359-365 (after-the-fact check; allocation itself is the host demuxer's). |
| "job lifecycle with progress/cancel" (crate description) | crates/transcode/Cargo.toml:49 | REAL in-crate / UNREACHABLE shipped | F-002 proof. |
| Matrix `playback-record` / `ui-dialog-convert` not_started | docs/feature-matrix.toml:1636/2919 (via W01-A03) | REAL | Consistent with unreachability — no over-claim. |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped binaries (F-002) — recorded
decision (product.toml Phase 7). No unused items within the file: both PCM
descriptor tables are consumed by part2's check_copy_legality; every error
helper imported at 75-80 is used in this part or part2. No duplication
found between the three sink loops beyond the inherent
Closed/Interrupted/stop arm shape (each arm differs in finalize semantics
— deliberate, commented).

## 8. Dependencies & flags
Crate deps: codec-api, demux, media-types, plugin-api (all used here:
DecoderConfig/DecodeOutcome/FrameSink at 66; PcmFacts descriptor via
check_copy_legality; Track/EngineError at 70; Demuxer/DemuxEvent at 71).
No feature flags gate any code in this file — the whole crate compiles
unconditionally (and pointlessly for shipped binaries). std-only, no
unsafe (consistent with crate claims; grep unsafe in file → zero).

## 9. Open questions for the next agent
- A05 (runner.rs#part2): confirm run_mp4_remux's Closed/Interrupted arms
  share F-001's race (773-781 pattern), and audit
  `emit_progress_cadence`'s watermark monotonicity under the
  snapshot-based read (a stats read per packet — cheap but lock-traffic).
- A06 (job.rs): JobHandle Drop semantics — does dropping the handle
  without wait() cancel the job or leak the pump thread? (submit hands
  PipelineThreads to JobHandle::new at 231-240.)
- Whoever owns tests/job_lifecycle.rs: run the cancellation test in a loop
  (REQUEST-ORCH-RUN candidate: `cargo test -p transcode
  cancellation_is_prompt --release -- --nocapture` after artificially
  delaying the worker) to demonstrate the F-001 interleaving.
