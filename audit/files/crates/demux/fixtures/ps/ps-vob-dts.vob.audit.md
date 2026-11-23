# AUDIT: crates/demux/fixtures/ps/ps-vob-dts.vob (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Type / size | `MPEG sequence, v2, program multiplex` (`file`) — real MPEG-PS; 192,512 bytes |
| SHA-256 | `254a6261d7d0dcf9a901260378209e25952b1c73b27cff655333e1c0436c0571` |
| File verdict | REAL (genuine fixture, actively used as differential evidence) |
| Reachable from shipped entry point? | N/A (test asset); consumers: crates/demux/src/ps/tests.rs:492,526; crates/demux/tests/ps_fixtures.rs:127; apps/cli/src/containers/ps.rs:908 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (exit 0); `sha256sum` (exit 0); `od -A x -t x1z \| head` (exit 0); `stat -c%s` (exit 0) |

## 1. One-paragraph verdict
Genuine committed MPEG-PS/VOB fixture: byte 0 starts `00 00 01 ba` (pack header) followed
by `00 00 01 bb` (system header) and a private-stream-2 `00 00 01 bf` nav packet — a real
DVD-VOB layout, not a hand-crafted stub. It is loaded by three separate suites as
differential evidence against ffmpeg, with pinned per-lane packet counts and md5 digests.

## 2. What actually works (verified)
— `file` -> "MPEG sequence, v2, program multiplex"; `od` head shows the pack/system/nav
headers quoted above. Loader `fixture()` (ps/tests.rs:484-491) reads it from
`fixtures/ps/` via CARGO_MANIFEST_DIR — committed-asset path, reachable.

## 3. Findings
None. Checked: type matches claimed container; size under the suite's 200 KiB budget
assert (ps/tests.rs:501 `original.len() < 200 * 1024` — 192,512 < 204,800); content is
referenced as evidence in docs/formats/container-mpegps.md:52 ("dts 48 128 B md5
bit-exact") and docs/formats/audio-dts.md:76, and the pins live in executed tests
(ps_fixtures.rs:126-149 lane table: ours [4,68,24] vs ffprobe [0,25,94], dts digest
`de45412596ea9fc909b4553055e420b0`, bytes 48,128 == ffmpeg_bytes 48,128).

## 4. Usage map
- crates/demux/src/ps/tests.rs: fuzz_lite mutation rounds (120 rounds × 8 byte flips)
  and clean-drain contract test.
- crates/demux/tests/ps_fixtures.rs: stream tables, packet counts, bit-exact payload
  md5s vs ffmpeg extractions.
- apps/cli/src/containers/ps.rs:908: CLI-level lane test (3 tracks: dvd_nav_packet /
  mpeg2video / dts, 24 dts PES, last PTS 136,114) — decode refusal typed.
- docs/worklog/archive-2026-10.md:9166,9679: measured differential provenance
  (ffmpeg 7.1.5, dts 48,128 B md5 bit-exact).
