# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/session.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 962 / 27 (incl. 2 trait impls, Drop, Default) |
| File verdict | UNREACHABLE (real code; cfg(macos) — not compiled by the shipped Linux release lane; additionally behind `features.hw-decode = false` + `playback.hw-decode-policy = "software-only"` defaults) |
| Reachable from shipped entry point? | NO. hw/mod.rs:94-97 gates: `#[cfg(target_os = "linux")] pub mod vaapi;` / `#[cfg(target_os = "macos")] pub mod videotoolbox;`. Release lane (baseline, Linux) never compiles this file. On macOS it would be reached only via `decode::H264Decoder::from_setting` with hw-decode enabled (default OFF per hw/mod.rs:41-49). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 3 |
| Commands run | rg unsafe inventory (44 unsafe items); rg field-usage; sed quote verification; rg baseline logs; rustup target list --installed -> "command not found" |

## 1. One-paragraph verdict
This is a complete, seriously-written VideoToolbox H.264 session (bring-up, decode, flush, teardown, probe) — no stubs, no fake successes — and its honesty labels are unusually candid: the module doc itself states nothing here was ever executed (session.rs:5-17 "compile-verified only … runtime hardware behavior NOT exercised"). But it is dead weight in every build this repo ships: cfg(macos)-gated out of the Linux release lane, and behind an OFF-by-default settings flag even on macOS. Two S2 defects sit inside it: the lane can never decode more than `pool_capacity` frames because no present-side release path exists (every surface is retained until teardown, so drain() hits `pool_exhausted` and drops frames); and the CFDictionary NULL-callbacks "documented" claim is unverifiable and, if wrong, is a use-after-free handed to VTDecompressionSessionCreate. The compile-verification claim itself cannot be reproduced (no darwin target in this sandbox) and three never-read struct fields would trip dead_code lints on any warning-clean check.

## 2. What actually works (verified)
Nothing in this file can execute on this host (cfg(macos)). Verified indirectly:
- The pure halves the session rides are host-tested and green: audit/runs/baseline/cargo-test-workspace.txt:654-659 (`feed::tests::avcc_sample_prefixes_every_nal_with_be32_lengths`, `stamp_nanos_round_trips_through_the_nanosecond_base`, `avcc_length_prefix_refuses_the_u32_overflow_never_truncates` all ok).
- The bounded pool this session allocates from is host-tested green: frame_pool.rs tests + baseline `h264_hw_path` 20/20 ok (cargo-test-workspace.txt:1127-1140).
- cfg gating verified: `sed -n '94,97p' crates/codecs/src/video/h264/hw/mod.rs`.

## 3. Findings
F-001 [S2]
Where: session.rs:481-483, 577-630; frame_pool.rs:26-31
Code:
```
    /// Handles handed to the codec layer (the present side's
    /// release hook is later-wave work; teardown walks this list).
    emitted: Vec<u64>,
```
What is wrong: there is no release path. `H264HwFrame.surface` handles go up as `FrameStorage::OpaqueHandle(frame.surface)` (decode.rs:466) and nothing anywhere calls back to free pool slots (rg over h264/: no `release_surface`, no consumer of `HwSurfacePool::release` outside tests/teardown). Once `emitted.len() == pool_capacity` (SPS `max_num_ref_frames + 6`, clamped 4..=64), every subsequent `drain()` enters the rollback arm and releases all newly decoded buffers:
```
                    return Err(H264HwError::Decode {
                        code: "h264.hw.videotoolbox.pool_exhausted",
                    });
```
Why it matters: a real macOS playback session stalls permanently after ~10-22 frames, and the frames decoded in the failing drain are destroyed (released), i.e. silent frame loss beyond the typed error. What correct looks like: the present-side release hook the code itself calls "[PROPOSAL]" (frame_pool.rs:28-31) — until it lands the lane is a demo, not a decoder. Confidence: HIGH — Verification: read (call-graph by rg; no release caller exists).
Mitigation: honestly documented; lane unreachable in shipped Linux build.

F-002 [S2]
Where: session.rs:211-223 (and 267-298)
Code:
```
unsafe fn new_dictionary(capacity: isize) -> Option<ffi::CFMutableDictionaryRef> {
    // SAFETY: the documented NULL-callbacks form selects the
    // kCFType callbacks; capacity is the caller's entry count.
    let dict = unsafe {
        ffi::CFDictionaryCreateMutable(
            std::ptr::null(),
            capacity,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
```
What is wrong: CoreFoundation's documented default-callbacks form is passing a pointer to `kCFTypeDictionaryKeyCallBacks`/`kCFTypeDictionaryValueCallBacks`; passing NULL is not documented as equivalent ("If this parameter is not a valid …CallBacks … the behavior is undefined" is the CF documentation posture). If NULL does not select kCFType callbacks, the dictionary does NOT retain the keys/values — and `set_true`/`set_i64` then `CFRelease` their Create-rule references immediately after `CFDictionarySetValue` (session.rs:274-275, 293-295), leaving dangling `CFString`/`CFNumber` pointers inside the spec/destination dictionaries that `VTDecompressionSessionCreate` subsequently reads. Why it matters: potential use-after-free on the lane's only bring-up path; the SAFETY comment asserts documentation it cannot cite (no macOS SDK in this sandbox — `[VERIFY]` is the module's own label). What correct looks like: declare the two `kCFType…CallBacks` constants in ffi.rs (they are exported symbols, linkable like `kCFBooleanTrue`) and pass them. Confidence: MEDIUM — Verification: UNVERIFIED (no macOS SDK; cannot diff against headers or run).
Note: the same NULL-vs-default question applies to every `std::ptr::null()` allocator argument — those ARE documented as "the default allocator", so only the callback NULLs are at risk.

F-003 [S3]
Where: session.rs:484-485, 486-488, 491-492, 756-759
Code:
```
    /// Coded dimensions of the current stream.
    coded: (u32, u32),
```
What is wrong: `coded`, `display_size`, and `pps` are written (configure:756-759, teardown:567-569) and never read anywhere (rg `self\.coded|self\.display_size|self\.pps` → assignments only). rustc's dead_code lint fires on never-read private fields, so any genuinely warning-clean `cargo check --target aarch64-apple-darwin` would have flagged them. The module's central claim — "compile-verified only, via `cargo check --target aarch64-apple-darwin`" (session.rs:8-9) — has no captured log in-tree (rg over docs/gates/logs, audit/runs/baseline: none; phase7-exit.md:227 calls it "lane evidence, cited"), and is not reproducible here (no rustup, no darwin target). Why it matters: the lane's only verification claim is unfalsifiable and the dead fields are evidence it was not warning-clean when last run (or never ran against this code). What correct looks like: capture the check output under docs/gates/logs/ or delete the dead fields. Confidence: HIGH — Verification: read + rg.
Severity note: raised to S3 because the repo's honesty story for this lane rests on that unverifiable compile claim.

F-004 [S3]
Where: session.rs:399-405, 417, 721, 530-535
Code:
```
/// The session invalidates the VT session in its `Drop` **before**
/// the `Arc<Shared>` drops, and
/// `VTDecompressionSessionInvalidate` is documented synchronous
/// with the last output callback — so `refcon` is always a live
/// `&Shared` here.
```
What is wrong: the entire refcon-lifetime safety argument (callback dereferencing `refcon.cast::<Shared>()` at :417, refcon minted via `Arc::as_ptr(&self.shared)` at :721) hangs on Invalidate's synchrony with in-flight callbacks, asserted here as "documented" but not checkable in this sandbox (no VideoToolbox headers). The Rust-side ordering itself is correct (Drop::drop → teardown → Invalidate+CFRelease before the `shared` field drops; field order independent since teardown nulls the handles first). Why it matters: if Invalidate ever returns while a callback is still executing on a VT worker thread, the callback races the Arc drop and the `queue` Mutex destroy — a use-after-free. What correct looks like: the claim should carry a `[VERIFY: header citation]` like the string constants do, not a flat "is documented". Confidence: MEDIUM — Verification: UNVERIFIED (no SDK).

F-005 [S4]
Where: session.rs:135-146
Code:
```
/// Coded (macroblock-aligned) size — ... `None` on arithmetic overflow —
/// typed upstream, never a truncating cast.
fn coded_size(sps: &SeqParameterSet) -> Option<(u32, u32)> {
```
What is wrong: only the `checked_add(1)` is checked; the `* 16` is a plain multiply. The doc's "None on arithmetic overflow" is only true because the landed parser separately bounds both fields to ≤65535 (sps.rs:276-277), making (65535+1)*16 = 2^20 unrepresentable of overflow. The safety is incidental (lives in another file), not in this function. What correct looks like: checked_mul, or a comment citing the parser bound. Confidence: HIGH — Verification: read (sps.rs:272-283 quoted).

F-006 [S4]
Where: session.rs:699-716
Code:
```
        let Some(attributes) = destination_attributes(width, height) else {
            // SAFETY: Create-rule handles, sole owner (the spec
            // dictionary was not passed to any framework call yet).
            unsafe {
                ffi::CFRelease(spec.cast());
                ffi::CFRelease(format_description.cast());
            }
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.specification",
            });
```
What is wrong: destination-attribute failure reports the same code (`...specification`) as decoder-specification failure at :703 — two distinct bring-up steps are indistinguishable in diagnostics. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: session.rs:608-610 vs frame_pool.rs:61-81
What is wrong: the typed `PoolExhausted` struct (capacity/emitted diagnostics, Display impl) is constructed by `pool.acquire`, then discarded here and flattened to the string `"h264.hw.videotoolbox.pool_exhausted"` — the seam has a typed condition and this lane erases it (stringly-typed error channel). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vt_serves | 127-133 | REAL-UNPROVEN | — | pure; profile/chroma/bitdepth gates; profile 88 (Extended) served — plausibly OK, Apple exposes Extended profile levels [VERIFY] |
| coded_size | 139-146 | REAL-UNPROVEN | S4 | F-005 |
| pool_capacity | 151-153 | REAL-UNPROVEN | — | clamp 4..=64, `usize::from(u8)` safe |
| cm_time_of | 159-174 | REAL-UNPROVEN | — | round-trip halves host-tested (feed tests, baseline) |
| stamp_of | 179-185 | REAL-UNPROVEN | — | invalid-CMTime → Missing, no guesses |
| pick_parameter_sets | 190-200 | REAL-UNPROVEN | — | lowest-PPS-id → matching SPS |
| new_dictionary | 211-223 | REAL-UNPROVEN | S2 | F-002 NULL callbacks |
| cf_string | 229-248 | REAL-UNPROVEN | — | 64-byte NUL-terminated buffer; interior-NUL refusal |
| cf_number_i64 | 251-263 | REAL-UNPROVEN | — | `&raw const` on local i64 |
| set_true / set_i64 | 267-298 | REAL-UNPROVEN | S2* | release-after-SetValue is only sound if F-002 resolves as claimed |
| decoder_specification | 303-314 | REAL-UNPROVEN | — | RequireHardwareDecode=true, honest |
| destination_attributes | 319-336 | REAL-UNPROVEN | S4 | F-006 code collision |
| release_buffer | 341-347 | REAL-UNPROVEN | — | null-tolerant |
| impl Send CallbackFrame | 372 | REAL-UNPROVEN | — | CF refcount atomic — sound |
| Shared::empty | 385-390 | REAL-UNPROVEN | — | |
| on_frame | 406-454 | REAL-UNPROVEN | S3* | F-004 refcon; panic-free arm (poisoned lock) verified by read; nulls checked |
| VideoToolboxSession::new | 510-523 | REAL-UNPROVEN | — | unopened mint, pool(0) |
| teardown | 531-570 | REAL-UNPROVEN | — | exactly-once release bookkeeping verified by read: queued + emitted + pool.drain |
| drain | 577-630 | REAL-UNPROVEN | S2 | F-001 exhaustion drops frames |
| Default / Drop | 633-643 | REAL-UNPROVEN | — | Drop → teardown |
| backend_id | 646-648 | REAL-UNPROVEN | — | |
| configure | 650-762 | REAL-UNPROVEN | — | typed refusals ordered before teardown; CFRelease on every early-exit path verified by read (steps 4,5,6 each release) |
| decode | 764-906 | REAL-UNPROVEN | — | typed refusals (mvc/field/sample); block-buffer copy; releases sample after submit; scratch reuse |
| flush | 908-923 | REAL-UNPROVEN | — | WaitForAsynchronousFrames then drain; unopened → Ok(empty) |
| probe | 938-962 | REAL-UNPROVEN | — | VTIsHardwareDecodeSupported; no→DriverIncompatible typed |

## 5. Test honesty
No tests in this file — module doc justifies why (a `#[cfg(test)]` inside `#[cfg(target_os = "macos")]` never compiles in the Linux gate; session.rs:14-17). Justification is valid. The host-tested substitutes (feed.rs round trips, frame_pool.rs lifecycle, h264_hw_path.rs seam tests) exercise everything EXCEPT this session's own logic — which remains 0% covered. Mutation reasoning: if `on_frame` pushed garbage or `drain()` double-released, no test in the repo would fail.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified only, via cargo check --target aarch64-apple-darwin" | session.rs:8-9, videotoolbox/mod.rs:46, docs/gates/phase7-exit.md:227 | UNVERIFIED | no captured log in-tree; no darwin target in sandbox (rustup absent); dead fields (F-003) argue against warning-clean run |
| "runtime hardware behavior NOT exercised — [VERIFY]" | session.rs:10 | REAL (honest) | no execution path on Linux; baseline tests contain zero videotoolbox symbol (rg -c → 0 matches) |
| "A re-configure tears the session down and rebuilds it" | session.rs:28-29 | REAL-UNPROVEN | read: configure:672 calls teardown() before rebuild |
| "surfaces beyond the pool cap … typed … never an unbounded retain" | session.rs:67 | REAL-UNPROVEN | read: drain rollback; but see F-001 (frames dropped, lane stalls) |
| pool "bounded by construction … never grows past one pipeline latency's worth" | session.rs:374-378 | REAL-UNPROVEN | queue only bounded by submitted-not-yet-drained frames; VT internal queue limits not verified [VERIFY] |

## 7. Dead / duplicated / unreachable code
- Fields `coded` (756), `display_size` (757), `pps` (759): written, never read (F-003).
- `emitted.clear()` at :760 is redundant after teardown drained it (harmless).
- Whole file unreachable from shipped Linux build (cfg macos, hw-decode OFF default).

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` module-scoped (session.rs:79) with ADR-001 §5 justification — the audited-exception pattern; 44 unsafe items inventoried, every one carries a `// SAFETY:` comment (verified by full read).
- No new dependencies; hand-rolled FFI (videotoolbox/ffi.rs — another agent's file; its constant values (KCV_PIXEL_FORMAT_420V, KCM_VIDEO_CODEC_TYPE_H264, KVT_DECODE_FRAME_ENABLE_ASYNC) are load-bearing for this session and carry their own [VERIFY] labels).
- Gating: `#[cfg(target_os = "macos")]` (hw/mod.rs:96).

## 9. Open questions for the next agent
- videotoolbox/ffi.rs auditor: confirm the CF constant values and whether kCFType callback structs should be declared to fix F-002.
- Whoever audits h264/decode.rs: confirm no present-side release path exists (F-001) — I found none.
- sps.rs auditor: `r.ue()? as u32` at sps.rs:274-275 truncates a ue() > u32::MAX BEFORE the ≤65535 check (a 2^32+5 width parses as 5) — parser-level truncation, out of my scope.
- Is there any CI lane that ever builds darwin? .github/workflows/release.yml:430 shows the lipo step COMMENTED OUT — macOS artifact lane appears disabled.
