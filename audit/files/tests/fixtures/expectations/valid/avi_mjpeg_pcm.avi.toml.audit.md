# AUDIT: tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 6,138 B / 9a0f9c15263a265f… (audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (expectation ledger; key digests independently recomputed and matched) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: `expectation/1` TOML for the class-A parent AVI (mjpeg 320x240 30fps +
pcm_s16le 44.1 kHz stereo, 2,000,000 µs): track plan, seek targets
[0, 500_000, 1_000_000, 1_999_000] with 33,333 µs tolerance, first_frame hash_y,
audio_probe first_samples_md5, and 60 per-sample essence sha256s.

Independent verification (python, no cargo): parent sha256/size match
(1f56202d7b113d38… / 425,516 — CORPUS.md row 62 verified earlier); chunk walk of
the parent found exactly 60 `00dc` video + 60 `01wb` audio chunks = matches
`video_sample_count = 60` and the 2 s @ 30 fps plan; **first video sample
sha256 recomputed = 35e3ec6c740edd122aa988b16629b3a5a663c68fbc54d24521e7dc0c0ffde03a ==
essence[0]** (verbatim match); **audio_probe first_samples_md5 recomputed by
concatenating all 01wb payloads (352,800 B = 2s·44100·2ch·2B), taking the first
4800 stereo instants, converting s16 -> f32 LE via v/32768.0, MD5 =
51882c74c405ecf7eb07f563ab97c87b == the committed value** (verbatim match). The
comment describes exactly this computation ("MD5 over the pinned f32 conversion
of the first 4800 sample-instants ... v/32768.0 as IEEE-754 f32, little-endian").

Consumers (rg, 8 hits): crates/codecs/tests/fixtures_parent.rs:28,
crates/codecs/tests/pcm_fixtures.rs:33, crates/demux/tests/fixtures_parent.rs:17,
crates/demux/tests/fixtures_class_b.rs:22, crates/demux/tests/adversarial_gaps.rs:55,
crates/media-probe/tests/fixtures_matrix.rs:247,
crates/playback-engine/tests/soak/scenario.rs:234,
crates/playback-engine/tests/soak_scenarios/runner.rs:1404,
crates/audio-output/tests/sine_sink_hash.rs:155. Generated-vs-handmade: generated
by `cargo xtask fixtures` (header line 1), drift-gated by `--check` per
tests/fixtures/CORPUS.md ("Ledger files ... verified by `--check` byte-comparison").
Matrix-evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl rows for the
AVI family derive from these fixtures. Zero defects found; the two pinned decode
digests are honest (would fail if the parent bytes drifted).
