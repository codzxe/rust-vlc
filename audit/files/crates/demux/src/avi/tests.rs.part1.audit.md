# AUDIT: crates/demux/src/avi/tests.rs#part1 (lines 1-625)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 625 (chunk 1-625 of 3128; parts 2-5 = 626-3128 → A08/A07/A06/A09) / 24 items (17 builders/harness + 7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (cfg(test) module — `avi/mod.rs:1271 mod tests`); the code under test is reachable: `lumen open x.avi` → probe "avi" → `apps/cli/src/containers/mod.rs` avi arm → AviDemuxer; baseline AVI lane executed end-to-end (`audit/runs/baseline/avi-firstvertical-open-session.txt`, 60/60 frames in `rc-smoke-engine-only.log.txt`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full Read of 1-625; `grep -n "mod tests"` avi/mod.rs → :1271; `grep -n "fn damage_total\|fn stream_format\|fn main_header"` avi/mod.rs → :382/:389/:398 (asserted accessors exist); baseline greps: 58 `avi::tests::` all ok, part1's 4 named tests at nofailfast:1641/1666/1669/1674; `grep -n "0x110\|avih"` tools/xtask/src/cmd/fixtures.rs → :1618; `sed -n` verification of every quoted range; `grep -n "scale\|ssize"` avi/header.rs → timebase + `ticks_for_sample_size` (builder-convention cross-check) |

## 1. One-paragraph verdict
Part 1 is the in-memory AVI builder plus the header/track-table test battery, and it is the rare test module that earns its keep: the builder's byte layouts were verified field-by-field against RIFF/AVISTREAMHEADER/BITMAPINFOHEADER/WAVEFORMATEX(TENSIBLE) rules (all correct, including the signed `biHeight` and the 40-byte extensible form), and every one of the 7 tests pins real observable output (codec strings, time bases, span ticks, track ordinals, typed error codes, Recovered events) — each would fail under a concrete mutation. All 7 executed and passed in the baseline. The single defect is cosmetic-but-real: the builder hard-codes `AVIF_HASINDEX` (0x110) even when it writes no `idx1`, so every `with_idx1=false` file is a self-lying header — which the passing tests incidentally prove the demuxer ignores, but which betrays the module's "mirrors fixtures.rs byte-for-byte" claim.

## 2. What actually works (verified)
- Builder layouts verified by manual field arithmetic: `avih` 56 B in spec order (35-50), `strh` 56 B (fcc+handler+12+scale+rate+start+length+suggested+quality+ssize+rcFrame, 53-74), `strf_video` 40-byte BITMAPINFOHEADER (76-87), `strf_wave` 16-byte WAVEFORMATEX (95-104), `strf_extensible` 18+22=40 B (120-135); WORD padding in `chunk` (24-26); idx1 offsets relative to the `movi` form FourCC (297) — exactly the three rules the header comment claims.
- All 7 part1 tests executed and passed in the baseline (`orchrun-test-workspace-nofailfast.log.txt`): `two_stream_start_reports_the_track_table` :1669, `strn_title_surfaces_on_the_track` :1666, `zero_rate_or_scale_skips_the_stream_not_the_file` :1674, `negative_biheight_accepts_bottom_up_dib_with_damage` :1641 (plus the two typed-refusal and the filter test; 58 `avi::tests::` total, 0 failed).
- Containment semantics genuinely asserted: a degenerate `strl` (zero scale, unsupported fcc) skips its stream, keeps sibling ordinals aligned (539-543), surfaces a `Recovered` record (545-551), and all-degenerate refuses typed (557-583) — the module docs' ED-e/M6 claim is test-pinned, not prose.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/avi/tests.rs:215 (with 292-293)
Code:
```rust
    chunk(&mut out, b"avih", &avih(0x110, streams.len() as u32));
```
and:
```rust
    if with_idx1 {
        let mut idx = Vec::new();
```
What is wrong: `0x110` = `AVIF_HASINDEX | AVIF_ISINTERLEAVED` is written unconditionally, but `idx1` is only written when `with_idx1` is true. Four part1 tests (`unsupported_stream_types_are_filtered_with_damage`, `zero_rate_or_scale_skips_the_stream_not_the_file`, `all_streams_degenerate_is_typed_no_supported_streams`, `negative_biheight_accepts_bottom_up_dib_with_damage`) all pass `with_idx1=false` — every one of those files declares an index it does not carry. The xtask fixture writer this module claims to mirror (`tools/xtask/src/cmd/fixtures.rs:1618`) writes the same flag but always backs it with an idx1.
Why it matters: the module header (13-17) claims "mirrors tools/xtask fixtures.rs byte-for-byte rules"; for the flags field with idx1 absent it does not. Real-world files with AVIF_HASINDEX and no idx1 are damaged files; the test corpus builds them by accident, so the "flag is never trusted" behavior (asserted at 409-411) is being proven by an unintended fixture shape rather than a deliberate one — a future "fix" of the builder would silently delete that coverage.
What correct looks like: `avih(if with_idx1 { 0x110 } else { 0x100 }, …)`, or a dedicated lying-flags test that names its intent.
Confidence: HIGH — Verification: read (sed-verified quotes; fixtures.rs:1618 compared; all four with_idx1=false call sites listed at 463/531/569/599).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `chunk` | 19-28 | REAL | — | WORD pad rule correct |
| `end_sized` | 30-33 | REAL | — | size excludes the 4 size bytes — correct RIFF semantics |
| `avih` | 35-50 | REAL | — | 56 B, spec field order verified |
| `strh` | 53-74 | REAL | — | 56 B, spec field order verified |
| `strf_video` | 76-87 | REAL | — | 40-byte BITMAPINFOHEADER, signed height |
| `strf_pcm` | 89-91 | REAL | — | tag-1 WAVEFORMATEX delegate |
| `strf_wave` | 95-104 | REAL | — | 16 B; avg-bytes arithmetic realistic |
| `GUID_PCM`/`GUID_IEEE_FLOAT` | 109-114 | REAL | — | KSDATAFORMAT SubFormat GUIDs, byte-exact |
| `strf_extensible` | 120-135 | REAL | — | 18+22 = 40 B extensible form; used by part2+ tests (float/impossible PCM) |
| `Stream` enum | 138-171 | REAL | — | covers video/raw-video/audio/short-strh/other |
| `video_stream` | 173-181 | REAL | — | 30 fps, 3 frames declared |
| `audio_stream` | 183-191 | REAL | — | sample-clock strh convention (scale=1, rate=48000); byte-rate convention covered by header.rs's `TimeBase::new(scale, rate)` which yields the same tick — cross-checked at avi/header.rs:261-268 |
| `MoviChunk`/`BuiltAvi` | 194-201 | REAL | — | records absolute chunk positions for idx1/assertions |
| `build` | 203-309 | REAL | S4 | F-001: flags/idx1 inconsistency |
| `demuxer_over` | 311-319 | REAL | — | `unreachable!` on harness failure — correct loud-test pattern |
| `drain_packets` | 323-334 | REAL | — | panics on unexpected walk error — correct |
| `two_stream_start_reports_the_track_table` | 340-412 | REAL | — | mutation-proof: span ticks, codecs, bases, ordinals, idempotent start, format seam, unknown-id None |
| `strn_title_surfaces_on_the_track` | 414-450 | REAL | — | NUL-terminated strn round-trip pinned |
| `unsupported_stream_types_are_filtered_with_damage` | 452-484 | REAL | — | filter + zero-byte Recovered record pinned |
| `all_streams_unsupported_is_typed` | 486-506 | REAL | — | error code + kind pinned |
| `zero_rate_or_scale_skips_the_stream_not_the_file` | 514-555 | REAL | — | containment + ordinal alignment pinned |
| `all_streams_degenerate_is_typed_no_supported_streams` | 557-583 | REAL | — | code+kind+stage pinned; mild overlap with 486 by design (different degeneracy class) |
| `negative_biheight_accepts_bottom_up_dib_with_damage` | 585-623 | REAL | — | height abs + flip note pinned |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| two_stream_start… | YES | span end tick 3 (declared length) vs movi's 2 chunks distinguishes header-derived span; codec strings, bases, handles all pinned |
| strn_title… | YES | NUL-strip or strn-ignore both break the assert |
| unsupported_stream_types… | YES | filter-abort → `unreachable!` panic; filter-noop → tracks.len()==2 fails; reason string pinned |
| all_streams_unsupported_is_typed | YES | Ok-branch panics; code pinned |
| zero_rate_or_scale… | YES | file-abort → panic; ordinal renumber → handle 1 assert fails |
| all_streams_degenerate… | YES | same shape as above + stage pinned |
| negative_biheight… | YES | rejection → panic; un-flipped height → StreamFormat assert fails |
| Baseline execution | — | all 58 avi tests ok, 0 failed (nofailfast:1617-1674 region) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "mirrors tools/xtask fixtures.rs byte-for-byte rules" | tests.rs:14-16 (module header) | SHALLOW | three listed rules (alignment/sizes/idx1-offsets) verified upheld; the avih flags field is NOT mirrored when `with_idx1=false` (F-001) |
| "in-memory containers built by the same byte-level rules the fixture writer uses… the committed corpus in `tests/` is the end-to-end conformance layer" | tests.rs:1-4 | REAL | layouts verified; conformance layer exists (fixtures generated per `xtask fixtures --check` PASS, baseline INDEX #12) |

## 7. Dead / duplicated / unreachable code
- `Stream::VideoRaw` and `strf_extensible`/GUID consts: declared in part1, exercised only by part2+ tests (e.g. `float_pcm_without_sample_ladder_skips_the_stream`, `impossible_pcm_combinations_skip_the_stream_siblings_survive`, baseline:1628/1635) — used, not dead.
- No dead helpers found in this range.

## 8. Dependencies & flags
- cfg(test)-only module (avi/mod.rs:1271); no feature gates; uses `crate::error`, `header::StreamFormat`/`PcmSample` (public seam, avi/mod.rs:389-398), `test_support::mem_stream`.
- No unsafe, no ignores, no allow(lint) beyond the two documented `lumen-allow-literal` fixture-value markers (432, 449) — those annotate round-trip data, verified benign.

## 9. Open questions for the next agent
- A08 (part2): does any test deliberately build a lying AVIF_HASINDEX file, or is the accidental coverage (F-001) the only one? If deliberate elsewhere, downgrade F-001 to a naming note.
- Whoever audits avi/mod.rs: the `Recovered{skipped:0}` "decision, not bytes" convention (header.rs:386-387) — check the engine does not double-count zero-byte recoveries against the recovery budget.
