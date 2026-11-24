# AUDIT: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.structure.json
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (registered AND digest-verified against MANIFEST.toml) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: JSON structure-map/1 — offsets of top-level AVI chunks and movi entries (size=6123, sha256=dc24d8b6bec5d3ff030ed234e0f3e491c2016c9d33354b331586d02d51cf0a32 — batch table this run).
Consumer: tests/fixtures/generated/MANIFEST.toml:16-21 row `generated/valid/avi_mjpeg_pcm.avi.structure.json` op=structure-map, parent the .avi: sha256 dc24d8b6… == my computed digest, size 6_123 == actual. Consumers: tools/xtask/src/cmd/fixtures.rs (generator), tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc2.avi.toml, CORPUS.md; the class-B derivatives derive cut positions from these offsets.
Content inspected (python3): schema structure-map/1, fixture field names the parent, size field 425516 matches the parent's manifest row, top_level RIFF/LIST hdrl offsets present. This is one of the 28 REGISTERED files — the drift gate covers it. Zero findings.

## 2. Findings
None. Registered, digest-verified, consumed by the generator + expectations + baseline
class-B tests. What was checked: manifest row equality (sha256+size, python3 this run),
JSON content sanity, consumer rg, CORPUS.md/expectation references.
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
