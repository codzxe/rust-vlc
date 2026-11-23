# AUDIT: crates/demux/src/es/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 130 / 2 pub harnesses + 1 private drain + 2 tests |
| File verdict | REAL-UNPROVEN (real harness logic; its designed consumer — cargo-fuzz — cannot run) |
| Reachable from shipped entry point? | NO — `pub mod fuzz` (es/mod.rs:44) is compiled unconditionally but no lumen-cli/desktop code calls it; only callers are fuzz/fuzz_targets/*.rs (nightly, unwired) and its own #[cfg(test)] sweeps |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux --lib es::fuzz` → 2 passed / 0 failed (exit 0); rg caller map; sed quote verification |

## 1. One-paragraph verdict
The harness logic is real and panic-safe by construction: probes over the 16-byte head,
three full open/start/drain lanes (h264/hevc annexb + aac) and a raw-MP3 lane with
parse_head probes at clamped input-derived offsets, all bounded by a 200k-event budget
with arithmetic that cannot slice out of range (verified by hand: `at <= len-4`). I ran
its two deterministic sweeps — green. But the harness's designed execution vehicle is
coverage-guided cargo-fuzz, and the fuzz campaign is SCAFFOLDED, NOT WIRED (nightly
unwired, fuzz/README.md:17 "one Lead decision outstanding") — so in practice this code
runs only against ~70 fixed bytes, and the module (plus the avi::test_support seam it
pulls in) is dead weight compiled into every build.

## 2. What actually works (verified)
— `cargo test -p demux --lib es::fuzz` -> 2 passed, 0 failed
  (empty/marker inputs; all-0x00/all-0xFF 4096-byte floods; 64 ramping prefixes 1..64 B).
— Lane entry points exist and are exercised: es/h264.rs:25, es/hevc.rs:26, es/aac.rs:30
  `open`, plus probes at :15/:16/:20 (rg verified).
— Offset math at :93-97 checked: `at = (i*97 as usize).min(len-4)`, `data[at..at+4]`
  in-bounds for every i — no panic path in the probe loop.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/es/fuzz.rs:22-28
Code:
```
//! # Wiring status (honest)
//!
//! The cargo-fuzz targets live at `fuzz/fuzz_targets/fuzz_demux_es.rs`
//! and `fuzz/fuzz_targets/fuzz_demux_mp3stream.rs` (Lead-owned nightly
//! wiring, see `fuzz/README.md`); the harness logic is meanwhile
//! proven on stable by the deterministic sweeps below and the crate's
//! ES corpus run.
```
What is wrong: The fuzz campaign this module exists for does not run —
fuzz/README.md:17 "Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding".
The "proven on stable" claim covers only the two deterministic sweeps (~70 fixed
inputs) and the corpus run; no coverage-guided execution has ever exercised these
harnesses. The doc is honest about it, but the effect is that the module's
"Never panics, never hangs" contract (:66-67) is verified against a negligible input
space. Confidence: HIGH — Verification: read + rg (no other callers).
F-002 [S4]
Where: crates/demux/src/es/mod.rs:44 + crates/demux/src/avi/mod.rs:121
Code:
```
pub mod fuzz;
```
(avi/mod.rs:121)
```
pub mod test_support;
```
What is wrong: Unconditional `pub mod` — the fuzz harness and the test-support seam it
depends on (`crate::avi::test_support::mem_stream`, es/fuzz.rs:77) are compiled into
every build including the release lane, while being unreachable from any shipped entry
point (linker GC may drop them from the final binary, but they are still type-checked,
compiled, and exported API in every profile). Test-support surface leaking into the
production compile graph because the fuzz consumers are unwired. Confidence: HIGH —
Verification: read + rg caller map.
What correct looks like: `#[cfg(any(test, fuzzing))]`-style gating or a separate
fuzzing feature.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET | 39 | REAL | — | 200k events/lane cap |
| drain_lane | 43-63 | REAL | — | budget-counted drain; `Err(_) => break` swallows by documented contract; a single non-emitting infinite next_event() would still hang (budget counts events, not work) — lane monotonicity is asserted in docs, UNVERIFIED by me |
| fuzz_demux_es | 68-86 | REAL | S4 | 3 lanes + 3 probes; head slice `len.min(16)` safe |
| fuzz_demux_mp3stream | 90-102 | REAL | S4 | parse_head at 8 clamped offsets — safe math verified |
| tests::empty_and_marker_inputs_survive | 108-114 | REAL | — | ran, green |
| tests::hostile_wide_inputs_survive | 116-129 | REAL | — | 00/FF floods + 64 ramps; ran, green |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES (panic only) | any panic in probe/open/start/drain fails it |
| hostile_wide_inputs_survive | YES (panic only) | sync-flood and start-code-flood shapes; no outcome asserts (by documented contract) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "absence-of-panic/hang/unbounded-allocation, nothing else" | :17-19 | REAL-UNPROVEN | panic-free on the fixed corpus (ran); hang-freedom rests on lane monotonicity (not executed) |
| "raw AAC (= the ADTS demuxer)" | :8 | REAL-UNPROVEN | es/aac.rs:30 exists; identity with the ADTS demuxer not verified here |
| "proven on stable by the deterministic sweeps" | :27 | REAL | 2/2 green — but that is all the execution this harness ever gets |

## 7. Dead / duplicated / unreachable code
The whole module is UNREACHABLE from shipped entry points (F-002) — compiled, exported,
called only by unwired cargo-fuzz targets and its own tests.

## 8. Dependencies & flags
plugin-api (Demuxer/DemuxEvent), crate::avi::test_support (production-graph leak,
F-002). No feature gates.

## 9. Open questions for the next agent
- es/h264.rs, es/hevc.rs, es/aac.rs, es/mp3.rs: verify each next_event() makes
  monotone progress on every path (the hang-freedom the budget cannot provide).
- Who owns the LEAD-DECISION on nightly fuzz wiring (fuzz/README.md) — 33 targets
  scaffolded, zero wired; if the decision is "never", F-002's gating becomes mandatory.
