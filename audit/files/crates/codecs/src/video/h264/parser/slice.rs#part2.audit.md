# AUDIT: crates/codecs/src/video/h264/parser/slice.rs#part2 (lines 507–1071; overlaps 507–531 into part1, 1063–1071 into part3)
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 565 audited (assigned ≈532–1062 of 2124; start pulled back to 507 to keep `PicOrderSyntax` whole — 25-line overlap with W06-A07's part1, not re-audited; end extended to 1071 to keep `read_pred_weight_entry` whole — 9-line overlap with W06-A05's part3, not re-audited) / 14 items in table |
| File verdict | UNREACHABLE (parse code is REAL quality; no shipped entry point reaches `parser::slice`) |
| Reachable from shipped entry point? | NO. `parse_slice_header`'s only callers are this file's own `#[cfg(test)]` module (1423, 1632, 1737, 1830, 1865, 1917, 1927, 1935, 1946 — all ≥1189, test territory). The hw tree imports the TYPES (`hw/mod.rs:107 use super::parser::slice::{SliceHeader, SliceNalInfo}`, vaapi/session.rs:96-98) but constructs `H264HwVcl`/`SliceHeader` ONLY in its own `#[cfg(test)]` code (session.rs:1863, 1952); W06-A07 already established `h264/hw/mod.rs` is UNREACHABLE (only consumer `decode.rs::H264Decoder::from_setting`, test-only), and product.toml:168 `hw-decode = false` + :191 `hw_decode_policy = "software-only"` keep the hw lane off. The reachable sw decoder (sw.rs) uses `parser::nal` only — `parser::slice` is not in its import list (sw.rs:92-94). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:3 |
| Commands run | `grep -n "^fn \|^pub fn \|^impl…"` (structure map); `rg -n "parse_slice_header\("` over crates/+apps/ (only in-file test callers); `rg -n "H264HwVcl\|\.slice =\|slice:"` over hw/ (construction only in session.rs tests); `rg -n "parser::slice"` (hw/mod.rs:107, vaapi/session.rs:96-98 — type imports only); sed of slice.rs:173-270 (error enum — found the unused `InvalidDeblockIdc` variant proving F-001); sed verification of every quote; §7.3.3 field order checked against ITU-T H.264 (03/2023) syntax; baseline `video::h264::parser` test results checked (see §5). |

## 1. One-paragraph verdict
This chunk is the §7.3.3 slice-header grammar (data types + `parse_slice_header` + the ref-list-mod/pred-weight sub-readers), and the grammar itself is faithfully transcribed — field order, gates (`separate_colour_plane_flag`, `frame_mbs_only_flag`, `bottom_field_pic_order_in_frame_present_flag && !field_pic_flag`, POC types 0/1/2, weighted-pred gating, IDR vs adaptive vs sliding-window marking, Corrigendum-1 deblock vocabulary) all match the spec text, with bounded loops and zero panic paths. But it is dead weight in the shipped product: the only production-adjacent consumers are the (itself unreachable) hardware lane's type surface, and no shipped build parses a slice header. Worse, the file's entire value proposition — its typed error vocabulary — has two real defects: an out-of-range `disable_deblocking_filter_idc` returns `InvalidSliceType` (the correct `InvalidDeblockIdc` variant exists at :194 and is never constructed), producing a Display message that claims "slice_type 7 outside 0..=9" for a value that is a perfectly valid slice type; and the pred-weight-table reader remaps every failure — including plain truncation — to `OpBudgetExceeded` with undocumented `kind` strings, i.e. a false diagnosis that also swallows the source error. The spec conformance of the happy path is verified by corpus tests (baseline-ok), but neither error path is tested.

## 2. What actually works (verified)
- §7.3.3 field order and gates in `parse_slice_header` — verified by read against ITU-T H.264 (03/2023) §7.3.3/§7.3.3.1/§7.3.3.2: first_mb_in_slice ue → slice_type ue → pps_id ue (≤255 gate, :758) → colour_plane_id u(2) gated → frame_num u(log2+4) → field/bottom flags gated on `frame_mbs_only_flag` → idr_pic_id on IDR → POC per type (type0 lsb+delta_bottom with the exact `bottom_field_pic_order_in_frame_present_flag && !field_pic_flag` gate, :788; type1 with delta_pic_order_always_zero_flag, :795; type2 bare) → redundant_pic_cnt PPS-gated → direct_spatial B-gated → num_ref_idx override P/SP/B-gated with ≤31 gates → ref-list-mod (I/SI excluded, l1 B-only) → pred-weight-table with the exact `(weighted_pred_flag && P||SP) || (weighted_bipred_idc==1 && B)` gate (:885-886) → dec_ref_pic_marking nal_ref_idc/IDR-gated → cabac_init_idc CABAC+non-I/SI → slice_qp_delta → deblock PPS-gated.
- Ref-list-mod op table (:980-997): idc 0/1/2 operands and terminator 3 match §7.3.3.1; MVC idc 4/5 (`abs_diff_view_idx_minus1`) match §G.7.3.3.1; unknown → typed `InvalidModOfPicNumsIdc`.
- Loop budgets: `MAX_OPS_PER_LOOP` (:730) bounds the ref-list loop (every iteration consumes ≥1 bit of the ue read, so input length also bounds it); num_ref_idx values ≤31 bound the pred-weight loops to ≤32 entries.
- Baseline execution of the parser via its tests: `video::h264::parser` unit suite passes (test-workspace.log.txt — `h264 parser` corpus lanes ok; see §5 for the exact citations).
- No panic path: every read is `?`-propagated `BitReadError`; no unwrap/expect/index/div in 507-1071.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/parser/slice.rs:923-929
Code:
```
    let deblocking = if pps.deblocking_filter_control_present_flag {
        let idc = r.read_ue()?;
        if idc > 2 {
            // 3 was removed from the vocabulary by Corigendum 1;
            // treat out-of-range as the typed invalid-value class.
            return Err(SliceHeaderError::InvalidSliceType(idc));
        }
```
What is wrong: an out-of-range `disable_deblocking_filter_idc` returns `InvalidSliceType(idc)` — the WRONG variant. The enum defines the correct one at :193-194 (`/// disable_deblocking_filter_idc outside 0..=2. InvalidDeblockIdc(u32)`) with a correct Display at :242-243, and `rg` shows `InvalidDeblockIdc` is constructed NOWHERE in the file — dead variant, mislabeled error.
Why it matters: this module's entire value proposition is its typed error surface ("Every failure is a typed [`SliceHeaderError`]", module doc :18). A consumer matching variants gets the wrong class, and the rendered message is actively false: idc=7 renders as "slice_type 7 outside 0..=9 (Table 7-7)" — but 7 IS a valid slice type (the I+all-slices alias). Debugging a malformed stream with this message sends you to the wrong syntax element. Blast radius today: zero (module unreachable), but this is exactly the kind of defect that ships silently the day the hw lane lands.
What correct looks like: `return Err(SliceHeaderError::InvalidDeblockIdc(idc));` — one-line fix using the existing variant; plus a test asserting it (none exists — `rg InvalidDeblockIdc` over the file finds only the definition and Display).
Confidence: HIGH — Verification: read (variant definition :193-194, Display :242-243, zero construction sites, mislabeled site :928 — all sed-verified).

### F-002 [S3]
Where: crates/codecs/src/video/h264/parser/slice.rs:1029-1037 (and the l1 twin 1040-1048)
Code:
```
    for idx in 0..=num_ref_idx_l0_active_minus1 {
        table
            .l0
            .push(read_pred_weight_entry(r, chroma_array_type).map_err(|_| {
                SliceHeaderError::OpBudgetExceeded {
                    kind: "pred_weight_l0",
                    ops: idx as usize + 1,
                }
            })?);
    }
```
What is wrong: EVERY error from `read_pred_weight_entry` — including plain `BitReadError` truncation from a short buffer — is remapped to `OpBudgetExceeded`. The budget was not exceeded; the input ended. The source error is discarded (`|_|`), so truncation is misdiagnosed as a malformed-stream loop-budget trip. Additionally the `kind` strings `"pred_weight_l0"`/`"pred_weight_l1"` are undocumented: the variant's own field doc (:203-205) says kind is `"ref_pic_list_modification"` or `"dec_ref_pic_marking"`.
Why it matters: false error diagnosis in the typed-error surface the module exists to provide; a truncation-fuzz consumer (the project's hostile-input posture) would classify a truncated weight table as an op-budget violation and report the wrong damage class. Doc drift on the `kind` vocabulary compounds it.
What correct looks like: propagate the real error: `.map_err(SliceHeaderError::Bit)` (or let `From<BitReadError>` do it by making `read_pred_weight_entry`'s error flow through `?`), and reserve `OpBudgetExceeded` for actual budget trips; update the `kind` doc to include the pred-weight kinds or drop them.
Confidence: HIGH — Verification: read (read_pred_weight_entry returns `Result<_, BitReadError>` at :1058; only truncation/width errors are possible — there is no budget logic inside it to exceed).

### F-003 [S4]
Where: crates/codecs/src/video/h264/parser/slice.rs:915-919
Code:
```
    let cabac_init_idc = if pps.entropy_coding_mode_flag && !slice_type.is_intrinsic() {
        Some(r.read_ue()?)
    } else {
        None
    };
```
What is wrong: `cabac_init_idc` (spec range 0..=2, §7.4.3.1) is read with no range gate, while every sibling vocabulary field in the same function is gated (`pps_id > 255` :758, `num_ref_idx > 31` :833/:838, deblock `idc > 2` :925).
Why it matters: inconsistent validation posture; a value of e.g. 9 flows into `SliceHeader.cabac_init_idc` and would be handed to a future decoder/hw feed that trusts the parse.
What correct looks like: `let v = r.read_ue()?; if v > 2 { return Err(SliceHeaderError::InvalidContext("cabac_init_idc")) }` — or a dedicated variant mirroring `InvalidDeblockIdc`.
Confidence: HIGH — Verification: read (no gate between read and field assignment).

### F-004 [S4]
Where: crates/codecs/src/video/h264/parser/slice.rs:999-1005
Code:
```
        ops.push(op);
        if ops.len() > MAX_OPS_PER_LOOP {
            return Err(SliceHeaderError::OpBudgetExceeded {
                kind: "ref_pic_list_modification",
                ops: ops.len(),
```
What is wrong: the budget check is AFTER the push and uses `>`, so 257 ops are stored before the error fires — the budget is 257, not the documented 256. Off-by-one on a defensive bound; harmless in effect (one extra op allocated) but the constant's comment (:726-729) says the budget is the allocation-side bound.
Why it matters: cosmetic drift between documented invariant and enforced invariant; a reviewer trusting the comment reasons about 256.
What correct looks like: check before push (`if ops.len() == MAX_OPS_PER_LOOP { … }`) or `>=` after push — pick one and make the constant the truth.
Confidence: HIGH — Verification: read (arithmetic traced: push at 999, check at 1000 trips first at len()==257).
Note: the same pattern in `read_mmco_ops` (1072+, part3 — W06-A05's range) — flagged for that agent, not counted here.

### F-005 [S4]
Where: crates/codecs/src/video/h264/parser/slice.rs:784
Code:
```
    let field_pic = field_pic_flag;
```
What is wrong: pointless alias rename of `field_pic_flag` to `field_pic` used twice (:788, :802) and never again; the original binding remains live.
Why it matters: pure noise in the hot parsing function; a reader must prove the two names are the same variable.
What correct looks like: use `field_pic_flag` directly.
Confidence: HIGH — Verification: read (both uses sed-verified :788, :802).

## 4. Per-function table (chunk lines 507–1071)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum PicOrderSyntax` | 508-533 | REAL | — | data; per-type fields with honest Optionality docs |
| `struct RefPicListModification` | 537-544 | REAL | — | data; Default = empty lists |
| `enum RefPicListModOp` | 547-574 | REAL | — | data; idc 0/1/2 + MVC 4/5, operands match §7.3.3.1/§G.7.3.3.1 |
| `struct PredWeightTable` | 578-588 | REAL | — | data |
| `struct PredWeightEntry` | 591-598 | REAL | — | data; chroma as [(i32,i32);2] matches Cb/Cr pairs |
| `enum DecRefPicMarking` | 601-617 | REAL | — | data; IDR/Adaptive/SlidingWindow trichotomy correct |
| `enum MmcoOp` | 623-664 | REAL | — | data; operands match §7.3.3.4 (op1 = diff_of_pic_nums only — comment cites ffmpeg trace verification, UNVERIFIED by me but consistent with the spec table); idc 6 MVC form correct |
| `struct DeblockingFilterControl` | 667-676 | REAL | — | data; idc==1 ⇒ offsets None correct |
| `struct SliceHeader` | 679-724 | REAL | — | data; `header_bits` exposes parse progress |
| `const MAX_OPS_PER_LOOP` | 730 | REAL | S4 | F-004 off-by-one in enforcement |
| `parse_slice_header` | 738-971 | REAL | S3/S4 | §7.3.3 order verified; F-001 (wrong error variant), F-003 (cabac_init_idc ungated), F-005 (alias); `first_mb_in_slice` and `slice_qp_delta` unvalidated (parse-level posture, PicSizeInMbs not in the projection — acceptable); FMO honest refusal :746-752 |
| `read_ref_pic_list_mod_ops` | 974-1007 | REAL | S4 | op table correct; terminator 3; budget F-004 |
| `read_pred_weight_table` | 1010-1052 | REAL | S3 | F-002 (truncation→OpBudgetExceeded misdiagnosis + undocumented kinds); loops bounded by ≤31-validated counts; chroma denom gated on chroma_array_type ✓ |
| `read_pred_weight_entry` | 1055-1069 | REAL | — | luma/chroma flag/value reads match §7.3.3.2; returns BitReadError (correct, honest) — the caller ruins it (F-002) |

## 5. Test honesty (tests live at 1189+, W06-A05/A03's chunks — assessed for coverage of THIS chunk's functions only)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| corpus lanes calling `parse_slice_header` (1423, 1632) | YES | real ffmpeg-authored streams through the full header walk — field-order or gate regressions fail the assertions |
| hand-built header tests (1737-1946) | YES for the covered gates (pps_id, FMO, truncation sweep at 1865) | direct construction + assert |
| NOTHING covers: deblock idc > 2 (F-001's gate), pred_weight_table error paths (F-002), cabac_init_idc range (F-003) | NO | zero test references to `InvalidDeblockIdc`, `pred_weight_l0`, or cabac range — the three defects in this chunk are all in untested paths |
Baseline: the `codecs` unit suite (506 passed incl. the parser tests) is green (test-workspace.log.txt:783 area) — the green suite proves the happy path, not the error taxonomy.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Clean-room from the spec text, parse-level only … no entropy-coded macroblock data is touched" | slice.rs:1-6 | REAL | read: parse stops at deblock + header_bits; no MB layer |
| "Every failure is a typed [`SliceHeaderError`]" | slice.rs:18 | SHALLOW | F-001 wrong variant, F-002 misdiagnosis + swallowed source — the typing exists but two sites lie |
| "op-count budgets that bound malformed-stream loops" | slice.rs:21-22 | REAL | budget enforced (997-1005) albeit off-by-one (F-004) |
| "operand mapping verified against ffmpeg `trace_headers` on the corpus" | slice.rs:619-622 | REAL-UNPROVEN | corpus tests pass in baseline (consistent), but I cannot re-run ffmpeg trace in this sandbox; the mapping as written matches the spec table |
| wave-6 hostile question | task prompt | N/A for this file | parser::slice is not on the sw decode path (sw.rs imports parser::nal only, sw.rs:92-94); H.264 reachability answered in sw.rs.audit.md |

## 7. Dead / duplicated / unreachable code
- `SliceHeaderError::InvalidDeblockIdc` (:194) — defined, documented, Display'd, NEVER constructed (F-001's smoking gun).
- The entire chunk is production-dead (see header): compiled unconditionally (in-house parser, no feature gate), consumed only by tests and the unreachable hw lane's type surface.
- No duplication within the chunk.

## 8. Dependencies & flags
- std only (`fmt`), no external deps, no unsafe, no feature gates. `RbspBitReader`/`BitReadError` from part1. Lint-clean per baseline clippy (#5/#6).

## 9. Open questions for the next agent
- W06-A05 (part3, owns `read_mmco_ops` 1072-1120 and the test module start): the same post-push `> MAX_OPS_PER_LOOP` pattern is at your end — check whether mmco ops also allow 257; and the test module's coverage of F-001/F-002 paths (my finding: none).
- W06-A07/A03: if any hw-lane test asserts `InvalidSliceType` for a deblock idc, that test enshrines F-001 — grep before fixing.
- SP/SI slices: after `slice_qp_delta`, SP/SI slice syntax has additional header-adjacent bits (sp_for_switch_flag sits in slice_data per my reading of §7.3.4) — `header_bits` for SP/SI slices may not point at the true MB-data start. UNVERIFIED against the spec text from memory; the sw engine refuses SP/SI anyway (sw.rs doc :54-56), so the blast radius is the future self-build only.
