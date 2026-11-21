# AUDIT: crates/subtitles/src/pgs/segment.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Lines / functions | 582 / 21 (12 production fns/impls + 5 type defs with logic + 9 tests + 1 test helper) |
| File verdict | REAL-UNPROVEN (in-crate test-proven; crate UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO — sole reverse-dep of crate `subtitles` is `fuzz/Cargo.toml:50` (own workspace, nightly-only, not shipped); no app/engine crate links it; `lumen` CLI refuses `subtitle.*` ids ("Phase-5 card", apps/cli/src/dispatch.rs:197-198). In-crate callers: pgs/mod.rs `PgsDecoder` (walk_segments/parse_*), pgs/palette.rs (`PaletteEntry`), tests/pgs_fuzz_survival.rs, pgs/fuzz.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `rg -n "walk_segments|PgsFraming|parse_pcs|…" src/pgs tests/` (caller map above); `sed -n` quote verification (all 5 quotes); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (subtitles unittests 267/0, pgs_fuzz_survival 2/0); `rg -c "#\[test\]" src/pgs/*.rs` → rle 8 + segment 9 + palette 4 + mod 11 = 32 (matrix "32 unit tests" claim exact) |

## 1. One-paragraph verdict
This is a real, bounds-checked, panic-free PGS segment grammar: the walk, all four
payload parsers, and the frame-rate table match the HDMV layout documented in the
header, arithmetic cannot underflow/overflow (pos ≤ len invariants hold on every
path; sizes are u16-bounded), and the 9 in-file tests assert real values (byte
orders, crop boxes, truncation paths) — they would fail under mutation of the
fields they pin. It is exercised only by the crate's own test suite (baseline
cites 267 unit tests + pgs_fuzz_survival green); nothing shipped links it. The
defects are doc-vs-code: two "typed upstream" claims are false (the caller does
no such accounting — partial WDS windows and short PDS entries vanish silently),
the END-overrun path makes `used` report full consumption and thereby suppresses
the caller's only truncation-damage signal, and the module doc's `.sup` framing
claim misdescribes the real (PES-wrapped) .sup file layout. No S0–S2: no panic,
no unbounded allocation (PCS capacity capped at 64, count is u8), no unit
confusion (90 kHz ticks named as such everywhere).

## 2. What actually works (verified)
- Grammar round-trips: 9 tests parse hand-built PCS/WDS/PDS/ODS bytes and assert
  exact field values — verified by reading; executed by baseline
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5271
  ("Running unittests src/lib.rs … subtitles … 267 passed; 0 failed") and
  :5552+ (pgs_fuzz_survival "2 passed").
- Panic-freedom of `walk_segments`/`parse_pcs`/`parse_wds`/`parse_pds`/`parse_ods`
  traced by hand: every index is preceded by an explicit length check
  (`data.len() - pos >= hdr` :122, `< 8`/`< 9`/`< 5`/`< 7` guards) and the
  `pos ≤ data.len()` invariant is maintained on all continue/break paths.
  No unwrap/expect/index panic exists in the production code (clippy
  `panic`/`unwrap_used` deny, baseline #5/#6 CLEAN).

## 3. Findings
### F-001 [S3]
Where: crates/subtitles/src/pgs/segment.rs:309-310 and :354-355 (contradicted by crates/subtitles/src/pgs/mod.rs:414-416, :418-425)
Code:
```
/// Parse a WDS payload into its windows (short tails are dropped by
/// the caller's truncation accounting — the fixed part is 1 byte).
```
and
```
    /// 5-byte entries — a short trailing entry is dropped (typed
    /// upstream via the entry accounting).
```
What is wrong: Both doc comments defer truncation reporting to a "caller's
truncation accounting" / "entry accounting" that does not exist. The only
production caller is `PgsDecoder::wds_segment` (mod.rs:414-416):
`self.windows = segment::parse_wds(seg.data);` — zero accounting; a WDS
declaring 5 windows with 1.5 windows of payload silently yields 1 window and
no `PgsDamage` event. Same for `pds_segment`: only the <2-byte `None` case is
typed; a PDS with 3½ entries drops the partial entry with no damage record.
Why it matters: malformed-input damage reporting — the crate's advertised
contract (`MAX_DAMAGE_EVENTS`, `PgsDamage::*`) — silently misses two of its
documented cases; a reader trusting these comments believes partial payloads
are observable upstream. What correct looks like: either the parsers return
`(payload, bytes_consumed)` so the caller can compare against the declared
count, or the doc comments state plainly that short tails are dropped
silently. Confidence: HIGH — Verification: read (both files).

### F-002 [S3]
Where: crates/subtitles/src/pgs/segment.rs:138-150
Code:
```
        if body + size > data.len() {
            // END may overrun its declared size (observed in the
            // wild; the reference allows it). Every other kind stops
            // the walk — the caller reports the truncation.
            if kind == SegmentKind::End {
                segments.push(RawSegment {
                    kind,
                    pts_90khz: pts,
                    data: &data[body..],
                });
                pos = data.len();
            }
            break;
        }
```
What is wrong: On an END segment whose declared size overruns the buffer,
`pos` is set to `data.len()`, so the returned `used` claims the whole buffer
was consumed. The caller's ONLY truncation signal is `used < payload.len()`
(mod.rs:334-344: `TruncatedSegment`/`LengthOverrun` damage). A payload
truncated mid-stream that happens to end on an END header with a nonzero
declared size is therefore reported as a clean, fully-parsed display set —
the truncation is unobservable. The segment's `data` is also stretched to all
remaining bytes regardless of the declared size. What correct looks like:
report the actual bytes consumed (the header at least), or emit a distinct
"end-overrun tolerated" damage event, and cite the exact reference lines
backing "the reference allows it". Why it matters: damage accounting is this
crate's substitute for error returns; this path defeats it. Confidence:
MEDIUM — the code behavior is HIGH (read; pinned by test
`walk_stops_on_overrun_except_end`:482-486 asserting `used == buf.len()`),
but the "reference allows it" claim about ffmpeg `pgssubdec.c` is UNVERIFIED
offline (no network; ffmpeg source not in tree).

### F-003 [S3]
Where: crates/subtitles/src/pgs/segment.rs:13-16
Code:
```
//! Matroska `S_HDMV/PGS` blocks carry the segments WITHOUT the
//! 4-byte PTS prefix (mkvtoolnix strips it); the standalone `.sup`
//! and the MPEG-TS PES carriage keep it. [`PgsFraming::detect`]
//! picks deterministically (whole-buffer consumption scores).
```
What is wrong: The claim that standalone `.sup` files are
`PTS:u32be | type | size | payload` misdescribes the real format: a Blu-ray
`.sup` is a sequence of PES packets (`00 00 01 BD`, PES length, PES optional
header carrying the PTS, then the segment) — ffmpeg reads them through
libavformat `supdec.c`, which parses the full PES header. A raw `.sup` file
fed to `walk_segments(_, PgsFraming::Sup)` would misparse from byte 0 (the
"PTS" would be `00 00 01 BD`, the "type" byte a PES-length byte). The `Sup`
framing matches a synthetic de-PESed form (PTS re-prefixed) that no in-tree
producer generates — the only in-tree callers pass Matroska-stripped or
test-built buffers. What correct looks like: name `Sup` what it is (a
PES-stripped carriage where the demuxer has re-attached the PTS), and
document the actual `.sup` PES framing as unsupported. Why it matters: the
module doc presents itself as "verified against the public reference decoder
behavior (ffmpeg … fetched 2026-10-25)"; a user feeding a real `.sup` gets
garbage walks with no error. Confidence: MEDIUM — ffmpeg `supdec.c` layout
recalled from memory; cannot fetch offline. UNVERIFIED against the live
source.

### F-004 [S4]
Where: crates/subtitles/src/pgs/segment.rs:56-62
Code:
```
    pub fn detect(data: &[u8]) -> PgsFraming {
        let (_, used_sup) = walk_segments(data, PgsFraming::Sup);
        let (_, used_stripped) = walk_segments(data, PgsFraming::Stripped);
        match (used_sup == data.len(), used_stripped == data.len()) {
            (true, false) => PgsFraming::Sup,
            _ => PgsFraming::Stripped,
        }
    }
```
What is wrong: Tie-break silently prefers `Stripped`; a crafted (or contrived
PTS-wraparound) `.sup` payload whose bytes also happen to strip-walk to exact
full consumption is misdetected as Matroska-stripped, and `PgsDecoder`
locks that framing in for the whole stream (mod.rs:305 `unwrap_or_else`).
The heuristic is documented (lines 50-54), deterministic, and only feeds
garbage-out (not memory-unsafety), so the blast radius is wrong cues on
adversarial input in a crate nothing ships. What correct looks like:
carriage knowledge at the demux layer (the `set_framing` API already exists,
mod.rs:311) and detect used only as a last resort with a damage event on
ambiguity. Confidence: HIGH (behavior read + traced to caller), LOW
real-world likelihood.

### F-005 [S4]
Where: crates/subtitles/src/pgs/segment.rs:545
Code:
```
        // The short trailing byte (0x99) is dropped, not panicked on.
```
What is wrong: The test buffer `[0x02, 0x00, 0x05, 0x41, 0x10, 0x20, 0x30,
0xFF, 0x99]` has TWO trailing bytes (0xFF, 0x99) beyond the one 5-byte
entry; the comment says "byte" (singular) and names only 0x99. What correct
looks like: "the 2 short trailing bytes (0xFF, 0x99) are dropped". Confidence:
HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PgsFraming` (enum) | 40-47 | REAL | — | two-variant framing marker |
| `PgsFraming::detect` | 56-63 | REAL | S4 (F-004) | 2× full walk O(2n); tie→Stripped; locked in by first caller |
| `PgsFraming::header_len` | 66-71 | REAL | — | 7/3, matches both grammars |
| `SegmentKind` (enum) | 75-89 | REAL | — | 0x14/0x15/0x16/0x17/0x80 + Unknown(u8) preserved for damage reporting |
| `SegmentKind::from_type_byte` | 92-101 | REAL | — | exhaustive match |
| `RawSegment<'a>` | 106-112 | REAL | — | zero-copy payload slice; Copy |
| `walk_segments` | 118-160 | REAL | S3 (F-002) | bounds-safe walk; END-overrun consumes rest & reports full use |
| `frame_rate_id_table` | 164-177 | REAL | — | 24/1001…60/1 matches HDMV ids; 0/9+ → None |
| `CompositionObject` | 180-190 | REAL | — | forced 0x40, crop 0x80 decode |
| `CompositionState::from_byte` | 207-214 | REAL | — | `>> 6`; `_` arm only reachable for 3 |
| `CompositionState::flushes_epoch` | 218-220 | REAL | — | non-Normal flushes; matches mod.rs epoch bank clear |
| `Pcs` (struct) | 224-235 | REAL | — | plain data |
| `parse_pcs` | 239-297 | REAL | — | len≥11 gate, per-object 8-byte gate, crop 8-byte gate; capacity `count.min(64)` (count≤255) — no unbounded alloc; returns None (whole-PCS drop) on short object ref |
| `Window` / `parse_wds` | 300-337 | REAL | S3 (F-001) | count u8; 9-byte gate; silent `break` on short tail |
| `PaletteEntry` / `Pds` | 340-357 | REAL | — | data carriers |
| `parse_pds` | 360-383 | REAL | S3 (F-001) | ≥2 gate; 5-byte stride; silent trailing-entry drop |
| `Ods` (struct) | 386-398 | REAL | — | header total semantic = width+height included (consistent with mod.rs:466-468 `saturating_sub(4)`) |
| `parse_ods` | 402-431 | REAL | — | ≥4 gate; first-fragment 7-byte gate; `fragment` copied via `to_vec` (alloc per fragment, bounded by u16 segment size) |
| tests: `sup_seg` helper | 438-444 | REAL | — | builds .sup framing bytes |
| `walk_sup_framing_including_pts` | 446-456 | REAL test | — | fails on PTS/type/size misread |
| `walk_strips_and_detects_both_framings` | 458-472 | REAL test | — | pins detect() both directions |
| `walk_stops_on_overrun_except_end` | 474-487 | REAL test | — | pins F-002 behavior as intended |
| `pcs_parses_full_grammar` | 489-513 | REAL test | — | full field map incl. crop+forced; would fail on any field mutation |
| `pcs_short_object_ref_is_none` | 515-521 | REAL test | — | truncation paths |
| `wds_parses_and_drops_short_tail` | 523-532 | REAL test | — | pins silent drop (F-001 behavior) |
| `pds_entry_order_is_y_cr_cb` | 534-547 | REAL test | S4 (F-005) | pins Y,Cr,Cb order; comment inaccuracy |
| `ods_first_and_continuation_parse` | 549-572 | REAL test | — | both descriptor flags + short-header None |
| `frame_rate_table_covers_the_hdmv_ids` | 574-581 | REAL test | — | spot-checks 5 of 9 entries; table trivial |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| walk_sup_framing_including_pts | YES | asserts pts_90khz Some(90_000), kinds, exact used==len |
| walk_strips_and_detects_both_framings | YES | asserts detect() outcome for both framings on distinct buffers |
| walk_stops_on_overrun_except_end | YES | asserts used==0 on PCS overrun and used==len on END overrun |
| pcs_parses_full_grammar | YES | every field asserted against distinct byte values (1920/1080, rate 2, #42, EpochStart, crop tuple) |
| pcs_short_object_ref_is_none | YES | both the count-without-data and <11-byte cases |
| wds_parses_and_drops_short_tail | YES | asserts exactly 1 of 2 windows parsed; empty input |
| pds_entry_order_is_y_cr_cb | YES | asserts (id,y,cr,cb,alpha) tuple byte-for-byte; <2-byte None |
| ods_first_and_continuation_parse | YES | header tuple (16,4,3), fragment bytes, flags, short None |
| frame_rate_table_covers_the_hdmv_ids | YES (partial) | only ids 1,3,8,0,9 checked; mutating id 2→(25,1) would pass — gap is trivial (table literals) |
All nine ran green in baseline (orchrun-test-workspace-nofailfast.log.txt:5269-5271, within the 267-test lib suite).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "PCS/WDS/PDS/ODS/END segments in BOTH framings (.sup PTS-prefixed + Matroska-stripped)" | docs/feature-matrix.toml row `subtitle-pgs` (evidence string, :992) | REAL (in-crate) | both framings implemented (segment.rs:41-47, 118-160) and tested; baseline green |
| "32 unit tests" for pgs | same row | REAL | `rg -c "#\[test\]"` → rle 8 + segment 9 + palette 4 + mod 11 = 32 (exact) |
| row status `implemented` (win/linux/macos) | same row (:989) | UNREACHABLE-in-product | crate linked by nothing shipped; parity-report baseline: 0/252 on all platform columns; "implemented" describes tree code, not the shipped binary |
| "Grammar verified against … ffmpeg libavcodec/pgssubdec.c, fetched 2026-10-25" | segment.rs:4-6 | UNVERIFIED | ffmpeg source not in tree, no network; field layouts match the HDMV spec from memory (MEDIUM confidence) |
| ".sup … keep[s] the 4-byte PTS prefix" | segment.rs:13-15 | FALSE-as-file-format (F-003) | real .sup is PES-wrapped (00 00 01 BD …); MEDIUM confidence, offline |
| "short tails are dropped by the caller's truncation accounting" (WDS) | segment.rs:309-310 | FALSE | mod.rs:414-416 has no accounting |
| "typed upstream via the entry accounting" (PDS) | segment.rs:354-355 | FALSE | mod.rs:418-425 types only the <2-byte case |

## 7. Dead / duplicated / unreachable code
No dead code in-file (all pub items consumed by mod.rs/palette.rs/tests/fuzz.rs —
caller map in header table). Unreachable-from-shipped-build: the entire file (see
header). No duplication found.

## 8. Dependencies & flags
Crate deps: `media-types` (path) only; dev: `testkit`, `media-clock` — used here
(`testkit::Must` in tests). No feature flags gate any of this code (no cfg in
file). No unsafe.

## 9. Open questions for the next agent
- pgs/mod.rs owner (W09 chunk holder): does `PgsDisplaySet.damage` surface to
  any consumer that could notice the F-001/F-002 gaps (i.e., is damage ever
  asserted on in tests beyond the in-file unit tests)?
- Whoever audits `fuzz/fuzz_targets/fuzz_sub_pgs.rs`: confirm the nightly-only
  campaign never ran in any baseline gate (INDEX.md #18 lists only
  `scripts/fuzz_registry_check.sh`, a diff check — not an execution).
- Cross-check F-003 against ffmpeg `libavformat/supdec.c` if any agent has an
  offline copy of the ffmpeg sources.
