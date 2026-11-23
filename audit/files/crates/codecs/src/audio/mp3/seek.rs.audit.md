# AUDIT: crates/codecs/src/audio/mp3/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 215 / 3 (+4 tests, 1 helper) |
| File verdict | UNREACHABLE (real math, no shipped entry point calls it) |
| Reachable from shipped entry point? | NO — the shipped `.mp3` lane's demuxer seek is a typed refusal (`demux.es.mp3.seek.unsupported`, crates/demux/src/es/mp3.rs:680-689); `cbr_seek`/`vbr_toc_seek`/`refine_to_frame` are called only by this module's tests and crates/codecs/src/audio/mp3/fuzz.rs:110-116 (`rg` over crates/apps) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `cargo test -p codecs --lib audio::mp3::seek` → 4 passed (0.00s); python3 dump of `fuzz/corpus/fuzz_codec_mp3/seed_mp3_vbr.mp3` Xing TOC (output below) |

## 1. One-paragraph verdict
The VBR interpolation's 255ths scale is empirically right — a python dump of the real LAME-written fixture (`seed_mp3_vbr.mp3`: frames=78, bytes=9208, toc[0]=0, toc[50]=138, toc[99]=255, first-frame offset 68, 68+9208 == file size 9276) confirms `byte_frac * total_bytes / 255` matches the encoder's own normalization, and the permille/anchor/next interpolation is correctly clamped (`min(999)`, `next.max(anchor)`). But the module carries two genuine math defects: (1) CBR seek freezes ONE frame's `frame_size` — which includes that frame's padding bit — as a stream constant, so the "frame-exact" claim is false for the most common CBR shape (128 kbps @ 44.1 kHz frames alternate 417/418 to realize 417.96; drift ≈ 0.23% of seek distance, ~0.5 s at 3 minutes); (2) for targets in the last 1% of a LAME VBR file (`permille` 990-999 → `percent` 99), the end-of-grid bound 255 saturates `byte_frac` to 255 and the offset lands exactly at EOF (first + total_bytes) — at/after the target, violating the module's own "never after" contract, and `refine_to_frame` from EOF returns `None`, making near-end VBR seeks unlandable. None of this is reachable today: the shipped demuxer refuses MP3 seeks outright, and docs/perf/breadth.md:145-152 honestly records the machinery as "NOT CLI-reachable" — though its "open path refuses their containers" clause is stale for mp3 (open IS wired, apps/cli/src/containers/mod.rs:66; only seek refuses).

## 2. What actually works (verified)
— `cargo test -p codecs --lib audio::mp3::seek` → 4 passed.
— python3 fixture dump: `toc[0] 0 toc[50] 138 toc[99] 255`, `frames 78 bytes 9208`, first frame at 68, file size 9276 == 68+9208 → the `/255` scale and `first_frame_offset + byte_frac * total_bytes / 255` formula match the encoder's own TOC exactly at both grid ends.

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/mp3/seek.rs:72-80 (+ header.rs:13-16)
Code:
```
pub fn cbr_seek(header: &FrameHeader, first_frame_offset: u64, target_sample: u64) -> SeekPlan {
    let spf = u64::from(header.samples_per_frame.max(1));
    let frame_index = target_sample / spf;
    SeekPlan {
        byte_offset: first_frame_offset + frame_index * u64::from(header.frame_size),
        first_sample_at_offset: frame_index * spf,
```
What is wrong: `header.frame_size` is ONE observed frame's size and "frame_size = 144·bitrate/sample_rate + padding" (header.rs:13-16) — the padding slot is per-frame. In a legal 44.1 kHz CBR stream (e.g. 128 kbps: 417.96 bytes/frame) encoders emit alternating 417/418-byte frames; freezing 417 (or 418) accumulates drift ≈ 0.23% of the traversed span. At 90% of a 3-minute file (~6900 frames) the plan's byte offset is ~6.6 KB (~16 frames ≈ 0.5 s) off its `first_sample_at_offset` claim, so the consumer's pre-roll drop mis-drops by the same amount. The doc's "Pure arithmetic — the plan is frame-exact" (L70-71) is false for the dominant 44.1 kHz CBR shape; only whole-ratio rates (48/32 kHz) are exact. What correct looks like: compute the byte offset from time×bitrate (or track exact byte span via the Xing `bytes`/`frames` when present) instead of frame_index×one_frame_size. Confidence: HIGH — Verification: read (arithmetic reasoning; the unit test cannot catch it — see F-004).
F-002 [S2]
Where: crates/codecs/src/audio/mp3/seek.rs:101-111
Code:
```
    let next = if percent + 1 < 100 {
        u64::from(toc.toc[percent + 1]).max(anchor)
    } else {
        255
    };
    let sub = permille % 10;
    // byte_frac is in 255ths (the Xing TOC's own scale).
    let byte_frac = anchor + (next - anchor) * sub / 10;
    let offset = toc.first_frame_offset + byte_frac * toc.total_bytes / 255;
```
What is wrong: for `percent == 99` (targets in the last 1% of samples) the upper interpolation bound is hard-coded 255; on real LAME files toc[99] IS 255 (verified on the fixture), so `byte_frac` saturates at 255 for every `sub` and the offset = `first_frame_offset + total_bytes` = exactly EOF. The landing is at/after the target — the module contract "the returned offset lands at or before the target frame, never after" (L27-29) is violated — and `refine_to_frame(data, EOF)` → `scan_sync` → `None`, so the seek cannot complete for the final ~1% of any LAME VBR file (~2.4 s of a 4-minute track). What correct looks like: interpolate inside entry 99 against the true end (or clamp byte_frac to `total_bytes - max_frame_size`). Confidence: HIGH — Verification: read + executed python dump of the fixture TOC (toc[99]=255, 68+9208==9276).
F-003 [S3]
Where: crates/codecs/src/audio/mp3/seek.rs:1-35 vs crates/demux/src/es/mp3.rs:680-689
Code:
```
    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "demux.es.mp3.seek.unsupported",
```
What is wrong: the module advertises "Seeking (card 5.A3)" but the only shipped MP3 lane refuses seek — the math here is dead weight until the demuxer's seek lands (docs/perf/breadth.md:145-152 records "none is reachable from the CLI on this build"). Latent F-001/F-002 therefore ship no user-visible bug today. Confidence: HIGH — Verification: read + rg (callers: tests + mp3/fuzz.rs only).
F-004 [S4]
Where: crates/codecs/src/audio/mp3/seek.rs:145-156
Code:
```
        let plan = cbr_seek(&h, 100, 1152 * 7 + 1111);
        assert_eq!(plan.byte_offset, 100 + 7 * 417);
```
What is wrong: the CBR test is a tautology — it asserts the function's own formula (`100 + 7·417`) with no external ground truth, so the padding-drift defect (F-001) is invisible to it by construction. The VBR test's synthetic linear TOC likewise never exercises a real encoder TOC or the entry-99 saturation. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/codecs/src/audio/mp3/seek.rs:112-115
Code:
```
    let frames_before = total_frames * permille / 1000;
    SeekPlan {
        byte_offset: offset,
        first_sample_at_offset: frames_before * spf,
```
What is wrong: `first_sample_at_offset` is derived from `permille` (the frame-count grid) while `byte_offset` is derived from the TOC's byte grid — on a damaged or non-linear TOC the pair can disagree arbitrarily (byte landing after the claimed sample), and the consumer's pre-roll arithmetic `target − first_sample_at_offset` then mis-drops. Documented as an estimate, so S4. Also `frame_index * frame_size` in `cbr_seek` can overflow u64 for absurd `target_sample` (u64::MAX at Layer I geometry) — debug panic/wrap; fuzz-only blast radius. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VbrToc | 41-53 | REAL | — | data |
| SeekPlan | 58-65 | REAL | — | data |
| cbr_seek | 72-80 | REAL | S2 | padding-bit drift (F-001); unreachable in shipped lane (F-003) |
| vbr_toc_seek | 90-117 | REAL | S2 | /255 scale verified vs real fixture; entry-99 EOF saturation (F-002) |
| refine_to_frame | 126-131 | REAL | — | thin scan_sync wrapper; None-at-EOF reachable via F-002 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cbr_seek_is_exact_frame_arithmetic | NO (for F-001-class bugs) | tautology: asserts the formula's own arithmetic |
| vbr_toc_seek_lands_at_or_before_the_target | PARTIAL | catches gross overshoot (pct ≤ 51) and granularity > 2 frames; misses entry-99 saturation and real-TOC shapes |
| a_degenerate_zero_toc_pins_the_stream_start | YES | all-zero TOC bounds asserted |
| refine_snaps_to_a_real_frame_header | YES | real header bytes at offset 5; empty → None |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Pure arithmetic — the plan is frame-exact" (L70-71) | seek.rs | FALSE for 44.1 kHz CBR | F-001 |
| "lands at or before the target frame, never after" (L27-29) | seek.rs | FALSE for last 1% of LAME VBR files | F-002 + fixture dump |
| "none is reachable from the CLI on this build" | docs/perf/breadth.md:145-152 | REAL (seek-wise) | demux/src/es/mp3.rs:680-689 refusal |
| "the open path refuses their containers" | docs/perf/breadth.md:148-149 | STALE for mp3 | open IS wired (apps/cli/src/containers/mod.rs:66); only seek refuses |
| TOC "100 entries over the whole file … 1%-granular" (L16-20) | seek.rs | REAL | matches Xing layout; fixture dump |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-003): exported pub (mp3/mod.rs:73), consumed by tests + mp3/fuzz.rs only. No duplication.

## 8. Dependencies & flags
super::header (FrameHeader, scan_sync). No feature gates. Note: transient disk-poisoning corrupted `#[must_use]` reads as `#ust_use]` on some reads during this audit (matches baseline `cargo-test-workspace.attempt1-diskpoisoned.txt`); re-reads returned the true bytes and the crate compiles — all quotes above re-verified on ≥2 reads.

## 9. Open questions for the next agent
- The demuxer-seek card: when `demux.es.mp3.seek` lands, will it use this module (fixing F-001/F-002 first) or reimplement (as the Xing-detection duplication already did — xing.rs F-002)?
- `scan_sync` (header.rs): does it bound its resync scan (a long garbage region → O(n) per call is fine, but confirm no quadratic behavior when refine is called per seek)?
