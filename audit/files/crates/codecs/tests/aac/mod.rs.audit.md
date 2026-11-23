# AUDIT: crates/codecs/tests/aac/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 194 / 9 |
| File verdict | REAL (test tooling) with one latent deadlock bug in the ffmpeg bridge |
| Reachable from shipped entry point? | N/A test-support module (`tests/aac/mod.rs`) consumed by the aac_* suites; aac_bitexact 2/2, aac_fuzz_survival 2/2, aac_corpus_differential 1/1 (SKIP: corpus absent) all executed by me |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/aac_bitexact-1e99e0008b8875c2` → 2 passed (exit 0); `./target/debug/deps/aac_corpus_differential-7843d789557f43b7 --nocapture` → 1 passed, "SKIP aac corpus differential: corpus root absent" (exit 0); `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c` → 2 passed (exit 0); `which ffmpeg` → /usr/bin/ffmpeg |

## 1. One-paragraph verdict
Honest test tooling for the AAC differential lanes: the f32→s16 conversion implements ffmpeg's exact rule (×32768, round-ties-even, clip — verified against libswresample's av_clip_int16(lrintf(...)) semantics, including the 32767.5→32768→clip-32767 path), the delta reporter counts differing samples and max delta rather than asserting blind equality, and the corpus/ffmpeg gates SKIP with messages instead of passing silently on absent resources (I observed the corpus SKIP live). One genuine defect: the m4a ADTS bridge writes all input to ffmpeg's stdin before draining stdout — a classic two-pipe deadlock for any extracted stream larger than the stdin pipe buffer, which a full-song corpus file would be. Two hygiene notes (blanket dead-code allow; delta stats ignore stream-length mismatch).

## 2. What actually works (verified)
— verified by executing all three consuming suites (bitexact 2/2; fuzz survival 2/2; corpus differential SKIP-message observed verbatim) and by hand-checking `f32_to_s16` against ffmpeg's rule for the boundary values ±32767/±32768/32767.5.

## 3. Findings
F-001 [S3]
Where: aac/mod.rs:162-180
Code:
```
    let mut child = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-f", "aac", "-i", "-"])
```
(then L172-177) `child.stdin.as_mut()...write_all(bytes)`
What is wrong: two-pipe deadlock. `write_all` pushes the whole extracted ADTS into ffmpeg's stdin pipe while nobody drains ffmpeg's stdout; decoded PCM (~11× the input byte rate for stereo 44.1 kHz AAC) fills the 64 KiB stdout pipe, ffmpeg blocks on write, stops reading stdin, our `write_all` blocks forever. Any extracted stream larger than the stdin pipe buffer (~64 KiB — i.e., any real song) hangs the differential until the CI timeout. Safe only for inputs that fit the pipe. Currently latent: the only caller (aac_corpus_differential.rs:159, m4a row 0029) runs only when the corpus exists, and in this environment the corpus is absent (SKIP observed). What correct looks like: `Command::output()` with stdin pre-filled (spawn + drop stdin) or a reader thread. Confidence: HIGH on the mechanism — Verification: read; the deadlock itself UNVERIFIED (would require a >64 KiB corpus file; corpus absent — cargo-build-budget/corpus-absent). The recorded expectations prove the recording run completed, so the recorded 0029 file must have been small.
F-002 [S4]
Where: aac/mod.rs:84-98
Code:
```
    let n = a.len().min(b.len()) / 2;
```
What is wrong: `s16_delta_stats` compares only the common prefix — a length mismatch (dropped/extra samples) is invisible to the "honest tolerance report". Safe only if every caller separately asserts length equality; the callers are outside my assignment (aac_bitexact.rs / aac_corpus_differential.rs — UNVERIFIED whether they do). Confidence: MEDIUM — Verification: read.
F-003 [S4]
Where: aac/mod.rs:1
Code:
```
#![allow(dead_code)]
```
What is wrong: blanket dead-code allow on a shared module (same rot pattern as renderer-wgpu's common/mod.rs). Today's consumers use the exported surface; future dead helpers will be absorbed silently. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / fixture_bytes | 18-31 | REAL | — | CARGO_MANIFEST_DIR-relative; panic-on-absence documented as repo-state error |
| corpus_dir | 37-44 | REAL | — | env override + sandbox default; SKIP-aware (observed live) |
| f32_to_s16 | 50-60 | REAL | — | ffmpeg rule verified incl. clip boundaries; NaN→0 via saturating cast (ffmpeg lrintf(NaN) also 0 in practice) |
| to_s16le | 64-70 | REAL | — | interleaved LE |
| decode_s16le | 74-78 | REAL | — | decode_stream + convert |
| s16_delta_stats | 84-98 | REAL | F-002 | min-prefix compare; differing/max-delta |
| ffmpeg_available | 102-109 | REAL | — | version-probe gate |
| ffmpeg_decode_s16le / ffmpeg_extract_adts | 113-154 | REAL | — | `Command::output()` — deadlock-free (std drains both pipes); stream-copy keeps the comparison apples-to-apples (documented) |
| ffmpeg_decode_s16le_adts_bytes | 162-188 | REAL | F-001 | manual spawn + blocking write_all — deadlock for >pipe-sized inputs |
| corpus_media | 191-194 | REAL | — | SKIP-aware read |

## 5. Test honesty
No tests in this module (it is the tooling). Its consumers: aac_bitexact (2/2 pass — committed fixtures vs recorded ffmpeg digests), aac_fuzz_survival (2/2 — mutation sweeps over fuzz_base_lc.aac), aac_corpus_differential (SKIPs cleanly without corpus; passes vacuously HERE — its differential evidence is environment-dependent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "its own correctness is pinned by the recorded expectations ... never trusted blind" | aac/mod.rs:7-10 | REAL | bitexact suite compares against recorded ffmpeg digests (executed, pass) |
| "ffmpeg's own float→s16 rule ... scale by 32768, round-ties-to-even, then clip" | aac/mod.rs:46-48 | REAL | hand-verified boundary behavior |
| "-f aac forces the raw-ADTS demuxer ... the forced input is the documented differential shape" | aac/mod.rs:156-161 | REAL | honest bridge documentation; mechanism read |

## 7. Dead / duplicated / unreachable code
None beyond the F-003 allow.

## 8. Dependencies & flags
codecs::audio::aac (decode_stream), media-types; ffmpeg subprocess as oracle (present in sandbox). No unsafe.

## 9. Open questions for the next agent
- aac_corpus_differential.rs owner: confirm length equality is asserted before s16_delta_stats (F-002), and fix the bridge deadlock (F-001) before the corpus lane runs on a full-song m4a.
