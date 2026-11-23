# AUDIT: crates/transcode/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 176 / 0 functions — 11 `pub mod` declarations + 6 re-export groups + one `no_run` doctest (facade/docs file) |
| File verdict | REAL — every "honest capability" the facade claims is backed by submodule code I or prior auditors executed; the crate is UNREACHABLE from shipped entry points and the bounded-resources story inherits queue.rs's S2 lost-wakeup defect |
| Reachable from shipped entry point? | NO — the whole `transcode` crate has zero workspace dependents (no app/tool Cargo.toml references it; rg over apps/, tools/ → 0). Matrix rows `playback-record` / `ui-dialog-convert` honestly not_started. Prior sibling audits (runner/record/job/limits/error/graph/mp4mux) all reach the same conclusion. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 (new in this file; crate-level S2 in queue.rs already filed by W10-A01) |
| Commands run | `./target/debug/deps/transcode-f17733efc4fd5aa3` → 50 passed exit 0; `wav_roundtrip-6d027aba70c47ea9` → 5 passed exit 0; `record-ceccd4cf7efbbea6` → 5 passed exit 0; `mp4_remux-2cce5ceb8b8d5666` → 7 passed exit 0 (this run, file 3); sed quote verification |

## 1. One-paragraph verdict
This is the crate facade: 176 lines of which ~150 are documentation whose claims I verified against executed code. "WAV PCM encode — a real RIFF/WAVE writer": true (wav_roundtrip suite 5/5, I ran it — bit-exact round-trips). "H.264/AAC remux into a minimal in-tree MP4 container": true (mp4_remux suite 7/7, I ran it — payloads byte-verbatim, refusals typed). "No filters … validation refuses each one" / "no codec encoders": matches graph.rs's audited typed-refusal posture. The two blemishes: the "Bounded resources … pin these as tests, not hopes" paragraph overstates — the queue's `interrupt()` lost-wakeup window (W10-A01's S2, queue.rs:178-182: notify without the mutex while the wake condition is an external AtomicBool) means a cancelled job can park a thread forever, which no suite pins; and the whole capability story ships nowhere (no dependent, not compiled into any release binary).

## 2. What actually works (verified)
— verified by my runs: transcode lib unit tests 50/50 (exit 0), wav_roundtrip 5/5 (exit 0, `fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly … ok`), record 5/5 (exit 0), mp4_remux 7/7 (exit 0, see my mp4_remux.rs audit). Baseline corroborates: 50+6+7+5+5 tests across the crate's binaries, 0 failed (audit/runs/baseline/cargo-test-workspace.txt:6337-6454).

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/lib.rs:131-141 (whole-crate scope)
Code:
```
//! Every accumulation point has an explicit, documented ceiling
//! (see [`JobLimits`]' module table): the demux→worker stage queue
//! is bounded on two axes (slots and payload bytes, the streaming
//! stack's doctrine) with parking backpressure; the event hub drops
//! and counts past capacity, never blocking the worker; the track
//! table is budget-checked right after `Demuxer::start`; the WAV
//! writer streams through with no payload buffer; all counters are
//! saturating. The round-trip and backpressure suites pin these as
//! tests, not hopes.
```
What is wrong: the paragraph is accurate about bounds but "pin these as tests, not hopes" conceals that the queue's CANCEL path is not pinned and is in fact defective — W10-A01's queue.rs F-001 [S2]: `interrupt()` notifies both condvars without holding the mutex, so a notify landing in the predicate-eval-to-park window is lost and a cancelled job can leave a parked producer or consumer asleep forever (queue.rs:178-182). The facade repeats the bounded story without the exception. Cross-file citation, not a new defect.
Confidence: HIGH — Verification: read (this file) + prior audit (queue.rs, W10-A01, cargo-test-verified)

F-002 [S4]
Where: crates/transcode/src/lib.rs:22 (`/// ```no_run`)
Code: (L18-20)
```
//! Assemble a [`TranscodeGraph`], then [`submit`] it (a
//! host-wired decode job; the plumbing stubs are the host's real
//! demuxer/decoder, elided):
//!
```
What is wrong: the crate's only doctest is `no_run` with Nop demuxer/decoder stubs — it proves the public surface COMPILES from a consumer's perspective (real value) but executes nothing; a reader could take "Assemble … then submit" as a demonstrated flow. Disclosed by the attribute; cosmetic.
Confidence: HIGH — Verification: read

## 4. Per-function table (facade — no functions; module inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| doc: pipeline diagram + doctest | 11-107 | REAL | S4 | no_run compile-only (F-002) |
| doc: honest capabilities | 109-129 | REAL | — | claims verified by executed suites (see §2) |
| doc: bounded resources | 131-141 | REAL-with-gap | S3 | F-001 queue interrupt exception |
| doc: layer posture | 143-151 | REAL | — | std-only, no unsafe (prior audits concur) |
| mod cancel/convert/error/graph/job/limits/mp4mux/queue/record/runner/wav | 153-163 | REAL | — | all audited by prior agents as REAL code |
| re-exports (CancelToken … submit) | 165-176 | REAL | — | surface matches submodule items |

## 5. Test honesty
No tests in this file. The doctest is compile-checked only (`no_run`) — honest via attribute. The crate's suites (50 lib + 6 job_lifecycle + 7 mp4_remux + 5 record + 5 wav_roundtrip) are real and green (baseline + my runs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "WAV PCM encode — a real RIFF/WAVE writer (16/24/32-bit integer and f32/f64 float)" | lib.rs:111-113 | REAL | wav_roundtrip 5/5 (ran); prior wav audit |
| "H.264/AAC remux into a minimal in-tree MP4 container … sample payloads byte-verbatim" | lib.rs:118-121 | REAL | mp4_remux 7/7 (ran); my mp4_remux.rs audit |
| "Every other conversion is a typed ErrorKind::Unsupported, never a degraded output" | lib.rs:122-123 | REAL | refusal tests in mp4_remux + graph.rs audit |
| "No filters — the graph models filter stages and validation refuses each one" | lib.rs:124-125 | REAL | graph.rs audit (W10-A01: validation-only, honest refusals) |
| "The round-trip and backpressure suites pin these as tests, not hopes" | lib.rs:140-141 | PARTIAL | bounds: pinned; interrupt/cancel path: defective and unpinned (F-001) |
| "L3 … std-only (C5), no unsafe" | lib.rs:143-145 | REAL | prior audits found no unsafe in the crate |

## 7. Dead / duplicated / unreachable code
The entire crate is unreachable from shipped entry points (see header). No duplication inside this file.

## 8. Dependencies & flags
Workspace-internal only: media-types, plugin-api, codec-api, demux (per posture doc L145-151). No features, no external deps (std-only claim). Not in any release lane dependency tree.

## 9. Open questions for the next agent
- If ui-dialog-convert (7.E3) lands, the queue interrupt fix (W10-A01 S2) must land first — a UI cancel button parking the worker thread forever is a user-visible hang.
- The encoder seam (lib.rs:127-129) is a PROPOSAL — nothing in tree implements `Encoder` consumption; confirm before quoting "conversion" anywhere.
