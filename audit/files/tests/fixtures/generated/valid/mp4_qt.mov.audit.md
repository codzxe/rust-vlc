# AUDIT: tests/fixtures/generated/valid/mp4_qt.mov
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 13754 B / 0ad6a4ece7e24770714887ecf0cd7d21421d2a0e9cb0c88647e329db00b98ca3 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/mp4_fixtures.rs + crates/media-probe/tests/registry_roundtrip.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |

## 1. Verdict (data audit)
Genuine QuickTime-family file: `ftyp qt  ` (brand `qt  `, version 0x0200), mdat-before-moov layout (mdat at offset 28 followed by moov) — the structurally interesting "moov at end" case for the MP4 demuxer. Consumed as a fixture row in the demux mp4 fixtures suite and as the media-probe registry's mov representative. That suite passes in the committed baseline.

## 2. What was checked and how
- batch.py: 13754 B, sha256 0ad6a4ec…; box walk: ftyp(20) / wide(8) / mdat(11871) / moov(1855) with mvhd/trak/edts/mdia/minf/stbl — mdat precedes moov (offset 28 < 2204), the QT-scope layout.
- Consumers: crates/demux/tests/mp4_fixtures.rs:45 (`fixture: "generated/valid/mp4_qt.mov"`) and :232 (its trunc0 derivative lane); crates/media-probe/tests/registry_roundtrip.rs:67 (`fixture: Some("generated/valid/mp4_qt.mov")`).
- Execution: audit/runs/baseline/cargo-test-workspace.txt — demux mp4 fixtures + media-probe suites passed (e.g. `parent_fixture_opens_with_expected_facts ... ok` line 3064).
- Generated-vs-handmade: xtask-generated family (seed lumen-2a3-v1) but NOT in MANIFEST.toml's 28 pinned entries (verified by python cross-check).

## 3. Findings
F-001 [S3] Not covered by the MANIFEST.toml drift gate (28/117 files pinned; the mov family is absent) — silent drift undetectable. Verification: ran (python manifest cross-check).
F-002 [S3] Not covered by the external-oracle pin perimeter either: scripts/oracle_check.sh pins only {.avi,.mp4} extensions ("a future .mkv/.mov/.webm/… fixture must extend the pin set"); this .mov has no ffmpeg-oracle sidecar. The ffmpeg-oracle discipline does not reach the QuickTime lane. Verification: read (oracle_check.sh:47-50).
