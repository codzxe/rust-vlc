# AUDIT: crates/codecs/tests/fuzz_loop_stable.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 551 / 15 helpers + 7 tests (5 default, 2 #[ignore] long) |
| File verdict | REAL (but its green does NOT establish the survival contract it documents — see F-001) |
| Reachable from shipped entry point? | Test binary: NO. The harnesses it drives (`codecs::audio::pcm::fuzz::fuzz_codec_pcm`, `codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg`) exercise production decoder code reachable from lumen CLI (pipeline_host.rs:242/253). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/fuzz_loop_stable-0e970d0b80a3fbfa --nocapture` → 5 passed / 2 ignored (1.35s; reproduced the pinned accounting EXACTLY: `pcm: 2000 inputs, ... 1361 distinct lengths`; `mjpeg: 1000 inputs, ... 599 deep, 726 distinct lengths` — byte-identical to the L414-420 recorded values). `--ignored --nocapture` (the long lane) → 2 passed (mjpeg 20000 inputs / 11947 deep / 30.4s; pcm 100000 inputs / 49.6s). |

## 1. One-paragraph verdict
A well-built deterministic seed-loop fuzzer: fixed seeds, six input flavors, cross-contaminated real-fixture seeds (JPEG bytes to the PCM harness and vice versa), anti-vacuity accounting (deep-entry floors, distinct-length floors), determinism pins, and honest #[ignore] gating for the long lane. I ran both the default and the long lanes — everything passes and the recorded accounting values reproduce exactly. The problem is evidentiary: the module documents its contract as the harnesses' own ("survival — no panic, no hang, no unbounded allocation"), and BOTH lanes are green while I hold a reproducible panic in the very harness this file drives (decoder.rs:976, hostile DC size 31 — see the decoder.rs audit; found by my own 6,000-iteration structured mutation run). The campaign's mutation grammar (bit flips, truncation, random tail, aligned u32 smash) almost never writes the specific byte values needed to arm that bug, so 20,000 iterations prove nothing about that corruption class. The test is real; the survival claim it is cited for is not established by it.

## 2. What actually works (verified)
- Default lane 5/5 pass; long lane (ignored) 2/2 pass — both executed by me, timings above.
- Determinism & decorrelation: `seeded_generation_is_deterministic_and_decorrelated` and `loop_accounting_is_deterministic` pass; the pinned default-run accounting (599 deep / 726 lengths) reproduced exactly on this machine.
- Seeds are real fixture bytes (avi_mjpeg_pcm.avi / mp4_mjpeg_pcm.mp4 via the common walkers) with cross-codec contamination as documented.
- The sub-magic regression pin (cap 1) survives.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/fuzz_loop_stable.rs:10-13 (contract statement) and 434-437 (the loop that allegedly establishes it)
Code:
```rust
//! never rewritten — on the pinned stable toolchain, over N seeded
//! pseudo-random inputs per target. The contract is the harnesses'
//! own: survival (no panic, no hang, no unbounded allocation); damage
//! surfaces as typed errors or concealed output inside the harness.
```
What is wrong: the green run does not establish this contract. Counterexample (verified in the decoder.rs audit): `fuzz_codec_mjpeg` PANICS at `decoder.rs:976` ("attempt to subtract with overflow") on a hand-built JPEG with DC HUFFVAL symbol 31, and my 7,500-iteration independent mutation loop hit the same site twice from random structured mutations of a valid JPEG — yet this file's default 1,000-input lane AND its 20,000-input long lane both pass, because the grammar (bit-flips, truncation, head+tail splice, aligned u32 smash) rarely writes the exact arming values. Anyone citing this suite as fuzz evidence for MJPEG robustness is citing a test that cannot see a demonstrated, trivially-reachable panic class. What correct looks like: add byte-assignment mutations with structure-aware value pools (0x00/0x01/0x0f/0x1f/0xff at DHT/entropy offsets), raise the default N, or run the long lane in CI; and fix decoder.rs:976 (size > 15 ⇒ `Stop::InvalidCode`). Confidence: HIGH — Verification: ran (both lanes green + 3 panic reproductions through the same harness).
F-002 [S4]
Where: crates/codecs/tests/fuzz_loop_stable.rs:261-273
Code:
```rust
fn assert_honest(stats: &Stats, n: usize, min_deep: usize, min_lengths: usize, label: &str) {
    assert_eq!(stats.inputs, n, "{label}: loop must run every iteration");
```
What is wrong: `assert_honest` measures generator activity (inputs counted, deep fraction, length spread), not harness outcome diversity — the harness's return is discarded (`harness(&input);` at L254 — "The pass is the return"). A decoder that returned instantly on every input would still pass every assertion here. The anti-vacuity accounting is honest about the generator only; nothing asserts any typed-error/concealment outcome ever occurred. Confidence: HIGH — Verification: read + both runs' stats show the accounting is all that is checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rng::new / next_u64 / below / byte / flip_bit | 66-102 | REAL | — | xorshift64* + splitmix64 decorrelation; zero-state guarded; determinism pinned by test |
| Magic / Spec | 104-119 | REAL | — | flavor enum |
| gen_input | 123-220 | REAL | S3 | six flavors; grammar gap is the root of F-001 |
| Stats / run_loop | 222-259 | REAL | S4 | accounting only; F-002 |
| assert_honest | 261-273 | REAL | S4 | floors on generator stats |
| avi_parent / mp4_parent | 280-293 | REAL | — | committed class-A fixtures |
| pcm_seeds / mjpeg_seeds | 298-345 | REAL | — | cross-contaminated seeds as documented |
| PCM_CAP / MJPEG_CAP / seeds consts | 351-362 | REAL | — | documented [PROPOSAL] caps |
| pcm_deep / mjpeg_deep | 366-373 | REAL | — | deep = SOI prefix for mjpeg; always-true for pcm (documented) |
| pcm_loop / mjpeg_loop | 375-421 | REAL | — | pinned accounting floors (W2-17); reproduced exactly |
| pcm_seed_loop_survives (test) | 423-426 | REAL | — | 2,000 inputs, passes |
| pcm_seed_loop_long (test) | 428-432 | REAL | — | #[ignore] 100,000; run manually: passes |
| mjpeg_seed_loop_survives (test) | 434-437 | REAL | S3 | passes while panic exists (F-001) |
| mjpeg_seed_loop_long (test) | 439-443 | REAL | S3 | 20,000 inputs, run manually: passes — same gap |
| sub_magic_lengths_are_survivable_inputs (test) | 445-464 | REAL | — | generator regression pin |
| seeded_generation_is_deterministic_and_decorrelated (test) | 466-495 | REAL | — | replay + decorrelation + zero-seed |
| loop_accounting_is_deterministic (test) | 503-550 | REAL | — | full-loop Stats determinism |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pcm/mjpeg_seed_loop_survives | ONLY for panic classes the grammar can arm | demonstrated: a real, reachable panic (decoder.rs:976) survives both lanes |
| sub_magic_lengths_are_survivable_inputs | YES | pins a specific generator off-by-one |
| seeded_generation_is_deterministic_and_decorrelated | YES | byte-equality on replay; assert_ne on nearby seed |
| loop_accounting_is_deterministic | YES | Stats tuple equality |
The honesty scaffolding (floors, determinism) is genuine; the survival evidence is weaker than the module doc implies.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The contract is the harnesses' own: survival (no panic...)" | L11-13 | SHALLOW (not established by this suite) | both lanes green while panic reproduced through the same harness (decoder.rs audit F-001) |
| "599/1000 = 59.9% deep, 726 distinct lengths" (recorded accounting) | L414-418 | REAL | my run: `mjpeg: 1000 inputs, ... 599 deep, 726 distinct lengths` |
| "The real seeds are externally authored bytes ... cross-contaminate codecs" | L29-33 | REAL | seed construction read; fixtures verified in common/mod.rs audit |
| harness bodies "reused verbatim, never rewritten" | L7-10 | REAL | direct imports of fuzz_codec_pcm / fuzz_codec_mjpeg |

## 7. Dead / duplicated / unreachable code
None. `Magic::PeriodicAudio` sub-magic branch (len < 2) is exercised by the cap-1 regression test.

## 8. Dependencies & flags
Workspace-only (codecs, common test module); no external RNG; two #[ignore] long tests documented with the run command. Caps documented as [PROPOSAL]. Runs in the default per-package lane (baseline 4744 passed includes the 5 default tests).

## 9. Open questions for the next agent
- Is the long lane (`--ignored`) wired into any CI workflow? Nothing in the baseline suggests it; if not, the deepest fuzz evidence the repo has for MJPEG survival is 1,000 seeded inputs.
- The demux crate has its own fuzz_loop_stable.rs (two binaries) — out of my scope.
