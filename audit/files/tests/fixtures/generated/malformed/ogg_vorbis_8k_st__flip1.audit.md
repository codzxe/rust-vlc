# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__flip1
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (bytes real, PINNED by exact expected outcomes in a green baseline test) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: Ogg Vorbis stereo 8000 Hz (~27.2 kbps), flip1 derivative (size=3245, sha256=1c6062cfed7e3971f1a026900dc26c804b088a78ab90149669440d54df897b3c — batch table this run).
Consumer: crates/demux/tests/ogg_fixtures.rs:565 DERIVATIVES walk; flip1 pinned at :513-520 — expects 401 payload bytes, 14 packets, reason 'ogg: page CRC mismatch', damage 2_745, 1 granule discontinuity. Baseline: `ogg_derivatives_stay_inside_the_typed_vocabulary ... ok` (:2384) and fuzz-survival ok (:2385).
F-001 S3: unregistered in generated/MANIFEST.toml (ogg family absent from the 28-row ledger) — yet the expected-outcome pin in ogg_fixtures.rs is stronger provenance than the manifest itself.

## 2. Findings
See F-001 in the verdict line above; details:
F-001: registration/execution gap — see the verdict text. Verification: read
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
