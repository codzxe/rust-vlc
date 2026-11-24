# AUDIT: fuzz/corpus/fuzz_demux_aiff/seed_aifc_u8.aifc
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus asset (fuzz/corpus/<target>/) |
| File verdict | REAL-UNPROVEN (real bytes, sane identity; never executed by any committed run) |
| Reachable from shipped entry point? | N/A — fuzz corpus |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; full_gate citation |

## 1. Verdict & evidence
Identity: IFF AIFF-C compressed audio (u8 samples) — the AIFC variant of the AIFF/AIFC pair the README's 5.A2 row names (size=8892, sha256=85ae4c5103a4fbc346d4b0423885a0eb1504541ff4a8e70c40c438023067d8d3 — batch table this run).
Consumption: scripts/fuzz_registry_check.sh (full_gate step 6) verifies the corpus dir is non-empty; full_gate.txt:57 `fuzz-registry: CLEAN … corpora present` PASS in the orchestrator baseline.
Siblings in the dir (seed_aiff_s16be.aiff, seed_aiff_trunc0.bin) cover the big-endian PCM and truncation shapes; this seed covers the compressed-sound 'sowt'-family variant.

## 2. Finding
F-001 [S3]
Where: fuzz/README.md:99-119 (corpus policy) and :165-168; audit/runs/baseline/full_gate.txt:56-58
Code: `fuzz-registry: CLEAN — 35 targets, diff empty, corpora present` /
      (README) "**No such run has happened yet** — the nightly wiring is the
      outstanding LEAD-DECISION above"
What is wrong: the ONLY committed consumer of this seed's bytes is the registry gate's
existence check (corpora present) — no cargo-fuzz campaign has ever run (README's own
admission), and the stable fuzz_loop_stable tests mutate fixture bytes via inline RNG,
NOT these seeds. The seed is real but its survival value is unproven anywhere.
Why it matters: a corrupt/regressive seed would never be caught by any gate.
What correct looks like: wire the nightly campaign (LEAD-DECISION) or a stable corpus
sweep test that feeds every committed seed through its harness.
Confidence: HIGH — Verification: read (README + full_gate log) + rg (no other consumers).
Checked otherwise: file identity, sha256/size, header bytes (python3), consumer rg across
tests/ crates/ fuzz/ tools/ scripts/ .github/.
