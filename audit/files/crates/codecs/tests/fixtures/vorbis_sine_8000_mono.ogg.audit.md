# AUDIT: crates/codecs/tests/fixtures/vorbis_sine_8000_mono.ogg (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| File verdict | REAL (genuine 8 kHz mono Vorbis Ogg; every filename claim verified; pinned by a passing ffmpeg-reference bitexact suite) |
| Reachable from shipped entry point? | N/A (test fixture; ogg+vorbis is a CLI-reachable lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `ffprobe`, python Ogg packet census, rg + od citation checks; baseline cite |

## 1. One-paragraph verdict
A real, truthfully-named asset. `file` reports "Ogg data, Vorbis audio, mono, 8000 Hz, ~19600 bps"; live ffprobe confirms `codec_name=vorbis, sample_rate=8000, channels=1, duration=1.2` — mono, 8000, sine (the generation command is recorded in tests/data/vorbis_reference.txt:11: ffmpeg `-ac 1 -c:a libvorbis -q:a 2` — external-author evidence, not self-authored bytes). My python Ogg census: 4 pages, 42 packets = 3 headers (id/comment/setup) + 39 data packets, consistent with 1.2 s at 8 kHz (≈9600 samples over ~256-sample blocks). It is consumed by three suites: `vorbis_bitexact.rs` (`fixtures_are_pinned_against_the_ffmpeg_reference`, baseline 5/5 ok in 0.54s — real decodes against the recorded reference section `[fixture.vorbis_sine_8000_mono]` at vorbis_reference.txt:83), `vorbis_fuzz_survival.rs` (one of the four mutation bases — subject to that suite's coverage caveats, my file-17 audit), and the head-trim test at :342.

## 2. What actually works (verified)
- `file` + `ffprobe` outputs above (pasted in run log) — 100% name agreement.
- Python packet census: 4 pages / 39 data packets / geometry consistent with the probed duration and rate.
- Baseline :1358-1369: vorbis_bitexact 5/5 ok (0.54s) including `fixtures_are_pinned_against_the_ffmpeg_reference` and `stream_head_encoder_delay_is_trimmed_decoder_side` (this fixture is one of the two head-trim lanes at :342).
- Reference provenance: vorbis_reference.txt records the exact ffmpeg authoring command and per-fixture digest sections.

## 3. Findings
None. Format, rate, channel count, duration, and packet geometry all match; digest row recorded and exercised in the baseline.

## 4-9. (binary asset — short-form)
- Cited as evidence by: vorbis_bitexact.rs (bitexact pins + head-trim), vorbis_fuzz_survival.rs (mutation base). Honest uses.
- Untouched on disk; no direct matrix-row citation.
- Cross-note: this fixture is also the corpus-independent fallback family for vorbis coverage — the only vorbis differential evidence that runs without /tmp/corpus.
