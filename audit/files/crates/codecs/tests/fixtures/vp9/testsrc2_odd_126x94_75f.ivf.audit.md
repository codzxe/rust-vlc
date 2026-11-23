# AUDIT: crates/codecs/tests/fixtures/vp9/testsrc2_odd_126x94_75f.ivf (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Type / size | `file` → "Duck IVF video file, codec VP90~, 126x94, 75 frames"; 44,969 bytes |
| Cited by | crates/codecs/tests/vp9_differential.rs:79 (lane C) |
| Verdict | REAL — committed fixture, bit-exact differential lane executed live |

## 1. Verdict
Genuine IVF/VP9 stream, deliberately odd-geometry (126×94 — not
macroblock-aligned: 126 = 8×15+6, 94 = 8×11+6), 75 frames at 25 fps.
Header bytes verified: `44 4b 49 46 00 00 20 00 56 50 39 30 7e 00 5e 00
19 00 00 00 4b ...` → DKIF, header-length 32, FourCC VP90, width 0x007E
= 126, height 0x005E = 94, rate 25, frames 0x4B = 75 — matching the
differential lane's declared metadata exactly.

## 2. What actually works (verified)
— verified by live run of the prebuilt vp9_differential binary (exit 0):
`testsrc2_odd_126x94_75f: 75 frames 126x94, stream MD5
5d879407bc72e7ea105bc214c64c2256 == record, 75/75 frames identical` —
the fixture decodes bit-exactly against the recorded ffmpeg/libvpx
reference digest, every frame identical, no skip path.

## 3. Findings
None. The lane's claims (frame count, geometry, MD5) are all asserted by
the consuming test and reproduced live.
