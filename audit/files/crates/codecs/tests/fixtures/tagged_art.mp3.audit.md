# AUDIT: crates/codecs/tests/fixtures/tagged_art.mp3 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Size / type | 52 845 bytes; `file` -> "Audio file with ID3 version 2.3.0, contains: MPEG ADTS, layer III, v1, 128 kbps, 44.1 kHz, Stereo" |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture); consumed by mp3_bitexact.rs, mp3_corpus_differential.rs, mp3_fuzz_survival.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (ID3v2.3 + MP3 CBR128 stereo 44.1k); `md5sum` -> b668e86fbc04cd09be34c07babc18253; `ffprobe` -> duration 2.037551 s, TAG:title="Cover Art Carrier", TAG:artist=Lumen, TAG:encoder=Lavf61.7.103; python3 ID3 check -> `ID3\x03\x00\x00` header, APIC frame at offset 80; all three consuming suites executed from prebuilt binaries -> green |

## 1. One-paragraph verdict
A real, purpose-built MP3 fixture: ~19.8 KB ID3v2.3 tag carrying an APIC attached-picture frame in front of 78 frames of CBR-128 MPEG-1 Layer III stereo 44.1 kHz audio (2.04 s) — exactly the "tag + art before the first MPEG frame" shape its name promises (ffprobe + raw-byte checks confirm). It serves three suites, all executed green by me from the prebuilt binaries: `mp3_bitexact::tagged_art_is_cbr128_behind_a_tag` (1 passed — decode via demux path and `decode_stream`, both matching the pinned reference `fixture.tagged_art` = mp3/44100/2ch/78 frames in tests/data/mp3_bitexact_reference.txt:91+), `mp3_corpus_differential` (2 passed; tagged_art is a corpus lane at :121), and `mp3_fuzz_survival` (2 passed; a fuzz seed at :59). Baseline corroborates (:1257-1260).

## 2. What actually works (verified)
- Identity: ffmpeg-authored (Lavf61.7.103 encoder tag), title "Cover Art Carrier", artist "Lumen" — a deliberately minted carrier for the art, not scraped media.
- Tag geometry: ID3v2.3.0 header bytes verified raw; APIC frame present at offset 80 (the art the demuxer must skip to find frame sync).
- Audio geometry: `file` + ffprobe agree on layer III v1 / 128 kbps / 44.1 kHz / stereo / 2.0376 s; the pinned reference's 78 frames × 1152 samples / 44100 ≈ 2.037 s — consistent.
- Execution: three prebuilt-binary runs, all exit 0 (bitexact 1/1 filtered on tagged_art, corpus 2/2, fuzz-survival 2/2 in 12.97 s).

## 3. Findings
None. Checked: type (file/ffprobe/python3 raw bytes), art presence (APIC), audio geometry vs the pinned reference, all three consumers executed green, citations (docs/formats/audio-mp3-mp2.md, docs/worklog/archive-2026-10.md, docs/gates/logs/phase7-exit-test.log).

## 4-9. (binary asset — not applicable)
Oracle note: mp3_bitexact's reference (lumen_pcm_md5, max_sample_delta) is a pinned record like the other audio lanes; the differential corpus lane is the cross-check surface. Details carried in the three consuming tests' existing audits.
