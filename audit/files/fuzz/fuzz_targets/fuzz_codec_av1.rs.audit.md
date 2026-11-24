# AUDIT: fuzz/fuzz_targets/fuzz_codec_av1.rs
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 fuzz_target! |
| File verdict | REAL-UNPROVEN (shim real, harness exists + baseline-proven sweeps; campaign never evidenced) |
| Reachable from shipped entry point? | NO — fuzz campaign asset; nightly audit.yml fuzz tier |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg harness fn; rg baseline (:619-620); rg audit.yml |

## 1. Verdict
Thin shim delegating to `codecs::video::av1::fuzz::fuzz_codec_av1` (exists at
crates/codecs/src/video/av1/fuzz.rs:52, rg verified). Harness logic baseline-proven:
`video::av1::fuzz::tests::empty_and_short_inputs_survive ... ok` and
`hostile_wide_inputs_survive ... ok` (cargo-test-workspace.txt:619-620). Registry-gate
member (full_gate.txt:57, 35 targets clean). No evidenced libFuzzer execution ever.

## 2. What actually works (verified)
- Harness fn exists; two in-crate hostile sweeps green in baseline (cites above) — the
  OBU-parse + rav1d contract path is exercised deterministically today.
- The fuzz/corpus/fuzz_codec_av1/ dir carries seed_ivf_av1.ivf (listed this run) so the
  campaign, when it runs, starts from a real IVF-wrapped AV1 shape.

## 3. Findings
F-001 [S3]
Where: fuzz/README.md:17-42 vs .github/workflows/audit.yml:325-351; docs/audit/reports/fuzz/ absent
What is wrong: identical stale-status + missing-evidence pair as the adts target (same
README block claims unwired while the nightly fuzz tier exists; no committed campaign
artifact proves any run). Cross-file duplicate of the W16-A02 stale-wiring finding.
Confidence: HIGH — Verification: read + ls (no committed reports).

## 4. Test honesty
No in-file tests; the av1 fuzz.rs module's own 2 sweeps are the equivalence layer (green).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "panic-free by construction, bounds documented there" | :3-4 | UNVERIFIED | av1/fuzz.rs module body not in my assignment |
| sweeps prove harness logic | :9-10 | REAL | baseline :619-620 |

## 6. Dead / duplicated / unreachable code
None — 1:1 delegation.

## 7. Dependencies & flags
libfuzzer_sys; rav1d dependency lives behind the av1 decoder crate, not here.

## 8. Open questions for the next agent
Same as adts: land one campaign summary artifact.
