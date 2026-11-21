# AUDIT: crates/codecs/src/video/h264/hw/vaapi/session.rs#part2
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Chunk boundary audited | **lines 534-1067** (part2 of 4; aligns with W06-A10's part3 = 1068-1599 and W06-A03's part4 = 1600-2134; part1 = 1-533 W06-A08 — decode() STRADDLES the 533/534 boundary: its head 448-533 is part1, its tail 534-663 is here) |
| Lines / functions | 534 / 17 (decode tail, flush, size_of_u32, new, with_root, va_version, next_free_surface, ensure_display, ensure_profile, teardown_stream, create_stream_objects, push_buffer, picture_poc, ref_lists) |
| File verdict (this part) | UNREACHABLE |
| Reachable from shipped entry point? | NO — same chain as part3/part4: `VaApiSession` ← `VaApiBridge` ← `hw::negotiate_lane`/`H264Decoder::from_setting` ← **zero production callers** (rg over apps/ crates/: only `crates/codecs/tests/h264_hw_path.rs:861,897` and a comment at apps/cli/src/pipeline_host.rs:342); shipped lane is `"h264" \| "avc1" => H264SwDecoder::new` (pipeline_host.rs:346); product.toml:168 `hw-decode = false` + :191 `hw_decode_policy = "software-only"`; `#[cfg(target_os = "linux")]` (hw/mod.rs:94) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | rg "session.rs#part" in w06 tsvs (chunk ownership); read part3/part4 audit headers (boundary alignment); sed -n quote verification; rg frame_pool.rs MAX_SURFACES (=64, so the 16+6=22 surface count never clamps); baseline test log :579-591 (session tests executed); read of libva usage patterns for the data-buffer claim |

## 1. One-paragraph verdict
Part2 is the FFI heart of the dead VA-API lane: the decode-call driver sequence
(begin/config buffers/render/destroy/end/sync), device bring-up, profile
verification, object teardown, POC derivation and ref-list dispatch. The unsafe
calls each carry real SAFETY comments, every VA status is checked and mapped
typed, pool handles are released on every error path, and the buffer-destroy
loop runs even on render failure — the discipline the module promises. The
defects: every slice re-pushes the ENTIRE access-unit data buffer (N slices →
N full driver-side copies — the canonical libva/FFmpeg shape is ONE data
buffer shared by per-slice parameter buffers), `ensure_display` leaks the
`vaGetDisplayDRM` display when `vaInitialize` fails (never stored, never
terminated — Drop cannot clean it), `push_buffer` error paths abandon
already-created buffer ids until the next context teardown, and a comment
claims buffers are destroyed "after vaEndPicture" while the code destroys them
before it (the code follows FFmpeg's sanctioned order; the comment lies). All
of it unreachable in shipped builds; none of the driver path is executed
anywhere (module doc 5-16).

## 2. What actually works (verified)
— Error-path pool hygiene: every `?` return in decode's tail releases the
  acquired handle (`inspect_err(|_| { let _ = self.pool.release(handle); })`
  at 559-561, 569-571, 581-583, 591-593; explicit releases at 619, 625, 632,
  650) — read, exhaustive walk of every return path in 448-663.
— `ensure_display` stores `display`/`fd`/`lib` only AFTER `vaInitialize`
  succeeds (784-786), so `Drop` (part3 range) can never `vaTerminate` an
  uninitialized display — read; this closes the Drop-UB question I raised
  before reading the whole function.
— `ensure_profile` (792-838): both the profile list AND the entrypoint list
  are queried and VLD membership required, with `count.clamp(0, cap)` slicing
  — read.
— Executed in baseline: `session_configure_without_parameter_sets_is_typed`
  (test at 2090-2112, part4 range) drives MY `ensure_display` failure path via
  `with_root` + a /dev/dri-less temp root → typed `h264.hw.vaapi.*` Configure
  refusal — test-workspace.log.txt:591 ok.
  `va_api_lane_backend_label_is_stable` (2114-2121) executes my
  `VaApiSession::new` (681-704) — :591 group ok.

## 3. Findings

### F-001 [S3] Every slice pushes a full copy of the whole access-unit data buffer — N× driver-side memory amplification
Where: session.rs:573-593 (quote 584-590)
Code:
```rust
            self.push_buffer(
                &calls,
                &mut ids,
                ffi::VA_SLICE_DATA_BUFFER_TYPE,
                slice_data.as_ptr().cast(),
                u32::try_from(slice_data.len()).unwrap_or(u32::MAX),
            )
```
What is wrong: `slice_data` is the concatenation of ALL the picture's VCL NALs
(536); this loop runs once per slice and pushes the ENTIRE buffer each time.
A 16-slice picture allocates 16 driver-side copies of the picture's data. The
canonical libva pattern (libva-utils test_decode, FFmpeg's VA-API path) is ONE
VA_SLICE_DATA_BUFFER_TYPE buffer per picture with per-slice
VASliceParameterBufferH264 entries carrying `slice_data_offset` into it —
which this code already computes correctly (534).
Why it matters: memory and bandwidth amplification proportional to slice
count in the per-picture hot path; also relies on drivers associating each
slice param with the last-pushed data buffer — a usage-shape deviation from
the documented model. Unreachable lane today, so no user impact: S3.
What correct looks like: push the slice-data buffer once before the slice
loop (or per-slice buffers carrying only that slice's span with offset 0).
Confidence: MEDIUM-HIGH — Verification: read; libva usage pattern from
libva-utils/FFmpeg memory (no driver in sandbox to execute).

### F-002 [S4] Comment/code mismatch: buffers destroyed BEFORE vaEndPicture while the comment claims the ABI requires after
Where: session.rs:610-623
Code:
```rust
        // Buffers are destroyable after vaEndPicture per the ABI;
        // destroy them regardless of the render outcome (no leak on
        // the error path).
        for id in ids {
```
```rust
        // SAFETY: live context.
        let status = unsafe { (calls.end_picture)(self.display, self.context) };
```
What is wrong: the destroy loop (613-617) runs before `end_picture` (623),
contradicting the comment. The CODE follows FFmpeg's established order
(render → destroy buffers → end picture); the COMMENT asserts the opposite
constraint.
Why it matters: a future maintainer "fixing" the code to match the comment
would reorder a correct sequence; the safety rationale documented is false.
What correct looks like: fix the comment ("FFmpeg order: destroy after
render, before end — vaCreateBuffer copies the payload").
Confidence: HIGH — Verification: read.

### F-003 [S4] push_buffer error paths abandon already-created buffer ids
Where: session.rs:552-593 (pattern; quote 552-561)
Code:
```rust
        self.push_buffer(
            &calls,
            &mut ids,
            ffi::VA_PICTURE_PARAMETER_BUFFER_TYPE,
            std::ptr::from_ref(&pic_param).cast(),
            size_of_u32::<VAPictureParameterBufferH264>(),
        )
        .inspect_err(|_| {
            let _ = self.pool.release(handle);
        })?;
```
What is wrong: when a mid-sequence `push_buffer` fails (say the IQ-matrix
create after the picture-parameter create succeeded), the `?` returns with
`ids` containing the already-created buffer ids — never `vaDestroyBuffer`ed.
The pool handle is released, but the driver-side buffers persist until the
context is destroyed (`teardown_stream`, 841-868, at re-configure or Drop).
Why it matters: a repeated failing create (flaky driver) accumulates
driver-side buffers within a live session; bounded only by teardown. Failure
path of an unreachable lane: S4.
What correct looks like: on any push error, destroy the ids collected so far
(the same loop as 613-617) before returning.
Confidence: HIGH — Verification: read of all three push sites.

### F-004 [S4] ensure_display leaks the VADisplay when vaInitialize fails
Where: session.rs:769-782
Code:
```rust
        let status = unsafe { (lib.initialize)(display, &mut major, &mut minor) };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.initialize", status));
        }
```
What is wrong: `vaGetDisplayDRM` (769) has already minted the display; on
initialize failure the function returns without `vaTerminate(display)` and
without storing it (stored only at 784) — so neither this path nor `Drop`
(part3) can release it. Each configure retry on a half-working driver leaks
one driver display context (the fd IS closed: local `fd` drops). Note the
sibling `probe_with` (part3 range) has the mirrored leak on its own
initialize-failure path — the two bring-up routines repeat the same shape.
Why it matters: repeated-probe/_configure leak on broken drivers; free fix.
What correct looks like: `vaTerminate(display)` before the error return.
Confidence: MEDIUM — Verification: read; libva's terminate-after-failed-init
expectation from libva docs (not executable here).
Severity: S4.

### F-005 [S4] Silent `u32::MAX` saturations and the max_entries clamp accept lies rather than refusing typed
Where: session.rs:524 and 534-535
Code:
```rust
        let max_entries = usize::from(sps.max_num_ref_frames.max(1)).min(VA_REF_LIST_SLOTS);
```
```rust
            let offset = u32::try_from(slice_data.len()).unwrap_or(u32::MAX);
            let data_size = u32::try_from(unit.nal.len()).unwrap_or(u32::MAX);
```
What is wrong: (a) the ref-list capacity is clamped to
`sps.max_num_ref_frames`, but the slice's `num_ref_idx_l0_active_minus1`
(passed verbatim in `slice_parameter`, part3 range) can legally declare more
active refs than that — the driver then sees an active count larger than the
list content actually built; nothing refuses the mismatch typed. (b) the
`unwrap_or(u32::MAX)` casts silently corrupt offset/size on >4 GiB inputs
instead of a typed refusal (unreachable in practice — S4 hygiene; the
file's own error discipline asks for typed everywhere).
What correct looks like: `num_ref_idx_active > max_entries` → typed
`h264.hw.vaapi.ref_list` refusal; `try_from(...)?` mapped to a typed
Truncated-class error.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| H264HwDecoder::decode (tail) | 534-663 | REAL | S3 | ref-list build per slice; per-slice data-buffer duplication (F-001); destroy-before-end order (F-002, comment-only defect); update_dpb called once per AU with the FIRST VCL's header (638-646) — correct per §7.4 (all slices of a picture share marking); emit + typed pool_exhausted; sync decode |
| H264HwDecoder::flush | 665-670 | REAL | — | returns empty — provably correct given synchronous vaSyncSurface per picture; contract-documented |
| size_of_u32 | 674-676 | REAL | — | trivial; u32::MAX saturation unreachable |
| VaApiSession::new | 681-704 | REAL | — | null/invalid-id defaults; executed by backend-label test |
| VaApiSession::with_root | 708-715 | REAL | — | cfg(test) seam; E0509 note honest |
| VaApiSession::va_version | 720-722 | REAL | — | accessor |
| VaApiSession::next_free_surface | 728-741 | REAL | — | bounded scan, round-robin, DPB-aware; None → typed exhaustion; surfaces count ≤ 22 < MAX_SURFACES=64 so no clamp starvation (frame_pool.rs:47) |
| VaApiSession::ensure_display | 745-788 | REAL | S4 | idempotent via lib.is_some; display stored only post-init ✓ (Drop-safe); init-failure display leak (F-004); executed in failure path by baseline test :591 |
| VaApiSession::ensure_profile | 792-838 | REAL | — | profile + VLD entrypoint both verified, count clamped; untrusted-driver cap allocation noted (trusted OS service) |
| VaApiSession::teardown_stream | 841-868 | REAL | — | context→config→surfaces order ✓; guards on INVALID_ID; pool.drain; reclaims F-003's leaked ids |
| VaApiSession::create_stream_objects | 872-953 | REAL | — | RT_FORMAT_YUV420 correct for served profiles (100=High is 8-bit 4:2:0); count 4..=22 within pool cap; partial-create error path leaves locals to config-teardown reclamation (S4-class edge, noted) |
| VaApiSession::push_buffer | 957-986 | REAL | S4 | num_elements=1 full-size form ✓; VA_INVALID_ID check ✓; caller-site id leak (F-003) |
| VaApiSession::picture_poc | 990-1035 | REAL | — | type0 §8.2.1.1 wrap with CHECKED i64 + typed overflow; type2 delegated; DUPLICATED with update_dpb's copy in part3 (W06-A10's drift finding — cross-ref, not re-filed); IDR lsb=0 assumption depends on parser (cross-agent) |
| VaApiSession::ref_lists | 1039-1067 | REAL | — | I/Si empty, B both, P/Sp l0 per §8.2.4 ✓; `map_or(16, …)` fallback dead (decode guaranteed sps) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| session_configure_without_parameter_sets_is_typed (2090-2112, part4 range; drives MY ensure_display) | Yes for the device_open failure mapping | temp-root bring-up fails typed; executed baseline :591 |
| va_api_lane_backend_label_is_stable (2114-2121; drives MY new()) | Yes trivially | label equality |
| NOTHING tests decode's driver sequence, create_stream_objects, teardown, push_buffer, picture_poc, ref_lists | No | module doc admits compile-verified only (5-16); the pure helpers they call (build_ref_list, picture_parameter, slice_parameter) are golden-tested in part3/part4 ranges — coverage lives in siblings, not here |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified + unit-verified in its pure parts only; runtime hardware behavior NOT exercised — [VERIFY]" | session.rs:5-16 | REAL | matches this chunk exactly: pure parts tested elsewhere, FFI sequence never executed |
| "Buffers are destroyable after vaEndPicture per the ABI" | session.rs:610-611 | FALSE (comment) | code destroys at 613-617 before end_picture at 623 (F-002); FFmpeg's sanctioned order is what the code does |
| "vaCreateBuffer copies the data into driver-owned storage, so no aliasing survives" | session.rs:966-969 | REAL-UNPROVEN | standard libva semantics; no driver run here |

## 7. Dead / duplicated / unreachable code
- Whole chunk UNREACHABLE from shipped entry points (chain in header;
  identical to part3/part4 conclusions — the four chunks agree).
- `with_root` is cfg(test)-only by design (test seam, documented).
- ref_lists' `map_or(16, …)` fallback is dead given decode's sps guard.
- POC type0 wrap arithmetic exists twice in the file (picture_poc here,
  update_dpb in part3) — W06-A10 filed the drift; not re-filed.

## 8. Dependencies & flags
- `#[cfg(target_os = "linux")]` on the vaapi module (hw/mod.rs:94); compiled
  into shipped Linux builds via unconditional `codecs` dependency; called by
  nobody. Module-level `#![allow(unsafe_code)]` (session.rs:74) is the
  ADR-001 audited exception; every unsafe site in this chunk carries a
  SAFETY comment (verified by read of 497, 595, 614, 622, 629, 766, 777, 799,
  807, 816, 823, 845, 854, 893, 911, 933, 966).

## 9. Open questions for the next agent
- part1 (W06-A08): does `configure` re-run `ensure_display` after a failed
  first configure (each retry re-dlopens libva — refcount growth), and does
  it store `self.sps/self.pps/display_size` from the CROPPED size?
- Parser wave: for IDR slices, is `pic_order: Type0 { lsb: 0, .. }`
  guaranteed (picture_poc's IDR POC=0 depends on lsb==0 there)?
- If the lane is ever enabled: F-001 (memory) and part3's F-001
  (fill_weights panic/corruption) and the ffi u16 bit-offset ABI mismatch
  (W06-A10's S1) all need fixing first.
