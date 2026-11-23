# AUDIT: crates/demux/src/asf/fixtures/asf_msmpeg4v3.asf (binary)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Size / type | 7,129 bytes — `file`: "Microsoft ASF" (verified) |
| File verdict | REAL (genuine ffmpeg-authored legacy-fourcc fixture, digest-pinned in two places, consumed by the green asf suite) |
| Reachable from shipped entry point? | Test fixture (crates/demux/src/asf/tests.rs:976); the ASF demuxer itself is UNREACHABLE from shipped entry points (see data.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe` (below); python3 sha256; digest-map and doc-table cross-check |

## 1. Verdict
Fully verified fixture with a double-pinned provenance chain: the committed bytes hash to sha256 `66201b9205f26a90...`, whose 16-hex prefix is pinned in BOTH the asf digest map (crates/demux/src/asf/tests.rs:1490 `"asf_msmpeg4v3.asf" => "66201b9205f26a90"`) and the plan doc's fixture table (docs/plans/asf-selfbuild.md:455: `| asf_msmpeg4v3.asf | 7,129 | 66201b9205f26a90 | legacy fourcc (Lead re-pin 6.W4: regeneration deterministic, verified twice; ...) |`). Independent ffprobe on the committed bytes: format asf, one video stream, `msmpeg4v3`, 64×48, 1.000 s — matching the pinned `FixtureTruth` at tests.rs:975-986 (fpo total 7,129; header 593; data (593, 6_450, 2); video (64, 48, "MP43", 24, 9_216, 0); ffprobe_codecs ["msmpeg4v3"]). The generation command is recorded verbatim in both the truth (`-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v msmpeg4v3 -b:v 100k`) and the plan doc (asf-selfbuild.md:60).

## 2. Evidence
- `file` -> "Microsoft ASF"; 7,129 bytes (matches doc table and fpo pin).
- ffprobe (this machine): `codec_name=msmpeg4v3, codec_type=video, width=64, height=48, format_name=asf, duration=1.000000` — every field matches the pinned truth.
- sha256 prefix `66201b9205f26a90` == digest map == doc table.
- The asf suite that walks this fixture passed in my run (36/36, incl. the live-ffprobe codec cross-check on this very file).

## 3. Notes
- The doc even records the fixture's maintenance history ("Lead re-pin 6.W4: regeneration deterministic, verified twice"). Nothing fabricated; zero findings.
- The MSMPEG-4 v3 (MP43) fourcc is outside the engine's codec vocabulary — this fixture's role is the demux-layer legacy-fourcc lane, exactly as documented.
