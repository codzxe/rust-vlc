# AUDIT: crates/codecs/src/video/mjpeg/parser.rs#part1 (lines 1–524)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 524 audited (chunk part1 of 1050; natural boundary at end of `parse_dht`; part2 = 525–1050 = `parse_dri`, `parse_sos`, `mod tests` — another agent's) / 16 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`lumen decode-bench` → apps/cli/src/pipeline_host.rs:242-245 `"mjpeg" => MjpegDecoder::new()` → crates/codecs/src/video/mjpeg/decoder.rs:546 `parser::parse_into(info, data)?` → this file. EXECUTED: audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt `v_frames=60 v_decoded=60`; avi-firstvertical-open-session.txt full lifecycle; 19/19 `video::mjpeg::parser::tests` ok in baseline test log :721-739) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool 1-530 (chunk + boundary context); `rg -n "parse_into\|JpegInfo"` over mjpeg/ (production caller decoder.rs:546); `rg -n "mjpeg::parser::tests"` baseline test log → 19/19 ok (:721-739); ZIGZAG cross-check vs `tools/xtask/src/cmd/fixtures.rs:887` (byte-identical); fuzz target registry — baseline run #18 `fuzz_registry_check` PASS 35 targets; arithmetic hand-verification of Huffman oversubscription check with 4 worked examples |

## Chunk boundary note
Part1 = 1–524 (module docs, `ZIGZAG`, `ComponentInfo`/`JpegInfo`, `empty`/`reset`, `u16_at`, `marker` consts, `parse` [cfg(test)], `parse_into`, `next_segment`, `is_jfif`, `parse_avi1`, `parse_sof0`, `parse_dqt`, `parse_dht`). Part2 = 525–1050 (`parse_dri`, `parse_sos`, `mod tests` — the tests cover BOTH parts; test bodies belong to part2's owner, I cite only names + baseline results).

## 1. One-paragraph verdict
The header half of the baseline-JPEG marker walker is the best-quality code audited so far in this wave: every index is preceded by a bounds proof I traced line by line, lengths use checked arithmetic, the Huffman table gets a real Annex C code-space (over-subscription) check, the DQT zigzag→natural mapping carries a documented regression history with a test that uses distinct per-position values, and the D-1 scratch-reuse discipline (`reset` keeps capacities) is implemented exactly as documented (`reset` field-parity with `empty` verified). It is reachable from the shipped CLI and executed end-to-end (60/60 AVI MJPEG frames in baseline). The single defect is a strictness inconsistency: `parse_sof0` silently tolerates trailing/short segment bytes where its sibling parsers refuse them typed.

## 2. What actually works (verified)
— End-to-end: baseline `decode-bench` on the AVI MJPEG fixture `v_frames=60 v_decoded=60` (mp4-smoke-bench-avi-mjpeg-pcm.txt) — this file's walk ran 60 times in the shipped binary.
— 19/19 in-file tests ok in baseline (test-workspace.log.txt:721-739), incl. `dqt_dezigzag_maps_scan_order_to_natural_positions`, `oversubscribed_huffman_counts_are_structural`, `segment_overrun_is_truncation`, `two_byte_quant_table_ids_reject_the_upper_half`, `parse_into_reuses_the_scratch_without_leaking_across_walks`.
— `ZIGZAG` byte-identical to the encoder ground truth `tools/xtask/src/cmd/fixtures.rs:887-896` (diffed by eye, both read) — the "shared ground truth" comment is true.
— Huffman over-subscription algorithm verified with worked examples: bits=[0,2]→ok, [0,3]→rejected, [0,2,1]→rejected, [0,0,4]→ok; shift/accumulate arithmetic matches Annex C.2.
— Bounds audit: every `data[i]` indexing site in 1-524 traced to a preceding length proof (details in per-function table); no panics reachable — consistent with the module doc's fuzz guarantee (fuzz registry check PASS, baseline run #18).

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/video/mjpeg/parser.rs:353-415 (`parse_sof0`; contrast 457-459 and 520-522)
Code:
```
    if seg_end < payload + 6 + 3 * count {
        return Err(bad_marker("mjpeg.sof.truncated_components", pos));
    }
```
What is wrong: `parse_sof0` only enforces a MINIMUM segment length. An SOF0 whose declared length exceeds `8 + 3·Nf` (trailing bytes) is accepted silently, and the strict `trailing_bytes` refusal that `parse_dqt` (457-459) and `parse_dht` (520-522) apply to their segments is missing here. Likewise `parse_dqt`/`parse_dht` accept a zero-table segment (declared length 2 → `while p < end` never runs, `p == end` → Ok) without comment.
Why it matters: inconsistent damage policy inside one walker — two parsers refuse trailing bytes typed, the third ignores them; a conformance/fuzzing story built on "structural damage is surfaced" has a silent hole for one marker type. No decode-wrong-bytes risk (the extra bytes are skipped by `next_segment` advancement), hence S4.
What correct looks like: `if seg_end != payload + 6 + 3 * count { return Err(bad_marker("mjpeg.sof.trailing_bytes", pos)); }` (or a documented one-line tolerance comment), and the same decision for empty DQT/DHT segments.
Confidence: HIGH — Verification: read (all three parsers compared; segment-length arithmetic cross-checked against §B.2.2: length = 8 + 3·Nf).

## 4. Per-function table (part1: lines 1–524)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-7 | REAL | — | bounds/no-panic claim read-verified; tolerances list cross-checked (see §6) |
| `ZIGZAG` const | 16-25 | REAL | — | standard §A.6 order; byte-identical to encoder's fixtures.rs:887 |
| `ComponentInfo` | 28-38 | REAL | — | id/h/v/quant_table; range enforcement documented per field |
| `JpegInfo` | 41-72 | REAL | — | huff_bits [17] with slot 0 unused (1..=16 lengths) — consistent |
| `Parsed` (cfg(test)) | 77-80 | REAL | — | test-only wrapper; production uses parse_into — honest cfg |
| `JpegInfo::empty` | 87-106 | REAL | — | quant_tables seeded [[1;64];4] — division-safe default |
| `JpegInfo::reset` | 114-132 | REAL | — | field-by-field parity with empty() verified (13/13 fields); capacities retained (D-1 claim true) |
| `u16_at` | 136-140 | REAL | — | .get + checked_add — no panic; callers' unwrap_or(0) fallbacks proven unreachable (see parse_sof0) |
| `marker` consts | 144-157 | REAL | — | SOI/EOI/SOS/DQT/DHT/DRI/SOF0/APP0/APPn/COM/TEM all standard §B.1 values |
| `parse` (cfg(test)) | 162-167 | REAL | — | stateless test convenience; production path is parse_into (decoder.rs:546) |
| `parse_into` | 188-287 | REAL | — | SOI gate; fill-byte skip (§F.1.2.3) correct incl. trailing-FF truncation; duplicate SOI/SOF guards; EOI-before-SOS refusal; APP0 JFIF/AVI1 dispatch; every branch advances pos or returns — no infinite loop; reserved/non-baseline/unknown marker taxonomy typed |
| `next_segment` | 291-313 | REAL | — | len ≥ 2 enforced; checked_add chain for end; end > len → Truncated — no overflow |
| `is_jfif` | 316-319 | REAL | — | reads past the segment's declared end without a length check, BUT traced: any such read forces the next marker check to fail (data[pos+4]='J'≠0xFF), so the mislabel is unobservable — noted, no finding |
| `parse_avi1` | 330-350 | REAL | — | None/Ok/Err trichotomy documented and implemented; segment-length failure delegated to caller's next_segment (documented); polarity 0..=2 vocabulary enforced |
| `parse_sof0` | 353-415 | REAL | S4 | F-001 (no trailing-byte refusal); precision-8 gate; zero-dim gate; Nf 1..=4; sampling 1..=4 spec-range then 1..=2 subset gate (documented); all indexing bounds-proven (payload+6+3·count ≤ seg_end ≤ len) |
| `parse_dqt` | 419-461 | REAL | — | Pq=0 only (16-bit refused); Tq ≤ 3; 65-byte stride; zero-value refusal (§B.2.4.1); zigzag→natural with documented bug history; trailing-bytes strict; empty segment silently accepted (folded into F-001) |
| `parse_dht` | 464-524 | REAL | — | Tc ≤ 1, Td ≤ 3; total ≤ 256; Annex C over-subscription check (worked-example verified); symbol truncation gate; in-place vals refill (capacity ≤ 256 bounded); trailing-bytes strict; empty segment accepted (F-001) |

## 5. Test honesty
Test bodies live in part2's range (624-1050); names + baseline results audited here, bodies left to part2's owner.
| Test (covers part1 code) | Would it fail if the code were broken? | Reason |
|---|---|---|
| dqt_dezigzag_maps_scan_order_to_natural_positions | YES | distinct per-position values — the exact property that once hid behind flat tables (comment :440-444 records the history) |
| oversubscribed_huffman_counts_are_structural | YES | pins the Annex C check this chunk implements (486-505) |
| segment_overrun_is_truncation / short_input_is_truncation | YES | pins next_segment bounds |
| missing_soi / eoi_before_sos / sos_before_sof / zero_dimensions / sampling_factor_three / progressive_sof2 / two_byte_quant_table_ids / avi1_* (4 tests) / jfif_app0_is_detected | YES | each pins a specific typed error code or flag this chunk emits |
| parses_the_minimal_grayscale_stream / parse_into_reuses_the_scratch_without_leaking_across_walks | YES | happy-path + the D-1 scratch discipline (a leak across walks would fail the second walk's assertions) |
All 19 ok in baseline (test-workspace.log.txt:721-739).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Everything is bounds-checked; no panics on any input (fuzz guarantee)" | parser.rs:5-7 | REAL | every indexing site traced to a length proof (§4); fuzz_codec_mjpeg in registry, check PASS (baseline #18) |
| "leading pad bytes are rejected" | parser.rs:177-180 | REAL | parse_into:195-197 (data[0]/data[1] must be FF D8 exactly) |
| "APPn/COM and unknown reserved segment markers with a length are skipped per §B.2 conventions" | parser.rs:181-182 | REAL | 258-267 (skip via next_segment) — note reserved markers WITHOUT length (0x02..=0xbf) are REFUSED typed (269-273), a stricter-than-documented but defensible reading; the doc sentence covers only the with-length class |
| "A second SOF/SOI, or a missing SOS, is structural" | parser.rs:183 | REAL | 225 (duplicate_soi), 251-252 (duplicate_sof), 226-229 (eoi_before_sos), 203-208 (Truncated at end-of-data without SOS) |
| "On error the scratch may hold a partially walked state — the next call resets it first" | parser.rs:186-187 | REAL | reset() at 198 runs on every entry (after the SOI gate — a no-SOI error leaves the scratch from the previous frame; the next successful call resets before walking: consistent) |
| ZIGZAG "identical table to the 2.A3 encoder … shared ground truth" | parser.rs:12-15 | REAL | byte-diff vs tools/xtask/src/cmd/fixtures.rs:887 — identical |

## 7. Dead / duplicated / unreachable code
`Parsed` + `parse` are `#[cfg(test)]`-gated production-free conveniences — legitimately dead in release builds, honestly gated. No other dead code in the chunk. `is_jfif`'s past-segment read is unreachable-observable (§4 note).

## 8. Dependencies & flags
No external deps; module unconditional (`pub mod mjpeg;` — codecs/src/video/mod.rs:35); no feature gates; no unsafe. The whole mjpeg tree compiles into every shipped build and is the one decoder with end-to-end evidence.

## 9. Open questions for the next agent
- part2 owner: `parse_sos` (553+) — does it enforce exact segment length (`4 + 1 + 2·Ns + 3` bytes) or repeat F-001's lenience?
- decoder.rs owner (W06-A04/A05): `entropy_offset` is set from parse_sos's return — confirm the scan decoder treats bytes past `data.len()` at `entropy_offset` as truncation, not silent empty frame.
- F-001's empty-DQT/DHT acceptance: if a stream relies on tables that were "declared" by a zero-table DQT, the failure should surface at `prepare_tables` (huff_present/quant_present gates) — decoder.rs owner should confirm the refusal is typed there.
