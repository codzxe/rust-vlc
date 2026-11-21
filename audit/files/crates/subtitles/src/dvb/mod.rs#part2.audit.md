# AUDIT: crates/subtitles/src/dvb/mod.rs#part2 (lines 613–1265 of 1265)
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 653 (613–1265) / 24 (8 impl fns + 1 enum + 3 impl fns of DvbCueBuilder + 5 test builders + 11 tests, incl. part1-context reads 1–612) |
| File verdict | UNREACHABLE (real, library-tested code; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — `subtitles` is a dependency of NOTHING shipped: only `fuzz/Cargo.toml` (own-workspace cargo-fuzz scaffold, nightly-only, not in any gate) references it; `apps/cli` and `apps/desktop-tauri` Cargo.tomls do not; demux mints the track kind (`demux/src/ts/mod.rs:0x90 => "pgssub"`, `0x06 => "ts-private-pes"`) but never calls into `crates/subtitles`. Call chain proven dead by: `rg -n "subtitles" **/Cargo.toml` → only fuzz/Cargo.toml. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 4 S4: 2 |
| Commands run | `rg -n "subtitles" **/Cargo.toml` (dependents: fuzz only); `sed -n` on all quoted ranges (verbatim check); `rg -n "RleOverrun" crates/subtitles` (no DVB test asserts the damage variant); read of dvb/{segment.rs,rle.rs,clut.rs,fuzz.rs} + compositing/{bitmap.rs,layout.rs,timing.rs} (contract of every callee); baseline `orchrun-test-workspace-nofailfast.log.txt` lines 5269+ (subtitles 267 unit tests + 15 test binaries all ok). |

## 1. One-paragraph verdict
Part2 (the RLE field painter, the display-set composer, the palette
application, and the cue builder) is genuinely implemented, spec-aware
code with strong lifecycle unit tests — and it is dead weight in the
shipped product because nothing outside the fuzz scaffold depends on
the crate. Worse than dead: it contains one real panic (`bank[index]`
slice index out of bounds) reachable from the public `decode_pes` API
with a two-PES stream that re-declares a region at a lower depth — a
direct breach of the module's "never panics" contract, and invisible
to the committed survival corpus because its mutator cannot insert the
second region segment that triggers it. The compose step also silently
drops fill-only regions, misreports missing regions under a
wrong damage variant, and the cue builder's "one nanosecond before the
next PTS" rule is off by exactly the one nanosecond it advertises.

## 2. What actually works (verified)
— The 12 in-file tests of this file (all in part2's range) ran green
in the orchestrator's no-fail-fast workspace run:
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269`
→ `Running unittests src/lib.rs (…/subtitles-6e1b683bb35a2c97)`,
`test result: ok. 267 passed; 0 failed; 0 ignored`, including
`full_page_lifecycle_two_updates` (exact RGBA byte assertions across
three display sets). NOTE: baseline runs #3/#4
(`test-workspace.log.txt`, `test-workspace-allfeatures.log.txt`)
aborted at apps/desktop / first-vertical BEFORE the `subtitles`
package ran — only the no-fail-fast rerun covers it.
— The default map tables (`map2to4=[0,7,8,0xF]`, `map2to8=[0,0x77,0x88,0xFF]`,
`map4to8=[0x00,0x11,…,0xFF]`, dvb/mod.rs:633-638) match the ffmpeg
`dvbsubdec` default maps as documented in the module header; the
0x10/0x11/0x12 depth gating at 656-704 correctly bounds every write
into the matching bank size for a fixed region depth (traced through
dvb/rle.rs write paths).

## 3. Findings

### F-001 [S1] Slice-index panic in `region_image` when a region's depth decreases across display sets (stale pbuf)
Where: crates/subtitles/src/dvb/mod.rs:826-830 (panic site; root cause at 462-468, part1's range — flagged for the part1 auditor)
Code:
```rust
        let bank = clut.bank(region.depth);
        let mut rgba = Vec::with_capacity(region.pbuf.len() * 4);
        for &index in &region.pbuf {
            let c = bank[index as usize];
```
and the root cause (region re-definition, same w/h, no fill):
```rust
            Some(existing) if existing.width == w && existing.height == h => {
                existing.depth = depth;
                existing.clut_id = region.clut_id;
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
```
What is wrong: `region_segment` (part1) re-assigns `existing.depth`
without clearing or re-masking `pbuf`, and `dirty` is never reset
after a compose. `Clut::bank()` (dvb/clut.rs:171-177) returns
`bank4`/`bank16`/`bank256` — 4/16/256 entries. So: display set 1
declares region 7 as 8-bit and an object paints index 200 (8-bit
sub-block `0x12` writes raw bytes, dvb/rle.rs:281-287); display set 2
re-sends region 7 with the same width/height but depth code 1 (2-bit),
no fill — `pbuf` still holds 200, `dirty` is still true, and the
EndOfDisplaySet compose runs `bank(2)[200]` → `index out of bounds:
the len is 4 but the index is 200` PANIC.
Concrete reproducer (grammar = the file's own test builders):
PES-1 = page_seg(5,1,1,[(7,10,20)]) + region_seg(7,4,2,depth_code=3,
clut=9,fill=false,0,[(1,0,0)]) + object_seg(1,[0x12,0xC8,0x00,0x00],[])
+ seg(0x80) — then PES-2 = page_seg(5,2,0,[(7,10,20)]) +
region_seg(7,4,2,depth_code=1,…) + seg(0x80). Second `decode_pes`
panics. (Top field: 0x12 = 8-bit block, 0xC8 = single pixel index
200, 00 00 = EOL — `pbuf[0] = 200`, dirty = true, first compose uses
bank256 and succeeds; PES-2 flips depth to 2 with the buffer intact.)
Why it matters: the module contract (mod.rs:30-32 "It never panics;
every malformation … is typed data", mod.rs:59-60 "Never: … panics")
is false; the fuzz target `fuzz_sub_dvb` (which reuses one decoder
across chunks, dvb/fuzz.rs:41-66) crashes on this input. The committed
survival suite cannot find it: its mutator only flips/sets/splices
bytes (tests/dvb_vobsub_fuzz_survival.rs:329-357) and every seed
carries region segments only in its first chunk, so a second-chunk
region re-definition at a different depth is never generated —
"survival" evidence does not cover the cross-display-set surface it
claims to.
What correct looks like: on depth change (or any region re-definition)
either clear pbuf to the fill color / zero and force re-acquisition,
or clamp/mask every stored index at compose time (`bank.get(index)`
with typed damage on miss), matching the reference decoder's fixed
256-entry palette lookup.
Confidence: HIGH — Verification: read (full trace through
segment.rs parse_region → region_segment → compose → region_image →
clut::bank; rle write ranges verified). Not executed —
REQUEST-ORCH-RUN (see log): `cargo test -p subtitles dvb` with the
reproducer test above.

### F-002 [S2] Fill-only regions are never composed — `region_fill_flag` backgrounds are invisible
Where: crates/subtitles/src/dvb/mod.rs:802-804
Code:
```rust
            if !region.dirty {
                continue;
            }
```
with the fill paths that guarantee `dirty == false` at 466-468 (part1
range, `existing.dirty = false;`) and 483-484 (`dirty: false`).
What is wrong: `dirty` is set only by `paint_line` (762) and is never
set by a region fill. A region whose entire content is its
`region_fill_colour` (background boxes — the documented purpose of
EN 300 743 §7.2.3's region fill flag) is silently skipped at compose.
The fill bytes are parsed, masked, stored — and then never displayed
unless some object also paints the region. The module docs and the
crate honesty notes never document this divergence; the test
`mode_change_clears_state` (1124-1127) enshrines it ("Region not
dirty (no object painted): no rects").
Why it matters: realistic broadcast pages that draw a background box
and put all foreground in objects, or fill-only emphasis regions, lose
their background entirely; a player built on this crate shows text
over video where the spec shows a filled box.
What correct looks like: fill sets `dirty = true` (the fill changes
the pixels that will be composed), or compose includes every page
region regardless of paint history and `dirty` is used only for
caching.
Confidence: MEDIUM — Verification: read for the behavior (HIGH —
traced + test pins it); the EN 300 743 fill semantics and ffmpeg's
compose behavior are UNVERIFIED (no spec or ffmpeg source in the
sandbox — the matrix row's "bit tables cross-checked line-by-line vs
ffmpeg" claim is not reproducible here).

### F-003 [S3] `compose` reports a missing region with a faked object id under the wrong damage variant
Where: crates/subtitles/src/dvb/mod.rs:795-799
Code:
```rust
                if missing_reported.insert(entry.region_id) {
                    self.damage(DvbDamage::ObjectWithoutRegion {
                        object_id: u16::from(entry.region_id) | 0x8000,
                    });
```
What is wrong: a page entry whose REGION never arrived is reported as
`ObjectWithoutRegion` — a variant documented (107-109) as "An object
data segment arrived for an object no region composition ever placed"
— with `region_id | 0x8000` masquerading as an object id. A consumer
rendering the ledger prints "object 32775 has no region placement"
for missing region 7. The `0x8000` bit trick is documented nowhere.
Why it matters: the damage ledger is this module's observability
contract; this entry lies about both the kind and the id.
What correct looks like: a dedicated `RegionMissingFromPage`/`PageRegionNotDefined`
variant, or at minimum a comment on the variant documenting the
compose reuse.
Confidence: HIGH — Verification: read.

### F-004 [S3] Cue end-span off by one nanosecond vs the documented rule
Where: crates/subtitles/src/dvb/mod.rs:884-887 (rule text at 51-55)
Code:
```rust
        let end = match (next_pts_ns, start) {
            (Some(next), Some(start)) if next > start => Some(next - 1),
```
with module doc:
```rust
//! end-display-time computation: the previous page stays up until
//! one nanosecond before the next update's PTS, and `page_time_out`
```
What is wrong: `CompositedCue::active_at` is half-open
`[start, end)` (compositing/timing.rs:38-49: `now_ns >= end` ⇒
inactive). With `end = next - 1`, the last active nanosecond is
`next - 2`, and t = `next - 1` is a dead zone: previous page inactive,
next page not yet started — exactly the "one-frame gap" the timing
module's rationale says the half-open rule exists to prevent. The
documented "stays up until one nanosecond before the next PTS" needs
`end = next` in a half-open encoding. Test `cue_builder_spans_and_positions`
(1239, 1246) asserts `end == next-1` AND `!active_at(next-1)`,
codifying the divergence from the module's own stated rule.
Why it matters: 1 ns of dead time per page boundary — invisible in
practice, but this crate's selling point is sample-exact ns timing
and the code contradicts its own spec text; whoever ports the rule to
a coarser base (ms) inherits a visible gap.
What correct looks like: `end = Some(next)` (half-open) and update the
test; or keep `next - 1` and make `active_at` inclusive — but not the
current mix.
Confidence: HIGH — Verification: read (both sides traced; timing.rs
tests pasted in baseline).

### F-005 [S3] Truncated map-table sub-blocks end the field silently — no damage event
Where: crates/subtitles/src/dvb/mod.rs:705-730
Code:
```rust
                0x20 => {
                    if data.len() - i < 2 {
                        return true;
                    }
```
(the same pattern for 0x21 at 717-719 and 0x22 at 724-726)
What is wrong: a 2/4/8-bit map-table sub-block whose table bytes are
truncated ends the whole field with `return true` and NO damage event
— no `TruncatedPayload`, nothing. The module contract is "every
malformation … is typed data" (30-32).
Why it matters: damage-ledger consumers (and the fuzz harness's
invariants) cannot distinguish a clean field from a truncated one;
the typed-damage promise has a hole exactly where the input is most
likely hostile.
What correct looks like: `self.damage(DvbDamage::TruncatedPayload { kind: "map-table", … })`
before the early return.
Confidence: HIGH — Verification: read.

### F-006 [S3] Out-of-range page positions silently teleport to bottom-center
Where: crates/subtitles/src/dvb/mod.rs:933-938
Code:
```rust
        let u = Rational::new(u32::from(x), w).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(y), h).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
```
What is wrong: page x/y are read as full u16 (segment.rs:118-119), so
x=5000 in a 720-wide display yields u > 1; `Anchor::new` rejects
fractions outside [0,1] (compositing/layout.rs:106-118) and the
`unwrap_or` fallback silently pins the region to BOTTOM_CENTER —
centered, 90 % down. A region authored (or corrupted) past the edge
renders in a completely different place with no damage event.
Why it matters: violates the typed-damage philosophy (this is exactly
a `BadClutId`-class situation: bad authoring data, reference fallback
— but here the fallback is silent and drastic); hostile input gets a
plausible-looking wrong render instead of a ledger entry.
What correct looks like: clamp u/v to [0,1] (edge) plus a damage
event, or skip the region typed.
Confidence: HIGH — Verification: read.

### F-007 [S4] Full-region RGBA clone per cue build
Where: crates/subtitles/src/dvb/mod.rs:906-911
Code:
```rust
            let bitmap = CueBitmap::from_rgba(
                region.width,
                region.height,
                region.height,
                region.rgba.clone(),
            )?;
```
What is wrong: `region.rgba.clone()` copies the whole region buffer
(per region, per display set) because `cues` borrows the update; the
crate's sibling paths (compositing_alloc.rs) advertise allocation
discipline.
Why it matters: O(region bytes) allocation per subtitle page; minor
at subtitle rates but avoidable (take `&mut DvbPageUpdate` and
`std::mem::take`, or `Cow`).
What correct looks like: move the buffer (update is consumed per
display set by design).
Confidence: HIGH — Verification: read. (Also: third arg `region.height`
is `baseline`, legal at == height; vertical alignment is Top so it is
inert — checked against bitmap.rs:105-135.)

### F-008 [S4] Dead defensive branch + untested damage arm with an overpromising test name
Where: crates/subtitles/src/dvb/mod.rs:763-765; test at 1173-1186
Code:
```rust
        if y >= region.height as usize {
            return (data.len(), x);
        }
```
and the test:
```rust
    fn rle_overrun_is_clipped_and_typed() {
```
(1174) whose body (1175-1186) asserts only `regions.len()` and
`rgba.len()` — it never asserts a `RleOverrun` damage event.
What is wrong: (a) `paint_line`'s `y >= height` early-out is
unreachable through `paint_field` (the 647 guard already rejects
`y >= rh`) — dead defense that hides the real invariant; (b) NO test
in the crate asserts `DvbDamage::RleOverrun` (rg over crates/subtitles:
only the enum def, the Display arm, and the producer at 648) — the
guard at 647-653 is executed by zero tests, and the test name
promises "typed" without checking it. (PGS, in contrast, does assert
its `RleOverrun` at pgs/mod.rs:970.)
Why it matters: untested damage arm + misleading test name = the
"typed" half of the clip-and-type contract is faith, not evidence.
What correct looks like: a test with a continuation byte (e.g. a
second 0x10 after a full row) asserting the RleOverrun ledger entry;
delete or debug_assert the dead branch.
Confidence: HIGH — Verification: read + `rg -n "RleOverrun" crates/subtitles`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DvbDecoder::paint_field` | 615-746 | REAL | S1-adjacent (F-001 enabler: legal 8-bit writes here), F-005 | Field walker: 0x10/0x11/0x12 depth gating correct (656-704); 0xF0 two-row step correct; map defaults match ffmpeg docs; truncation of 0x20/0x21/0x22 silent (F-005); guard 647 fires RleOverrun (untested, F-008b); no panic paths (rle clips); O(field bytes) |
| `DvbDecoder::paint_line` | 750-782 | REAL | F-008a | Row dispatch to rle::decode_line_{2,4,8}bit; sets dirty pre-check; dead `y>=height` branch; `(data.len(), x)` consume-rest return is correct but undocumented |
| `DvbDecoder::compose` | 786-819 | REAL | F-002, F-003 | End-of-display-set assembly; BadClutId fallback to default banks correct (verified by test 1138-1160); dedups missing-region damage via HashSet; skips !dirty regions (F-002) |
| `DvbDecoder::region_image` | 821-839 | REAL | F-001 (panic site) | Palette application; `bank[index]` unguarded; capacity×4 exact; O(pixels) |
| `enum Rle` | 845-849 | REAL (trivial) | — | Table carrier with optional map |
| `DvbCueBuilder::new` | 865-867 | REAL (trivial) | — | Stores extent |
| `DvbCueBuilder::cues` | 876-925 | REAL | F-004, F-007 | Span end rule off-by-one vs doc (F-004); timeout fallback saturating; ns_base literal construction justified in comment (889-895, gcd(1,1e9)=1 verified); z saturating add; zero-sized regions skipped (dead: compose only emits w,h ≥ 1) |
| `DvbCueBuilder::layout_for` | 933-946 | REAL | F-006 | Rational anchor over display extent; TitleSafe margin divergence documented in comment (929-932); extent comes from builder, not the update — caller must keep them in sync (API footgun, no enforcement) |
| tests: `seg`/`page_seg`/`region_seg`/`clut_seg`/`object_seg` | 955-1025 | REAL (builders) | — | Grammar mirrors segment.rs; used by all tests below |
| `empty_payload_is_typed_not_panicking` | 1027-1036 | REAL | — | Asserts first damage |
| `truncated_object_is_typed` | 1038-1050 | REAL | — | Asserts TruncatedPayload |
| `full_page_lifecycle_two_updates` | 1052-1115 | REAL (strong) | — | Exact RGBA bytes, 3 display sets, is_clean; would fail on palette/paint/EOL/state bugs |
| `mode_change_clears_state` | 1117-1136 | REAL | enshrines F-002 | Pins dirty-skip as intended |
| `bad_clut_id_uses_default_banks` | 1138-1160 | REAL | — | Asserts default bank pixel |
| `object_before_region_is_typed` | 1162-1171 | REAL | — | |
| `rle_overrun_is_clipped_and_typed` | 1173-1186 | SHALLOW | F-008b | Name overpromises; asserts no damage; the RleOverrun arm has zero coverage crate-wide |
| `region_over_cap_is_disabled` | 1188-1201 | REAL | — | 4000×4000 > 4 MiB cap |
| `unknown_pixel_block_stops_object` | 1203-1220 | REAL | — | |
| `cue_builder_spans_and_positions` | 1222-1247 | REAL | enshrines F-004 | Asserts end==next-1 AND !active_at(next-1) — pins the divergence |
| `timeout_bounds_trailing_page` | 1249-1264 | REAL | — | Timeout fallback |

## 5. Test honesty (in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_payload_is_typed_not_panicking | YES | asserts the damage value, not just no-panic |
| truncated_object_is_typed | YES | matches! on the variant |
| full_page_lifecycle_two_updates | YES | byte-exact RGBA across 3 display sets incl. repaint + clear |
| mode_change_clears_state | PARTLY | asserts absence of rects — passes for BOTH correct reset and the F-002 skip bug |
| bad_clut_id_uses_default_banks | YES | exact pixel + exact damage first |
| object_before_region_is_typed | YES | |
| rle_overrun_is_clipped_and_typed | NO (for the "typed" half) | no damage assertion; only length checks (F-008) |
| region_over_cap_is_disabled | YES | variant match |
| unknown_pixel_block_stops_object | YES | variant match |
| cue_builder_spans_and_positions | YES (but pins the wrong rule) | exact ns + anchor + half-open checks — enshrines F-004 |
| timeout_bounds_trailing_page | YES | exact end ns |
| dvb_vobsub_fuzz_survival (crate test, feeds fuzz_sub_dvb) | NO for F-001 | mutator cannot synthesize a second-chunk region at a different depth (deletion/flip/set only, seeds carry region segs only in chunk 1) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "EN 300 743 segment/CLUT/RLE/page machine -> CompositedCue" (status windows/linux/macos = implemented) | docs/feature-matrix.toml row subtitle-dvb-subs (996-1006) | UNREACHABLE + partially FALSE on robustness | Code exists and is library-tested (no-fail-fast run: 267/0), but no shipped binary depends on `crates/subtitles` (rg Cargo.toml → fuzz only), and the "never panics" contract is violated by F-001 |
| "230/0 subtitles gates incl. dvb_vobsub_fuzz_survival" | same row, evidence string | REAL-UNPROVEN for the survival claim's coverage | the survival suite ran green (no-fail-fast log), but its mutation grammar cannot reach the cross-display-set depth change (F-001) — "green" does not mean the surface is covered |
| "It never panics; every malformation … is typed data" | dvb/mod.rs:30-32 module contract | FALSE | F-001 panic; F-005 silent truncation; F-006 silent anchor fallback |
| "the previous page stays up until one nanosecond before the next update's PTS" | dvb/mod.rs:51-55 | FALSE | F-004: last active ns is next-2 |
| lib.rs: "no panics on any input — failure is typed" (crate-level claim) | crates/subtitles/src/lib.rs:78-80 | FALSE | F-001 (via public decode_pes) |

## 7. Dead / duplicated / unreachable code
- The entire `crates/subtitles` crate is unreachable from shipped
  entry points (see header). Within part2: `paint_line`'s y-guard
  (763-765) is dead via the only caller; `cues`' zero-size region skip
  (903-905) is dead given compose's invariants; `Rational::new(…).unwrap_or(ZERO)`
  in layout_for can't fail (den = extent.max(1)).
- Test builders `seg/page_seg/region_seg/clut_seg/object_seg`
  (955-1025) are duplicated near-verbatim in
  tests/dvb_vobsub_fuzz_survival.rs:52-127 — copy-paste duplication
  (S4-level; noted, not counted separately).

## 8. Dependencies & flags
- No feature flags gate this code (no cfg in part2 besides
  `#[cfg(test)]`). Crate is std-only + media-types (Cargo.toml).
- Workspace membership via `crates/*` glob: built and linted by
  baseline runs #1/#5, tested only by the no-fail-fast rerun (runs
  #3/#4 abort before the `subtitles` package).

## 9. Open questions for the next agent
- dvb/mod.rs#part1 owner: confirm the F-001 root-cause reading of
  462-470 (depth reassign without pbuf invalidation) and check the
  bottom-field-reuse rule at 596-602 ("reference reuses the top data
  one row down" — ffmpeg reuses top field data but the +1 row offset
  comes from `field=1` here; verify no double-offset).
- Whoever owns pgs/mod.rs: does PGS re-declare object/epoch state with
  the same stale-buffer hazard as F-001? (PGS epoch start flushes —
  check whether depth-analog fields exist.)
- Wave synthesizer: matrix rows subtitle-pgs/dvb-subs/cc-608-708/vobsub
  claim "implemented" on three platforms while NO shipped binary
  depends on the crate — recommend a wave-level S0 ledger entry
  (claim of user-visible capability vs zero reachability).
