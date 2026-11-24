# CLAIM AUDIT: docs/03-engine-plan.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 12 (>300 LOC doc, depth mode) |
| REAL 5 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 2 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 1 (acceptance criterion 1 vs baseline failing evidence tests) |

## 1. Blunt verdict
The best-kind planning doc: nearly every number and name is checkable against the tree, and
the checks land — probe window, queue caps, epoch file, crate registry, first-vertical
choice. Two claims age badly: §8's `SessionHarness`/sinks remain name-shapes the testkit
doc-comment openly defers, and §7's own acceptance criterion ("a second run is identical")
is violated TODAY by the shipped tree's failing evidence tests. Note: this ledger CORRECTS
the dead agent's blanket "testkit drivers exist NOWHERE" — `ScriptSource` is real and used.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "bounded per-medium queues" caps: audio 200 / video 300 / subtitle 128 | §4.4(b) | REAL | packet-queue/src/limits.rs:38-47, doc-comments cite doc 03 §4.4(b); `AUDIO_DEFAULT_PACKETS = cap(200)`, `VIDEO_DEFAULT_PACKETS = cap(300)` |
| 2 | probe window 64 KiB, hard cap 1 MiB | §4.2 | REAL | media-probe/src/config.rs:15-22 `DEFAULT_MAX_LOOKAHEAD: usize = 64 * 1024` |
| 3 | "Recommendation [PROPOSAL]: candidate 2, AVI + MJPEG + PCM" | :974 | REAL | baseline rc-smoke headless-decode (AVI/MJPEG/PCM fixture) PASS; engine-info decoders: mjpeg, pcm_s16le (audit/runs/baseline/) |
| 4 | "a second run is identical" (acceptance criterion 1) | :995 | FALSE-as-of-baseline | audit/runs/baseline/full_gate.txt:36-39: committed_transcripts_match_fresh_runs FAILED, final_frame_is_current FAILED (tests/integration/first-vertical/tests/evidence.rs:47/:162) |
| 5 | `lumen play fixture.avi` … "exit code 0" | :994 | REAL | baseline mp4-smoke: `lumen open` H.264+AAC walks to Stopped, exit 0 |
| 6 | generation/epoch semantics §5 | :842 | REAL | crates/playback-engine/src/epoch.rs exists; session.rs/state.rs reference generation/epoch (rg) |
| 7 | media-types foundations (ids, Track, MediaKind, 7-kind error taxonomy) | §3 | REAL | crates/media-types/src/lib.rs:14 doc-comment names stream/track/generation ids, Track, MediaKind; crate present |
| 8 | crate registry (input, media-probe, codec-api, audio/video-engine, renderer-wgpu, playback-engine, media-clock) | §2 | REAL | all present in crates/ listing (31 crates) |
| 9 | renderer isolated: "depends only on media-types frame handles" | §2.3(3) | REAL-UNPROVEN | renderer-wgpu present; boundary check cited to CI, not re-run here |
| 10 | "`ScriptSource`: `ByteStream` from a byte buffer plus a fault script" | :1024 | REAL | crates/input/src/contract.rs:13 "in-memory byte source with scriptable faults"; used across buffered.rs tests + playback-engine scenarios_b host |
| 11 | §8 testkit drivers: `SessionHarness`, sinks, leak instruments | §8 | PAPER-ONLY | SessionHarness repo-wide = 2 hits, both name-shape mentions in crates/testkit/src/lib.rs:10-13 ("land with the Phase-2 engine traits they exercise") + Cargo.toml description |
| 12 | §4.13 deterministic seeded shuffle | §4.13 | PAPER-ONLY | set-shuffle is a flag; demo-transport sequential advancement (dead agent's finding, re-checked: no seeded-shuffle impl found in playback-engine/src listing) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Acceptance criterion 1: "a second run is identical" | baseline full_gate.txt:36-39 two FAILED first-vertical evidence tests | Baseline: the shipped slice fails its own plan's determinism criterion (regression vs the plan's acceptance record) |

## 4. Findings
F-001 [S2] Where: docs/03-engine-plan.md:995.
Code (verbatim): "   audio, and both stop at `Ended`; exit code 0; a second run is identical."
What is wrong: the first-vertical slice's determinism acceptance criterion is currently
violated — its own evidence tests fail at baseline. Why it matters: the plan's acceptance
gate is red in shipped state; any doc claiming the slice "meets acceptance criteria" is
false. What correct looks like: re-pin transcripts or fix the nondeterminism, then green.
Confidence: HIGH — Verification: baseline cited (not re-run: cargo budget).
F-002 [S4] §8's driver set is half-real: `ScriptSource` landed (input crate), but
`SessionHarness`/sinks/leak-instruments remain testkit name-shapes per its own doc-comment.
The plan reads as one deliverable; the tree has half of it. Confidence: HIGH —
Verification: rg counts (SessionHarness = 2 doc-mentions repo-wide).
F-003 [S4] §4.13 seeded shuffle unimplemented (flag-only) — dead agent's finding
re-confirmed by src listing. Confidence: MEDIUM (code-level re-scan, not full read).

## 5. Prior-audit reconciliation
Dead W19-A09's audit (old tree audit/docs/03-engine-plan.md.audit.md) said "§8 testkit
drivers exist NOWHERE (S3)". That is overbroad: ScriptSource exists and is load-bearing in
input tests. The correct verdict is split (REAL for ScriptSource, PAPER-ONLY for
SessionHarness/sinks) — recorded above. Its other claims (probe/queue defaults, epoch,
first vertical) re-verified identically by me.
