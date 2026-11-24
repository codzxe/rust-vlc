# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__garbage0
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 7341 B / 3abf94d9c86c154701a7bd416cc92b66a350783af47bce2a98f83763fdbc30db |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/ogg_fixtures.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (data audit)
Genuine `garbage-append` Class-B derivative: the valid `ogg_vorbis_8k_st.ogg` (3245 B, 0 common-prefix diff bytes — pure prefix) with 4096 bytes of garbage appended (delta +4096 = GARBAGE_LEN, tools/xtask/src/cmd/fixtures.rs:2434 "one 4 KiB page"). Ogg page magic `OggS` intact at the head (3 page headers counted); the demuxer must therefore conceal/recover rather than refuse — the pinned outcome class. The ogg fixtures suite passes in the committed baseline.

## 2. What was checked and how
- batch.py: 7341 B, sha256 3abf94d9…, `ogg={'pages': 3, …}`; PAIRS vs valid parent: delta +4096, 0 diff bytes in common prefix.
- Consumers: crates/demux/tests/ogg_fixtures.rs (derivative table entry `name: "garbage0"` at line 551; module doc lines 68-72 record the derivative set and "garbage0 appends 4096 `0xEE` bytes past the recorded sites").
- Execution: audit/runs/baseline/cargo-test-workspace.txt — ogg_fixtures suite passes (`every_ogg_fixture...` ok; see demux section of the baseline).
- Generated-vs-handmade: xtask-generated; NOT in MANIFEST.toml (python cross-check).

## 3. Findings
F-001 [S3] Not covered by the MANIFEST.toml drift gate (ogg family absent from the 28 pinned entries). Verification: ran (python manifest cross-check).
