# AUDIT: crates/codecs/src/video/av1/decoder.rs#part2
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 506-1043 of 1044 (chunk part2 of 2; nominal split 522 — mid-`emit_picture`; audited from 506 so the whole function is covered; lines 506-521 double-covered with part1/W06-A10) / 10 units: emit_picture, copy_plane, picture_color, engine_state_error, classify_engine_error, pooled_frame_slot, `Decoder::{configure, decode, flush}`, tests module (4 tests + 1 helper) |
| File verdict | REAL (executed end-to-end: rav1d engine seam, differential + conformance + CLI open-path lanes) |
| Reachable from shipped entry point? | YES. `lumen open <file.ivf>` → apps/cli containers/ivf.rs (AV01 → `av1`) → pipeline_host.rs:304-308 `"av1" \| "av01"` → `Av1Decoder::new()` → these `configure/decode/flush`. Feature: `codec-av1` in codecs default (codecs/Cargo.toml:50), built unmodified by release.yml:193/474/627/763/911. Executed: av1_differential 6/6 (baseline test-workspace.log.txt:891-904, incl. pixel-MD5 vs ffmpeg), av1_conformance 1/1 (:880-889), CLI lane `containers::ivf::tests::generated_av1_lane_decodes_the_full_stream ... ok` + `av1_lanes_decode_the_full_stream ... ok` (orchrun-test-workspace-nofailfast.log.txt:2797, 2812). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Read 120-214, 260-509 (part1 context: consts, Rav1dHandle, struct, feed/drain — boundary understanding), Read 506-1043 (my range, twice); Grep `fn drop_stale_packet` → codec-api/src/decode.rs:203 (import origin); baseline log greps: av1 suites (:873-904), `deps/lumen_cli` absent from test-workspace.log.txt (fail-fast died at apps/desktop) but present in orchrun-test-workspace-nofailfast.log.txt:2769-2812 (CLI lanes executed there); sed verification of quoted ranges via Read. |

## 1. One-paragraph verdict
This is the output/pooling half of the rav1d-backed AV1 decoder: typed output-boundary gates (8-bit 4:2:0 only, positive strides, non-null planes — each a named refusal, never a misread), a hand-checked strided-plane copy, conservative colorimetry mapping, the MJPEG-style pooled output slots, and the `Decoder` trait impl with the reference-state gate (skip dependent TUs until an independent frame decodes cleanly — the documented rav1d-1.1.0 upstream-abort containment). The execution evidence is strong: pixel-MD5 differential vs ffmpeg, a 14-second conformance matrix, fuzz-lite damage survival, and the CLI's full-stream IVF lanes. Two real defects: the output-slot pool permanently degrades to per-frame allocation after any mid-stream resolution change (stale-size slots are skipped forever and never replaced), and the damage-fold discards the emitted-frame count of a partially-successful call. The unsafe blocks are the best-documented in the hw tree (per-op SAFETY comments, edition-2024 discipline), and the rav1d-internal invariants they lean on are honestly marked where unverifiable.

## 2. What actually works (verified)
— Pixel-exact decode through this exact code path: baseline `av1_differential` 6/6 ok including `differential_pixel_md5_against_ffmpeg_dump` and `midstream_corruption_resyncs_and_recovers` (:891-904); `av1_conformance ... ok` 14.05s (:880-889).
— CLI open-path execution: `containers::ivf::tests::generated_av1_lane_decodes_the_full_stream ... ok`, `av1_lanes_decode_the_full_stream ... ok`, `vp9_lane_decodes_the_full_stream ... ok` (orchrun-test-workspace-nofailfast.log.txt:2797-2802, 2812) — note these ran only in the orchestrator's no-fail-fast rerun; the primary baseline run died at apps/desktop before ever reaching lumen-cli (see §7).
— Part2's 4 unit tests executed: baseline test-workspace.log.txt:519-523 (`descriptor_gate`, `eagain_classification`, `damage_folds_into_recovered`, `pending_map_bounded_and_lifo_tolerant`), all ok.
— `copy_plane` bounds re-derived by me: caller guarantees `(rows−1)·stride + row_bytes ≤ plane size` and `row_bytes ≤ stride` (stride is width-aligned by rav1d's allocator); `src.add(row·stride)` stays in-plane; dst slices are exact — no OOB path.
— `picture_color`: conservative-default mapping (unsignaled → BT.709 limited) matches the module-doc assumption; seq_hdr deref lifetime verified by me against the call order (deref inside emit_picture at :576; `dav1d_picture_unref` only at drain_pictures:469, after emit_picture returns).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/av1/decoder.rs:741-775
Code:
```
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
```
```
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
    *next = 0;
    Ok(fresh)
```
What is wrong: The pool only ever *appends* slots (`slots.len() < FRAME_SLOTS`); it never replaces one. After a mid-stream resolution change the four pooled slots keep the old byte length, the `!= len` guard skips them all forever, every subsequent frame takes the fresh-allocation path (:754), and — because the pool Vec retains one `SharedBytes` per stale slot — the four old-size buffers stay alive for the rest of the stream.
Why it matters: AV1 permits mid-stream resolution changes (spatial-layer/adaptive streams; the conformance corpus includes size-change lanes). For such a stream this decoder — whose docs sell "steady-state decodes refill a slot in place" (:731-733) and which feeds the `zero_alloc_decode` discipline — silently degrades to one heap allocation per frame plus dead retained buffers. Not a crash; a standing perf/memory contract break in realistic streams.
What correct looks like: When the fresh path runs and a stale slot is exclusively owned (`get_mut` succeeds), replace it (`slots[idx] = fresh.clone()`); or key slots by (len) in a small map; or clear the pool on a len change.
Confidence: HIGH — Verification: read; `SharedBytes::get_mut` exclusivity semantics assumed from name + MJPEG precedent (get_mut succeeds only when downstream released the frame — replaced slots are safe exactly when get_mut succeeded).
### F-002 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:893-902 (fold itself at :299-310, part1's function)
Code:
```
            .map(|emitted| {
                let mut outcome = if emitted > 0 {
                    DecodeOutcome::Emitted {
                        frames: emitted as u32,
                    }
                } else {
                    DecodeOutcome::NeedMoreInput
                };
                Self::take_damage(&mut outcome, damage);
                outcome
            })
```
What is wrong: When a call both emits frames and accrues damage, `take_damage` overwrites `Emitted { frames }` with `Recovered { skipped }` — the emitted count is discarded even though the frames were pushed to `out`. The unit test (:1005-1014) pins this: `Emitted{frames:2}` + damage 1 → `Recovered{skipped:1}`.
Why it matters: Any consumer that counts decoded frames from the outcome (bench/accounting surfaces) undercounts damaged calls despite the sink having received real frames. Information loss is by type design (`Recovered` has no frames field — codec-api's `DecodeOutcome`), so the fix belongs upstream of this file, but this decode path is where real frames get misreported.
What correct looks like: `DecodeOutcome` variant carrying both (e.g. `Recovered { skipped, emitted }`), or the fold preserves frames by summing into a richer outcome; then the CLI bench lane can trust the count.
Confidence: HIGH — Verification: read + test-pinned; who consumes `frames` (pipeline_host accounting vs sink) left as an open question (§9).
### F-003 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:950-956
Code:
```
        let mut eos = Dav1dData::default();
        // SAFETY: `ctx` is the live handle; `eos` is a valid local
        // of the exact pointee type (all-None fields — the empty-
        // data form the ABI defines).
        unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut eos))) };
```
What is wrong: The return value of the EOS `dav1d_send_data` is discarded. If the send fails (bad state, EINVAL), flush proceeds to drain anyway — usually harmless (drain returns EAGAIN → `Drained`), but the failure is unobserved and the "documented EOS idiom" claim itself is unverifiable here (rav1d 1.1.0 source not in the sandbox; the empty-`Dav1dData`-means-drain contract is asserted, not proven).
Why it matters: A silent swallowed error on the one call whose whole purpose is to change engine state; if the idiom is wrong (or changes at a rav1d bump), flush degenerates to "no-op returning Drained" with zero diagnostic.
What correct looks like: Match the result: success → drain; `EAGAIN` → drain then retry once; anything else → `classify_engine_error("av1.flush.send")`; cite the rav1d source line for the EOS idiom in the comment.
Confidence: MEDIUM — Verification: read; EOS-idiom correctness UNVERIFIED (no rav1d source locally).
### F-004 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:135-146 (context, part1 range) consumed at :699 and :486 (part2 consumers)
Code:
```
/// `ENOMEM` on every platform we ship (Linux, Windows, macOS, the
/// BSDs — errno 12 everywhere; rav1d returns it negated on picture
/// allocation failure).
const ERRNO_ENOMEM: c_int = 12;
```
What is wrong: EAGAIN/EINVAL detection goes through std's platform-portable `ErrorKind` mapping (:145-159, the module's own "errno values differ across platforms" rationale), but ENOMEM is a hard-coded 12 compared by equality — the file contradicts its own portability argument one function up.
Why it matters: Consistency/hygiene; on any platform where ENOMEM ≠ 12 the allocation-failure classification silently falls to the Corrupt branch (misdiagnosis, not a crash). The named platforms all use 12 today.
What correct looks like: `errno_kind(code) == ErrorKind::OutOfMemory` (std maps ENOMEM to OutOfMemory), deleting the constant.
Confidence: HIGH — Verification: read; `classify_engine_error` at :699 and `drain_pictures` at :486 both compare `-ERRNO_ENOMEM` directly.

## 4. Per-function table (part2 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `emit_picture` | 506-611 | REAL | — | Typed gates: geometry (≤0), I420-only, 8-bit-only, positive strides, non-null planes — each named code; `w*h` math overflow-proof on 64-bit (max 65536² = 2³²; on 32-bit targets it could wrap, but rav1d cannot produce an un-allocatable 4-GiB+ picture first — unreachable in practice). Counter↔timestamp round-trip consistent with feed stamping (:382 → :577). |
| `copy_plane` (unsafe) | 621-636 | REAL | — | Per-op SAFETY comments; bounds re-derived (see §2); no OOB path. |
| `picture_color` | 642-682 | REAL | — | Unsafe seq_hdr deref with lifetime argument I verified against the call order; conservative BT.709 fallbacks for exotic pri/trc/matrix; range polarity correct (`color_range != 0` → Full). |
| `engine_state_error` | 687-695 | REAL | — | Trivial typed constructor. |
| `classify_engine_error` | 698-729 | REAL | S4 | F-004; three-way taxonomy (ENOMEM/EINVAL/Corrupt) each named. |
| `pooled_frame_slot` | 735-776 | REAL | S3 | F-001; impossible-path (`fresh_slot_not_exclusive`) honestly typed; rotation cursor correct. |
| `Decoder::configure` | 779-805 | REAL | — | Descriptor gate + fresh engine per generation (honest flush — rav1d refs not selectively discardable); state resets complete (pending/slots/counter/gate). Note: pending pictures inside the old engine are dropped un-drained — intended for the seek path, worth a doc line. |
| `Decoder::decode` | 814-920 | REAL | S4 | Not-configured/no-engine typed; stale-generation drop (codec-api helper); discontinuity → `dav1d_flush` + gate close; framing-first (obu) with gate close on refusal; gate skip is `Recovered{Discontinuity}`; refs bookkeeping (909-918) sound (corrupt closes, clean independent opens, Unsupported output refusals leave open — correct, the engine did decode). F-002 fold. |
| `Decoder::flush` | 922-967 | REAL | S4 | F-003; 0-frames+damage → `Recovered{skipped}` (not `Emitted{0}`) — correct; pending cleared. |
| tests (4 + helper) | 975-1043 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_delivery_vocabulary | YES | Positive+negative asserts (av1/av01 in; vp9/"" out). |
| eagain_classification_is_platform_correct | YES | Derives the platform's EAGAIN via std's own mapping then asserts both classifiers against it and against −22 (EINVAL); content-sensitive on every platform. |
| damage_folds_into_recovered | YES (and it PINS F-002) | Asserts the overwrite semantics including accumulation and the zero-damage no-op. |
| pending_map_bounded_and_lifo_tolerant | YES | Cap eviction (1024→stays 1024), out-of-order pop, double-pop → Missing. |
| All 4 | Executed | baseline test-workspace.log.txt:519-523, ok, none ignored. |
| (integration) av1_differential / av1_conformance / CLI ivf lanes | Executed | :880-904 and orchrun:2797-2812 — these cover the whole part2 surface with real ffmpeg-generated streams, including the damage/resync and tenbit-refusal lanes. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "steady-state decodes refill a slot in place; … only an all-busy rotation allocates" | decoder.rs:731-734 | SHALLOW after a resolution change | F-001: one size change defeats the pool for the rest of the stream. |
| "v0 output boundary … 8-bit 4:2:0 only — everything else is a typed refusal, never a misread" | decoder.rs:523-524 | REAL | :537-541 + `tenbit_fixture_is_a_typed_refusal_not_a_misdecode ... ok` (baseline :898). |
| "feeding an empty `Dav1dData` sets the engine's drain flag (its documented EOS idiom)" | decoder.rs:947-949 | REAL-UNPROVEN | Rav1d source absent from sandbox; behavior consistent with `midstream`/differential outcomes (reorder-delay frames do surface in the suites), but the idiom itself is citation-less (F-003). |
| "rav1d latches the error for exactly one get call; the next TU resyncs" (drain_pictures comment, :497-499 — part1 range, cited here because decode's damage accounting leans on it) | decoder.rs:497-499 | REAL-UNPROVEN | No rav1d source in sandbox; fuzz-lite damage survival (baseline :896) is the only executed evidence; an unbounded `continue` loop if the latch claim is wrong. |
| "a library component must not write to the process's stderr" (logger silenced) | decoder.rs:195-197 | REAL | `settings.logger = zeroed` at :203; no stderr noise in any baseline test output (av1 sections are clean). |

## 7. Dead / duplicated / unreachable code
- Nothing unreachable in part2 — every function is on the shipped av1 lane.
- Baseline-evidence gap (not this file's code): the primary `cargo test --workspace` run (test-workspace.log.txt) DIED at apps/desktop's zero_alloc_transport failure — exit 101, no `--no-fail-fast` — so every package sorting after "desktop" (input, lumen-cli, media-*, playback-engine, renderer-wgpu, streaming, testkit, video-engine, platform-*, …) never ran in it; their only executed evidence is the orchestrator's `orchrun-test-workspace-nofailfast.log.txt` rerun. The INDEX row #3 aggregate ("1681 passed / 1 failed") does not disclose this. Recorded here because I hit it while hunting av1 CLI-lane evidence; belongs in the baseline-run ledger, not the repo.

## 8. Dependencies & flags
- rav1d `=1.1.0`, `default-features = false`, `bitdepth_8|bitdepth_16`, asm OFF (codecs/Cargo.toml:125; admission record docs/dependencies/records/rav1d-1.1.0.md) — matches the mod.rs purity story. `codec-av1` default-on; never on wasm32 (codecs/Cargo.toml:62-65).
- Part2 unsafe blocks: 4 (copy_plane ×2 ops, picture_color seq_hdr deref, flush EOS send) — all with SAFETY comments; unsafe-op-in-unsafe-fn discipline documented (:627-629).

## 9. Open questions for the next agent
- Does any shipped consumer count decoded frames from `DecodeOutcome::Emitted::frames` (bench.rs's v_decoded, first-vertical evidence transcripts)? If yes, F-002 is an undercount bug, not just info loss — check apps/cli/src/bench.rs and the decode worker.
- W06-A10 (part1): `drain_pictures`' unbounded `continue` on non-EAGAIN engine errors (:497-500) trusts the latch-once invariant — worth a rav1d-source citation or a drain-iteration bound; also `take_damage` (:299-310) is where F-002's fold lives.
- Rav1d admission-record owner: quote the `on_error` unwrap line and the empty-data EOS line from the pinned tarball — both are load-bearing safety arguments currently resting on prose.
