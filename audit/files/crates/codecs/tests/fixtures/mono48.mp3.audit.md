# AUDIT: crates/codecs/tests/fixtures/mono48.mp3 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A04) |
| Date | 2026-11-23 |
| Size / type | 12,564 bytes — `Audio file with ID3 version 2.3.0, contains: MPEG ADTS, layer III, v1, 64 kbps, 48 kHz, Monaural` |
| md5 / sha256 | 2dd5776b4c124ad88afcb34878ab3645 / 45cadbb515ac40a94de6d5b5fb021812d0877a049980b16b3239f0b39d1abac7 |
| ffprobe facts | mp3, 48000 Hz, mono, 1.536 s, 64 kbps CBR, 64 packets/frames |
| File verdict | REAL — name-faithful MPEG-1 Layer III mono 48 kHz CBR fixture; every pinned reference fact reproduced live |
| Reachable from shipped entry point? | N/A (test fixture; consumers below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe`; `ffmpeg -f md5` + `-f s16le | wc -c` + `-count_packets`; `./target/debug/deps/mp3_bitexact-a1d379f06feffc22` → 2/2 (1.17s); `./target/debug/deps/mp3_corpus_differential-7377bbfd607a4401` → 2/2 (0.17s); `./target/debug/deps/mp3_fuzz_survival-a38f4eaafbfa5a92` → 2/2 (17.63s); `./target/debug/deps/media_probe-440f45029f776138` → 80/80; baseline cite for the registry_roundtrip lane |

## 1. One-paragraph verdict
Genuine, name-faithful MP3 fixture: ID3v2.3-tagged MPEG-1 Layer III, mono, 48 kHz, 64 kbps CBR (file/ffprobe agree). Every fact the in-tree reference pins for it I reproduced live with the sandbox ffmpeg 7.1.5-0+deb13u1 (the reference's own provenance version): PCM MD5 `204e24ea83aee0b18df7b041dc898b16` == `[fixture.mono48] ffmpeg_pcm_md5` (mp3_bitexact_reference.txt:67), 144,000 s16le bytes == 72,000 samples/channel × 1 ch (:66), 64 audio frames == `frames = 64` (:65). The consuming suites all ran green on these exact bytes, including the honest ±1-LSB lossy differential against live ffmpeg.

## 2. What actually works (verified)
- `ffmpeg -i mono48.mp3 -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=204e24ea83aee0b18df7b041dc898b16` — byte-exact vs the pin.
- `-f s16le | wc -c` → 144000 (72000 mono samples); `-count_packets` → 64 — both match the reference section.
- `mp3_bitexact` 2/2: `fixtures_decode_the_recorded_window_exactly` asserts this fixture's facts/window pins (frames, samples, ffmpeg PCM MD5, lumen window MD5 `95e0c8a02a5a2cce9c6175165fdbaaa6`, max delta ≤ 1).
- `mp3_corpus_differential` 2/2: `corpus_and_fixture_differential_vs_ffmpeg` — the live ffmpeg differential (mono48 row: 114/72000 samples differing, ±1 — matching docs/formats/audio-mp3-mp2.md:60's published `0.1583%, ±1`).
- `mp3_fuzz_survival` 2/2 (17.63s): mutated mono48 never panics, settles typed (gate lane; deep sweep #[ignore]d by design).
- media-probe: mono48 is a probe lane (`crate_fixture: Some("../codecs/tests/fixtures/mono48.mp3")`, expected "mp3", registry_roundtrip.rs:180-184); `every_landed_container_probes_to_its_format_id ... ok` at baseline cargo-test-workspace.txt:3695 (binary not present in target/ now — baseline cited per spec §2; the current 80-test media_probe lib binary ran 80/80 for what it covers).
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixtures read fresh from disk at runtime).

## 3. Findings
None. Checked: tag/container version, codec facts vs name, decode cleanliness, pinned MD5/sample-count/frame-count vs live reproduction, four consuming suites executed (three live + one baseline-cited), doc citations, ±1-tolerance disclosure honesty.

## 4. Doc citations
- docs/formats/audio-mp3-mp2.md:60 — row 5: "fixture mono48.mp3 | MPEG-1 Layer III mono | 48000/1 | 72000 | 114 (0.1583%, ±1) | `95e0c8a02a5a2cce9c6175165fdbaaa6` | `204e24ea83aee0b18df7b041dc898b16`" — every number re-verified here (ffmpeg MD5 reproduced live; lumen MD5 + delta held by the executed differential).
- docs/worklog/archive-2026-10.md:10180 — the measurement record with the same lane numbers.
- crates/codecs/tests/data/mp3_bitexact_reference.txt:61-69 — the machine-read pin section, reproduced field-for-field.

## 5-9. (binary asset — tables N/A; consuming suites' pins hold; no dead code; no deps; no open questions)
