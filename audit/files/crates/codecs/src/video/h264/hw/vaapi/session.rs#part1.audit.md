# AUDIT: crates/codecs/src/video/h264/hw/vaapi/session.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 1-533 of 2135 (chunk part1 of 4; part2=534-1067 W06-A09, part3=1068-1599 W06-A10, part4=1600-2134 W06-A03 — note: part4's header mislabels this ownership; the wave TSVs are authoritative) / 16 units (3 consts-groups, 11 fns/structs, `unsafe impl Send`, first half of `decode`) |
| File verdict | UNREACHABLE (real logic, no shipped entry point; see below) |
| Reachable from shipped entry point? | NO. `VaApiSession` ← `VaApiBridge` (hw/mod.rs:453-458) ← `H264Decoder::from_setting` (decode.rs) ← zero production callers (Grep crates/+apps/: only `crates/codecs/tests/h264_hw_path.rs` and a comment at apps/cli/src/pipeline_host.rs:342). Shipped lane: pipeline_host.rs:346 `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; product.toml:168 `hw-decode = false`, :191 `hw_decode_policy = "software-only"`; vaapi activation `[PROPOSAL]` pending ADR-013 (hw/mod.rs:62-63). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | Read 1-729 (part1 + boundary context into part2 for the leak/straddle analysis); Read frame_pool.rs (acquire/release/drain semantics — no Drop guard on handles); Grep `fn picture_poc|fn ref_lists|fn teardown_stream` (signature map); Read slice.rs:960-1007 (the parser's idc mapping my F-001 builds on); Grep `scaling_list` crates/codecs/src (F-004 contradiction); Grep `framing` (dead field note); rg -n "vaapi" audit/runs/baseline/test-workspace.log.txt:579-591 (execution evidence). |

## 1. One-paragraph verdict
Part1 is the session's honest front matter plus the pure §8.2 reconstruction layer (profile mapping, PicNum wrap, §8.2.4 ref-list building, coded-size math) and the first half of `decode` through the pool-acquire/begin-picture/slice-loop. The pure halves are genuinely built and golden-tested (baseline :579-591 all ok), and the module header is admirably honest about the driver-mediated path being compile-only in this GPU-less sandbox. But the reference-list reconstruction implements a **fabricated spec operation** — the parser maps `modification_of_pic_nums_idc` 1 (the ADD-direction short-term reorder) to a "short-term→long-term re-flag" (slice.rs:984-986), and this session implements that re-flag (re-setting the driver-facing entry's flags and frame_idx), so any real stream using idc-1 reordering — routine with B-pictures — would hand the driver a mistyped reference entry. The direction bit itself (0 vs 1) is additionally erased (both ops resolve through a dual-direction match). Second: two error returns inside `decode` leak the acquired surface slot permanently, violating the trait's own "session stays usable after a Decode error" contract — after ≤ pool-capacity damaged pictures the session bricks with pool_exhausted until a reconfigure. All of this sits in a lane no shipped binary can reach.

## 2. What actually works (verified)
— Pure reconstruction executed in baseline: `initial_ref_list_orders_short_term_desc_then_long_term_asc`, `frame_num_wrap_demotes_older_cycle_pictures`, `reorder_ops_reorder_and_reflag`, `wrap_encoded_reorder_target_resolves`, `missing_target_and_mvc_ops_are_typed_errors`, `coded_size_is_macroblock_math` — test-workspace.log.txt:579-591, all `ok`.
— `pic_num_of` (168-175) matches §8.2.3 exactly (previous-cycle entries get `frame_num − MaxFrameNum` when `frame_num > current`); re-derived by me from the spec rule.
— `coded_size` (319-329) checked arithmetic: `checked_add/checked_mul` chain, `u32::MAX` width → typed None (baseline test asserts this, :579).
— `next_free_surface` (728-741, boundary context): `0..count` loop guards the `% count` — no division by zero on an empty surface set; held-by-pool-or-DPB predicate correct.
— configure's typed-refusal ladder (405-446): every refusal is a named code; state resets (dpb/POC/cursor) all present (439-444).

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:232-272 (with parser twin at crates/codecs/src/video/h264/parser/slice.rs:984-986)
Code:
```
            // idc 0/1: a short-term picture by the §8.2.4.3.2
            // picNumX derivation (possibly re-flagged long-term).
            short_term_op => {
```
```
                        if matches!(
                            short_term_op,
                            RefPicListModOp::ShortTermToLongTermByPicNum { .. }
                        ) {
                            entry.long_term = true;
                            entry.long_term_frame_idx = u32::try_from(modulo).unwrap_or(0);
                        }
```
What is wrong: Per H.264 §7.3.3.1/§8.2.4.3.2, `modification_of_pic_nums_idc` 1 selects a **short-term** picture with picNumX = CurrPicNum + (abs_diff_pic_num_minus1+1) (ADD direction); it re-flags nothing (short-term→long-term re-flagging lives in `dec_ref_pic_marking` MMCO ops 3/6, which the parser models separately and correctly at slice.rs:1084). The parser maps idc 1 to a fabricated `ShortTermToLongTermByPicNum` (slice.rs:984-986), and this session implements the fabrication: the matched entry is re-flagged `long_term` with `LongTermFrameIdx = modulo` (= CurrPicNum+diff mod MaxFrameNum — not even a LongTermFrameIdx), so the driver-facing `VAPictureH264` carries `VA_PICTURE_H264_LONG_TERM_REFERENCE` and a bogus frame_idx for what the bitstream declared a plain short-term reorder. Compounding it, the direction bit is erased: `ShortTermByPicNum` (idc 0, SUBTRACT) resolves through a dual match — `pic_num == negative` (subtract) OR `frame_num == (curr+diff) % max && frame_num > current` (add, :253-261) — so an add-direction target can resolve to the subtract-direction picture when both candidates exist in the DPB.
Why it matters: Any stream that uses idc-1 reordering — ordinary for B-picture reordering — gets a wrong reference list handed to the driver: wrong reference typing and wrong ordering → misprediction → visual corruption, silently. The module's own typed-refusal table promises "damage: refuse, never mispredict" (:56); this is a spec misreading, not damage. It is pinned as golden behavior by part4's `reorder_ops_reorder_and_reflag` (1730-1771), so the test suite certifies the bug. Mitigation: the lane is unreachable in shipped builds today (ADR-013 pending); the moment it activates, every idc-1-using stream mis-decodes.
What correct looks like: parser: idc 1 → `ShortTermByPicNum` with a direction bit (or two variants Add/Sub); session: compute picNumX per spec form for the actual direction and never mutate the entry's long-term state here; delete the re-flag block; rewrite the golden test to the spec semantics; add a conformance fixture that exercises idc 1.
Confidence: HIGH on the op-semantics contradiction (spec §7.3.3.1/§8.2.4.3.2 read against slice.rs:980-997 and session.rs:232-272; ffmpeg's `decode_ref_pic_list_modification` treats 0/1 as subtract/add of short-term pictures — auditor domain knowledge, no spec copy in sandbox for a verbatim quote: UNVERIFIED-provenance, HIGH-agreement). The two-form ambiguity the author themselves flags (module header :59-63 "[VERIFY: no conformance corpus ran through this path]") is adjacent but distinct — this finding is not ambiguity, it is a different operation than the spec defines.
### F-002 [S2]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:506 and :533 (acquire at 491-496)
Code:
```
        let handle = self
            .pool
            .acquire(surface_id)
            .map_err(|_| H264HwError::Decode {
                code: "h264.hw.vaapi.pool_exhausted",
            })?;
```
```
        let poc = self.picture_poc(&sps, &header, first_vcl.nal.idr_pic_flag)?;
```
```
            let (list0, list1) = self.ref_lists(&self.dpb, &vcl.slice, max_entries)?;
```
What is wrong: `decode` acquires a pool slot (491-496) and releases it on every VA-call failure (501, 528, 559/569/581/591, 619, 625, 632, 650 — verified each) but NOT on the two pure-logic error returns: `picture_poc` (506: POC overflow / type mismatch → `h264.hw.vaapi.poc`) and `ref_lists` (533: `h264.hw.vaapi.ref_list` / `h264.hw.vaapi.mvc` — the module's own "damage: refuse" path). `HwSurfacePool` has no Drop guard (frame_pool.rs: acquire at :142, release at :186, drain at :227 — a handle is the only way back to Free), so each such error permanently consumes one of the bounded surfaces (capacity = num_ref_frames + 6, min 4 — :104-115) until the next configure's `teardown_stream` → `pool.drain()` (:867).
Why it matters: The trait contract this session implements says "The session stays usable after a `Decode` error (the codec layer counts damage and continues)" (hw/mod.rs:298-299). It does not: a damaged stream (truncated/corrupt — the normal case for a media player) that trips ~capacity ref-list refusals bricks the session — every subsequent picture fails `pool_exhausted` and there is no recovery path short of a reconfigure nobody triggers. The typed-refusal design ("refuse, never mispredict") silently becomes "refuse N times, then die".
What correct looks like: A scope guard (or explicit release) covering every `?` between acquire and emit; same for the `update_dpb` `?` at 638-646 (part2's range — flagged for W06-A09), and a note that begin_picture-without-end_picture on these bail-outs leaves the driver-side picture abandoned (tolerable per libva, but should be documented).
Confidence: HIGH — Verification: read; every release site enumerated; frame_pool.rs read in full; picture_poc/ref_lists error arms read (995-1067).
### F-003 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:449-451
Code:
```
        let (sps, pps) = match (&self.sps, &self.pps) {
            (Some(sps), Some(pps)) => (sps.clone(), pps.clone()),
```
What is wrong: Both parameter sets are deep-cloned (they contain Vecs: ref-list ops, pred-weight tables, VUI) on every access unit — a per-picture heap allocation churn purely to appease the borrow checker across the `&mut self` calls below; the `calls` fn-pointer snapshot (477-486) shows the author knows the pattern but stopped short of applying it here.
Why it matters: Per-frame clone in the decode hot path (the checklist's "clone/to_vec/collect per frame"); SPS/PPS are stable within a generation, so the clone is pure waste on every single picture.
What correct looks like: Split borrows (take `&SeqParameterSet` out via a scoped block before the `&mut self` calls, or move the pure computations — poc, ref lists — before any `&mut self` mutation and pass `&self` reads after), or `Arc` the parameter sets.
Confidence: HIGH — Verification: read.
### F-004 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:57 (contradicted by ffi.rs:460-466)
Code:
```
//! | scaling lists | — (ffi.rs honesty note) | not detectable until the parser retains the flag; flat defaults used, `[VERIFY]` |
```
What is wrong: The honest table here admits SPS scaling lists are undetectable and flat IQ defaults are shipped — but ffi.rs:460-466 tells the reader the opposite: "A stream whose SPS signals `seq_scaling_matrix_present_flag` is refused typed by the session (`h264.hw.vaapi.scaling_list`) rather than decoded with different quantization than the bitstream asked for". No such refusal code exists (Grep `scaling_list` across codecs/src: only the ffi.rs claim, the honest table, and the parser's walk-and-discard at h264/parser/sps.rs:226-235). The parser walks `seq_scaling_matrix` and retains nothing, so such a stream reaches `VAIQMatrixBufferH264::flat_default()` (session.rs:520) and decodes with wrong quantization.
Why it matters: Two docs in the same module disagree about a correctness-relevant behavior; the one that promises safety (typed refusal) is the false one. If the lane ever activates, scaling-list streams silently mis-decode (wrong dequant) instead of refusing — exactly the "never a guess" posture this module is built on.
What correct looks like: Parser retains `seq_scaling_matrix_present_flag` (one bool); session refuses `h264.hw.vaapi.scaling_list` at configure; ffi.rs's claim becomes true. Until then, fix ffi.rs's honesty note to match session.rs's.
Confidence: HIGH — Verification: read both sites + parser sps.rs:226-235; Grep shows zero refusal sites.
### F-005 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:392-398
Code:
```
// SAFETY: the only non-Send member is the opaque `VADisplay` raw
// pointer (see the struct docs above): it is created and destroyed
// by this session alone (single owner, no aliasing) and every use
// goes through libva calls, which are internally locked per display
// (the 2.22.0 va.h thread-safety contract). Moving the session to
// another thread therefore races with nothing.
unsafe impl Send for VaApiSession {}
```
What is wrong: The justification leans on an external contract ("internally locked per display, the 2.22.0 va.h contract") that is unverifiable from this tree (no libva headers in the sandbox; see my ffi.rs audit F-001 — the same unverifiable-provenance family) and historically nuanced in libva (thread-safety hardening landed gradually across 1.x/2.x).
Why it matters: `unsafe impl Send` on a raw-pointer-holding struct is the highest-stakes line in the file; its safety argument should be checkable, not a citation of a header nobody vendored.
What correct looks like: Vendor the cited va.h excerpt (license-permitting; MIT) beside the ffi declarations, or cite the libva docs URL + commit in the comment; alternatively structure the session so the display lives on one thread (channel-fed worker) and delete the unsafe impl.
Confidence: MEDIUM (the claim is plausible) — Verification: UNVERIFIED (no header available).
### F-006 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:123-135
Code:
```
    match sps.profile_idc {
        66 | 88 => Some(VA_PROFILE_H264_MAIN), // Baseline/Extended → Main superset
        77 => Some(VA_PROFILE_H264_MAIN),
```
What is wrong: Plain Baseline (66, no constraint_set1) and Extended (88) map to `VAProfileH264Main`; libva's `VAProfileH264Baseline` (=5) is never used. Two wrinkles: (a) drivers that expose Baseline but not Main (embedded GPUs exist) will refuse streams they could decode; (b) Extended-profile SP/SI slices — which `slice_type_code` happily packs (138-146) — are not decodable under a Main config.
Why it matters: Interop loss on minority drivers and a silent profile mismatch for SP/SI streams; no test covers either (part4's mapping test only asserts the mapping itself).
What correct looks like: Use `VA_PROFILE_H264_CONSTRAINED_BASELINE`/`VA_PROFILE_H264_BASELINE` for baseline streams when the driver table has it (prefer exact, fall back to Main), and refuse SP/SI slices under a Main profile.
Confidence: MEDIUM — driver-population claims are domain knowledge (UNVERIFIED); the mapping itself is read and test-pinned (part4 `profile_mapping_covers_the_served_profiles_only`, baseline :587).

## 4. Per-function table (part1 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + consts | 1-115 | REAL | — | Honest [VERIFY] posture; VA_DPB_SLOTS=16 / VA_REF_LIST_SLOTS=32 match the frozen ABI (ffi.rs struct widths); SURFACES_BEYOND_DPB=6 per VA-API guidance. |
| `va_profile_for` | 123-135 | REAL | S4 | F-006. CB detection via constraint_set1 (bit 0x40) correct. |
| `slice_type_code` | 138-146 | REAL | — | Table 7-7 mapping P0/B1/I2/SP3/SI4 correct; parser pre-folds the +5 repeats. |
| `DpbPicture` | 151-163 | REAL | — | Raw facts only; wrap derived per-use — right shape. |
| `pic_num_of` | 168-175 | REAL | — | §8.2.3-exact (re-derived); executed via wrap tests (baseline :581). |
| `RefListError` | 178-184 | REAL | — | Two-variant typed vocabulary. |
| `build_ref_list` | 189-287 | REAL (spec-wrong) | S2 | F-001. Initial order §8.2.4.2.1-correct (short-term PicNum desc, long-term idx asc — golden-covered); idc-2 long-term lookup correct; idc-0/1 handling fabricated + direction-erased. `max_entries.min(VA_REF_LIST_SLOTS)` truncate fine. |
| `splice_in` | 290-294 | REAL | — | §8.2.4.3.3 insert-at-position semantics; position monotonicity correct. |
| `va_picture_of` | 299-316 | REAL | — | frame_idx = LongTermFrameIdx for long-term per §8.2.2 mapping; top==bottom POC correct for frame pictures (field pictures refused). |
| `coded_size` | 319-329 | REAL | — | Checked math; field-doubling branch unreachable via session (frame_mbs_only enforced at :413) but exercised by the direct unit test. |
| `VaApiSession` struct | 335-390 | REAL | — | 17 fields, all owned/typed; teardown order documented. |
| `unsafe impl Send` | 398 | REAL-UNPROVEN | S4 | F-005. |
| `backend_id` | 401-403 | REAL | — | Delegates to capability enum — stable label. |
| `configure` | 405-446 | REAL | — | Typed refusal ladder; note: a `create_stream_objects` failure after `teardown_stream` (:434) leaves sps/pps stale-but-Some and surfaces empty → subsequent decodes fail typed at `next_free_surface`, never on destroyed ids — acceptable, undocumented. |
| `decode` (first half) | 448-533 | REAL (leaky) | S2 | F-002 (leaks at :506/:533); F-003 (per-AU clone); MVC/field refusals typed; MVC NAL scan (458-466) is O(units) per AU — fine. `offset`/`data_size` `unwrap_or(u32::MAX)` at :534-535 would corrupt slice params for >4 GiB NALs — impossible inputs, acceptable. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (part1's functions are tested from part4's module) | YES for arithmetic | Wrap/order/coded-size goldens are hand-computed and content-sensitive (baseline :579-591). |
| `reorder_ops_reorder_and_reflag` (session.rs:1730-1771) | NO — it PINS the bug | It asserts the fabricated idc-1 re-flag semantics (F-001); "correcting" the code to the spec FAILS this test. A golden generated from the implementation under test. |
| configure/decode driver halves | — | No test reaches them (no GPU; part4's configure test short-circuits at bring-up — W06-A03's F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "reconstructs list0/list1 per §8.2.4 (initial order + the parsed ref_pic_list_modification ops)" | session.rs:44-46 | FALSE (for idc 1) | F-001: idc 1 is implemented as a long-term re-flag, not §8.2.4.3.2's add-direction short-term move. |
| "§8.2.4.3's picNumX modular arithmetic … the exact two-form reading is [VERIFY: no conformance corpus ran through this path]" | session.rs:59-63 | REAL (honest gap) | The dual-form match at :253-261 is exactly the acknowledged guess; no corpus test exists. |
| "damage: refuse, never mispredict" (ref-list op naming an absent picture) | session.rs:56 | REAL (but see F-002) | :274-278 typed MissingTarget; the refusal path itself leaks the surface slot. |
| "Every failure lands in the typed Configure error channel, never a panic" | session.rs:25-26 | REAL | configure traced: all failures typed; no unwrap in 405-446. |
| "libva's public API is thread-safe per display (internally locked; the 2.22.0 va.h contract)" | session.rs:345-346 | UNVERIFIED | No header in sandbox (F-005). |

## 7. Dead / duplicated / unreachable code
- The whole part1 surface is unreachable from shipped entry points (header table).
- `H264HwStreamInfo.framing` (populated by decode.rs:425) is read by no adapter — the vaapi session never touches it (Grep `framing` in session.rs: no matches); dead field for the only compiled lanes (hw/mod.rs owner's territory, noted in passing).
- The `modulo`-arm of the dual match (:259-260) exists to serve the parser's mislabeled idc-1 flow; under a spec-correct parser it would be reachable only via ShortTermByPicNum's add case, which the parser cannot express (F-001).

## 8. Dependencies & flags
- `#[cfg(target_os = "linux")]` gate (hw/mod.rs:94) — compiles into every Linux release artifact, dead at runtime (product.toml:168/191).
- No Cargo feature gate (in-house hw modules unconditional, codecs/Cargo.toml:71-72).
- `#![allow(unsafe_code)]` (:74) overrides workspace deny (root Cargo.toml:54) — the sanctioned E2 exception; the three unsafe blocks in my range (:499 begin_picture + the two clones' FFI context) each carry SAFETY comments.

## 9. Open questions for the next agent
- W06-A09 (part2): (a) the `update_dpb` `?` at :638-646 leaks the pool handle like F-002 — please file/merge; (b) `push_buffer` early-returns (:552-593) skip the buffer-destroy loop at :613-617 — created VABufferIds leak until the context teardown frees them driver-side; (c) `picture_poc`'s Type0 wrap arithmetic (1005-1010: `>= max/2` vs `> max/2` asymmetry) deserves a spec cross-check — the two comparisons differ by one and I could not decide which is right without the spec text.
- Whoever owns parser/slice.rs: F-001's root is the idc-1 mapping at :984-986 — the session fix is cosmetic until the parser is fixed.
- ADR-013 ratifiers: F-001/F-002/F-004 are pre-activation blockers in my view; the lane's "honest gaps" table is accurate but its §8.2.4 claim is not.
