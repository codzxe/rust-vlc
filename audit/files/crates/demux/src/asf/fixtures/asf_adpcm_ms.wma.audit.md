# AUDIT: crates/demux/src/asf/fixtures/asf_adpcm_ms.wma (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Type / size | `Microsoft ASF` (`file`) — genuine ASF (header GUID `30 26 B2 75 8E 66 CF 11 ...` at offset 0); 6,936 bytes |
| SHA-256 | `02ca9cfcf394078b6e040a175218f16fe94c436da79f3b3536af918eb7440826` — matches the committed 16-hex pin `02ca9cfcf394078b` (asf/tests.rs:1495) |
| File verdict | REAL (differential fixture with structural + sha pins, executed green) |
| Reachable from shipped entry point? | N/A (test asset); consumers: asf/tests.rs:1052 (FixtureTruth table), :1128 `fixture_family_walks_match_the_measured_truth`, :1333 `fixture_family_cross_checks_against_live_ffprobe` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (exit 0); `sha256sum` (exit 0, prefix match verified); `od` head (exit 0); `cargo test -p demux --lib asf::tests::fixture_family` → 2 passed / 0 failed (exit 0) |

## 1. One-paragraph verdict
A real ffmpeg-authored ASF/ADPCM fixture, committed in-tree precisely because CI's
ffmpeg 6.1.x emits different bytes than the 7.1.5 the pins were measured under
(tests.rs:1074-1076 documents the rationale) — byte-stable differential evidence. It is
pinned structurally (header 486 B, 5 children, FPO 6,936/2/3,100/3,200, data object
486/6,450/2, WAVEFORMATEX tag 0x0002 mono 8 kHz align 1,024 bits 4 cbSize 32, codec
`adpcm_ms`) and by sha prefix, and both fixture-family tests ran green in this session.

## 2. What actually works (verified)
— `cargo test -p demux --lib asf::tests::fixture_family` -> 2 passed
  (walks-match-measured-truth + live-ffprobe cross-check; the second requires ffmpeg —
  it ran and passed in this sandbox).
— sha256 prefix match: computed `02ca9cfcf394078b6e04...` vs pinned
  `02ca9cfcf394078b` — the committed bytes ARE the pinned bytes.
— ASF header GUID verified at offset 0 by hexdump.

## 3. Findings
None. Checked: type matches claim; size 6,936 matches the FPO pin (tests.rs:1059
`fpo: (6_936, 2, 3_100, 3_200)`); regeneration command recorded
(docs/plans/asf-selfbuild.md:70 — the exact ffmpeg lavfi/sine/adpcm_ms invocation);
sha pinned; both consuming tests executed green.

## 4. Usage map
- asf/tests.rs:1052 — FixtureTruth row (structural pins above).
- asf/tests.rs:1128, :1333 — the two executed suite drivers.
- docs/plans/asf-selfbuild.md:70,184,460 — provenance (ffmpeg command, wfx tag notes,
  "decodable audio (ADPCM)" verdict row).
