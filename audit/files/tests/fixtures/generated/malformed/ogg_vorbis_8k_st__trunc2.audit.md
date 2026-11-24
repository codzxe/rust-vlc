# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc2
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (bytes real, PINNED typed-refusal outcome in a green baseline test) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: Ogg Vorbis stereo 8000 Hz, trunc2 derivative (truncated mid-page) (size=1500, sha256=19d48391206fdb70cab45ed7ba374811bf8dedb875bbc84b965e621da91102eb — batch table this run).
Consumer: ogg_fixtures.rs:565 DERIVATIVES; trunc2 pinned at :474-481 — fails at start with terminal `demux.ogg.truncated-page`, sticky on replay. Baseline :2384/:2385 both ok.
F-001 S3: unregistered in generated/MANIFEST.toml (same as flip1 sibling).

## 2. Findings
See F-001 in the verdict line above; details:
F-001: registration/execution gap — see the verdict text. Verification: read
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
