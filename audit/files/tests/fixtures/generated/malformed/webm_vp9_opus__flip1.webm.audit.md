# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__flip1.webm
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (bytes real, consumed by a green baseline test) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: WebM container, flip1 derivative of the webm_vp9_opus parent (size=27259, sha256=8593fc66a5e5eb9a1fab8e80d0966deb60298d35660095dcd7ebe693936e86d8 — batch table this run).
Consumer: crates/demux/tests/mkv_fixtures.rs:364 in the same 18-name list; same test :345 green in baseline (:2358).
F-001 S3: unregistered in tests/fixtures/generated/MANIFEST.toml — the drift gate (`cargo xtask fixtures --check`) covers only the 28 avi_mjpeg_pcm/mp4_mjpeg_pcm rows; this file's provenance/drift rides on prose. Same shape as W16-A02's cross-cutting finding.

## 2. Findings
See F-001 in the verdict line above; details:
F-001: registration/execution gap — see the verdict text. Verification: read
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
