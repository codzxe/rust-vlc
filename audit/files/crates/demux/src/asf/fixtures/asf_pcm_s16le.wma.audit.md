# AUDIT: crates/demux/src/asf/fixtures/asf_pcm_s16le.wma (binary)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Size / type | 26,106 bytes — `file`: "Microsoft ASF" (verified) |
| File verdict | REAL (genuine ffmpeg-authored ASF fixture, digest-pinned, consumed by the green asf suite) |
| Reachable from shipped entry point? | Fixture consumed by crates/demux/src/asf/tests.rs (test-only); the ASF demuxer itself is UNREACHABLE from shipped entry points (see data.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (Microsoft ASF); `ffprobe -show_entries format,stream` -> asf / pcm_s16le / 8000 Hz / mono / 1.000 s; `sha256sum`-equivalent python3 -> `29c7a469c3627a6f2d74...`; asf suite run (36 passed) uses this fixture |

## 1. Verdict
A real, verified ASF fixture with a complete provenance chain: the committed bytes hash to sha256 `29c7a469c3627a6f...`, whose 16-hex prefix is pinned in the asf digest map (crates/demux/src/asf/tests.rs:1494 `"asf_pcm_s16le.wma" => "29c7a469c3627a6f"`) and in docs/plans/asf-selfbuild.md:459 (`| asf_pcm_s16le.wma | 26,106 | 29c7a469c3627a6f | decodable audio (PCM) |`). The generation command is recorded verbatim in both the test truth (tests.rs:1039) and the plan doc (asf-selfbuild.md:69: `-f lavfi -i "sine=frequency=440:duration=1:sample_rate=8000" -ac 1 -c:a pcm_s16le`). Independent ffprobe on the committed bytes agrees with the pinned truth: format asf, one audio stream, pcm_s16le, 8,000 Hz, mono, 1.000 s — matching the truth table's `fpo (26_106, ...)`, `data (456, 25_650, 8)`, `wfx (0x0001, 1, 8_000, 2, 16, 0)` (tests.rs:1037-1046).

## 2. Evidence
- `file` -> "Microsoft ASF"; size 26,106 matches the doc's table and the test truth.
- ffprobe (this machine, ffmpeg 7.1.5): `codec_name=pcm_s16le, sample_rate=8000, channels=1, format_name=asf, duration=1.000000` — every value matches the pinned `FixtureTruth`.
- The asf test suite that walks this fixture passed in my run (36/36, incl. the live-ffprobe cross-check that compares codec identity on this very file).
- No doc/matrix row outside the asf family cites it; docs/plans/asf-selfbuild.md is the citing doc and is internally consistent with the bytes.

## 3. Notes
- Nothing fabricated: bytes, digest pin, doc table, and independent probe all agree. Zero findings.
