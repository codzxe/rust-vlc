# AUDIT: crates/subtitles/src/dvb/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Lines / functions | 1265 / 23 (+11 tests, 5 test helpers) |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point links it) |
| Reachable from shipped entry point? | NO — no crate outside `crates/subtitles` and `fuzz/` depends on the subtitles crate (Cargo.toml scan, see F-008) |
| Findings | S0: 0 S1: 1 S2: 3 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b dvb::` -> 38 passed, 0 failed (exit 0); `sed -n` quote verification; Cargo.toml dependency scan |

## 1. One-paragraph verdict
This is a genuinely implemented EN 300 743 page/region/object/CLUT state machine with real
byte-level tests that pass (38/38 executed from the prebuilt baseline binary), not a stub.
But its headline contract is false: `decode_pes` CAN panic — a region redefined to a smaller
CLUT depth without the fill flag leaves stale out-of-range pixel indices in the pixel buffer,
and `region_image` indexes the depth-n bank with them (slice OOB, panics in release too). The
module is also dead code from the product's perspective: nothing in the playback stack depends
on the subtitles crate, so no demuxer ever feeds it a PES payload — the "which demuxers emit
subtitle packets" answer is *none*.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b dvb::` -> `test result: ok. 38
passed; 0 failed; 0 ignored` (full_page_lifecycle_two_updates asserts exact RGBA bytes across
three display sets incl. repaint and clear; cue_builder_spans_and_positions asserts ns-exact
half-open spans and rational anchors).

## 3. Findings
F-001 [S1]
Where: crates/subtitles/src/dvb/mod.rs:462-471 and 820-830
Code:
```
            Some(existing) if existing.width == w && existing.height == h => {
                existing.depth = depth;
                existing.clut_id = region.clut_id;
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
                    existing.dirty = false;
                }
```
```
        let bank = clut.bank(region.depth);
        let mut rgba = Vec::with_capacity(region.pbuf.len() * 4);
        for &index in &region.pbuf {
            let c = bank[index as usize];
```
What is wrong: A region composition that keeps width/height but changes depth (or falls back to
depth 4 via an invalid depth code) updates `depth` and leaves the pixel buffer untouched when
`fill` is clear. `dirty` also stays true from a prior object paint. `clut::Clut::bank` (clut.rs:171-177)
returns `bank4` (len 4) for depth 2 and `bank16` (len 16) for depth 4, so a previously painted
8-bit index (e.g. 200) is sliced as `bank[200]` — index-out-of-bounds panic in **both** debug and
release. Trigger: PES #1 = region 7 @8-bit + object painting value 200 + end-of-display-set;
PES #2 = same region id/dims @2-bit, fill=0, + end-of-display-set (or an invalid depth code 0/4-7).
Why it matters: it falsifies the module contract below and crashes any embedder on a hostile or
buggy broadcast stream. What correct looks like: clear/quantize the pixel buffer on depth change
(or clamp `index` into the bank, or always treat depth change as a new buffer).
Confidence: HIGH — Verification: read (full code-path trace; not executed — cannot add a test to
read-only source; existing tests never change depth on an existing region).

F-002 [S2]
Where: crates/subtitles/src/dvb/mod.rs:29-32 (contract) vs crates/subtitles/src/dvb/segment.rs:379-381
Code:
```
//! set, the page timeout, and a **capped typed damage ledger**. It
//! never panics; every malformation (truncated segment, bad CLUT id,
//! RLE overrun, object placed before its region, region too large)
//! is typed data and the decode continues to the next segment.
```
```
        let x = u16::from_be_bytes([data[5], data[6]]);
        let wx = u16::from_be_bytes([data[7], data[8]]);
        let y = u16::from_be_bytes([data[9], data[10]]);
```
(with `w = wx - x + 1` two lines below in segment.rs)
What is wrong: The "never panics" contract is additionally broken by `parse_display_def`
(segment.rs, W09-A08's file): `wx - x + 1` / `wy - y + 1` are u16 subtractions that underflow
when a display-definition segment with the window flag set carries wx < x (or wy < y) —
panic in debug builds, silent wrap in the release lane. Why it matters: same contract, same
public entry (`decode_pes` -> `display_def_segment` -> `parse_display_def`). What correct
looks like: checked_sub or reject the window as typed damage. Cross-file: filed against
mod.rs's contract; segment.rs detail belongs to W09-A08.
Confidence: HIGH — Verification: read (arithmetic trace; underflow is unconditional for wx<x).

F-003 [S2]
Where: crates/subtitles/src/dvb/mod.rs:801-803
Code:
```
            if !region.dirty {
                continue;
            }
```
What is wrong: A region whose only content is its fill color (region composition with
fill=1, no object) is never composed — `fill` sets `dirty = false` (L468) and compose skips
non-dirty regions. In EN 300 743 the fill color is the region's visible pixel content
(broadcast backing boxes); the reference ffmpeg dvbsub decoder composites every page region
each display set and has no dirty gate, so fill-only regions render there. Why it matters:
solid-color subtitle backgrounds silently disappear. What correct looks like: mark the region
dirty on fill (fill paints w*h pixels), or compose non-dirty regions that have a fill color.
Confidence: MEDIUM — Verification: read (ffmpeg behavior from memory, no oracle available
in sandbox — reference comparison UNVERIFIED).

F-004 [S3]
Where: crates/subtitles/src/dvb/mod.rs:793-797
Code:
```
                if missing_reported.insert(entry.region_id) {
                    self.damage(DvbDamage::ObjectWithoutRegion {
                        object_id: u16::from(entry.region_id) | 0x8000,
                    });
                }
```
What is wrong: A page entry whose *region* is missing is reported as `ObjectWithoutRegion`
with `object_id = region_id | 0x8000`. The Display impl then prints "object 32775 has no
region placement" — a fabricated object id for what is a missing region 7. Typed damage that
lies about its own kind. What correct looks like: a `RegionMissingFromPage` variant.
Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/subtitles/src/dvb/mod.rs:1173-1187
Code:
```
    fn rle_overrun_is_clipped_and_typed() {
        ...
        let up = dec.decode_pes(&payload, None);
        assert_eq!(up.regions.len(), 1);
        assert_eq!(up.regions[0].rgba.len(), 3 * 4, "row fully painted");
    }
```
What is wrong: The test name promises "clipped AND TYPED" but the body never asserts a
`DvbDamage::RleOverrun` event — delete the damage emission and this test still passes.
Why it matters: the damage-ledger half of the contract is untested for the RLE path.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/subtitles/src/dvb/mod.rs:344-351
Code:
```
        if payload.is_empty() || used < payload.len() {
            if segments.is_empty() {
                self.damage(DvbDamage::TruncatedSegment { kind: None });
            } else {
                self.damage(DvbDamage::LengthOverrun);
            }
        }
```
What is wrong: `walk_segments` also stops on a bad sync byte (non-0x0F) or a trailing <6-byte
fragment; both are reported as `LengthOverrun` ("segment length overruns the PES payload"),
which mislabels a sync/fragment problem as a length problem. Cosmetic damage-ledger inaccuracy.
Confidence: HIGH — Verification: read (walk_segments semantics at segment.rs:47-66).

F-007 [S4]
Where: crates/subtitles/src/dvb/mod.rs:398-406
Code:
```
    fn page_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.saw_page = true;
        if self.page_id.is_none() {
            self.page_id = Some(seg.page_id);
        }
```
What is wrong: The page id is *recorded* but never *enforced* — region/CLUT/object segments
for any other page id in the same PES are applied to the locked page's state (EN 300 743
multiplexes ancillary pages in one PES; the doc hand-waves "demuxed upstream", and no such
upstream exists — see F-008). Also, a repeated page version early-returns before the
state==1/2 acquisition/mode-change reset, so an acquisition page with an unchanged version
skips the reset. Both are edge-case state-machine divergences with no test.
Confidence: HIGH (behavior) / MEDIUM (spec reading) — Verification: read.

F-008 [S2 — reachability]
Where: repo-wide dependency graph (fuzz/Cargo.toml:50, crates/subtitles/Cargo.toml; no other manifest)
Code:
```
fuzz/Cargo.toml:50:subtitles = { path = "../crates/subtitles" }
```
What is wrong: An exhaustive scan of every Cargo.toml outside target/ shows the subtitles
crate is a dependency of NOTHING except the fuzz harness. apps/cli, apps/desktop,
apps/desktop-tauri, apps/web, crates/playback-engine, crates/demux, crates/codecs — none
reference it. playback-engine's subtitle selection path (crates/playback-engine/src/session.rs:2276-2279)
states "A provider that cannot build the subtitle decoder deselects honestly (the optional
kind — never a failure)" — there is no provider, so subtitle selection is a structural no-op.
Why it matters: 1265 lines of DVB decoder (plus segment/rle/clut/fuzz siblings) are dead code
in every shipped binary; the entire "subtitles" capability advertised by this crate is
UNREACHABLE from playback. Nuance: demuxers DO recognize and tag subtitle tracks (PS
"dvd_subtitle" private streams demux/src/ps/mod.rs:171, MP4 sbtl/text/subt/clcp handlers
demux/src/mp4/mod.rs:529, MKV track type 0x11, TS "pgssub" demux/src/ts/mod.rs:365) — the
packets can exist in the pipeline, but no decoder provider consumes them (engine-info
baseline: decoders = mjpeg, pcm_s16le).
Confidence: HIGH — Verification: ran Cargo.toml scan (`rg` over all manifests; only
fuzz/Cargo.toml and a comment in crates/media-types/Cargo.toml match).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DvbDamage + Display::fmt | 96-198 | REAL | — | exhaustive variants, honest messages except F-004 |
| DvbRegionImage | 201-213 | REAL | — | plain data |
| DvbPageUpdate::is_clean | 241-243 | REAL | — | trivial |
| RegionState/Placement/DisplayDef | 248-265 | REAL | — | internal state |
| DvbDecoder::new / Default / reset | 286-318 | REAL | — | reset = full re-init, correct |
| DvbDecoder::damage | 320-326 | REAL | — | capped at 64, drops counted |
| DvbDecoder::decode_pes | 329-396 | REAL | S1/S2 | contract "never panics" false (F-001, F-002); PTS 90kHz->ns via checked Timestamp |
| DvbDecoder::page_segment | 398-434 | REAL | S4 | version-repeat early return, no page-id filter (F-007) |
| DvbDecoder::region_segment | 436-516 | REAL | S1 | depth-change stale-buffer panic root (F-001); cap 4 MiB enforced w*h<=4194304 |
| DvbDecoder::clut_segment | 518-541 | REAL | — | versioned skip per §7.2.4.2 |
| DvbDecoder::display_def_segment | 543-563 | REAL | S2 | 8192 clamp; delegates to underflowing parser (F-002) |
| DvbDecoder::object_segment | 565-618 | REAL | — | coding!=0 typed; empty bottom field reuses top (documented reference behavior) |
| DvbDecoder::paint_field | 621-741 | REAL | — | bounds-checked walk; map-table sub-blocks length-checked; guard before paint |
| DvbDecoder::paint_line | 744-785 | REAL | — | row slice bounded; rle::decode_* return used<=len (rle.rs:183,259 min-capped; 8-bit i<=len+1 but while-gate absorbs) |
| DvbDecoder::compose | 788-818 | REAL | S2/S3 | fill-only regions skipped (F-003); missing-region damage lie (F-004) |
| DvbDecoder::region_image | 820-849 | REAL | S1 | unclamped `bank[index as usize]` (F-001) |
| DvbCueBuilder::new/cues/layout_for | 863-949 | REAL | — | ns-exact half-open spans verified by test; `from_rgba(w,h,h,..)` passes baseline=height (valid, bottom row) |
| tests (5 helpers + 11) | 952-1265 | REAL | S3 | strong byte-level assertions; rle_overrun test weak (F-005) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_payload_is_typed_not_panicking | YES | asserts exact damage variant |
| truncated_object_is_typed | YES | asserts exact variant |
| full_page_lifecycle_two_updates | YES | exact RGBA bytes x3 display sets, repaint + clear |
| mode_change_clears_state | YES | asserts empty output after state 2 |
| bad_clut_id_uses_default_banks | YES | asserts variant + default-bank pixel |
| object_before_region_is_typed | YES | asserts variant |
| rle_overrun_is_clipped_and_typed | NO (half) | never asserts the RleOverrun damage (F-005) |
| region_over_cap_is_disabled | YES | asserts variant + empty regions |
| unknown_pixel_block_stops_object | YES | asserts variant |
| cue_builder_spans_and_positions | YES | ns-exact span + anchor rationals; note: only square 2x2 region ever passes through cues() — non-square path (from_rgba baseline/stride semantics) untested |
| timeout_bounds_trailing_page | YES | exact end ns |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "It never panics" | mod.rs:31 | FALSE | F-001 (release+debug OOB panic), F-002 (debug underflow) |
| "region pixel buffers are capped at 4 MiB" | mod.rs:70-72 | REAL | region_segment L450 saturating_mul gate; region_over_cap test |
| "every malformation ... is typed data" | mod.rs:31-32 | SHALLOW | true for the enumerated cases; depth-change and display-window underflow are NOT typed (F-001/F-002) |
| "Regions compose at the end-of-display-set segment (or ... once page+region+object have all been seen" | mod.rs:53-55 | REAL | decode_pes fallback L388-390 |
| docs/formats/sub-dvb.md | (W19-A09's doc) | not audited here | — |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped binaries (F-008). No dead functions inside the
file itself; every private fn is called.

## 8. Dependencies & flags
Depends on crate::compositing (bitmap/layout), media_types::time, testkit (dev). No feature
gates. `pub mod` exposure only consumed by in-crate tests/fuzz.

## 9. Open questions for the next agent
- W09-A08 (segment.rs): confirm the `wx - x + 1` u16 underflow (F-002) and check parse_region
  fill_color byte selection against the spec.
- W09-A04 (rle.rs): decode_line_8bit can return `used = data.len() + 1` (rle.rs:305-307 path);
  mod.rs's while-gate makes it safe today, but it is a fragile invariant — flag it.
- Whoever owns playback-engine wiring: is a subtitles decoder provider EVER planned (the
  session.rs "provider" seam), or is the whole crate product-dead?
