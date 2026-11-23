# AUDIT: crates/demux/src/ogg/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 99 / 4 (3 budget constants + fuzz_demux_ogg) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test/fuzz surface: invoked by tests/ogg_fixtures.rs:620-634 and tests/corpus_ogg.rs:290+ (deterministic sweeps, RUN and pass); the libFuzzer shim fuzz/fuzz_targets/fuzz_demux_ogg.rs calls it (exists, registered in fuzz/Cargo.toml:118-119) — an actual libFuzzer campaign is UNVERIFIED (nightly wiring pending per the target's own comment) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p demux --test ogg_fixtures survive` -> exit 0, 1 passed (0.02s); `ls fuzz/fuzz_targets/` + grep of fuzz/Cargo.toml and fuzz/README.md (target registration); sed quote verification |

## 1. One-paragraph verdict
A clean, honestly-scoped fuzz harness: one pass = construct + start + drain with a hard 500k event budget, then 3 data-derived seek probes (fractions of the observed last granule) each followed by a bounded 64-event drain — so the post-seek paths are fuzz surface too, which most harnesses forget. The contract is explicitly absence-of-panic/hang/unbounded-allocation with typed errors swallowed BY DESIGN (the conformance suites pin the flavors), saturation on all arithmetic, and the demuxer's own MAX_RECOVERY_SCAN_BYTES/MAX_PACKET_BYTES ceilings referenced. The deterministic in-crate sweeps that exercise it run and pass (my run: fixtures + derivatives + chain truncations + every-1024th-byte flips). One stale doc: the module's "Wiring status (honest)" paragraph says the cargo-fuzz target file has not landed — it HAS (fuzz/fuzz_targets/fuzz_demux_ogg.rs exists, registered in fuzz/Cargo.toml and fuzz/README.md, with libfuzzer-sys 0.4.13 as a dependency); what is actually pending is the nightly campaign wiring, as the target's own newer comment states.

## 2. What actually works (verified)
— verified by `cargo test -p demux --test ogg_fixtures survive` -> `test ogg_fixtures_and_derivatives_survive_the_fuzz_harness ... ok` (my run, exit 0; matches baseline ogg_fixtures section: 4 passed).
— verified by ls/grep: fuzz/fuzz_targets/fuzz_demux_ogg.rs exists and is a thin `fuzz_target!` shim over this module; registered at fuzz/Cargo.toml:118-119 and fuzz/README.md:59.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ogg/fuzz.rs:12-19
Code:
```
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_ogg.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI/WAV cards recorded; until it lands the harness logic is
//! proven by the deterministic in-crate sweeps
```
What is wrong: The doc claims the target file has not landed. It has: `fuzz/fuzz_targets/fuzz_demux_ogg.rs` exists, calls this module via `fuzz_target!`, is registered in fuzz/Cargo.toml (`name = "fuzz_demux_ogg"`), and libfuzzer-sys 0.4.13 is already a dependency of the fuzz package. The still-true part (per the target file's own comment) is only that the nightly campaign wiring is pending. Why it matters: stale status docs misdirect the next contributor (they would re-create an existing file). What correct looks like: update the paragraph to the target file's own current wording. Confidence: HIGH — Verification: read + ls + grep. Confidence on campaign status: campaign NOT run here — UNVERIFIED (nightly toolchain absent in sandbox).

Zero S0-S3 findings. Checked and clean: event budget enforced after every event (line 73-76, counted on all arms including swallowed errors? — no: `Err(_) => break` exits before increment, correct); seek-probe arithmetic is saturating with `ticks.max(0)` clamp; post-seek drain bounded at 64 with EOS/Err early exit; the harness never panics by construction (no unwrap/indexing; `i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3)` is const-safe).

## 4. Per-function table (file ≤100 LOC — compact table included anyway)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET / FUZZ_POST_SEEK_DRAIN / FUZZ_SEEK_PROBES | 39/42/45 | REAL | — | [PROPOSAL]-tagged budgets, enforced |
| fuzz_demux_ogg | 50-99 | REAL | — | budgeted drain + 3 seek probes + bounded post-seek drains; errors swallowed by contract |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ogg_fixtures_and_derivatives_survive_the_fuzz_harness (caller, ogg_fixtures.rs:610-638) | YES | any panic in the demuxer over committed fixtures/derivatives/truncations/flips propagates through this harness |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-8 | REAL | body matches: drains, swallows, budgets |
| "The cargo-fuzz target file ... until it lands" | fuzz.rs:14-17 | FALSE (stale) | target file exists + registered (fuzz/Cargo.toml:118-119, README:59) |
| "Event budget: every loop has a hard cap" | fuzz.rs:23-24 | REAL | line 74-76; post-seek loop 91-96 — note `start()`/`reanchor()` themselves are unbudgeted, relying on the demuxer's structural bounds, which the doc discloses as the "first belt" |

## 7. Dead / duplicated / unreachable code
None in-file.

## 8. Dependencies & flags
No new deps (media-types/plugin_api/super). The libFuzzer dependency lives in the separate fuzz package (Lead-owned), not the demux crate. No feature gates.

## 9. Open questions for the next agent
- fuzz/README.md owner: is the nightly campaign wiring ever validated in CI? No campaign run exists in audit/runs/baseline/ — the "+FB" card's fuzzing evidence rests entirely on the deterministic sweeps.
- corpus_ogg.rs's `corpus_oggs_survive_the_fuzz_harness` soft-skips when the corpus is absent (same vacuous-pass pattern as ts_fuzz_survival F-001 — flag for that file's auditor).
