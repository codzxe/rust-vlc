# AUDIT: crates/demux/src/asf/fixtures/asf_mjpeg.asf (binary)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Type / size / sha256 | `Microsoft ASF` (file) / 26,321 bytes / `1454c0a3a62b640e60ddf2f56a8fab53048210cdc023be3b1753e7946331dd61` |
| Fixture verdict | REAL (genuine, sha-pinned, exercised live) — but its demuxer is unreachable from shipped entry points |
| Cited as evidence by | docs/feature-matrix.toml:216 (row `container-asf-wmv`: "ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe codec + -count_packets cross-check"); docs/plans/asf-selfbuild.md:456 (26,321 B, hash `1454c0a3a62b640e`, "decodable video"); asf/tests.rs:990-1001 (FixtureTruth) + :1491 (sha pin) |
| Commands run | `file asf_mjpeg.asf` (Microsoft ASF); `stat`/`sha256sum` (values above); `cargo test -p demux --lib fixture_family` → 2/2 ok incl. live ffprobe cross-check (1.21s); python byte-scan (header GUID @0 ✓, DATA ✓, SIMPLE_INDEX ✓, SP@388 type=video) |

## 1. Blunt verdict
Genuine ffmpeg-generated ASF file (testsrc 64x48@10fps, 1s, mjpeg 200k —
tests.rs:991), byte-stable and sha256-pinned by the test that consumes it
(tests.rs:1491 pin matches my independently computed hash). The
FixtureTruth row (tests.rs:990-1001: header 585, 5 children, 1 mjpeg video
track, data (585, 25,650, 8), SI (26,235, 86), video (64,48,"MJPG",24))
was verified live by `fixture_family_walks_match_the_measured_truth` and
cross-checked against live ffprobe — both executed and passed in this run.
Hostile note: the matrix row that cites this fixture family as evidence
carries status windows=implemented/macos=implemented while its own notes
(feature-matrix.toml:217) record that the player's open path REFUSES asf
files (`probe.no-demuxer`, missing probe signature + registry arm) — the
gap is platform-independent, so the windows/macos "implemented" statuses
contradict the recorded linux-native measurement; the fixture proves the
demuxer works in unit tests, not that a user can open it.

## 2. Provenance
- Generation command recorded in-tree: tests.rs:991 `-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v mjpeg -b:v 200k` (mirrors docs/plans/asf-selfbuild.md:61).
- Integrity: sha256 prefix pin `1454c0a3a62b640e` (tests.rs:1491) == my `sha256sum` output. Regeneration fallback (fixture_dir/generate, tests.rs:1083-1119) only runs if the in-tree dir is absent.

## 3. Usage
- `fixture_family_walks_match_the_measured_truth` + `fixture_family_cross_checks_against_live_ffprobe` (asf/tests.rs:1128, 1325) — both ran OK in this session.
- mjpeg is the repo's first-vertical decodable codec; this fixture is the only asf-family file exercising a decodable video payload end-to-end in tests.
