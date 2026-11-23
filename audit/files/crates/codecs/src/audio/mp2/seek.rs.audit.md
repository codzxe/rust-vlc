# AUDIT: crates/codecs/src/audio/mp2/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 223 / 4 fns (+2 test helpers, 4 tests) |
| File verdict | UNREACHABLE (real, correct, tested code — no shipped entry point calls it) |
| Reachable from shipped entry point? | NO: exported at mp2/mod.rs:79 but the only callers in the whole tree are tests — mp2/seek.rs's own tests, mp2/decoder.rs:661-674 (a #[test]), and crates/codecs/tests/mp2_corpus_differential.rs:155-178. Apps' hosts use only `Mp2Decoder` (apps/cli/src/pipeline_host.rs:266-269, apps/web/src/host.rs:629-632); demux has no mp2 lane (crates/demux/src/es/ contains aac.rs, mp3.rs, annexb.rs — no mp2) and cannot depend on codecs (L1 vs L3) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::mp2::seek` → 4 passed / 0 failed (exit 0); rg caller census over apps/ + crates/ (exit 0) |

## 1. One-paragraph verdict
The arithmetic itself is right: CBR frame math is division-by-zero-guarded (`samples_per_frame.max(1)`, `frame_size.max(1)`, `saturating_sub`), clamping is honest (target beyond end → last frame; byte beyond end → total_samples), duration uses saturating mul/div, and `refine_to_frame` correctly delegates to the family's verified `scan_sync` (mp3/header.rs:392-404, which re-validates each candidate with `parse_frame_header`). I ran the 4 tests: pass, including the 44.1 kHz padded-stream snap test and a cross-pin against the mp3 planner. But the entire module is a shipped nothing: no production code path in the repository calls any of its four functions. MP2 playback reaches only `Mp2Decoder`; seeking an MP2 track in a real container is container-level (MKV cues / ASF), and the raw-ES lane this planner was built for does not exist (demux/src/es has mp3.rs but no mp2.rs).

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::mp2::seek` → `test result: ok. 4 passed; 0 failed` (frame-exact arithmetic incl. clamp-beyond-end, family-planner agreement, inverse mapping, padded-stream snap onto a real header that re-parses Ok).
— verified by caller census: rg `cbr_seek|sample_at_byte|duration_nanos|refine_to_frame|Mp2SeekPlan` across apps/ + crates/ → only codecs-internal test hits.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mp2/seek.rs:46-88 (whole public surface) and mp2/mod.rs:79
Code:
```rust
pub fn cbr_seek(geo: &Mp2Geometry, target_sample: u64) -> Mp2SeekPlan {
    let spf = u64::from(geo.samples_per_frame.max(1));
    let frame_index = (target_sample / spf).min(geo.total_frames.saturating_sub(1));
```
What is wrong: dead-in-product capability. The module doc sells "MP2 seeking (card 5.J3)" and `sample_at_byte` claims to be "the demuxer's duration/progress arithmetic" (L18-20) — no demuxer uses it (demux cannot import codecs; its ES lane has no MP2 support at all). Every caller is a test. Why it matters: any doc/matrix row claiming MP2 seek/duration support has no wired path behind it, and the "family planner agreement" test pins two test-only modules to each other. What correct looks like: either wire the planner into a real ES/mp2 demux lane + engine seek path, or mark the module as library-surface-only. Confidence: HIGH — Verification: read + rg census (apps/, crates/, excluding tests). File-level verdict: UNREACHABLE.
F-002 [S4]
Where: crates/codecs/src/audio/mp2/seek.rs:62-69
Code:
```rust
    let frames_in = (byte_offset - geo.first_offset) / u64::from(geo.frame_size.max(1));
    (frames_in * spf).min(geo.total_samples)
```
What is wrong (minor): `sample_at_byte` floors to the *nominal* frame grid; on padded 44.1/22.05/16/32 kHz streams the doc itself admits it is "an estimate within one frame" — acceptable and documented, but the forward direction (`cbr_seek` + `refine_to_frame`) is the only exact one; progress bars built on this can jitter ±1152 samples. Documented limitation, hygiene note. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cbr_seek | 46-54 | REAL (tests) / UNREACHABLE shipped | F-001 | guards spf=0; clamp to last frame tested |
| sample_at_byte | 62-69 | REAL (tests) / UNREACHABLE shipped | F-002 | floor estimate on fractional rates, documented |
| duration_nanos | 74-79 | REAL (tests) / UNREACHABLE shipped | — | saturating mul/div, rate guarded `.max(1)` |
| refine_to_frame | 86-88 | REAL (tests) / UNREACHABLE shipped | — | 1-line delegate to verified scan_sync |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cbr_seek_is_exact_frame_arithmetic | YES | numeric offsets/sample indices asserted, clamp pinned |
| cbr_seek_agrees_with_the_family_planner | YES (drift) | cross-module pin vs mp3::cbr_seek over 5 targets |
| sample_at_byte_inverts_on_integral_frame_sizes | YES | boundary 719/720 pinned; clamp + duration asserted |
| refine_snaps_a_padded_stream_to_a_real_header | YES | real 1253/1254 mixed stream; snapped header re-parses |
But all of it proves a module nothing ships.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`sample_at_byte` … is the demuxer's duration/progress arithmetic" | seek.rs:18-20 | FALSE (no demuxer calls it) | rg census: zero non-test callers; demux/src/es has no mp2 lane |
| "refine … snaps forward to the next real frame header and the consumer drops the pre-roll samples … the same rule the family's VBR seek and the FLAC seek planner use" | seek.rs:12-16 | REAL-UNPROVEN (the snap is real+tested; the "consumer" that drops pre-roll is not any shipped consumer — none exists for this path) | tests verify the snap only |

## 7. Dead / duplicated / unreachable code
The whole file (see F-001). Duplication note: mp3/seek.rs implements the same CBR/refine pattern for its own (also test-only) surface — family duplication with a drift-pin test, which is the only thing keeping them honest.

## 8. Dependencies & flags
`super::header::Mp2Geometry`, `crate::audio::mp3::scan_sync` (mp2 depends on mp3 module per Cargo.toml comment "codec-mp2 = [codec-mp3]"). Behind default feature `codec-mp2` — compiled into the shipped build, never called.

## 9. Open questions for the next agent
- Does any feature-matrix row cite `mp2` seek/duration evidence (e.g. `crates/codecs/tests/mp2_corpus_differential.rs` seek test) as product capability? If yes, that row is FALSE-by-unreachability.
- The demux ES lane (mp3.rs) — if an MP2 ES lane is ever added, wire this planner in or delete it.
