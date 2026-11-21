# AUDIT: crates/subtitles/src/dvb/rle.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 495 / 11 (+1 test helper, 11 tests) |
| File verdict | UNREACHABLE (real, careful, in-crate test-proven code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `rle::decode_line_*` called only from `dvb/mod.rs:770,774,778` (`paint_line`); `BitReader` also re-used by `vobsub/spu.rs:42`. The `subtitles` crate is consumed only by its own tests and the non-gated nightly `fuzz/` workspace; no app/engine crate depends on it (see ass/tags.rs audit §2 for the full proof) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n 'decode_line_|non_mod|align\(\)' crates/subtitles/src/dvb` (call sites + non_mod flow) · `rg -n 'LineStop|BufferExhausted|RowFull' crates/subtitles` (LineStop consumers) · read `dvb/mod.rs:575-780` (paint_field/paint_line) · `sed -n` quote verification · baseline citation (below) |

## 1. One-paragraph verdict
The EN 300 743 §7.2.5 RLE code tables are transcribed coherently (2/4/8-bit
prefix trees match the module's own table documentation, run-length offsets
3/12/29, 2/4/9/25, 127 all internally consistent), the bit reader is
bounds-guarded with zero-extension past EOF, every pixel write is double-end
clamped so no malformed stream can panic, and the 11 hand-packed bit-vector
tests are genuinely mutation-sensitive. Two real defects: the
`non_modifying` (non_mod) suppression test is applied to the RAW 2/4-bit code
in single-pixel paths but to the MAPPED CLUT index in run paths — with the
default `map4to8` (`[0x00, 0x11, …]`) the two paths visibly disagree for a
non-modifying 4-bit object in an 8-bit region, and no test in the crate
exercises `non_mod = true` at all; and the `LineStop` "damage data" is
discarded by every production caller, so the module's typed-damage story for
this file is not what the doc claims. The header's "exactly as the reference
decoders (ffmpeg dvbsubdec, the wave's differential oracle)" claim
contradicts the feature matrix, which admits NO DVB differential oracle
exists (ffmpeg refuses to encode text→bitmap). As with the whole crate:
nothing shipped links this code.

## 2. What actually works (verified)
- Code-table structure vs the module's own documentation: every prefix arm
  (`001 nnn vv`, `0001`, `000001`, `000010 nnnn vv`, `000011 nnnnnnnn vv`,
  `000000`; 4-bit `0 nnn`/`10 nn`/`11 10`/`11 11`/`11 01`/`11 00`; 8-bit
  `00 0 rrrrrrr` / `00 1 rrrrrrr vvvvvvvv`) matches the documented tables —
  verified by reading both sides in this file; run-length offsets (3, 12, 29,
  2, 4, 9, 25, and the 8-bit 1..127) are consistent between doc, code, and
  the tests' hand-packed bit vectors.
- All 11 in-file tests pass: baseline
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` lists
  `test dvb::rle::tests::two_bit_codes … ok` through
  `eight_bit_eol_and_exhaustion … ok`, inside the subtitles lib run
  `test result: ok. 267 passed; 0 failed` (line ~5269-5540). The fuzz
  survival lane `dvb_vobsub_fuzz_survival` also passed there.
- No-panic discipline: `fill` clamps `start`/`end` to `row.len()` on both
  ends (rle.rs:317-318); single-pixel writes are guarded by `x < row.len()`;
  `BitReader::read` guards `self.pos / 8 < self.data.len()` (rle.rs:64);
  `x` overflow impossible (runs ≤ 284, x clamped to `row.len()` after every
  fill, singles bounded by buffer bits on 64-bit usize).
- `BitReader` is shared with `vobsub/spu.rs:42` (`use
  crate::dvb::rle::BitReader;`) — `align()` is used at spu.rs:332, so no
  dead accessor.
- Zero allocations in all three decoders (borrowed row, borrowed field) —
  the "Never … allocations" doc claim holds (stronger than promised).

## 3. Findings

### F-001 [S2] non_mod suppression checks the RAW code in single-pixel paths but the MAPPED value in run paths
Where: crates/subtitles/src/dvb/rle.rs:131-146 (and the 4-bit twin at 206-230, fill at 316-325)
Code:
```
        let bits = gb.read(2);
        if bits != 0 {
            if x < row.len() && (!non_mod || bits != 1) {
                let v = map.map_or(u32::from(bits), |m| u32::from(m[bits as usize]));
                row[x] = v as u8;
            }
            x += 1;
            continue;
```
versus the run path in the same function:
```
                // 001 nnn vv — 3..10 pixels.
                let run = u32::from(gb.read(3)) + 3;
                let v = gb.read(2);
                let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                x = fill(row, x, run, v as u8, non_mod);
```
with `fill` suppressing on the value it receives:
```
    if !(non_mod && v == 1) {
        for slot in &mut row[start..end] {
            *slot = v;
        }
    }
```
What is wrong: for single-pixel codes the non-modifying test is
`bits != 1` on the RAW 2/4-bit code, applied BEFORE `map` translation; for
run codes the value is translated through the 2to4/2to8/4to8 map FIRST and
`fill` then tests the MAPPED value against 1. The two paths therefore
disagree whenever the map crosses the value 1 — including the DEFAULT
`map4to8 = [0x00, 0x11, 0x22, …]` (dvb/mod.rs:635-638): a non-modifying
4-bit object in an 8-bit region writes RUNS of code 1 (mapped 0x11 ≠ 1 →
painted) but suppresses SINGLE pixels of code 1 (raw == 1). Custom
`0x20/0x21/0x22` map sub-blocks can map other codes onto 1, suppressing
runs whose singles would paint.
Why it matters: EN 300 743 non-modifying regions are how broadcast subtitle
streams implement partial updates; inconsistent suppression corrupts the
update region — wrong pixels on screen, not just damage counters. Exactly one
of the two paths can match the reference decoder (ffmpeg `dvbsubdec` checks
the raw code pre-map in both paths, to the best of my recollection —
UNVERIFIED, no network/source in repo), so at least one path is wrong.
What correct looks like: one canonical decision — either test the raw code
in `fill`'s callers (pass the raw code alongside) or test the mapped index
in the single-pixel paths — plus at least one test with `non_mod = true`
and a map present.
Confidence: HIGH on the internal inconsistency (both code paths quoted
verbatim above); MEDIUM on which side matches ffmpeg (UNVERIFIED — no
ffmpeg source in-tree, no network).

### F-002 [S3] LineStop "damage data" is discarded by every production caller
Where: crates/subtitles/src/dvb/rle.rs:94-103 (enum) and the call sites dvb/mod.rs:770,774,778
Code (rle.rs):
```
/// Why a pixel line stopped early (damage data, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineStop {
    /// The EOL code was seen; the rest of the byte is padding.
    EndOfLine,
```
Code (dvb/mod.rs:770, the only production consumers):
```
            Rle::Two(m) => {
                let (used, x_end, _stop) = rle::decode_line_2bit(data, row, x, m.as_ref(), non_mod);
                (used, x_end)
```
What is wrong: the module doc promises clipped/exhausted lines surface as
"a typed damage event, never a panic" (rle.rs:36-38) and names LineStop the
"damage data" — but all three production call sites bind `_stop` and throw
it away; `rg 'LineStop' crates/subtitles` shows the enum is consumed only by
rle.rs's own tests. The `RleOverrun` damage actually recorded by the decoder
comes from a different mechanism (paint_field's placement guard,
dvb/mod.rs:647-652), so `BufferExhausted` vs `RowFull` never reaches the
damage ledger.
Why it matters: the honest-degradation story (a wave-level selling point of
this crate) is weaker than documented for DVB pixel data: early buffer
exhaustion mid-line is silent; a reader of rle.rs believes damage is
reported where it is not.
What correct looks like: paint_line forwards the stop reason into
`self.damage(...)` (e.g. a `DvbDamage::RleEarlyStop`) or the enum and the
third tuple slot are deleted.
Confidence: HIGH — Verification: read + `rg` (all consumers listed).

### F-003 [S3] Header claims ffmpeg is "the wave's differential oracle"; the matrix says no DVB differential exists
Where: crates/subtitles/src/dvb/rle.rs:1-4
Code:
```
//! DVB subtitle pixel RLE (ETSI EN 300 743 §7.2.5, Table 9/10/11) —
//! the 2-bit, 4-bit and 8-bit code tables, exactly as the reference
//! decoders (ffmpeg `dvbsubdec`, the wave's differential oracle)
//! interpret them.
```
What is wrong: docs/feature-matrix.toml row `subtitle-dvb-subs` (line 1006,
notes) states the opposite for this exact domain: "honest gap: NO
reference-encoder differential (ffmpeg 7.1.5 refuses text->bitmap; bit
tables cross-checked line-by-line vs ffmpeg RLE semantics), no corpus
lane". The file header's "the wave's differential oracle" wording asserts a
differential relationship that never ran; the "cross-checked line by line"
act left no committed artifact (no ffmpeg dump, no table-vs-table file under
docs/ or tests/fixtures — fixtures dir holds only ass/fuzz/canonical/vtt).
Why it matters: evidence inflation at the file's front door; the tables may
well be correct, but "exactly as the reference decoders interpret them" is
an unprovable claim presented as proven, and it contradicts the row that
cites this file.
What correct looks like: header says "transcribed from EN 300 743 and
manually compared against ffmpeg's dvbsub_read_{2,4,8}bit_string; no
automated differential exists (see matrix row)".
Confidence: HIGH — Verification: read of both sites; contradiction is
textual, not behavioral.
Confidence that the tables themselves are wrong: LOW (they are internally
consistent and match my recollection of EN 300 743's structure).

### F-004 [S4] decode_line_8bit returns an unclamped byte count; 2/4-bit clamp
Where: crates/subtitles/src/dvb/rle.rs:291-306
Code:
```
        // 00 — run code follows.
        let n = *data.get(i).unwrap_or(&0);
        i += 1;
        let run = u32::from(n & 0x7f);
```
through the tail:
```
    (i, x, stop)
}
```
What is wrong: when the buffer ends right after a `00` byte (or after a
`00 8x` run header), `data.get(i).unwrap_or(&0)` reads a phantom byte and
`i += 1` still executes, so the returned count can be `data.len() + 1` or
`+ 2`. The 2-bit and 4-bit decoders clamp their return
(`(gb.consumed_bytes().min(data.len()), x, stop)` — rle.rs:183, 259); the
8-bit one does not.
Why it matters: inconsistent return contract across the three siblings; the
current caller absorbs it (paint_field's `i += used` then fails `i <
data.len()` and exits — no panic today), but any future caller slicing
`&data[used..]` panics. Also untested: no 8-bit test asserts `used` for a
truncated buffer.
What correct looks like: `i.min(data.len())` in the return, mirroring the
siblings; one truncated-buffer `used` assertion.
Confidence: HIGH — Verification: read of both siblings' returns and the
caller's loop guard (dvb/mod.rs:642, 654, 671).

### F-005 [S4] Test helper's consumption assertion is slack
Where: crates/subtitles/src/dvb/rle.rs:337
Code:
```
        assert!(used <= data.len() + 1, "consumed {used} of {}", data.len());
```
What is wrong: the 2-bit decoder's return is clamped
(`.min(data.len())`), so `used <= data.len()` always holds; allowing `+1`
weakens the assertion so a regression that removes the clamp would still
pass this check.
Why it matters: the one invariant guarding the byte-walk contract is tested
looser than the code guarantees.
What correct looks like: `assert!(used <= data.len())`.
Confidence: HIGH — Verification: read of rle.rs:183 clamp vs the assert.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BitReader` (struct) | 46-50 | REAL | — | Borrowed slice + bit pos; shared with vobsub (spu.rs:42) — not dead. |
| `BitReader::new` | 53-55 | REAL | — | Trivial. |
| `BitReader::read` | 60-73 | REAL | — | MSB-first; `debug_assert!(n <= 16)` (callers pass ≤ 8); zero-extension past EOF documented as reference behavior; bounds-guarded index (line 64) — no panic. |
| `BitReader::consumed_bytes` | 76-78 | REAL | — | `div_ceil` byte-align semantics; correct for EOL+padding. |
| `BitReader::align` | 81-83 | REAL | — | Used by vobsub/spu.rs:332 only — legit cross-module reuse. |
| `BitReader::bit_len` / `pos` | 85-91 | REAL | — | Accessors. |
| `LineStop` (enum) | 96-103 | REAL | S3 (F-002) | Correct data, wrong story: production callers discard it; test-only consumer. |
| `decode_line_2bit` | 110-184 | REAL | S2 (F-001) | Prefix tree matches doc + EN 300 743 structure; clipping; progress guaranteed (≥2 bits/iter); non_mod raw-vs-mapped inconsistency. |
| `decode_line_4bit` | 187-260 | REAL | S2 (F-001) | Same table discipline (0000-nibble continuation grammar); same non_mod inconsistency; EOL via `0 000` and `11 00`. |
| `decode_line_8bit` | 263-307 | REAL | S4 (F-004) | Byte codes correct; truncated reads via `data.get().unwrap_or(&0)` typed; unclamped return count. |
| `fill` | 316-325 | REAL | S2 partner (F-001) | Double-end clamp — the no-panic core, pinned by test; suppression test operates on the value it is handed (mapped for 2/4-bit runs — the inconsistency's other half). |
| tests helper `check_2bit` | 332-338 | REAL | S4 (F-005) | Slack `+1` in the consumption assert. |
| 11 tests | 340-494 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| two_bit_single_pixel_codes | YES | Hand-packed `0b01_10_11_00` → exact `[1,2,3]` + EOL; any table-code change flips it. |
| two_bit_switch_runs_and_eol | YES | Pins `001 000 01` = run 3 of 1 AND the `0001` single-zero code. |
| two_bit_long_runs | YES | Pins 12..27 and 29..284 forms with exact full-row fills. |
| two_bit_map_table_translates | YES | Pins the map path AND `used == 2` byte accounting. |
| two_bit_run_clipped_at_row_end | YES | 284-px run into a 5-wide row: exact 5 pixels, no wrap. |
| two_bit_run_after_x_passed_row_end_is_clipped_not_panicking | YES | The fuzz-survival pin: x past end + run → no slice panic (would fail on any clamp regression). |
| two_bit_buffer_exhausted_is_typed | YES | Distinguishes zero-extension-EOL from true exhaustion on empty input. |
| four_bit_codes | YES | Hand-packed nibbles pin singles + `10 nn vvvv` run form. |
| four_bit_zero_runs_and_eol | YES | Pins `0 nnn` zero-run, `11 10` run, and RowFull-on-exhaustion. |
| eight_bit_codes | YES | Pins `00 82 vv` colored run, `00 03` zero run, `00 80 vv` zero-run no-op, EOL, `used == 8`. |
| eight_bit_eol_and_exhaustion | YES | EOL, empty-buffer, truncated-color (clipped, typed) — but does NOT assert `used` on the truncated case (F-004 gap). |
Missing coverage (not a test-honesty failure of existing tests, but a gap):
NO test anywhere in the crate passes `non_mod = true` (rg over src/ shows
non_mod only ever `false` in rle.rs tests; mod.rs passes the object flag in
production) — the S2 inconsistency is invisible to the suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the 2-bit, 4-bit and 8-bit code tables, exactly as the reference decoders (ffmpeg dvbsubdec, the wave's differential oracle) interpret them" | rle.rs:1-4 | UNVERIFIED / overstated (F-003) | No DVB differential exists per matrix row `subtitle-dvb-subs` notes (feature-matrix.toml:1006); no committed cross-check artifact; tables verified internally consistent only. |
| "A run that would cross the row end is clipped … a typed damage event, never a panic" | rle.rs:36-38 | SHALLOW | Clipping+no-panic REAL (fill clamp + test); "typed damage event" not delivered from this file — LineStop discarded (F-002); RleOverrun damage comes from paint_field's guard instead. |
| "Panics, allocations proportional to anything but the row length" (Never section) | rle.rs:40-42 | REAL | Zero allocations in all three decoders; no panic path found (bounds guards at 64, 133, 175, 208, 284, 317-318). |
| "EN 300 743 … commits ab696b4+93a53ec … 230/0 subtitles gates incl. dvb_vobsub_fuzz_survival" | matrix row `subtitle-dvb-subs` evidence (feature-matrix.toml:1005) | REAL (in-crate) | Baseline no-fail-fast log: `dvb::rle::tests::*` all ok within 267/267; `dvb_vobsub_fuzz_survival` suite ok. |
| "A row that fills exactly does NOT end implicitly: the reader keeps consuming … until the EOL" | rle.rs:29-35 | REAL (deliberate policy) | Code loops until EOL/exhaustion regardless of x; documented divergence justification (0xF0 walk); ffmpeg's per-line `x < w` stop NOT replicated — flagged for the record, unverifiable here. |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (proof in the
  ass/tags.rs audit §2; same crate-level fact).
- `LineStop` is production-dead (test-only consumer) — F-002.
- 2-bit/4-bit/8-bit decoders duplicate the map-then-fill boilerplate 3× per
  function (9 near-identical `let v = map.map_or(...); x = fill(...)` blocks)
  — the S2 inconsistency was born in exactly this duplication. Not filed
  separately; the fix for F-001 should collapse it.

## 8. Dependencies & flags
- No `[features]`, no cfg gates, no unsafe, std-only. `BitReader` is
  `pub(crate)` shared with vobsub — a small, sane intra-crate seam.
- No external deps (crate is media-types-only at runtime).

## 9. Open questions for the next agent
- dvb/mod.rs owner (same wave): confirm which non_mod side matches ffmpeg
  `dvbsub_read_2bit_string`/`_4bit_string` (raw-code check pre-map, I
  believe) and fix F-001's wrong side; add a non_mod=true test with a map.
- dvb/mod.rs owner: paint_field's placement guard (line 647) reports
  RleOverrun when `byte != 0xF0 && x >= rw` — verify a line that ends
  exactly at the region edge with a legit 0xF0 next is not mis-damaged
  (x_end from fill is clamped to rw, so it should be fine — traced, not
  tested here).
- Whoever owns the ffmpeg-oracle story: the DVB row's "cross-checked
  line-by-line" claim needs an artifact (a committed table comparison or a
  recorded ffmpeg decode) or softer wording.
