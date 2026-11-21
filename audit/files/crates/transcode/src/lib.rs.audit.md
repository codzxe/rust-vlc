# AUDIT: crates/transcode/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 177 / 4 items (module docs incl. doctest, 11 `pub mod` declarations, 5 re-export groups) |
| File verdict | REAL (crate root; docs + wiring only) / UNREACHABLE from shipped — the transcode crate has zero dependents |
| Reachable from shipped entry point? | NO — `rg "transcode = \{ path" **/Cargo.toml` → zero hits; no app or crate depends on transcode; `lumen`/`lumen-tauri` compile none of it. The crate exists for the Phase-7 (7.E1–7.E3) consumer that has not landed (matrix rows playback-record/ui-dialog-convert honestly `not_started`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read of lib.rs:1-177 complete; `rg -c "unsafe"` over transcode/src (doc-comment mentions only — zero `unsafe` blocks/fns/impls, verifying the :145 claim); `rg "unwrap\(\)|expect\(|panic!|todo!|unimplemented!"` on lib.rs (zero); `ls crates/transcode/src` (all 11 declared modules exist as files); `rg -ln "crate::convert|crate::queue|crate::wav"` (no orphaned module — limits/record/runner consume them); `rg -n "pub fn submit" runner.rs` (:137, real); matrix rows playback-record (:1630+)/ui-dialog-convert (:2183+) read (not_started, honest); baseline: transcode tests/doctests NEVER RAN (both baseline runs abort before the transcode test binary — doctest is `no_run` anyway) |

## 1. One-paragraph verdict
A clean crate root: the capability ledger ("honest capabilities") matches what the code actually contains — I verified the MP4 remux claims against my mp4mux.rs#part2 audit (real, with the two truncation defects noted there), the no-filters/no-encoder refusals are stated as refusals, the dependency posture (L3, std-only, no unsafe) is factually correct (zero unsafe blocks in the whole crate), and every declared module exists with internal consumers. Nothing in this file lies. The one structural truth the docs do not say out loud: the crate is an orphan — zero dependents, nothing shipped can reach it, and its execution evidence (round-trip/backpressure suites) exists as files but never ran in the committed baseline.

## 2. What actually works (verified)
— Capability ledger vs code: "WAV PCM encode … every lossy pair is a typed refusal" (convert/ wav lanes — A05's files, existence verified), "H.264/AAC remux into a minimal in-tree MP4 container" (mp4mux — audited by me this session: REAL with S3/S4 defects), "No filters — the graph models filter stages and validation refuses each one", "No codec encoders — WAV PCM is container serialization" — each claim names a real module; none overclaims (the `[PROPOSAL]` markers on 7.E3 are explicit).
— `no unsafe` claim (:145) verified: `rg "unsafe \{|unsafe fn|unsafe impl"` over transcode/src → zero matches.
— All 11 `pub mod` targets exist as files; the four not re-exported at root (convert, error, queue, wav) are consumed internally (limits.rs, record.rs, runner.rs reference them) — no orphaned module.
— The doctest (:22-107) is honestly marked `no_run` with the stubs elided via `#` lines and the caveat "the plumbing stubs are the host's real demuxer/decoder, elided" — it proves the public API shape compiles, and claims nothing more.
— Zero `unwrap/expect/panic/todo/unimplemented` in the file (workspace lints would deny them anyway).

## 3. Findings
None. A file with zero findings must say exactly what was checked (per instructions):
1. Fakeness — no code bodies at all beyond re-exports; no todo/unimplemented/log-only bodies (grep: zero).
2. Correctness — no logic to check; re-export names verified against their source modules (`submit` runner.rs:137, `Mp4Remuxer` mp4mux.rs:194, `Recorder/Remuxer` record.rs, `CancelToken` cancel.rs, `JobLimits` limits.rs).
3. Doc claims — all six capability/bounds claims traced (§6 below); the one soft claim ("round-trip and backpressure suites pin these as tests, not hopes") is real-as-files, unproven-as-executed (baseline gap, not a doc lie).
4. Architecture — module list complete, layering claims match Cargo.toml (codec-api/demux/media-types/plugin-api, codecs/input dev-only).
5. Tests — none in this file; the doctest is `no_run` by design (compilation proof only).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (capability ledger) | 1-151 | REAL | — | Every claim traced (§6); no overclaim found. |
| doctest | 22-107 | REAL | — | `no_run`, stubs elided and disclosed; API-shape proof only. |
| `pub mod` × 11 | 153-163 | REAL | — | All files exist; no orphans (internal consumers verified). |
| re-exports | 165-176 | REAL | — | Names verified against source items. |

## 5. Test honesty
No tests in this file. The doctest is `no_run` (compiles, never executes — would fail on API breakage, proves no behavior; appropriate for an example needing a real source). The crate's behavioral evidence lives in tests/{wav_roundtrip,mp4_remux,record,job_lifecycle}.rs — all unassigned in wave 10 and never executed in the committed baseline (both baseline runs abort before transcode's test binaries).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no `unsafe`" | :145 | REAL | rg: zero unsafe blocks/fns/impls in transcode/src. |
| "Every accumulation point has an explicit, documented ceiling" | :131-138 | REAL-UNPROVEN | mp4mux ceilings verified by me (MAX_REMUX_SAMPLES/MAX_SAMPLE_ENTRY_BYTES enforced); queue/hub/track-table ceilings are queue.rs/job.rs territory (A05/A04) — existence of JobLimits module verified, enforcement not re-audited here. |
| "all counters are saturating" | :139-140 | REAL | Consistent with the saturating counters I read (mp4mux put_u32_saturating policy; JobStats update sites in runner.rs:792 use shared.update_stats — spot-checked, full audit is runner's). |
| "The round-trip and backpressure suites pin these as tests, not hopes" | :140-141 | REAL-UNPROVEN | tests/{wav_roundtrip,mp4_remux,job_lifecycle}.rs exist with the right shapes (mp4_remux's remux→re-demux→packet-identical asserts scanned); never executed in baseline. |
| "No filters … the stage is modeled, not faked" | :124-125 | REAL | graph.rs models FilterStage; validation refusal is graph.rs/runner.rs territory (A03-graph is wave-10 assigned to another agent — cross-noted). |
| playback-record / ui-dialog-convert `not_started` | docs/feature-matrix.toml:1630+/2183+ | REAL | Zero dependents matches the honest rows. |

## 7. Dead / duplicated / unreachable code
The entire crate is unreachable from shipped (header) — but not *dead* in the stale sense: it is a Phase-7 deliverable whose consumer has not landed, and the matrix says so. No dead items within the file.

## 8. Dependencies & flags
Declared deps match the doc's layer posture (read of Cargo.toml at session start): codec-api, demux, media-types, plugin-api (prod); codecs, input, testkit (dev-only). No feature gates. Workspace lints inherited (`[lints] workspace = true`).

## 9. Open questions for the next agent
- A04/A05 (runner parts): the doc's bounded-queue/backpressure claims live in your ranges — the lib.rs ledger above leans on them.
- Orchestrator: assign transcode's four test files; REQUEST-ORCH-RUN `cargo test -p transcode --no-fail-fast --release --locked` (same ask as my mp4mux audit).
