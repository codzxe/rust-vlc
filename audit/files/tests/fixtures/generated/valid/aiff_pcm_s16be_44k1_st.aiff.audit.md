# AUDIT: tests/fixtures/generated/valid/aiff_pcm_s16be_44k1_st.aiff
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 3165 (binary; 35,334 bytes) / n/a |
| File verdict | REAL (data fixture, ffmpeg-authored class-A parent) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/md5 (365bdd18e7324c9a…, md5 4b788ab988d36002e861ef91fa213af4); header parse; rg consumers |

## 1. One-paragraph verdict
Genuine AIFF container: `FORM` size 0x000089fe = 35,326 = filesize-8
(self-consistent), `AIFF` form type, `COMM` at offset 12 declaring 2 channels,
8,820 sample frames (0x2274), 16-bit big-endian PCM, `SSND` chunk at 38.
8,820 frames × 2ch × 2B = 35,280 B payload + 54 B headers = 35,334 B — the
file is exactly its declared content, no slack. ffmpeg-authored per
docs/formats/container-aiff.md:36-38 and consumed by the demux differential
suite with ffmpeg-measured truth constants.

## 2. What actually works (verified)
— python3 header walk: FORM/AIFF/COMM/SSND offsets and COMM fields (2ch,
8820 frames, 16 bits) match the filename's claim (`pcm_s16be_44k1_st`).
— Consumer: crates/demux/tests/wav_aiff_fixtures.rs:126 registers the
fixture with ffprobe-measured truth (codec/rate/channels/frames/md5) and the
suite passes in the committed baseline
(audit/runs/baseline/cargo-test-workspace.txt — per-package demux suite ok).
— crates/media-probe/tests/registry_roundtrip.rs:86 uses it for probe
round-trip evidence.
— docs/formats/container-aiff.md:60-61: "demux suite PASS (163 unit + 8
wav_aiff_fixtures ...)".

## 3. Findings
F-001 [S3]
Where: tests/fixtures/CORPUS.md (manifest gap, same as the trunc4 audit)
What is wrong: the corpus manifest documents the avi/mp4/xtask families but
never lists the AIFF family; no committed tool regenerates this parent
(it is ffmpeg-authored, so at minimum the ffmpeg invocation/pins belong in
CORPUS.md). Why it matters: provenance of a differential-truth fixture rests
on an unrepeatable "this session" claim (wav_aiff_fixtures.rs:48-49).
What correct looks like: CORPUS.md row with ffmpeg version + command.
Confidence: HIGH — Verification: rg "aiff" tests/fixtures/CORPUS.md → no hits.

## 4. Matrix evidence
docs/corpus/compat-matrix-20261110.json cites it (rg hit) as the aiff lane;
scripts/quality coverage sweeps count ".aiff" extensions against it.
