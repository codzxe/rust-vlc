# AUDIT: crates/codecs/src/video/h264/hw/vaapi/session.rs#part3 (lines 1067-1599)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 533 (of 2134) / 10 (chunk boundaries: part2 ends mid-`ref_lists` at 1067; `probe_with` continues past 1599 into part4 to line 1607 — audited as one unit, tail noted) |
| File verdict | UNREACHABLE (real DPB/FFI code of a lane no shipped entry point constructs) |
| Reachable from shipped entry point? | NO — same chain as capability.rs: `VaApiSession`/`probe` ← `vaapi::discover`/bridge ← `hw::negotiate_lane` (hw/mod.rs:483) ← `decode.rs:312 H264Decoder::from_setting` ← no production caller (rg `H264Decoder`: codecs-internal + comment at apps/cli/pipeline_host.rs:342). Additionally `#[cfg(target_os = "linux")]` (hw/mod.rs:94). Module doc itself: sandbox has no GPU — driver path never executed anywhere (session.rs:5-16). |
| Findings | S0: 0 S1: 1 S2: 3 S3: 3 S4: 1 |
| Commands run | rg `fn pic_num_of|fn build_ref_list` (context read of part1 helpers 152-316); rg `pred_weight|mmco|SetMaxLongTerm` in session.rs (test coverage: none for MMCO/weights — both test SliceHeaders set `pred_weight_table: None` at 1880/1969); Read ffi.rs:478-608 (struct layout + size pins); baseline test log :579-595 (15 session tests ok). |

## 1. One-paragraph verdict
The DPB bookkeeping (§8.2.5) and VA-API buffer packing in this chunk are written to a high standard of typed errors and documented invariants — and the pure parts are golden-tested (15 session tests, baseline :579-593). But three genuine correctness defects sit in the DPB/packing model: MMCO op 4 is a silent no-op (stale long-term references are never evicted), `fill_weights` shifts a `1` into a u8 flag for indices up to 31 (debug panic on valid weighted streams with >8 refs; silent bit-aliasing in release), and the slice-parameter packing writes through an ffi binding whose `slice_data_bit_offset` is modeled u16 where the real libva 2.22.0 header uses u32 — a field-level ABI mismatch the 3128-byte size pin cannot catch (both layouts are 3128). All of it is unreachable from shipped builds and has never touched a GPU (module doc lines 5-16 admit it), so nothing user-visible is broken today — but this is exactly the code that would silently produce garbage the day the lane is enabled.

## 2. What actually works (verified)
— `update_dpb` IDR path (§8.2.5.1 clear) and the wrapped-PicNum MMCO target arithmetic: `target = frame_num − diff` matches `pic_num_of`'s relative space for both same-cycle and previous-cycle targets (hand-traced: current=2, max=16, diff=3 → target −1 matches prev-cycle raw 15 via 15−16). Baseline test `frame_num_wrap_demotes_older_cycle_pictures` (:581 ok) covers the space.
— `picture_parameter` bitfield packing — test `picture_parameter_packs_the_bitfield_words` (:584 ok).
— Probe failure taxonomy (`probe_with` 1438-1597): every early exit returns a typed `HwUnavailableReason` (NoDeviceNode/PermissionDenied/LibraryUnavailable/DriverIncompatible); no guessed success. The two failure tests (`probe_reports_no_device_node_on_a_dri_less_root` :586, `probe_reports_permission_denied_on_a_locked_node` :589) and the real-host outcome test (:585) ran for real in baseline.
— `slice_data_bit_offset = 8 + header.header_bits` (line 1325) is the spec-correct derivation.

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1158
Code:
```rust
                            MmcoOp::SetMaxLongTermFrameIdx { .. } => {}
```
What is wrong: MMCO op 4 (`memory_management_control_operation == 4`, `max_long_term_frame_idx_plus1`) is a silent no-op. Per §8.2.5.4.4 it evicts every long-term entry with `LongTermFrameIdx >= max_long_term_frame_idx`; this DPB does not even track the bound.
Why it matters: any conformant stream that uses MMCO4 (the standard way to flush long-term references — e.g. periodic long-term reference schemes) leaves stale long-term pictures in the DPB forever; they remain eligible for `build_ref_list` long-term ordering (part1) and for `MmcoOp::MarkLongTermUnused` idx collisions — references the encoder retired are still used → mispredicted frames with no error raised. No test covers MMCO ops at all (rg: zero `MmcoOp` mentions in the test module).
What correct looks like: track `max_long_term_frame_idx`, evict `long_term && long_term_frame_idx >= max` on op 4 (and treat `max_long_term_frame_idx_plus1 == 0` as "no long-term pictures"); add a test.
Confidence: HIGH — Verification: read (spec §8.2.5.4.4 semantics; code has no eviction anywhere in update_dpb).

### F-002 [S1]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1393-1405 (called from slice_parameter:1361-1378)
Code:
```rust
    for (index, entry) in entries.iter().take(VA_REF_LIST_SLOTS).enumerate() {
        if let Some((weight, offset)) = entry.luma {
            *luma_flag |= 1 << index;
```
What is wrong: `luma_flag: &mut u8` (ffi.rs:534 `pub luma_weight_l0_flag: u8`, doc "(bit i = entry i)") but `index` ranges to `VA_REF_LIST_SLOTS − 1 = 31` (line 1393 `take(VA_REF_LIST_SLOTS)`). `1u8 << index` for index ≥ 8 is a shift overflow: **panic in debug builds** on a valid weighted-prediction stream with >8 reference entries; in release the shift is masked (`index % 8`) so entry 9 sets the flag bit of entry 1 — wrong per-entry weights delivered to the driver. Same pattern for `*chroma_flag |= 1 << index` (line 1400). The 8-bit storage cannot represent entries 8–31 at all — the arrays are `[i16; 32]` next to an 8-bit flag word.
Why it matters: valid H.264 (`num_ref_idx_active` up to 32 with explicit weighted prediction) panics a debug build and silently corrupts weights in release. Zero test coverage: both test `SliceHeader`s set `pred_weight_table: None` (session.rs:1880, :1969), so `fill_weights` has never executed in any run.
What correct looks like: refuse typed at configure when weighted lists exceed the representable width (the lane's own "typed refusals" doctrine), or widen the flag to u32 if the real ABI allows — which is an ffi.rs question (cross-flagged to A08, who owns ffi.rs: must verify the real `VASliceParameterBufferH264.luma_weight_l0_flag` width in libva 2.22.0; the size pin 3128 does not discriminate).
Confidence: HIGH — Verification: read + Rust shift semantics + ffi.rs:533-536 type confirmed.

### F-003 [S2]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1325 (+ ffi.rs:504)
Code:
```rust
    sp.slice_data_bit_offset = u16::try_from(8 + header.header_bits).unwrap_or(u16::MAX);
```
What is wrong: the repo's frozen-ABI binding declares `slice_data_bit_offset: u16` (ffi.rs:504), but libva 2.22.0's `VASliceParameterBufferH264` declares `uint32_t slice_data_bit_offset` (its doc "bit offset from the start of NAL header buffer" is quoted verbatim at ffi.rs:500-503 — the field is 4 bytes in the header). Both layouts total 3128 bytes (u32 bit-offset consumes 2 header bytes that reappear as padding before `RefPicList0` at offset 28 in both shapes), so the compile-time size pin (ffi.rs:608) passes while the FIELD layout diverges: the driver reads `slice_data_bit_offset` from offset 12 (u32) — picking up the repo's `slice_data_bit_offset` (u16 @12) plus `first_mb_in_slice` (u16 @14) as the high half — and reads `first_mb_in_slice` from offset 16 where the repo wrote `slice_type`.
Why it matters: if this lane ever runs, EVERY slice buffer hands the driver a wrong bit offset and a wrong first-macroblock — guaranteed garbage decode, no error. This is precisely the class of bug "size-pinned frozen ABI" claims to exclude; the pin gives false confidence.
What correct looks like: `slice_data_bit_offset: u32` in ffi.rs (value still ≤ 8+header_bits, no truncation issue), pin retained; add a field-offset assert (`memoffset`-style or a const offset computation) so layout — not just size — is pinned. Cross-file: ffi.rs is W06-A08's assignment — verify against the actual libva header (I could not read the system libva headers in this sandbox: UNVERIFIED against the installed libva; the u32 claim is from the libva header's published shape).
Confidence: MEDIUM — Verification: read + layout arithmetic (both shapes 3128 — verified by hand); the real header's type is UNVERIFIED in-sandbox (no libva dev files; A08 to confirm).
### F-004 [S2]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1278 (+ 123-134 profile map)
Code:
```rust
        false, // transform_8x8_mode_flag — not retained; scaling-list note applies
```
What is wrong: the picture parameter hardcodes `transform_8x8_mode_flag = false` because the in-house PPS parser does not retain the flag — while `va_profile_for` (line 132) maps `profile_idc == 100` to `VA_PROFILE_H264_HIGH` and the probe accepts High as served. High-profile streams (x264's default for ≥720p) commonly use 8×8 transforms; the slice data handed to the driver then contains 8×8-transform residual syntax while the parameter buffer says it does not.
Why it matters: silent misdecode (driver parses residual with the wrong transform), not a typed refusal — the exact "never a fake success" contract this module otherwise enforces for field pictures / poc_type1 / MVC. Same family: `MinLumaBiPredSize8x8` hardcoded false (line 1268) and scaling lists defaulted flat (module-doc honesty table line 57, `[VERIFY]`).
What correct looks like: either the parser retains `transform_8x8_mode_flag` (and 8x8 scaling lists) or the lane refuses PPS/streams it cannot represent truthfully.
Confidence: HIGH — Verification: read (line 1278 + profile map 129-134 + module-doc honesty table 50-57).

### F-005 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1184-1203 (vs 990-1021)
Code:
```rust
                } else if *lsb < self.prev_ref_poc_lsb
                    && self.prev_ref_poc_lsb.wrapping_sub(*lsb) >= max / 2
                {
                    self.prev_ref_poc_msb.wrapping_add(max)
```
What is wrong: the §8.2.1.1 MSB wrap logic is implemented twice — once in `picture_poc` (part2, checked i64 arithmetic with typed overflow error) and once here in `update_dpb` (wrapping arithmetic, silently wrapping). Two copies of subtle spec arithmetic with different overflow disciplines = drift risk; the wrap conditions themselves match (verified by side-by-side read).
Why it matters: the next editor fixes a bug in one copy; the DPB's stored `prev_ref_poc_msb` diverges from the POC computation's assumption → wrong POCs for wrapped streams.
What correct looks like: one shared `advance_poc_type0(prev_lsb, prev_msb, lsb, max) -> (msb, Result)` helper used by both.
Confidence: HIGH — Verification: read of both sites.

### F-006 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1289-1291, 1304, 1325-1326, 1331-1332, 1341-1342, 1345-1349
Code:
```rust
        picture_width_in_mbs_minus1: u16::try_from(sps.pic_width_in_mbs_minus1).unwrap_or(u16::MAX),
```
What is wrong: ~10 packing sites clamp out-of-range parsed values silently (`unwrap_or(u16::MAX)`, `unwrap_or(0)`). For malformed streams (huge `ue(v)` values — the parser accepts them), the driver receives a quietly wrong buffer instead of a typed refusal.
Why it matters: contradicts the module's stated doctrine ("typed refusals, never guesses", session.rs:48); a malformed stream misdecodes instead of erroring, wasting the damage-accounting the parser layer built.
What correct looks like: `?`-map each try_from to `H264HwError::Decode { code: "h264.hw.vaapi.field_range" }`.
Confidence: HIGH — Verification: read (sites listed).

### F-007 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1530-1542
Code:
```rust
        let status = unsafe { (lib.initialize)(display, &mut major, &mut minor) };
        if status != VA_STATUS_SUCCESS {
            return (
```
What is wrong: when `vaInitialize` fails, the probe returns without `vaTerminate` — the libva display handle created by `vaGetDisplayDRM` (line 1514) leaks. Every other exit path after initialize terminates (line 1585).
Why it matters: process-lifetime resource leak on driver-initialization failure (repeated probes accumulate); minor since probes are one-shot per negotiation.
What correct looks like: terminate before the failed-initialize return (libva documents vaTerminate as valid after a failed vaInitialize).
Confidence: MEDIUM — Verification: read; libva's tolerance of terminate-after-failed-init UNVERIFIED in-sandbox (no libva).

### F-008 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1409-1422
Code:
```rust
    let _ = va_status_text(status);
    H264HwError::Decode { code }
```
What is wrong: both status mappers stringify the VA status and immediately discard it — the typed error carries only the static op code ("h264.hw.vaapi.create_buffer"), losing the driver's actual diagnosis (VA_STATUS_ERROR_ALLOCATION_FAILED etc.). The `let _ =` calls exist only to touch the stringifier.
Why it matters: a hardware lane whose failures are undiagnosable in the field; the comment defers to a `[VERIFY: structured status payload]` that does not exist.
What correct looks like: `code: concat!`-static + the status text in the error's message channel (H264HwError::Decode is `{ code }` only — the error type itself needs a payload field; cross-file).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ref_lists` (tail arms, boundary with part2) | 1057-1067 | REAL | — | I/SI → empty lists; B → both; P/SP → l0 only; boundary: body in part2 |
| `update_dpb` | 1069-1217 | REAL | S2 | IDR clear + wrapped-PicNum MMCO targets verified correct; MMCO4 no-op (F-001); sliding-window runs even under adaptive marking (deviation from §8.2.5.3's condition — benign for conformant streams since adaptive control keeps DPB in bounds; noted, not separately rated); MMCO3 target-missing is silently ignored (1149-1156) while ref-list op missing-target errors typed — inconsistent damage policy, folded here |
| `Drop for VaApiSession` | 1220-1233 | REAL | — | teardown_stream → terminate → drop fd: order correct; partial-configure states depend on part2/4's teardown_stream (not verifiable in my range) |
| `pick_parameter_sets` | 1235-1247 | REAL | — | min-id PPS → its SPS; single-pair convention — multi-SPS switching streams simplified (per-slice pps_id validation not visible in my range; open question to A03) |
| `picture_parameter` | 1249-1307 | REAL | S2 | packing verified vs ffi fields; hardcoded-false 8x8 flag on a served High profile (F-004); silent clamps (F-006); `second_chroma_qp_index_offset = chroma_qp_index_offset` is spec-correct when 8x8 absent |
| `slice_parameter` | 1310-1381 | REAL | S2 | `slice_data_bit_offset` written into mis-shaped binding field (F-003); num_ref_idx override clamps (F-006 family) |
| `fill_weights` | 1383-1407 | REAL | S1 | shift overflow + unrepresentable flags (F-002); never executed by any test |
| `va_op_error` / `va_configure_error` | 1409-1422 | REAL | S3 | discard driver status (F-008) |
| `probe` / `probe_with` (through 1599; tail 1600-1607 in part4) | 1424-1607 | REAL | S4 | typed failure taxonomy verified at every exit (tests :585/:586/:589 real); vaInitialize-failure display leak (F-007); bounded profile-query allocation; vendor-string CStr handling SAFETY-documented |

## 5. Test honesty (tests live in part4's range, lines 1615+; they exercise MY functions)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| picture_parameter_packs_the_bitfield_words | PARTIAL | locks bitfield words; would NOT catch F-004 (hardcoded false is part of the golden) — golden-vs-code circularity risk if goldens were generated from this code |
| slice_parameter_carries_the_parsed_facts | PARTIAL | covers scalar fields; `pred_weight_table: None` (1880) → fill_weights (F-002) uncovered |
| frame_num_wrap_demotes_older_cycle_pictures / wrap_encoded_reorder_target_resolves / reorder_ops_reorder_and_reflag | YES (part1 functions) | exact ordering/flag asserts |
| probe_reports_no_device_node_on_a_dri_less_root / …_permission_denied… / probe_on_the_real_host_is_the_typed_sandbox_outcome | YES | real filesystem fixtures; the only lane code that genuinely executes in this environment |
| (no test names MMCO / update_dpb / SetMaxLongTerm / sliding) | — | update_dpb's §8.2.5 semantics (F-001) have zero direct coverage |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runtime hardware behavior NOT exercised — [VERIFY]" (whole driver pipeline) | session.rs:5-16 | REAL (honest admission) | no GPU in sandbox; probe tests only exercise typed failure paths |
| "buffer field packing — pure functions, golden-pinned" | session.rs:13-15 | REAL | tests :584/:590 (with the circularity caveat above) |
| "§8.2.4.3's picNumX … two-form reading is [VERIFY: no conformance corpus ran]" | session.rs:59-63 | REAL (admission) | build_ref_list accepts both forms (245-261, part1) — flagged to A08: the two forms name DIFFERENT pictures in general; accepting either can splice the wrong reference |
| "every failure lands in the typed Configure error channel, never a panic" | session.rs:25-26 | FALSE for fill_weights | debug-build panic on valid weighted streams (F-002) |
| "size-pinned frozen-ABI buffers" | session.rs:70-71 + ffi.rs:606-608 | SHALLOW | size pins pass while field layout diverges (F-003) |

## 7. Dead / duplicated / unreachable code
Whole lane unreachable (header). POC-MSB logic duplicated (F-005). `slice_type_code`/`va_picture_of`/`pic_num_of` are part1 (not re-audited, read for context only).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` at line 74 (module-level; ADR-001 E2 exception). Unsafe blocks in my range: 1514 (get_display_drm), 1530 (initialize), 1546/1552 (profile query), 1573/1577 (vendor CStr), 1227 (terminate in Drop) — each has a SAFETY comment; invariants are plausible and locally enforced (null checks, count clamp at 1555). FFI through `super::ffi` (A08) and `super::library`/`device` (A01). Linux-only via hw/mod.rs:94.

## 9. Open questions for the next agent
- **A08 (ffi.rs)**: verify `slice_data_bit_offset` width (u32 per published libva header?) and the u8 flag fields against the real libva 2.22.0 header; add field-offset pins, not just size pins (F-002/F-003).
- **A08 (session.rs part1)**: build_ref_list's two-form picNumX acceptance (245-261) can splice the wrong picture when both candidates exist — needs a conformance corpus or oracle differential, which the module admits never ran.
- **A03 (part4)**: does the decode loop validate per-slice `pic_parameter_set_id` against `pick_parameter_sets`'s single choice? Does `teardown_stream` handle a configure that failed between vaCreateConfig and vaCreateContext?
- REQUEST-ORCH-RUN: none (codecs lane is pure-read; the GPU path cannot run in this sandbox regardless).
