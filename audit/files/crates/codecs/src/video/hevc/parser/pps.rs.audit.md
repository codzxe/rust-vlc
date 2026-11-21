# AUDIT: crates/codecs/src/video/hevc/parser/pps.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 245 / 3 (parse, parse_with_buffer, read) — no other impls, no in-file tests |
| File verdict | UNREACHABLE (real, spec-ordered, real-bytes-test-executed parser; zero shipped entry points — no HEVC decoder lane exists) |
| Reachable from shipped entry point? | NO. `rg "hevc\|h265\|HevcDecoder"` over apps/ → **zero** hevc arms in apps/cli/src/pipeline_host.rs (match arms end at h264/pcm); `HevcDecoder` (the only consumer chain: decode.rs:346 `store_pps` → `PicParameterSet::parse`; hw.rs:107-112 `HevcHwPps`) is constructed only by crates/codecs/tests/hevc_hw_path.rs. apps/cli/src/containers/ts.rs:968 even asserts `Ok(_) => unreachable!("no hevc decoder exists in this build")`. Baseline: sample-hevc.mp4 → Opening → Error → Idle (INDEX.md CORRECTION v2 smoke matrix). Other consumers: hevc/fuzz.rs:52 (fuzz harness), hevc/parser/nal.rs:1462 + slice.rs:460 (in-crate tests), tests/hevc_parser_corpus.rs. Module compiled unconditionally (`pub mod parser;` hevc/mod.rs:41; in-house zero-dep modules ungated per codecs/Cargo.toml:26). |
| Findings | S0: 0 S1: 0 S2: 0* S3: 2 S4: 2 (*plus one S2 inherited from sps.rs F-001 — shared defect, counted there, detailed as my F-001 and NOT re-counted, per wave non-duplication protocol) |
| Commands run | Read tool full file (verbatim authority); `rg -n "PicParameterSet"` + `rg "hevc::parser\|video::hevc"` repo-wide (consumer census); `rg -n "hevc\|h265\|HevcDecoder" apps/cli/src/pipeline_host.rs` (no arm); `sed -n` quote verification pps.rs:96-100,138,147-148,161-162,184-193,200-205; Read vps.rs:232-244 (bounded_ue), nal.rs:798-828 (ue/se guards), sps.rs:494-521 (skip_scaling_list_data loop structure re-verified), slice.rs:447-466 (PPS_0100 seam test), corpus.rs:95-148/255-281/390-441, fuzz.rs:40-60; hand-decode of PPS_0100 `44 01 c0 73 c1 89` (nal.rs:987) field-by-field; baseline greps: test-workspace.log.txt:630-631 (fuzz hostile ok), :636 (corpus_0044 ok), :648 (seam_derivation ok), :1112-1114 (hevc corpus 3/3 ok) |

## 1. One-paragraph verdict
A structurally complete, correctly ordered §7.3.2.3 walk — every base-syntax field is read in spec order, the two range-checked ids match their spec bounds (0..=63, 0..=15), the extension bit is read and honestly refused with a typed `Unsupported` (the exact discipline the h264 twin lacks), and the parse of a real Main-profile PPS (`PPS_0100`, hand-decoded end-to-end including its WPP bit) plus a real RExt lane's PPS executed green in the baseline. It is also production-dead three times over: no shipped entry point has any HEVC lane, the only `Decoder` consumer is test-constructed, and the hardware seam it feeds has no bridges. Defects: the scaling-list call desyncs on any scaling-list PPS (inherited S2, sps.rs F-001), five `se(v)` fields carry no range check despite the doc advertising "out-of-range fields" errors, the tile-grid bounds are hard-coded 0..=30 against the spec's picture-relative limits, and the file has zero direct tests — no test anywhere pins a single PPS error path.

## 2. What actually works (verified)
— Field order and gating re-derived against H.265 §7.3.2.3 line by line (pps.rs:126-205): all 30+ base syntax elements present, in order, with correct if-guards (diff_cu_qp_delta_depth under cu_qp_delta_enabled; tile block under tiles_enabled; beta/tc under control-present && !disabled). Hand-decoded the in-tree real fixture `PPS_0100 = [0x44,0x01,0xc0,0x73,0xc1,0x89]` (nal.rs:987) bit-by-bit: the walk reproduces exactly — pps_id 0, sps_id 0, WPP on (`entropy_coding_sync_enabled` = 1), deblocking control absent, `log2_parallel_merge_level_minus2` 0, extension 0, stop bit last. Verified by Read + manual bit walk.
— Real-bytes execution in baseline: `video::hevc::parser::nal::tests::corpus_0044_parameter_sets_and_slice_prefixes_parse ... ok` (test-workspace.log.txt:636) — the 0044 RExt raw .265 lane asserts VPS+SPS+PPS all parse (unreachable! on failure) and the id lattice holds (`pps.seq_parameter_set_id == sps...`, nal.rs:1479); `slice::tests::seam_derivation_matches_the_0100_sps ... ok` (:648) parses PPS_0100 and asserts three derived seam fields; `hevc_parser_corpus` 3/3 ok (:1112-1114) incl. the 0100 hvcC lane with `PicParameterSet::parse(&cfg.arrays[2].nals[0]).must()` (corpus.rs:423).
— Panic-freedom executed: hevc fuzz harness walks PPS `parse_with_buffer` over hostile/mutated bytes (fuzz.rs:52; nal.rs tests 5000 mutated seeds incl. PPS_0100 at :1391/:1428) — `empty_and_marker_inputs_survive` + `hostile_wide_inputs_survive` ok (test-workspace.log.txt:630-631). Exp-Golomb overflow guarded (`hevc.ue.overflow` >31 leading zeros, nal.rs:802-806), so the `as i32`/`as u8` casts on se/ue results are lossless (magnitude ≤ 2^31−1).
— Honest typed extension refusal: `pps_extension_present_flag` read and refused `Unsupported{"hevc.pps.extension"}` (pps.rs:200-205), declared in the doc (pps.rs:99-100). Buffer reuse contract honored (`unescape_rbsp_into` clears first; nal.rs:327-twin verified for h264, same shape here — corpus reuses one buffer across VPS/SPS/PPS at corpus.rs:273-275).

## 3. Findings

### F-001 [S2 — INHERITED: same defect as sps.rs F-001, counted there; not re-counted in this file's S-tally]
Where: crates/codecs/src/video/hevc/parser/pps.rs:189-193 (call site), sps.rs:497-521 (defect)
Code:
```
        let mut scaling_list_data_present = false;
        if r.bit()? == 1 {
            scaling_list_data_present = true;
            super::sps::skip_scaling_list_data(r)?;
        }
```
What is wrong: `skip_scaling_list_data` walks a phantom second matrix dimension — 112 matrix entries where §7.3.3 defines 20 (I independently re-verified the nested `while j < 6 { while k < 6 { …matrix read… } }` double loop with identical `step` at sps.rs:498-518 before reading the prior audit). For any PPS with `pps_scaling_list_data_present_flag` = 1, the bit reader desynchronizes inside the scaling list; every subsequent read (`lists_modification_present`, `log2_parallel_merge_level_minus2`, extension flag, `finish()`) consumes garbage — the PPS then fails with a wrong typed error or, worse, returns Ok with wrong fields.
Why it matters: scaling-list PPSs are produced by x265 `--scaling-list` and several hardware/broadcast encoders; for this file's consumer chain (the dead hw feed) every such stream is unparseable. Recorded on this file because the call site is here; the S2 was counted once at sps.rs F-001 (their finding already states "shared by the PPS (pps.rs:192), so PPS scaling lists corrupt too").
What correct looks like: single matrixId loop per sizeId (6/6/6/2 = 20 entries) — fix belongs in sps.rs; this file then inherits it.
Confidence: HIGH — Verification: read (both files) + loop-structure re-derivation; cross-ref audit/files/crates/codecs/src/video/hevc/parser/sps.rs.audit.md F-001.

### F-002 [S3]
Where: crates/codecs/src/video/hevc/parser/pps.rs:96-99 (doc claim) vs :138, :147-148, :185 (reads)
Code:
```
    /// Typed [`HevcError`]s: wrong NAL type, header violations,
    /// emulation violations, truncation mid-syntax, out-of-range
    /// fields, `pps_extension_present_flag` set (`Unsupported {
```
```
        let init_qp_minus26 = r.se()? as i32;
        let cb_qp_offset = r.se()? as i32;
        let cr_qp_offset = r.se()? as i32;
```
```
                deblocking_offsets = Some((r.se()? as i32, r.se()? as i32));
```
What is wrong: the `# Errors` contract advertises "out-of-range fields", but all five `se(v)` reads are unbounded: `pps_cb_qp_offset`/`pps_cr_qp_offset` (§7.4.3.2.1 hard bounds −12..=12) and `pps_beta_offset_div2`/`pps_tc_offset_div2` (−12..=12) accept any i32-range se value; `init_qp_minus26` (−(26+QpBdOffsetY)..=+(25+QpBdOffsetY)) is deferred with a doc note but no downstream check exists in this crate. Additionally `diff_cu_qp_delta_depth` is bounded at 4 (:143) where the conformance maximum is `CtbLog2SizeY − MinCbLog2SizeY` ≤ 3 — one impossible value accepted.
Why it matters: the only bounded fields are the ue(v) ones; a non-conforming PPS with cb_qp_offset +500 parses Ok and (in the intended hw-feed consumer) would be forwarded to a session verbatim. The declared validation contract is half-true.
What correct looks like: a `bounded_se` twin of `bounded_ue` (vps.rs:232) applied to the four ±12 fields; init_qp either bounded per-bit-depth or the doc drops "out-of-range fields" for it; diff depth max 3.
Confidence: HIGH for code facts (read); spec ranges from §7.4.3.2.1 domain knowledge (no spec in tree).
Verification: read; UNVERIFIED-executed (no test exercises an out-of-range se field — see F-004).

### F-003 [S3]
Where: crates/codecs/src/video/hevc/parser/pps.rs:161-162
Code:
```
            num_tile_columns_minus1 = bounded_ue(r, "num_tile_columns_minus1", 30)?;
            num_tile_rows_minus1 = bounded_ue(r, "num_tile_rows_minus1", 30)?;
```
What is wrong: both tile-grid axes are hard-capped at 0..=30 with no comment and nothing in-tree deriving 30. The H.265 conformance bounds are picture-relative — `num_tile_columns_minus1 ≤ PicWidthInCtbsY − 1`, `num_tile_rows_minus1 ≤ PicHeightInCtbsY − 1` — so any picture with ≥32 CTB columns/rows can legally carry values above 30 (1920px at CTB 32 → 60 columns; 4K tiled encodes; portrait streams with ≥32 CTB rows at CTB 32 → e.g. 1080 tall/32 = 34 rows). Such conforming streams get a typed `OutOfRange` refusal here.
Why it matters: a wrong refusal on a spec-legal (if uncommon) input class, with an error message that names a bound the spec does not have. Blast radius is small today (consumers dead; encoders rarely emit >31 tile columns), so S3 not S2 — but if the bound came from misremembering VVC's fixed tile-column cap, it is simply the wrong spec generation.
What correct looks like: bound against the active SPS geometry (the parser already has SPS context upstream via `pps_seq_parameter_set_id`), or at minimum widen to a defensive absolute (e.g. 127) and document why.
Confidence: MEDIUM — code fact HIGH (read); the spec bound text is from domain knowledge and CANNOT be verified from this tree (no spec copy) — flagged UNVERIFIED-spec. If the orchestrator has an H.265 copy, §7.4.3.2.1 settles it.

### F-004 [S4]
Where: crates/codecs/src/video/hevc/parser/pps.rs (whole file — zero `#[cfg(test)]`), vs h264/parser/pps.rs:172-374
Code:
```
        let pps_extension_present_flag = r.bit()?;
        if pps_extension_present_flag == 1 {
            return Err(HevcError::Unsupported {
                code: "hevc.pps.extension",
            });
        }
```
What is wrong: no test anywhere pins a single behavior of this file's error surface or fields: `rg "hevc.pps"` over crates/ returns only the three production error-string literals — `wrong_nal_type`, `extension`, and decode.rs:346's wrapper are never asserted by any test; the only field-level pins are `pps.seq_parameter_set_id` (nal.rs:1479, corpus.rs:427) and the three seam fields (slice.rs:462-465). The h264 twin ships three unit tests including exact-`Err` equality; this file ships none.
Why it matters: the extension refusal, the wrong-NAL gate, the out-of-range paths (F-002/F-003), and 30+ parsed fields have no regression net — a mutation deleting the extension check or swapping cb/cr reads would pass every test in the repo (fuzz-lite only proves no-panic).
What correct looks like: port the h264 twin's test shape (crafted full-field fixture + typed-error table) — the harness patterns already exist in this crate.
Confidence: HIGH — Verification: rg (no test references) + baseline log census.

### F-005 [S4]
Where: crates/codecs/src/video/hevc/parser/pps.rs:28-31, 41-42, 59-62, 85-86
Code:
```
    /// `num_ref_idx_l0_default_active_minus1` (0..=15).
    pub num_ref_idx_l0_default_active_minus1: u64,
```
What is wrong: width indiscipline in the public struct: five fields bounded ≤4/≤15/≤30 are stored as `u64` while flag-sized neighbors use `bool` and offsets use `i32` — the h264 twin stores the same-minus1 fields as `u8`. Six 8-byte fields in a struct the (dead) hw lane clones per parameter-set activation (hw.rs:107-112, decode.rs:353-358).
Why it matters: API-noise and struct bloat on a public type; readers cannot tell which fields are bounded-tiny without reading the parser.
What correct looks like: `u8` for the ≤15/≤30 fields, `u8`/`u32` for depth — matching the twin.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PicParameterSet::parse` | 101-104 | REAL (test-executed) | — | thin allocating wrapper (corpus.rs:423 `.must()` executed in baseline) |
| `PicParameterSet::parse_with_buffer` | 111-123 | REAL | — | NAL-type gate (:113-117), unescape into cleared caller buffer, `finish()` trailing gate; executed via fuzz + corpus reuse |
| `PicParameterSet::read` | 126-244 | REAL | S2-inherited (F-001), S3 (F-002, F-003) | field order re-derived exact vs §7.3.2.3 incl. PPS_0100 hand-decode; scaling-list call desyncs (F-001); se fields unbounded (F-002); tile caps 30 (F-003); extension refusal typed+honest (:200-205) |

## 5. Test honesty
No tests in this file. External evidence:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| nal.rs `corpus_0044_parameter_sets_and_slice_prefixes_parse` (baseline :636 ok) | YES for "0044-shape" breakage | unreachable! if PPS is None; id-lattice + 64 slice-prefix parses each unwrap_or_else(unreachable!) — a field desync AFTER the sets (slice prefixes) trips it; but it cannot see scaling-list/tile/extension shapes (0044 has none) |
| slice.rs `seam_derivation_matches_the_0100_sps` (:648 ok) | PARTIAL | pins parse-success + 3 seam fields of PPS_0100 only; any other field may mutate silently |
| corpus `corpus_mp4_0100_hvcc_field_assertions_via_ffprobe` (:1113 ok) | PARTIAL | `.must()` on parse + ONE field (seq id); annexb lanes use `.ok()` tolerance (corpus.rs:131) — a PPS-rejecting regression on those lanes degrades to "no slice" silently |
| fuzz-lite / hevc::fuzz (:630-631 ok) | NO (by design) | panic-freedom only; `let _ =` results |
| NO test pins `hevc.pps.wrong_nal_type`, `hevc.pps.extension`, any OutOfRange path, or 30+ fields | — | F-004 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "clean-room parse of the card's field set: ids, tile-grid basics, loop-filter bits, init QP; ref-list defaults + deblocking/entropy fine print retained" | pps.rs:1-5 | REAL | every listed field parsed, order exact (§2, PPS_0100 walk) |
| `# Errors`: "wrong NAL type … out-of-range fields … pps_extension_present_flag set (Unsupported) … bad trailing bits" | pps.rs:96-100 | SHALLOW | wrong-NAL ✓ (:113-117), extension ✓ (:200-205), trailing ✓ (finish) — but "out-of-range fields" is untrue for all five se(v) fields (F-002) |
| "no software HEVC decode here … routes the parsed result to a hardware session" | hevc/mod.rs:9-20 (module docs, ADR-001) | REAL | this file feeds only the hw seam (hw.rs:107-112), which has zero shipped callers (header chain) |
| HEVC matrix row: software decode `blocked`/`hw-only` | docs/feature-matrix.toml (hevc rows, per decode.rs audit) | REAL | consistent with the no-lane trace + baseline sample-hevc Error session |

## 7. Dead / duplicated / unreachable code
- Whole file UNREACHABLE from shipped entry points (header chain; ts.rs:968 asserts the absence of the lane).
- Inherited shared defect at :192 (F-001) — defect body lives in sps.rs (counted there).
- Structural mirror of h264/parser/pps.rs (parse/parse_with_buffer/read + bounded reads) — deliberate twin, documented in mod docs; the divergences are instructive: hevc reads its extension bit honestly where h264 misreports the tail (h264 audit F-001), and h264 bounds its se fields where hevc does not (this file F-002).

## 8. Dependencies & flags
No Cargo deps; `use super::nal::{…}` + `use super::vps::bounded_ue` (pub(crate) helper, vps.rs:232-244). Module unconditional (hevc/mod.rs:41; codecs/Cargo.toml:26 keeps in-house zero-dep codecs ungated) — dead weight compiled into every shipped binary.

## 9. Open questions for the next agent
- F-003 spec verification: an H.265 §7.4.3.2.1 copy settles whether the tile bounds are picture-relative (my belief) — REQUEST-ORCH-RUN noted in log.
- If any wave owns hevc/hw.rs consumers: does any session branch on `tiles_enabled`/`num_tile_*` (the fields this parse retains but nothing provably reads)?
- F-004's test port is mechanical; whoever next touches this crate's tests should port the h264 twin's three-test shape.
