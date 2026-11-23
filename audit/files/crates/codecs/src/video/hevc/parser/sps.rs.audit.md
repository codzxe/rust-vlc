# AUDIT: crates/codecs/src/video/hevc/parser/sps.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 604 / 13 (+3 structs) — zero in-file tests |
| File verdict | REAL parser with one genuine mis-walk (inter-RPS NumDeltaPocs) and a SAR table hole; UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (hevc module unwired — see nal.rs audit F-003; demux/media-probe do their own parsing, no codecs dep) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | sed/awk/od (quote + byte verification); rg (callers, ASPECT_TABLE, tests); prebuilt `codecs-bae0f1756cc0a809` suite covers this file via nal fuzz-lite + slice seam test (baseline cargo-test-workspace.txt:745) |

## 1. One-paragraph verdict
A careful clean-room SPS syntax walk with typed bounds on every ue field,
checked conformance-window arithmetic, and honest typed refusals for
`sps_extension` — I verified the byte-exactness of the quoted regions and
the arithmetic of `cropped_size`/`pic_size_in_ctbs_y` (all checked ops, no
overflow path found). Two real defects: the inter-RPS predicted-set walk
returns the WRONG `NumDeltaPocs` (the reference's count, not ref+1−drops),
which desynchronizes the bit position of every later `st_ref_pic_set` on
streams that use inter-RPS prediction at the SPS level — contradicting the
module doc's "skipped correctly (bit-accurate walk)" claim; and
`ASPECT_TABLE` covers only idc 1..4, silently discarding the SAR for
idc 5..254 (40:33 et al.), contradicting its own field doc. No in-file
tests; field-exactness evidence lives in corpus-gated integration tests
that skip in this environment, plus the pinned-0100 seam test which does
run.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:745
`seam_derivation_matches_the_0100_sps ... ok` (parses the real pinned x265
SPS: poc-lsb width 8, 10 slice-address bits — cross-checked facts) and :807
`fuzz_lite_no_panics_on_random_and_mutated_bytes ... ok` (SPS::parse on
10k random + 5k mutated inputs, no panics). `cropped_size`/`pic_size_in_ctbs_y`
checked-arithmetic verified by read (u32 bounds: conf_win ≤ 65535 each,
pic ≤ 65535, products ≤ 131070 — no overflow possible).

## 3. Findings

F-001 [S2]
Where: crates/codecs/src/video/hevc/parser/sps.rs:471-478 (and loop :465)
Code:
```rust
        // NumDeltaPocs for a predicted set keeps the reference's
        // count minus the unset deltas — the exact value needs the
        // use_delta bookkeeping we deliberately do not retain; the
        // walk itself consumed the right bits either way. Returning
        // the reference's count keeps subsequent set references in
        // range for the walk (worst case it over-reads flags, all
        // bounds-checked).
        Ok(ref_deltas)
```
What is wrong: per §7.3.8, `NumDeltaPocs[stRpsIdx]` for a predicted set is
`NumDeltaPocs[RefRpsIdx] + 1 − (number of unused, non-delta entries)`. The
code returns `ref_deltas` — the reference's count — unconditionally: not
ref+1 even when every picture is used, and not decremented for drops. The
next set that predicts from THIS one loops `for _ in 0..=ref_deltas`
(:465) with the wrong count, reading the wrong number of
used/use_delta flag pairs, so the bit position after that set is wrong and
every subsequent SPS field mis-parses (typed Truncated/finish errors on
valid streams, or silently wrong retained fields). The comment's claim
"the walk itself consumed the right bits either way" is only true for the
current set, not for the dependent sets the wrong count poisons.
Why it matters: the module doc (:6-7) promises scaling lists / RPS are
"*skipped correctly* (bit-accurate walk)" — false for any SPS-level
inter-RPS prediction with drops or additions (HM/VTM-style encodes).
What correct looks like: track used/use_delta in the loop, return
ref + 1 − drops.
Confidence: HIGH for the count being wrong (spec arithmetic, read);
MEDIUM that real-world streams hit it at the SPS level (x265 writes mostly
explicit sets; reference-software encodes do not) — Verification: read,
UNVERIFIED by execution (no fixture with inter-RPS prediction).
Currently dead code from shipped entry points, which keeps this at S2 and
not higher.

F-002 [S3]
Where: crates/codecs/src/video/hevc/parser/sps.rs:107-110 and :526-537
Code:
```rust
/// Aspect-ratio idc table (Annex D, Table D-1) for the idcs the
/// corpus carries; unlisted idcs resolve through the explicit
/// Extended_SAR — never a guess.
const ASPECT_TABLE: [(u8, u16, u16); 4] = [(1, 1, 1), (2, 12, 11), (3, 10, 11), (4, 16, 11)];
```
What is wrong: Table D-1 defines idcs 1..=16 (and more in later editions);
this table carries 1..=4 only. `read_vui` (:528-537) reads Extended_SAR
fields only when `idc == 255`; for idc 5..254 the parsed 8-bit idc is
discarded and `sar` stays `None`. The struct doc (:89-91) claims "unlisted
idcs fall back to the explicit Extended_SAR, idc 255, via `sar`" — false
for 5..254: nothing is read, the value is dropped.
Why it matters: valid streams with `aspect_ratio_idc` 5 (40:33 — standard
16:9 SD/ED, common in broadcast HEVC) through 16 lose their SAR silently;
the parser reports "no SAR signalled".
What correct looks like: the full Table D-1 (idc 1..=16), idc 255 →
Extended_SAR, and 17..254 genuinely unspecified.
Confidence: HIGH — Verification: read (both branches traced).
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/codecs/src/video/hevc/parser/sps.rs:400-403
Code:
```rust
    /// `CtbLog2SizeY` — the log2 CTB size (6..=7 for the 64/32 CTBs
    /// the corpus's x265 uses; the range over all legal fields is
    /// 3+3+0 .. 3+3+3 = 6..=9? no: min 8 → log2 3, max 64 → log2 6;
    /// the fields cap it inside 6..=9... computed, not asserted).
```
What is wrong: the doc comment argues with itself on the record
("= 6..=9? no: ... 6..=9..."), and neither the doc nor the code validates
the spec's CTB-size range: the parsed fields admit CtbLog2SizeY 3..=9
(8..512 px), while H.265 constrains CTB size to 16/32/64 (log2 4..=6).
Out-of-spec geometry is accepted and only caught if `pic_size_in_ctbs_y`
overflows u32 (impossible: ≤ 65535² / 8² fits).
Confidence: HIGH — Verification: read + arithmetic.

F-004 [S4]
Where: crates/codecs/src/video/hevc/parser/sps.rs (whole file, no `#[cfg(test)]`)
What is wrong: zero unit tests in a 604-line parser. The field-exactness
evidence lives in tests/hevc_parser_corpus.rs, whose three corpus rows
print "SKIP ... corpus-v4 absent" in this environment (rg: :214, :286,
:334) and return Ok — so here the only executed SPS-field assertions are
the pinned 0100-SPS seam facts (poc width 8, address bits 10) in
slice.rs:448-466 and the panic-freedom fuzz. `cropped_size`'s underflow
branch, the SAR table, and `pix_fmt_name` have no executing test in this
environment.
Confidence: HIGH — Verification: rg + baseline log.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ChromaFormat::from_idc/idc | 29-48 | REAL | - | total, round-trips |
| ChromaFormat::sub_size | 54-61 | REAL | - | Table 6-1 correct incl. mono (1,1) |
| ChromaFormat::pix_fmt_name | 66-80 | REAL | - | ffprobe spelling; no executing test here |
| SeqParameterSet::parse | 194-197 | REAL | - | allocating wrapper |
| parse_with_buffer | 204-216 | REAL | - | NAL-type gate + unescape + finish() |
| SeqParameterSet::read | 219-373 | REAL | - | typed bounds on every ue; extension → Unsupported :336-340 |
| cropped_size | 383-398 | REAL | - | checked mul/sub, w>0/h>0, underflow typed |
| ctb_log2_size_y | 405-409 | REAL | S4 | F-003 no spec-range validation |
| pic_size_in_ctbs_y | 418-428 | REAL | - | u64 div_ceil + checked mul + u32 try_from |
| skip_st_ref_pic_set | 435-492 | REAL (buggy) | S2 | F-001 predicted-set count wrong |
| skip_scaling_list_data | 497-521 | REAL | - | coefNum = min(64, 1<<(4+2i)) ✓, dc for i>1 ✓, sizeId-3 step ✓ |
| read_vui | 524-604 | REAL | S3 | F-002 SAR table hole; rest of walk (bitstream_restriction, HRD) traced correct |

## 5. Test honesty
No in-file tests. See F-004; corpus integration tests skip silently in
this environment (verified: corpus dir ABSENT, eprintln+return pattern at
hevc_parser_corpus.rs:214/286/334).
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling lists, short-term RPS ... *skipped correctly* (bit-accurate walk)" | sps.rs:6-7 | FALSE for inter-RPS predicted sets | F-001 |
| "unlisted idcs fall back to the explicit Extended_SAR, idc 255" | sps.rs:89-91 | FALSE for idc 5..254 | F-002 |
| "nothing here decodes" | sps.rs:7-8 | REAL | no decode code in file |
## 7. Dead / duplicated / unreachable code
Unreachable from shipped entry points (module unwired). No internal dead
code found; all fields are consumed by decode.rs/slice.rs seams.
## 8. Dependencies & flags
Depends only on sibling nal.rs/vps.rs helpers (bounded_ue, skip_hrd,
ProfileTierLevel) — verified present and total (vps.rs:176-246).
## 9. Open questions for the next agent
- Craft an SPS with 2+ st_ref_pic_sets using inter_prediction to prove
  F-001's bit-drift end-to-end (blocked here: no corpus, no build budget).
- vps.rs (unassigned to me): does ProfileTierLevel::read bound the
  reserved bits it must ignore?
