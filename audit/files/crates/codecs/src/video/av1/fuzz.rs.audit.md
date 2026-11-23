# AUDIT: crates/codecs/src/video/av1/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 115 / 1 harness fn + 1 sink + 2 tests |
| File verdict | REAL (harness + wired libFuzzer shim; deterministic sweeps pass live) — campaign itself nightly-gated, disclosed |
| Reachable from shipped entry point? | Harness: via the cargo-fuzz target `fuzz/fuzz_targets/fuzz_codec_av1.rs` (exists, calls this fn verbatim) — fuzz campaigns are dev tooling, not shipped. Code under test IS shipped: `"av1"` arm in apps/cli/src/pipeline_host.rs:305 (`codec-av1` default on) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A small honest fuzz harness: one call = one dirty buffer through both the pure OBU parse and the full shipped-decoder contract cycle (configure → decode → flush over rav1d, asm-off), with the contract explicitly scoped to absence-of-panic/hang/unbounded-allocation ("nothing else"). The libFuzzer shim exists and calls exactly this function (verified by reading fuzz/fuzz_targets/fuzz_codec_av1.rs); the nightly campaign is disclosed as Lead-owned wiring not yet run. The in-crate deterministic sweeps (33 short inputs + all-0xFF/all-0x00 4 KiB buffers) pass live. Two hygiene notes: the `CountingSink` writes a frame count that nothing ever reads (a free unbounded-emission check, wasted), and the stable-side sweep is 37 inputs — the "proven on stable" phrasing rests on that plus the conformance suites.

## 2. What actually works (verified)
- `timeout 60 ./target/debug/deps/codecs-bae0f1756cc0a809 video::av1::fuzz --test-threads=2` → `2 passed; 0 failed` (0.04s).
- Wiring: `fuzz/fuzz_targets/fuzz_codec_av1.rs` exists, is registered in `fuzz/Cargo.toml:223-224`, and its body is `fuzz_target!(|data: &[u8]| { codecs::video::av1::fuzz::fuzz_codec_av1(data); })` — the doc's wiring claim is REAL, not phantom.

## 3. Findings

F-001 [S4 — CountingSink's count is write-only; the free runaway-emission assert is missing]
Where: av1/fuzz.rs:38-47, 89-91
Code:
```
impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}
```
What is wrong: `sink.frames` is never read. One TU can legitimately emit at most a handful of frames; asserting `sink.frames <= (say) 8` after the cycle would turn the sink into a real unbounded-emission detector — the exact class of bug the module doc says the harness contracts against ("never allocates beyond input-derived bounds"). As written the counter is dead state.
Confidence: HIGH — Verification: read (rg for `frames` — only the increment site).

F-002 [S4 — "proven on stable by the deterministic sweeps" rests on 37 fixed inputs]
Where: av1/fuzz.rs:20-23, 98-114
Code:
```
//! the harness logic is meanwhile proven on stable by the
//! deterministic sweeps below and the crate's AV1
//! conformance/differential suites.
```
What is wrong: nothing dishonest — the conformance/differential suites are the real weight (av1_conformance binary exists in target/debug/deps) — but the sweeps alone (32 short sequential patterns + 2 constant buffers) are a smoke, and the nightly campaign has never run (Lead wiring pending). The wiring claim is verified; the campaign's absence is disclosed. Recording so no one cites "fuzzed" for AV1 without the nightly qualifier.
Confidence: HIGH — Verification: read + ran the sweeps; fuzz campaign not runnable here (UNVERIFIED: cargo-build-budget — building fuzz/ requires the nightly toolchain).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink (FrameSink) | 37-47 | REAL | S4 | F-001 dead counter |
| fuzz_codec_av1 | 52-92 | REAL | — | both surfaces; typed swallows by contract; first-byte-driven truncation cut |
| tests ×2 | 98-114 | REAL | — | pass live |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_short_inputs_survive | YES (panic-aborts only) | the contract is absence-of-panic — a panic in parse/decode/flush fails the test |
| hostile_wide_inputs_survive | YES (same mechanism) | 0xFF/0x00 maximal-false-positive headers |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target is fuzz/fuzz_targets/fuzz_codec_av1.rs" | :20-21 | TRUE | file exists, calls this fn, registered in fuzz/Cargo.toml:223 |
| "harness logic is meanwhile proven on stable by the deterministic sweeps" | :22-23 | TRUE-but-thin | sweeps pass; 37 inputs (F-002) |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :13-16 | TRUE | code matches — nothing else is asserted (and F-001 notes the missing free check) |
| "Memory is bounded by the input" | :27-29 | REAL-UNPROVEN | rav1d's internal ceilings not audited here; UNVERIFIED |

## 7. Dead / duplicated / unreachable code
`CountingSink.frames` (F-001). The module compiles into the shipped binary via `pub mod fuzz` (av1/mod.rs:79) — dev tooling riding the production feature; harmless (no side effects at rest).

## 8. Dependencies & flags
codec-api, media-types, rav1d-backed Av1Decoder (`codec-av1` default on). The fuzz crate is a separate workspace-external package (nightly, Lead-owned).

## 9. Open questions for the next agent
- Has the nightly campaign ever actually run anywhere (CI evidence)? fuzz/README.md claims Lead-owned wiring — if a doc cites campaign coverage numbers, verify they exist.
