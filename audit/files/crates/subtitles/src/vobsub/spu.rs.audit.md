# AUDIT: crates/subtitles/src/vobsub/spu.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Lines / functions | 635 / 8 production fns (+9 test fns/helpers, 1 const, 2 types) |
| File verdict | UNREACHABLE (real, test-executed library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — only consumer of the `subtitles` crate in the entire repo is `fuzz/Cargo.toml:50` (separate lumen-fuzz workspace, not shipped); `apps/cli` and `apps/desktop-tauri` manifests have zero `subtitles` dependency. In-crate chain: `decode_spu`/`extract_spu_pes` ← `vobsub/mod.rs:146,181` ← `vobsub::parse` ← `subtitles` lib root; nothing outside the crate calls it. |
| Findings | S0: 0 S1: 0 S2: 4 S3: 3 S4: 3 |
| Commands run | rg "decode_spu\|extract_spu_pes\|SpuDecoded" (callers: only vobsub/mod.rs + own tests); rg "subtitles::" across repo (only fuzz shims + crate-internal); rg subtitles in **/Cargo.toml (only fuzz/); rg "dvd_subtitle" (apps/cli/src/containers/ps.rs:62 — classification only, no feed); rg "SubtitleParser" (plugin-api trait exists, subtitles crate has NO impl — lib.rs:101 "Future 5.M1 unification"); sed/awk/od verifications of every quote below; baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5504-5510 (7 spu tests ok). |

## 1. One-paragraph verdict
This is real, non-stub SPU-decode code: a typed 9-variant error model, a DCSQT chain walker with loop guards, a nibble-RLE two-field weaver, and an MPEG-PS PES extractor, all panic-free by construction (bounds re-verified by hand below) and covered by 7 tests that pass in the baseline run (no-failfast log lines 5504-5510). It is nevertheless unreachable from the shipped binary — the `subtitles` crate has no consumer outside its own tests and the non-shipped fuzz workspace — so "implemented" in the feature matrix means "library code + green unit tests", not "a user can get a VobSub rendered". Hostile reading also finds four S2 correctness defects: coordinate validation that silently mis-decodes single-axis inversions, a PTS extraction that reads past the declared PES header, silent zero-fill of a truncated final RLE line (violating the module's own typed-damage contract), and an argument-arity divergence on command 0x00 against the claimed ffmpeg oracle that cannot be checked in-sandbox.

## 2. What actually works (verified)
— RLE nibble semantics: `nibble_rle_runs` hand-computed table checks (0x53→(1,1), 0x02F0→(188,0), fill-to-end) pass in baseline run (orchrun-test-workspace-nofailfast.log.txt:5506) and the arithmetic in `decode_run` (thresholds 1→4→16→64, max 4 nibbles, run ≤ 16383) was re-derived by hand — matches the module doc §RLE.
— Full decode + interlace weave: `full_spu_decode`/`interlaced_weave_is_correct` assert concrete index rows (e.g. row0=[1,2], row1=[3,1]) — baseline ok (log:5504-5505).
— Typed truncation errors: `overrun_is_typed_error`, `truncated_and_invalid_are_typed` — baseline ok (log:5507, 5509).
— Termination/bounds of `decode_field`: `x` is reset only after `x >= w` so `room = w - x` never underflows; `row+x+take ≤ row+w` stays inside both field slices (top: `(ceil(h/2)-1)*2w+w ≤ w*h`; bottom slice `indices[w..]` of len `w*(h-1)` fits `(h/2-1)*2w+w`); DCSQT chain strictly increases (`next < seq`/`next == seq` breaks) — no infinite loop.
— Nothing verified beyond the crate: no shipped-binary path executes this code (see reachability row).

## 3. Findings

### F-001 [S2] Coordinate validation misses single-axis inversion; `w == 0` check is dead; legit 1-line bitmaps rejected
Where: crates/subtitles/src/vobsub/spu.rs:248-260
Code:
```
    if offset1 < 0 || offset2 < 0 || (x2 < x1 && y2 < y1) {
        return Err(SpuError::MissingCommands);
    }
```
```
    let w = x2.saturating_sub(x1) + 1;
    let h = y2.saturating_sub(y1) + 1;
    if w == 0 || h <= 1 {
        return Err(SpuError::MissingCommands);
    }
```
What is wrong: the guard fires only when BOTH axes are inverted (`&&`). With only `x2 < x1`, `saturating_sub` clamps `w` to 1 and the decoder silently produces a 1-pixel-wide bitmap — wrong output, no error. With only `y2 < y1`, the clamp yields `h == 1` and the packet is rejected as `MissingCommands` — a mis-categorized error (coords WERE present). `w == 0` can never be true (`saturating_sub(..)+1 ≥ 1`), so that half of line 258 is dead — evidence the author intended a real width check that the saturation already destroyed. A legitimate single-line subpicture (`y2 == y1`, `h == 1`) is also rejected, even though the field decoder handles it (`div_ceil(1,2)=1` top line, `h/2=0` bottom returns early at line 303).
Why it matters: this crate's selling point (per `docs/formats/sub-vobsub.md` §Working subset) is "typed `IdxDamage`/damage ledger, never silent" — here malformed coords produce silently wrong pixels or a wrong error category, and a valid edge-case stream is refused.
What correct looks like: `if x2 < x1 || y2 < y1 { return Err(...CoordsInvalid) }` and drop the dead `w == 0` arm; allow `h == 1` (fields already handle it) or reject it with an honest variant.
Confidence: HIGH — Verification: read; arithmetic re-derived; no oracle needed.

### F-002 [S2] `extract_spu_pes` MPEG-2 PTS read ignores `header_len`; reads PTS out of the substream/payload bytes
Where: crates/subtitles/src/vobsub/spu.rs:383-390
Code:
```
                let flags = sub[q + 1];
                let header_len = sub[q + 2] as usize;
                let hp = q + 3;
                if flags & 0x80 != 0 && hp + 5 <= sub.len() {
                    pts = Some(decode_pts(&sub[hp..hp + 5]));
                }
                let _ = header_len;
```
What is wrong: the 5-byte PTS is read from `q+3` whenever the PTS flag is set and the FILE is long enough — `header_len` is never checked to be ≥ 5. With `flags=0x80, header_len=0` (exactly the shape the module's own test builds at line 611: `[0x81, 0x80, 0x00]`), `hp..hp+5` covers the substream ID byte plus payload bytes, so `pts` becomes garbage derived from subtitle data; only the test's short buffer (`hp+5 > sub.len()`) hides the bug. The `let _ = header_len;` line explicitly discards the very value that should have gated the read. Additionally the PTS is never validated against `header_len` upper bound either (a `header_len` smaller/larger than 5 both mis-slice).
Why it matters: wrong timestamps silently corrupt cue timing (the pairing layer in vobsub/mod.rs consumes this PTS); the crate's damage-ledger discipline is bypassed.
What correct looks like: `if flags & 0x80 != 0 && header_len >= 5 && hp + 5 <= sub.len()` (and ideally `header_len >= 5` exactly gates; otherwise skip PTS with a typed damage event).
Confidence: HIGH — Verification: read + hand-trace of the test at lines 606-621 (pts stays `None` only because `hp+5=14 > 12`).

### F-003 [S2] Truncated final RLE line silently zero-fills — violates the module's own "typed overrun" contract
Where: crates/subtitles/src/vobsub/spu.rs:309-334 (loop head 310-318)
Code:
```
    loop {
        if gb.pos() > gb.bit_len() {
            return Err(x);
        }
        let (len, color) = decode_run(&mut gb);
        let room = w - x;
        if len != usize::MAX && len > room {
            return Err(x);
        }
        let take = len.min(room);
```
What is wrong: `BitReader::read` (crates/subtitles/src/dvb/rle.rs:60-73) zero-extends past the end while still advancing `pos`; `decode_run` on exhausted bits returns the fill-to-end sentinel `(usize::MAX, 0)`. If this happens on the LAST field line, the line is completed with color-0, `y+1 == field_h` breaks the loop, and `decode_field` returns `Ok` — a truncated bitmap decodes "successfully" with a silently zero-filled final line. The `pos > bit_len` check only fires on the NEXT iteration, which never comes. The dvb BitReader's own doc (rle.rs:57-59) says zero-extension is safe because "a typed overrun is reported by the caller" — this caller fails that contract in exactly the last-line case (any mid-line truncation whose remaining nibbles decode to fill-to-end also succeeds silently, e.g. truncated mid-run).
Why it matters: module doc lines 38-39 promise "A run that would cross the line end is a typed decode error (the reference rejects the packet)" — a missing final line is worse (whole-line invention) and produces NO error at all. Silent corruption on corrupt input is precisely what this crate claims never to do.
What correct looks like: after the loop, require the reader to have been exhausted only at a line boundary AND with all `field_h` lines sourced from real data — e.g. track whether the last run came from zero-extension (pos > bit_len at fill time) and return `Err` instead of `Ok`.
Confidence: HIGH — Verification: read + hand-trace of both truncation positions (mid-field and last-line); BitReader EOF semantics verified in dvb/rle.rs:60-73.

### F-004 [S2] Command `0x00` consumed with zero arguments — desync risk vs the claimed reference grammar; `forced` set unconditionally
Where: crates/subtitles/src/vobsub/spu.rs:187-190 and module doc 22-26
Code:
```
            match cmd {
                0x00 => forced = true,
                0x01 => start_ms = Some((date << 10) / 90),
                0x02 => end_ms = Some((date << 10) / 90),
```
Doc (lines 24-26): "`0x05` coords (12-bit x1/x2/y1/y2), `0x06` top/bottom field RLE offsets. `0x01`/`0x02` take NO arguments".
What is wrong: the module's own command table singles out `0x01`/`0x02` as the only argument-less commands — implying `0x00` ("forced/menu") carries parameters (in the de-facto DVD SPU grammar and in ffmpeg's `dvdsubdec.c`, the claimed differential oracle at line 6-7, command 0x00 is followed by 2 parameter bytes from which the forced flag is read). This code consumes none, so on any stream that emits `0x00 xx yy` the two parameter bytes are re-interpreted as commands: benign only if they happen to be `0x00 0x00`; any other value either fakes an unknown command (silent `_ => break` at line 236) or misparses as colormap/alpha/coords — desynchronizing the walk. `forced = true` unconditionally also means a non-forced stream carrying the menu command is flagged forced.
Why it matters: forced-subtitle VobSub streams are common (the command exists precisely for them); a desync here yields wrong or dropped cues with no damage event.
What correct looks like: consume 2 parameter bytes for `0x00` and derive `forced` from them, mirroring the reference; or, if this is a deliberate subset, say so in the doc and emit a typed damage event instead of silently mis-walking.
Confidence: MEDIUM — Verification: read; the in-file doc/code contradiction is verified; the ffmpeg arity itself is UNVERIFIED (no ffmpeg source in sandbox, no network — blocked from oracle check). The doc claim "cross-checked line-by-line against ffmpeg's dvdsub decoder tables" (docs/formats/sub-vobsub.md:48) is not backed by any committed artifact.

### F-005 [S3] `used_color` is write-only dead code
Where: crates/subtitles/src/vobsub/spu.rs:265 and 323
Code:
```
    let mut used_color = [false; 4];
```
```
        used_color[usize::from(color.min(3))] = true;
```
What is wrong: the array is allocated, threaded through both `decode_field` calls, and written — then never read anywhere in the file (verified: only occurrences are 265, 272, 276 pass-throughs, 323 write). `color.min(3)` is additionally redundant (`color = v & 3 ≤ 3`). This is an unfinished feature (likely intended transparency/optimization or validation) shipped as dead state.
Why it matters: dead code in a decoder misleads the next maintainer into thinking color usage is tracked; per-function cost is trivial but it is exactly the "log-line-only body / ignored result" fakeness pattern the audit hunts.
What correct looks like: either consume it (e.g. drop fully-unused palette entries into the damage ledger) or delete it.
Confidence: HIGH — Verification: read + rg of the identifier within the file.

### F-006 [S3] `find_start_code` linear scan false-positives on `00 00 01 BD` inside PES payloads
Where: crates/subtitles/src/vobsub/spu.rs:422-436
Code:
```
/// Find `00 00 01 <code>` from `from` (the linear scan the idx
/// filepos targets rely on — a sub-only region has no other PES).
fn find_start_code(data: &[u8], from: usize, code: u8) -> Option<usize> {
```
What is wrong: the scan matches start-code bytes ANYWHERE, including inside RLE bitmap payloads; a payload containing `00 00 01 BD` is split into a phantom PES (the extraction loop then advances `p = end.max(at + 6)` past the real packet boundary, potentially dropping the remainder of the true SPU). The doc comment's justification ("a sub-only region has no other PES") addresses other PES types, not payload-embedded sequences — SPU payload bytes are arbitrary, so this is reachable with ordinary subtitle data, not just adversarial input.
Why it matters: mis-split extraction silently drops or corrupts cues on real files; no damage event is emitted.
What correct looks like: after parsing a PES, resume the scan strictly at the parsed `end` (the code does use `p = end.max(at+6)`, which helps — but the initial scan from `p` can still land mid-payload when a previous packet was skipped via the `continue` at line 404); at minimum emit a typed damage event when a candidate start code yields a non-SPU parse.
Confidence: MEDIUM — Verification: read + trace of the `continue` path (line 401-404 sets `p = at + 4`, re-entering the scan INSIDE the previous packet's region).

### F-007 [S4] Big-offset (HD-header) path undocumented; standard SPU with size field 0 is silently reinterpreted
Where: crates/subtitles/src/vobsub/spu.rs:141-150
Code:
```
    let big_offsets = u16::from_be_bytes([data[0], data[1]]) == 0;
    let offset_size = if big_offsets { 4 } else { 2 };
```
What is wrong: the module doc's "# SPU layout (standard, non-HD)" section (lines 12-20) documents only the 2-byte-offset form; the 4-byte-offset branch is implemented but never documented. A standard-mode packet whose declared size is exactly 0 (first BE16 == 0 — garbage but typed elsewhere) is silently reinterpreted as big-offset: `size` is then read from bytes 2-5 (the command offset + bitmap start) producing a nonsense size that usually ends in `SizeOverCap` — a typed error, but the wrong category and for the wrong reason.
Why it matters: undocumented format branches in a format decoder are where reference-divergence hides; the doc/test suite never exercises this path (no test constructs a big_offsets packet).
What correct looks like: a doc section for the HD-header form + at least one test for both the accepted and rejected HD paths.
Confidence: HIGH — Verification: read; rg confirmed no test touches `big_offsets`/HD header construction.

### F-008 [S4] DCSQT at the exact tail boundary is skipped, mis-categorized as `MissingCommands`
Where: crates/subtitles/src/vobsub/spu.rs:176-180
Code:
```
    let mut seq = cmd_pos;
    while seq > 0 && seq < data.len().saturating_sub(2 + offset_size) {
```
What is wrong: a DCSQT whose header occupies exactly the last `2 + offset_size` bytes (`seq == len - 2 - offset_size`) is fully readable but excluded by the strict `<`; the walk never runs, `offset1/2` stay -1, and the caller sees `MissingCommands` instead of the `CommandOverrun` the entered loop would have produced (`pos == data.len()` hits line 182-184). Both are typed errors; only the category is wrong.
Why it matters: damage-ledger precision — consumers distinguishing "feed more data"/"discard packet"/"missing commands" get the wrong diagnosis at the boundary.
What correct looks like: `<=` with the command-byte overrun check doing the rejection (it already exists).
Confidence: HIGH — Verification: read + boundary trace.

### F-009 [S4] `pts_decode_90khz` is a self-inverse tautology; PTS extraction test never asserts a PTS
Where: crates/subtitles/src/vobsub/spu.rs:624-634 (and 606-621)
Code:
```
        let b = [
            (((pts >> 30) & 0x07) << 1) as u8 | 0x21,
            ((pts >> 22) & 0xFF) as u8,
```
What is wrong: the test encodes the PTS with the inverse of `decode_pts`'s own bit layout, then asserts `decode_pts` recovers it — any symmetric misunderstanding (e.g. both encoder and decoder masking the wrong marker bit) passes. `pes_extraction_finds_substreams` (606-621) constructs a PES with the PTS flag set but, due to F-002's shape (header_len 0, short buffer), yields `pts = None` and asserts nothing about it — the extraction→PTS path is entirely untested against a real value.
Why it matters: the timestamp chain (PES PTS → cue timing) is exactly where unit drift hides; the suite gives false confidence.
What correct looks like: hard-code a known-good 5-byte PTS (from the MPEG spec or a real .sub fixture) and assert the decoded 90 kHz value; assert `found[0].2 == Some(expected)` in a PES test with a proper header_len=5.
Confidence: HIGH — Verification: read + trace.

### F-010 [S3] Bitmap allocation unbounded by any canvas sanity: 12-bit coords allow a 16 MiB `indices` buffer from a tiny packet
Where: crates/subtitles/src/vobsub/spu.rs:256-264
Code:
```
    let w = x2.saturating_sub(x1) + 1;
    let h = y2.saturating_sub(y1) + 1;
    if w == 0 || h <= 1 {
        return Err(SpuError::MissingCommands);
    }
```
```
    let mut indices = vec![0u8; w * h];
```
What is wrong: `x1/x2/y1/y2` are 12-bit (≤ 4095), so `w*h` can reach 4096×4096 = 16,777,216 bytes from a packet of at most 128 KiB (SPU_MAX_SIZE) — a ~128× amplification, and there is no check against the video canvas (DVD subpictures live inside 720×576). The crate's fuzz-survival doc claims "no unbounded allocations" (docs/formats/sub-vobsub.md:53-54); this is bounded, but only by the 12-bit field width, not by policy.
Why it matters: a corrupt `.sub` (or a hostile one) forces repeated 16 MiB zeroed allocations through the pairing loop per entry — a memory-amplification DoS surface on a path designed to be fuzzed.
What correct looks like: cap `w*h` (e.g. 720×576×2 or an explicit MAX_BITMAP_PIXELS constant) with a typed error, mirroring SPU_MAX_SIZE's discipline.
Confidence: MEDIUM — Verification: read; field widths from the 0x05 parse (lines 215-218) are 12-bit by construction.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `impl Display for SpuError::fmt` | 68-97 | REAL | — | trivial, all 9 variants covered, matches enum |
| `impl Error for SpuError` | 99 | REAL | — | empty marker impl, correct |
| `SPU_MAX_SIZE` const | 101-103 | REAL | — | 128 KiB, enforced at 151-153, consumed by vobsub/mod.rs:167 |
| `decode_spu` | 137-291 | REAL (flawed) | S2 | F-001 coords, F-003 last-line path, F-004 cmd 0x00, F-007 big_offsets, F-008 boundary, F-010 alloc; termination proven (chain strictly increases); no panics (all indexing preceded by length checks 182-227) |
| `decode_field` | 295-336 | REAL (flawed) | S2 | F-003; bounds/termination hand-verified (see §2); writes dead `used_color` (F-005) |
| `decode_run` | 341-354 | REAL | — | thresholds re-derived; max run 16383 matches doc; no overflow (u32, 4 nibbles max) |
| `extract_spu_pes` | 366-420 | REAL (flawed) | S2 | F-002 PTS/header_len, F-006 rescan-inside-packet via `p = at+4` continue; `p` always advances (no hang); MPEG-1 PTS branch (391-400) untested |
| `find_start_code` | 424-436 | REAL | S3 | F-006; linear O(n) scan, `from` clamp absent but loop bound `i+4 <= len` safe |
| `decode_pts` | 439-445 | REAL | S4 | 33-bit PTS + marker masking; tested only tautologically (F-009) |
| `tests::build_spu` (helper) | 457-500 | REAL | — | hand-builds valid packets; `debug_assert_eq!` only in debug builds |
| `tests::nibble_rle_runs` | 502-524 | REAL | — | hand-computed table, mutation-sensitive |
| `tests::full_spu_decode` | 526-544 | REAL | — | asserts geometry/indices/colormap/dates concretely |
| `tests::interlaced_weave_is_correct` | 546-561 | REAL | — | pins weave order top/bottom |
| `tests::overrun_is_typed_error` | 563-573 | REAL | — | would fail if the `len > room` check were deleted |
| `tests::truncated_and_invalid_are_typed` | 575-603 | REAL | — | TooSmall/InvalidCommandOffset/NeedMoreData |
| `tests::pes_extraction_finds_substreams` | 605-621 | REAL (weak) | S4 | no PTS assertion (F-009); single MPEG-2 shape only |
| `tests::pts_decode_90khz` | 623-634 | REAL (tautology) | S4 | F-009 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| nibble_rle_runs | YES | independent hand-computed expected values |
| full_spu_decode | YES | concrete index/color/date assertions on a builder-independent fixture shape |
| interlaced_weave_is_correct | YES | row-by-row weave assertions |
| overrun_is_typed_error | YES | removes the guard → decode "succeeds" → `matches!` fails |
| truncated_and_invalid_are_typed | YES | three distinct typed variants pinned |
| pes_extraction_finds_substreams | PARTIALLY | substream/offset/payload pinned; PTS untested (None by accident of buffer length) |
| pts_decode_90khz | NO (weakly) | encoder built from the decoder's own inverse — symmetric bugs pass (F-009) |
| Untested paths (no test at all) | — | SizeOverCap, all HdUnsupported variants (0x83-0x86), MissingCommands, big_offsets/HD header, MPEG-1 PTS branch (391-400), unknown-command `_ => break` (236), chain guards (239-244), command 0x00, F-003's silent last-line truncation |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the PS demuxer (5.I2a) already routes `0xBD` + substream `0x20..=0x3F` payloads as `"dvd_subtitle"`; this module is the standalone idx/sub file path and the SPU decode both feed" | spu.rs:7-10 module doc | HALF-TRUE | classification exists (apps/cli/src/containers/ps.rs:62 — `dvd_subtitle` by private-1 substream byte) but NOTHING feeds this module: no crate outside `subtitles` depends on it (rg over all Cargo.tomls → only fuzz/Cargo.toml:50); "both feed" has no code path |
| "SPU grammar … the nibble-RLE interlaced bitmap, the MPEG-PS PES walker; multi-PES SPUs accumulate exactly like the reference" | docs/formats/sub-vobsub.md §Working subset (27-32) | REAL-UNPROVEN | code exists and unit tests pass (baseline log 5504-5510), but "exactly like the reference" is unverifiable: F-003/F-004 document divergences, and no differential corpus exists (doc admits this at 55-58) |
| "RLE semantics cross-checked line-by-line against ffmpeg's `dvdsub` decoder tables" | docs/formats/sub-vobsub.md:47-49 | UNVERIFIED | no ffmpeg source or cross-check artifact in the tree; sandbox has no network; F-004 shows an unchecked arity assumption |
| "fuzz_target = `fuzz_sub_vobsub` (in-crate harness `src/vobsub/fuzz.rs`)" | docs/formats/sub-vobsub.md front-matter 16-17 | PAPER-ONLY (registry) | in-crate harness exists (src/vobsub/fuzz.rs), but no `fuzz_sub_vobsub` target exists: fuzz/fuzz_targets/ holds 35 files, sub-targets only srt/ass/vtt; fuzz/Cargo.toml has no such [[bin]] — the "Lead-owned wiring" is still unwired |
| feature-matrix row `subtitle-vobsub` status implemented (win/linux/macos) | docs/feature-matrix.toml:1035-1045 | UNREACHABLE | "implemented" = library + tests only; shipped `lumen` binary cannot reach it (no dependency edge from apps/*); engine-info (baseline INDEX) lists only mjpeg/pcm decoders |
| row title "VobSub + SVCD/CVD subtitles" with status implemented | docs/feature-matrix.toml:1038-1041 vs docs/formats/sub-vobsub.md:64-66 | FALSE (for the SVCD/CVD half) | the record's own Deferred section: "SVCD/CVD siblings … NOT implemented"; row 18-A06's territory, cross-noted |

## 7. Dead / duplicated / unreachable code
- `used_color` — write-only (F-005).
- `w == 0` guard — dead condition (F-001).
- HD command handling split across two arms: `0x85` parsed-then-rejected inside the `0x05` arm (211-222) while `0x83|0x84|0x86` get a dedicated arm (232-234) — duplicated HdUnsupported paths with inconsistent pre-parsing.
- The whole module is unreachable from shipped entry points (see header row) — dead-in-production, alive-in-tests.

## 8. Dependencies & flags
- `use crate::dvb::rle::BitReader` — internal reuse, pub(crate), fine.
- `testkit::Must` in tests — dev-dep, matches Cargo.toml dev-dependencies.
- No feature flags gate this file. Crate has zero external deps (media-types only) — matches the "Std-only by C5" note in Cargo.toml.

## 9. Open questions for the next agent
- For A08 (vobsub/mod.rs): does the pairing layer consume `SpuDecoded.size` (line 289 `size.max(data.len())`) correctly when declared < received? And does `render_cue` clip `width/height` against the video canvas (F-010's amplification lands there)?
- For A08: does any vobsub path set `forced` semantics downstream (F-004's unconditional `forced = true`)?
- For A01 (idx.rs): the doc claims idx `filepos` targets pair with `extract_spu_pes` offsets — verify the offset bases agree (file-absolute vs PES-relative).
- For W18/W19 (docs): the `fuzz_sub_vobsub` registry claim and the SVCD/CVD row-title overclaim need matrix/record corrections.
- ORCH: consider a REQUEST-ORCH-RUN differential once a real .idx/.sub corpus exists — none is in the sandbox (doc admits, sub-vobsub.md:55-58).
