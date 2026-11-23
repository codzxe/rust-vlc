# AUDIT: crates/demux/fixtures/ps/ps-mp2-video-audio.mpg (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Type / size | `MPEG sequence, v1, system multiplex` (`file`) — real MPEG-PS (v1 packs, no nav); 75,776 bytes |
| SHA-256 | `30f351e446cb97f81e5a0fd30dc21064f56859934b39d6cfd076f7d8a9281e58` |
| File verdict | REAL (genuine fixture, differential evidence + seek-path input) |
| Reachable from shipped entry point? | N/A (test asset); consumers: crates/demux/src/ps/tests.rs:499,524; crates/demux/tests/ps_fixtures.rs:84-99; apps/cli/src/containers/ps.rs:765,853,1014 (incl. 600_000µs seek variant); apps/cli/tests/unsupported_path.rs:128 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (exit 0); `sha256sum` (exit 0); `od -A x -t x1z \| head` (exit 0); `stat -c%s` (exit 0); rg usage map |

## 1. One-paragraph verdict
Genuine MPEG-PS fixture: pack header `00 00 01 ba` at 0, system header `00 00 01 bb`,
video PES `00 00 01 e0` — real layout, not a stub. It is the workhorse of the PS
differential suite (ffmpeg bit-exact md5 pins for both lanes) and the CLI's PS open and
seek tests.

## 2. What actually works (verified)
— `file` -> "MPEG sequence, v1, system multiplex"; head hexdump shows
`00 00 01 ba / 00 00 01 bb / 00 00 01 e0`.
— Pinned evidence: ps_fixtures.rs:84-99 lane — ours [32,5,0] vs ffprobe [25,39,0],
  digests `b235981565a1b5f08783097bb9f40627` / `903e70465aecd25c6e5cf6c9f38b3daf`,
  bytes [63_898, 8_150, 0] == ffmpeg_bytes (bit-exact `-c copy` match,
  docs/formats/container-mpegps.md:50).

## 3. Findings
None. Checked: type/size consistent with claims; under the 200 KiB suite budget
(ps/tests.rs:501); used across three test targets (demux lib, demux integration, CLI) —
no orphaned-asset risk.

## 4. Usage map
- crates/demux/src/ps/tests.rs: fuzz-lite mutations + clean-drain contract.
- crates/demux/tests/ps_fixtures.rs: stream tables, counts, bit-exact payload md5s.
- apps/cli/src/containers/ps.rs: open test (765), lane test (853), seek test at 600_000µs
  (1014) — the seek path exercises this fixture end-to-end through the CLI host.
- apps/cli/tests/unsupported_path.rs:128 — unsupported-codec path input.
- docs/formats/video-mpeg12.md:63 — decoder seam provenance.
