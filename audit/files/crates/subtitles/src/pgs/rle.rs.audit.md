# AUDIT: crates/subtitles/src/pgs/rle.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 231 / 2 prod (decode_object, RleDamage::any) + 8 tests |
| File verdict | UNREACHABLE (real, tested; crate not wired to shipped apps) — decode of LEGAL streams verified CORRECT, hostile-stream handling diverges from its own contract |
| Reachable from shipped entry point? | NO — `decode_object` called only from pgs/mod.rs:556 (`rle::decode_object`) inside `crates/subtitles`; sole tree-wide dependent is `fuzz/Cargo.toml:50` (non-member) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b pgs::rle` → 8 passed 0 failed (exit 0); `python3` cross-verification (4 hand-crafted cases, Rust-algorithm transcription vs independent per-row reference semantics) — output pasted below |

## 1. One-paragraph verdict
The PGS RLE code table is implemented correctly for legal streams — python cross-verification of a hand-crafted 4×2 case (literals, `00 81 07` color run, `00 00` EOL, second row) reproduces the in-repo test vector exactly against an independent reference implementation, and all 8 tests pass. But the hostile-input path violates the module's own headline invariant: runs are clipped at the CANVAS edge (`n = w*h`), never at the ROW edge, so a run that crosses a row boundary paints pixels into the following row(s) — precisely the "smear pixels to the wrong row" the doc says must never happen — and when the crossing run happens to complete the canvas, the damage ledger records NOTHING (python case B: `00 88 05` into 4×2 → both rows filled with color 5, `clipped=0 short=0 truncated=false`). The reference (ffmpeg decode_rle) rejects run overflow per row. Additionally the 4 MiB canvas cap the doc advertises is not enforced in this function at all — it lives in the caller.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b pgs::rle` → `test result: ok. 8 passed; 0 failed`.
— verified by `python3` (independent per-row reference vs faithful transcription of this code): case A legal stream `[0x01,0x02,0x03,0x00,0x81,0x07,0x00,0x00,0x00,0x82,0x07,0x01,0x09]` into 4×2 → both implementations produce `[1,2,3,7,7,7,1,9]`, clean damage — matches the committed test. Long forms `00 4L LL` / `00 CL LL cc` and `00 8L cc` verified in cases and tests.

## 3. Findings
F-001 [S2]
Where: rle.rs:18-27 (contract), rle.rs:114-119 (impl)
Code: (verbatim)
```
//! a short line SNAPS to the next row boundary and a crossing run
//! is CLIPPED at the canvas edge — each recorded once per object in
//! [`RleDamage`] (the caller's `RleOverrun` ledger entry), because
//! a subtitle renderer must never smear pixels to the wrong row.
```
```
        let take = run.min(n - pos);
        if take < run {
            damage.clipped_runs += 1;
        }
        out[pos..pos + take].fill(color);
        pos += take;
```
What is wrong: the write cursor `pos` is flat over the whole `w*h` canvas; a run is clipped only at `n`, never at the row width `w`. A run crossing a row boundary smears color into the next row — the exact thing the doc's own invariant forbids. Python proof (executed): `00 88 05` (run of 8, color 5) into 4×2 → Rust-semantics output `[5,5,5,5,5,5,5,5]` with damage `(clipped=0, short=0, truncated=false)` — ZERO damage recorded; reference semantics reject the run (x+run > w → ffmpeg "run overflow" error). Sibling case `00 85 07` (run 5 into 4-wide) paints row 1 col 0 from a row-0 run and only escapes the ledger as `truncated` by luck of an unfilled canvas. Why it matters: the fuzz/mutation corpus (pgs_fuzz_survival) feeds exactly such streams; damage accounting — the crate's stated dirty-world deliverable — under-reports, and decoded bitmaps contain cross-row garbage the reference would refuse. What correct looks like: clip at `w - (pos % w)` per run, record `clipped_runs`, per the doc's own invariant.
Confidence: HIGH — Verification: ran (python cross-implementation, output above); ffmpeg comparison from spec knowledge — UNVERIFIED against ffmpeg source (not vendored).
F-002 [S3]
Where: rle.rs:29-32 (cap), rle.rs:60-62 (entry)
Code: (verbatim)
```
/// Object pixel-buffer cap: width × height indices, 4 MiB — twice a
/// 1080p canvas (Blu-ray objects are at most the video extent),
/// tight against hostile dimensions (D-2, the DVB precedent).
pub const MAX_OBJECT_PIXELS: u32 = 4 * 1024 * 1024;
```
```
pub fn decode_object(data: &[u8], width: u32, height: u32) -> (Vec<u8>, RleDamage) {
    let n = width as usize * height as usize;
    let mut out = vec![0u8; n];
```
What is wrong: the doc says "The canvas cap bounds allocation (D-2)" (line 27) but `decode_object` — a PUBLIC function — never reads `MAX_OBJECT_PIXELS`; the cap is enforced only by the caller (pgs/mod.rs:469, `w.saturating_mul(h) <= rle::MAX_OBJECT_PIXELS`). A direct call `decode_object(data, 65535, 65535)` allocates ~4 GiB before reading a single byte. In-crate parsed path is guarded (u16 ODS dims validated in ods_segment), so no file triggers it today; the pub API contradicts the documented self-bounding.
Confidence: HIGH — Verification: read + rg (`MAX_OBJECT_PIXELS` referenced in rle.rs only as the const definition).
F-003 [S4]
Where: rle.rs:8 (table), rle.rs:101-111 (EOL handling)
Code: (verbatim)
```
        if run == 0 {
            // End of line.
            let col = pos % w;
            if col > 0 {
                damage.short_lines += 1;
```
What is wrong: the code table documents ONLY `00 00` as end-of-line, but the implementation treats ANY zero-length run as EOL — including the 3-byte long form `00 40 00` (flags 0x40, length 0) and the 2-byte `00 80`+color form. Python case D (executed): `[0x03, 0x00,0x40,0x00, 0x04]` into 4×2 → Rust snaps the cursor to row 1 (`short_lines=1`, pixel 4 lands at row 1 col 0) while reference semantics treat it as a 0-length no-op (pixel 4 adjacent to pixel 3). Undocumented behavior extension on hostile input; also `00 80` consumes a color byte before acting as EOL.
Confidence: HIGH — Verification: ran (python, case D).
F-004 [S4]
Where: rle.rs:226-230
Code: (verbatim)
```
    fn zero_dimensions_decode_to_empty() {
        let (img, dmg) = decode_object(&[0x01, 0x02], 0, 0);
        assert!(img.is_empty());
        assert!(dmg.truncated || !dmg.any());
    }
```
What is wrong: the second assertion is a tautology — `dmg.truncated || !dmg.any()` is true for every possible damage value (if not truncated and nothing else flagged, `!any()` is true). It asserts nothing; only the `img.is_empty()` check is real.
Confidence: HIGH — Verification: read (truth-table check).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| decode_object | 60-128 | REAL (hostile-path defect) | S2/S3/S4 | legal streams exact (python-verified); cross-row smear F-001; no cap F-002; zero-run-EOL F-003; bounds otherwise airtight (pos<n invariant, `data.get` guards, w>0 guaranteed before `% w`) |
| RleDamage::any | 49-51 | REAL | — | trivial, correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| literal_pixels_and_color_runs | YES | exact 8-pixel vector + clean damage |
| eol_and_short_color_runs | YES | snap + zero-fill discriminated |
| long_runs_both_colors | YES | 16-bit forms + exact-fill |
| overrun_run_is_clipped_and_typed | YES | canvas-clip + clipped_runs=1 |
| short_line_snaps_to_next_row | YES | position discriminates snap vs shift |
| truncated_data_pads_transparent | YES | truncated flag + zero pad |
| trailing_bytes_past_full_canvas_are_ignored | YES | no-damage pinned |
| zero_dimensions_decode_to_empty | NO | F-004 tautology |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verified against the reference decoder (ffmpeg pgssubdec.c decode_rle, fetched 2026-10-25)" | rle.rs:3-4 | UNVERIFIED | no ffmpeg source/.sup fixtures in-tree |
| "a subtitle renderer must never smear pixels to the wrong row" | rle.rs:25-26 | FALSE vs code | F-001 python case B |
| "a crossing run is CLIPPED at the canvas edge — each recorded" | rle.rs:23-25 | PARTLY FALSE | clip happens, but crossing runs that complete the canvas record NOTHING (case B) |
| "The canvas cap bounds allocation (D-2)" | rle.rs:27 | FALSE for this fn | F-002: cap enforced only by caller mod.rs:469 |
| code table nn / 00 00 / 00 0L / 00 4L LL / 00 8L cc / 00 CL LL cc | rle.rs:6-13 | REAL | python case A + long-run tests |

## 7. Dead / duplicated / unreachable code
- No dead functions. `MAX_OBJECT_PIXELS` is used only outside this file (mod.rs) despite the doc placing the cap here. Whole file unreachable from shipped entry points.

## 8. Dependencies & flags
- None beyond std; no unsafe; no features. One `Vec` per object bounded by caller's cap.

## 9. Open questions for the next agent
- pgs/mod.rs owner: `palette.rgba[index as usize]` consumes decode_object output where `color` may be any u8 — confirm the palette bank is 256 entries (OOB would panic); that check belongs to palette.rs/mod.rs, not audited here.
- Anyone with ffmpeg source: confirm decode_rle's run-overflow is an error return (would firm F-001's "reference rejects" from UNVERIFIED to confirmed).
