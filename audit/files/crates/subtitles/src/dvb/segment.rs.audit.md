# AUDIT: crates/subtitles/src/dvb/segment.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 643 / 11 fns+methods, 9 type defs, 9 tests |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — `subtitles` crate is a dependency of NOTHING except `fuzz/Cargo.toml` (rg over all Cargo.tomls). `lumen-cli` (the `lumen` binary) does not link it; apps/cli/src/pipeline_host.rs:1237-1238 drops subtitle frames: "Subtitle/data limbs are not wired in the first vertical. Frame::Subtitle(_) => Ok(())". Internal callers: dvb/mod.rs `decode_pes` → `walk_segments`/`parse_*` (crates/subtitles/src/dvb/mod.rs:343,396,435,515,537,560), reached by the crate's own tests + `fuzz_sub_dvb` only. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg "subtitles" **/Cargo.toml` (only fuzz + self) · `rg "\bSubtitle" --type rust` (CLI drops frames at pipeline_host.rs:1238) · `rg` over audit/runs/baseline/test-workspace.log.txt (NO subtitles test binary in the 1681-pass run — fail-fast aborted before the crate) · `rg` over audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269+ (subtitles unit binary: 267 passed, incl. all 9 `dvb::segment::tests` at log lines 5382-5390) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
Real, non-stub EN 300 743 segment grammar with genuinely good truncation typing and a decent unit suite — but it is dead code in the shipped product: no shipped binary links the `subtitles` crate, so no subtitle path (this file included) is reachable or test-proven "in the shipped build". Within the code itself there is one unchecked-arithmetic defect (u16 under/overflow in `parse_display_def` window extent — panics under overflow-checks, including on a spec-legal max=0xFFFF/min=0 window), one unverified-spec CLUT bit-layout divergence risk (`full_range` read from bit 0), an overstated "differential oracle" header claim (the oracle never executed), and an internal fallback inconsistency between `parse_region` and `depth()`. Tests are self-consistent (they bake in this file's own bit interpretation) so they prove regression resistance, not spec conformance.

## 2. What actually works (verified)
— 9/9 unit tests of this file passed in the orchestrator's repair run: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` lines 5382-5390 (`test dvb::segment::tests::header_walk_and_truncation ... ok` … `region_parse_4bit ... ok`), unit binary summary at line 5269ff ("running 267 tests … 267 passed; 0 failed"). NOT the primary INDEX #3 run — that run (1681 passed/1 failed) aborted fail-fast at apps/desktop `zero_alloc_transport` BEFORE the subtitles crate ever ran; the INDEX's "1681 passed" therefore contains zero evidence about this crate.
— `rg "walk_segments\|parse_page\|parse_region\|parse_clut\|parse_object\|parse_display_def" crates/subtitles/src/dvb/mod.rs` → all six entry fns called from `decode_pes`/segment dispatch (mod.rs:343,396,435,515,537,560), i.e. the module is wired into `DvbDecoder`, not orphaned inside the crate.

## 3. Findings

### F-001 [S1]
Where: crates/subtitles/src/dvb/segment.rs:373-384
Code:
```
    let (x, y, w, h) = if display_window {
        if data.len() < 13 {
            return None;
        }
        let x = u16::from_be_bytes([data[5], data[6]]);
        let wx = u16::from_be_bytes([data[7], data[8]]);
        let y = u16::from_be_bytes([data[9], data[10]]);
        let wy = u16::from_be_bytes([data[11], data[12]]);
        (x, y, wx - x + 1, wy - y + 1)
```
What is wrong: `wx - x + 1` and `wy - y + 1` are unchecked u16 arithmetic with two overflow paths: (a) malformed window with max < min (e.g. wx=0, x=100 → 0−100+1) underflows; (b) SPEC-LEGAL values wx−x = 0xFFFF (min=0, max=65535, both representable 16-bit addresses) overflow on the `+1`. Under overflow-checks (dev profile / `cargo test` default) both PANIC — violating the crate's stated "no panics on any input" contract (lib.rs:70-72) and the fuzz-harness contract "Never panics" (dvb/fuzz.rs:39-40), since `fuzz_sub_dvb` feeds arbitrary bytes straight into this path. In release (shipped/tested profile, overflow-checks off) it silently wraps: case (a) → width 65437, case (b) → width 0; mod.rs:549 then rejects both (`w <= 8192 && w > 0` gate) so the display definition is silently dropped without any damage event.
Why it matters: a fuzz corpus byte flip in a display-definition segment panics dev-profile runs of the crate's own fuzz-survival AC (the committed survival suite runs the LCG-mutated corpus; it passed only because the baseline ran `--release` where this wraps instead of panicking). The release path swallows the damage untyped. The baseline survival "proof" therefore did not exercise the panic mode its own contract forbids.
What correct looks like: `let w = (wx.max(x) - x + 1)` is still overflow-prone; compute in u32 and type-reject: `let w = u32::from(wx).checked_sub(u32::from(x)).and_then(|d| d.checked_add(1)).map(|v| v.min(0xFFFF) as u16)` with a `None`/damage return for max<min and clamp for the 0x10000 case; record a damage event either way.
Confidence: HIGH — Verification: read + arithmetic traced; panic mode not executed (no builds allowed) — release-wrap path corroborated by mod.rs:548-555 gate read.

### F-002 [S2]
Where: crates/subtitles/src/dvb/segment.rs:253-262
Code:
```
            let entry_id = rest[0];
            let flags = rest[1];
            let depth_2 = flags & 0x80 != 0;
            let depth_4 = flags & 0x40 != 0;
            let depth_8 = flags & 0x20 != 0;
            let full_range = flags & 1 == 1;
            if !(depth_2 || depth_4 || depth_8) {
                break;
            }
            let size = if full_range { 6 } else { 5 };
```
What is wrong: `full_range` is read from bit 0 (`flags & 1`). EN 300 743 §7.2.4's CLUT-entry syntax orders the flag bits 2-bit/4-bit/8-bit/full_range followed by 4 reserved bits — i.e. full_range at mask 0x10 with the reserved nibble at bits 3-0. If the spec ordering is as read, every limited-range entry on a real stream (flag bit 4 set, bit 0 clear) is parsed as limited-range-but-encoded-as… inverse: full-range entries parse as 5-byte limited entries, and limited entries as 6-byte full entries — each misparse also DESYNCS the entry walk (`size` wrong → subsequent `rest` offsets wrong → garbage colors or walk stop). The depth-flag masks 0x80/0x40/0x20 (bits 7-5) match the same table's first three bits, making bit 0 for the FOURTH flag inconsistent with the table's own ordering.
Why it matters: if the mask is wrong this breaks every real DVB CLUT (wrong colors + entry desync); if it is right, nothing — but nothing in the tree can tell: no spec copy, no ffmpeg source, and the claimed differential oracle never ran (F-003). The in-tree test (line 575: flags `0b0010_0001` — bit 0 set) simply encodes the code's own interpretation.
What correct looks like: read the spec bit (verify mask 0x10 vs 0x01 against EN 300 743 table 8 / ffmpeg `dvbsub_parse_clut_segment`), or at minimum accept both encodings tolerantly is NOT correct (size differs) — it must match the wire format exactly; add a fixture from a real broadcast capture once the mask is pinned.
Confidence: MEDIUM — Verification: UNVERIFIED against the authoritative source — no EN 300 743 text or ffmpeg source in the repo, no network access, no corpus file with a real CLUT segment. Blocked: external spec access. REQUEST-ORCH-RUN: differential of `ClutSegment::entries` vs ffmpeg dvbsubdec on a synthetic full-range CLUT segment.

### F-003 [S3]
Where: crates/subtitles/src/dvb/segment.rs:5-9
Code:
```
//! Bit layouts follow the reference decoders (ffmpeg `dvbsubdec`,
//! this wave's differential oracle) exactly: page region entries are
//! 6 bytes (`id`, reserved, x:16, y:16), region object entries are 6
//! bytes (+2 for character-coded objects) with the object type in the
//! top 2 bits of byte 2 and 12-bit x/y packed around it.
```
What is wrong: "this wave's differential oracle" is a claim of executed evidence that does not exist. The wave's own test file states the opposite: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:24-31 — "the reference-encoder differential is NOT generatable here and the AC rides on the hand-crafted bit vectors in the unit suites (tables cross-checked line-by-line …)". "Cross-checked line-by-line" is a manual review, not a run. Additionally the described char-object layout (y at bytes 4-5, fg/bg appended at bytes 6-7) contradicts the EN 300 743 §7.2.3 syntax order, where `foreground_pixel_code`/`background_pixel_code` precede `object_vertical_position` (which would put fg/bg at bytes 4-5 and y at bytes 6-7 for char objects). One of the two orders is wrong and the tree contains zero evidence to arbitrate.
Why it matters: the comment is the only "evidence" a future reader has that these bit layouts are proven; it misrepresents a manual cross-check as an executed differential oracle. Character-coded subtitle objects (types 1/2) would get wrong positions AND wrong colors if the order is wrong.
What correct looks like: either run the differential and cite it, or rewrite the comment to say "layouts hand-derived from a reading of EN 300 743 + ffmpeg source; differential NOT executed (see tests/dvb_vobsub_fuzz_survival.rs)" and pin the char-object order against the spec/ffmpeg.
Confidence: HIGH (that the oracle never ran — proven by the test file's own honesty note); UNVERIFIED (which bit order is actually correct).
Confidence: HIGH / Verification: read (contradiction between two in-tree files proven); external bit-order question UNVERIFIED.

### F-004 [S3]
Where: crates/subtitles/src/dvb/segment.rs:141-165
Code:
```
    pub fn depth(&self) -> u8 {
        let depth = 1u16 << (self.depth_code & 7);
        if (2..=8).contains(&depth) {
            depth as u8
        } else {
            4
        }
    }
```
plus in `parse_region` (lines 157-165):
```
    let depth_code = (data[6] >> 2) & 7;
    let depth = 1u16 << (depth_code & 7);
    let (fill_color, objects) = if depth == 8 {
        (data[8], &data[10..])
    } else if depth == 4 {
        (data[9] >> 4, &data[10..])
    } else {
        ((data[9] >> 2) & 3, &data[10..])
    };
```
What is wrong: for an invalid depth code (0,4,5,6,7 → depths 1,16,32,64,128) `parse_region` interprets `fill_color` via the 2-bit branch (`(data[9] >> 2) & 3`) while `depth()` reports the fallback 4 — a region painted with a 2-bit-indexed fill under a 4-bit depth. The two functions disagree about the same malformed input. (`depth()`'s `& 7` is also dead: `depth_code` is already masked at line 157.)
Why it matters: fill colors of malformed-depth regions are read from the wrong field; downstream `RegionState.depth` (mod.rs:443-464) uses `depth()` = 4 while the fill byte was a 2-bit CLUT index — wrong fill color, inconsistent state. Blast radius limited to invalid depth codes on malformed streams.
What correct looks like: parse_region should derive `fill_color` from the SAME depth `depth()` reports (i.e. the 4-bit interpretation `(data[9] >> 4)` for the fallback), or carry an explicit `depth_valid: bool` so the caller can drop the fill.
Confidence: HIGH — Verification: read + both code paths traced.

### F-005 [S4]
Where: crates/subtitles/src/dvb/segment.rs:111-121
Code:
```
        // `as_chunks` yields (chunks, remainder): a trailing partial
        // entry (< 6 bytes) is dropped here; its length is
        // damage-accounted upstream.
        let (entries, _rest) = self.regions.as_chunks::<6>();
```
What is wrong: the doc comment says the dropped trailing partial entry is "length-accounted by the damage ledger" (line 110) / "damage-accounted upstream", but `parse_page`/`region_refs` produce no such accounting signal: the remainder is discarded (`_rest`), and no caller can distinguish a whole page from a truncated one (mod.rs `page_segment` records nothing about it — verified mod.rs:391-433 has no partial-entry damage kind).
Why it matters: the comment asserts a guarantee the code does not provide; a truncated page-composition region list silently renders fewer regions with zero damage events, contradicting the crate's typed-damage story.
What correct looks like: return the remainder length (e.g. `region_refs() -> (impl Iterator, usize)` or a `trailing: u8` field on PageSegment) and let mod.rs record a `TruncatedPayload` damage event.
Confidence: HIGH — Verification: read of this file + mod.rs:391-433 (no partial-entry damage path exists).

### F-006 [S4]
Where: crates/subtitles/src/dvb/segment.rs:196-224
Code:
```
            let (fg, bg, size) = if object_type == 1 || object_type == 2 {
                if objs.len() < 8 {
                    break;
                }
                (Some(objs[6]), Some(objs[7]), 8)
            } else {
                (None, None, 6)
            };
```
What is wrong: a truncated char-coded object entry (<8 bytes but ≥6) silently `break`s with no length signal to the caller — same unaccounted-truncation gap as F-005, on the region-object walk.
Why it matters: truncated object lists on malformed streams produce silently-fewer placements, no damage event; comment discipline ("length-accounted by the damage ledger", line 110) not honored here either.
What correct looks like: surface the consumed/dropped byte count so mod.rs can emit a typed damage event, as done for the top-level walk (`walk_segments` returns `used`).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SegmentKind::of` | 26-38 | REAL | — | Exhaustive match incl. Unknown(u8) typed skip; no panics; trivially correct table (type codes 0x10-0x14,0x80,0xFF match EN 300 743 table 2). |
| `walk_segments` | 51-73 | REAL | — | Bounds-checked (`p+6<=len`, `data_start+len<=len`); usize adds cannot overflow (len ≤ 65535); returns `used` for caller damage accounting — mod.rs:343-350 uses it. No sync-resync (stops at first bad byte) — matches doc claim, damage-typed upstream. |
| `parse_page` | 86-96 | REAL | — | `len<2 → None`; version/state nibble extraction correct vs §7.2.2; state 3 (reserved) passed through raw for caller. |
| `PageSegment::region_refs` | 111-121 | REAL | S4 | 6-byte chunks, BE16 x/y; drops partial tail unaccounted (F-005). |
| `RegionSegment::depth` | 143-150 | REAL | S4 | Fallback 4 for invalid codes; redundant `& 7` (already masked at 157). |
| `parse_region` | 153-177 | REAL | S3 | 10-byte minimum; depth branches 8/4/2 match ffmpeg layout; fill/depth disagreement on invalid codes (F-004). |
| `RegionSegment::object_refs` | 196-224 | REAL | S3/S4 | 12-bit x/y right-aligned packing ((b2&0x0F)<<8|b3) consistent with page-address convention; char-object fg/bg order vs spec UNVERIFIED (F-003); silent truncation break (F-006); allocates a Vec per call (fine: ≤ region object count, cold path). |
| `parse_clut` | 234-243 | REAL | — | 2-byte minimum, borrow view; fine. |
| `ClutSegment::entries` | 249-301 | REAL | S2 | Full-range flag bit 0 vs spec bit-4 risk (F-002); limited-range Y6/Cr6/Cb6/T2 unpack (281-295) verified bit-by-bit correct MSB-first; stops on invalid flags/truncation — matches its own comment. |
| `parse_object` | 317-349 | REAL | — | `len<3 → None`; coding_method≠0 → typed early return (caller mod.rs:568-573 records `ObjectCodingUnsupported`); `top_len+bottom_len>rest.len() → None` — usize adds bounded ≤131070, no overflow; both field slices bounds-checked. |
| `parse_display_def` | 364-393 | REAL | S1 | u16 under/overflow on window extent (F-001); non-window path (BE16+1) correct. |
| 9 struct/enum defs | 13-23,41-46,77-84,102-106,126-139,182-191,228-232,307-315,353-362 | REAL | — | Pure data, derives only, no logic. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `header_walk_and_truncation` (401-416) | YES | Asserts consumed=15, seg count, kinds, page_id, payload slices — mutation of sync/len handling flips them. |
| `bad_sync_stops_walk` (418-423) | YES | used=0 + empty vec pinned. |
| `page_region_refs_parse` (426-455) | YES | Two concrete (id,x,y) tuples + trailing partial entry dropped. |
| `region_parse_4bit` (458-492) | YES | Every header field asserted incl. fill color `(0xF4>>2)&3` (2-bit). |
| `region_object_12bit_positions` (495-551) | YES for regressions / NO for spec conformance | Asserts x=0xABC, y=0x123, fg=9, bg=10 — but the vectors are hand-built with the code's own bit convention; a spec-conformance error in y/fg/bg order or alignment passes this test (F-003). |
| `region_invalid_depth_falls_back_to_4` (554-570) | YES for depth() / NO for F-004 | Pins depth()=4 but never asserts the fill_color of the invalid-depth region — the F-004 inconsistency is unobservable to the suite. |
| `clut_entries_full_and_limited` (573-601) | YES for unpack math / NO for F-002 | Limited-range unpack values pinned (y=0b10100100 etc.); but full_range bit position is baked in (flags 0b0010_0001) — a wrong mask passes. |
| `object_parse_fields` (604-622) | YES | Field lengths, slices, coding-method-1 early return, truncation None all pinned. |
| `display_def_parse` (625-641) | PARTIAL | Pins 720x576, window +1 semantics (360x337), truncated→None — but has NO case for wx<x or wx−x=0xFFFF (F-001 untested). |
All 9 executed: orchrun-test-workspace-nofailfast.log.txt:5382-5390, all `ok`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Bit layouts follow the reference decoders (ffmpeg dvbsubdec, this wave's differential oracle) exactly" | segment.rs:5-6 header | SHALLOW (manual cross-check only) | tests/dvb_vobsub_fuzz_survival.rs:24-31 admits the differential was NOT generatable; no executed oracle in tree or baseline. |
| lib.rs:104-112 "Carriage landed earlier: TS stream type 0x06 → private PES (demux/src/ts), the demuxer hands the decoder the payload after the PES header" | crates/subtitles/src/lib.rs | FALSE (as a reachability claim) | demux/src/ts/mod.rs:355 maps 0x06 to the codec string "ts-private-pes" only; NO decoder consumes it (rg: zero non-test callers of `subtitles::dvb` outside the crate/fuzz); apps/cli/src/pipeline_host.rs:1238 discards Frame::Subtitle. The payload is handed to nobody. |
| "page state machine … reducing to CompositedCues" (lib.rs:104-111) | lib.rs | REAL-UNPROVEN (unit-level only) | mod.rs compose path exists and 267 unit tests pass (orchrun log), but never executed from any shipped entry point. |

## 7. Dead / duplicated / unreachable code
The entire file (and crate) is unreachable from shipped entry points: only `fuzz/` (not shipped) and the crate's own tests link it. No intra-file dead code found (all fns called from dvb/mod.rs dispatch).

## 8. Dependencies & flags
Crate deps: media-types only (+dev testkit, media-clock). No feature flags gate this file; no cfg(test) leakage into production paths. Workspace lints (deny unwrap/expect/panic/todo) hold — zero unwrap/expect/panic in this file (verified by read).

## 9. Open questions for the next agent
1. F-002/F-003 need one authoritative bit-layout arbitration: EN 300 743 tables 7/8 (or ffmpeg dvbsubdec source) for (a) CLUT full_range mask, (b) char-object fg/bg vs y byte order. Any wave-16 corpus agent: a single real .ts DVB capture settles both.
2. dvb/mod.rs (A-agent for that file): `display_def_segment` silently drops rejected display definitions (no damage event) — related to F-001's release path.
3. The primary baseline INDEX #3 test summary (1681 passed) excludes the whole subtitles crate (fail-fast abort); any agent citing it as workspace-wide test evidence must cite orchrun-test-workspace-nofailfast.log.txt instead.
