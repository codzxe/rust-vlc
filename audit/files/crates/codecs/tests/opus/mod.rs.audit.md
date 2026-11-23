# AUDIT: crates/codecs/tests/opus/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 265 / 8 helpers (test-support module; `#![allow(dead_code)]` by design) |
| File verdict | REAL (composition driver + oracle helpers; the only executed consumer skip-greens in this environment for want of the corpus) |
| Reachable from shipped entry point? | N/A (test-support). The decode path it drives (demux::ogg → `codecs::audio::opus::OpusDecoder`) IS CLI-reachable: pipeline_host "opus" arm |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg consumer traces; sed reads; baseline cite; corpus-absence already proven this session |

## 1. One-paragraph verdict
Good test-support code: `decode_through_ogg` drives the REAL composition (in-memory Ogg demux → verbatim OpusHead‖OpusTags header join as `extra_data` → the contract `OpusDecoder`), with `unreachable!` on every step so any deviation fails loudly; the ffmpeg-oracle helpers and the tolerance comparator implement the admission record's stated bar (zero samples beyond one s16 LSB). The consumer differential (`opus_corpus_differential.rs`, 2 tests) is green in the baseline (:1292-1296) but finished in 0.01s — the corpus directory is absent (proven earlier this session), so both lanes took the loud-skip path and the geometry/pre-skip/granule/tolerance assertions never executed here; their evidence lives in the admission record/worklog (UNVERIFIED in this sandbox). Two hygiene nits in the comparators: native-endian parsing of a documented-LE format, and silent zip truncation on length mismatch.

## 2. What actually works (verified)
- Read the full driver: real demuxer calls (`OggDemuxer::new/start/next_event/serial_numbers/header_packets`), serial resolution by `OpusHead` prefix sniff (:104-113), lazy decoder construction on first data packet with the joined header chain (:138-154) — matches the documented L2→L3 seam.
- Consumer pinned-length discipline verified: the caller asserts `output_samples(&run) == valid` AND `ref_samples == valid` (opus_corpus_differential.rs:78, :101-105) before comparing, so the comparator's equal-length contract is enforced by both sides independently.
- Baseline :1290-1296: 2/2 ok (skip-green; 0.01s — no ffmpeg oracle ran).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/opus/mod.rs:194-211
Code:
```
pub fn f32_max_delta(ours: &[u8], theirs: &[u8]) -> f32 {
    let a: Vec<f32> = ours
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_ne_bytes(*b))
```
What is wrong: the doc comment says "f32le byte streams" and the producer (`ffmpeg_decode_f32le`, :248-252, `-f f32le`) emits little-endian, but the parser uses `from_ne_bytes`. On a big-endian host every sample would be byte-swapped garbage — the differential would fail loudly (not silently pass), so no false-green risk on the shipped LE hosts (x86_64, wasm32, aarch64-apple all LE), but the helper's name/doc and parsing disagree. Same in `samples_beyond` (:214-231). What correct looks like: `f32::from_le_bytes`. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/codecs/tests/opus/mod.rs:207-210
Code:
```
    a.iter()
        .zip(&b)
        .map(|(x, y)| (*x - *y).abs())
        .fold(0.0f32, f32::max)
```
What is wrong: `zip` silently truncates to the shorter stream — a length mismatch between our decode and the oracle would be invisible to the comparator (extra/missing tail ignored). The doc says "of equal length" but nothing enforces it in the helper. The current caller pins both lengths to the same `valid` constant first, so no live hazard; the helper is a trap for future callers. What correct looks like: `assert_eq!(a.len(), b.len())` inside, or a length-mismatch return. Confidence: HIGH — Verification: read + caller trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 28-35 | REAL | — | LUMEN_CORPUS override; None → caller skips (absent in this sandbox — proven) |
| CaptureSink::accept | 49-63 | REAL | — | audio-only, pts ticks captured, Missing = i64::MIN |
| decode_through_ogg | 86-179 | REAL | — | full real composition; per-packet Emitted/NeedMoreInput only; counts packets/granule; exposes pre_skip/decoded_raw |
| concatenated | 182-184 | REAL | — | |
| output_samples | 187-190 | REAL | — | / (channels×4) — f32 assumption consistent with format |
| f32_max_delta | 194-211 | REAL | S4 | F-001 (ne vs le), F-002 (zip truncation) |
| samples_beyond | 214-231 | REAL | S4 | same two nits |
| ffmpeg_available / ffmpeg_decode_f32le | 234-265 | REAL | — | live oracle; -ar NOT forced (correct: opus is 48 kHz native); unreachable! on ffmpeg failure |

## 5. Test honesty
No tests in this file (support module). The consuming differential's assertions are strong (channels/rate/packets/pre_skip=312/decoded_raw/granule/valid-window/first-pts=0, then zero-samples-beyond-1-LSB + max|Δ|<1e-3 vs live ffmpeg) — but in THIS environment both corpus tests skip green (baseline 0.01s, corpus absent). The unit-level opus decoder tests elsewhere (other agents) are the only executed opus evidence in the baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no synthetic fixture anywhere in the path" | mod.rs:5-6 | REAL | decode_through_ogg uses only demuxer+decoder APIs |
| "two independent float decoders are not bit-exact; s16-audible equivalence is zero samples beyond one s16 LSB" | mod.rs:7-9 | REAL-UNPROVEN | the bar is implemented; the corpus lanes that measure it skip in this sandbox (worklog/admission-record numbers not re-executed) |
| "f32le byte streams" | mod.rs:193 | INCONSISTENT with from_ne_bytes | F-001 |

## 7. Dead / duplicated / unreachable code
`#![allow(dead_code)]` covers helper subsets unused by particular consumers — acceptable for shared test support, but it also masks genuinely dead helpers if a consumer is deleted.

## 8. Dependencies & flags
Test-support; depends on demux (ogg test_support mem_stream), codecs, plugin-api, testkit. No features.

## 9. Open questions for the next agent
- opus_corpus_differential.rs auditor: confirm the worklog/admission record holds the measured max|Δ| rows (1.378e-06 mono, 5.1 lane) so the skip-green baseline has at least authoring-time evidence.
- Is any committed (non-/tmp) Opus fixture possible, as the ADPCM breadth suite does, so the composition driver runs in every baseline?
