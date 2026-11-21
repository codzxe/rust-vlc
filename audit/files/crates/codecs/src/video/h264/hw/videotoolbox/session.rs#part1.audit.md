# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/session.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 1-480 of 963 (chunk part1 of 2; boundary per W06-A07's part2 header: part1 = :1-480 — module docs, pure helpers, CF helpers, callback channel, struct head; part2/A07 = :481-962) / 16 units: 6 consts, vt_serves, coded_size, pool_capacity, cm_time_of, stamp_of, pick_parameter_sets, new_dictionary, cf_string, cf_number_i64, set_true, set_i64, decoder_specification, destination_attributes, release_buffer, CallbackFrame(+Send), Shared, on_frame, struct head |
| File verdict | UNREACHABLE / REAL-UNPROVEN (compiles only on macOS targets — not even compiled by any baseline run; every pure helper unexecuted anywhere) |
| Reachable from shipped entry point? | NO. `#[cfg(target_os = "macos")]` (hw/mod.rs:96-97); `VideoToolboxSession` ← `VideoToolboxBridge` (hw/mod.rs:459-464) ← `H264Decoder::from_setting` ← zero production callers (Grep crates/+apps/: tests + one comment at pipeline_host.rs:342). Shipped lane: `H264SwDecoder` (pipeline_host.rs:346); product.toml:168 `hw-decode = false`, :191 `hw_decode_policy = "software-only"`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read 1-963 (full file — part2's drain/decode/flush/probe read for boundary-claim verification of my part1 findings); Grep function map; Grep `pic_width_in_mbs_minus1` parser/sps.rs:274-283 (bounds my F-002); baseline rg: `hw::feed` tests :557-562 (the OS-neutral halves' execution evidence); cross-read audit/files/.../videotoolbox/{ffi.rs,mod.rs,session.rs#part2}.audit.md (W06-A05/A07 — boundary + non-duplication check). |

## 1. One-paragraph verdict
Part1 is the macOS lane's CF-object plumbing, pure helpers, and the asynchronous output callback — none of which is compiled on this host (the whole module is `cfg(target_os = "macos")`), none of which has a single executed test anywhere (the file honestly explains why it carries no test module), and whose "compile-verified via cargo check --target aarch64-apple-darwin" claim has no committed artifact (W06-A05/A07 already established that gap for the lane; the committed darwin crosscheck covers only platform-macos). Statically, the CF Create/Release discipline is correct (I traced every retain/release pair), the four `KEY_*` CFString contents match the real Apple header values per my domain knowledge, and the refcon lifetime argument holds against the Drop order I verified in part2. One genuine behavioral defect: the output callback discards per-frame decode failures while claiming the decode call's status carries them — false in the asynchronous submission mode this lane uses — so damaged frames vanish with no damage count, breaking the seam's own damage-accounting contract. One hygiene defect: an unchecked `* 16` that is safe only via an unstated parser bound, contradicting its VA-API twin's checked math.

## 2. What actually works (verified)
— CF retain/release discipline traced by hand, all Create-rule objects: `cf_string` releases its key after `SetValue` (275, 294-295); `set_i64` releases both objects and handles the partial-failure order (289-292); `decoder_specification`/`destination_attributes` release the dictionary on helper failure (309-311, 331-333); `decode` (part2) releases block/sample at each stage — no leak path found in part1's helpers.
— KEY_* contents verified against auditor knowledge of the public Apple headers: `kVTVideoDecoderSpecification_RequireHardwareDecode` = CFSTR("RequireHardwareDecode"), `kCVPixelBufferPixelFormatTypeKey` = CFSTR("PixelFormatType"), `kCVPixelBufferWidthKey`/`HeightKey` = CFSTR("Width")/CFSTR("Height") — all four match (UNVERIFIABLE in sandbox: no SDK; the `[VERIFY]` labels at :108/111/113/115 are honest).
— The OS-neutral halves this file leans on ARE host-executed: `hw::feed` tests 6/6 ok (baseline test-workspace.log.txt:557-562 — AVCC be32 prefixing, u32-overflow refusal, stamp_nanos round trip).
— refcon lifetime: verified against part2's Drop path — `Drop::drop` (639-643) → `teardown` invalidates the VT session (535) before `CFRelease` (537) and before the `Arc<Shared>` field drops; `on_frame`'s deref (417) is sound for every in-flight callback (claims VT's invalidate-is-synchronous-with-last-callback semantics — UNVERIFIABLE here, no Apple docs in sandbox).
— `on_frame`'s Mutex poisoning handling (449-452): poison-tolerant `into_inner` keeps the extern "C" boundary panic-free — correct (unwinding across extern "C" is an abort).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:395-397 and 418-420 (consumed by decode's async submit at :886-903, part2)
Code:
```
/// Failure frames (`status != noErr`) surface nothing — the decode
/// call's own status carries that truth upstream; a null image
/// buffer equally so.
```
```
    if status != ffi::NO_ERR || image_buffer.is_null() {
        return;
    }
```
What is wrong: In the submission mode this lane uses (`VTDecompressionSessionDecodeFrame(..., KVT_DECODE_FRAME_ENABLE_ASYNC, ...)`, :886-894), the function's return value is the *submission* status; per-frame decode failures (malformed bitstream, decoder malfunction) arrive ONLY as the callback's `status` — which this callback drops on the floor. So the comment's justification is false for the mode chosen, and a damaged frame vanishes with no error, no damage count, and no surfaced picture: `decode` returns whatever the drain found (possibly `Ok` with 0 frames).
Why it matters: The seam's contract this session implements says in-stream damage is "skipped-and-counted recovery … never a panic" and the codec layer "counts damage and continues" (hw/mod.rs:47-49, 298-299). The VA-API lane counts (its decode errors are typed per-op); this lane skips without counting — the engine cannot distinguish "not ready yet" from "frame destroyed by damage", and a corrupt stream plays as silent frame loss with zero diagnostics. Dead lane today (no shipped caller), so S3 not S2.
What correct looks like: Push a failure record (or atomic damage counter) into `Shared` when `status != noErr`; have `drain` fold it into the returned `H264HwDecoded`/typed `Decode` error the way the VA-API lane's `va_op_error` does; fix the comment.
Confidence: HIGH on the code behavior (read); MEDIUM on the exact VT status-propagation split (auditor domain knowledge of VideoToolbox; no SDK in sandbox to quote the header — the async-callback-carries-decode-status part is UNVERIFIED-provenance but high-confidence).
### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:143-145
Code:
```
    let width = (sps.pic_width_in_mbs_minus1.checked_add(1)?) * 16;
    let height = (sps.pic_height_in_map_units_minus1.checked_add(1)?) * 16;
    Some((width, height))
```
What is wrong: The `* 16` multiplications are unchecked — the doc comment promises "None on arithmetic overflow — typed upstream, never a truncating cast" (:137-138), but an SPS with `pic_width_in_mbs_minus1 ≥ 2^27` would wrap (release) or panic (debug). The check passes today ONLY because the parser bounds both fields to 65,535 (h264/parser/sps.rs:276-277) — an invariant this file neither states nor checks, and one the VA-API twin does not rely on (vaapi/session.rs:319-329 uses `checked_mul` throughout).
Why it matters: Same-file inconsistency with the documented promise; a future parser-bound relaxation silently converts this into an overflow bug with no test to catch it (nothing in this lane compiles on the CI host).
What correct looks like: `checked_mul(16)?` — mirrors the VA-API twin exactly.
Confidence: HIGH — Verification: read both twins; parser bound confirmed at sps.rs:276-283.

## 4. Per-function table (part1 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + consts | 1-116 | REAL-UNPROVEN | — | Honest [VERIFY] posture; SURFACES_BEYOND_DPB/MIN_SURFACES mirror the VA-API lane; KEY_* contents verified-by-knowledge (§2). |
| `vt_serves` | 127-133 | REAL-UNPROVEN | — | Profiles 66/77/88/100 + 4:2:0 8-bit + !separate_colour_plane — matches the typed-refusal table; `chroma_format_idc.unwrap_or(1)` correct for the implied-4:2:0 profiles. Zero tests. |
| `coded_size` | 139-146 | REAL-UNPROVEN | S4 | F-002; `!frame_mbs_only → None` doubles as the field-picture refusal's arithmetic twin. |
| `pool_capacity` | 151-153 | REAL-UNPROVEN | — | `(refs+6).clamp(4, 64)` — bounded by construction; max_num_ref_frames is u8 so no overflow. |
| `cm_time_of` | 159-174 | REAL-UNPROVEN | — | nanos@10⁹ + VALID flag; Missing → all-zero invalid CMTime. Round-trip base host-tested in feed (:562), these wrappers not. |
| `stamp_of` | 179-185 | REAL-UNPROVEN | — | Strict inverse (flags AND timescale) — a VT-returned rescaled time degrades to Missing, never a guess. Sound. |
| `pick_parameter_sets` | 190-200 | REAL-UNPROVEN | — | Lowest-PPS-id → its SPS; missing SPS → None → typed refusal. Same convention as the VA-API lane. |
| `new_dictionary` | 211-223 | REAL-UNPROVEN | — | NULL-callbacks form = kCFType callbacks; null-tolerant result. Correct per CF docs (knowledge). |
| `cf_string` | 229-248 | REAL-UNPROVEN | — | 64-byte stack buffer; rejects overlong + interior-NUL — the four ASCII keys can never trip it; honest None arm. |
| `cf_number_i64` | 251-263 | REAL-UNPROVEN | — | SInt64 type; `&raw const` — correct. |
| `set_true`/`set_i64` | 267-298 | REAL-UNPROVEN | — | Retain/release pairs traced — correct (§2). |
| `decoder_specification` | 303-314 | REAL-UNPROVEN | — | 1 entry (RequireHardwareDecode=true) — the ADR-001 no-silent-software guarantee. |
| `destination_attributes` | 319-336 | REAL-UNPROVEN | — | '420v' (VideoRange NV12) + coded dims; short-circuit && keeps partial failure consistent; releases on failure. Coded-vs-display size trap is part2's finding (A07 F-002), cross-noted. |
| `release_buffer` | 341-347 | REAL-UNPROVEN | — | Null-tolerant exactly-once release helper. |
| `CallbackFrame` + `unsafe impl Send` | 355-372 | REAL-UNPROVEN | — | CF refcount-only usage across threads — sound; pixel data never touched by this lane. |
| `Shared` | 379-390 | REAL-UNPROVEN | — | `Mutex<Vec<CallbackFrame>>`; unbounded Vec — bounded-in-practice by per-call drain (verified against part2's drain: all-or-nothing, :577-630) and submission rate; claim "never grows past one pipeline latency" plausible but untested. |
| `on_frame` | 406-454 | REAL-UNPROVEN | S3 | F-001 (swallowed decode failures). Retain-once/push-once discipline correct; poison-tolerant lock; `u32::try_from` clamps on absurd buffers. |
| struct head (`VideoToolboxSession` fields 1-4) | 467-480 | REAL-UNPROVEN | — | Field docs honest (Create-rule; "present side's release hook is later-wave work" at :481-483 — the structural cap A07 filed as their pool finding). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no tests in this file — documented choice) | — | :15-17: a `#[cfg(test)]` module here would never run in the Linux gate, so none is written. The tradeoff is real, but the OS-neutral pure helpers (vt_serves, coded_size, pool_capacity, cm_time_of/stamp_of, pick_parameter_sets) could be host-tested if extracted like `feed.rs` was — feed's tests (:557-562) prove the pattern works. Today: zero executed evidence for ANY function in this file, including the pure ones. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified only, via `cargo check --target aarch64-apple-darwin`" | session.rs:8-9 | REAL-UNPROVEN | No committed artifact covers the codecs crate (W06-A05/A07 established; orchrun-platform-macos-crosscheck.log.txt covers platform-macos only). Not compiled by any baseline run (cfg macos). |
| "the host-testable pure halves … live in feed … those run in this host's suite" | session.rs:12-15 | REAL | baseline :557-562, 6/6 ok. |
| "RequireHardwareDecode = true … A machine without a hardware VT decoder therefore fails session creation — never a silent software fallback" | session.rs:48-57 | REAL-UNPROVEN | decoder_specification (:303-314) does set it; whether VT honors it as a hard requirement is Apple-docs knowledge (plausible, UNVERIFIED in sandbox). |
| "Failure frames surface nothing — the decode call's own status carries that truth upstream" | session.rs:395-397 | FALSE | F-001: with the async flag (:890) the submission status does not carry per-frame decode errors. |
| KEY_* contents "per VideoToolbox's public header docs" | session.rs:106-116 | REAL (by knowledge) | All four CFSTR contents match auditor knowledge of the Apple headers; no SDK to quote — [VERIFY] labels honest. |

## 7. Dead / duplicated / unreachable code
- Entire part1 unreachable from shipped entry points (header table); not even compiled on Linux/Windows builds.
- Duplication with the VA-API lane: SURFACES_BEYOND_DPB/MIN_SURFACES consts, pick_parameter_sets, coded_size, pool-capacity math — same rules re-implemented per lane (the vaapi twin at vaapi/session.rs:104-115/123-135/319-329). Cross-lane drift is already visible: coded_size differs in overflow checking (F-002), and ref-list/POC logic exists only on the VA-API side. A shared pure module (like frame_pool.rs) is the missing seam. S4-class, noted here because both halves are now audited.
- No other dead code in part1.

## 8. Dependencies & flags
- `#[cfg(target_os = "macos")]` (hw/mod.rs:96-97) — macOS release artifacts (release.yml:627+) would carry it compiled; runtime-dead under product.toml:168/191 regardless.
- `#![allow(unsafe_code)]` (:79) — the sanctioned E2 exception; 9 unsafe blocks in part1, each with a SAFETY comment (the refcon deref and the CF calls' invariants are stated at each site).
- No Cargo feature; no external deps (hand-rolled ffi, no `-sys`).

## 9. Open questions for the next agent
- W06-A07 (part2, already filed the pool/display_size/Send findings — no duplication intended here): F-001's damage-counter fix belongs in `on_frame`+`Shared`+`drain` — spans both our ranges.
- Whoever owns hw/mod.rs: the damage-accounting contract sentence (hw/mod.rs:47-49) should name the VideoToolbox deviation or the lane should comply.
- If a macOS CI lane ever exists: extract the pure helpers OS-neutrally and test them host-side (feed.rs precedent); until then every "verified" claim in this file is static-only.
