# AUDIT: fuzz/fuzz_targets/fuzz_demux_es.rs
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 20 lines / 1 fuzz_target! macro |
| File verdict | REAL-UNPROVEN (thin shim; harness fn exists; compiles-on-stable claimed; no campaign) |
| Reachable from shipped entry point? | NO — fuzz lane only; campaign unwired (Lead decision pending, fuzz/README:13-31) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg harness existence (exit 0); RAN scripts/fuzz_registry_check.sh -> CLEAN 35 targets (exit 0) |

## 1. One-paragraph verdict
Correct thin libFuzzer shim, exactly what the registry promises: one call = one full
untrusted pass through the real harness `demux::es::fuzz::fuzz_demux_es` — which I verified
exists at crates/demux/src/es/fuzz.rs:68 (`pub fn fuzz_demux_es(data: &[u8])`). The shim adds
no logic, no panics, no allocation. The wiring status is HONESTLY disclosed in the header
("SCAFFOLDED, NOT WIRED" — campaign needs the Lead's nightly toolchain decision); the
registry gate I RAN confirms target/harness/README/corpus consistency.

## 2. What actually works (verified)
- `rg -n "fn fuzz_demux_es" crates/demux/src/es/fuzz.rs` -> line 68 (exit 0).
- `bash scripts/fuzz_registry_check.sh` -> `fuzz-registry: CLEAN — 35 targets, diff empty,
  corpora present` (exit 0) — bins/files/readme/harness sets identical.
- Baseline full_gate.txt:56-58 records the same gate PASS.

## 3. Findings
None specific to this shim. Systemic fuzz-lane status (honest, README): no libFuzzer campaign
has ever run in-tree; survival is instead proven by in-crate deterministic sweeps
(crates/demux/src/es/fuzz.rs, host-input sweeps) — those ran in the per-package baseline
(PACKAGE demux exit_code=0, cargo-test-workspace.txt:1698-2481).

## 4. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compiles on stable ... verified 2026-11-09 (35 targets)" | fuzz/README.md:31-33 | REAL-UNPROVEN | no cargo in sandbox; registry gate (bash) passes; compile claim not re-executed |
| harness body "panic-free by construction" | shim header line 7 | REAL-UNPROVEN | the harness module's own tests pass in baseline (demux package) |

## 5. Open questions
- None for the shim. The campaign wiring decision is the repo's known open item.
