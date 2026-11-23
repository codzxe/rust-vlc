# AUDIT: crates/codecs/tests/fixtures/mpeg2_24k.mp3 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A07) |
| Date | 2026-11-23 |
| Size / type | 6,476 bytes — `Audio file with ID3 version 2.4.0, contains: MPEG ADTS, layer III, v2, 64 kbps, 24 kHz, Stereo` (MPEG-2 LSF, not 2.5) |
| md5 / sha256 | 5c52ef9357da850e25dc8b08ba0a53d6 / f9e387c2b3fdddb69e23725bfe1c938a28c6e300798c3c7232d2737058ab92d7 |
| Structure (my parse) | ID3v2.4 tag exactly 34 B (audio at 44); first frame header 0xFFF38400 = MPEG-2 / Layer III / 24 kHz LSF / stereo |
| ffprobe facts | mp3, 24000 Hz, stereo, 65 packets/frames |
| File verdict | REAL — name-faithful MPEG-2 LSF Layer III 24 kHz fixture; every pinned reference fact reproduced live; doubles as the media-probe tag-skip anchor |
| Reachable from shipped entry point? | N/A (test fixture; consumers below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe` (+`-count_packets`); `ffmpeg -f md5` + `-f s16le \| wc -c`; python3 ID3/frame-header parse; suites `mp3_bitexact` 2/2, `mp3_corpus_differential` 2/2, `mp3_fuzz_survival` 2/2, `media_probe` 80/80 (this session); baseline cite for the registry_roundtrip lane |

## 1. One-paragraph verdict
Genuine, name-faithful fixture: ID3v2.4-tagged MPEG-2 LSF (low-sampling-frequency) Layer III, stereo, 24 kHz — my byte parse confirms the 34-byte tag (the exact shape media-probe's es.rs documents as its tag-skip anchor, es.rs:399, registry_roundtrip.rs:166-174) and the first frame header 0xFFF38400 (MPEG-2/L3/24 kHz/stereo). Every pinned reference fact reproduced live with the sandbox ffmpeg 7.1.5: PCM MD5 `bba85e8d4e9c1d0b6eb3f0dae13a37c9` == `[fixture.mpeg2_24k] ffmpeg_pcm_md5` (mp3_bitexact_reference.txt:77), 144,000 s16le bytes == 36,000 samples/channel × 2 ch (:76), 65 audio frames == `frames = 65` (:75). All consuming suites ran green on these exact bytes.

## 2. What actually works (verified)
- `ffmpeg -i mpeg2_24k.mp3 -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=bba85e8d4e9c1d0b6eb3f0dae13a37c9` — byte-exact vs the pin.
- 144,000 bytes = 36,000 stereo samples; `-count_packets` → 65 — both match the reference section (the LSF window: 65 × 576-sample frames minus LAME delay/padding, documented at reference :24-39).
- `mp3_bitexact` 2/2 (`fixtures_decode_the_recorded_window_exactly` iterates this fixture's pins incl. lumen MD5 `c808aa5c0f0af3eba1ab9401840bd0fa`, max delta ≤ 1); `mp3_corpus_differential` 2/2 (±1 differential, this row published as `58 (0.0806%, ±1)`); `mp3_fuzz_survival` 2/2 — all executed this session.
- `media_probe` lib suite 80/80; the probe lane using this fixture (`every_landed_container_probes_to_its_format_id`) is `ok` at baseline cargo-test-workspace.txt:3695 (its test binary absent from target/ now — baseline cited per spec §2).
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixtures read fresh from disk at runtime).

## 3. Findings
None. Checked: tag version and exact 34-byte size vs the es.rs anchor comment, LSF version/rate vs name, codec facts, decode cleanliness, pinned MD5/sample-count/frame-count vs live reproduction, all five consumers executed or baseline-cited, doc citations, ±1-tolerance disclosure honesty.

## 4. Doc citations
- docs/formats/audio-mp3-mp2.md:61 — row 6: "fixture mpeg2_24k.mp3 | MPEG-2 LSF Layer III stereo | 24000/2 | 36000 | 58 (0.0806%, ±1) | `c808aa5c0f0af3eba1ab9401840bd0fa` | `bba85e8d4e9c1d0b6eb3f0dae13a37c9`" — facts reproduced live; lumen MD5 + delta held by the executed suites.
- docs/worklog/archive-2026-10.md:10180 — the measurement record with the same lane numbers.
- crates/codecs/tests/data/mp3_bitexact_reference.txt:71-79 — the machine-read pin section, reproduced field-for-field.
- crates/media-probe/src/signature/es.rs:399 + tests/registry_roundtrip.rs:166-174 — the tag-skip anchor role (34-byte tag verified in the bytes).

## 5-9. (binary asset — tables N/A; consuming suites' pins hold; no dead code; no deps; no open questions)
