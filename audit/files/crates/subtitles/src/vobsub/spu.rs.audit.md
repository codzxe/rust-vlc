# AUDIT: crates/subtitles/src/vobsub/spu.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 635 / 7 prod (decode_spu, decode_field, decode_run, extract_spu_pes, find_start_code, decode_pts, Display::fmt) + 7 tests |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — only chain: vobsub/mod.rs `parse_vobsub`→`extract_spu_pes`/`decode_spu` (mod.rs:146,181) → `vobsub_cues`; the ONLY tree-wide dependent of `crates/subtitles` is `fuzz/Cargo.toml:50` (`subtitles = { path = "../crates/subtitles" }`), and `fuzz/` is NOT in root members (`Cargo.toml:41`); `rg subtitles apps/cli/Cargo.toml apps/desktop/Cargo.toml` → no match (exit 1) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b vobsub::spu` → 7 passed 0 failed (exit 0); `rg -n 'subtitles' apps/*/Cargo.toml` → exit 1; `od -c` line 386 (display-mangling check); baseline cargo-test-workspace.txt:5974-5980 (7/7 ok) |

## 1. One-paragraph verdict
The SPU grammar (header, DCSQT chain, commands 0x00-0x06, HD typed-unsupported, nibble RLE with growing thresholds, interlaced two-field weave, PES walker with PTS) is genuinely implemented, panic-free by construction (every slice access is guarded; verified by reading all bounds + the passing fuzz-survival suite), and covered by 7 real tests that I executed (7/7 pass on the prebuilt binary). But the coordinate validation is inverted: a legally single-row SPU (y1==y2) is rejected with a lying `MissingCommands` error while an inverted x-range (x2<x1, garbage) is silently accepted as a 1-pixel-wide bitmap; the RLE doc comment states a max run of 16383 where the code maxes at 255; `used_color` is computed and never read; and the whole crate is dead code from the shipped binary's perspective — nothing in apps/* depends on it.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b vobsub::spu` → `test result: ok. 7 passed; 0 failed` (all 7: full_spu_decode, interlaced_weave_is_correct, nibble_rle_runs, overrun_is_typed_error, pes_extraction_finds_substreams, pts_decode_90khz, truncated_and_invalid_are_typed). decode_run threshold semantics independently re-derived by hand: max 4 nibbles, v ≤ 1023, run ≤ 255, fill-to-end iff v<4 — matches ffmpeg's dvdsubdec nibble accumulation shape (UNVERIFIED against ffmpeg source: not vendored in-tree).

## 3. Findings
F-001 [S2]
Where: spu.rs:248-259
Code: (verbatim)
```
    if offset1 < 0 || offset2 < 0 || (x2 < x1 && y2 < y1) {
        return Err(SpuError::MissingCommands);
    }
    let (offset1, offset2) = (offset1 as usize, offset2 as usize);
    if offset1 >= data.len() || offset2 >= data.len() {
        return Err(SpuError::CommandOverrun);
    }

    let w = x2.saturating_sub(x1) + 1;
    let h = y2.saturating_sub(y1) + 1;
    if w == 0 || h <= 1 {
        return Err(SpuError::MissingCommands);
```
What is wrong: coordinate sanity is exactly backwards. (a) `x2 < x1` alone (invalid, garbage packet) does NOT error — `x2.saturating_sub(x1)+1` collapses it to `w == 1`, so a malformed SPU decodes a 1-wide garbage bitmap that flows downstream as a cue. (b) `y1 == y2` (a legal 1-row subpicture) is rejected because `h <= 1`, with `MissingCommands` — an error name that lies about the cause. (c) `w == 0` is dead: `saturating_sub(..)+1` can never be 0. Why it matters: mod.rs:21-24 promises "every malformation … is typed data"; here one malformation is silently accepted and one legal input is rejected as malformed. What correct looks like: `if x2 < x1 || y2 < y1 { return Err(...) }` (or a dedicated InvalidCoords variant) and allow `h == 1`.
Confidence: HIGH — Verification: read (arithmetic re-derived; no test covers inverted coords).

F-002 [S3]
Where: spu.rs:265, spu.rs:323
Code: (verbatim)
```
    let mut used_color = [false; 4];
```
```
        used_color[usize::from(color.min(3))] = true;
```
What is wrong: `used_color` is written by both `decode_field` calls and never read afterwards (rg confirms no other use in the file). Dead computation, presumably a vestige of ffmpeg's crop/palette logic that was never finished — the transparent-border crop in mod.rs re-derives its own scan.
Confidence: HIGH — Verification: read + rg `used_color` (2 writes, 0 reads).

F-003 [S3]
Where: spu.rs:388-417
Code: (verbatim)
```
                let _ = header_len;
                q += 3 + header_len.min(len.saturating_sub(3));
                len = len.saturating_sub(3 + header_len);
```
What is wrong: two defects. (1) `q` advance is capped at `len-3` but `len` subtraction uses the full `3 + header_len` — for `header_len > len-3` the pointer and the length disagree (payload silently becomes empty). (2) No cross-check of the PES payload end against the next start code: a corrupt/oversized `len` makes `end` swallow subsequent `00 00 01 BD` packets, and since the walk resumes at `p = end`, every later SPU PES in the file is silently dropped — no error, no damage event (this fn has no damage channel at all). Why it matters: one bad length byte kills the rest of the subtitle track invisibly. What correct looks like: clamp `end` to the next start code and surface a typed damage/parse signal.
Confidence: MEDIUM — Verification: read (control-flow re-derived); no in-tree test exercises MPEG-1 headers or oversized lengths.

F-004 [S3]
Where: spu.rs:309-334
Code: (verbatim)
```
    loop {
        if gb.pos() > gb.bit_len() {
            return Err(x);
        }
        let (len, color) = decode_run(&mut gb);
```
What is wrong: the exhaustion check runs BEFORE `decode_run`; `BitReader::read` (dvb/rle.rs:60-73) zero-extends past the end. If the final row of a field is completed by runs synthesized from out-of-bounds zero bits, `y >= field_h` breaks the loop and the field decodes `Ok` — a truncated SPU is accepted, with fabricated pixels, instead of the typed error the module contract promises. Mid-field truncation is caught one row late (one row of color-0 fill is written before `Err`). Blast radius: truncated `.sub` tails.
Confidence: MEDIUM — Verification: read (loop traced); no test feeds a truncated field stream.

F-005 [S4]
Where: spu.rs:36-38
Code: (verbatim)
```
//! threshold (`1, 4, 16, 64`): a 1-nibble code `n ≥ 4` is a run of
//! `n >> 2` pixels of color `n & 3`; longer codes prefix zero
//! nibbles and reach runs up to 16383; a final value `< 4` fills the
```
What is wrong: false number. The accumulator (decode_run, spu.rs:341-354) reads at most 4 nibbles with constraints n1≤3, n1·16+n2<16, ·16+n3<64 → max value 1023 (0x3FF), max run `1023>>2` = 255. "16383" (0x3FFF) is unreachable. The code is right; the doc is wrong.
Confidence: HIGH — Verification: read + hand simulation (nibbles 0,3,F,F → v=1023 → run 255).

F-006 [S4]
Where: spu.rs:455
Code: (verbatim)
```
    /// display start stamps the first sequence's date (0), the
    /// stop the second's (0x100 -> 2901 ms), like the reference
```
What is wrong: 0x100<<10 = 262144; 262144/90 = 2912.71 → 2912 ms, not 2901. The assertion (line 542) uses the formula and gets 2912; the comment arithmetic is wrong.
Confidence: HIGH — Verification: read + python-style arithmetic.

F-007 [S4]
Where: spu.rs:128-129, spu.rs:289
Code: (verbatim)
```
    /// The declared SPU size (for PES accumulation).
    pub size: usize,
```
```
        size: size.max(data.len()),
```
What is wrong: the field doc says "declared SPU size" but the value returned is `max(declared, received)` — a lie for over-delivered buffers. No consumer reads `SpuDecoded::size` for accumulation anyway (mod.rs:163-171 concatenates span payloads and only checks `SPU_MAX_SIZE`), so the field is also close to dead.
Confidence: HIGH — Verification: read + rg for `.size` consumers in vobsub/mod.rs.

F-008 [S4]
Where: spu.rs:211-222
Code: (verbatim)
```
                0x05 | 0x85 => {
                    if data.len() - pos < 6 {
                        return Err(SpuError::CommandOverrun);
                    }
                    x1 = ((data[pos] as usize) << 4) | (data[pos + 1] as usize >> 4);
```
What is wrong: 0x85 (HD coords) is fully parsed, its values assigned, and only then rejected — so a TRUNCATED 0x85 reports `CommandOverrun` while a complete one reports `HdUnsupported`; inconsistent error taxonomy for the same malformation class. Additionally the PES walker's PTS branches (spu.rs:379-405, MPEG-2 `pts` and both MPEG-1 variants) have zero test coverage: `pes_extraction_finds_substreams` sets flags 0x80 but supplies no PTS bytes (`hp+5 <= sub.len()` fails, pts stays `None`) — the PTS wiring into `extract_spu_pes` output is never executed by any test.
Confidence: HIGH — Verification: read + test-body trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display::fmt | 68-97 | REAL | — | error text matches variants |
| decode_spu | 137-291 | REAL (S2 defect F-001) | S2 | grammar walk correct; coord check inverted; HD typed; cap 128 KiB enforced (151-153) |
| decode_field | 295-336 | REAL (S3 F-004) | S3 | weave bounds verified safe for odd/even h; no panic possible; exhaustion check one row late |
| decode_run | 341-354 | REAL | — | threshold semantics correct; max run 255 (doc lies, F-005) |
| extract_spu_pes | 366-420 | REAL (S3 F-003) | S3 | linear total scan, progress guaranteed; len/advance inconsistency; swallows subsequent PES |
| find_start_code | 424-436 | REAL | — | trivial correct scan |
| decode_pts | 439-445 | REAL | — | 33-bit PTS + markers; roundtrip-tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nibble_rle_runs | YES | asserts exact (run,color) for 5 hand-computed nibble sequences |
| full_spu_decode | YES | end-to-end: coords, weave row order, colormap nibble order, both dates |
| interlaced_weave_is_correct | YES | 2x4 bitmap distinguishes top/bottom field placement — would fail on any weave swap |
| overrun_is_typed_error | YES | 188-px run in 2-wide row → exact RleOverrun{x:0,width:2} |
| truncated_and_invalid_are_typed | YES | TooSmall/InvalidCommandOffset/NeedMoreData all shape-asserted |
| pes_extraction_finds_substreams | PARTIAL | only MPEG-2 no-PTS happy path; PTS + MPEG-1 branches dead in tests (F-008) |
| pts_decode_90khz | YES for decode_pts | but not for its caller |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Reference: ffmpeg `dvdsubdec.c` (this wave's differential oracle)" | spu.rs:6-7 | UNVERIFIED / overstated | no ffmpeg source or .sub fixture in-tree; docs/worklog/archive-2026-10.md admits "NO reference-encoder differential"; tests are hand-crafted, not oracle-generated |
| "longer codes … reach runs up to 16383" | spu.rs:36-38 | FALSE | F-005: max run 255 |
| "lines are byte-aligned after each row" | spu.rs:39-40 | REAL-UNPROVEN | `gb.align()` at spu.rs:332 exists; consistent with tests, not differentiable against ffmpeg in-sandbox |
| "A run that would cross the line end is a typed decode error" | spu.rs:38-39 | REAL (with S3 hole) | spu.rs:315-317; last-row truncation hole F-004 |
| "dates are `(date << 10) / 90` ms" | spu.rs:29-30 | REAL | spu.rs:189-190 + tests |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (see header table).
- `used_color` (F-002) dead. `w == 0` check (F-001c) dead. `SpuDecoded::size` effectively unused downstream.

## 8. Dependencies & flags
- `crate::dvb::rle::BitReader` (pub(crate), same crate). `testkit::Must` dev-only. No feature gates. Std-only. No unsafe.

## 9. Open questions for the next agent
- W09-A10 (mod.rs): does `render_cue`'s crop depend on F-001's w=1 collapse? Does any mod.rs test feed inverted coords (would pin the S2)?
- Anyone with a real ffmpeg build: confirm dvdsubdec accepts h==1 and rejects x2<x1 (would confirm F-001 as an oracle divergence, not just a design flaw).
