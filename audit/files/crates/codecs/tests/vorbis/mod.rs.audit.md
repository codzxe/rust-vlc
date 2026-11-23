# AUDIT: crates/codecs/tests/vorbis/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 391 / 12 fns (shared test-support module, no tests of its own) |
| File verdict | REAL — every live helper exercised end-to-end by my runs of its 4 consumer test binaries |
| Reachable from shipped entry point? | NO — test-support module (included by tests/vorbis_{bitexact,bench,fuzz_survival,vlc_differential}.rs via `mod vorbis;` → tests/vorbis/mod.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` -> 5/5 ok (0.86s); `./target/debug/deps/vorbis_vlc_differential-3a9245a4339ccea0` -> 1/1 ok (VLC half PENDING-INFRA by design); rg caller analysis; sed verification |

## 1. One-paragraph verdict
Solid shared test surface, verified by execution rather than by reading alone: my
run of vorbis_bitexact (5/5) drives this module's fixture loader, reference
parser, full Ogg→codec-api decode driver, window derivation, f32 MD5
canonicalization, and live ffmpeg f32le oracle end-to-end against real
fixtures; the VLC-differential consumer runs its Lumen half for real and skips
its VLC half with an honest PENDING-INFRA record (no vlc binary in the sandbox —
the header says so and my `which` agrees via the test's own probe). Defects:
`window()` is a dead near-duplicate of `window_at()` hidden by a module-level
`#![allow(dead_code)]`; the decode driver picks the FIRST Ogg serial instead of
the vorbis track's serial (single-stream assumption); the `-0.0` byte-pattern
canonicalization is little-endian-only; and `ffmpeg_decode_f32le` maps ALL
audio streams (`-map 0:a`), which aborts on multi-audio files instead of
comparing the vorbis track.

## 2. What actually works (verified)
— `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` -> "5 passed; 0 failed"
including `fixtures_are_pinned_against_the_ffmpeg_reference ... ok` (the live
ffmpeg oracle ran — ffmpeg is on PATH here) and
`stream_head_encoder_delay_is_trimmed_decoder_side ... ok` (the window_at
derivation path).
— `./target/debug/deps/vorbis_vlc_differential-3a9245a4339ccea0` -> 1/1 ok
(Lumen half real, VLC half PENDING-VLC by explicit design).
— Module resolution confirmed: all four `tests/vorbis_*.rs` targets declare
`mod vorbis;`, resolving to this file (rg output in run log).

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/vorbis/mod.rs:308-326 (dead) vs 280-306 (live twin)
Code:
pub fn window(sink: &CaptureSink, channels: usize, ffmpeg_samples: usize) -> Vec<u8> {
    // Raw timeline: cumulative samples before each frame.
    let mut raw_pos = 0usize;
    let mut shift = 0i64;
    for (index, ticks) in sink.pts.iter().enumerate() {
        if *ticks != i64::MIN {
            // This frame's raw start is raw_pos; the container says
            // its valid start is ticks.
            shift = raw_pos as i64 - ticks;
            let _ = index;
            break;
        }
What is wrong: `window` has ZERO callers (rg `window\(` over all four consumer
files: no matches; all use `window_at`). It is a stale copy of the same
raw→valid-shift derivation, contains a dead `let _ = index;`, and survives only
because line 1 is `#![allow(dead_code)]`. Why it matters: two near-identical
timeline derivations invite fixing a bug in one and not the other — and the
dead one is already the un-fixed twin (no head_trim fallback, no doc updates).
What correct looks like: delete `window`.
Confidence: HIGH — Verification: rg caller analysis + read.

F-002 [S4]
Where: crates/codecs/tests/vorbis/mod.rs:177-180
Code:
    let serial = *demuxer
        .serial_numbers()
        .first()
        .unwrap_or_else(|| unreachable!("no serials"));
What is wrong: the vorbis TRACK is found by codec name (L172-175), but the
serial used to fetch its header packets is simply the FIRST serial in the file.
For a multi-logical-stream Ogg (vorbis + theora/kate, or chained streams) the
first serial may belong to another stream; `VorbisHeaders::from_packets` then
fails and the test panics via unreachable! (fail-loud, but the driver is
unusable for exactly the multi-stream corpus files whose reference sections are
named "audio-only"). What correct looks like: resolve the vorbis track's own
serial (the demuxer's track table should carry it).
Confidence: HIGH (code path) / UNVERIFIED (whether any committed fixture is
multi-stream — corpus absent).
F-003 [S4]
Where: crates/codecs/tests/vorbis/mod.rs:332-340
Code:
pub fn canonical_f32_md5(bytes: &[u8]) -> String {
    let mut canon = bytes.to_vec();
    for chunk in canon.as_chunks_mut::<4>().0 {
        if *chunk == [0, 0, 0, 0x80] {
            chunk.fill(0);
        }
    }
What is wrong: the -0.0 byte pattern `[0,0,0,0x80]` is the LITTLE-ENDIAN
spelling; on a big-endian host the canonicalization silently no-ops and every
pinned digest mismatches. Test-only, LE host documented elsewhere in the suite;
still an unacknowledged platform assumption. What correct looks like:
`f32::from_ne_bytes` + `== -0.0` test, or a documented LE pin.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/codecs/tests/vorbis/mod.rs:374-390
Code:
    let out = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-c:a", "pcm_f32le", "-f", "f32le", "-"])
What is wrong: `-map 0:a` selects EVERY audio stream; a multi-audio-stream
input makes ffmpeg fail into the raw muxer (one stream only) → unreachable!
panic. The h264 suite's equivalent uses `-map 0:v:0`. Fail-loud, but the
comparison is single-track by construction; first-audio-stream mapping would be
correct and robust.
Confidence: MEDIUM — Verification: read (ffmpeg multi-stream raw-muxer failure
not executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / fixture_bytes | 28-41 | REAL | — | executed via bitexact run |
| reference_path / reference / fixture_reference / reference_section | 44-75 | REAL | — | executed (pins parsed) |
| parse_reference | 79-101 | REAL | — | ini parser, verbatim values |
| corpus_dir | 106-113 | REAL | — | LUMEN_CORPUS else /tmp/my-project/corpus-v4 (absent here → corpus lanes skip) |
| CaptureSink | 116-140 | REAL | — | i64::MIN sentinel for Missing pts |
| decode_through_ogg | 157-265 | REAL | S4 | full L1→L3 composition; first-serial assumption (F-002); generation taken from the demuxer's first packet (documented, correct) |
| concatenated | 268-270 | REAL | — | |
| window_at | 280-306 | REAL | — | derived trim from first Present pts; head_trim fallback documented |
| window | 308-326 | DEAD | S3 | zero callers (F-001) |
| canonical_f32_md5 | 332-340 | REAL | S4 | LE-only pattern (F-003) |
| f32_max_delta | 344-361 | REAL | — | bounded lossy tolerance; zip truncates on length mismatch (benign — callers assert equal lengths first) |
| ffmpeg_available / ffmpeg_decode_f32le | 364-391 | REAL | S4 | -map 0:a (F-004); executed live in my run |

## 5. Test honesty
No tests in this file (it is the shared surface). Consumer verdicts from my
runs: vorbis_bitexact 5/5 REAL (live ffmpeg oracle + pinned MD5 + max-delta ≤
1e-7 + bit-exact silence lane); vorbis_vlc_differential 1/1 REAL-with-honest-
PENDING (VLC oracle absent; Lumen half computed and pinned; skip is explicit in
test name and header, not hidden).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the decode-through-ogg driver (the full L1→L3 composition ... no synthetic fixture anywhere in the path)" | L2-6 | REAL | decode_through_ogg uses demux::ogg + codec-api decoder; executed |
| "its correctness is pinned by the recorded expectations ... never trusted blind" | L11-13 | REAL | bitexact run pins vs ffmpeg reference data |
| "-0.0 canonicalized to +0.0 ... the reference data explains why" | L328-331 | REAL | rationale documented; LE caveat unmentioned (F-003) |

## 7. Dead / duplicated / unreachable code
`window` — dead (F-001). `#![allow(dead_code)]` (L1) is the module-level
blanket that keeps it (and any future rot) invisible to the compiler.

## 8. Dependencies & flags
codecs::audio::vorbis (symphonia-codec-vorbis engine behind it), demux::ogg,
codec-api, media-types, plugin-api, testkit; external ffmpeg on PATH for the
live oracle. No feature gates.

## 9. Open questions for the next agent
- With corpus-v4 present: do any vorbis corpus lanes hit the first-serial
  assumption (F-002) or `-map 0:a` (F-004)? The corpus half is UNVERIFIED here.
- The PENDING-VLC differential: the 3.E3 harness pin (VLC 3.0.23) has never
  executed anywhere in this repo's evidence — treat the "differential" row in
  any matrix with care.
