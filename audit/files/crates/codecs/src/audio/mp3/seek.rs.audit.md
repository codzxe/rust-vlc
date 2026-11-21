# AUDIT: crates/codecs/src/audio/mp3/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 216 / 5 items (`VbrToc`, `SeekPlan`, `cbr_seek`, `vbr_toc_seek`, `refine_to_frame`) + 4 tests + 1 helper |
| File verdict | UNREACHABLE — real, test-executed planning code with zero production callers; the shipped raw-MP3 seek is a typed-refusal stub in the demuxer |
| Reachable from shipped entry point? | NO for this module: `rg -n "cbr_seek\|vbr_toc_seek\|refine_to_frame\|VbrToc"` over crates/ and apps/ returns only tests (mp3_corpus_differential.rs:164-202, mp2_corpus_differential.rs:156-191), fuzz (mp3/fuzz.rs:50,114-128) and the mp3/mp2 mod.rs re-exports. The DECODER is reachable (`pipeline_host.rs:261-265` `"mp3" \| "mp1"`, `"mp2"`), and raw .mp3/.mp2 files route through demux/src/es/mp3.rs (`RawMp3Demuxer`) — whose own `seek` is an unconditional stub (es/mp3.rs:680-689) that never calls this module (demux L2 cannot depend on codecs L3; es/mp3.rs:14 documents the split). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | full Read of the file · `rg -n "cbr_seek\|vbr_toc_seek\|refine_to_frame\|VbrToc" crates apps --include=*.rs` (callers: tests/fuzz/re-export only) · `sed -n '670,690p' crates/demux/src/es/mp3.rs` (the stub seek) · `grep -n "fn seek" crates/plugin-api/src/demux.rs` (the Demuxer trait seam) · `sed -n '155,215p' crates/codecs/tests/mp3_corpus_differential.rs` (the accuracy test is fixture-based) · baseline test-workspace.log.txt:435-438 (4/4 unit tests ok) and :1167-1174 (corpus binary 2/2 ok, 0.06s) · docs/formats/audio-mp3-mp2.md:9,27,34 (the seek claims) |

## 1. One-paragraph verdict
The arithmetic in this module is correct and honestly documented (CBR exact; VBR bounded by TOC granularity with a stated ±1%-plus-one-frame accuracy contract, tenth-of-a-percent interpolation, monotone-entry guard, per-mille clamping that bounds past-end targets) — and all four unit tests plus a fixture-based accuracy test execute in the baseline. But none of it ships: no production code calls any function here; the raw-MP3 demuxer that owns .mp3 files implements `seek` as an unconditional typed refusal, and the docs record claims "implemented (probe/decode/seek/gapless both layers)" — the seek half of that claim is false in the shipped product (planner unreachable + demuxer stub). A user pressing seek on an .mp3 in the shipped binary gets `demux.es.mp3.seek.unsupported` while the record says seek is implemented.

## 2. What actually works (verified)
- 4/4 unit tests execute in baseline — test-workspace.log.txt:435-438 (`a_degenerate_zero_toc_pins_the_stream_start`, `cbr_seek_is_exact_frame_arithmetic`, `refine_snaps_to_a_real_frame_header`, `vbr_toc_seek_lands_at_or_before_the_target` — all ok).
- Fixture-based VBR accuracy test executes — mp3_corpus_differential binary 2/2 ok (baseline :1167-1174, 0.06s; `vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly` reads the committed `vbr_q4.mp3` fixture — no corpus dependency on that test).
- CBR arithmetic hand-verified: 0xFF 0xFB 0x90 0x00 → MPEG-1 L3 128 kbps 44.1 kHz → frame_size 417, spf 1152 — the test's exact values (100 + k·417) are correct.
- VBR interpolation bounds: `permille` clamped to [0,999]; past-end targets clamp to the 99th entry with `next = 255` (103-107) so the byte landing stays ≤ `first_frame_offset + 254/255 · total_bytes` — no past-end explosion; `(next - anchor)` guarded by `.max(anchor)` (104) so a locally decreasing pair cannot invert; all intermediate products bounded (255·total_bytes, total_frames·999 — no u64 overflow for any file < ~72 PB).
- `refine_to_frame` = thin wrapper over `scan_sync` (header.rs, W07-A06's file) which "only returns offsets whose header parses" — the wrapper adds no risk.

## 3. Findings

### F-001 [S2]
Where: docs/formats/audio-mp3-mp2.md:9 and :27 (claims) vs crates/demux/src/es/mp3.rs:680-689 (shipped reality) and this module (the claimed implementation)
Code (the doc claim):
```markdown
- status = **implemented** (probe/decode/seek/gapless both layers)
```
Code (the shipped raw-MP3 seek):
```rust
    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "demux.es.mp3.seek.unsupported",
        ))
    }
```
What is wrong: the record claims MP3 (and MP2, line 34 "seek = CBR arithmetic both directions + `refine_to_frame` snapping") seek as implemented. In the shipped binary: (a) this module — the cited CBR/VBR planning — has no production caller at all (verified by repo-wide caller search); (b) the demuxer that owns .mp3/.mp2 files refuses every seek typedly, ignoring the target. Seek on a raw MP3 does not work anywhere in the product.
Why it matters: "doc says works, code is stub" — the S0 pattern — realized on the seek half of a P0 codec row's record. Rated S2 only because the planner code itself is real and test-proven (not fabricated) and MP3-in-container (e.g. MKV) seek works via the container's own index, not this path; the gate should consider escalating.
What correct looks like: wire `es/mp3.rs::seek` to the framing arithmetic (its own module already has the geometry) and the Xing TOC (via a demux-owned parse — the layering forbids calling this module from L2), or downgrade the record's seek claim to match the stub.
Confidence: HIGH — Verification: read (caller search + stub read verbatim); no run of `lumen seek` on an .mp3 exists in the baseline (smoke is open/decode only) — the stub's unconditional `Err` needs no run.

### F-002 [S3]
Where: crates/codecs/src/audio/mp3/seek.rs:73-131 (the whole public surface)
Code:
```rust
pub fn cbr_seek(header: &FrameHeader, first_frame_offset: u64, target_sample: u64) -> SeekPlan {
```
What is wrong: `cbr_seek`, `vbr_toc_seek`, `refine_to_frame`, `VbrToc`, `SeekPlan` are dead in production — their only exercisers are the in-crate tests, two corpus-differential tests, and the fuzz harness. Re-exported at mp3/mod.rs:73 / mp2/mod.rs:79 with no consumer.
Why it matters: 216 lines of seek planning that the product cannot reach, while the product's actual MP3 seek is a stub — the code exists to be cited in docs but not to be used. Maintenance cost plus a misleading surface.
What correct looks like: either the demuxer's seek lane consumes this arithmetic (via a demux-owned mirror, respecting the layer boundary) or the module is folded into the test/fuzz layer and the re-exports dropped.
Confidence: HIGH — Verification: read + repo-wide `rg` caller search (output in the header table).

### F-003 [S4]
Where: crates/codecs/src/audio/mp3/seek.rs:27-28 and 43-45 (doc claims) — untested and false for damaged TOCs
Code:
```rust
//! So the honest contract is: the returned offset lands **at or
//! before** the target frame, never after, and the consumer must
```
```rust
    /// decode forward and drop samples up to the target (pre-roll drop —
```
and:
```rust
    /// percent-of-bytes at `i` percent of
    /// frames. Real files are monotone non-decreasing; a damaged TOC
    /// simply lands somewhere at-or-before (bounds still hold).
```
What is wrong: "never after" and "a damaged TOC simply lands somewhere at-or-before" hold only for TOCs that are accurate. An inflated entry (`toc[percent]` above the true byte fraction — e.g. a hostile or buggy encoder TOC), a non-monotone table across entries, or a lying `bytes`/`frames` field all land AFTER the target; the consumer's pre-roll drop (`target − first_sample_at_offset`) then goes negative with no defined behavior. The in-file tests use a perfect synthetic linear TOC and an all-zero TOC (both monotone), and the fixture test ASSERTS monotonicity as a precondition instead of testing the damaged case — the doc's damaged-TOC claim is asserted nowhere.
Why it matters: the contract statement is what a future consumer would code against; it is wrong exactly in the case it claims to cover (damage). Blast radius today: none (no consumer — F-002), so S4.
What correct looks like: either clamp the landing to the frame-count-consistent bound (`frames_before` and `byte_frac` derived from the same permille are mutually consistent; cross-checking them would detect inflated TOCs), or reword the contract to "at-or-before for accurate TOCs; a wrong TOC can overshoot; consumers must treat negative pre-roll as zero".
Confidence: HIGH — Verification: read; arithmetic traced for the inflated-entry case (anchor alone can exceed the target's true byte fraction).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VbrToc` | 39-53 | REAL / UNREACHABLE | S3 | data carrier; `total_frames` u32, `total_bytes` u64 |
| `SeekPlan` | 55-65 | REAL / UNREACHABLE | S3 | data carrier |
| `cbr_seek` | 73-80 | REAL / UNREACHABLE | S3 | exact arithmetic; `spf.max(1)` guards div-by-zero; no clamp to stream length (past-end targets yield past-end offsets — the documented consumer contract/refine handles it: `refine_to_frame` past end → `None`); `frame_index × frame_size` overflow needs a caller-supplied target > ~10^16 — caller-trust, unreachable in practice |
| `vbr_toc_seek` | 91-117 | REAL / UNREACHABLE | S3/S4 | per-mille clamping bounds past-end targets; `.max(anchor)` guards local inversions; overshoot possible with lying TOC fields (F-003); `(target-1)×1000` overflow needs target > 1.8×10^16 — caller-trust |
| `refine_to_frame` | 127-131 | REAL / UNREACHABLE | S3 | thin `scan_sync` wrapper; `None` on no-header |
| test helper `hdr_128k` | 138-143 | REAL | — | 4-byte real header |
| test `cbr_seek_is_exact_frame_arithmetic` | 145-156 | REAL | — | exact offsets/samples for 3 targets — mutation-detecting |
| test `vbr_toc_seek_lands_at_or_before_the_target` | 158-186 | REAL | — | synthetic perfect TOC; ≤51% for a 50% target, ≤2 frames slack — mutation-detecting; does not cover damaged TOCs (F-003) |
| test `a_degenerate_zero_toc_pins_the_stream_start` | 188-204 | REAL | — | zero TOC: head pins first frame; near-end target bounded |
| test `refine_snaps_to_a_real_frame_header` | 206-214 | REAL | — | snaps to offset 5 past garbage; empty input → `None` |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| cbr_seek_is_exact_frame_arithmetic | YES | exact byte/sample values for 3 targets |
| vbr_toc_seek_lands_at_or_before_the_target | YES | percent + frame-slack bounds would break on index/interpolation mutations |
| a_degenerate_zero_toc_pins_the_stream_start | YES | exact head offset + bounded end |
| refine_snaps_to_a_real_frame_header | YES | exact snap offset + None case |
| vbr_toc_seek_accuracy… (integration, committed fixture) | YES | ±600 B window around the true midpoint + refined landing must parse as a header — note it allows landing ≤600 B AFTER the midpoint, quietly acknowledging the "never after" doc claim is approximate |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = **implemented** (probe/decode/seek/gapless both layers)" | docs/formats/audio-mp3-mp2.md:9 | FALSE (seek half) | F-001: planner unreachable (rg proof) + demux/src/es/mp3.rs:680-689 stub |
| "CBR+VBR seek planning" as part of the implemented MP3 surface | docs/formats/audio-mp3-mp2.md:27 | UNREACHABLE | the planning exists (this file, test-proven) but no shipped lane reaches it |
| "seek = CBR arithmetic both directions + `refine_to_frame` snapping to a real header (…)" (MP2) | docs/formats/audio-mp3-mp2.md:34 | UNREACHABLE + stub | same proof; `RawMp3Demuxer` owns .mp2 (es/mp3.rs:72 EXTENSIONS) and refuses seek |
| "the returned offset lands at or before the target frame, never after" | seek.rs:27-28 (module doc) | FALSE (for damaged/inaccurate TOCs) | F-003 |

## 7. Dead / duplicated / unreachable code
- The entire public surface — dead in production (F-002). The demuxer's own `es/mp3.rs` re-implements the frame-walk geometry it needs (and detects Xing frames only to skip them, es/mp3.rs:595-629) — a partial duplication of this module's knowledge across the L2/L3 boundary, documented as deliberate at es/mp3.rs:14.

## 8. Dependencies & flags
- Depends only on `super::header::{FrameHeader, scan_sync}` (W07-A06's file — its correctness underpins `refine_to_frame`). Compiled under `codec-mp3` (default set; baseline #1 CLEAN). No unsafe, no panics (all arithmetic bounded or caller-trusted, see table).

## 9. Open questions for the next agent
- Demux wave owner: `es/mp3.rs:680-689` — the seek stub — plus its `let _ = target;` ignored parameter is the exact S0-pattern site behind F-001; it is outside my assignment and unclaimed in wave 7 (verify in the demux wave's files).
- W07-A04 (mp3/decoder.rs): does the decoder's `decode_stream` expose `first_frame_offset`/`xing` (consumed by the corpus test) to any production consumer, or is that surface test-only too?
- W07-A06 (header.rs): `scan_sync`'s validation strength (it gates every `refine_to_frame` landing) — the corpus test only proves it on one fixture.
