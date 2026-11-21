# AUDIT: apps/web/src/render/present.rs#part1 (lines 1-427)
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part1 = 1-427 — ALL production code (module docs, types, conversion, mailbox); `#[cfg(test)] mod tests` starts at line 428 and belongs to part2 (W12-A07, 428-875). Equal-split anchor ~438 adjusted to the natural production/tests seam (same convention as W10-A01 client.rs#part1). |
| Lines / functions | 427 / 14 (2 consts, 2 structs, 2 enums, 2 enum impls, 6 free fns, 1 mailbox impl of 5 methods — see table) |
| File verdict | REAL (production half: conversion math verified against the standard coefficients; mailbox discipline verified; executed end-to-end on the REAL first-vertical fixture by host tests in baseline) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` (lumen-web is unshipped: no dist/, no page, no packaging/CI reference — see mock_fetch.rs audit). YES within the web lane, proven by call chain: engine decode sink → `host.rs:401 guard.mailbox.store(&video)` → pump `host.rs:1152 consumer.take_present()` → `take()` → `rgba_from_video` → `host.rs:942 take_pending_frame` → abi.rs raw frame lane → worker.js `lumen_web_frame_addr/len`. Executed: baseline `orchrun-test-workspace-nofailfast.log.txt:3119` (`host::tests::pump_slices_play_the_fixture_to_its_end ... ok`, REAL avi_mjpeg_pcm.avi) and `:3200` (`the_present_path_ships_one_frame_per_pump ... ok`) plus `:3186-3194` (9/9 present tests). The last mile (worker.js → canvas in a browser) never ran anywhere in baseline. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `Read` 1-427 (+428-665 tests for verification context); `Grep PresentMailbox\|rgba_from_video\|UploadFrame` (callers: host.rs:107/271/292/316/810/942, render/mod.rs:52 re-export); `sed -n` host.rs:310-330/930-960/395-401/1985-2050 (call chain + fixture test); read of media-types frame.rs:170-239 (`FrameStorage::Cpu(SharedBytes)`, `SharedBytes` = thin `Arc<Vec<u8>>` — packet.rs:369-371) and time.rs:158-180/742-748 (`convert` rounds-to-nearest, `map_timestamp`); read of codecs mjpeg decoder.rs:1059-1065 (`i420_total` — identical packed-plane convention); baseline log greps (:3119, :3186-3194, :3200). |

## 1. One-paragraph verdict
The production half of the present lane is the cleanest file I have audited in this wave: the YUV→RGB math is the textbook BT.601/709/2020 × limited/full derivation (I recomputed all eight coefficients — 601 limited gives 1.402/1.772/0.344/0.714, the standard values), the packed-plane layout provably matches the MJPEG decoder's own `i420_total` convention, the mailbox is a correct latest-wins with counted drops and take-time conversion, every malformed-geometry path is a typed refusal with checked arithmetic, the 32 MiB bound is validated BEFORE any loop arithmetic can overflow usize on wasm32, and the whole lane is executed on the real first-vertical fixture by baseline-passing host tests. The four findings are hygiene: one `?` that can propagate an uncounted refusal (on a path that would require a 4.6-exabyte frame to reach), a stats-field doc that misnames what it counts, an error-vocabulary Display that no production code ever prints (the only store() call site discards the value), and a duplicated time-base constant. The browser last mile remains unproven everywhere — no run in baseline ever drew one of these frames to a canvas.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3119` → `host::tests::pump_slices_play_the_fixture_to_its_end ... ok`: the REAL `avi_mjpeg_pcm.avi` opens through the real path and pumps to `Ended`, which drives `mailbox.store` (host.rs:401) and `take_present` (host.rs:1152) on real MJPEG I420 output — this chunk's `plane_layout`/`rgba_from_video`/`PresentMailbox` all execute there.
— verified by baseline `:3186-3194` → 9/9 `render::present::tests` pass, including `conversion_matches_the_reference_within_one_lsb` (3 matrices × 2 ranges × 8 samples vs an independent reference, ±1 LSB) and `mailbox_is_latest_wins_with_counted_drops` (part2's tests pinning this chunk's functions).
— verified by read: the coefficient derivation is the standard one — G = Y − (2Kb(1−Kb)/Kg)·Cb − (2Kr(1−Kr)/Kg)·Cr reproduces 0.344136/0.714136 for 601, and `r_cr = 2(1−Kr)`/`b_cb = 2(1−Kb)` give 1.402/1.772; limited range maps [16,235]/[16,240] via (v−16)/219 and (c−128)/224, full range via /255 — all correct, with deterministic f32 ops and half-away-from-zero rounding (`f32::round`).
— verified by read: the "cheap refcounted clone" claim (:349-350) is TRUE — `FrameStorage::Cpu(SharedBytes)` with `SharedBytes` a thin `Arc<Vec<u8>>` (media-types packet.rs:369-371), so `store`'s `frame.clone()` is refcount-only.
— verified by read: the "exactly what the first vertical's MJPEG decoder emits" claim (:23-26) is TRUE — `i420_total` (mjpeg decoder.rs:1059-1065) computes `Y(w×h) ‖ Cb(⌈w/2⌉×⌈h/2⌉) ‖ Cr` with the same `div_ceil` chroma and no stride — identical to `plane_layout`'s I420 arm, so real decoder output passes the exact-length check instead of refusing `TruncatedPlanes`.
— verified by read: panic-freedom is structural — the `MAX_UPLOAD_BYTES` gate at :198 bounds `w×h ≤ 8 Mi` before the per-pixel loops, so no usize product in the loop (:216, :212) can overflow even on wasm32 (u32 usize); every slice index is provably in-range under the exact-length precondition (:191); `as_chunks::<4>` remainders cannot exist (length ≡ 0 mod 4 by the same precondition).

## 3. Findings

### F-001 [S4]
Where: apps/web/src/render/present.rs:377-380
Code:
```
        if layout.rgba_bytes(frame.size)? > MAX_UPLOAD_BYTES {
            self.refused += 1;
            return Err(UploadError::TooLarge);
        }
```
What is wrong: the `?` propagates `rgba_bytes`' `Err(UploadError::TooLarge)` (u64 overflow in `w×h×4`) WITHOUT incrementing `self.refused` — the one refusal path in `store` that escapes the "every one counts" discipline (:74-75 "every one counts, none kill the session").
Why it matters: the mailbox's stats are the only place a refusal is ever observed (the production call site discards the `Result`, host.rs:401 `let _ =`), so an uncounted refusal is an invisible refusal. Reachability: to hit it, `plane_layout` must have succeeded with `w×h > u64::MAX/4 ≈ 4.6e18` AND the frame must carry `bytes.len() == layout.total ≥ 4.6e18` — a ~4.6-exabyte allocation. Latent by physical impossibility; still the wrong pattern.
What correct looks like: `match layout.rgba_bytes(frame.size) { Ok(b) if b > MAX_UPLOAD_BYTES => { self.refused += 1; return Err(UploadError::TooLarge); } Ok(_) => {} Err(e) => { self.refused += 1; return Err(e); } }`.
Confidence: HIGH — Verification: read (arithmetic traced; overflow window computed for u32 dims).

### F-002 [S4]
Where: apps/web/src/render/present.rs:329-330 (counter incremented at :385, after all refusal paths)
Code:
```
pub struct PresentStats {
    /// Frames the consumer offered.
    pub stored: u64,
```
What is wrong: `stored` counts only ACCEPTED offers — every refusal path (:362, :369, :374, :378) returns before `self.stored += 1` (:385). "Frames the consumer offered" is `stored + refused`; `stored` alone is "offers staged".
Why it matters: a reader computing offer rate from `stored` undercounts by the refusal count; the doc and the field disagree. The test `refused_offers_count_and_keep_the_staged_frame` (baseline :3192) pins the code's actual behavior, not the doc's words.
What correct looks like: doc: "Offers accepted for staging (refusals are counted separately in `refused`)" — or count `stored` at entry.
Confidence: HIGH — Verification: read of all return paths vs the increment site.

### F-003 [S4]
Where: apps/web/src/render/present.rs:94-114 (sole production call site: host.rs:401)
Code:
```
impl UploadError {
    /// The human sentence.
    #[must_use]
    pub fn as_str(self) -> &'static str {
```
(+ host.rs:395-401:)
```
                // A typed upload refusal (oversize / unsupported format) counts in
                // the mailbox's own stats — presentation bounds
                // never fail decode.
                let _ = guard.mailbox.store(&video);
```
What is wrong: the entire human-sentence surface (`as_str` + `Display`) has no production consumer: the only `store` call site discards the `Result` (deliberately — the comment says stats carry the truth), `take` swallows the error into a counter (:403-406), and no event/log/JSON path formats `UploadError`. The "typed refusal names them" claim (:83-84 — "the typed refusal names them") is true of the TYPE, but no shipped observer ever reads the name.
Why it matters: an operator debugging "video black, audio fine" on the web lane gets `refused: 47` and nothing about WHICH format or WHY; the vocabulary's diagnostic value is dead code in production.
What correct looks like: the pump report (or a debug event) carrying `UploadError::as_str` for the first refusal per session — the data is already in hand at the discard site.
Confidence: HIGH — Verification: read + `Grep as_str\(\)` over apps/web (no UploadError consumer outside this file's tests).

### F-004 [S4]
Where: apps/web/src/render/present.rs:43-49
Code:
```
/// The canonical nanosecond base (media-clock's own constant,
/// mirrored here so the present lane reads in one import — the pts
/// conversion is the same `NANO_BASE` mapping every host runs).
pub const NANO_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};
```
What is wrong: a second `NANO_BASE` in the crate — `apps/web/src/open.rs:27` imports and uses `media_clock::NANO_BASE` (the real one). The mirror is documented but is exactly the drift-shaped duplication the comment hand-waves past: if media-clock's base ever changed, this lane would silently disagree with the open lane's own ns conversion.
Why it matters: two sources of truth for one canonical constant inside the same 4-file crate; the "reads in one import" justification does not survive contact with open.rs already importing the original.
What correct looks like: `use media_clock::NANO_BASE;` (the dependency is already in the graph — media-clock is a declared dependency of lumen-web per Cargo.toml).
Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-39 | REAL | — | Every concrete claim I could check held up: packed-plane convention == decoder's (verified), latest-wins == mailbox (verified), take-time conversion (verified :398), transfer-rides-unconverted honestly labeled v1. |
| `NANO_BASE` (const) | 43-49 | REAL | S4 | Duplicate of media-clock's (F-004); value 1/1e9 correct. |
| `UploadFrame` (struct) | 51-65 | REAL | — | Lean and honest; `pts_ns` correctly `Option` (missing ≠ 0). |
| `MAX_UPLOAD_BYTES` (const) | 67-72 | REAL | — | 33,554,432 ≥ 4K RGBA 33,177,600 — the "covers with margin" claim is arithmetically true (1.1%). |
| `UploadError` (enum) | 74-92 | REAL | S4 | 5 typed refusals; sentences dead in production (F-003). |
| `UploadError::as_str` | 94-108 | REAL | S4 | Correct strings; no production reader (F-003). |
| `Display for UploadError` | 110-114 | REAL | S4 | Delegates to `as_str`; same dead-surface note. |
| `plane_layout` | 116-157 | REAL | — | I420: `div_ceil` chroma, all offsets/total checked (no overflow panic); RGBA: `w×h×4` checked; NV12/YUYV422 typed refusal. Zero-extent refused. Matches mjpeg `i420_total` exactly (verified). |
| `PlaneLayout` (struct) | 159-167 | REAL | — | Private; u64 fields throughout — no truncation. |
| `PlaneLayout::rgba_bytes` | 169-177 | REAL | — | Checked `w×h×4`; the overflow-`Err` path is the F-001 uncounted escape hatch (unreachable-in-practice). |
| `rgba_from_video` | 179-248 | REAL | — | OpaqueHandle typed refusal; exact-length plane check (longer OR shorter refused); bound gate BEFORE the loops (wasm32 usize safety); pts via `map_timestamp`+`convert` (rounds-to-nearest, `.ok()` only swallows ~century-scale overflow — unreachable); I420 loop indices provably in-range; Bgra8 swizzle correct; RGBA passthrough exact. Per-pixel f32 + per-frame `Vec` allocation — acceptable v1 (mailbox bounds work to what is presented; no perf claim made). |
| `YuvCoefficients` (struct) | 250-268 | REAL | — | Doc comments state the derivations; values verified standard. |
| `yuv_coefficients` | 270-291 | REAL | — | Kr/Kb for 601/709/2020 exact (0.299/0.114, 0.2126/0.0722, 0.2627/0.0593); ranges 219/224/16 and 255/255/0; Kg > 0 for all three (no div-by-zero). |
| `convert_yuv` | 293-313 | REAL | — | Standard equations; normalize-offset-then-scale; half-away-from-zero rounding; clamp [0,255]; no NaN possible (u8 inputs, finite coefficients). |
| `PresentMailbox` (struct) | 315-324 | REAL | — | `Option<VideoFrame>` + 4 counters. |
| `PresentStats` (struct) | 326-339 | REAL | S4 | `stored` doc misnames offered-vs-staged (F-002). |
| `PresentMailbox::new` | 341-346 | REAL | — | Default delegate. |
| `PresentMailbox::store` | 348-387 | REAL | S4 | Validate-then-stage; displaced counted BEFORE overwrite; refcounted clone (verified cheap); F-001's uncounted `?`. |
| `PresentMailbox::take` | 389-408 | REAL | — | Take-time conversion (displaced frames never pay for pixels — verified structure); internal-inconsistency path counts refused and answers None (no panic propagation). |
| `PresentMailbox::is_pending` | 410-414 | REAL | — | Used by host.rs:323. |
| `PresentMailbox::stats` | 416-426 | REAL | — | Snapshot; used by host.rs:330. |

## 5. Test honesty (if tests exist in/for this file)
Tests live at 428-875 (part2, W12-A07's range) — boundary noted. They exercise THIS chunk's functions; honesty assessed from the test bodies read for verification:
| Test (baseline :3186-3194) | Would it fail if the code were broken? | Reason |
|---|---|---|
| `conversion_matches_the_reference_within_one_lsb` | YES | Independent f64 reference over 3×2×8 (matrix, range, sample) grid, ±1 LSB — catches coefficient, offset, scale, and rounding errors. |
| `endpoints_are_exact_for_every_matrix` | YES | Black/white extremes exact — catches clamping and scale errors. |
| `chroma_siting_is_top_left_420` | YES | Odd/even pixels share chroma — pins the siting convention. |
| `odd_dimensions_round_chroma_up` | YES | `div_ceil` layout pinned. |
| `mailbox_is_latest_wins_with_counted_drops` | YES | Displaced/stored/converted counters pinned. |
| `refused_offers_count_and_keep_the_staged_frame` | YES | Refusal does not evict a staged frame; counters pinned. |
| `upload_refusals_are_typed` | YES | ZeroExtent/TooLarge/Unsupported/TruncatedPlanes/OpaqueHandle all pinned. |
| `rgba_passthrough_and_bgra_swizzle` | YES | Exact byte equality. |
| `pts_rides_the_upload` | YES | pts conversion pinned. |
Plus the lane-level proof: `host::tests::pump_slices_play_the_fixture_to_its_end` (:3119) and `the_present_path_ships_one_frame_per_pump` (:3200) run this chunk on REAL decoder output.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "decoded CPU frames → RGBA (the single allowed CPU→GPU-adjacent copy)" | present.rs:5-7 | REAL | One conversion per presented frame at take-time (verified); passthrough formats copy once. |
| "using the frame's OWN colorimetry: BT.601/709/2020 matrix × limited/full range" | present.rs:14-16 | REAL | `yuv_coefficients(frame.color.matrix, frame.color.range)` (:214); all 6 pairs standard-verified. |
| "I420 … exactly what the first vertical's MJPEG decoder emits (its `i420_total`)" | present.rs:23-26 | REAL | mjpeg decoder.rs:1059-1065 — identical convention (verified side-by-side). |
| "at most ONE frame staged, the newest wins, displaced frames are COUNTED" | present.rs:31-35 | REAL | `pending: Option<VideoFrame>`; `displaced` counter at :381-383; test :3189. |
| "the conversion happens at TAKE time so a displaced frame never pays for pixels" | present.rs:35-37 | REAL | `take()` → `rgba_from_video` (:398); store only clones (refcount, verified). |
| "an 8K frame is refused TYPED … the presenter counts the refusal" | present.rs:69-71 | REAL | 8K RGBA = 132,710,400 > 33,554,432 → `TooLarge` + `refused += 1` (:377-379); test :3193. |
| "presentation bounds must never bound decode" | present.rs:70-71 | REAL | Store-refusal returns `Err` without touching pending; engine loop continues (host.rs:395-401 discard is the consumer side of this contract). |
| "deterministic on every platform" (:293-294) | present.rs:293-294 | REAL | IEEE-754 f32 + `f32::round` (ties away from zero); no FMA contraction in Rust; part2's ±1-LSB reference test passes on the baseline host. |
| doc 09 §4.2 row 1 / §4.1 mailbox / doc 03 §4.11 references | present.rs:1-2, :33 | UNVERIFIED | The design docs themselves are other waves' assignments; the CODE side of each referenced behavior is verified above. |

## 7. Dead / duplicated / unreachable code
- `UploadError::as_str`/`Display`: no production consumer (F-003) — test-facing only.
- `NANO_BASE`: duplicate of `media_clock::NANO_BASE` (F-004).
- `rgba_from_video`'s `pub` + render/mod.rs:52 re-export: production callers are `PresentMailbox::take` only; the re-export is unused API surface (kept honest: it is the lane's documented conversion entry).
- No unreachable branches found; `PixelFormat::Nv12 | Yuyv422` arm in `rgba_from_video` (:237-239) is unreachable-through-`plane_layout` but deliberately total (comment :235-236) — defensive, not dead by accident.

## 8. Dependencies & flags
- Uses only `media_types` (frame, time) — already a core dependency. No unsafe, no feature gates, no external crates. Workspace lints respected (no unwrap/expect/panic; the single `unwrap_or` forms are the lint-legal fallbacks).
- wasm32-safety verified by construction: the 32 MiB gate precedes all usize math (see per-function notes); `usize::try_from(rgba_bound)` cannot fail post-gate.

## 9. Open questions for the next agent
- part2 (W12-A07): does any test pin the `store`→refuse→`store` sequence keeping the FIRST staged frame while counting both refusals (the `refused_offers_count_and_keep_the_staged_frame` test's exact shape)? And is `debug_assert_eq!(rgba.len(), rgba_bound)` (:241) ever compiled in the baseline test profile (release) — if not, the exactness claim rests on the loops alone.
- host.rs auditors: the `let _ =` at :401 is a deliberate discard — confirm no OTHER store() call site exists (my grep found none) and whether the pump report surfaces `refused` anywhere user-visible (F-003's fix would land there).
- worker.js/e2e auditor: the raw frame lane's last mile (canvas draw of these RGBA bytes) has zero executed evidence in baseline — if the web matrix ever runs, `pts_rides_the_upload`'s consumer (frame pacing in harness.js) is the thing to watch.
