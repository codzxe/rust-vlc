# AUDIT: crates/codecs/tests/mp3_bitexact.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / functions | 129 / 2 tests + 1 const |
| File verdict | REAL — four-leg hard-assert bit-exact suite; executed 2/2 live WITH the external ffmpeg oracle leg running (no SKIPs) |
| Reachable from shipped entry point? | Test file: not shipped. Code under test IS shipped: `codecs::audio::mp3::decode_stream` is dispatched by the CLI's mp3 arm — apps/cli/src/pipeline_host.rs:261 `"mp3" \| "mp1" =>` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/mp3_bitexact-a1d379f06feffc22` -> 2 passed / 0 failed (exit 0, 0.99s); rerun `--nocapture` -> zero SKIP lines, i.e. the live-ffmpeg leg executed for all 6 fixtures (`which ffmpeg` -> /usr/bin/ffmpeg); rg citation checks (pipeline_host arm, reference sections, helper impls) |

## 1. One-paragraph verdict
The MP3/MP2 bit-exact suite does exactly what its header promises, four
legs per fixture, and I ran it live with all legs active: (1) rate/channels
vs recorded ffprobe facts; (2) walked audio-frame count and gapless-window
samples-per-channel vs the reference; (3) the exact s16le MD5 regression
pin; (4) live ffmpeg decode of the same window with an asserted equal
length and max-sample-delta <= the recorded ±1 LSB tolerance. Both tests
passed in 0.99s with ffmpeg on PATH, and the --nocapture rerun printed
zero "SKIP live-ffmpeg" lines — leg 4 genuinely executed for all six
fixtures (cbr128, vbr_q4, mono48, mpeg2_24k, mp2_48k, tagged_art — exactly
the six `[fixture.*]` sections in the reference file at :41-91). The
tag-neutrality test is not a tautology: it proves tagged_art's PCM
MD5-equals cbr128's AND that the ID3 parser actually surfaced the embedded
Picture frame (a behavioral assert on the tag path, not just byte skip).
The helper chain is deadlock-free by construction (ffmpeg reads a file,
`.output()` drains both pipes — unlike the aac stdin-bridge variant W06-A10
flagged), and the min-prefix weakness of `s16_delta_stats` is neutralized
by the explicit window-length assert at :91-95. The single defect is the
environment degradation shape of leg 4.

## 2. What actually works (verified)
— verified by `./target/debug/deps/mp3_bitexact-a1d379f06feffc22` ->
`test fixtures_decode_the_recorded_window_exactly ... ok`,
`test tagged_art_is_cbr128_behind_a_tag ... ok`, 2 passed / 0 failed,
exit 0; `--nocapture` shows no SKIP lines → live oracle leg ran.

## 3. Findings

F-001 [S4]
Where: crates/codecs/tests/mp3_bitexact.rs:103-105
Code:
```rust
        } else {
            eprintln!("SKIP live-ffmpeg tolerance for {name}: no ffmpeg binary");
        }
```
What is wrong: on an ffmpeg-less machine the ONLY leg that compares
against the external oracle degrades to an eprintln while the test still
reports "ok". The three remaining legs are hard asserts against recorded
values, so the pass is not vacuous — but a golden that was wrong from the
day it was recorded (decoder bug baked into `lumen_pcm_md5`) is detectable
only by leg 4, which cannot fail in that environment. The skip is loud and
the reference file documents the tolerance contract, so this is hygiene,
not fraud. Contrast: mp2_corpus_differential's ffmpeg-absent gate skips the
entire committed-fixture comparison (S4, W06-A05) — here 3 of 4 legs still
run.
Confidence: HIGH — Verification: read (branch) + ran (branch NOT taken
here; ffmpeg present).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FIXTURES` | 37-44 | REAL | - | 6 names == 6 reference sections 1:1 (verified) |
| `fixtures_decode_the_recorded_window_exactly` | 46-107 | REAL | S4 | 4-leg assert chain; leg-4 env degradation (F-001) |
| `tagged_art_is_cbr128_behind_a_tag` | 109-128 | REAL | - | tag-neutrality + ID3 Picture behavioral assert |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_decode_the_recorded_window_exactly | YES | one drifted sample breaks the MD5 pin (:82-86); a window-math change breaks frames/samples asserts (:69-79); an oracle-visible regression breaks the live delta assert (:98-102); decode failure panics loudly (:53) |
| tagged_art_is_cbr128_behind_a_tag | YES | tag-skip regression breaks MD5 equality (:115); tag-parser regression breaks the Picture-frame assert (:121-127) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "six committed fixtures decode against the recorded ffmpeg 7.1.5 reference" | :1-5 | REAL | 6 fixtures, 6 sections; reference header records provenance + exact commands |
| "the gapless-window sample count ... Xing/Info frame excluded, LAME delay/padding trimmed on the +529-shifted pipeline timeline" | :11-15 | REAL | reference-file header derives the window; samples asserts ran green live |
| "a lossy decoder's MD5 is not ffmpeg's, but it must never drift" | :16-18 | REAL | distinct lumen_pcm_md5 vs ffmpeg_pcm_md5 columns in the reference; both pinned |
| "±1 LSB" tolerance | :21 | REAL | live leg 4 green with recorded `max_sample_delta` tolerances |
| "The real-corpus rows (2 MP3s) live in mp3_corpus_differential.rs" | :23 | REAL | file exists (audited separately) |

## 7. Dead / duplicated / unreachable code
None. `s16_delta_stats`'s min-prefix shape (shared with aac helper) is
defused here by the length assert at :91-95.

## 8. Dependencies & flags
Test-only; tests/mp3/mod.rs helpers (OnceLock-cached reference parse,
loud unreachable! on missing sections), testkit::Must. No features.

## 9. Open questions for the next agent
- None blocking. (Corpus leg lives in mp3_corpus_differential.rs — its
  vacuous-skip shape was already flagged there by W06-A05.)
