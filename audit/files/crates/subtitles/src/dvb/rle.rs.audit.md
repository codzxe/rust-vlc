# AUDIT: crates/subtitles/src/dvb/rle.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 495 / 8 production fns + 11 tests + 1 helper |
| File verdict | UNREACHABLE (genuine, fuzz-pinned line decoder; crate never linked into a shipped binary) |
| Reachable from shipped entry point? | NO — `subtitles` is not a dependency of lumen-cli/desktop/web (see pgs/mod.rs audit F-002); in-crate chain: DvbDecoder::decode_display_set (dvb/mod.rs) -> pixel_data -> paint_line (dvb/mod.rs:770-778) -> decode_line_{2,4,8}bit |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `./target/debug/deps/subtitles-70f89de07410626f dvb::rle` → 11 passed, 0 failed (exit 0); `./target/debug/deps/dvb_vobsub_fuzz_survival-bc4f65a1d8582a31` → 2 passed (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
The 2/4/8-bit EN 300 743 line decoders are genuinely implemented, panic-free
(fuzz sweep passes — I ran it), allocation-free (only `fill` writes into the
caller's row), and terminate on all inputs (every loop iteration advances the
bit cursor ≥2 bits). Two real defects: the `non_mod` (non-modifying color)
check is applied to the RAW code for single pixels but to the MAPPED value for
runs — internally inconsistent and divergent from ffmpeg when an in-stream
map table maps a non-1 code to 1; and the module doc's 4-bit zero-run range
("2..9") contradicts the code (produces 3..9). The header's claim that ffmpeg
is "the wave's differential oracle" is false — the wave's own fuzz test admits
no ffmpeg-generated corpus exists, so reference fidelity rests on the author's
manual transcription, never executed against a reference.

## 2. What actually works (verified)
— `./target/debug/deps/subtitles-70f89de07410626f dvb::rle` → `test result:
ok. 11 passed; 0 failed` (all three code tables, clipping, maps, exhaustion).
— `./target/debug/deps/dvb_vobsub_fuzz_survival-bc4f65a1d8582a31` → 2 passed
(seeded mutations + truncations through the full DVB decoder never panic;
region RGBA sizes exact; damage ledger capped).
— Bounds: every slice write goes through `fill` (L316-325) which clamps
`start`/`end` into `row.len()`; single-pixel writes are guarded by
`x < row.len()`; `used` is clamped by `.min(data.len())` (L183/259) so the
caller's `&data[i..]` cannot overrun.

## 3. Findings

F-001 [S3]
Where: crates/subtitles/src/dvb/rle.rs:131-146 (single vs run), 227-245 (4-bit), 301-303 (8-bit), 316-324 (fill)
Code:
```
        let bits = gb.read(2);
        if bits != 0 {
            if x < row.len() && (!non_mod || bits != 1) {
                let v = map.map_or(u32::from(bits), |m| u32::from(m[bits as usize]));
                row[x] = v as u8;
            }
```
(and, six lines later, the run path)
```
                let v = gb.read(2);
                let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                x = fill(row, x, run, v as u8, non_mod);
```
(and `fill`'s check)
```
    if !(non_mod && v == 1) {
```
What is wrong: `non_mod` suppresses pixels of "value 1", but the two code
paths test different values: single pixels test the RAW 2/4-bit code
(`bits != 1`, before the map table), runs test the MAPPED value inside `fill`
(`v == 1`, after `map[v]`). When an in-stream map sub-block (0x20/0x21/0x22,
dvb/mod.rs:705-730 — attacker-controlled bytes) maps code 2 → 1, a RUN of
code-2 pixels is silently skipped while single code-2 pixels are written.
ffmpeg's `dvbsub_read_{2,4}bit_string` tests the raw code (`non_mod != 1 ||
bits != 1`) in both paths, so this diverges from the reference and from its
own sibling path. Why it matters: non_mod regions are used for progressive
reveal (animation) — pixels silently missing from runs = corrupted subtitle
rendering on valid-ish streams, plus an internal inconsistency that guarantees
one of the two paths is wrong. What correct looks like: check the raw code
value before mapping in all paths (pass `code == 1` out of the decoders, or
apply non_mod before `fill`).
Confidence: HIGH on the internal inconsistency (read); MEDIUM on ffmpeg's
exact behavior (no ffmpeg source in sandbox — UNVERIFIED against oracle).

F-002 [S3]
Where: crates/subtitles/src/dvb/mod.rs:770-778 (only production caller), rle.rs:94-103
Code:
```
/// Why a pixel line stopped early (damage data, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineStop {
```
(caller, dvb/mod.rs:770)
```
                let (used, x_end, _stop) = rle::decode_line_2bit(data, row, x, m.as_ref(), non_mod);
```
What is wrong: `LineStop::BufferExhausted` is documented as "damage data" but
the sole production caller binds it to `_stop` and drops it. A pixel-data
field whose bits run out mid-line (truncated object) decodes with zero-fill
and NO damage event in the DVB ledger — the malformation is silently absorbed,
contradicting the decoder-wide "every malformation is typed" contract.
Why it matters: hostile/truncated DVB bitstreams produce "clean" decodes; the
damage ledger (capped, surfaced to callers) lies by omission. What correct
looks like: paint_line propagates `BufferExhausted` into a DvbDamage variant
(truncated pixel line).
Confidence: HIGH — Verification: read + rg (no other production caller).

F-003 [S4]
Where: crates/subtitles/src/dvb/rle.rs:16-17 vs 216-222
Code:
```
//! **4-bit**: `1..15` = one pixel; after a `0000` nibble: `0 nnn`
//! (`nnn != 0`) = 2..9 pixels of 0; `10 nn vvvv` = 4..7 of `vvvv`;
```
(vs the code)
```
            0 => {
                let run = u32::from(gb.read(3));
                if run == 0 {
                    stop = LineStop::EndOfLine;
                    break;
                }
                x = fill(row, x, run + 2, map.map_or(0, |m| m[0]), false);
```
What is wrong: the doc says `0 nnn` (nnn≠0) yields 2..9 pixels; the code
yields `nnn + 2` for nnn ∈ 1..7 = 3..9. Doc and code disagree by one — one of
them contradicts EN 300 743 Table 9 (no spec copy in sandbox to arbitrate;
ffmpeg-from-memory says +2, i.e. the code is right and the doc wrong).
Why it matters: the module doc is the declared transcription of the spec;
an off-by-one in it means the "cross-checked line by line" claim was not
performed on this line. What correct looks like: fix the doc (or the code
after checking Table 9).
Confidence: HIGH (doc/code contradiction provable from the file); UNVERIFIED
which matches the spec.

F-004 [S4]
Where: crates/subtitles/src/dvb/rle.rs:1-4
Code:
```
//! DVB subtitle pixel RLE (ETSI EN 300 743 §7.2.5, Table 9/10/11) —
//! the 2-bit, 4-bit and 8-bit code tables, exactly as the reference
//! decoders (ffmpeg `dvbsubdec`, the wave's differential oracle)
//! interpret them.
```
What is wrong: there is no differential oracle. The wave's own fuzz test
(crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:26-33, "Why no
ffmpeg-generated corpus (honest)") states ffmpeg refuses text→bitmap
transcoding and no DVB corpus exists, so "the reference-encoder differential
is NOT generatable here". "Exactly as the reference decoders interpret them"
is a manual-transcription claim, never executed against ffmpeg. The tables
may well be right (they match ffmpeg's structure as best I can recall), but
the header asserts a verification method that never ran.
Confidence: HIGH — Verification: read (contradicting doc cited in-repo).

F-005 [S4]
Where: crates/subtitles/src/dvb/rle.rs:316-318
Code:
```
    let start = x.min(row.len());
    let end = (x + run as usize).min(row.len());
```
What is wrong: `x + run as usize` is an unchecked add. `x` advances past the
row end unclamped (by design, L311-313) and can reach ~36× the field-buffer
length via all-max runs; on a 32-bit target (wasm32) a ~128 MiB pixel-data
field could wrap `x` so `end < start`, and `&mut row[start..end]` panics even
in release — violating "Never … panics" (L40-42). On 64-bit targets the wrap
is physically unreachable (needs ~2^63 pixels of advance) and the crate is not
built for wasm32 by any current lane, so this is latent.
What correct looks like: `x.saturating_add(run as usize).min(row.len())`.
Confidence: MEDIUM — Verification: read (arithmetic; not executable on any
available target).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BitReader::new` | 53-55 | REAL | — | borrows field buffer |
| `BitReader::read` | 60-73 | REAL | — | MSB-first, zero-extension past end (matches reference), `debug_assert!(n<=16)` never violated by callers (≤8) |
| `BitReader::consumed_bytes` | 76-78 | REAL | — | div_ceil byte align |
| `BitReader::align` | 81-83 | REAL | — | unused in-tree? see §7 |
| `BitReader::bit_len`/`pos` | 85-91 | REAL | — | loop guards |
| `decode_line_2bit` | 110-184 | REAL | S3 | F-001 non_mod inconsistency; tree matches doc table; termination proven (≥2 bits/iter) |
| `decode_line_4bit` | 187-260 | REAL | S3/S4 | F-001 + F-003 doc off-by-one |
| `decode_line_8bit` | 263-307 | REAL | — | byte codes; `00 80 vv` run-0 no-op matches test; truncated color reads 0 (typed stop) |
| `fill` | 316-325 | REAL | S4 | clamped both ends; F-005 unchecked add (32-bit latent) |
| `LineStop` enum | 95-103 | REAL | S3 | F-002: BufferExhausted never surfaces as damage |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| two_bit_single_pixel_codes / switch_runs_and_eol / long_runs | YES | exact expected pixels incl. run-length arithmetic (13, 31 px) |
| two_bit_map_table_translates | YES | asserts mapped value 0x88 and used=2 (bit-accurate EOL position) |
| two_bit_run_clipped_at_row_end | YES | 284-px run into 5-wide row, all pixels set |
| two_bit_run_after_x_passed_row_end_is_clipped_not_panicking | YES | the fuzz-survival pin: x past end + run → no slice panic |
| two_bit_buffer_exhausted_is_typed | YES | distinguishes EOL-in-padding vs empty buffer |
| four_bit_codes / zero_runs_and_eol | YES | exact nibble packing, RowFull vs EOL distinction |
| eight_bit_codes / eol_and_exhaustion | YES | run-0 no-op, truncated color → zeros, stop reasons |
Weakness: every vector was written by the decoder's author from the same
spec reading — no external oracle (F-004). A shared misreading of Table 9/10
would pass all tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly as the reference decoders … interpret them" (differential oracle) | rle.rs:1-4 | FALSE (as a verification claim) | F-004; tests/dvb_vobsub_fuzz_survival.rs:26-33 admits no oracle |
| "A run that would cross the row end is clipped … a typed damage event, never a panic" | rle.rs:36-38 | SHALLOW | clipping verified (tests, ran); the *damage event* part is dropped by the caller (F-002) |
| "Never … Panics, allocations proportional to anything but the row length" | rle.rs:40-42 | REAL (64-bit) / latent 32-bit panic (F-005) | fuzz survival ran, 0 panics |
| 4-bit "`0 nnn` … = 2..9 pixels of 0" | rle.rs:16-17 | FALSE vs its own code (3..9) | F-003 |

## 7. Dead / duplicated / unreachable code
- `BitReader::align` (L81-83): no callers in the crate (rg: only definition).
  Dead helper.
- `LineStop::RowFull`/`BufferExhausted`: produced, tested, never consumed in
  production (F-002) — half-dead vocabulary.
- Copy-paste: the three `decode_line_*` functions repeat the same
  single-pixel/non_mod/map logic with drift between the paths (F-001 is the
  direct consequence — the single path and run path diverged).

## 8. Dependencies & flags
No deps, no unsafe, no allocation in decode paths. `pub(crate)` visibility —
not part of the crate's public API; only reachable via DvbDecoder.

## 9. Open questions for the next agent
- dvb/mod.rs auditor: confirm pixel_data's placement guard (mod.rs:647-653)
  covers every case F-002's dropped `BufferExhausted` would have reported.
- Anyone with EN 300 743 access: arbitrate F-003 (nnn+1 vs nnn+2).
- If a map-table-bearing real DVB stream ever lands in fixtures, add a
  non_mod+map differential case (F-001's divergence would show).
