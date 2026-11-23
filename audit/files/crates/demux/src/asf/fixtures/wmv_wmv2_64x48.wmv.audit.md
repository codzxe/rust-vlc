# AUDIT: crates/demux/src/asf/fixtures/wmv_wmv2_64x48.wmv (binary)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Type / size / sha256 | `Microsoft ASF` (file) / 7,123 bytes / `f1803712b502c41aeef426ddd13f793d53ecdf79bd19af601901cf3ed01c9b5a` |
| Fixture verdict | REAL (genuine, sha-pinned, exercised live) — but its demuxer is unreachable from shipped entry points |
| Cited as evidence by | docs/feature-matrix.toml:216 (row `container-asf-wmv` evidence string, same fixture differential); docs/plans/asf-selfbuild.md:453 (7,123 B, hash `f1803712b502c41a`, "video-only + SI"); asf/tests.rs:948-959 (FixtureTruth) + :1488 (sha pin) |
| Commands run | `file wmv_wmv2_64x48.wmv` (Microsoft ASF); `stat`/`sha256sum` (values above); `cargo test -p demux --lib fixture_family` → 2/2 ok incl. live ffprobe cross-check; python byte-scan (header GUID @0 ✓, DATA ✓, SIMPLE_INDEX ✓, SP@388 type=video, EC field = `20FB5700-5B55-11CF-A8FD-00805F5C442B` — this file is the disproof of guid.rs's all-zero `EC_NONE` "observed" claim) |

## 1. Blunt verdict
Genuine ffmpeg-generated WMV2 file (testsrc 64x48@10fps, 1s, 100k —
tests.rs:949), sha256-pinned by the consuming test (tests.rs:1488 pin
matches my independently computed hash) and verified live by both
fixture-family tests in this session. Its FixtureTruth row (header 587,
5 children, 1 wmv2 video track, data (587, 6,450, 2), SI (7,037, 86),
video (64,48,"WMV2",24, 9,216, extradata 4)) passed against the real
bytes and live ffprobe. Same hostile note as the sibling fixture: the
matrix row citing this family is windows/macos "implemented" while its own
notes record the open path refusing asf files (`probe.no-demuxer`) — the
missing probe signature/registry arm is platform-independent, so those
statuses contradict the recorded linux-native measurement.

## 2. Provenance
- Generation command recorded in-tree: tests.rs:949 `-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v wmv2 -b:v 100k` (mirrors docs/plans/asf-selfbuild.md:58).
- Integrity: sha256 prefix pin `f1803712b502c41a` (tests.rs:1488) == my `sha256sum` output.

## 3. Usage
- `fixture_family_walks_match_the_measured_truth` + `fixture_family_cross_checks_against_live_ffprobe` (asf/tests.rs:1128, 1325) — both ran OK in this session.
- Secondary evidentiary role: this file's Stream Properties EC-type bytes are the executed disproof of the `EC_NONE` all-zero claim in this run's guid.rs audit (F-001 there).
- wmv2 decode is codec-blocked (matrix row codec-video-wmv-vc1, blocked_reason macos/web) — the fixture validates demux-layer packet walks only.
