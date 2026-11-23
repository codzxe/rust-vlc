# AUDIT: crates/playback-engine/tests/scenarios_b/main.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 72 / 2 (alloc_state accessor + module root) |
| File verdict | REAL (module root for the 16 doc-10 §6.4 scenario modules; process-global counting allocator wired; one conflation in the stack-vs-fakes summary sentence) |
| Reachable from shipped entry point? | Test-harness root (CI-only). The stack beneath (Engine, AviDemuxer, MjpegDecoder/PcmDecoder via host.rs:1496/705) is the shipped one. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n`/`rg` verification (exit 0); baseline cargo-test-workspace.txt:4386-4416 — scenarios_b binary: 23 passed / 0 failed in 5.06 s |

## 1. One-paragraph verdict
A clean harness root: it declares exactly the 16 scenario modules matching doc 10 §6.4's rows (verified name-by-name against the module table and the baseline's 23 passing tests), installs the process-global `CountingGlobalAlloc` so the D-2 bounded-memory scenarios are exact in-process, exposes `alloc_state()` for them, and shares `tests/common` via `#[path]`. The single defect is a conflation in its own summary: it states "the AVI demuxer and the MJPEG/PCM decoders are the real production stack" in the same breath as "the named fakes are ... the scripted demux stage" — both are true, but of DIFFERENT scenarios (scenario_05 opens the real AVI parent; scenarios 13/14 drive the scripted `TimelineDemuxer`), and the sentence as written claims both simultaneously.

## 2. What actually works (verified)
— All 16 modules exist and run: baseline :4386-4416 lists every `scenario_01`..`scenario_16` test passing (23 tests total, 5.06 s).
— The module table (lines 7-24) matches doc 10 §6.4's rows including pass criteria columns (spot-checked rows 1-5 and 8 against docs/10-testing-ci-quality.md:682-688).
— The serializer discipline claim ("Every test takes the whole-binary serializer first") is honored in the scenario files I audited (scenario_05_stop.rs:26 `serialize_tests()`).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/main.rs:30-33
Code:
```
//! entry lives in `lumen-cli`, which this crate cannot dev-depend on
//! without the cycle boundaries.toml rejects). The engine, the state
//! machine, the AVI demuxer and the MJPEG/PCM decoders are the real
//! production stack; the named fakes are the scripted access source,
//! the scripted demux stage, and the observing/vanishing sink.
```
What is wrong: the sentence reads as one simultaneous configuration — real AVI demuxer AND scripted demux stage. In reality the scenarios split: scenario_05 (and the other PARENT-based ones) open the real `AviDemuxer` over `input::FileAccess` (host.rs:1475-1496), while scenarios 13/14 construct `TimelineDemuxer::audio_timeline` (host.rs:1552) — a synthesized timeline demuxer. A reader citing this header as "scenarios run the real demuxer" would be wrong for a quarter of the suite; a reader citing it as "scripted" would be wrong for the rest. What correct looks like: name which scenarios use which stage. Confidence: HIGH — Verification: read of main.rs + host.rs:1475-1552. (The ARCH-5/ARCH-8 honesty note about the in-test dispatch instead of `lumen-cli`'s is accurate and properly reasoned.)

## 4. Per-function table (file < 100 LOC; brief)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| alloc_state | 48-50 | REAL | — | allocator snapshot access for the D-2 scenarios |
| module root (16 mods + #[path] common) | 52-72 | REAL | S4 | all 16 §6.4 rows wired; F-001 conflation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (root — no tests of its own) | n/a | the 23 tests across its modules all pass (baseline :4386-4416); scenario-level honesty audited per module (see scenario_05_stop.rs audit) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The sixteen scenario modules each own one §6.4 row" | main.rs:5-6 | REAL | 16 modules declared; table matches doc 10 §6.4 |
| "The engine, the state machine, the AVI demuxer and the MJPEG/PCM decoders are the real production stack; the named fakes are ... the scripted demux stage" | main.rs:31-33 | SHALLOW | true per-scenario, not simultaneously (F-001) |
| "allocator live-bytes deltas stand in for RSS, which is not measurable in-proc" | main.rs:37-40 | REAL | CountingGlobalAlloc installed; reopen/long-media/exit scenarios use it |

## 7. Dead / duplicated / unreachable code
None. `probe` and `common` modules are used by the scenarios.

## 8. Dependencies & flags
testkit (allocator), commands; `#[path = "../common/mod.rs"]` shares the crate's fake vocabulary. No feature gates; runs in the default CI lane (baseline :4386).

## 9. Open questions for the next agent
- The other 15 scenario modules are outside my assignment — the module table's pass-criteria column vs each module's actual assertions (the scenario_05 F-001 reinterpretation pattern) should be spot-checked for rows 11-16 especially (deadlines, recovery, bounded memory).
