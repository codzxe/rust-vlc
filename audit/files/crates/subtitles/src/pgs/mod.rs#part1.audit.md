# AUDIT: crates/subtitles/src/pgs/mod.rs#part1 (lines 1–580 of 1092)
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 580 (1–580) / 22 (3 consts, PgsDamage enum+Display, 2 structs+2 small impls, ObjectState, PgsDecoder: 11 fns incl. compose) — test module 676–1092 belongs to part2; its tests are cited as execution evidence only |
| File verdict | UNREACHABLE (real, library-tested code; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — same proof as dvb#part2: `rg -n "subtitles" **/Cargo.toml` → only `fuzz/Cargo.toml` (own-workspace cargo-fuzz scaffold, nightly-only); `apps/cli`/`apps/desktop-tauri` do not depend on `subtitles`; demux mints `"pgssub"` (`demux/src/ts/mod.rs`) but never calls this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `sed -n` on every quoted range; read of pgs/{segment.rs,palette.rs,rle.rs} (callee contracts: parse_pcs bounds, Palette `[Rgba;256]`, decode_object bounds, PgsFraming::detect, flushes_epoch); `rg -n "TRANSPARENT_BLACK\|flushes_epoch\|fn detect" crates/subtitles/src/pgs`; baseline `orchrun-test-workspace-nofailfast.log.txt:5269+` (267/0 unit tests incl. all pgs tests green). |

## 1. One-paragraph verdict
The PGS epoch state machine is real, careful work: fragment assembly,
bank caps, epoch flushes and the END-composition are implemented and
covered by byte-exact lifecycle tests (green in the no-fail-fast
baseline). Unlike its DVB sibling, I found NO panic path — the palette
is a fixed `[Rgba; 256]` and every RLE write is clipped — and the
damage ledger is honest about the cases it covers. The defects that
remain are semantic: palette-update display sets replace palettes
wholesale (HDMV fade deltas revert untouched entries to transparent —
test-pinned as intended, claimed reference-matching, UNVERIFIABLE
here), the missing-END fallback composes per BLOCK so a Matroska-split
display set emits a partial intermediate image dressed as a complete
one, `windows` is write-only dead state, and the sticky framing
detection can silently mis-lock a `.sup` stream from one degenerate
first payload. All of it is unreachable from any shipped binary.

## 2. What actually works (verified)
— Traced every indexing site in part1: `palette.rgba[index as usize]`
(compose:557) is bounded by `[Rgba; 256]` (palette.rs:97);
`ods.fragment[..frag]`/`&ods.fragment[..take]` are min-clamped
(486, 503-504); `object.remaining -= take` cannot underflow (min).
No DVB-F-001-style panic family exists in PGS. Verified by read of
pgs/palette.rs:92-119, pgs/rle.rs:60-128, mod.rs:448-571.
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt:5269`
→ subtitles 267/0 unit tests, including
`pgs::tests::epoch_lifecycle_full_display_set`,
`epoch_normal_reuses_objects_palette_update`,
`multi_fragment_ods_assembles`,
`palette_missing_drops_set_object_missing_skips`,
`rle_overrun_and_short_fragment_are_typed` — all ok. (Note: baseline
runs #3/#4 aborted BEFORE the subtitles package; only the no-fail-fast
rerun executes it.)

## 3. Findings

### F-001 [S3] Palette-update display sets: delta PDS entries replace the palette wholesale — unchanged entries revert to transparent
Where: crates/subtitles/src/pgs/mod.rs:408-411, 427-445
Code:
```rust
        // A palette-update PCS re-uses the referenced palette's
        // entries with only the deltas applied — the reference does
        // not implement it (logged); we honor the palette ref.
        self.pcs = Some(pcs);
```
plus the replacement insert:
```rust
        self.palettes.insert(
            pds.id,
            palette::Palette::from_entries(pds.id, pds.version, pds.entries.into_iter(), matrix),
        );
```
What is wrong: HDMV palette-update display sets (PCS
`palette_update_flag`, common for fades) transmit only the CHANGED
palette entries. `Palette::from_entries` starts from all-`TRANSPARENT_BLACK`
(palette.rs:109-113), so every entry the delta does not mention loses
its previous color. The test `epoch_normal_reuses_objects_palette_update`
(797-831) pins exactly this: after a version-bumped PDS defining only
entry 2, "entry 1 reverted to transparent" — an object still
referencing entry 1 renders a hole.
Why it matters: any real disc that changes SOME palette entries while
objects still use the unmentioned ones renders partially invisible
subtitles; the fade use-case only survives when encoders resend every
in-use entry per step.
What correct looks like: when the PCS sets `palette_update`, merge the
delta into the existing palette (libbluray behavior) instead of
replacing it.
Confidence: MEDIUM — Verification: read for the behavior (HIGH, test
pins it); the HDMV delta semantics are domain knowledge (spec text not
in sandbox); the inline claim "the reference does not implement it" is
UNVERIFIED (no ffmpeg source available — the wave's "fetched
2026-10-25" claims in palette.rs/rle.rs headers are not reproducible
here). If the reference DOES apply deltas, this is S2.

### F-002 [S3] Missing-END fallback composes per BLOCK — split display sets emit a partial intermediate image
Where: crates/subtitles/src/pgs/mod.rs:359-363
Code:
```rust
        // Streams that omit the END segment: compose once a PCS has
        // been seen (the DVB missing-end-of-display-set fallback).
        if !self.composed && self.pcs.is_some() {
            self.compose();
        }
```
What is wrong: the API contract (298-300) says one call = "one PES
payload / Matroska block's worth of segments" — but a Matroska display
set legitimately SPANS blocks (mkvtoolnix splits large ODS across
blocks). Block 1 of a split set contains PCS (+PDS+first ODS) and no
END → the fallback composes a PARTIAL object (remaining > 0,
`rle::decode_object` pads transparent) and returns it as a normal
`PgsDisplaySet` whose `PgsObjectImage` claims full `width/height`.
The only trace is a generic `RleOverrun` damage entry. Block 2 (ODS
continuation + END) then composes the real image. The DVB fallback
this was ported from at least requires page+region+object all seen;
this one fires on PCS alone.
Why it matters: per-block callers (the documented usage) present a
truncated subtitle at block 1's PTS on every split display set, plus a
spurious damage event that pollutes the ledger's meaning.
What correct looks like: track whether the live object set has
`remaining > 0` for any PCS-referenced object and suppress the fallback
compose (or mark the set partial) until fragments complete.
Confidence: MEDIUM — Verification: read (code path traced, HIGH);
frequency of split blocks in real MKVs is domain knowledge, not
measured (no corpus lane — the matrix row itself admits "no corpus
lane").

### F-003 [S4] `windows` is write-only dead state with a misleading comment
Where: crates/subtitles/src/pgs/mod.rs:239-241, 414-416
Code:
```rust
    /// The epoch's windows (tracked for the state machine; the
    /// reference places objects by their PCS x/y directly).
    windows: Vec<segment::Window>,
```
```rust
    fn wds_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.windows = segment::parse_wds(seg.data);
    }
```
What is wrong: `windows` is written by `wds_segment`, cleared on epoch
flush (406), and read by NOTHING (`rg -n "windows" crates/subtitles/src/pgs/mod.rs`
→ decl, clear, assignment only). "Tracked for the state machine" is
false — no state transition consults it; the WDS parse result is
discarded work every display set.
Why it matters: dead state plus a comment that claims purpose; readers
believe window geometry influences placement.
What correct looks like: delete the field (parse WDS for damage only),
or actually clamp object placement to window bounds.
Confidence: HIGH — Verification: read + rg.

### F-004 [S4] Sticky framing detection locks on the first payload — degenerate first block silently mis-locks a `.sup` stream
Where: crates/subtitles/src/pgs/mod.rs:302-307
Code:
```rust
        let framing = self
            .framing
            .unwrap_or_else(|| segment::PgsFraming::detect(payload));
        self.framing = Some(framing);
```
What is wrong: detection runs exactly once. `PgsFraming::detect`
returns `Stripped` for an empty or unwalkable buffer (segment.rs:56-66),
so a `.sup` stream whose first fed payload is empty/truncated (a
short first read, a dropped first block) locks `Stripped` for the
whole stream: every later `.sup` segment mis-walks (the 4-byte PTS
prefix is treated as segment bytes), producing a stream of
UnknownSegment/truncation damage with no hint that re-detection or
`set_framing` is the fix.
Why it matters: one bad first block permanently corrupts framing for a
stream that "does not switch mid-play" — exactly the scenario the
sticky rule was meant to protect.
What correct looks like: re-detect while no segment has ever parsed
cleanly (framing lock only after the first successful walk), or emit a
typed damage event when the locked framing starts failing wholesale.
Confidence: HIGH — Verification: read (detect semantics traced in
segment.rs:56-66).

### F-005 [S4] Per-compose clones of the full palette and each object's RLE buffer
Where: crates/subtitles/src/pgs/mod.rs:538, 546
Code:
```rust
        let Some(palette) = self.palettes.get(&pcs.palette_id).cloned() else {
```
```rust
            let Some(object) = self.objects.get(&comp.id).cloned() else {
```
What is wrong: `cloned()` copies a 256-entry palette (1 KiB) and the
object's ENTIRE reassembled RLE `Vec` (the declared length is a u24 —
multi-MB for hostile streams) per composed object per display set,
purely to appease the borrow checker around `self.damage`.
Why it matters: O(payload bytes) avoidable allocation per display set
in a crate that advertises allocation discipline (MP §12); on hostile
4 MiB objects this doubles peak memory per compose.
What correct looks like: restructure to collect damage after the loop,
or use `std::mem::take` on the RLE once fragments are complete.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts MAX_DAMAGE_EVENTS / MAX_EPOCH_PALETTES / MAX_EPOCH_OBJECTS | 77-82 | REAL | — | caps enforced at 433, 479 |
| `enum PgsDamage` + `Display` | 86-176 | REAL | — | 13 variants, honest doc comments incl. whole-set-drop semantics |
| `PgsObjectImage` | 181-193 | REAL | — | carries `forced` flag for track handling (never consumed downstream — see Open Questions) |
| `PgsDisplaySet::is_clean` | 214-219 | REAL (trivial) | — | |
| `ObjectState` | 223-230 | REAL | — | fragment assembly state |
| `PgsDecoder::{Default,new,reset}` | 257-288 | REAL | — | reset() full re-init (seek safety documented) |
| `PgsDecoder::damage` | 290-296 | REAL | — | cap + dropped counter, mirrored in DVB |
| `decode_display_set` | 302-308 | REAL | F-004 | sticky framing (see finding) |
| `set_framing` / `framing` | 311-318 | REAL | — | override escape hatch |
| `decode_framed` | 320-380 | REAL | F-002 | walk + dispatch + fallback compose + damage take; pts fallback from PCS at 385-392 correct precedence (container beats embedded) |
| `pcs_segment` | 382-412 | REAL | F-001 (site) | flush on every non-Normal state (segment.rs:218-221); claim "reference flushes on every non-zero state" UNVERIFIED (no reference source in sandbox); palette_update flag parsed in segment.rs but unused here (finding F-001) |
| `wds_segment` | 414-416 | REAL | F-003 | dead-end state |
| `pds_segment` | 418-446 | REAL | F-001 (replacement), S4 note | version-skip no-op correct; bank cap correct; matrix keys off live PCS height — PDS-before-any-PCS uses height 0 → BT.601 silently (out-of-model edge, noted not filed) |
| `ods_segment` | 448-517 | REAL | — | first/continuation assembly verified: size cap (469), bank cap (479), declared = total−4 clamp (485), FragmentOverrun on both first (487) and continuation (513) overflow; re-definition replaces in-flight object; unreachable defensive header damage arm (458-466) documented as such |
| `compose` | 521-571 | REAL | F-002 (executor), F-005 | blank-display path (531-536); PaletteMissing drops whole set (538-544) per claimed reference; ObjectMissing skips object; RLE damage aggregated into one RleOverrun per object (552-554) — includes `truncated` (partial fragment case typed); no panic (palette 256-bounded) |

## 5. Test honesty (part1 functions are exercised by tests owned by part2's range — cited, not deep-audited)
| Test (pgs/mod.rs:676+) | Would it fail if the code were broken? | Reason |
|---|---|---|
| epoch_lifecycle_full_display_set (764) | YES | byte-exact RGBA, extent, framing detection asserted |
| epoch_normal_reuses_objects_palette_update (797) | YES — but pins F-001 | asserts entry-1 transparency after version bump (enshrines replace-not-merge) |
| empty_composition_clears_and_epoch_start_flushes (833) | YES | flush semantics |
| multi_fragment_ods_assembles (863) | YES | fragment assembly |
| palette_missing_drops_set_object_missing_skips (883) | YES | both damage arms |
| truncated_and_unknown_segments_are_typed (918) | YES | |
| rle_overrun_and_short_fragment_are_typed (955) | YES | asserts `PgsDamage::RleOverrun` (mod.rs:970) — unlike DVB, the RleOverrun arm IS asserted |
| invalid_object_size_is_typed (989) | YES | |
| sup_framing_uses_segment_pts_when_container_silent (1009) | YES | PTS fallback |
| damage_ledger_is_capped (1077) | YES | cap + overflow accounting |
| NO test | — | F-002 (split display set across two decode calls) and F-004 (degenerate first payload locking framing) have zero coverage |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "PCS/WDS/PDS/ODS/END segments in BOTH framings (.sup PTS-prefixed + Matroska-stripped), epoch state machine, multi-fragment ODS assembly, RLE 0x00 escapes, BT.601-vs-BT.709-by-height palette -> CompositedCue; 32 unit tests + fuzz_sub_pgs survival harness, all green" (status implemented ×3 platforms) | docs/feature-matrix.toml row subtitle-pgs (983-995) | UNREACHABLE (code real + tests green per no-fail-fast baseline; no shipped binary reaches it); the "all green in this tree 2026-10-26" claim is consistent with baseline `orchrun-test-workspace-nofailfast.log.txt` | rg Cargo.toml (fuzz-only dependent); baseline log 5269+ |
| "It never panics; every malformation … is typed data" (mod.rs:51-53) | module contract | REAL (for part1's surface) | traced every index/arith site (see §2); rle/palette bounds verified |
| "the reference flushes on every non-zero state" | mod.rs:29-30 + segment.rs:218-221 | REAL-UNPROVEN | internal consistency yes; reference source not in sandbox — UNVERIFIED |
| "fetched 2026-10-25" ffmpeg cross-checks | pgs/palette.rs:4-9, pgs/rle.rs:3-4 | UNVERIFIED | no network, no ffmpeg source in tree; arithmetic of the constants re-derived by hand (BT.601: 1634/401/832/2066/1192 = round(k·255/224·1024) ✓; BT.709: 1836/218/546/2163 ✓ vs standard Kr/Kb) — constants are right, the "fetched" provenance is not checkable |

## 7. Dead / duplicated / unreachable code
- `windows` field (F-003) — dead state.
- `PgsObjectImage::forced` — surfaced "for the track's forced-subtitle
  handling" (188-190) but no consumer exists in-tree (rg: only
  construction sites) — dead until a track layer lands.
- `CompositionState::EpochContinue` flush path — spec-rare, untested
  (only EpochStart/Acquisition exercised per test names).
- Whole crate unreachable from shipped binaries (header).

## 8. Dependencies & flags
- No feature flags; std-only + media-types (same crate manifest).
- pgs/rle.rs and pgs/palette.rs are separate agents' files — their
  contracts were consumed here for reachability of part1's invariants
  (palette 256-array, decode_object clipping), not audited in this
  file.

## 9. Open questions for the next agent
- pgs/mod.rs#part2 (581-1092): the cue builder repeats DVB's
  `end = next - 1` half-open off-by-one (604-606) and
  `Anchor::new(...).unwrap_or(BOTTOM_CENTER)` silent fallback (660-662)
  — mirror findings of dvb#part2 F-004/F-006; also `object.rgba.clone()`
  (633). Audit accordingly.
- Whoever owns pgs/segment.rs: verify `parse_ods` truly guarantees a
  header on `first` fragments (mod.rs:458-460 claims unreachability of
  the defensive arm) and the END-size-overrun allowance in
  `walk_segments`.
- Wave synthesizer: subtitle-pgs row's "implemented" ×3 platforms has
  the same zero-reachability problem as DVB — consolidate at wave level.
