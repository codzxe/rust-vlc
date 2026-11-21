# AUDIT: crates/codecs/src/video/h264/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 795 / 27 |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO — `H264Decoder` has zero production callers; the shipped h264 lane instantiates `H264SwDecoder` (apps/cli/src/pipeline_host.rs:346). Call chain for the shipped h264 path: `lumen open` → apps/cli host → `PipelineHost::decoder_for` → `H264SwDecoder::new` (crates/codecs/src/video/h264/sw.rs). `H264Decoder` is constructed only in crates/codecs/tests/h264_hw_path.rs. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 2 |
| Commands run | `rg -n "H264Decoder" apps/ crates/playback-engine/` → only a comment (pipeline_host.rs:342) + tests; `sed -n` quote verification on decode.rs:75-78,604-609,743-759; `rg -n "default = \\[" -A 13 crates/codecs/Cargo.toml`; `sed -n '193p;219p' .github/workflows/release.yml`; `sed -n '168p;191p' product.toml`; baseline: audit/runs/baseline/{INDEX.md,mp4-smoke-open-session-420.txt,test-workspace.log.txt} |

## 0. WAVE-6 HOSTILE QUESTION — is H.264 decode reachable from the shipped `lumen` CLI / lumen-tauri build?

**YES — software H.264 decode is reachable AND was executed in the baseline; hardware H.264 decode (this file) is NOT reachable.**

Feature-flag trace (all lines verified by sed):
1. `crates/codecs/Cargo.toml:47-59` — `default = [ … "codec-h264-sw", … ]` (line 53) and `crates/codecs/Cargo.toml:73` — `codec-h264-sw = ["dep:rust_h264"]`. Default features compile the software engine.
2. `apps/cli/Cargo.toml:51` — `codecs = { path = "../../crates/codecs" }` with no `default-features = false` → CLI gets `codec-h264-sw`.
3. `.github/workflows/release.yml:193` — `run: cargo build --release --locked -p lumen-cli -p desktop` (repeated at 474, 627, 763, 911). No `--no-default-features`, no `--features` override anywhere in the workflow → the release CLI ships with `codec-h264-sw` ON.
4. `.github/workflows/release.yml:219` — `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` (repeated at 499, 652, 803, 954). lumen-tauri → `desktop` (apps/desktop-tauri/Cargo.toml:56) → `lumen-cli` (apps/desktop/Cargo.toml:40) → same pipeline host → same software decoder.
5. `product.toml:168` — `hw-decode = false …` and `product.toml:191` — `hw_decode_policy = "software-only"`: the product config forbids the hardware lane; the hardware `H264Decoder` of THIS file is therefore doubly dead in shipped builds (no caller + policy off).
6. Consumption point: `apps/cli/src/pipeline_host.rs:346` — `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),`.
7. Execution evidence: `audit/runs/baseline/mp4-smoke-open-session-420.txt` — `lumen open …/sample-420.mp4` (h264 Constrained Baseline 4:2:0) → `state-changed Buffering → Playing → Stopped → Idle`, EXIT=0. (Contrast: yuv444p sample refuses with `h264.sw.unsupported` — the SW decoder's own 4:2:0-only gate; see INDEX.md CORRECTION v2.)
8. Standing lies around the reachable path (INDEX.md CORRECTION v2, re-confirmed by flags above): `lumen engine-info` hard-codes `decoders: mjpeg, pcm_s16le` (apps/cli/src/host.rs `engine_info()`) — under-reports h264; `lumen decode-bench` prints "no decoder in this build (first vertical: mjpeg + pcm)" for MP4 tracks (apps/cli/src/bench.rs fall-through) — misleading tool output about a build that CAN decode h264 4:2:0.
9. The `feature-matrix.toml` rows agree with this reality: row `codec-video-h264-software` (line 419) notes "software is the shipped-default path"; row `codec-video-h264` (line 406) "stays partial pending the Phase-7 runtime activation".

## 1. One-paragraph verdict
This file is the **hardware** H.264 decode-path integration: parser → §7.4.1.2.4 access-unit assembler → `hw` session seam, honestly documented as no-software-fallback. The code is real and non-trivial (27 functions, AU boundary detection, damage accounting, typed refusals), and its logic is pinned by a 967-line integration suite (crates/codecs/tests/h264_hw_path.rs) plus fuzz-lite tests that ran clean in the baseline workspace test run (1681 passed, test-workspace.log.txt). But no shipped entry point ever constructs `H264Decoder`: the CLI/tauri h264 lane goes to `H264SwDecoder`, and `product.toml` keeps `hw-decode = false` / `software-only`. The file is therefore UNREACHABLE in shipped builds — 795 lines of compiled-in dead weight whose only consumers are tests. Internal defects below matter only for the future Phase-7 activation, which is exactly why they are recorded.

## 2. What actually works (verified)
— AU-assembly, typed-refusal, and damage-accounting logic verified by `cargo test --workspace` (baseline run #3: 1681 passed / 1 failed / 24 ignored, the 1 failure being an unrelated zero-alloc claim in apps/desktop) — the codecs suite incl. `h264_hw_path` (corpus 0019 full-stream, fuzz-lite never-panic tests) passed. File-level behavior additionally verified by reading: configure refusal chain (lines 604-609), delimiter/AVC end-of-packet close (688-691, 739-741).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/decode.rs:185-228 (whole type), evidence apps/cli/src/pipeline_host.rs:346
Code:
```rust
"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```
What is wrong: `H264Decoder` (this entire file) has no production caller. `rg -n "H264Decoder" apps/ crates/playback-engine/` returns only the explanatory comment at apps/cli/src/pipeline_host.rs:342. Only crates/codecs/tests/h264_hw_path.rs constructs it.
Why it matters: 795 lines + the whole `hw::{vaapi,videotoolbox}` subtree compile into every shipped binary and can never execute; bug surface and binary size for zero user-visible function (checklist: "feature flags gating real code that shipped builds never enable" — here it is not even flag-gated, it is unconditionally compiled and simply uncalled).
What correct looks like: either gate the hw lane behind a cargo feature (as the matrix note itself proposes for Phase 7) or wire `H264Decoder::from_setting` into the pipeline host behind `playback.hw-decode-policy != software-only`.
Confidence: HIGH — Verification: read + rg callers + product.toml:168/191.

### F-002 [S2]
Where: crates/codecs/src/video/h264/decode.rs:75-78, 478-488
Code:
```rust
/// Cap on NAL units buffered per pending access unit (MP §12: an
/// adversarial stream must not grow the accumulator unboundedly; a
/// conformant picture is orders of magnitude below this).
const MAX_PENDING_UNITS: usize = 4096;
```
and
```rust
    fn push_opaque(&mut self, nal: &[u8], unit_type: NalUnitType) {
        if self.pending.len() < MAX_PENDING_UNITS {
            self.pending.push(H264HwUnit {
                nal: nal.to_vec(),
```
What is wrong: the cap bounds the COUNT of buffered NALs, not the BYTES. Each `nal.to_vec()` copies a file-controlled length; 4096 × attacker-chosen NAL sizes is unbounded memory (e.g. 4096 × 1 MiB ≈ 4 GiB). Under Annex-B framing non-VCL units never trigger a picture boundary, so a hostile stream of big SEI/filler NALs accumulates freely across packets.
Why it matters: MP §12's own promise ("an adversarial stream must not grow the accumulator unboundedly") is violated in byte terms; OOM on valid-ish input is S1-class once this lane is reachable. Today the file is unreachable from shipped builds, hence S2 not S1.
What correct looks like: a byte budget alongside the unit cap (e.g. `pending_bytes += nal.len()` with a MAX_PENDING_BYTES cap, skip+count beyond it), as the MJPEG sibling's marker-scan budget does.
Confidence: HIGH — Verification: read (push_opaque, decode loop 666-735: no byte accounting anywhere).

### F-003 [S3]
Where: crates/codecs/src/video/h264/decode.rs:743-751
Code:
```rust
        Ok(if emitted > 0 {
            if self.skipped > 0 {
                DecodeOutcome::Recovered {
                    skipped: self.skipped,
                    reason: RecoverReason::CorruptInput,
                }
```
What is wrong: when a call both emits frames and skips damage, the outcome is `Recovered { skipped }` and the emitted-frame count is silently dropped — `DecodeOutcome::Recovered` has no `frames` field (crates/codec-api/src/decode.rs:138-143).
Why it matters: any consumer accounting decoded frames from `DecodeOutcome` (stats, bench, A/V sync pacing) under-counts on damaged streams — exactly the streams where accounting matters most. The frames do reach the sink, so this is accounting loss, not data loss.
What correct looks like: extend `Recovered` with `frames: u32` (codec-api change) or return `Emitted` and expose the skip count via a separate damage counter on the decoder.
Confidence: HIGH — Verification: read both enums (codec-api/src/decode.rs:124-144).

### F-004 [S3]
Where: crates/codecs/src/video/h264/decode.rs:394-404, 413-419
Code:
```rust
    fn active_sps(&self) -> Option<&H264HwSps> {
        let pps = self
            .pps
            .values()
            .min_by_key(|p| p.pps.pic_parameter_set_id)?;
        self.sps.get(&pps.pps.seq_parameter_set_id)
    }
```
What is wrong: the "active" SPS for surface negotiation is always the one referenced by the lowest-id PPS. A mid-stream SPS activation that changes sampling/resolution (adaptive stream, resolution switch) does update `self.sps` and re-offers the whole set to the hw session, but `self.format`/`self.display_size` (lines 417-419) are re-derived from the SAME lowest-id PPS's SPS — the new one.
Why it matters: frames after a 4:2:0→4:2:2 switch keep the stale `PixelFormat` label (frames carry `format: self.format.unwrap_or(Nv12)` at line 464); a renderer trusting the label misinterprets the opaque surface's real format.
What correct looks like: derive the presentation format from the SPS referenced by the access unit actually being closed (its slices' `pic_parameter_set_id`), or have the hw session report the per-frame surface format.
Confidence: MEDIUM — Verification: read (no per-AU SPS tracking exists in close_picture); realistic-case wrongness discounted by the file being unreachable today.

### F-005 [S3]
Where: crates/codecs/src/video/h264/decode.rs:98-106
Code:
```rust
/// SPS → slice-parser projection. `delta_pic_order_always_zero_flag`
/// is `false` because the LANDED SPS parser discards that bit (see
/// module docs — recorded limitation, never a guess).
fn slice_sps_info(sps: &SeqParameterSet) -> SliceSpsInfo {
```
(and line 106: `delta_pic_order_always_zero_flag: false,`)
What is wrong: for POC-type-1 streams whose SPS sets the flag true, the slice header parse runs with the wrong syntax (reads `delta_pic_order_cnt_bottom` bits that are not present), which can still round-trip the `pic_parameter_set_id` check in `parse_vcl` (line 516-517) and yield a WRONG `SliceHeader` — garbage `frame_num`/POC keys break first-VCL boundary detection (every slice its own picture) instead of surfacing as a clean skip.
Why it matters: silent misparse beats loud refusal in the worst way; the module doc itself admits "a set-flag stream surfaces as slice-header parse damage", but the damage can be *well-formed garbage*, not a parse error.
What correct looks like: retain the flag in `SeqParameterSet` (one bit) and project it truthfully.
Confidence: MEDIUM — Verification: read; the flag is hard-coded false and sps.rs's parse discards it (per doc; sps.rs is W06-A09's part, cross-ref open question).
Confidence note: the doc's own admission makes this a recorded, honest limitation — hence S3, not S2.

### F-006 [S4]
Where: crates/codecs/src/video/h264/decode.rs:464, 477
Code:
```rust
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
```
What is wrong: defensive fallback labels a frame Nv12 when no SPS ever configured the format — a guessed label on a frame that should not exist (hw session cannot be configured without SPS/PPS).
Why it matters: a lie in the label namespace; if reachable it misinforms the renderer. Blast radius ~zero (unreachable path, hw would have to emit frames unconfigured).
What correct looks like: refuse to emit when `self.format.is_none()` (typed `h264.hw.surface_format`).
Confidence: HIGH — Verification: read.

### F-007 [S4]
Where: crates/codecs/src/video/h264/decode.rs:645-660
Code:
```rust
                            Err(_) => {
                                    // Framing damage: the splitter has
                                    // terminated itself; count and keep
                                    // what did split.
                                    self.skipped += 1;
                                    break;
```
What is wrong: under AVC length-prefixed framing, one framing error counts as exactly ONE skipped unit regardless of how many NALs remained unsplit in the packet.
Why it matters: damage accounting under-reports; a consumer using `skipped` to decide concealment severity gets 1 for "rest of packet lost".
What correct looks like: count remaining undecodable bytes/units (the splitter knows how much it did not consume).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `unsupported` | 85-96 | REAL | — | builds typed Unsupported error; used at configure paths |
| `slice_sps_info` | 101-111 | REAL | S3 | F-005 flag hard-false |
| `slice_pps_info` | 114-126 | REAL | — | plain field copy |
| `surface_format` | 130-140 | REAL | — | bit_depth!=0→None; 420/mono→Nv12; 422→Yuyv422; 444→None |
| `PictureKey` (struct) | 146-155 | REAL | — | §7.4.1.2.4 comparison key |
| `PictureKey::of` | 158-174 | REAL | — | poc_type0 Some((lsb, delta_bottom)) |
| `H264Decoder` (struct) | 185-228 | REAL | S3 | F-001 unreachable in shipped builds |
| `impl Debug` | 230-246 | REAL | — | manual, no trait-object Debug |
| `impl Default` | 248-270 | REAL | — | AnnexB default framing |
| `new` | 278-280 | REAL | — | zero-bridge constructor |
| `with_bridges` | 285-290 | REAL | — | test/integration entry |
| `from_setting` | 310-319 | REAL | — | policy wiring; software-only → no accel → typed refusal at configure (test :856) |
| `hw_events` | 326-328 | REAL | — | accessor |
| `accepts` | 332-334 | REAL | — | "h264", "avc1", "avc1.*" |
| `seed_extradata` | 337-363 | REAL | — | avcC vs Annex-B CodecPrivate; Annex-B arm swallows non-SPS/PPS silently by design |
| `store_sps` | 367-378 | REAL | — | typed error, hw_configured reset |
| `store_pps` | 381-392 | REAL | — | same |
| `active_sps` | 398-404 | REAL | S3 | F-004 lowest-PPS-id heuristic |
| `sync_hw_config` | 407-436 | REAL | S3 | F-004 stale format on SPS switch; refuses 4:4:4/10-bit typed |
| `close_picture` | 440-475 | REAL | S2/S4 | F-002 accumulation feed; F-006 Nv12 fallback; non-VCL-only pending carries over (correct) |
| `push_opaque` | 478-488 | REAL | S2 | F-002 count-capped, byte-uncapped `to_vec` |
| `parse_vcl` | 494-527 | REAL | S3 | ambiguity refusal is honest; F-005 wrong-context misparse risk |
| `parse_slice_header_pub` | 532-539 | REAL | S4 | pure indirection "to keep trait import out of hot name space" — dead weight, one caller |
| `parse_error` | 543-552 | REAL | — | typed mapping |
| `hw_error` | 555-571 | REAL | — | DeviceLost→HardwareLost mapping present |
| `Decoder::configure` | 574-615 | REAL | — | full state reset; hw open refusal chain verified (604-609) |
| `Decoder::decode` | 617-760 | REAL | S3/S4 | F-003 accounting, F-007 undercount; stale-packet drop; AUD/EOS close |
| `Decoder::flush` | 762-793 | REAL | — | closes pending + drains session |

## 5. Test honesty (crates/codecs/tests/h264_hw_path.rs — 967 lines, ran in baseline run #3)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no_accelerator_is_typed_unsupported_never_silent (:302) | YES | asserts configure → Err with `h264.hw.required` — mutation to Ok(()) fails it |
| accelerator_without_bridge_is_typed_no_bridge (:318) | YES | same class |
| annexb_stream_feeds_aus_and_emits_opaque_frames (:366) | YES | asserts AU unit counts/pts through FakeSession — breaks assembly mutation |
| multi_slice_picture_is_one_au (:482) | YES | the §7.4.1.2.4 key comparison is the subject |
| damage_is_counted_recovery_and_decoder_stays_usable (:566) | YES | skipped-count assertions |
| corpus_0019_annexb_full_stream_survives_and_feeds_every_picture (:699) | YES | real ffmpeg-authored fixture (fixtures dir), per-picture feed count |
| fuzz_lite_random_packets_never_panic (:785) | YES (panic class only) | LCG-driven random packets; catches panics, not wrong output |
| from_setting_software_only_default_is_off_and_refuses_typed (:856) | YES | policy OFF semantics |
Caveat: every runtime test rides FakeBridge/FakeSession — a mock of the *dependency* (the hw backend), legitimate for assembly logic; no test exercises a real VAAPI/VideoToolbox session (matrix row notes admit "runtime hardware decode NOT exercised in sandbox"). The real backends are compile-verified only.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no software fallback anywhere in this file … typed Unsupported configure error (h264.hw.required / h264.hw.no_bridge)" | decode.rs module docs :10-15 | REAL | code 604-609; tests :302/:318 |
| "Delimiter NALs close the pending picture, and so does end-of-packet under length-prefixed framing" | decode.rs :26-29 | REAL | code 688-691, 739-741 |
| "a NAL that fails parsing is skipped and counted … Recovered { CorruptInput }" | decode.rs :33-40 | REAL | code 669-727; test :566 |
| "4:2:0/monochrome → Nv12 … 4:4:4, over-8-bit refused with typed h264.hw.surface_format" | decode.rs :42-54 | REAL | `surface_format` 130-140 + `sync_hw_config` 421 |
| "an adversarial stream must not grow the accumulator unboundedly" | decode.rs :75-77 | SHALLOW | count-capped, BYTE-uncapped (F-002) |
| hw row "stays partial pending the Phase-7 runtime activation" | feature-matrix.toml:416 notes | REAL | matches unreachable status (F-001) |
| sw row "software is the shipped-default path" | feature-matrix.toml:429 notes | REAL | pipeline_host.rs:346 + baseline 420 play session |

## 7. Dead / duplicated / unreachable code
The file itself is the dead code (F-001): unreachable in shipped CLI/tauri builds, unconditionally compiled (no cfg gate on `pub mod decode`, h264/mod.rs:51). `parse_slice_header_pub` (532-539) is a needless indirection with exactly one caller.

## 8. Dependencies & flags
No new deps; consumes in-house `parser` + `hw` seam. Not behind any cargo feature — compiled unconditionally (contrast: `sw` behind `codec-h264-sw`, h264/mod.rs:55-56). `installed_bridges()` (hw/mod.rs:449-460) compiles the VAAPI lane on linux targets only.

## 9. Open questions for the next agent
- W06-A09 (nal.rs part1): does `SeqParameterSet::parse` really discard `delta_pic_order_always_zero_flag` (F-005's premise)? Confirm in sps.rs.
- Whoever audits sw.rs / bench.rs / host.rs: the engine-info/decode-bench under-reporting (§0 items 8) deserves S-levels in their files.
- Phase-7 activation owner: F-002's byte budget must land before any real adapter is wired.
