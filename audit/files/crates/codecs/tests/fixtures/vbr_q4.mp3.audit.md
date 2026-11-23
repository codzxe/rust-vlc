# AUDIT: crates/codecs/tests/fixtures/vbr_q4.mp3 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A04) |
| Date | 2026-11-23 |
| Size / type | 9,276 bytes — `Audio file with ID3 version 2.4.0, contains: MPEG ADTS, layer III, v1, 64 kbps, 44.1 kHz, Stereo` (64 kbps = first-frame header; the stream is VBR, avg 36.4 kbps) |
| md5 / sha256 | 118ca544efaadde14f7286d514f80c65 / 0816a2ddca85d6b6ae4636c649497ae54de61526abaff2aade91348399323935 |
| ffprobe facts | mp3, 44100 Hz, stereo, 2.0376 s, format bit_rate 36420 (VBR), 78 packets/frames |
| File verdict | REAL — name-faithful LAME VBR-q4 fixture with genuine Xing header (TOC machinery exercised by the seek test) |
| Reachable from shipped entry point? | N/A (test fixture; consumers below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe` (+`-count_packets`); `ffmpeg -f md5` + `-f s16le \| wc -c`; python3 ID3/Xing/frame-header scan; suites `mp3_bitexact` 2/2, `mp3_corpus_differential` 2/2, `mp3_fuzz_survival` 2/2 (executed this session, see File 5 entries) |

## 1. One-paragraph verdict
Genuine, name-faithful VBR fixture: ID3v2.4 (58-byte tag) + MPEG-1 Layer III stereo 44.1 kHz, LAME VBR q4 — my byte scan finds the Xing header at offset 104 inside the first frame (the VBR/TOC machinery the name and the seek lane depend on), first frame header 0xFFFB5000 decodes to MPEG-1/L3/64 kbps/44100/stereo, and the container-average bitrate is genuinely variable (36,420 bps). Every pinned reference fact reproduced live with the sandbox ffmpeg 7.1.5: PCM MD5 `aa132c33436a959052f262e82d5d1e86` == `[fixture.vbr_q4] ffmpeg_pcm_md5` (mp3_bitexact_reference.txt:57), 352,800 s16le bytes == 88,200 samples/channel × 2 ch (:56), 78 audio frames == `frames = 78` (:55).

## 2. What actually works (verified)
- `ffmpeg -i vbr_q4.mp3 -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=aa132c33436a959052f262e82d5d1e86` — byte-exact vs the pin.
- 352,800 bytes = 88,200 stereo samples; `-count_packets` → 78 — both match the reference section; Xing frame excluded from the audio count per the reference's documented window (:24-39).
- Consuming suites executed green this session on these exact bytes: `mp3_bitexact` 2/2 (`fixtures_decode_the_recorded_window_exactly` asserts this fixture's full pin set incl. lumen MD5 `988161d1c81b5723c44ebab84fa2afb9`, max delta ≤ 1); `mp3_corpus_differential` 2/2 — including the fixture-specific `vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly` (the Xing TOC seek lane) and the ±1 differential (254/88200 differing == the published `0.1440%, ±1`); `mp3_fuzz_survival` 2/2 (mutated-fixture lane, 17.63s).
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixtures read fresh from disk at runtime).

## 3. Findings
None. Checked: tag/container version, VBR reality (Xing header + variable average bitrate vs name), codec facts, decode cleanliness, pinned MD5/sample-count/frame-count vs live reproduction, fixture-specific seek lane executed, doc citations, ±1-tolerance disclosure honesty.

## 4. Doc citations
- docs/formats/audio-mp3-mp2.md:59 — row 4: "fixture vbr_q4.mp3 | MPEG-1 Layer III LAME VBR q4 (Xing) | 44100/2 | 88200 | 254 (0.1440%, ±1) | `988161d1c81b5723c44ebab84fa2afb9` | `aa132c33436a959052f262e82d5d1e86`" — every number re-verified here (Xing presence confirmed in the bytes, ffmpeg MD5 reproduced live, lumen MD5 + delta held by the executed differential).
- docs/worklog/archive-2026-10.md:10180 — the measurement record with the same lane numbers.
- crates/codecs/tests/data/mp3_bitexact_reference.txt:51-59 — the machine-read pin section, reproduced field-for-field.

## 5-9. (binary asset — tables N/A; consuming suites' pins hold; no dead code; no deps; no open questions)
