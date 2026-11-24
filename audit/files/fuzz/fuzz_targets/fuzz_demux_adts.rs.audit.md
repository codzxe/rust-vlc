# AUDIT: fuzz/fuzz_targets/fuzz_demux_adts.rs
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 fuzz_target! |
| File verdict | REAL-UNPROVEN (shim real, harness exists and is baseline-proven; the libFuzzer campaign itself never evidenced) |
| Reachable from shipped entry point? | NO — fuzz campaign asset; nightly CI lane .github/workflows/audit.yml:325-351 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg harness fn; rg baseline; rg audit.yml + campaign script |

## 1. Verdict
Thin libFuzzer shim that delegates to the real in-crate harness
`demux::adts::fuzz::fuzz_demux_adts` (verified to exist at crates/demux/src/adts/fuzz.rs:47).
The harness logic is baseline-proven by deterministic sweeps: `corpus_adts_survive_the_fuzz_
harness ... ok` (cargo-test-workspace.txt:2175, adts_corpus 4/4 ok). The registry gate passed
in the orchestrator baseline (full_gate.txt:57 "35 targets, diff empty, corpora present").
The target itself has never executed under libFuzzer in any evidenced run.

## 2. What actually works (verified)
- Harness fn exists at the exact registry-named path (rg).
- Baseline sweep green (cite above) — the survival contract over real ADTS corpus bytes.
- Registry-gate membership: the shim is counted in the 35-target diff-empty check (baseline).

## 3. Findings
F-001 [S3]
Where: fuzz/README.md:17-42 ("Status: SCAFFOLDED, NOT WIRED") vs .github/workflows/audit.yml:325-351
Code: `fuzz:` job — `bash scripts/quality/25_fuzz_campaign.sh 180 "${MINE[@]}"` (5 shards,
      nightly schedule 02:00)
What is wrong: the README status block says nothing wires the campaign until a Lead decision
lands, but audit.yml's nightly `fuzz` tier runs cargo-fuzz per target today. The README is
stale; worse, no campaign evidence is committed (docs/audit/reports/fuzz/ does not exist —
checked this run), so "the nightly lane runs" is also UNVERIFIED. Both the doc and the
evidence state are wrong in opposite directions.
What correct looks like: update the README status; land one campaign summary artifact.
Confidence: HIGH — Verification: read (audit.yml sed 325-360; ls docs/audit/reports/fuzz → absent).

## 4. Test honesty
No tests in-file; equivalence-layer evidence is the adts_corpus sweep (green) and the
registry gate (green). The shim forwards verbatim — nothing else to test.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compiles on stable (`cargo check` in fuzz/)" | :8-9 | UNVERIFIED | no cargo in this env; registry gate is the nearest evidence |
| "harness logic is proven by the in-crate deterministic hostile-input sweeps" | :9-10 | REAL | baseline :2175 ok |

## 6. Dead / duplicated / unreachable code
None — 1:1 delegation.

## 7. Dependencies & flags
libfuzzer_sys::fuzz_target!; no features; separate fuzz workspace root (README:170-182).

## 8. Open questions for the next agent
Did any scheduled audit.yml fuzz run ever execute? No committed artifact says yes.
