# AUDIT: crates/demux/tests/fuzz_loop_stable.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 391 / 5 tests + Rng, Spec, gen_input, Stats, run_loop, avi_deep, avi_seeds, avi_loop |
| File verdict | REAL (deterministic seeded fuzz loop with honest anti-vacuity accounting; executed by me and in the baseline) |
| Reachable from shipped entry point? | Integration test; the harness under test (`demux::avi::fuzz::fuzz_demux_avi`) exercises the AVI demuxer, which IS shipped (containers/mod.rs "avi" arm) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `timeout 300 ./target/debug/deps/fuzz_loop_stable-cbc96175dc98d3c2 --test-threads=1 --nocapture` -> `[fuzz_loop_stable] avi: 1000 inputs, 195882563 bytes, 827 deep, 743 distinct lengths, 2.365431594s`, 4 passed / 1 ignored; baseline citation audit/runs/baseline/cargo-test-workspace.txt:2271-2281 (ran in 1.74s there too) |

## 1. One-paragraph verdict
This is what a deterministic stable-toolchain fuzz loop should look like: a seeded xorshift64* generator with six input flavors (random, RIFF-magic lies, bit-flips, truncations, head+garbage-tail, u32-smash), a survival contract over the real `fuzz_demux_avi` harness, and — the part most fuzz suites fake — hard anti-vacuity accounting: the deep-entry fraction (≥ 80%) and the distinct-length spread (≥ 700) are pinned near the *recorded* run values so generator drift fails loudly. I ran it: 1000 inputs, 195,882,563 bytes, 827 deep, 743 distinct lengths — byte-identical to the values recorded in the code comment (lines 283-287: "827/1000 = 82.7% ... 743 distinct lengths"), proving cross-environment determinism. The baseline ran it too (1.74 s). The only debts are cosmetic: the 20,000-input long variant is `#[ignore]`d (never runs in the default lane), and the "deep" proxy only verifies the RIFF/AVI magic prefix, which the code itself admits.

## 2. What actually works (verified)
- Full suite executed: `./target/debug/deps/fuzz_loop_stable-cbc96175dc98d3c2 --test-threads=1 --nocapture` -> 4 passed, 1 ignored, with the loop line quoted above. The in-code pinned values (827 deep / 743 lengths) match my fresh run exactly.
- Baseline: `test avi_seed_loop_survives ... ok` inside `test result: ok. 4 passed; 0 failed; 1 ignored ... finished in 1.74s` (cargo-test-workspace.txt:2277-2281) — this test does NOT skip, unlike the corpus differentials.
- The real-seed parent fixture loads and is used (the loop ran; `avi_seeds()` would have panicked otherwise).
- The claimed cargo-fuzz scaffolding exists: fuzz/fuzz_targets/fuzz_demux_avi.rs, fuzz/Cargo.toml, fuzz/README.md (verified by ls/rg) — the doc's "wiring status" story checks out.

## 3. Findings

F-001 [S4]
Where: crates/demux/tests/fuzz_loop_stable.rs:305-309
Code:
```rust
#[test]
#ust_use]  // (display artifact; real bytes `#[ignore = ...]` verified via od)
fn avi_seed_loop_long() {
    avi_loop(20_000);
}
```
(The raw source at 306 is `#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]`; the display pipeline mangled the bracket — verified with od.)
What is wrong: the 20,000-input variant never runs in the default test lane or the baseline (one of the repo's 51 ignored). The 1000-input variant runs and provides the same contract at 1/20th the coverage, so this is a depth limitation, not a fake — but the only path that exercises 20× the mutation space is a manual invocation nobody is required to make. Confidence: HIGH — Verification: ran (ignored in my run and in baseline line 2274).
Confidence: HIGH — Verification: ran

F-002 [S4]
Where: crates/demux/tests/fuzz_loop_stable.rs:228-235
Code:
```rust
fn avi_deep(data: &[u8]) -> bool {
    data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"AVI "
}
```
What is wrong: the anti-vacuity metric counts inputs that merely *carry the magic* — a "deep" input is not a demuxer deep walk. The code says so honestly ("A landed `start()` is not guaranteed past this prefix"), so this is a naming/precision debt, not a deception: the loop could in principle pass its 80% floor while every magic-prefixed input fails `start()` at byte 12. A stronger proxy would count harness-internal engagements (the AVI fuzz harness would need to expose that). Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rng (new, next_u64, below, byte, flip_bit) | 66-104 | REAL | — | splitmix64-decorrelated xorshift64*; modulo bias negligible; zero-state guarded (75) |
| Spec / gen_input | 108-193 | REAL | — | six flavors traced; cap enforced at 191; sub-magic branch boundary-pinned by test 3 |
| Stats / run_loop | 197-226 | REAL | — | inputs/bytes/deep/lengths accounting |
| avi_deep | 233-235 | REAL | S4 | F-002 magic-prefix proxy |
| avi_seeds | 244-253 | REAL | — | three depths of the real parent; non-empty assert; would panic loudly on a shrunk fixture (acceptable) |
| avi_loop | 264-298 | REAL | — | anti-vacuity floors: deep ≥ 4n/5, lengths ≥ 700 — my run: 827, 743 |
| avi_seed_loop_survives | 300-303 | REAL | — | 1000 inputs, ran green (mine + baseline) |
| avi_seed_loop_long | 305-309 | REAL | S4 | F-001 ignored variant |
| sub_magic_lengths_are_survivable_inputs | 311-329 | REAL | — | cap-11 boundary regression |
| seeded_generation_is_deterministic_and_decorrelated | 331-357 | REAL | — | replay + decorrelation + zero-seed |
| loop_accounting_is_deterministic | 364-390 | REAL | — | full-loop Stats pin |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| avi_seed_loop_survives | YES (panic/hang class) | any panic in the AVI demuxer over 1000 seeded inputs fails it; vacuity guarded by the 827/743 floors (a degenerate generator fails loudly) |
| avi_seed_loop_long | same, deeper — but NEVER RUNS by default | F-001 |
| sub_magic_lengths... | YES | boundary + survival |
| seeded_generation... | YES | byte-identical replay, seed decorrelation |
| loop_accounting... | YES | Stats equality |
Verified: my run green, baseline green, deterministic values reproduced exactly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runs the SAME harness loop body — fuzz_demux_avi, reused verbatim, never rewritten" | fuzz_loop_stable.rs:7-10 | REAL | import at line 60; no local reimplementation of the harness |
| "The contract is the harness's own: survival (no panic, no hang, no unbounded allocation)" | fuzz_loop_stable.rs:10-13 | REAL | no outcome asserts beyond accounting — matches the AVI fuzz harness contract |
| "Fixed seed ... any survival regression is exactly reproducible" | fuzz_loop_stable.rs:52-55 | REAL | my run reproduced the recorded 827/743 exactly |
| "the iteration count N is a fixed constant ... Total work bounded by N × (cap + harness budget)" | fuzz_loop_stable.rs:47-48 | REAL | 1000 × ≤512 KiB = ≤ 512 MB worst case; my run: 196 MB in 2.37 s |
| "near-verbatim twin of the codecs crate's tests/fuzz_loop_stable.rs generator" | fuzz_loop_stable.rs:37-40 | REAL | baseline shows the codecs twin (7 tests, mjpeg/pcm, cargo-test-workspace.txt:1102-1110) |

## 7. Dead / duplicated / unreachable code
The generator duplication with the codecs crate is deliberate and documented (integration binaries cannot share code without a new dependency). No dead code in this file.

## 8. Dependencies & flags
`demux::avi::fuzz::fuzz_demux_avi`, `testkit::Must` + `testkit::fixture_bytes` (the committed class-A AVI parent). No features. The suite is part of `cargo test -p demux` and RUNS in the baseline lane (not ignored, unlike corpus differentials).

## 9. Open questions for the next agent
- The AVI fuzz harness itself (crates/demux/src/avi/fuzz.rs) is another agent's file — my ogg findings (recover_sync/scan CPU amplification) suggest checking whether the AVI harness's recovery paths have the same bytes-bounded/CPU-unbounded shape.
- Consider exposing an engagement counter from the harness so "deep" can mean "start() landed" rather than "magic present" (F-002).
