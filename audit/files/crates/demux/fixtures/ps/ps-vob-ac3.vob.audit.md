# AUDIT: crates/demux/fixtures/ps/ps-vob-ac3.vob (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Type / size | `file` -> "MPEG sequence, v2, program multiplex"; 157,696 bytes |
| File verdict | REAL (valid, non-empty, actively used fixture) |
| Reachable from shipped entry point? | N/A (test asset) — consumed by demux test lanes below |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> MPEG PS v2; `ffprobe -show_entries ...` -> dvd_nav_packet(0x1bf) + mpeg2video(0x1e0) + ac3(0x80), duration 1.006322 s; `wc -c` -> 157696 |

## 1. One-paragraph verdict
A genuine MPEG-PS VOB fixture: ffprobe decodes it as a 3-stream program
(DVD nav data 0x1bf, MPEG-2 video 0x1e0, AC-3 audio 0x80), ~1.0 s, 157,696
bytes — not a placeholder or random bytes. It is pinned as evidence in two
committed test surfaces: crates/demux/tests/ps_fixtures.rs:102-121 (a lane
row with stream ids, packet counts ours [4,68,7] vs ffprobe [0,25,32], MD5
digests, byte counts, PTS spans) and crates/demux/src/ps/tests.rs:499,525
(fuzz-lite mutation + clean-drain contracts). No defect found in the asset.

## 2. What actually works (verified)
— verified by `ffprobe -v error -show_entries format,stream` -> 3 streams
exactly as the ps_fixtures.rs lane claims (dvd_nav_packet/mpeg2video/ac3),
exit 0. The consuming test binary `ps_fixtures` passes in baseline
(audit/runs/baseline/cargo-test-workspace.txt, demux section).

## 3. Findings
None for the asset itself. Checked: type (file), size, stream composition
vs the pinned lane (matches: 3 streams, same codecs/ids), citation surface
(2 files, both live), digest presence in the lane row (ps_fixtures.rs:110-113
carries MD5s for video/audio tracks).

## 4-9. (short binary-asset audit; sections 4-9 not applicable)
Note for next agent: the lane row honestly documents packet-count and
last-PTS divergences from ffprobe with reasons (comment at
ps_fixtures.rs:114-120) — audit the *reasons* when auditing ps_fixtures.rs;
the fixture itself carries no claims.
