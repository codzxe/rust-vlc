# AUDIT: crates/demux/src/adts/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 96 / 1 (fuzz_demux_adts) + 3 budget consts |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for release binaries (test/fuzz-surface only: in-crate `#[cfg(test)]` sweeps, `tests/adts_corpus.rs`, and the nightly-gated `fuzz/fuzz_targets/fuzz_demux_adts.rs` shim — verified bytes: `demux::adts::fuzz::fuzz_demux_adts(data)`). The AdtsDemuxer it drives IS reachable (containers registry, "adts" lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 byte-reads of fuzz/Cargo.toml + fuzz target (display-pipeline artifact bypassed), rg callers/baseline, ls fixtures, sed quotes. No cargo runs. |

## 1. One-paragraph verdict
A real, bounded fuzz harness: construct -> start -> drain with a 500k-event budget -> three seek probes at fractions of the last observed tick, each followed by a 64-event drain (the post-seek paths are exercised, which many harnesses forget). All error outcomes are swallowed by design and the module doc says so in bold — the contract is absence-of-panic/hang/unbounded-allocation, with typed flavors pinned by the conformance suite elsewhere (verified: adts/tests.rs drains assert terminal codes; baseline lines 1768, 2173-2175 all `ok`). The honest limitations: the registered libFuzzer campaign has NEVER executed (fuzz/ is a stable-compiled scaffold, "NOT wired into any gate" per its own manifest), so all fuzz evidence is the deterministic sweeps — 4 committed .bin derivatives (verified present, 2-8 KiB) plus corpus-conditional truncation/flip sweeps that SKIP when the corpus dir is absent (it is absent in this sandbox); the seek probes hardcode a 1/48_000 base regardless of the stream's real base; and the in-crate test name "typed_or_recover" overpromises what the panic-only harness checks.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt: `adts::tests::committed_mutation_fixtures_are_typed_or_recover_never_panics ... ok` (line 1768), `corpus_adts_survive_the_fuzz_harness ... ok` (2175) — the harness body executes in CI over real bytes.
— verified by ls: crates/demux/fixtures/adts/{0059-trunc-4k,0110-trunc-8k,0059-flip-idx15,0059-zero-range}.bin all present (2048-8192 B).
— read: budget accounting is correct (increment-then-check; EOS/Err break before counting), `saturating_mul` + `.max(0)` make the seek-target arithmetic panic-free and overflow-free; reanchor errors `continue` cleanly.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/adts/fuzz.rs:12-18 + fuzz/Cargo.toml (scaffold note)
Code (verbatim, 14-18):
```
//! The cargo-fuzz target file is a Lead-owned dependency decision
//! (`libfuzzer-sys`), exactly as the AVI/WAV/Ogg cards recorded; until
//! it lands the harness logic is proven by the deterministic in-crate
//! sweeps (bit-flip/truncation families over the committed fixtures
//! and the real corpus, run by the unit + integration suites).
```
What is wrong: the target file exists and compiles on stable (verified: fuzz/fuzz_targets/fuzz_demux_adts.rs is a `fuzz_target!` shim; fuzz/Cargo.toml: "It is NOT wired into any gate: building it requires a nightly toolchain"), so the doc's "until it lands" understates the state: the scaffold landed, the campaign never runs. Zero libFuzzer executions have ever gated this repo; "fuzz" evidence = 4 committed derivatives + corpus sweeps that `eprintln!("...SKIP")` and return when the corpus is missing (adts_corpus.rs:396-398; corpus absent in this sandbox). Docs are honest about the mechanism, but the registered name `fuzz_demux_adts` (doc 04 §8.5) suggests a coverage that does not exist. Confidence: HIGH — Verification: read + python3 byte-reads (the shell display pipeline mangles these filenames; bytes verified directly).
F-002 [S4]
Where: crates/demux/src/adts/fuzz.rs:77-84
Code (verbatim):
```
    if let Ok(base) = TimeBase::new(1, 48_000) {
        for fraction in 1..=FUZZ_SEEK_PROBES {
            let ticks = last_ticks.saturating_mul(fraction as i64)
                / i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3);
```
What is wrong: the seek probes re-wrap the stream's own `last_ticks` (observed on the stream's real time base — ADTS lanes are typically 1/44100 etc.) in a hardcoded 1/48_000 base, so the probe targets are the wrong times by the base ratio whenever the stream base differs. Harmless for the panic-contract (any target exercises the path) but misleading to maintainers and it under-exercises base-conversion edges in the seek path. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/src/adts/tests.rs:630-643 (the caller's test name)
Code (verbatim, 630-631):
```
fn committed_mutation_fixtures_are_typed_or_recover_never_panics() {
```
What is wrong: the test's name promises "typed or recover" verification, but its body only reads the fixtures, asserts non-empty, and calls the swallow-everything harness — the typed/recover outcomes are pinned by the OTHER adts tests, not this one. Panic-only, like the corpus sweep. Confidence: HIGH — Verification: read of tests.rs:630-643.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET / FUZZ_POST_SEEK_DRAIN / FUZZ_SEEK_PROBES | 34-42 | REAL | — | published bounds, proposal-tagged |
| fuzz_demux_adts | 47-96 | REAL | S4 x2 | bounded drain + 3 seek probes; base hardcode (F-002); campaign never runs (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| adts::tests::committed_mutation_fixtures_are_typed_or_recover_never_panics | NO (panic-only) | harness swallows outcomes (F-003); the real pins live in the drain-based tests |
| corpus_adts_survive_the_fuzz_harness | NO (panic-only), corpus-conditional | truncation/flip sweeps at 4 KiB stride + dirty/garbage dirs; SKIPs when corpus absent (it is, here) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-10 | REAL | read of the body; errors swallowed at 68, 85-87, 90 |
| "until it lands the harness logic is proven by the deterministic in-crate sweeps" | fuzz.rs:16-18 | REAL (and the only proof that exists — see F-001) | baseline 1768 + 2175 pass; corpus sweeps skip-when-absent |
| "Never panics, never hangs, never allocates without a ceiling — by construction of the demuxer and the budgets above" | fuzz.rs:44-46 | REAL-UNPROVEN as a universal claim | budgets verified by read; the ADTS demuxer's own construction is another file's audit; no campaign has ever tested the claim at scale |

## 7. Dead / duplicated / unreachable code
`i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3)` (line 80): the fallback equals the constant — dead defensive branch. Nothing else.

## 8. Dependencies & flags
media-types + plugin_api + super::AdtsDemuxer + super::test_support::mem_stream. The libfuzzer-sys dependency lives only in the separate fuzz/ workspace (nightly), not in the shipped build. No unsafe.

## 9. Open questions for the next agent
- Who owns getting `cargo +nightly fuzz run fuzz_demux_adts` into a gate (Lead decision per fuzz/README)? Until then every "fuzz" row in any matrix is deterministic-sweep evidence only.
- adts/mod.rs auditor: verify MAX_RECOVERY_SCAN_BYTES / MAX_FRAME_BYTES ceilings are enforced in the demuxer (this file only references them).
