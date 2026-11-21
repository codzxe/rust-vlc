# AUDIT: crates/codecs/src/video/mjpeg/decoder.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 1–694 of 1949 audited (chunk part1 of 3; nominal 1–650, extended to 694 = natural boundary end of `take_output`; `prepare_planes` at 701 onward belongs to part2/W06-A05; the tests module at 1201+ was audited by W06-A03) / 30 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`lumen decode-bench` → apps/cli/src/host.rs:948 `HostDecoderProvider::new` → apps/cli/src/pipeline_host.rs:242-245 `"mjpeg" => { let mut decoder = codecs::video::mjpeg::MjpegDecoder::new(); … }` → crates/codecs/src/video/mjpeg/mod.rs:362-375 `context.decode_scan`/`frame_len`/`assemble_frame` → this file. EXECUTED: audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt `v_frames=60 v_decoded=60`, avi-firstvertical-open-session.txt full lifecycle) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `Read` decoder.rs 1–730 (chunk + boundary); `rg -n "sof_index|td|ta|quant_table"` parser.rs (validation proven: sof_index=position-lookup parser.rs:575-579, td/ta≤3 at 594-596, Tq≤3 at 390-392, h/v 1..=4 at 386-388, then 1..=2 at 403-410 — the direct indexes at decoder.rs:611-624 cannot panic); `rg -n "decode_scan|assemble_frame|frame_len|take_output|Recovered|FRAME_SLOTS"` mod.rs (contract path uses this chunk's split decode at 362-375, Recovered{ConcealedFrame} at 394-396); `rg -n "NoIdctStages|decode_scan_with"` crates/codecs (stub arms used only by examples/stage_bench.rs:107,314-324 and part3 tests); `grep -n "video::mjpeg" audit/runs/baseline/test-workspace.log.txt` (lines 651-720: 70 tests ok, incl. all 18 decoder::tests); `sed -n` on every quoted range. |

## 1. One-paragraph verdict
The front half of the MJPEG decoder — the reusable `DecodeContext`, the plane/Huffman-table reuse keys, the `ScanHooks` profiling seam, and the MCU scan walk `scan_into` — is real, disciplined, bounds-safe code that ran green end-to-end in the baseline (60/60 frames decoded in the shipped `lumen` binary on the AVI first-vertical lane, plus 18 executed decoder unit tests covering context reuse, concealment, pooled-slot safety and restart boundaries). Every direct index (`planes[sof_index]`, `dc_tables[td]`, `quant_tables[quant_table]`) is protected by parser-side validation I traced by hand (parser.rs:386-392, 575-608), and the concealment path is observable (`concealed`/`cause` → `Recovered{ConcealedFrame}` at mod.rs:394-396). The real defects are modest: a misnumbered restart marker discards the entire remainder of the frame with no resync (libjpeg recovers), two defensive bounds-checks silently swallow would-be invariant violations instead of failing loudly, and a doc comment asserts codegen identity that nothing in the repo can prove.

## 2. What actually works (verified)
— Executed in the shipped binary: `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60` through `MjpegDecoder` → `DecodeContext::decode_scan`/`assemble_frame` (this chunk's API at 472-530).
— All 18 `video::mjpeg::decoder::tests` ran ok in baseline (`test-workspace.log.txt` lines 676-693), including `context_reuse_is_bit_exact_and_reallocates_only_on_change` (680, pins the `PlaneKey`/`DhtKey` reuse in 226-373), `pixel_cap_rejects_oversize_frames` (687, pins MAX_PIXELS at 105/564-570), `header_error_does_not_poison_the_persistent_scratch` (685, pins the headers-restore at 501-503), `restart_boundaries_crossing_mcu_rows_decode_bit_exact` (691, pins the restart logic at 594-609), `staged_full_scan_is_bit_exact_vs_the_production_path` (692, pins FullStages≡pre-seam output).
— Panic-safety of the scan walk's direct indexes verified by tracing parser validation: `parse_sos` resolves `sof_index` via `position()` (parser.rs:575-579, never out of range), rejects `td > 3 || ta > 3` (594-596) and missing tables (597-608); `parse_sof0` rejects `Tq > 3` (390-392), `h/v` outside 1..=4 (386-388), and restricts to 1..=2 (403-410), so `8 * h_max` at decoder.rs:577-578 cannot be 0.
— Concealment observability chain verified: `concealed=true` + `cause` set at 642-651 → carried in `DecodedMeta` (669-676) → mod.rs:394-396 `DecodeOutcome::Recovered { reason: RecoverReason::ConcealedFrame }`.

## 3. Findings

### F-001 [S3] Restart-marker mismatch conceals the rest of the frame — no resync, unlike libjpeg/VLC
Where: crates/codecs/src/video/mjpeg/decoder.rs:594-609
Code:
```
                    let expected = (restart_count & 7) as u8;
                    if reader.take_restart(expected).is_err() {
                        concealed = true;
                        cause = Some(ConcealCause::RestartMismatch);
                        break 'scan;
                    }
```
What is wrong: A single missing or misnumbered RSTn marker abandons the scan at that MCU: every remaining MCU of the frame becomes mid-gray via `gray_blocks` (661-667). libjpeg's restart-interval logic (jdmarker.c `read_restart_marker` / `process_restart`) resynchronizes at the next marker and continues decoding; VLC inherits that robustness for damaged MJPEG AVIs — a very realistic input class (this repo's own fuzz.rs tests `hostile_marker_soup`).
Why it matters: On a mildly corrupted MJPEG stream, one bad restart marker turns a 99%-recoverable frame into a mostly-gray one. The degradation is honest and observable (`concealed=true`), but it is materially worse output than both reference decoders on the same input.
What correct looks like: On restart mismatch, resynchronize: scan forward byte-wise for the next `FF D0..D7`, reset `dc_pred`, and continue the MCU walk, keeping `concealed=true` for the skipped MCUs only.
Confidence: HIGH — Verification: read; libjpeg behavior from spec §F.1.2.3 + libjpeg jdmarker.c semantics (not runnable here: UNVERIFIED by execution, but the code path taken on mismatch is verified in-source).

### F-002 [S4] `write_block`/`gray_blocks` silently return on out-of-bounds — invariant violations would produce stale pixels, not an error
Where: crates/codecs/src/video/mjpeg/decoder.rs:189-197 (and identical pattern 211-216)
Code:
```
            let Some(dst_row) = self.data.get_mut((y0 + y) * self.stride..) else {
                return;
            };
            let Some(dst) = dst_row.get_mut(x0..x0 + 8) else {
                return;
            };
            if let Some(src) = samples.get(y * 8..y * 8 + 8) {
```
What is wrong: The bounds checks cannot fail under the current invariants (plane geometry is built from the same MCU grid that generates `(col,row)`, verified: block_cols=mcu_w·h_i, col=mx·h_i+bx < block_cols), so this is dead defensiveness that fails *silently*: if a future geometry change ever broke the invariant, blocks would be dropped without any signal, and — worse — `gray_blocks` would stop mid-fill leaving previous-frame bytes in the reused (non-clearing) planes, exactly the leak class the module docs (43-50, 83-88) claim is impossible. Additionally `samples.get(y*8..y*8+8)` on a `&[u8; 64]` with `y<8` is always `Some` — pure dead code.
Why it matters: Silent-fallback guards on the reuse-without-clearing path convert a would-be loud bug into invisible stale-pixel leakage; the `pooled_slot_refill_is_bit_exact_and_stale_byte_safe` test (baseline line 688) only pins the *current* geometry, not the guard behavior.
What correct looks like: `debug_assert!` the invariant and `return Err(...)`/`corrupt_entropy` on violation (or at minimum a counter/tracing signal), and delete the always-true `samples.get`.
Confidence: HIGH — Verification: read; invariants traced through `prepare_planes` (701-730) and `scan_into` block-coordinate math (632-633).

### F-003 [S4] Doc asserts "codegen-identical" for the ScanHooks seam — unprovable by any committed evidence
Where: crates/codecs/src/video/mjpeg/decoder.rs:243-245 (repeated 536-539)
Code:
```
/// (the W7-b profiling seam). The production walk always runs
/// [`FullStages`], whose impls are `#[inline]` one-line forwardings
/// to the real stages, so after monomorphization the production
```
(continues: "instantiation is codegen-identical to the pre-seam body (the seam adds no branch, no call).")
What is wrong: The claim of codegen identity is asserted as fact; the repo evidence (`staged_full_scan_is_bit_exact_vs_the_production_path`, baseline line 692) proves *bit-exact output*, not codegen identity. No godbolt/asm dump/objdump comparison is committed anywhere.
Why it matters: A zero-cost-abstraction claim stated as verified fact without the means to verify it is exactly the "perf claims the method cannot support" class; a later refactoring (e.g. dropping `#[inline]`) would silently invalidate it.
What correct looks like: State "bit-exact (pinned by test); codegen identity expected from `#[inline]` forwarding but not verified" — or commit an asm diff.
Confidence: HIGH — Verification: read; `rg` over repo found no asm/codegen evidence for this claim.

### F-004 [S4] `#[allow(dead_code)]` on the three profiling stub structs
Where: crates/codecs/src/video/mjpeg/decoder.rs:282,296,308
Code:
```
#[allow(dead_code)] // profiling arm: constructed by examples/stage_bench.rs
pub(crate) struct NoIdctStages;
```
What is wrong: The allow is honest (comment says who constructs them, and examples/stage_bench.rs:107,314-324 does), but it still suppresses the compiler's own reachability knowledge inside the lib target: if `stage_bench.rs` were deleted, these stubs would rot silently instead of erroring.
Why it matters: Checklist class "`#[allow]` hiding real lints" — minor here because the consumer exists and is named, but the lint could be kept alive by moving the stubs behind `#[cfg(any(test, feature = …))]` or into the example itself.
What correct looks like: Move `NoIdctStages`/`NoWriteStages`/`NoIdctNoWriteStages` into `examples/stage_bench.rs` (they only serve it and the tests) or gate them so dead code is detected.
Confidence: HIGH — Verification: `rg` construction sites (stage_bench.rs only + part3 test module).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (concealment/context/handoff) | 1-98 | REAL | — | Claims traced: reuse keys ✓ (tests 680/683), Recovered mapping ✓ (mod.rs:394), Ns==Nf proof ✓ (parser.rs:560); "codegen-identical" unproven → F-003 |
| `MAX_PIXELS` const | 99-105 | REAL | — | 8 MiP cap; enforced at 564-570; test 687 pins refusal |
| `OutputFormat` enum | 107-118 | REAL | — | I420/Rgba8; chosen by `choose_format` (part2) |
| `ConcealCause` enum | 120-134 | REAL | — | 4 damage classes; set at 643-651 |
| `DecodedImage` struct | 136-149 | REAL | — | stateless-path output (stateless `decode` fn lives in part2) |
| `DecodedMeta` struct | 151-165 | REAL | — | metadata-only twin; used by pooled path |
| `Plane` struct | 167-180 | REAL | — | padded plane geometry fields |
| `Plane::write_block` | 182-199 | REAL | S4 | silent OOB return + dead `samples.get` → F-002 |
| `Plane::gray_blocks` | 182-219 | REAL | S4 | same silent-return pattern → F-002; fills 128 per block; test 684 pins tail-only fill |
| `PlaneKey` struct | 222-236 | REAL | — | geometry identity; zero-padded sampling implies component-count equality (reuse safety) |
| `ScanHooks::transform` (default) | 255-263 | REAL | — | `#[inline]` forward to `idct_block_dispatch` |
| `ScanHooks::store` (default) | 265-270 | REAL | — | forward to `Plane::write_block` |
| `FullStages` | 273-276 | REAL | — | production arm; bit-exact vs pre-seam pinned by test 692 |
| `NoIdctStages` | 278-290 | REAL | S4 | profiling stub; `#[allow(dead_code)]` → F-004 |
| `NoWriteStages` | 292-304 | REAL | S4 | black_box anchor; F-004 |
| `NoIdctNoWriteStages` | 306-321 | REAL | S4 | F-004 |
| `DhtKey` struct | 323-331 | REAL | — | raw BITS/HUFFVAL identity |
| `DhtKey::matches` | 333-343 | REAL | — | allocation-free compare; content-sensitivity pinned by test 683 |
| `DhtKey::of` | 345-354 | REAL | — | snapshot on change only; `vals.clone()` bounded by 256-symbol DHT limit |
| `DhtKey::update_from` | 356-372 | REAL | — | capacity reuse (V-3); clear+extend per table |
| `DecodeContext` struct | 375-402 | REAL | — | scratch-only state; `plane_builds`/`table_builds` are test-only counters (part3's S4, fields declared here) |
| `Default for DecodeContext` | 404-419 | REAL | — | `[const { None }; 4]` table init; no allocation |
| `DecodeContext::new` | 421-425 | REAL | — | trivial delegate |
| `DecodeContext::decode` | 427-457 | REAL | — | resize-without-memset reuse (441-448, tests 680/688); `frame_len`+`assemble_frame_into` are part2 |
| `DecodeContext::decode_scan` | 459-474 | REAL | — | pooled-path entry; used by mod.rs:362 |
| `DecodeContext::decode_scan_with` | 476-505 | REAL | — | headers parked via `mem::replace` and restored on BOTH outcomes (501-503) — pinned by test 685 |
| `DecodeContext::frame_len` | 507-516 | REAL | — | delegates to `frame_total_for` (part2) |
| `DecodeContext::assemble_frame` | 518-530 | REAL | — | length contract; test 676 pins refusal on mismatch |
| `DecodeContext::scan_into` | 532-677 | REAL | S3 | core walk; component gate 552-559; pixel cap 561-570 (saturating, overflow-safe); restart handling → F-001; direct indexes 611-624 proven safe via parser validation; dc_pred reset per restart ✓; concealment tail-fill 661-667 ✓ |
| `DecodeContext::output` (cfg(test)) | 679-688 | REAL | — | test-only read accessor, honestly gated |
| `DecodeContext::take_output` | 690-694 | REAL | — | `mem::take`; test 693 |

## 5. Test honesty (tests for this chunk live in part3's range, 1201+; executed)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| context_reuse_is_bit_exact_and_reallocates_only_on_change (680) | YES | asserts bit-equality across 60 frames on one context AND `plane_builds`/`table_builds` counts — breaks if reuse keys or walk change |
| pixel_cap_rejects_oversize_frames (687) | YES | constructs an over-cap frame and asserts typed refusal |
| header_error_does_not_poison_the_persistent_scratch (685) | YES | decodes bad-then-good frames; asserts the good frame is bit-exact after the error |
| restart_boundaries_crossing_mcu_rows_decode_bit_exact (691) | YES | restart interval chosen to straddle MCU rows; compares against stateless reference |
| staged_full_scan_is_bit_exact_vs_the_production_path (692) | YES | FullStages vs pre-seam output equality |
| pooled_slot_refill_is_bit_exact_and_stale_byte_safe (688) | YES | poisons slots with stale bytes, asserts exact overwrite |
Mutation check (reasoned, not run): breaking `PlaneKey` equality (e.g. dropping `sampling`) would flip `plane_builds` counts → test 680 fails; removing the pixel-cap check → 687 fails; skipping `dc_pred` reset at restarts → 691 fails (values drift).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "concealed = true … surfaced as DecodeOutcome::Recovered { ConcealedFrame }" | decoder.rs:13-16 | REAL | mod.rs:394-396; executed lane mp4-smoke-bench-avi-mjpeg-pcm (no damage there, but mapping is static) |
| "every block of every plane is rewritten each frame" (reuse-without-clear safety) | decoder.rs:43-50 | REAL | parser.rs:560 Ns==Nf gate + write_block/gray_blocks partition; tests 680/688 |
| "steady-state handoff allocates nothing" | decoder.rs:74-77 | REAL-UNPROVEN | code path verified (mod.rs pooled slots), but no allocation-counting run for the video path in baseline (zero-alloc test exists only for desktop transport, W01 baseline #3) |
| "after monomorphization … codegen-identical" | decoder.rs:243-245 | REAL-UNPROVEN | bit-exactness pinned (test 692); codegen identity unverified → F-003 |
| lib.rs:49-52 "No `unsafe`, no panics on any input" | crates/codecs/src/lib.rs | REAL-UNPROVEN | no unsafe in this chunk ✓; panic-freedom argued by tracing parser validation ✓; but the fuzz targets were never *executed* in baseline (fuzz_registry_check only diffs the registry, gate #18) — UNVERIFIED by execution |

## 7. Dead / duplicated / unreachable code
- `samples.get(y*8..y*8+8)` (195-197): always-Some slice check on a fixed `&[u8;64]` — dead (F-002).
- The three profiling stub structs are dead within the lib target by design (F-004) — used by examples/stage_bench.rs and part3 tests only.
- `DecodedImage` (137-149) is the stateless-path payload; the stateless `decode` fn is in part2 — no duplication in this range.

## 8. Dependencies & flags
- No feature gates in this file; the mjpeg module is unconditional in `crates/codecs/Cargo.toml` (in-house decoder, per the Cargo.toml comment block) — shipped in every build including `--no-default-features` codecs subsets.
- Depends on: `super::bitreader` (my file #6), `super::color` (part1/part2 other agents), `super::error`, `super::huffman`, `super::idct_simd` (my file #5), `super::parser` (part1 other agent + my part2), `media_types::packet::SharedBytes`.
- W06-A03's part3 header mis-attributes part1 to W06-A05; the tsv files are authoritative: part1 = W06-A04 (me).

## 9. Open questions for the next agent
- part2 (W06-A05): verify `assemble_frame_into` overwrites *every* byte on the I420 crop path for odd geometries (the neutral-fill path) and that `frame_total_for` matches the pixel-cap arithmetic — the pooled-slot safety argument (83-88) leans on both.
- part2: confirm `choose_format` maps grayscale (1 component) to I420 neutral chroma as the module docs claim (110-113).
- huffman.rs owner: `decode_block`'s DC coefficient `receive_extend` — test 690 covers sign handling; check the 16-bit magnitude edge.
