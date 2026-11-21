# AUDIT: crates/codecs/src/video/hevc/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 766 / 23 (incl. 2 trait impls, 2 helper fns, 1 struct+key type) |
| File verdict | UNREACHABLE (real, tested code — zero production callers) |
| Reachable from shipped entry point? | NO — `HevcDecoder` is constructed only by `crates/codecs/tests/hevc_hw_path.rs` (rg "HevcDecoder" across repo: definitions + that one test file). `apps/cli/src/pipeline_host.rs` `decoder_for` has no "hevc"/"hvc1"/"hev1" arm (match arms end at h264/pcm); `apps/cli/src/containers/ts.rs:968` even asserts `unreachable!("no hevc decoder exists in this build")`. Baseline: `mp4-smoke-open-session-hevc.txt` — HEVC sample → Opening → Error → Idle. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 2 |
| Commands run | Grep "HevcDecoder" (callers → tests only); sed -n on 367-379, 436-452, 548-577 (quote verification); sed hevc/mod.rs:45-55 (export); baseline test-workspace.log.txt:1086-1108 (17/17 hevc_hw_path tests ok); Grep "fn drop_stale_packet" → codec-api/src/decode.rs:203 |

## 1. One-paragraph verdict
This is genuine, carefully-written code: hvcC/Annex-B extradata seeding, §7.4.2.4.4 access-unit assembly with the non-VCL-intervened boundary condition, bounded pending-unit accumulation, PPS-context ambiguity refusal, typed error taxonomy, and a 17-test integration suite that ran green in the baseline (test-workspace.log.txt:1086-1108). But none of it is reachable from any shipped entry point: the CLI/tauri `HostDecoderProvider` dispatches no HEVC descriptor, no hardware bridge exists in-tree, and product.toml keeps `hw-decode = false` (line 168). The module is honest about the hw-only posture (`hevc.hw.required`/`hevc.hw.no_bridge` typed refusals, verified by tests) — the dishonesty risk lives in any doc/matrix row that presents "HEVC decode: implemented" without the unreachable caveat. Beyond reachability, the error paths have real state-consistency and labeling defects (F-001..F-003) that a future wiring would inherit.

## 2. What actually works (verified)
— 17/17 `tests/hevc_hw_path.rs` tests passed in baseline `cargo test --workspace` (audit/runs/baseline/test-workspace.log.txt:1086-1108: `test result: ok. 17 passed; 0 failed`), incl. `no_accelerator_is_typed_unsupported_never_silent` (asserts `ErrorKind::Unsupported` + code `hevc.hw.required`), `corpus_0044_annexb_full_stream_survives_and_feeds_every_picture`, `fuzz_lite_random_packets_never_panic`, `stale_packets_are_dropped_on_sight`, `multi_segment_picture_is_one_au`, `damage_is_counted_recovery_and_decoder_stays_usable`.
— Read-traced: configure→seed_extradata→sync_hw_config and decode→NAL walk→boundary-detect→close_picture state machines against the module-doc claims; the "no software fallback anywhere in this file" claim (lines 10-15) is true — every hw-open failure is a typed `Err` (lines 568-572).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:439-441 (and twin at 747-749)
Code:
```
            out.accept(Frame::Video(VideoFrame {
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
```
What is wrong: when `self.format` is `None`, emitted frames are silently labeled `Nv12` instead of refusing typed. `format` is `None` whenever `active_sps()` fails to resolve — which happens when the lowest-id known PPS references an SPS id that was never stored (store/parse fine) while a higher-id PPS with a resolvable SPS is the one that actually parses the VCL (parse_vcl skips only unresolvable contexts, lines 474-484). A 4:2:2 stream of that damaged shape gets Nv12-labeled surfaces — exactly the "wrong label" the module's own honesty section (lines 53-65) promises never to produce ("an honest gap … not a wrong label").
Why it matters: every consumer of the frame (renderer format negotiation, screenshots) trusts `PixelFormat`; a silent default converts a detectable inconsistency into wrong output. Also contradicts the file's typed-refusal discipline.
What correct looks like: `let Some(format) = self.format else { return Err(unsupported("hevc.hw.surface_format")); }` — or compute the label from the SPS that actually parsed the picture's slices.
Confidence: MEDIUM — Verification: read; path requires a PPS-referencing-missing-SPS stream shape; no test pins it; whole file unreachable in shipped builds.

### F-002 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:560-577
Code:
```
        self.framing = HevcFraming::AnnexB;
        self.generation = Some(cfg.generation);

        if let Some(extra) = cfg.extra_data.as_ref() {
            self.seed_extradata(extra.as_slice())?;
        }
```
What is wrong: `configure` mutates shared state (clears vps/sps/pps/pending at 550-559, sets `generation` at 560) BEFORE the fallible steps. If `seed_extradata`, `HevcHwPath::open` (568-572) or `sync_hw_config` (576) fails, the struct is left half-mutated: new generation committed, all parameter sets wiped, `self.hw` still holding the PREVIOUS generation's session (assignment at 569 only runs on `Ok`).
Why it matters: a caller that ignores/queues the error (or retries `decode` before reconfiguring) now decodes under `generation = new` with zero parameter sets into a stale hw session — cross-generation state bleed. The contract says configure runs "again, on a fresh generation, after seeks" (quoted in apps/cli/src/pipeline_host.rs ReseedingDecoder docs), so reconfigure-after-error is a realistic engine path.
What correct looks like: build the new state locally, commit (including `generation` and `hw` swap) only after every fallible step succeeds; or on failure, restore the old generation.
Confidence: HIGH — Verification: read (state flow traced line by line).

### F-003 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:86-89, 456-467, 686-694
Code:
```
const MAX_PENDING_UNITS: usize = 4096;
```
```
        if self.pending.len() < MAX_PENDING_UNITS {
            self.pending.push(HevcHwUnit {
                nal: nal.to_vec(),
```
What is wrong: the accumulator bound is a unit COUNT, not a byte bound. Each pending unit owns a full `to_vec()` copy of its NAL. A crafted Annex-B stream of continuation segments (`first_slice_segment_in_pic == 0`, so no boundary ever fires — only first-segments and delimiters close pictures) with large NALs retains up to `4096 × NAL-size` bytes ≈ file-size-proportional heap. The comment claims "MP §12: an adversarial stream must not grow the accumulator unboundedly" — the growth is bounded in units but unbounded in bytes.
Why it matters: a multi-GB hostile file (or a lying server feeding a "local" stream) turns into multi-GB heap on the decode path; the demux byte caps (product.toml `cache_demux_mib = 32`) do not bound what the decoder itself retains across packets.
What correct looks like: a byte budget on `pending` (sum of `nal.len()`), units beyond it counted as skipped damage — mirroring how the queue layer bounds bytes, not just counts.
Confidence: MEDIUM — Verification: read (no byte bound anywhere in the push paths); exploit needs an AU that never closes, which continuation-only streams produce.

### F-004 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:363-373
Code:
```
    /// The "active" SPS for surface negotiation: the one referenced
    /// by the lowest-id known PPS (single-parameter-set streams — the
    /// whole corpus and virtually every real file — have exactly
    /// one; multi-PS streams re-sync on their in-band activations).
    fn active_sps(&self) -> Option<&HevcHwSps> {
```
What is wrong: the comment's escape hatch ("multi-PS streams re-sync on their in-band activations") describes behavior that does not exist: `format`/`display_size` are only ever recomputed from `active_sps()` (lowest-id PPS's SPS), never from the PPS context that actually parsed the current picture. A conformant multi-SPS stream switching 4:2:0 ↔ 4:2:2 pictures labels every frame with the lowest-id SPS's format for its whole life.
Why it matters: the comment papers over a real limitation of the heuristic instead of admitting it; the next agent wiring HEVC would trust the comment. (The 4:2:2-mislabeled case is F-001's twin with a conformant stream.)
What correct looks like: either implement per-picture re-sync (track the SPS of the PPS that parsed the picture's first segment) or rewrite the comment to state the limitation: "multi-PS streams are labeled by the lowest-id PPS's SPS; per-picture re-sync is not implemented".
Confidence: HIGH — Verification: read (no code path recomputes format outside sync_hw_config/active_sps).

### F-005 [S4]
Where: crates/codecs/src/video/hevc/decode.rs:91-93, 444, 752
Code:
```
/// The conservative hw-path colorimetry (matches the H.264 sibling's
/// posture; SPS VUI color is a 7.B negotiation item).
const HW_COLOR: ColorDescription = ColorDescription::BT709_LIMITED;
```
What is wrong: every emitted frame is labeled BT.709 limited regardless of the stream's actual VUI colorimetry (e.g., BT.601 SD content gets a BT.709 tag → wrong colors downstream). The comment admits it is a placeholder pending 7.B.
Why it matters: color metadata consumers (renderer CSC) will apply the wrong matrix on non-709 content when this path eventually ships.
What correct looks like: parse `vui_parameters` colour description in the SPS (already partially parsed by the 5.G1 parser?) and thread it through; until then this stays a recorded gap.
Confidence: HIGH — Verification: read; HW_COLOR used unconditionally at both emission sites.

### F-006 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:159-194 (whole `HevcDecoder` surface)
Code:
```
pub struct HevcDecoder {
    /// Installed hardware bridges (the 7.B plug point).
    bridges: Vec<Arc<dyn HevcHwBridge>>,
```
What is wrong: 766 lines of decoder + its hw seam compile into every shipped binary (module is unconditional, crates/codecs/src/video/mod.rs:32) but have zero production callers — no host dispatch arm, no bridge installed in-tree, product.toml `hw-decode = false` (line 168). Only `crates/codecs/tests/hevc_hw_path.rs` constructs it.
Why it matters: any feature-matrix row or doc that counts HEVC decode as a shipped capability inflates the product; the binary carries dead weight; the shipped HEVC experience is `Error → Idle` (baseline mp4-smoke-open-session-hevc.txt) with EXIT=0.
What correct looks like: the honest state is exactly what docs/formats/video-hevc.md frames it as — a *seam* awaiting 7.B adapters. Matrix/doc rows must say UNREACHABLE/not-shipped, and the wave's hostile-question answer must record: H.264 IS reachable (see my log), HEVC is NOT.
Confidence: HIGH — Verification: Grep "HevcDecoder" repo-wide (definitions, mod.rs re-export, tests only); apps/cli/src/pipeline_host.rs decoder_for read; baseline hevc smoke log.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `unsupported(code)` | 96-107 | REAL | — | builds typed Unsupported error; trivial, correct |
| `surface_format(sps)` | 112-122 | REAL | — | 8-bit 420/mono→Nv12, 422→Yuyv422, else None; matches doc claim exactly |
| `struct PictureKey` + `PictureKey::of` | 128-151 | REAL | — | §7.4.2.4.4 comparison key; fields match module doc |
| `struct HevcDecoder` (fields) | 159-194 | UNREACHABLE | S3 | F-006; state documented per-field |
| `impl Debug` | 196-212 | REAL | — | manual Debug, no secrets, correct |
| `impl Default` | 214-235 | REAL | — | all fields initialized; framing defaults AnnexB |
| `HevcDecoder::new` | 241-243 | REAL | — | zero bridges — truthful v0 (no adapter exists in-tree) |
| `HevcDecoder::with_bridges` | 248-253 | REAL-UNPROVEN | — | no in-tree caller (Phase-7 plug); struct-update correct |
| `accepts` | 257-263 | REAL | — | hevc/hvc1/hev1 + dotted prefixes |
| `seed_extradata` | 266-301 | REAL | — | hvcC vs Annex-B branch; non-PS hvcC arrays skipped by type (32/33/34 kept) — OK |
| `store_vps` | 307-323 | REAL | — | idempotent re-activation by byte compare; hw_configured invalidated on change — correct |
| `store_sps` | 326-342 | REAL | — | same shape as store_vps — correct |
| `store_pps` | 345-361 | REAL | — | same shape — correct |
| `active_sps` | 367-373 | SHALLOW | S3/S4 | lowest-id-PPS heuristic; misleading "re-sync" comment (F-004) |
| `sync_hw_config` | 376-409 | REAL | — | early-return when hw_configured/empty; format None-latch is the F-001 precondition; snapshot-before-mutate borrow handling correct |
| `close_picture` | 413-453 | REAL | S3 | non-VCL-only pending carries over (correct); `hevc.hw.required` guard for None hw; Nv12 fallback (F-001); per-frame out.accept error propagated |
| `push_opaque` | 456-467 | REAL | S3 | count-bounded only (F-003); sets non_vcl_since_vcl |
| `parse_vcl` | 473-501 | REAL | — | candidates from resolvable PPS→SPS; sorted by id; round-trip check `slice.slice_pic_parameter_set_id == pps_id`; ambiguity refused — correct; O(PPS) clones per VCL NAL (perf, minor) |
| `parse_error(e, code)` | 507-516 | REAL | — | typed Bitstream error w/ source |
| `hw_error(e, code)` | 519-535 | REAL | — | Configure→Unsupported, Decode→Bitstream, DeviceLost→HardwareLost mapping correct |
| `Decoder::configure` | 538-578 | REAL | S3 | descriptor refusal resets error format name (good); F-002 non-atomic error path |
| `Decoder::decode` | 580-731 | REAL | — | generation gate + drop_stale_packet (codec-api/src/decode.rs:203); framing split both forms; AUD/EOS/EOB close; boundary detection incl. non_vcl_since_vcl; length-prefixed end-of-packet close; outcome matrix Emitted/Recovered/NeedMoreInput correct |
| `Decoder::flush` | 733-765 | REAL | — | closes pending then hw.flush; Nv12 fallback twin (F-001); Drained vs Emitted correct |

## 5. Test honesty
Tests live in `crates/codecs/tests/hevc_hw_path.rs` (not this file); baseline ran them: 17/17 ok.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no_accelerator_is_typed_unsupported_never_silent | YES | asserts `ErrorKind::Unsupported` AND code `hevc.hw.required` — mutation to silent Ok or different kind fails it |
| accelerator_without_bridge_is_typed_no_bridge | YES | same double assertion on `hevc.hw.no_bridge` |
| wrong_codec_descriptor_is_refused_typed | YES | asserts `hevc.codec_descriptor` on h264 descriptor |
| corpus_0044_annexb_full_stream_survives_and_feeds_every_picture | YES (state machine) | feeds real corpus bytes; would catch AU-assembly regressions (mutation: drop the non_vcl_since_vcl condition → all-IDR stream merges pictures → AU count assertion fails) |
| damage_is_counted_recovery_and_decoder_stays_usable | YES | pins skipped-count + continued usability |
| multi_segment_picture_is_one_au | YES | pins the continuation-segment path |
| stale_packets_are_dropped_on_sight | YES | pins drop_stale_packet integration |
| fuzz_lite_random_packets_never_panic / fuzz_lite_configure_with_random_extradata_never_panics | WEAK-YES | survival tests: catch panics, not mislabels (F-001/F-004 shapes are unlabeled by them) |
| hw_session_failure_is_typed_engine_error / hw_configure_failure_is_typed_unsupported / annexb_stream_feeds_aus… / hvcc_extradata_configures… / hvc1_descriptor_aliases… / orphan_continuation_segment… / decode_before_configure… / alien_bridge… | YES | each pins a specific typed code or AU count (verified names against assertions read at tests/hevc_hw_path.rs:326-370) |
Note: the fake bridge/session mocks the *hw seam*, not the decoder under test — legitimate boundary mock.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`HevcDecoder` configures from `hvcC` or Annex-B extradata or in-band parameter sets … assembles access units per §7.4.2.4.4 … feeds … to a hardware session" | docs/formats/video-hevc.md:38-48 | REAL (as library code) / UNREACHABLE (as shipped capability) | code lines 266-301, 538-578, 629-711; zero production callers (Grep) |
| "4:4:4 and >8-bit — Main 10 … typed `hevc.hw.surface_format` refusal" | docs/formats/video-hevc.md:50-55 | REAL | decode.rs:112-122 + 392; test hw_configure_failure lane green in baseline |
| "There is no software fallback anywhere in this file" | decode.rs:10-15 module doc | REAL | configure 568-572: every open failure → typed Err; no sw path exists |
| "56/56 AUs assembled" (corpus 0044) | docs/formats/video-hevc.md:49-51 | REAL (test-cited) | corpus_0044_annexb_full_stream_survives_and_feeds_every_picture ok (baseline log 1094) |
| HEVC playback available | (any user-facing claim; engine-info does NOT list hevc) | UNREACHABLE | baseline mp4-smoke-open-session-hevc.txt: Opening → Error → Idle |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable-from-shipped-builds (F-006) but compiled unconditionally (video/mod.rs:32 `pub mod hevc;`). Near-duplication: close_picture's frame-emission block (438-451) is copy-pasted in flush (746-757) — a shared `emit_frame` helper is the fix; same for store_vps/store_sps/store_pps (three structurally identical bodies differing in map + id field — a macro or generic would remove ~40 duplicated lines).

## 8. Dependencies & flags
Depends on: codec-api (Decoder, DecodeOutcome, prelude — `drop_stale_packet` from codec-api/src/decode.rs:203), media-types (EngineError/ErrorKind/Frame/VideoFrame/FrameStorage/PixelFormat/ColorDescription), super::hw (HevcHw* seam), super::parser (nal/pps/slice/sps/vps). No external crates; module unconditional in codecs (crates/codecs/src/video/mod.rs:32) — NOT behind `codec-hevc` (no such feature exists; the codec-* features gate only external engines, Cargo.toml:60-81). product.toml has no per-codec entry; `hw-decode = false` (product.toml:168) matches the no-bridge reality.

## 9. Open questions for the next agent
- h264/decode.rs (the twin this file mirrors): does its configure commit generation before fallible steps too (F-002 twin)? It IS reachable via hw lane settings — severity there would be higher.
- Who is supposed to install `HevcHwBridge` implementations (7.B1-7.B3)? If a future release.yml adds an adapter crate, F-001/F-002/F-004 become shipped-path defects — re-audit at wiring time.
- codec-api `Decoder::configure` contract: does it promise "on Err the decoder MUST be treated as dead"? If yes, F-002 is a contract violation (S2) — I could not find such wording in codec-api/src/decode.rs within my chunk (contract.rs#part2 audited separately; see that audit).
