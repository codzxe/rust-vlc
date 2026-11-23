# AUDIT: crates/codecs/src/video/h264/hw/vaapi/session.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 2134 / 33 fns + 2 impls + 14 tests |
| File verdict | REAL-UNPROVEN (code) **but UNREACHABLE from shipped entry points** — driver path never executed anywhere; only pure helpers + failure paths are proven |
| Reachable from shipped entry point? | **NO** — see F-001 chain |
| Findings | S0: 0 S1: 0 S2: 4 S3: 4 S4: 1 |

## 1. One-paragraph verdict
This is a genuinely-written, spec-aware VA-API H.264 session (2134 loc, hand-rolled 2.22.0 FFI, §8.2 DPB/ref-list/POC reconstruction, honest typed-refusal table) — but it is **dead code in the shipped product**: nothing in `lumen-cli` or `desktop` ever constructs it (the CLI hardwires `"h264" | "avc1"` to `H264SwDecoder`, pipeline_host.rs:346). Even its own module docs admit the driver-mediated path was never run ("no GPU ... `[VERIFY]`"). Inside the lane there are real correctness landmines for the day someone wires it up: a picNumX "two-form" match that contradicts the spec's subtraction-based derivation, a silent MMCO-4 no-op, and error paths that abandon a VA context between `vaBeginPicture` and `vaEndPicture`. The pure half (profile mapping, ref-list initial order, buffer packing, probe failure paths) is real and host-tested — I ran the 24 vaapi tests live, all pass.

## 2. What actually works (verified)
- `timeout 100 ./target/debug/deps/codecs-bae0f1756cc0a809 vaapi --test-threads=2` → `test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 482 filtered out`. Covers profile mapping, initial ref-list order, frame_num wrap, reorder ops, coded_size, slice/picture parameter packing, probe NoDeviceNode/PermissionDenied fixture paths, and `probe_on_the_real_host_is_the_typed_sandbox_outcome` (live: this host has no /dev/dri → typed NoDeviceNode arm).
- Probe failure paths are the *only* runtime-executable hardware-adjacent code in this sandbox; they behave typed as documented.

## 3. Findings

F-001 [S2 — unreachability of a 2134-loc lane]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:681 (mint), apps/cli/src/pipeline_host.rs:346
Code:
```
    pub fn new(node: DriNode) -> VaApiSession {
```
```
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```
What is wrong: `VaApiSession` is minted only by `VaApiBridge::open` (vaapi/mod.rs:173). The bridge list is installed only by `H264Decoder::from_setting` (decode.rs:310-317); `rg 'H264Decoder::from_setting'` finds callers **only in crates/codecs/tests/h264_hw_path.rs:861,897,911,956** and one comment (pipeline_host.rs:342). The shipped CLI decoder factory hardwires H.264 to the software decoder regardless of `playback.hw-decode-policy`; `rg H264Decoder apps/` → only that comment. Desktop: zero references. So the entire lane (session+bridge+probe) is compiled into the Linux release build (cfg(target_os="linux"), not feature-gated) yet no shipped entry point reaches it. The settings schema even exports `cliFlag: "--playback-hw-decode-policy"` (ui/src/screens/settings/generated/schema.ts:982) — a user-visible knob that flips nothing in the CLI h264 path.
Why it matters: 2000+ loc of `unsafe` FFI shipped dark; docs/UI advertise a policy the decode path ignores.
What correct looks like: either wire `from_setting` into the CLI factory keyed on the policy setting, or cfg-gate the lane out of release builds and drop the dead setting.
Confidence: HIGH — Verification: ran rg over apps/ + crates/ (production callers absent).

F-002 [S2 — picNumX derivation contradicts the spec form]
Where: session.rs:244-252 (and header claim L59-63)
Code:
```
                let difference = u64::from(abs_diff) + 1;
                // The spec form: (CurrPicNum + diff) % MaxFrameNum.
                let modulo = (u64::from(current_frame_num) + difference) % u64::from(max_frame_num);
```
What is wrong: H.264 §8.2.4.3.2 derives picNumX by **subtraction** (`CurrPicNum − differenceOfPicNums`, wrap-form `(CurrPicNum + MaxFrameNum − diff) % MaxFrameNum`; differenceOfPicNums is constrained to 1..=MaxFrameNum−1). The code's `modulo` uses **addition**, so the second match disjunct (L259-260) selects a *different picture* than the spec whenever `2·diff ≢ 0 (mod MaxFrameNum)`. The `negative` disjunct (wrapped `CurrPicNum − diff`) is the defensible reading; the `modulo` disjunct matches no standard reading. The same wrong form is baked into the re-flag value at L270 (`long_term_frame_idx = modulo` — asserted as 13 in the L1744-1753 test where the spec's picNumX is 5) and into test `wrap_encoded_reorder_target_resolves` (L1774-1783), which encodes `abs_diff_pic_num_minus1 = 27` with MaxFrameNum 16 — a value outside the spec's legal 0..14 range for that max, i.e. the test proves the author's own misreading, not conformance. Author flags the path `[VERIFY: no conformance corpus ran through this path]` (L62-63) — the flag is earned, this is the bug it was guarding.
Why it matters: wrong reference picture selection = corrupted P/B decode on real hardware, silently.
What correct looks like: picNumX = (CurrPicNum + MaxFrameNum − diff) % MaxFrameNum, compare against the §8.2.3 wrapped PicNum space (which `negative` already approximates); drop the additive disjunct.
Confidence: HIGH on the contradiction (spec + FFmpeg's `(cur_pic_num − (val+1)) mod max_frame_num` reading); MEDIUM that no encoder emits the additive encoding. Verification: read (no conformance corpus runnable — no GPU, lane unreachable).

F-003 [S2 — MMCO op 4 (SetMaxLongTermFrameIdx) is a silent no-op]
Where: session.rs:1158
Code:
```
                            MmcoOp::SetMaxLongTermFrameIdx { .. } => {}
```
What is wrong: MMCO 4 sets `MaxLongTermFrameIdx`; per §8.2.5.4 long-term pictures with `LongTermFrameIdx >= MaxLongTermFrameIdx` must be removed from the DPB. The code ignores the op entirely: stale long-term pictures survive, corrupting subsequent long-term ref-list order and `LongTermByPicNum` resolution. Not listed in the module's typed-refusal table (L50-57) — it's a silent gap, not an honest refusal.
Why it matters: any stream using long-term refs + MMCO 4 decodes wrong refs on hardware.
What correct looks like: implement the eviction, or refuse typed (`h264.hw.vaapi.mmco4`) like MMCO 6 is at L1165-1169.
Confidence: HIGH — Verification: read.

F-004 [S2 — error paths abandon the VA picture between begin/end]
Where: session.rs:527-533 (and 552-593, 605-621)
Code:
```
            if vcl.slice.field_pic_flag {
                let _ = self.pool.release(handle);
                return Err(H264HwError::Decode {
                    code: "h264.hw.vaapi.field_picture",
                });
            }
            let (list0, list1) = self.ref_lists(&self.dpb, &vcl.slice, max_entries)?;
```
What is wrong: after `vaBeginPicture` succeeds (L499), five distinct returns (field_pic at L527, `ref_lists` `?` at L533, both `push_buffer` calls L552-593 via `inspect_err`, and the `render_picture` failure at L605-621) exit **without calling `vaEndPicture`**. libva's contract pairs Begin/End on a context; leaving a picture in progress and later issuing a fresh `vaBeginPicture` on the same context is a driver-protocol violation (wedged context or driver-side leaks, driver-dependent).
Why it matters: one damaged access unit poisons every later picture in the stream on real hardware.
What correct looks like: an error after successful begin must still issue `vaEndPicture` (with whatever buffers exist) before returning, or tear the context down.
Confidence: MEDIUM-HIGH (libva contract read, not executed — no driver here). Verification: read.

F-005 [S3 — High profile accepted while its tools are hard-coded off]
Where: session.rs:1278 (and 1296, 1302)
Code:
```
        false, // transform_8x8_mode_flag — not retained; scaling-list note applies
```
What is wrong: `va_profile_for` serves profile_idc 100 (High) (L132), but `picture_parameter` writes `transform_8x8_mode_flag = false` unconditionally because the parser never retained the PPS flag. x264 `--profile high` enables 8x8 transforms by default — the most common H.264 content would be fed a picture-parameter that contradicts its slice data. Also `num_slice_groups_minus1: 0` is hard-coded (L1296) though parsed, and `second_chroma_qp_index_offset` duplicates `chroma_qp_index_offset` (L1302, admitted by comment). None of these are in the typed-refusal table — silent lies to the driver.
What correct looks like: refuse High-profile-with-8x8 typed until the parser retains the flag, or retain it.
Confidence: HIGH. Verification: read.

F-006 [S3 — per-slice full-buffer duplication + comment claims sharing]
Where: session.rs:548-550, 584-590
Code:
```
        // Order per the ABI's own usage: picture-parameter,
        // IQ-matrix, then per slice: slice-parameter, slice-data
        // (the data buffer is shared; its id is passed per slice).
```
What is wrong: the slice-data buffer is **not** shared: `push_buffer(VA_SLICE_DATA_BUFFER_TYPE, slice_data.as_ptr(), slice_data.len())` is called once per slice (L584-590), each a fresh `vaCreateBuffer` of the *whole* concatenated slice_data. An N-slice picture copies N× the picture's slice bytes driver-side (O(N·bytes), e.g. 8-slice 4K → 8 full copies per frame), and the comment misstates what the code does.
What correct looks like: one slice-data buffer per picture (the ABI's `slice_data_offset`/`num_elements` exist exactly for that), or at least per-slice ranges.
Confidence: HIGH. Verification: read.

F-007 [S3 — comment lies about the upstream surface-format gate]
Where: session.rs:885-887
Code:
```
        // Config: RT format YUV420 (the 8-bit 4:2:0 surface; other
        // samplings were refused typed by the codec layer's
        // surface_format gate upstream).
```
What is wrong: the gate (decode.rs:130-138) maps `ChromaFormat::Yuv422 => Some(PixelFormat::Yuyv422)` — 4:2:2 **passes** upstream; only 4:4:4 and >8-bit are refused. The session itself never checks `chroma_format_idc`. A (non-conformant but parsed) profile_idc-100 4:2:2 stream would reach `vaCreateSurfaces(VA_RT_FORMAT_YUV420)` with `seq_fields.chroma_format_idc=2` packed at L1262 — surface format contradicting the parameter set. Conformant 4:2:2 needs profile 122, which `va_profile_for` refuses, so exposure is limited to malformed streams — but the comment states a guarantee the code does not provide.
Confidence: HIGH. Verification: read (decode.rs:130-138 cross-checked).

F-008 [S3 — `unsafe impl Send` on a raw VADisplay relies on an unverified libva contract]
Where: session.rs:392-398
Code:
```
// SAFETY: the only non-Send member is the opaque `VADisplay` raw
// pointer (see the struct docs above): it is created and destroyed
// by this session alone (single owner, no aliasing) and every use
// goes through libva calls, which are internally locked per display
// (the 2.22.0 `va.h` thread-safety contract). Moving the session to
// another thread therefore races with nothing.
```
What is wrong: the cited "2.22.0 va.h thread-safety contract" is asserted, not verified; libva's historical contract is thread-safe *per-context*, with display-level calls requiring external serialization (VA/HW constraints documented in libva's own `THREAD SAFETY` caveats). Since the trait requires `Send` and the engine may move the decoder across threads, a wrong premise here is a data race inside the driver.
Confidence: LOW-MEDIUM (no libva source in sandbox to pin the 2.22.0 wording) — Verification: UNVERIFIED (cannot execute or read libva headers here).

F-009 [S4 — per-AU SPS/PPS clone + per-list DPB copy]
Where: session.rs:449-450, 197
Code:
```
        let (sps, pps) = match (&self.sps, &self.pps) {
            (Some(sps), Some(pps)) => (sps.clone(), pps.clone()),
```
What is wrong: every access unit clones the full SeqParameterSet (Vec + VUI tree) and PicParameterSet; `build_ref_list` does `dpb.to_vec()` per list per slice. Per-frame heap churn in the (currently unreachable) hot path. Also `va_op_error` (L1410-1416) deliberately discards `va_status_text` — diagnostics lost, `let _ =` swallow.
Confidence: HIGH. Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| va_profile_for | 123-135 | REAL | — | host-tested; CB/Main/High only, High10/422/444 refuse |
| slice_type_code | 138-146 | REAL | — | Table 7-7 fold, tested |
| pic_num_of | 168-175 | REAL | — | §8.2.3 wrap; correct |
| build_ref_list | 189-287 | SHALLOW | S2 | F-002: additive picNumX disjunct wrong; rest (initial order, splice, idc-2) sound + tested |
| splice_in | 290-294 | REAL | — | §8.2.4.3 insert semantics, tested |
| va_picture_of | 299-316 | REAL | — | flags/frame_idx per §8.2.2; note both fields get same POC (frame pictures) |
| coded_size | 319-329 | REAL | — | checked math, overflow→None, tested |
| Send impl | 398 | REAL-UNPROVEN | S3 | F-008: unverifiable libva contract premise |
| backend_id | 401-403 | REAL | — | constant "hw.vaapi", tested |
| configure | 405-446 | REAL-UNPROVEN | — | typed refusals (field_pic, poc_type1, geometry, profile) real; device bring-up never executed |
| decode | 448-663 | REAL-UNPROVEN | S2 | F-004 begin/end abandonment; pipeline order per ABI; pool release on every error path (good) |
| flush | 665-670 | REAL | — | honest: sync decode ⇒ nothing buffered |
| size_of_u32 | 674-676 | REAL | — | safe cast helper |
| new | 681-704 | REAL | — | unopened mint |
| with_root | 709-715 | REAL (cfg(test)) | — | test seam, unused in prod |
| va_version | 720-722 | REAL | — | accessor |
| next_free_surface | 728-741 | REAL | — | bounded, round-robin, DPB-aware |
| ensure_display | 745-788 | REAL-UNPROVEN | S4 | typed failures; leaks VADisplay if vaInitialize fails (no terminate); retry re-dlopens |
| ensure_profile | 792-838 | REAL-UNPROVEN | — | real query+contains logic, clamped counts |
| teardown_stream | 841-868 | REAL-UNPROVEN | — | guards invalid ids; pool.drain |
| create_stream_objects | 872-953 | REAL-UNPROVEN | S3 | F-007: YUV420 hard-coded + gate comment false |
| push_buffer | 957-986 | REAL-UNPROVEN | — | invariants documented once, num_elements=1 |
| picture_poc | 990-1035 | REAL-UNPROVEN | S4 | type-0 wrap per §8.2.1.1 (>=/>) correct; delta_bottom added to *top* field order cnt (L1016-1019) — bottom should carry it |
| ref_lists | 1039-1067 | REAL-UNPROVEN | — | I/Si empty lists per spec |
| update_dpb | 1077-1217 | SHALLOW | S2 | F-003 MMCO-4 no-op; sliding window + IDR clear + MMCO 1/2/3/5 real; MMCO-5 doesn't evict duplicate long-term idx (spec 8.2.5.4) |
| Drop::drop | 1221-1232 | REAL-UNPROVEN | — | reverse-order teardown |
| pick_parameter_sets | 1237-1247 | REAL | — | lowest-pps-id convention |
| picture_parameter | 1250-1307 | SHALLOW | S3 | F-005: transform_8x8/num_slice_groups/second_chroma hard-coded |
| slice_parameter | 1310-1381 | REAL | — | golden-tested packing incl. pred weights |
| fill_weights | 1384-1407 | REAL | — | flag-bit packing, bounded by 32 |
| va_op_error / va_configure_error | 1410-1422 | REAL | S4 | status text discarded |
| probe | 1430-1432 | REAL | — | live-tested on this host (NoDeviceNode) |
| probe_with | 1438-1607 | REAL | — | full typed ladder; terminates probe display; vendor string copied safely |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| profile_mapping_covers_the_served_profiles_only | YES | pure map, negative case included (High10→None) |
| initial_ref_list_orders_short_term_desc_then_long_term_asc | YES | real §8.2.4 order incl. long-term asc |
| frame_num_wrap_demotes_older_cycle_pictures | YES | wrap-aware PicNum, honest r3 fix note |
| reorder_ops_reorder_and_reflag | YES for ordering / NO for re-flag idx | asserts frame_idx==13 which is the *wrong* picNumX per spec (F-002) — tautological vs the bug |
| wrap_encoded_reorder_target_resolves | NO (vs spec) | encodes out-of-range diff=28 w/ MaxFrameNum 16; proves the author's misreading |
| missing_target_and_mvc_ops_are_typed_errors | YES | damage → typed MissingTarget, MVC → MvcViewOp |
| coded_size_is_macroblock_math | YES | incl. overflow → None |
| slice_parameter_carries_the_parsed_facts | YES | field-by-field golden incl. override |
| picture_parameter_packs_the_bitfield_words | YES | seq_fields/pic_fields bit words, va_reserved zero rule |
| probe_reports_no_device_node_on_a_dri_less_root | YES | fixture root, executed (ran live) |
| probe_reports_permission_denied_on_a_locked_node | YES | chmod-000 fixture, root-skip guard honest |
| probe_on_the_real_host_is_the_typed_sandbox_outcome | PARTIAL | shape-only when /dev/dri exists (admitted) |
| session_configure_without_parameter_sets_is_typed | WEAK | accepts any `h264.hw.vaapi.*` code (bring-up fails first — acknowledged in comment) |
| va_api_lane_backend_label_is_stable | YES | trivial label |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "VA-API ... runtime-tested here in its failure paths; driver-mediated decode path compile-verified only [VERIFY]" | hw/mod.rs:57-63 | TRUE | ran probe tests live; no driver in sandbox |
| "spec's `( CurrPicNum + differenceOfPicNums ) % MaxFrameNum` form" | session.rs:59-60 | FALSE | spec §8.2.4.3.2 subtracts; F-002 |
| "other samplings were refused typed by the codec layer's surface_format gate upstream" | session.rs:885-887 | FALSE for 4:2:2 | decode.rs:135 maps Yuv422→Yuyv422 (passes) |
| "the data buffer is shared; its id is passed per slice" | session.rs:550 | FALSE | a new full-size buffer is created per slice (L584-590) |
| "typed refusals ... never guesses" table | session.rs:48-57 | PARTLY FALSE | MMCO-4 silently ignored instead of refused (F-003) |
| feature-matrix HW row evidence | docs (W?? agents' scope) | not audited here | — |

## 7. Dead / duplicated / unreachable code
- The whole file is compiled-into-release-but-uncalled (F-001). `with_root`, `va_version` are test/diag-only. The `Root` fixture member never set in production.

## 8. Dependencies & flags
- cfg(target_os = "linux") via parent `hw/mod.rs:94`; NOT behind a Cargo feature — it ships in the release lane binary as dark code. Zero crate deps (hand-rolled FFI over dlopen'd libva 2.22.0). `#![allow(unsafe_code)]` module escape hatch (ADR-001 documented exception).

## 9. Open questions for the next agent
- libva 2.22.0 `va.h` thread-safety wording (F-008) — needs the header text to settle Send correctness.
- VA-API's exact `slice_data_bit_offset` semantics w.r.t. emulation-prevention bytes (ffi.rs:502-506 claims "spec's exact definition; EPB stays in the data buffer") — driver behavior unverifiable here; the feed keeps EPB (hw/mod.rs:82-85) and header_bits count unescaped bits, so a slice header containing an EPB sequence would shift the bit offset under some driver readings. UNVERIFIED.
- Whether any doc/matrix row cites the VAAPI lane as "landed" — matrix rows are another agent's chunk; the shipped CLI's h264 error string (pipeline_host.rs:377) lists h264 as landed (true — SW lane).
