# AUDIT: crates/demux/src/ts/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-23 |
| Lines / functions | 86 / 1 harness + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test/fiering surface by design (not player code): consumed by the cargo-fuzz shim `fuzz/fuzz_targets/fuzz_demux_ts.rs` (verified: exists, calls `demux::ts::fuzz::fuzz_demux_ts`), the in-crate tests, and tests/ts_fuzz_survival.rs (baseline 3/3 ok). The TS demuxer itself IS CLI-wired (`"mpegts" => ts::open` arm). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b ts::fuzz --test-threads=2` → 10 passed / 0 failed (includes the 2 harness tests), exit 0; `rg` wiring census; `od -c` quote check |

## 1. One-paragraph verdict
A small, honest, correctly-bounded fuzz harness that does exactly what its contract states: construct → `start` → drain with a hard 200 000-event budget, swallowing typed errors by documented design (the outcome pinning lives in `ts_corpus`/`ts_pes_corpus`/`ts_fuzz_survival`, all green in the orchestrator baseline I cite). No fakeness, no dead code, no unbounded work: the budget check precedes the increment (:49-52), every match arm terminates or continues, and the only allocation is the one bounded input copy (`data.to_vec()`, fuzz-input-sized). Unlike the PS sibling harness it performs no seek probes — and that is correct, not a gap: `TsDemuxer::seek` is a typed refusal and `seekability()` returns `SeekSupport::None` (ts/mod.rs:1154-1167), so there is no seek surface to fuzz. The "honest wiring" claims all verified against reality.

## 2. What actually works (verified)
- `./target/debug/deps/demux-5e0c7194d741938b ts::fuzz --test-threads=2` → `test result: ok. 10 passed; 0 failed` — includes `ts::fuzz::tests::empty_and_sync_inputs_survive` and `hostile_wide_inputs_survive` (all-0x47 max-false-resync, all-0xFF, all-0x00, 96 ramp inputs).
- Baseline audit/runs/baseline/cargo-test-workspace.txt:2408-2412: `corpus_head_mutations_survive ... ok`, `truncation_sweep_over_packet_boundaries ... ok`, `mutations_never_panic_and_stay_deterministic ... ok` (tests/ts_fuzz_survival.rs, 3/3).
- Wiring census: `fuzz/fuzz_targets/fuzz_demux_ts.rs` exists and its body is exactly `demux::ts::fuzz::fuzz_demux_ts(data);`; registry row `fuzz_demux_ts` present at fuzz/README.md:62; "campaign not run in CI" status honestly documented at fuzz/README.md:36-40 ("nothing in the local gate or CI builds this directory *in campaign mode*").

## 3. Findings
None. What was checked and how (per quality bar for zero-findings files):
- Fakeness: no todo!/unimplemented!/unreachable! (rg over the file); no ignored parameters — `_target` appears only in ts/mod.rs's own seek, not here; no constant returns; body is a real drain loop.
- Correctness: budget check precedes increment so next_event is invoked at most 200 000 times; `Err(_) => break` swallow is the documented contract (:8-11); no arithmetic on parsed input in this file; no unwrap/expect/index/panic sites; `data.to_vec()` bounded by fuzz input size.
- Bounds claims (:22-24 "Memory: the demuxer's own ceilings … this harness adds none") — cross-checked by read: the harness adds only the input copy; demuxer-side caps are ts/mod.rs's concern (out of my assignment, covered by the survival corpus).
- Test honesty: both tests are absence-of-panic smoke sweeps — they would fail only on a panic/hang, which IS the harness contract; deeper mutation coverage lives in tests/ts_fuzz_survival.rs (baseline-cited above), so the split is honest, not tautological.
- Docs vs reality: every concrete claim in :1-24 checked — registry name (README:62 ✓), shim path (:15-18 ✓), nightly-not-run status (README:36-40 ✓), "the 188-byte packet walk, PSI assembly … ride that drain" — consistent with the demuxer's start/next_event structure (read of ts/mod.rs walk loop :1150-1152 `self.walk += PACKET_BYTES`); actual libFuzzer campaign execution is UNVERIFIED (by design not run in CI — the README admits it; no fabrication).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_demux_ts | 37-60 | REAL | — | budgeted drain; errors swallowed by contract; used by libFuzzer shim + survival suites |
| tests::empty_and_sync_inputs_survive | 66-71 | REAL | — | smoke: empty/1-byte/188-byte sync inputs |
| tests::hostile_wide_inputs_survive | 73-85 | REAL | — | smoke: degenerate byte patterns + 96 ramps ≤96 B |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_sync_inputs_survive | Only on panic/hang/oom | that is the contract; outcome pinning delegated to ts_corpus/ts_pes_corpus (baseline green) |
| hostile_wide_inputs_survive | Only on panic/hang/oom | same; complements ts_fuzz_survival's seeded mutations |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cargo-fuzz target is fuzz/fuzz_targets/fuzz_demux_ts.rs" | fuzz.rs:15-16 | REAL | file read; body calls this harness |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's TS corpora" | fuzz.rs:17-18 | REAL | my run (2/2) + baseline ts_fuzz_survival 3/3, ts_corpus 2/2, ts_pes_corpus 4/4 |
| "Event budget: hard cap below; positions are monotone" | fuzz.rs:22 | REAL-UNPROVEN | cap verified by read (:49-52); monotonicity is the demuxer's walk invariant (ts/mod.rs:1150) — not independently executed beyond the survival corpora |
| campaign running under libFuzzer | — (nowhere claimed) | N/A | README:36-40 explicitly says campaign mode does not run in CI; nothing overclaims |

## 7. Dead / duplicated / unreachable code
None. (The harness is intentionally not reachable from the shipped binary — fuzz surface only; the file documents this.)

## 8. Dependencies & flags
Depends on `super::TsDemuxer` and `super::test_support::mem_stream` (ts/test_support.rs:20). No feature gates, no unsafe, no external crates.

## 9. Open questions for the next agent
- ts/mod.rs (not my file): the harness trusts the demuxer's own ceilings ("section assembler caps, damage-log cap" :23-24) — the next agent auditing ts/mod.rs/psi.rs should confirm those caps exist and bound, since this harness explicitly adds none.
- If the Lead's nightly campaign decision lands (fuzz/README.md UNTIL clause), re-run this target under ASan; the current absence-of-panic evidence is deterministic-seed only.
