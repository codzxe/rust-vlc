# AUDIT: fuzz/corpus/fuzz_demux_mp3stream/seed_mp3_vbr.mp3
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (VBR MP3 with ID3v2.4 tag) |
| Size / sha256-prefix | 9276 bytes / 0816a2ddca85d6b6 (verified locally) |
| File verdict | REAL — ffmpeg bitexact-authored VBR stream |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256, tag dump (ID3v2.4, TIT2 "Bitexact VBR...") |

## 1. Verdict
Real MP3 stream: `ID3\x04\x00\x00` header with TIT2 frame reading
"Bitexact VB..." — the ffmpeg `-bitexact` naming convention used by the
repo's deterministic fixture lane. VBR variant feeds fuzz_demux_mp3stream
(raw MP3 stream parser). Doubled in fuzz/corpus/fuzz_codec_mp3/ and
fuzz/corpus/fuzz_codec_mp2/ (same filename `seed_mp3_vbr.mp3` in all three
dirs — deliberate cross-format seeds). Presence gated (baseline
full_gate.txt:57-58 PASS).

## 2. What actually works (verified)
- First 32 bytes: `ID3\x04\x00\x00\x00\x00\x00:TIT2\x00\x00\x00\x0e\x00\x00\x03Bitexact VB` — verified locally.
- os.listdir: fuzz/corpus/fuzz_demux_mp3stream = [seed_mp3_48k.mp3, seed_mp3_mpeg2_24k.mp3, seed_mp3_vbr.mp3]; identical filename in fuzz_codec_mp3/ and fuzz_codec_mp2/ — verified.

## 3. Findings
None.
