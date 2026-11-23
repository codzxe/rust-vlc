# AUDIT: crates/codecs/src/video/h264/sw.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 641 / 16 (incl. trait impls; no in-file tests) |
| File verdict | REAL (executed: 17/17 differential + hostile tests pass live, ffmpeg oracle on PATH) |
| Reachable from shipped entry point? | **YES** — `apps/cli/src/pipeline_host.rs:346` `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`; feature `codec-h264-sw` is in codecs' `default` set (crates/codecs/Cargo.toml:47-60) and apps/cli depends on codecs with defaults; `target/release/.fingerprint/rust_h264-*` proves it compiles into the release lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |

## 1. One-paragraph verdict
This is the real, shipped H.264 decoder: a thin but careful codec-api wrapper (`H264SwDecoder`) around the external pure-Rust `rust_h264 = "=0.6.1"` engine (crates.io, zero deps), wired unconditionally into the CLI's decode factory. The wrapper's own logic (NAL framing on both wires, EPB unescape, damage accounting, pooled I420 output, typed refusals) is genuine, and the admission differential (`tests/h264_sw_differential.rs`) is the strongest test evidence I have seen in this repo: six libx264 lanes × both entry forms, pinned MD5s **plus a live ffmpeg re-check that actually ran** (ffmpeg 7.1.5 is on PATH in this sandbox) — I ran it: 17/17 passed, bit-exact. The hostile-focus answer: **yes, sw.rs is the decoder the shipped build uses for H.264; VAAPI/HW is never constructed.** The engine itself supports 4:2:2 (Yuv422), 10-bit (`bit_depth_luma_minus8 == 2`), MBAFF and field pictures internally (rust_h264 decoder.rs:22-47, 417-420, 173-177) — but this wrapper refuses 4:2:2 and >8-bit at the output boundary with the typed `h264.sw.surface_format` (L370-374), so the shipped surface is 8-bit 4:2:0 only (High profile + B-frames verified bit-exact, lane D). Real defects are minor: the frame-slot pool is permanently defeated by a mid-stream resolution change, and leading Annex-B garbage is dropped without being counted, contradicting the module's damage-accounting claim.

## 2. What actually works (verified)
- `timeout 110 ./target/debug/deps/h264_sw_differential-c293eb5d418fbe53 --test-threads=2` → `17 passed; 0 failed` (35.5s), incl. `lane_d_high_ipb_{annexb,mp4}` (8x8 transforms + weighted pred, bit-exact vs live ffmpeg), `hostile_truncation_sweep_is_panic_free`, garbage-recovery lanes. `/usr/bin/ffmpeg` exists → the live-oracle branch (`ffmpeg_reference`, test file L253-272) executed, not just pinned MD5s.
- `file`-level wiring verified by rg: `H264SwDecoder::new` ← pipeline_host.rs:346; `pub mod sw` behind `codec-h264-sw` (h264/mod.rs:55-56) which is in `default`.

## 3. Findings

F-001 [S3 — pooled-slot pool is permanently defeated by a size change]
Where: sw.rs:599-640 (esp. 606-617, 635-638) + 420-446 (configure never clears `slots`)
Code:
```
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
```
```
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
```
What is wrong: a slot whose length differs from the new frame size is skipped forever — it is never resized or replaced, and once `slots.len() == FRAME_SLOTS` (4), the freshly allocated slot is never stored. After **one** mid-stream SPS resolution change (or a same-process new generation with a different size, since `configure` does not clear `slots` either), every subsequent frame takes the `fresh` path: a full `vec![0; len]` zero-fill allocation per frame (1080p ≈ 3 MiB/frame) for the rest of the stream. The "zero steady-state allocation" claim (L126-131, L595-598) holds only until the first size change.
Why it matters: per-frame multi-MB alloc+memset churn in the hot decode path of a real stream scenario (the repo's own corpus contains a param-midchange lane for the mpeg12 sibling).
What correct looks like: when a free (`get_mut`-able) slot has the wrong size, replace it (`slots[idx] = fresh`), or clear slots on size change / in `configure`.
Confidence: HIGH — Verification: read + traced.

F-002 [S4 — Annex-B leading garbage dropped silently, contradicting the damage-accounting claim]
Where: sw.rs:30-32 (claim), 470-472 (use)
Code:
```
//! the **damage
//! accounting** (every skipped unit is counted and reported as
//! [`DecodeOutcome::Recovered`], never a silent drop), and the whole
```
```
        let mut nals: Vec<&[u8]> = Vec::new();
        match self.framing {
            None => nals.extend(AnnexBNals::new(packet.data.as_slice())),
```
What is wrong: `AnnexBNals::next` skips to the first start code (`self.pos = next_start_code(self.data, 0)?`, parser/nal.rs:296-298) and yields nothing for the bytes it discarded — sw.rs never counts them. A packet with leading garbage before the first `00 00 01` loses those bytes with `skipped == 0` (outcome `NeedMoreInput`), which contradicts "every skipped unit is counted". The AVC arm does count framing damage (L481-485); only the Annex-B leading-garbage case is silent.
Confidence: HIGH — Verification: read (parser/nal.rs:279-313 cross-checked).

F-003 [S4 — configure() commits the generation before extradata seeding can fail]
Where: sw.rs:435-444
Code:
```
        self.ordered = OrderedDecoder::new();
        self.generation = Some(cfg.generation);
```
What is wrong: `self.generation` is set before `seed_extradata` runs; if seeding fails (`?` at L443), the decoder is left half-configured: `generation` is `Some`, so a subsequent `decode` proceeds (L453-461 gate passes) against a rebuilt-but-unseeded engine instead of answering `h264.sw.not_configured`. Callers that treat configure-Err as fatal never hit this, but the state machine is wrong on the error path.
What correct looks like: set `generation` last, or reset to `None` on the error path.
Confidence: HIGH — Verification: read.

F-004 [S4 — per-packet NAL Vec + zeroed fresh slots]
Where: sw.rs:470-472, 618
Code:
```
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
```
What is wrong: every packet allocates a `Vec<&[u8]>` of its NAL slices (small, but per-packet), and fresh slots are zero-filled only to be immediately overwritten by `fill` (a wasted `memset` of the whole frame). Trivial, bounded, but pointless work in the hot path.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Debug::fmt | 137-153 | REAL | — | truthful label rust_h264-0.6.1 |
| Default::default | 155-169 | REAL | — | BT709_LIMITED family default |
| new | 175-177 | REAL | — | shipped constructor (pipeline_host.rs:346) |
| accepts | 182-184 | REAL | — | h264/avc1/avc1.* family |
| unsupported | 187-198 | REAL | — | typed h264.* Unsupported |
| parse_error | 203-212 | REAL | — | carries source |
| engine_refusal | 217-229 | REAL | — | engine rule string in source — honest why |
| seed_extradata | 235-258 | REAL | — | avcC switch + Annex-B CodecPrivate; feeds SPS/PPS typed |
| bridge_nal | 265-273 | REAL | — | reusable RBSP buffer, EPB unescape errors typed to caller |
| feed_param_set | 279-309 | REAL | — | SPS facts adopted only after engine accepts |
| adopt_sps_color | 314-355 | REAL | — | VUI map; 6→Bt709 posture documented; unspecified keeps current |
| emit_frame | 360-416 | REAL | — | 8-bit-420 gate (typed refusal for 4:2:2/10-bit); even-size crop reasoning correct for 4:2:0; bounds-checked copies; pooled slot |
| Decoder::configure | 420-446 | REAL | S4 | F-003 generation-before-seed; slots not cleared (F-001) |
| Decoder::decode | 448-562 | REAL | S4 | both framings, damage counting (except F-002 case), Unsupported fatal-per-generation honest |
| Decoder::flush | 564-592 | REAL | — | drains OrderedDecoder with PTS metas |
| pooled_frame_slot | 599-640 | SHALLOW | S3 | F-001 stale-size pool defeat; fresh-slot exclusivity error is loud, not fabricated |

## 5. Test honesty (tests live in tests/h264_sw_differential.rs — cited, not my file)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lane A-F × {annexb, mp4} (12) | YES | pinned MD5 + **live ffmpeg 7.1.5 oracle re-check** (ran: ffmpeg on PATH); wrong pixel/size/order fails |
| hostile_truncation_sweep_is_panic_free | YES | off-by-one truncation sweep across fixtures |
| garbage / recovery / contract-state lanes (4) | YES | Recovered accounting, typed refusals, unconfigured decode/flush |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact against ffmpeg 7.1.5's own libx264 decode on BOTH entry forms" | sw.rs:17-19 | TRUE (executed) | 17/17 differential pass with live oracle branch active |
| "4:2:2 and over-8-bit surfaces refuse with the typed h264.sw.surface_format" | sw.rs:51-53 | TRUE | emit_frame L370-374 |
| "every skipped unit is counted ... never a silent drop" | sw.rs:30-32 | FALSE (Annex-B leading garbage) | F-002 |
| "zero steady-state allocation" (RBSP) | sw.rs:126-131 | TRUE for RBSP / FALSE for slots after size change | F-001 |
| rust_h264 "pure Rust, zero dependencies, no build script, no FFI" | sw.rs:10-12 | TRUE | registry src listing: no build.rs, deps = none in Cargo.toml (checked registry cache) |
| "Interlaced-source pictures ... weaved into full frames by the engine and flagged interlaced" | sw.rs:48-50 | REAL-UNPROVEN | rust_h264 has MBAFF/field support (decoder.rs:173-177) but no PAFF/MBAFF lane exists in the differential — UNVERIFIED claim |

## 7. Dead / duplicated / unreachable code
None. Every function is on the shipped path.

## 8. Dependencies & flags
- `rust_h264 = { version = "=0.6.1", optional = true }` (crates/codecs/Cargo.toml:174), feature `codec-h264-sw` in `default` — shipped ON in the release lane (fingerprint evidence in target/release). Engine supports 4:2:2 + 10-bit + MBAFF/field internally; wrapper surface is 8-bit 4:2:0.

## 9. Open questions for the next agent
- PAFF/MBAFF end-to-end behavior: engine supports it, no differential lane exercises it (doc claim UNVERIFIED). A field-picture fixture lane would settle it.
- `EngineNalType::from(header.unit_type.id())` numeric-id mapping to the engine's enum — assumed aligned (rust_h264 nal.rs); not diffed exhaustively.
