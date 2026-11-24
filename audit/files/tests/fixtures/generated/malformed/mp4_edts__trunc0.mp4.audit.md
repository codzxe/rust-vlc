# AUDIT: tests/fixtures/generated/malformed/mp4_edts__trunc0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (bytes real, consumed by a green baseline test) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: ISO Media MP4 v1 (edts variant), trunc0 derivative (size=13100, sha256=9f46d300a37be82d8e21804e6b4d0a97acad1d18a075eea6a988e4a981bfdd9c — batch table this run).
Consumer: crates/demux/tests/mp4_fixtures.rs:236 in the MALFORMED list -> `malformed_mp4_fixtures_survive_the_fuzz_harness` (:243) — baseline mp4_fixtures 5 passed (cargo-test-workspace.txt:2368 binary, result ok).
F-001 S3: unregistered in generated/MANIFEST.toml (28-row set has mp4_mjpeg_pcm__trunc0 but NOT mp4_edts__trunc0) — no sha256 ledger, no drift gate for this file.

## 2. Findings
See F-001 in the verdict line above; details:
F-001: registration/execution gap — see the verdict text. Verification: read
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
