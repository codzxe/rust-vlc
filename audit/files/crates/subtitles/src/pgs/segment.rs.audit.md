# AUDIT: crates/subtitles/src/pgs/segment.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 583 / 8 prod (detect, header_len, from_type_byte, walk_segments, frame_rate_id_table, from_byte, flushes_epoch, parse_pcs, parse_wds, parse_pds, parse_ods — 11 counting small impls) + 9 tests |
| File verdict | UNREACHABLE (real, tested code; crate not wired to any shipped app) |
| Reachable from shipped entry point? | NO — chain: pgs/mod.rs `decode_framed`→`walk_segments` (mod.rs:334), `parse_pcs` (393), `parse_wds` (415), `parse_pds` (419), `parse_ods` (449); `crates/subtitles` is depended on only by `fuzz/Cargo.toml:50` (not a workspace member, root Cargo.toml:41); demux mentions "pgssub" only as a TS stream-name string (crates/demux/src/ts/mod.rs:365), no decode routing |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b pgs::segment` → 9 passed 0 failed (exit 0); `rg 'S_HDMV|PGS|pgs' crates/demux/src` → 1 cosmetic hit; baseline cargo-test-workspace.txt pgs segment tests ok |

## 1. One-paragraph verdict
The PGS segment grammar (both framings, deterministic detection, PCS/WDS/PDS/ODS/END payloads, HDMV frame-rate table) is correctly implemented with airtight bounds (every slice access guarded; pos ≤ len invariant holds through both framings — re-derived by hand, no panic path exists), and the 9 tests are honest and pass when executed. The byte layouts match the HDMV/PGS spec as documented (PCS 11-byte fixed part + 8/16-byte object refs, WDS 1+9n, PDS 2+5n, ODS 4+7+RLE) — verified against my knowledge of ffmpeg pgssubdec, though no oracle exists in-tree (UNVERIFIED). The defects are documentation lies about damage accounting: parse_wds and parse_pds claim short tails are "typed upstream", but the upstream caller does no such accounting — truncated window/palette entries vanish silently, contradicting the crate's "every malformation is typed data" contract.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b pgs::segment` → `test result: ok. 9 passed; 0 failed`. detect() picks Sup vs Stripped correctly on both crafted buffers (test walk_strips_and_detects_both_framings). Bounds arithmetic re-derived: loop guard `data.len() - pos >= hdr` with pos ≤ len invariant → no subtraction underflow, no OOB index in either framing.

## 3. Findings
F-001 [S3]
Where: segment.rs:309-310, segment.rs:353-355; consumer mod.rs:414-416
Code: (verbatim)
```
/// Parse a WDS payload into its windows (short tails are dropped by
/// the caller's truncation accounting — the fixed part is 1 byte).
pub fn parse_wds(data: &[u8]) -> Vec<Window> {
```
```
    /// 5-byte entries — a short trailing entry is dropped (typed
    /// upstream via the entry accounting).
    pub entries: Vec<PaletteEntry>,
```
```
    fn wds_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.windows = segment::parse_wds(seg.data);
    }
```
What is wrong: both comments are false. The "caller's truncation accounting" for WDS is a bare assignment (mod.rs:415) — a window entry short of its 9 bytes is silently dropped, no `PgsDamage`, not even a count check (`count` vs `windows.len()`). Same for PDS: a short trailing 5-byte entry is dropped with zero damage signal (pds_segment, mod.rs:418-446, checks only `parse_pds` returning None, which requires < 2 bytes). Why it matters: the crate contract (pgs/mod.rs:42-44: "every malformation … is typed data") is violated, and a corrupt stream loses subtitle windows/palette entries invisibly. What correct looks like: return the parsed count vs expected count, or have the caller compare declared `count` with `windows.len()` and record `TruncatedPayload`.
Confidence: HIGH — Verification: read (both call sites traced).

F-002 [S4]
Where: segment.rs:252-286
Code: (verbatim)
```
        objects.push(CompositionObject {
            id,
            window_id,
            forced: flags & 0x40 != 0,
            x,
            y,
            crop,
        });
    }
    Some(Pcs {
```
What is wrong: after the object loop, `p` may be < `data.len()` — trailing bytes after the last object ref are silently accepted (no full-consumption check, no damage). A PCS declaring 1 object but carrying 30 extra bytes parses clean. Minor silent-garbage acceptance; contrast with parse_pcs's strict short-ref rejection two lines up.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: segment.rs:114-117, segment.rs:139-141
Code: (verbatim)
```
/// the walk stops at the first segment whose declared size overruns
/// the remaining bytes (the reference behavior — except `END`, whose
/// declared size is allowed to overrun, per the reference decoder).
```
```
            // END may overrun its declared size (observed in the
            // wild; the reference allows it). Every other kind stops
```
What is wrong: the "per the reference decoder" claim is UNVERIFIABLE in-sandbox (ffmpeg pgssubdec.c is not vendored; no .sup fixture exists under tests/fixtures — only ass/canonical/fuzz/vtt text fixtures). The behavior itself is pinned by the in-repo test (walk_stops_on_overrun_except_end, executed, passes) but the attribution to ffmpeg is unproven; if false, END-overrun streams would be accepted where the reference rejects. Also the END segment's `data` is set to the entire remaining buffer (line 146: `data: &data[body..]`) regardless of the declared size, and `used = data.len()` masks the overrun from the caller's `used < payload.len()` truncation check (mod.rs:335).
Confidence: MEDIUM — Verification: read; ffmpeg comparison UNVERIFIED (no source in-tree).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PgsFraming::detect | 56-63 | REAL | — | 2x walk, deterministic tie→Stripped; empty→Stripped documented |
| PgsFraming::header_len | 66-71 | REAL | — | 7/3 correct |
| SegmentKind::from_type_byte | 92-101 | REAL | — | 0x14/15/16/17/80 + Unknown |
| walk_segments | 118-160 | REAL | S4 | bounds safe (re-derived); END-overrun semantics attributed to reference UNVERIFIED (F-003) |
| frame_rate_id_table | 165-177 | REAL | — | HDMV ids 1-8, 24000/1001…60/1 — matches spec table |
| CompositionState::from_byte | 207-214 | REAL | — | b>>6, _→EpochContinue |
| CompositionState::flushes_epoch | 218-220 | REAL | — | !Normal — matches mod.rs epoch machine |
| parse_pcs | 239-297 | REAL | S4 | grammar correct incl. crop flag 0x80/forced 0x40; trailing bytes ignored (F-002) |
| parse_wds | 311-337 | REAL | S3 | correct 1+9n; short tail silently dropped, comment lies (F-001) |
| parse_pds | 360-383 | REAL | S3 | correct 2+5n Y/Cr/Cb/alpha; short entry silently dropped, comment lies (F-001) |
| parse_ods | 402-431 | REAL | — | 4-byte fixed + 7-byte first-fragment header; u24 total incl. w/h — consistent with consumer's `total-4` (mod.rs:485) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| walk_sup_framing_including_pts | YES | pts value, kinds, used==len asserted |
| walk_strips_and_detects_both_framings | YES | detect() both directions + pts stripped |
| walk_stops_on_overrun_except_end | YES | pins the END exception exactly (used==len for overrun END) |
| pcs_parses_full_grammar | YES | every field incl. crop tuple + forced + flushes_epoch |
| pcs_short_object_ref_is_none | YES | truncation → None |
| wds_parses_and_drops_short_tail | YES | pins the drop behavior (but not any damage — see F-001) |
| pds_entry_order_is_y_cr_cb | YES | byte-order discriminating |
| ods_first_and_continuation_parse | YES | header tuple + fragment bytes + truncated-first → None |
| frame_rate_table_covers_the_hdmv_ids | YES | 5 entries incl. 1001-based |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Grammar verified against the public reference decoder behavior (ffmpeg pgssubdec.c, fetched 2026-10-25)" | segment.rs:4-6 | UNVERIFIED | no ffmpeg source or .sup fixture in-tree; worklog archive-2026-10.md admits no reference-encoder differential |
| "short tails are dropped by the caller's truncation accounting" (WDS) | segment.rs:309-310 | FALSE | F-001: mod.rs:414-416 has no accounting |
| "typed upstream via the entry accounting" (PDS) | segment.rs:353-355 | FALSE | F-001: mod.rs:418-446 has no entry accounting |
| END overrun "per the reference decoder" | segment.rs:117,140 | UNVERIFIED | F-003 |
| PCS/WDS/PDS/ODS byte layouts | segment.rs:18-37 | REAL (per spec knowledge) | tests pcs_parses_full_grammar/ods_first_and_continuation_parse encode the layouts and pass |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (header table).
- No dead functions found; `#[must_use]` on detect/frame_rate_id_table is respected at call sites (mod.rs:305).

## 8. Dependencies & flags
- None beyond std + `testkit::Must` (dev). No features. No unsafe. Segment walk allocates one RawSegment per segment and one Vec per PCS/WDS/PDS/ODS — bounded by payload size.

## 9. Open questions for the next agent
- pgs/mod.rs owner: does any test exercise a truncated WDS/PDS entry and assert the damage ledger? (If yes, F-001 downgrades to comment-only; I found none in mod.rs tests up to line 793.)
- Anyone with ffmpeg source: does pgssubdec really allow END size overrun? (F-003)
