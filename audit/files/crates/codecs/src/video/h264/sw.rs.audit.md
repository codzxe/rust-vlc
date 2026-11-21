# AUDIT: crates/codecs/src/video/h264/sw.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 641 / 18 (incl. const FRAME_SLOTS, struct, 3 impl blocks, free fn) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <mp4/mkv/ts>` → apps/cli/src/pipeline_host.rs:346 `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)` (H264SwDecoder imported at :65); demux normalizes every H.264 fourcc to `"h264"` (crates/demux/src/mp4/mod.rs:440 `b"avc1" \| b"avc2" \| b"avc3" \| b"avc4" \| b"avcp" => Some("h264")`; crates/demux/src/mkv/header.rs:785). Feature gate: `codec-h264-sw` is in codecs' `default` (crates/codecs/Cargo.toml:53), apps/cli/Cargo.toml:51 takes defaults, release.yml:193/474/627/763/911 build `-p lumen-cli` with no feature subset; tauri closure carries it (apps/desktop-tauri/Cargo.lock:397-406, 2888). EXECUTED in baseline: `mp4-smoke-open-session-420.txt` (INDEX.md CORRECTION v2) — h264 CB 4:2:0 MP4 → Buffering → Playing → Stopped → Idle, EXIT=0; `tests/h264_sw_differential.rs` 17/17 ok (test-workspace.log.txt:1063-1087). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:5 |
| Commands run | `rg -n "H264SwDecoder"` over crates/codecs/src + apps/cli/src (callers: pipeline_host.rs:65,346 only — the sole production consumer); `rg -n "avc3\|avc4"` over crates/demux+apps (mp4/mod.rs:440 normalizes, so `avc3` files reach this decoder); `rg -n '"avc1"'` over apps/cli+demux (nothing mints an `avc1`/`avc1.*` descriptor — the accepts arms are test-only); `rg -n "fn pooled_frame_slot"` (4 copies: mjpeg/decoder.rs:870 pub(crate) original, h264/sw.rs:599, av1/decoder.rs:735, vp9/decoder.rs:398) + sed of mjpeg/decoder.rs:886-908 (the original EVICTS — the h264 copy does not); sed verification of every quote below; baseline test log cited for execution. |

## 1. One-paragraph verdict
This is the H.264 software decoder the owner's MP4 bug report ultimately lands on, and it is real, wired, and executed: the `rust_h264` engine behind the codec-api contract, in-house NAL framing/EPB/damage accounting, avcC-vs-Annex-B framing switch, generational rebuild on configure, pooled I420 output. Baseline proves it end-to-end (sample-420 MP4 plays; 17 differential tests ok vs a pinned ffmpeg-7.1.5 MD5 oracle on six lanes × two entry forms — a genuinely hostile, mutation-killing suite). The defects found are real but small-bore: the pooled-slot copy dropped the eviction branch the mjpeg original has, so a mid-stream resolution change permanently breaks the zero-steady-state-allocation posture (per-frame full-frame allocations forever); the same function is copy-pasted 4× across the video decoders with drift; the `interlaced` flag is a one-way latch; the VUI transfer mapping force-overwrites where primaries/matrix keep defaults; flush drops damage accounting when nothing was emitted. No S0/S1/S2: no panic path found on hostile input (truncation sweep test passes; all length checks guarded at 394-399), and the typed refusals the Wave-0 smoke saw (`h264.sw.unsupported` for 4:4:4) are the documented contract, not fakery.

## 2. What actually works (verified)
- Full decode lane for 8-bit 4:2:0 H.264 (CB/Main/High, B-frames, odd crop, single-IDR still) — verified by baseline `test-workspace.log.txt:1063-1087`: `tests/h264_sw_differential.rs` … `test result: ok. 17 passed` (lanes A–F on BOTH Annex-B (real `demux::es`) and MP4/AVCC (real `demux::mp4`) entry forms, MD5-pinned against ffmpeg 7.1.5, re-verified live when ffmpeg is on PATH per the test's own header), plus `mp4-smoke-open-session-420.txt` playing end-to-end (INDEX.md CORRECTION v2).
- Typed refusal posture: 4:2:2/4:4:4/10-bit → `h264.sw.surface_format` (370-374), engine family limits → `h264.sw.unsupported` (526-535) — the exact refusal the original yuv444p smoke produced (`mp4-smoke-open-session.txt:64` `error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`), matching code to observed behavior.
- Damage model: in-stream `UnexpectedEof`/`InvalidSyntax` counted and recovered (536-541), hostile battery (`hostile_garbage_packets_recover`, `hostile_truncation_sweep_is_panic_free`) ok in baseline — no unwrap/expect/index panic path in the file (all slice accesses length-guarded at 394-404).
- Seek/reconfigure: whole-session rebuild per generation (435-441) with extradata re-seed via the host's `ReseedingDecoder` (pipeline_host.rs:387+), avoiding the engine's no-mid-stream-reset corruption hazard.
- Reachability of the hw-off default: `product.toml:168` `hw-decode = false` + `:191` `hw_decode_policy = "software-only"` — this file is the shipped H.264 path, and it is compiled in every release lane (see header row).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/sw.rs:635-639
Code:
```
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
    *next = 0;
    Ok(fresh)
```
What is wrong: when the pool is full (`slots.len() == FRAME_SLOTS`) and no slot matches `len` — exactly the state after a mid-stream SPS resolution change, since `configure` resets `slot_next` (440) but never clears `slots`, and `emit_frame` computes `total` from the new frame — the fresh slot is NEVER entered into the rotation. Every subsequent frame takes the cold path: `SharedBytes::from_vec(vec![0; len])` (618) per frame, forever.
Why it matters: the module's stated posture ("pooled output-slot ceiling … bounded memory by construction, MP §12", 96-98, and `emit_frame`'s "zero steady-state allocation" framing) silently degrades to a full-frame-size allocation + zero-fill per frame for the rest of the stream after one resolution change. The mjpeg ORIGINAL this is copied from handles it — mjpeg/decoder.rs:897-908 evicts (`let evict = *next; slots[evict] = fresh.clone();`) with the comment "bounded at FRAME_SLOTS, then evicting". Dynamic-resolution H.264 streams exist in the project's own TS corpus (`0113-dynres-video.ts`, apps/cli/src/containers/ts.rs:849). av1/decoder.rs:771-775 and vp9/decoder.rs:429-433 carry the same dropped branch (other agents' files).
What correct looks like: the mjpeg eviction branch: when `slots.len() == FRAME_SLOTS`, replace `slots[*next]` with the fresh slot (and advance `*next`), so the pool converges to the active geometry; optionally clear the pool in `configure`.
Confidence: HIGH — Verification: read (both implementations sed-verified side by side; mjpeg/decoder.rs:886-908 vs this 605-640).

### F-002 [S3]
Where: crates/codecs/src/video/h264/sw.rs:599-640 (duplicated at crates/codecs/src/video/av1/decoder.rs:735-777, crates/codecs/src/video/vp9/decoder.rs:398-436; original at crates/codecs/src/video/mjpeg/decoder.rs:870-911)
Code:
```
fn pooled_frame_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]),
) -> Result<SharedBytes, EngineError> {
```
What is wrong: a ~40-line helper exists four times in the same crate. The mjpeg one is `pub(crate)` and error-typed; the three copies are private re-implementations that differ only in error type — and have already drifted (F-001: the copies lost the eviction branch the original has). The comment at 595-598 even says "the mjpeg/vp9/av1 `pooled_frame_slot` pattern", documenting the duplication rather than sharing it.
Why it matters: four sites to keep in sync for a memory-behavior invariant; the drift has already produced a real defect divergence. This is exactly the checklist's copy-paste-duplication item with observed divergence.
What correct looks like: one `pub(crate)` generic over the error type (or returning `Result<SharedBytes, EngineError>` with a Into wrapper) in a shared module, consumed by all four decoders.
Confidence: HIGH — Verification: read (all four bodies sed-compared).

### F-003 [S4]
Where: crates/codecs/src/video/h264/sw.rs:314-317
Code:
```
    fn adopt_sps_color(&mut self, info: rust_h264::nal::SpsInfo) {
        if !info.progressive {
            self.interlaced = true;
        }
```
What is wrong: `interlaced` is a one-way latch — set on any non-progressive SPS, never cleared by a later progressive SPS (or by in-band SPS change).
Why it matters: a stream that switches from interlaced to progressive source mid-stream (SPS change, e.g. ad-inserted or stitched content) mislabels every later emitted frame `interlaced: true` (412), so downstream deinterlacing/rendering misbehaves for the remainder of the generation. `configure` resets it (439) but only there.
What correct looks like: `self.interlaced = !info.progressive;` — the flag should track the ACTIVE SPS, not any SPS ever seen.
Confidence: HIGH — Verification: read (all writes to `self.interlaced` grep-verified: 125 field, 316 set, 439 reset — no clear path in `adopt_sps_color`).

### F-004 [S4]
Where: crates/codecs/src/video/h264/sw.rs:324-331
Code:
```
        let transfer = match info.transfer {
            13 => Some(TransferCharacteristics::Srgb),
            16 => Some(TransferCharacteristics::Pq),
            18 => Some(TransferCharacteristics::Hlg),
            // 1 (BT.709), 6 (BT.601), 14/15 (extended gamma) and
            // every unspecified value: the BT.709 posture.
            _ => Some(TransferCharacteristics::Bt709),
        };
```
What is wrong: `transfer` is ALWAYS `Some` — including VUI "unspecified" (2) — so it always overwrites `color.transfer` (347-349). `primaries` and `matrix` correctly use `_ => None` (keep current posture, 322, 336). The asymmetry means a stream whose first SPS sets sRGB (13) and whose second SPS leaves transfer unspecified silently reverts to BT.709 instead of keeping the previous posture.
Why it matters: minor colorimetry mislabeling on SPS-switching streams; also an internal inconsistency the comment papers over by calling it "the BT.709 posture" (the same comment logic applied to primaries would be a lie there, which is why they differ).
What correct looks like: `2 => None` (unspecified keeps current) with the BT.709 fallback living only in the field default (161), matching primaries/matrix handling.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/codecs/src/video/h264/sw.rs:589-591
Code:
```
        } else {
            DecodeOutcome::Drained
        })
    }
```
What is wrong: `flush` reports `Drained` when `emitted == 0`, even if `self.skipped > 0` (an undisplayable drained frame was counted at 380 or 397) — the damage accounting is silently dropped. `decode`'s equivalent tail (554-558) correctly returns `Recovered` for `skipped > 0, emitted == 0`.
Why it matters: inconsistent damage reporting between the two entry points; a caller tracking corruption via `Recovered` misses flush-time damage.
What correct looks like: mirror decode's three-way outcome: `skipped > 0` → `Recovered`, else `Drained`.
Confidence: HIGH — Verification: read (both tails sed-verified: 545-561 vs 580-591).

### F-006 [S4]
Where: crates/codecs/src/video/h264/sw.rs:470
Code:
```
        let mut nals: Vec<&[u8]> = Vec::new();
```
What is wrong: per-packet heap allocation for the NAL list (grows by push for every packet, on top of the reused `rbsp` buffer the file is otherwise careful about — 130-132 "zero steady-state allocation").
Why it matters: one small Vec alloc+free per packet in the hottest path; contradicts the file's own steady-state-allocation posture. Cosmetic next to F-001 but same theme.
What correct looks like: a `Vec<&[u8]>` field cleared per call (`self.nal_refs.clear()`), or feed NALs inline without collecting (the Annex-B iterator could drive the loop directly; only the AVC arm needs the break-on-damage nuance it already has).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const FRAME_SLOTS` | 96-98 | REAL | — | 4 slots; ceiling only enforced on push, see F-001 |
| `struct H264SwDecoder` | 110-135 | REAL | — | state coherent; `framing: Option<u8>` packs length_size (1-4, validated upstream in AvcConfig) |
| `Debug::fmt` | 137-153 | REAL | — | no secrets; honest backend string |
| `Default::default` | 155-169 | REAL | — | explicit zeros; BT709_LIMITED default posture |
| `new` | 175-177 | REAL | — | trivial delegate |
| `accepts` | 182-184 | REAL | S4 | `"avc1"`/`"avc1."*` arms unreachable from production: no shipped lane mints them (mp4/mod.rs:440 and mkv header.rs:785 normalize to `"h264"`); exercised only by tests/h264_hw_path.rs:346. Harmless defense, dead in practice |
| `unsupported` | 187-198 | REAL | — | typed refusal factory |
| `parse_error` | 203-212 | REAL | — | maps parser failures; source chained |
| `engine_refusal` | 217-229 | REAL | — | carries engine rule string — honest why-unsupported |
| `seed_extradata` | 235-258 | REAL | — | avcC → framing switch + SPS/PPS feed; Annex-B branch silently ignores non-SPS/PPS NALs (by design, configure-time); damage → typed Err |
| `bridge_nal` | 265-273 | REAL | — | `Err(())` damage signal; no panic; `EngineNalType::from(header.unit_type.id())` — unknown ids map to engine's catch-all (engine's domain) |
| `feed_param_set` | 279-309 | REAL | — | adopt-after-accept ordering correct; Unsupported fatal at configure (documented) |
| `adopt_sps_color` | 314-355 | REAL | S4 | F-003 latch, F-004 transfer asymmetry; primaries 4..=7→Bt601 and matrix 5..=7→Bt601 match H.273 |
| `emit_frame` | 360-416 | REAL | — | 8-bit-420 gate (370-374); zero-dim and short-plane guards count damage not panic (377-399); crop-unit-evenness argument for `/2` is spec-correct for 420 (odd displays only exist for 422/444, refused above); pooled slot F-001 |
| `Decoder::configure` | 420-446 | REAL | — | full session rebuild; slots NOT cleared (F-001 interaction); descriptor refusal typed |
| `Decoder::decode` | 448-562 | REAL | — | stale-generation drop via `drop_stale_packet`; framing split; damage counted; Unsupported fatal per generation (documented contract); F-006 Vec |
| `Decoder::flush` | 564-592 | REAL | S4 | drains with metas; F-005 outcome gap |
| `pooled_frame_slot` | 599-640 | REAL | S3 | F-001 (no eviction), F-002 (4th copy); fresh-slot get_mut impossibility handled loudly (626-632) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `lane_a..f_*` (12 tests) | YES | MD5 of decoded rawvideo pinned against an INDEPENDENT oracle (ffmpeg 7.1.5 `libx264` decode, fixtures authored by ffmpeg, constants from that decode, re-checked live when ffmpeg on PATH) — any decode regression, wrong crop, wrong plane order, or PTS loss (lane C reordering) changes the hash; run through the REAL demuxers, not mocks |
| `hostile_garbage_packets_recover` | YES | asserts `Recovered` outcome + decoder reuse — would fail if damage were fatal or silent |
| `hostile_truncation_sweep_is_panic_free` | YES | off-by-one sweep of every truncation length — kills any index/slice panic (the unwrap-free claim is actually exercised) |
| `hostile_garbage_then_real_recovers` | YES | resync-after-damage assertion |
| `contract_state_rules` / `contract_damaged_extradata_refuses_typed` | YES | wrong-descriptor and unconfigured-decode/flush must answer the exact typed errors |
Baseline: all 17 ok (test-workspace.log.txt:1063-1087). No `#[ignore]`, no mocks of the subject, no self-generated goldens (oracle is external ffmpeg). This is the honest version of a differential suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exact against ffmpeg 7.1.5's own libx264 decode on BOTH entry forms" (module doc :16-19) | sw.rs header | REAL | baseline h264_sw_differential 17/17 ok incl. both entry forms per lane |
| "every skipped unit is counted and reported as DecodeOutcome::Recovered, never a silent drop" (:31-32, 60-62) | sw.rs header | REAL (with S4 gap) | decode tail 545-561; flush tail drops it (F-005) — flush-time-only gap |
| "zero steady-state allocation" (rbsp, :130-132) | sw.rs header | SHALLOW-adjacent | rbsp IS reused; but NAL Vec per packet (F-006) and permanent cold path after resolution change (F-001) contradict the steady-state posture |
| engine-info `decoders: mjpeg, pcm_s16le` | INDEX.md standing defect | FALSE (under-report) | this decoder is compiled+reachable in the same binary (release.yml:193 defaults; smoke-420 plays) — the hard-coded literal hides it; defect lives in apps/cli/src/host.rs (other agent) |
| wave-6 hostile question: H.264 decode reachable from shipped CLI/tauri? | task prompt | YES — REAL | codecs/Cargo.toml:53 default `codec-h264-sw`; apps/cli/Cargo.toml:51 defaults; release.yml:193 `cargo build --release --locked -p lumen-cli -p desktop`; tauri Cargo.lock:397-406+2888 carries rust_h264; executed in mp4-smoke-open-session-420.txt |

## 7. Dead / duplicated / unreachable code
- `accepts` `"avc1"` / `"avc1."*` arms: no production lane mints those descriptors (mp4/mkv normalize to `"h264"`); test-only surface (h264_hw_path.rs:346). Not removed — defensive.
- `pooled_frame_slot`: 4th copy of a 4× duplication, drifted (F-001/F-002).
- Nothing else dead; every method has a live caller.

## 8. Dependencies & flags
- `rust_h264 = "=0.6.1"` optional behind `codec-h264-sw` (Cargo.toml:73,174); default ON (:53). All release lanes build with defaults → decoder ships (see header). Pure-Rust, zero-dep admission record cited in Cargo.toml:159-174.
- Uses `codec-api`, `media-types` (path deps), `std` only otherwise. No unsafe in this file. Lints inherited (workspace deny unwrap/expect/panic — file complies; no unwrap/expect/index panic found).

## 9. Open questions for the next agent
- av1/decoder.rs:735 and vp9/decoder.rs:398 carry the same no-evict copy (F-001) — confirm and file from those files' owner.
- `EngineNalType::from(header.unit_type.id())` (270): if the in-house `NalUnitType` has values outside the engine's enum, what does `from` produce? Traced to rust_h264's enum (external crate) — the in-house parser only produces H.264-valid types, but the mapping's exhaustiveness is the engine's contract; not verifiable from this repo (UNVERIFIED — external crate source not in tree).
