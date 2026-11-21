# AUDIT: crates/subtitles/src/dvb/mod.rs (part1, lines 1–632 of 1265)
> Chunk scope: part1 = lines 1–632 (boundaries from wave-00
> assignments.tsv LOC column: part1=632, part2=633). Covers: module
> docs, `MAX_DAMAGE_EVENTS`/`MAX_REGION_PIXELS` consts, `DvbDamage` +
> `Display`, `DvbRegionImage`, `DvbPageUpdate`/`is_clean`, the private
> state structs (`RegionState`, `Placement`, `DisplayDef`),
> `DvbDecoder` struct, `Default`, `new`, `reset`, `damage`,
> `decode_pes`, `page_segment`, `region_segment`, `clut_segment`,
> `display_def_segment`, `object_segment`. `paint_field` (615–746)
> straddles the 632/633 boundary and is audited whole in the part2
> file (W09-A06), per the ids.rs chunk precedent; it is listed in the
> table below only for boundary accounting. `paint_line`, `compose`,
> `region_image`, `Rle`, `DvbCueBuilder` (750–947) and `mod tests`
> (950–1265) are part2 (A06) — read here for cross-reference only.

| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 632 (of 1265) / 21 production units + 1 straddler |
| File verdict | UNREACHABLE (real, internally-tested state machine; zero shipped-entry-point callers — see Reachability) |
| Reachable from shipped entry point? | **NO.** `rg -l "subtitles::" crates apps` → only `crates/subtitles/src/{srt,vtt}/mod.rs` and `crates/subtitles/tests/*` (the crate's own tests). `rg -n "DvbDecoder|decode_pes|DvbCueBuilder" crates apps` → only `crates/subtitles` itself + docs. The only external dependant is `fuzz/Cargo.toml:50` — the fuzz scaffold is its own workspace root (root `Cargo.toml` members `["crates/*","apps/*","tools/xtask","tests/integration/first-vertical","tools/mem-audit","tools/sweep"]` does not include `fuzz/`) and is "NOT wired into any gate" (fuzz/Cargo.toml header). `apps/cli` (the `lumen` binary) and the excluded `apps/desktop-tauri` have no path into `subtitles`. `lumen engine-info` features (baseline `lumen-engine-info.txt`): `first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor` — no subtitle lane. playback-engine's "class-S subtitle seam" (`crates/playback-engine/src/pipeline.rs:1036-1038`) deselects subtitle tracks when no provider can build a decoder — and no provider exists. |
| Findings | S0: 0 S1: 1 S2: 2 S3: 3 S4: 1 |
| Commands run | `sed -n '1,340p;340,635p;635,850p;850,1069p;1070,1266p' crates/subtitles/src/dvb/mod.rs` (full file read for context; my range audited); `sed -n '462,470p;826,831p;79,82p;470,476p;391,395p;568,574p;548,556p;24,32p;57,60p'` (verbatim quote verification — all quotes below match); `cat crates/subtitles/src/dvb/segment.rs` (segment grammar cross-check: `parse_region` fill_color is at region depth; `walk_segments` returns consumed bytes); `cat crates/subtitles/src/dvb/rle.rs` (map-table write values: raw codes at native depth, `map2to8=[0,0x77,0x88,0xFF]`, `map4to8=[0,0x11..0xFF]` at depth 8); `cat crates/subtitles/src/dvb/clut.rs` (`bank(depth)`: 2→4 entries, 4→16, 8→256); `rg -n "DvbDecoder|decode_pes|DvbCueBuilder|DvbPageUpdate" .` (caller map: crate-internal only); `rg -l "subtitles::" crates apps`; `rg -ni "subtitle" crates/demux/src apps/cli/src` (no wiring: only playback-engine seam comments + demux `ts::tests::dvb_text_prefixes_and_composition` — DVB text in TS tables, not this decoder); `rg -n "pub fn from_parts|pub struct Nanoseconds" crates/media-types/src/time.rs` (PTS conversion semantics); `rg -n "deps/subtitles|test result" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (subtitles tests: lib 267/0, dvb_vobsub_fuzz_survival 2/0 — ran ONLY in the orchestrator's no-fail-fast rerun, not in baseline runs #3/#4 which aborted at `apps/desktop`/`first-vertical` before reaching the `subtitles` package). |

## 1. One-paragraph verdict
This is a genuinely implemented EN 300 743 page state machine — versions, acquisition/mode-change resets, capped typed damage, per-region CLUT-index pixel buffers, half-open cue spans — with 267 passing lib tests plus a fuzz-survival suite (cited from `orchrun-test-workspace-nofailfast.log.txt`, NOT from the primary baseline test logs, which aborted before the `subtitles` package ran). But it is dead code in the shipped product: nothing outside `crates/subtitles` (and the unwired `fuzz/` scaffold) calls it, so no DVB subtitle can ever be decoded by `lumen` or `lumen-tauri`. Worse, the module's two headline contracts are both false as written: "It never panics" (line 30) is contradicted by a reachable index-out-of-bounds panic (F-001), and "Never … unbounded allocation" (line 59) is contradicted by ~5 GiB of attacker-driven allocation from a ~20 KB PES (F-002).

## 2. What actually works (verified)
— State machine core (page version no-op, state 1/2 reset, region alloc/fill, CLUT versioning, object paint, end-of-display-set compose): verified by reading the code end-to-end against segment.rs/rle.rs/clut.rs, and by baseline run `orchrun-test-workspace-nofailfast.log.txt:5269-5639` → `subtitles` lib `test result: ok. 267 passed; 0 failed`, incl. `full_page_lifecycle_two_updates` with pixel-exact RGBA assertions and `dvb_vobsub_fuzz_survival` `2 passed`.
— Fuzz survival claim at crate level: `tests/dvb_vobsub_fuzz_survival.rs` runs seeded LCG mutation sweeps through `fuzz_sub_dvb` — same log, `test result: ok. 2 passed`.

## 3. Findings

### F-001 [S1] Region depth change keeps stale 8-bit pixel buffer → bank index-out-of-bounds PANIC at compose
Where: crates/subtitles/src/dvb/mod.rs:462-470 (root cause, part1); panic site :826-830 (`region_image`, part2 — cross-referenced)
Code:
```rust
        match self.regions.get_mut(&region.region_id) {
            Some(existing) if existing.width == w && existing.height == h => {
                existing.depth = depth;
                existing.clut_id = region.clut_id;
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
```
Panic site (part2, read for cross-reference):
```rust
        let bank = clut.bank(region.depth);
        let mut rgba = Vec::with_capacity(region.pbuf.len() * 4);
        for &index in &region.pbuf {
            let c = bank[index as usize];
```
What is wrong: when a region composition arrives for an existing region with the SAME width/height but a SMALLER depth, the code updates `existing.depth` (and, without `region.fill`, leaves `pbuf` untouched). `pbuf` still holds indices painted at the old, deeper depth — e.g. 8-bit paint writes raw `v ∈ 1..=255` (`rle.rs:285 row[x] = v`) or map2to8 values `0x77/0x88/0xFF` (`rle.rs:134-135`). At compose, `clut.bank(region.depth)` for depth 2 returns a 4-entry slice (`clut.rs:171-177`), and `bank[index as usize]` is an unchecked slice index: `bank4[0x77]` panics.
Trigger (traced, no code run): PES 1 = page comp (region 7) + region comp (7, w×h valid, depth_code 4→depth 8, fill=false) + object data painting value 0xFF + end-of-display-set → composes fine (bank256). PES 2 = region comp (7, same w/h, depth_code 1→depth 2, fill=false) + end-of-display-set → `compose()` sees the region still dirty, `region_image` indexes `bank4[0xFF]` → panic. Two ~40-byte segments.
Why it matters: the module's central contract is "**It never panics**; every malformation … is typed data" (`mod.rs:30-32`), and the fuzz-survival suite exists to prove exactly that. This is a malformed-but-parseable input (both region compositions are individually well-formed EN 300 743 segments) that panics the decoder. In any future wiring (the doc's own "next step": Lead wires `DvbDecoder` into the TS track path, `docs/formats/sub-dvb.md:79`), a hostile or merely buggy DVB stream kills the process. No test covers a depth change (checked: all 12 tests in `mod tests` use one depth per region).
What correct looks like: on depth change, reallocate/clear `pbuf` (ffmpeg's reference reallocate-on-geometry-change behavior), or clamp/mask the index at compose (`bank[index as usize & (bank.len()-1)]` is wrong; the correct fix is clearing the buffer on depth change), or size all banks to 256 entries. At minimum, a `DvbDamage` variant.
Confidence: HIGH — Verification: read (full trace through `region_segment` → `paint_field`/`paint_line` → `rle.rs` write values → `clut.rs::bank` → `region_image` unchecked index). Not executed (no cargo runs permitted; REQUEST-ORCH-RUN below).

### F-002 [S2] Per-region 4 MiB cap with NO page-level cap: ~1 GiB pixel buffers + ~4 GiB RGBA per display set from a ~20 KB PES
Where: crates/subtitles/src/dvb/mod.rs:79-82 (the cap), :470-476 (allocation), doc claim :57-60
Code:
```rust
/// Region pixel-buffer cap: width × height × 1 byte (CLUT indices),
/// 4 MiB — twice a 1080p canvas, generous against the 320×256-typical
/// broadcast regions, tight against hostile dimensions (D-2).
pub const MAX_REGION_PIXELS: u32 = 4 * 1024 * 1024;
```
```rust
            _ => {
                let mut pbuf = vec![0u8; (w * h) as usize];
```
What is wrong: the cap is per-region, but nothing bounds the number of regions in a page: region ids are u8 (≤256), page entries dedupe to ≤256 (`mod.rs:428-430`). A single PES of roughly 20 KB — page composition listing 256 regions (1536 B) + 256 region compositions at 4 MiB each (~10 KB) + 256 one-line object data segments (~6 KB) to mark them dirty (`paint_line` sets `dirty = true` before its early return, `mod.rs:762-764`) — allocates 256 × 4 MiB = 1 GiB of `pbuf`, then `compose`→`region_image` (`mod.rs:827`, part2) allocates `pbuf.len() × 4` RGBA per region: another 4 GiB, all retained in one `DvbPageUpdate`. The doc's "Never … unbounded allocation (region pixel buffers are capped at 4 MiB, D-2)" presents the per-region cap as if it bounded the decode; it does not.
Why it matters: OOM abort / memory-exhaustion DoS from a 20 KB attacker-controlled payload, in a decoder whose contract promises boundedness and whose fuzz-survival evidence (`docs/formats/sub-dvb.md:52-56`) claims "zero panics/hangs/unbounded allocations". The LCG-mutated corpus simply never built a 256-region page.
What correct looks like: a page-total pixel budget (e.g. sum of region areas ≤ some MiB), or bounding `display_extent` and rejecting regions that don't fit it, like the PGS path's canvas cap.
Confidence: HIGH — Verification: read (arithmetic traced: 256 × 4 MiB; `dirty` set unconditionally at `paint_line` entry; `region_image` `Vec::with_capacity(pbuf.len()*4)`); not executed (REQUEST-ORCH-RUN below).

### F-003 [S2] `seg.page_id` ignored for every segment — multi-page streams corrupt state; the "demuxed upstream" assumption has no upstream
Where: crates/subtitles/src/dvb/mod.rs:391-395 (and the absence of any `page_id` check in `region_segment`/`clut_segment`/`object_segment`/`display_def_segment`)
Code:
```rust
    fn page_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.saw_page = true;
        if self.page_id.is_none() {
            self.page_id = Some(seg.page_id);
        }
```
What is wrong: EN 300 743 multiplexes ancillary pages in the same PES stream; `walk_segments` parses a `page_id` for every segment (`segment.rs:59`, field `RawSegment::page_id`), and this decoder locks the FIRST page id then processes **every** subsequent segment regardless of its `page_id`. A region/CLUT/object segment belonging to page B is applied to the page-A state machine (version counters, region maps, placements mix). The struct doc admits the assumption — "EN 300 743 multiplexes ancillary pages, demuxed upstream" (`mod.rs:213-215`) — but no such demux exists anywhere: this crate is unreached (see Reachability) and the TS demuxer has zero subtitle wiring (`rg -ni "subtitle" crates/demux/src` → 0 hits).
Why it matters: the moment the Lead wires this decoder into the TS track path (the documented next step), any conformant multi-page DVB stream (common in broadcast) produces cross-contaminated pages — wrong regions composed, phantom version no-ops, misplaced objects. The documented precondition is unenforceable and unimplemented.
What correct looks like: filter segments by `self.page_id` (one `if seg.page_id != expected { return; }` per handler, or one check in the `decode_pes` dispatch loop).
Confidence: HIGH — Verification: read (dispatch loop `mod.rs:352-365` passes `seg` wholesale; no handler reads `seg.page_id`).

### F-004 [S3] Region-composition version semantics not implemented (every segment rebuilds placements)
Where: crates/subtitles/src/dvb/mod.rs:490-511
Code:
```rust
        // A new region composition replaces the old placement list
        // for this region.
        for list in self.objects.values_mut() {
            list.retain(|p| p.region_id != region.region_id);
        }
```
What is wrong: EN 300 743 §7.2.3 gives region compositions a version; a repeated version means "unchanged". The decoder honors version skips for pages (`mod.rs:404`), CLUTs (`:523-528`) and the display definition (`:545-547`) but not for regions: every region-composition segment wipes and rebuilds that region's placements, even a byte-identical repeat. Object data arriving before a repeated region composition (ordering damage, out-of-order PES) loses its placement and is typed `ObjectWithoutRegion`.
Why it matters: unnecessary state churn and dropped objects on streams that repeat region compositions per display set — realistic broadcast behavior.
What correct looks like: `if existing.version == region.version { return; }` mirroring `clut_segment`.
Confidence: MEDIUM — Verification: read (asymmetry verified in code; broadcast repeat frequency asserted from spec knowledge, not from a corpus — none exists, see sub-dvb.md "no corpus lane").
### F-005 [S3] Fill-only regions never compose (`fill` clears `dirty`), and reference divergence is untested
Where: crates/subtitles/src/dvb/mod.rs:466-469 (with compose gate at :802, part2)
Code:
```rust
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
                    existing.dirty = false;
                }
```
What is wrong: a region whose only content is `region_fill_flag` + a non-transparent `fill_color` (a solid box — legal EN 300 743, occasionally used as a background/matte) is marked NOT dirty, and `compose()` skips non-dirty regions (`mod.rs:802-804`), so it produces no image. The test suite pins this behavior (`mode_change_clears_state`, `mod.rs:1122-1127`: fill=true region, no object → `assert!(up.regions.is_empty())`), so it is intentional — but the reference decoders compose the region's pixel buffer regardless of object presence, so a fill-only subtitle renders in VLC and renders nothing here.
Why it matters: silent content loss for a legal stream shape; the divergence is undocumented (neither the module doc nor sub-dvb.md mentions fill-only regions being dropped).
What correct looks like: fill with a non-default color should set `dirty = true` (fill_color 0 = the transparent default keeps `dirty = false`), or the divergence should be a documented damage event.
Confidence: MEDIUM — Verification: read for this code's behavior (HIGH); the ffmpeg-composes-fill-only-regions claim is UNVERIFIED (no ffmpeg source in sandbox; sub-dvb.md itself notes the differential is not generatable).
### F-006 [S3] Object coding method 2 (progressive pixels) refused — legal content dropped
Where: crates/subtitles/src/dvb/mod.rs:568-574
Code:
```rust
        if object.coding_method != 0 {
            self.damage(DvbDamage::ObjectCodingUnsupported {
                object_id: object.object_id,
                coding_method,
            });
            return;
        }
```
What is wrong: EN 300 743 defines coding method `10` (2) as progressive coding of pixels — the same RLE tables without field interleave. The decoder refuses everything but 0 (bitmap). The damage doc claims "EN 300 743 allows" the refusal; the spec defines the format, and reference decoders render it, so this is a capability gap dressed as damage-typing.
Why it matters: any stream using progressive-coded objects (rare but standard-defined) loses all its subtitle content with only a damage line as trace.
What correct looks like: treat coding 2 as coding 0 with field=0 stepping 1 row per `0xF0` (the `paint_field` loop already parameterizes the row step — `y += 2` at `:733` — a stride parameter covers it), or document the drop in sub-dvb.md's deferred list (it is absent there).
Confidence: MEDIUM — Verification: read for the refusal (HIGH); reference-decoder support for method 2 is UNVERIFIED in-sandbox.
### F-007 [S4] Invalid display-definition dimensions silently ignored — no damage event
Where: crates/subtitles/src/dvb/mod.rs:548-556
Code:
```rust
        let (w, h) = (u32::from(dds.width), u32::from(dds.height));
        if w > 0 && h > 0 && w <= 8192 && h <= 8192 {
            self.display_def = Some(DisplayDef {
                width: w,
                height: h,
```
What is wrong: a DDS with w/h = 0 or > 8192 is dropped without any `DvbDamage` event — the old (or default 720×576) extent silently persists. The contract says "every malformation … is typed data" (`mod.rs:31-32`).
Why it matters: damage-ledger consumers (the future seam) cannot distinguish "no DDS" from "DDS rejected"; debugging silent extent fallbacks costs real time.
What correct looks like: a `DvbDamage::DisplayDefInvalid { w, h }` push in the else branch.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `MAX_DAMAGE_EVENTS` const | 75-77 | REAL | — | 64; enforced in `damage`; overflow counted (`damage_dropped`) and surfaced as `DamageOverflow` per-PES (`:375-380`). |
| `MAX_REGION_PIXELS` const | 79-82 | REAL | — | Per-region only — no page-level cap → F-002. Doc comment overstates the guarantee. |
| `enum DvbDamage` | 85-127 | REAL | — | 13 variants; every decoder refusal is typed. Used by tests via `matches!`. |
| `Display for DvbDamage::fmt` | 129-195 | REAL | — | Pure formatting, one arm per variant, no panics. Untested directly (no test asserts the strings) — cosmetic only. |
| `struct DvbRegionImage` | 199-209 | REAL | — | Plain data (x, y, w, h, rgba). `rgba` length invariant `w*h*4` asserted by fuzz-survival suite (sub-dvb.md evidence). |
| `struct DvbPageUpdate` | 212-228 | REAL | — | Plain data carrier; `page_id` doc admits the multiplexing assumption → F-003. |
| `DvbPageUpdate::is_clean` | 230-235 | REAL | — | `damage.is_empty()`. Trivial and correct; exercised by tests. |
| `struct RegionState` (private) | 239-247 | REAL | — | `pbuf` = CLUT indices at 8-bit width; depth invariance is the F-001 hazard. |
| `struct Placement` (private) | 251-256 | REAL | — | Copy data. |
| `struct DisplayDef` (private) | 259-264 | REAL | — | Copy data; version used for the §7.2.1 skip. |
| `struct DvbDecoder` | 267-285 | REAL | — | State machine fields; `objects: HashMap<u16, Vec<Placement>>` unbounded in count (u16 ids → 65 536 entries max; placement vecs 1 per region per object — bounded by 256 regions each: fine). |
| `Default for DvbDecoder` | 287-291 | REAL | — | Delegates to `new`. |
| `DvbDecoder::new` | 294-313 | REAL | — | All-empty init. Clippy-clean. |
| `DvbDecoder::reset` | 315-318 | REAL | — | `*self = Self::new()` — seek/discontinuity hook. No caller outside tests (unreachable, like the rest). |
| `DvbDecoder::damage` | 320-326 | REAL | — | Cap + drop count. Correct. |
| `DvbDecoder::decode_pes` | 328-389 | REAL | — | Dispatch loop; PTS→ns via `Timestamp::from_parts(ticks,1,90_000).ok().and_then(as_nanos().ok())` — negative or overflowing ticks silently become `None` (cue start `Missing`), untested, acceptable; `used < payload.len()` mislabels a bad-sync stop as `LengthOverrun` (cosmetic). Fallback compose `:368-370` matches the documented reference behavior. Damage take/reset discipline correct (`:374-380`). |
| `DvbDecoder::page_segment` | 391-431 | REAL | S2 (F-003) | First-page lock only; no page_id filter. Version no-op `:404-409` correct per §7.2.2.2 (object data still processed after — right). Trailing-entry damage accounting `:419-425` consistent with `region_refs` chunk-drop (segment.rs:111-121). Duplicate region id → keep first, documented. |
| `DvbDecoder::region_segment` | 433-512 | REAL | S1/S2/S3 (F-001, F-002, F-004, F-005) | Depth-validity damage `:443-449` right; size cap + removal `:450-460` right; same-dims update path is the F-001 stale-pbuf root; `_` arm allocates `w*h` (≤4 MiB) with no page budget (F-002); placement guard `:501-509` reads CURRENT region dims (consistent); placement rebuild has no version gate (F-004); fill clears `dirty` (F-005). |
| `DvbDecoder::clut_segment` | 514-534 | REAL | — | Version skip `:523-528` per §7.2.4.2; `Clut::from_entries` defaults banks first (partial redefinition keeps old entries) — matches reference shape. |
| `DvbDecoder::display_def_segment` | 536-556 | REAL | S4 (F-007) | Version skip; 1..=8192 extent filter; silent else. Window case relies on `parse_display_def` (segment.rs:373-381) whose `wx - x + 1` u16 arithmetic can underflow on hostile window coords → debug-build panic / release wrap — segment.rs owns that (flagged to A06/segment owner in Open Questions). |
| `DvbDecoder::object_segment` | 558-611 | REAL | S3 (F-006) | Coding≠0 refused (F-006); `break` on hostile top-field skips remaining placements (reasonable); bottom-field result ignored (`:603-609`) — harmless (nothing follows); empty-bottom→reuse-top matches the documented reference rule `:596-602`. |
| `DvbDecoder::paint_field` | 613-746 | (part2 — straddler) | — | Starts at 615 inside part1, ends 746 in part2; audited whole in W09-A06's part2 file per chunk precedent. Listed for boundary accounting only. |

## 5. Test honesty (tests live in part2 lines 950–1265; they exercise part1 functions)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_payload_is_typed_not_panicking` | Yes | Asserts damage variant on empty payload (`decode_pes` `:344-350`). |
| `truncated_object_is_typed` | Yes | 2-byte object payload → `TruncatedPayload{kind:"object-data"}`. |
| `full_page_lifecycle_two_updates` | Yes | Pixel-exact RGBA asserts over fill + 2-bit paint + interlaced bottom field + repaint + empty-page clear. Strongest test in the module. Mutation check: swap `map2to4` for `map2to8` at `:657-661` → top-row colors change → `assert_eq!(&r.rgba[4..8], &[255,255,255,255])` fails. |
| `mode_change_clears_state` | Yes | State-2 page drops regions (also pins the F-005 fill-only-drop behavior — it would fail if fill set `dirty=true`, i.e. it enshrines the divergence). |
| `bad_clut_id_uses_default_banks` | Yes | Asserts damage variant AND default-bank white pixel. |
| `object_before_region_is_typed` | Yes | Placement-less object → `ObjectWithoutRegion`. |
| `rle_overrun_is_clipped_and_typed` | Partially | Asserts the row painted; does NOT assert the `RleOverrun` damage variant appears (the 280-px run in a 3-wide row must clip-damage). Weakened assertion. |
| `region_over_cap_is_disabled` | Yes | 4000×4000 → `RegionSizeInvalid` + no regions. |
| `unknown_pixel_block_stops_object` | Yes | 0x99 byte → `UnknownPixelBlock`. |
| Coverage gaps (mutation reasoning) | — | NO test: (a) same-dims depth change → F-001 panic survives all 267 lib tests + fuzz; (b) many-region page → F-002 survives; (c) second page_id → F-003 survives; (d) region version repeat → F-004 survives. All four mutations would pass the entire existing suite. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Status: implemented (win/linux/macos; web not_started)" | docs/formats/sub-dvb.md:3; matrix row `subtitle-dvb-subs` (feature-matrix.toml:1003) | UNREACHABLE (implemented-at-crate, unreachable-in-product) | Code real (this audit); tests pass (`orchrun-test-workspace-nofailfast.log.txt`: lib 267/0, dvb_vobsub 2/0); zero shipped callers (Reachability row). "Implemented" on a player feature matrix implies a user-visible capability; none exists — the crate's next-step list itself says the Lead still has to "wire `DvbDecoder` into the TS track path" (sub-dvb.md:79). |
| "Zero panics; bounded: damage ledger 64 events (overflow counted), region pixel buffers 4 MiB" | sub-dvb.md:43-45 | FALSE | F-001 (reachable panic) + F-002 (page-total unbounded ≈5 GiB). |
| "It never panics; every malformation … is typed data" | mod.rs:30-32 (module contract) | FALSE | F-001; F-007 (DDS rejection not typed). |
| "I/O, unsafe, unbounded allocation …, panics" (Never list) | mod.rs:57-60 | FALSE | F-001 (panics), F-002 (unbounded at page level). |
| "page_state 1 (acquisition) / 2 (mode change) reset regions/objects/CLUTs; repeated version = no-op; composition at end-of-display-set (or once page+region+object are all seen…)" | sub-dvb.md:35-39 | REAL | `page_segment:413-418`, `:404-409`, `decode_pes:368-370`; tested (`full_page_lifecycle_two_updates`, `mode_change_clears_state`). |
| "no PMT descriptor plumbing of its own — pairs with the TS demuxer's 5.I1/5.REC-1 track plumbing" | sub-dvb.md:11-13 | FALSE (the pairing does not exist) | `rg -ni "subtitle" crates/demux/src` → 0 wiring hits; playback-engine pipeline.rs:1036-1038 deselects subtitle tracks because no provider exists. |
| "fuzz-lite survival … zero panics/hangs/unbounded allocations" | sub-dvb.md:52-56 | SHALLOW (as evidence of the panic/bounds claims) | The seeded LCG corpus never generates (a) same-dims depth changes nor (b) 256-region pages — precisely the two shapes that break the contract (F-001/F-002). The survival result is real but proves less than claimed. |

## 7. Dead / duplicated / unreachable code
- The entire `impl DvbDecoder` + `DvbCueBuilder` public surface is dead code from the product's perspective: no caller outside the crate's own tests and the unwired `fuzz/` scaffold (see Reachability). Not dead per rustc (pub API of a workspace member), dead per shipped binary.
- No duplication found within the crate; the DVB damage-ledger/cue-builder vocabulary is intentionally mirrored by the PGS module (worklog archive-2026-10.md:9938) — shared-shape, not copy-paste (different state machines; verified by reading pgs listing only).

## 8. Dependencies & flags
- `crates/subtitles/Cargo.toml`: deps = `media-types` only; dev-deps `testkit`, `media-clock`. No feature flags anywhere in the crate (no `#[cfg(feature` in dvb/**) — nothing is compile-gated, so the whole module builds in every workspace lane (baseline build #1 clean, `build-release-workspace.log.txt`).
- No unsafe in this range (`rg "unsafe" crates/subtitles/src/dvb/mod.rs` → 0). Workspace lints deny `unwrap/expect/panic/todo` — and the file contains none (consistent with clippy-clean baseline #5).

## 9. Open questions for the next agent
- To W09-A06 (dvb/mod.rs part2): confirm the F-001 panic site (`region_image:826-830`) and check whether any part2 test covers a depth change (I found none). `compose`'s `ObjectWithoutRegion { object_id: region_id | 0x8000 }` (`:796-798`) for missing regions is a semantic abuse of an object-typed variant — your call whether to file.
- To the segment.rs owner: `parse_display_def` window branch `wx - x + 1` / `wy - y + 1` (segment.rs:381) is u16 arithmetic that underflows on hostile window coords (debug panic / release wrap). The 8192 filter in `display_def_segment` catches the wrapped huge values but not the debug-panic path.
- Wave-level: `subtitle-dvb-subs`, `subtitle-srt`, `subtitle-vtt`, `subtitle-pgs`, `subtitle-cc-608-708`, `subtitle-vobsub` all say "implemented" — all six share the same UNREACHABLE status (verified for the crate as a whole via `rg -l "subtitles::"`). Whoever owns the matrix rows should reconcile "implemented" with `cargo xtask parity-report`'s 0/252.
- REQUEST-ORCH-RUN: `cargo test -p subtitles dvb::tests:: -- --nocapture` (confirm 267 lib tests still green in this tree) and a 6-line repro for F-001: PES1 = page+region(depth 8)+object(0xFF)+END, PES2 = region(same dims, depth 2, no fill)+END → expect panic at region_image. (I cannot run cargo per hard rule 8.)
