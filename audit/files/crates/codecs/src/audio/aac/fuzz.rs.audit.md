# AUDIT: crates/codecs/src/audio/aac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 163 / 2 (+2 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz/test infrastructure) — but wired for real: `fuzz/fuzz_targets/fuzz_codec_aac.rs` calls `codecs::audio::aac::fuzz::fuzz_aac_stream(data)`; `crates/codecs/tests/aac_fuzz_survival.rs:18` drives it with seeded mutations. Module is `pub` and unconditional in the codecs crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::aac::fuzz` -> 2 passed (exit 0); `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c` -> 2 passed (exit 0); rg/sed verification (exit 0) |

## 1. One-paragraph verdict
A genuine four-pass hostile-input harness: raw ADTS header parse, resync walk, full `decode_stream` (engine included), and the codec-api contract path over up to 64 walked frames — with typed errors swallowed per the documented absence-of-panic contract. The in-crate smoke tests are shallow but honest about it; the real evidence is the survival suite (seeded LCG mutations of a committed `fuzz_base_lc.aac` fixture, ran: 2 passed). The defects: the module's headline "wiring status (honest)" doc is now FALSE — the fuzz/Cargo.toml scaffolding and libfuzzer-sys 0.4.13 dep it says are owed have landed; `let _ = first;` is a dead statement; and `FuzzOutcome.contract_frames` is asserted by no test anywhere (mp2's harness asserts its analogue only as `<= 64`), so pass 4 has zero positive-coverage evidence.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::aac::fuzz` -> `test result: ok. 2 passed` (empty/garbage/0xFF-fill/header-only inputs survive).
- `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c` -> `test result: ok. 2 passed; 0 failed` — seeded deterministic LCG mutants of the committed fixture, every outcome classified Decoded/Typed.
- Wiring verified by read: `fuzz/Cargo.toml` (own workspace root, libfuzzer-sys = "0.4.13", line 43), `fuzz/fuzz_targets/fuzz_codec_aac.rs` `fuzz_target!(|data: &[u8]| { codecs::audio::aac::fuzz::fuzz_aac_stream(data); });`.
- Bounds claim cross-checked (read): `SAMPLES_PER_FRAME = 1024`, channel gate ≤ 2 (decoder.rs:36,73), `SAMPLE_SLOTS = 4` (decoder.rs:399), frames tile the input (`frame.offset + frame.header.frame_len` bounded by walk) — the MP §12 bound statement holds.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/aac/fuzz.rs:19-31
Code:
```
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_aac.rs` exactly as doc 04 §3.1
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land the
//! harness logic is proven by the in-crate deterministic smoke tests
```
What is wrong: both "Lead-owned" blockers HAVE landed: `fuzz/Cargo.toml` exists (own-workspace scaffold, `libfuzzer-sys = "0.4.13"`), and `fuzz/fuzz_targets/fuzz_codec_aac.rs` exists and calls this harness. The doc's present-tense "until they land" is false; only the nightly-toolchain/CI wiring remains (recorded honestly in fuzz/Cargo.toml's header). Why it matters: the section titled "honest" now misstates the repo state — a reader would believe the target file does not exist. What correct looks like: update the status paragraph to "scaffold landed, campaign build still needs nightly wiring". Confidence: HIGH — Verification: read (both files exist; cat quoted).
F-002 [S4]
Where: crates/codecs/src/audio/aac/fuzz.rs:92 + 131
Code:
```
    if let Some(first) = frames.first() {
```
```
        let _ = first;
    }
```
What is wrong: `first` is bound solely to enter the non-empty case and then explicitly discarded — the binding is dead, the pattern should be `if !frames.is_empty()`. Why it matters: pure dead code in a security-facing harness; also misleading (suggests `first` matters to the contract pass). What correct looks like: `if frames.is_empty() { return outcome; }`. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/codecs/src/audio/aac/fuzz.rs:69-71 + tests 140-162
Code:
```
    /// Frames the contract path emitted.
    pub contract_frames: usize,
```
What is wrong: no test anywhere asserts `contract_frames > 0`. Both in-crate tests expect 0 (empty/garbage/header-only inputs), and the survival suite only reads `.decoded`. If the contract pass silently regressed (configure refusal, first-frame error → `break` at line 124-126), every existing test still passes. (The decoder's own contract tests in decoder.rs cover the path separately, so the pass is not dead — but the harness's observability for it is.) Why it matters: one of the four advertised passes has zero positive assertion; the field is dead telemetry. What correct looks like: one smoke test over a valid walked fixture asserting `contract_frames > 0` (the mp2 harness at least asserts the `<= 64` bound). Confidence: HIGH — Verification: read + rg (`contract_frames` asserted nowhere).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink::accept | 55-60 | REAL | — | saturating count, Ok sink |
| fuzz_aac_stream | 76-134 | REAL | — | 4 passes, bounded (.take(64)), get()-sliced frames; F-002/F-003 |
| 2 tests | 140-162 | SHALLOW-but-honest | — | absence-of-panic only, documented as such |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_garbage_inputs_never_panic | PARTIAL | catches panics/hangs on 3 hostile shapes; asserts nothing about pass 3/4 behavior |
| a_header_only_buffer_is_survived | PARTIAL | one 7-byte shape |
| mutated_fixtures_never_panic_and_settle_typed (survival, ran) | YES for panic/hang | every LCG mutant classified; a panic fails the run — the actual fuzz evidence |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 4 passes: header parse, walk, stream pipeline, contract path | fuzz.rs:4-12 | REAL | code 82-131 |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:14-17 | REAL | errors swallowed; survival suite |
| "The cargo-fuzz target file is proposed ... until they land" | fuzz.rs:19-31 | FALSE (stale) | F-001: scaffold + dep landed |
| bounds: frames ≤ input_len/7, 4 pooled slots, fixed passes | fuzz.rs:33-40 | REAL | decoder.rs constants read |

## 7. Dead / duplicated / unreachable code
- `let _ = first;` (F-002).
- `contract_frames` telemetry: never asserted (F-003).
- fuzz/Cargo.toml itself self-describes as not wired into any gate (stable-pinned toolchain) — CI-campaign status claim verified consistent there.

## 8. Dependencies & flags
No direct external deps (codec-api/media-types only); the fuzz campaign package (fuzz/) depends on libfuzzer-sys 0.4.13 and the codecs crate by path. Harness is `pub` (survival tests + target file import it from outside).

## 9. Open questions for the next agent
- aac/decoder.rs and aac/adts.rs are separate assignments — the harness's correctness leans on their gates (profile/channel/1024-frame refusals); spot-checked only here.
- Was the fuzz/ scaffold landed by a later card (5.Q1 per the target file header vs 5.C2a here)? The card-number mismatch (5.C2a vs 5.Q1) between the two headers suggests the doc paragraph predates the scaffold card and was never revisited.
