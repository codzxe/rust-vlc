# AUDIT: fuzz/corpus/fuzz_demux_mkv/seed_mkv_flip1.mkv
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (cargo-fuzz convention) |
| Size / sha256-prefix | 26105 bytes / 12151d0963d7fb5a (verified locally) |
| File verdict | REAL — byte-identical copy of a committed fixture |
| Reachable from shipped entry point? | NO — fuzz seeds reach only the (unwired) libFuzzer campaign |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 vs tests/fixtures/generated/malformed/mkv_h264_aac__flip1.mkv -> MATCH |

## 1. Verdict
Real seed: sha256 `12151d0963d7fb5a` is byte-identical to
tests/fixtures/generated/malformed/mkv_h264_aac__flip1.mkv — exactly the
"copied from the committed CC0 fixtures" provenance fuzz/README.md:101-103
claims. Presence is enforced: the registry gate
(scripts/fuzz_registry_check.sh, "corpora present for every target")
PASSED in the orchestrator baseline (audit/runs/baseline/full_gate.txt:56-58:
"fuzz-registry: CLEAN — 35 targets, diff empty, corpora present").

## 2. What actually works (verified)
- Hash match against the fixture it claims to copy (command above).
- fuzz/README.md:112-116 (verbatim): "2. `fuzz/corpus/<target>/` relative to
  the repository root (the committed default — cargo-fuzz's own convention)."
- Gate evidence: audit/runs/baseline/full_gate.txt:57-58.

## 3. Findings
None specific to this file. Campaign-level status is recorded in the fuzz
README itself ("SCAFFOLDED, NOT WIRED" — no run has consumed these seeds in
campaign mode); that is a repo-level accepted state, not a defect of this
file. Matrix rows cite "8k fuzz rounds" (row 524) via the in-crate fuzz-lite
loops, not this seed.
