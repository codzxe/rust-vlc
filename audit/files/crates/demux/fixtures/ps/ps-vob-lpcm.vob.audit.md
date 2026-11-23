# AUDIT: crates/demux/fixtures/ps/ps-vob-lpcm.vob (binary asset, 16 assigned parts -> one audit)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Type / size / sha256 | MPEG sequence v2 program multiplex; 241,664 B; 4366d145275b69f5806fd226456725fe2554e9773f2f41dee367eeba4dd387fc |
| File verdict | REAL (fixture content independently re-parsed; matches every pin that cites it) |
| Reachable from shipped entry point? | YES as test evidence — loaded by crates/demux/tests/ps_fixtures.rs:148 and crates/demux/src/ps/tests.rs:527 (via fixture()/fixture_path), also apps/cli/src/containers/ps.rs:909 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (MPEG sequence, v2, program multiplex); `sha256sum`; `od -A d -t x1 -N 64`; python3 byte-scan of start codes/PTS/payloads (exit 0 each) |

## 1. One-paragraph verdict
Genuine MPEG-PS/VOB binary whose structure I re-parsed byte-by-byte with independent
python3 code: 118 pack headers (00 00 01 BA), 2 system headers, 68 video PES (0xE0),
4 private-2 nav PES (0xBF), 48 private-1 PES (0xBD) all carrying DVD-LPCM substream
0xA0. Every quantitative pin that cites this fixture in the repo — 4nav+68v+48lpcm,
first PTS 48,754, last PTS 138,583, and the 96,000 B pure-sample payload — is
reproduced exactly by my independent scan. No fabrication.

## 2. What actually works (verified)
- `python3` scan -> pack/BD/E0/BF counts 118/48/68/4; substream ids {0xA0: 48}; BD PTS
  first=48754 last=138583; per-BD pure-sample payload totals 96,000 B.
- Pins matched: crates/demux/tests/ps_fixtures.rs:147-170 (`ours: [4, 68, 48]`,
  `first_pts ... Some(48_754)`, `last_pts ... Some(138_583)`, `bytes ... 96_000`,
  `ffmpeg_bytes ... 96_144` = 48×(3+2000) delta claim — arithmetic checks:
  96,144−96,000 = 144 = 3×48, and my payload total is exactly 96,000).
- apps/cli/src/containers/ps.rs:909 pin `("ps-vob-lpcm.vob", "pcm_dvd", 48, 138_583)`
  matches the byte-level reality.
- docs/formats/container-mpegps.md:53 row "| ps-vob-lpcm.vob | nav+mpeg2video+pcm_dvd,
  0+25+48 | 4nav+68v+48lpcm | ours 96 000 B ... ffmpeg 96 144 = +3 B × 48 |" —
  consistent with my scan (the "25" is ffmpeg's AU re-packetization, pinned not claimed
  as ours).

## 3. Findings
None. Checked: magic bytes (00 00 01 BA at offset 0), MPEG-2 pack syntax, stream-id
composition, LPCM substream range (0xA0-0xBF valid for DVD LPCM), PTS decodability,
payload-size arithmetic against all four citing test/doc sites, sha256 stability.
Provenance: ps_fixtures.rs:1-8 states the fixtures are "four ffmpeg-authored products"
authored and measured with ffmpeg 7.1.5 / Lavf61.7.103 — no in-repo generation script
exists (fixtures/ps/ contains only the 4 binaries; UNVERIFIED: I cannot re-derive the
ffmpeg authoring command from the repo, but the binary self-authenticates as MPEG-PS).

## 4. Per-function table
N/A — binary asset.

## 5. Test honesty
Tests using this fixture assert concrete byte counts, PTS values and (elsewhere) md5
digests against ffmpeg-measured truth; they would fail if the demuxer regressed or the
fixture were swapped. Not tautological.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| nav+mpeg2video+pcm_dvd; 4nav+68v+48lpcm; 96,000 B vs ffmpeg 96,144 | docs/formats/container-mpegps.md:53 | REAL | my independent byte scan (§2) |
| ours [4,68,48], first 48_754, last 138_583, bytes 96_000 | crates/demux/tests/ps_fixtures.rs:147-170 | REAL | my scan reproduces all four numbers |
| drains clean / no panic under fuzz-lite truncation | crates/demux/src/ps/tests.rs:520-536 | REAL-UNPROVEN | property is about demuxer code, not fixture; not executed (cargo-build-budget) |

## 7. Dead / duplicated / unreachable code
N/A.

## 8. Dependencies & flags
None; static test asset.

## 9. Open questions for the next agent
Whether the ps demuxer's 7-byte DVD private-1 prefix strip (96,000 B claim) is honored
on truncation boundaries is a src/ps question, not a fixture question.
