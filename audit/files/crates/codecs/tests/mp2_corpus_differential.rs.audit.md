# AUDIT: crates/codecs/tests/mp2_corpus_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 221 / 3 helpers + 3 tests |
| File verdict | REAL live-oracle differential — ran live (ffmpeg present): ±1 LSB on both committed fixtures, matching the recorded doc table byte-for-byte |
| Reachable from shipped entry point? | N/A (integration test); exercises the SHIPPED `codecs::audio::mp2` (CLI "mp2" row, pipeline_host.rs:266) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/mp2_corpus_differential-9e9fc1613813b89c --nocapture` → 3 passed; 0 failed, exit 0 (2 corpus rows SKIP-observed, fixture rows ran live); `which ffmpeg` → /usr/bin/ffmpeg |

## 1. One-paragraph verdict
A real differential against a live ffmpeg oracle, and I ran it: both
committed fixtures decode sample-locked (equal byte length asserted) with
max delta exactly 1 LSB on ~24.5%/25.3% of samples — and the printed
MD5s/percentages match the recorded table in
docs/formats/audio-mp3-mp2.md:62 verbatim, so the doc's honest
"±1-LSB lossy tolerance, not bit-exact" posture is corroborated by fresh
execution. The pass bar (max_delta ≤ 1) is a real assertion that would
fail on decoder drift; the corpus rows carry a loud-fail MD5-drift check
by design. The two corpus-dependent rows and the seek pin SKIPped in this
environment (corpus-v4 absent — observed live), consistent with the
header's documented skip-with-message convention. One structural nit: the
ffmpeg-absent gate skips the fixture rows too.

## 2. What actually works (verified)
— verified by live run (exit 0):
`mp2_48k.mp2 rate=48000 ch=2 samples=72576 differing=35526 (24.4750%)
max_delta=1 lumen_md5=6db7b51b… ffmpeg_md5=bf3bee58…` and
`mp2_lsf_22k.mp2 … differing=18098 (25.3388%) max_delta=1 …` — identical
numbers and MD5s to docs/formats/audio-mp3-mp2.md:62. The LSF geometry
test (22 050 Hz, 2 ch, 96 kbps, V2, samples = frames×1152×2) passed
no-skip.

## 3. Findings

F-001 [S4]
Where: crates/codecs/tests/mp2_corpus_differential.rs:80-84 (and :114-116)
Code:
```rust
fn corpus_and_fixture_differential_vs_ffmpeg() {
    if !ffmpeg_available() {
        eprintln!("SKIP mp2 corpus differential: no ffmpeg binary on PATH");
        return;
    }
```
What is wrong: the ffmpeg gate returns before the two committed-fixture
rows (:114-116), which need no corpus — only ffmpeg. On a machine without
ffmpeg the entire differential, including its always-committed evidence
lanes, passes without comparing a single sample. The skip is messaged and
documented (:19), but bundling the fixture rows behind the same early
return means "passed" is environment-dependent even for in-repo bytes.
What correct looks like: run the fixture rows whenever ffmpeg exists
(already true), and distinguish "0 rows compared" from "rows compared" in
the outcome (e.g., assert !rows.is_empty() after the gates).
Confidence: HIGH — Verification: read (ffmpeg present here, so rows ran;
the absent-ffmpeg path is traced, not executed — UNVERIFIED by execution).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CORPUS_MP2 table | 35-38 | REAL | - | recorded ffmpeg MD5s (2026-10-24, ffmpeg 7.1.5) |
| run_row | 52-77 | REAL | - | equal-length assert + delta stats + dual MD5s |
| corpus_and_fixture_differential_vs_ffmpeg | 79-146 | REAL | S4 | ±1 LSB bar asserted; corpus MD5-drift loud-fail; F-001 gate shape |
| cbr_seek_on_the_padded_corpus_stream_refines_exactly | 154-197 | REAL (SKIP here) | - | corpus-gated; refine-snap-forward arithmetic + <64B slack asserts |
| the_generated_lsf_fixture_walks_and_decodes_with_declared_geometry | 202-221 | REAL | - | no-skip; ran live; full geometry pin |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_and_fixture_differential_vs_ffmpeg | YES (with ffmpeg+corpus) | window equality, ±1 bar, MD5 drift all asserted; WITHOUT ffmpeg: NO (F-001) |
| cbr_seek... | YES (with corpus) | exact snap + slack bounds; skipped here |
| the_generated_lsf_fixture... | YES | hard asserts, loud fail (unreachable! on decode error) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded BOTH by Lumen and by the external ffmpeg ... compared on the identical frames × 1152 window" | :1-4 | REAL | live run, equal-length assert |
| "if the corpus was re-imaged, the row FAILS loudly" | :15-17 | REAL (design) | :103-106 assert_eq on MD5; not executable here (corpus absent) |
| mp2_48k ±1 24.475% MD5 6db7b51b…/bf3bee58… | docs/formats/audio-mp3-mp2.md:62 | REAL | my live output matches exactly |
| "8/8 lanes inside the ±1-LSB lossy tolerance" | audio-mp3-mp2.md:65 | REAL-UNPROVEN | 2 of 8 lanes re-executed here (mp2 fixtures); MP3 lanes + corpus lanes UNVERIFIED (corpus absent; mp3 suite not in my assignment) |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
ffmpeg/ffprobe on PATH (oracle), testkit Must, common/md5 helpers,
tests/mp2.rs module. No feature gates.
## 9. Open questions for the next agent
- tests/mp2.rs helper module (not assigned): does `ffmpeg_decode_s16le`
  use `-f s16le -acodec pcm_s16le` (i.e., decode-only, no resample)?
  A resampling oracle would fake the ±1 comparison.
