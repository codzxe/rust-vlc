# AUDIT: crates/codecs/src/video/h264/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 794 / 26 (3 Decoder trait methods, 12 private helpers, 3 ctors, 2 projections, 2 error mappers, Debug/Default) |
| File verdict | REAL-UNPROVEN (hw path) + UNREACHABLE from shipped build |
| Reachable from shipped entry point? | NO — H264Decoder is never constructed outside crates/codecs tests; shipped registry uses the software decoder (see F-001 chain) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | sed/rg verification of every quote; executed h264_hw_path suite (20/20, covers this file's Decoder impl via fake adapter); rg construction sites for H264Decoder across apps/ and crates/ |

## 1. One-paragraph verdict
decode.rs is the H.264 HARDWARE-path Decoder: avcC/Annex-B extradata in → in-tree parser → §7.4.1.2.4 access-unit assembly → hw session seam → opaque-handle VideoFrames to the codec-api FrameSink. It is NOT a rust_h264 wrapper — that is sw.rs (`H264SwDecoder`, feature `codec-h264-sw`); decode.rs is pure in-tree code plus the hw seam, and it contains no software fallback (typed `h264.hw.required`/`h264.hw.no_bridge` refusals instead — matches ADR-001 and the matrix row). The code is genuinely implemented: bounded pending (MAX_PENDING_UNITS=4096 at both push sites), generational stale-drop, counted damage recovery, typed error taxonomy, no unwrap/expect/index panics on parsed input. BUT it is dead in the shipped product: apps/cli's registry constructs `H264SwDecoder` for h264/avc1, `H264Decoder::from_setting` appears only in a comment, and the `playback.hw-decode-policy` setting is validated by the schema yet read by nothing at runtime — the hw decoder is UNREACHABLE from lumen-cli/desktop, and the policy setting is wired to nothing. Three correctness/doc defects: the non-VCL attachment rule contradicts its own doc in the open-picture flow; the parse_vcl "ambiguity" branch is dead code with a misleading comment; POC type-1 streams degrade (documented).

## 2. What actually works (verified)
— verified by executing `../../target/debug/deps/h264_hw_path-f4e0fd72a3109215 --nocapture --test-threads=1` -> 20/20 ok: configure (avcC + Annex-B + in-band), AU assembly (boundary rule, multi-slice, AUD/EOS close, end-of-AVC-sample close), pts plumbing, stale drop, damage counting, typed hw errors, policy trail — all pinned against this file's impl via the fake adapter.
— verified by `rg -n "fn drop_stale_packet" crates/codec-api/src/decode.rs` -> L203: real shared guard; decode.rs calls it (L631).
— verified by `rg -n "pic_parameter_set_id" crates/codecs/src/video/h264/parser/slice.rs` -> L953 `pic_parameter_set_id: pps_id as u8`: bitstream-read, context-independent (basis of F-003).
— verified by `sed -n '252p' crates/codecs/src/video/h264/parser/sps.rs` -> `let _always_zero = r.bit()?;` — the SPS parser discards the flag exactly as decode.rs docs admit (basis of F-004).

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/video/h264/decode.rs:310-319 (from_setting) vs apps/cli/src/pipeline_host.rs:342-346
Code: (verbatim, pipeline_host.rs)
```
            // `software-only` default resolves here, and the decoder's
            // own `configure` gate stays the honest authority.
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```
What is wrong: the entire hw decode path in this file is unreachable from the shipped entry points. Call-chain proof: lumen-cli open -> playback-engine pipeline `decoder_for` seam -> apps/cli/src/pipeline_host.rs:346 constructs `H264SwDecoder` for "h264"/"avc1" — `H264Decoder` is constructed nowhere outside crates/codecs tests (rg over apps/ + crates/: only this comment mentions it). The `playback.hw-decode-policy` setting exists in the schema (crates/settings-schema/src/runtime.rs:216 `schema_meta("playback.hw-decode-policy")`) and its values are validated in product-config tests, but NO runtime code reads the policy to select this decoder — a user setting `prefer-hardware` still gets the software decoder. Why it matters: dead shipped code + a UI/settings knob wired to nothing (spec 6.5); the matrix row honestly says "partial pending Phase-7 runtime activation (ADR-013)", so this is not fraud — but the file verdict for the shipped build is UNREACHABLE. What correct looks like: registry reads the resolved policy and constructs `H264Decoder::from_setting(policy)` for the hw lane (falling back typed), or the setting is removed until Phase 7. Confidence: HIGH — Verification: rg construction sites (no hits outside codecs), read of both files.
F-002 [S3]
Where: crates/codecs/src/video/h264/decode.rs:29-31 (doc) vs 446, 690-691, 701-709 (code)
Code: (verbatim doc)
```
//! one AVC sample is one access unit). Non-VCL units attach to the
//! picture that follows them; in-band SPS/PPS activate before the
//! slices that follow and are re-offered to the session immediately.
```
and code L446:
```
        let units = std::mem::take(&mut self.pending);
        self.pending_key = None;
```
What is wrong: when a non-VCL unit (e.g. SEI) arrives while a picture is still open, it is appended to `pending` and then closed INTO THAT PICTURE'S access unit by the next boundary VCL (`close_picture` takes all of `pending`); it does not attach to the picture that follows. The inline comment at L703-707 ("Non-VCL units already pending ... sit in `pending` ahead of it") describes the carry-over that only happens when the previous picture was ALREADY closed (AUD/EOS delimiter or end-of-AVC-sample) — in the open-picture flow the units were consumed by the previous AU. Doc and inline comment contradict the code for that flow. Why it matters: hardware sessions receive SEI at the tail of the wrong AU (usually tolerated — SEI is advisory — and SPS/PPS ACTIVATION is immediate and correct regardless of raw-byte placement, L675-685), but the documented assembly rule is false as stated. What correct looks like: either buffer trailing non-VCL units across the boundary close, or fix both docs to state the actual rule. Confidence: HIGH — Verification: read + traced all three flows.
F-003 [S3]
Where: crates/codecs/src/video/h264/decode.rs:513-526
Code: (verbatim)
```
        let mut best: Option<(SliceHeader, SliceNalInfo)> = None;
        let mut ambiguity = false;
        for (pps_id, sps_info, pps_info) in candidates {
            if let Ok(slice) = parse_slice_header_pub(&self.rbsp, nal, &sps_info, &pps_info)
                && slice.pic_parameter_set_id == pps_id
                && best.replace((slice, nal)).is_some()
            {
                // Two contexts both parse and round-trip-match: the
                // slice is ambiguous -- refuse rather than guess.
                ambiguity = true;
                break;
            }
        }
        if ambiguity { None } else { best }
```
What is wrong: `pic_parameter_set_id` is read from the bitstream at a context-independent position (slice.rs:953 — it precedes all SPS/PPS-dependent syntax), so `slice.pic_parameter_set_id == pps_id` can hold for at most ONE candidate (the PPS whose id the slice itself declares; HashMap keys are unique). The `ambiguity` branch is unreachable dead code and its comment ("Two contexts both parse and round-trip-match") describes an impossible state. Also a per-VCL `candidates` Vec allocation (L504) for a loop that needs at most the one matching PPS. Why it matters: misleading invariant + wasted allocation per slice NAL; a reader believes multi-context ambiguity is handled when it cannot occur. What correct looks like: look up `self.pps` by the slice's declared id directly (parse the id first, then the header under that single context). Confidence: HIGH — Verification: read + parser source check.
F-004 [S3]
Where: crates/codecs/src/video/h264/decode.rs:22-26, 98-100, 106 vs parser/sps.rs:252
Code: (verbatim, sps.rs)
```
                let _always_zero = r.bit()?; // delta_pic_order_always_zero_flag
```
What is wrong: the SPS parser discards `delta_pic_order_always_zero_flag`, so the slice projection hardcodes `delta_pic_order_always_zero_flag: false` (L106); POC-type-1 streams with the flag set mis-parse slice headers and surface as counted damage instead of correct AU assembly. Honestly documented (L22-26 "recorded as a parser follow-up, not papered over"). Rare in practice (x264/ffmpeg default to POC type 0/2). S3 as a real decode gap with doc admission. Confidence: HIGH — Verification: read of both files.
F-005 [S4]
Where: crates/codecs/src/video/h264/decode.rs:424-427
Code: (verbatim)
```
        let stream = H264HwStreamInfo {
            framing: Some(self.framing),
            sps: self.sps.values().cloned().collect(),
            pps: self.pps.values().cloned().collect(),
```
What is wrong: every parameter-set (re)activation deep-clones ALL stored SPS/PPS NALs (Vec per entry); in-band-SPS streams re-offer per keyframe. Plus per-packet `nals: Vec<&[u8]>` (L638) and per-NAL `nal.to_vec()` (L481, L713). The `rbsp` buffer is reused (L224 "zero steady-state allocation" is true for RBSP only). Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/codecs/src/video/h264/decode.rs:651-657
Code: (verbatim)
```
                                Err(_) => {
                                    // Framing damage: the splitter has
                                    // terminated itself; count and keep
                                    // what did split.
                                    self.skipped += 1;
                                    break;
                                }
```
What is wrong: the entire unsplit remainder of a damaged AVC packet counts as ONE skipped unit regardless of how many NALs it contained — coarse but monotone accounting; the Recovered skip count under-reports damage units. Confidence: HIGH — Verification: read.
F-007 [S4]
Where: crates/codecs/src/video/h264/decode.rs:464, 470, 777, 783
Code: (verbatim, L464)
```
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
```
What is wrong: if the internal invariant (format set before any frame emission) ever broke, frames would be silently mislabeled Nv12 (and generation defaulted to FIRST) instead of erroring. Analysis shows the invariant holds post-configure (any parseable VCL implies a synced SPS/PPS, and sync errors are typed at L421), so these are unreachable fallbacks — defensive masks, not bugs. Confidence: HIGH — Verification: read + flow trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unsupported | 85-96 | REAL | - | typed Unsupported factory |
| slice_sps_info | 101-111 | REAL | S3 | hardcodes delta flag false (F-004) |
| slice_pps_info | 114-126 | REAL | - | u8→u32 widening casts safe |
| surface_format | 130-140 | REAL | - | THE gate: mono/420→Nv12, 422→Yuyv422, 444 & >8bit→None (typed h264.hw.surface_format at L421). NOTE: NO profile/level gating in this file — profile refusal is delegated to the hw session's configure; shipped sw path gates in sw.rs (h264.sw.unsupported) |
| PictureKey::of | 158-173 | REAL | - | §7.4.1.2.4 fields incl. POC type-0 |
| Debug for H264Decoder | 230-246 | REAL | - | exposes pending_units — the data the hw-path test SHOULD have asserted against (test file F-001) |
| Default / new | 248-280 | REAL | - | zero-adapter truthful state |
| with_bridges | 285-290 | REAL | - | test plug point |
| from_setting | 310-319 | REAL | S2 | implemented + unit-tested, but NO shipped caller (F-001) |
| hw_events | 326-328 | REAL | - | ADR-001 report surface, pinned by tests |
| accepts | 332-334 | REAL | - | h264/avc1/avc1.* |
| seed_extradata | 337-363 | REAL | - | avcC sniff → Avc framing + SPS/PPS seed; else Annex-B CodecPrivate walk |
| store_sps / store_pps | 367-392 | REAL | - | typed parse errors, hw_configured invalidation |
| active_sps | 398-404 | REAL | - | lowest-pps-id heuristic, doc'd |
| sync_hw_config | 407-436 | REAL | S4 | PS clone churn (F-005); surface_format refusal typed here |
| close_picture | 440-475 | REAL | S3 | non-VCL tail attachment (F-002); no-VCL carry-over correct |
| push_opaque | 478-488 | REAL | - | bounded by MAX_PENDING_UNITS |
| parse_vcl | 494-527 | REAL | S3 | dead ambiguity branch + per-VCL alloc (F-003); context round-trip check itself sound |
| parse_slice_header_pub | 532-539 | REAL | S4 | pointless indirection (admits it in its own comment) |
| parse_error / hw_error | 543-571 | REAL | - | DecodeCause::Bitstream / HardwareLost mapping pinned by tests |
| Decoder::configure | 574-615 | REAL | - | full per-generation reset; engine accel precedence over negotiated; typed open failures |
| Decoder::decode | 617-760 | REAL | S4 | total over any bytes; outcome accounting (Emitted/Recovered/NeedMoreInput) exact; AVC damage granularity F-006 |
| Decoder::flush | 762-793 | REAL | - | closes pending + drains session; Drained-on-empty pinned |

## 5. Test honesty
No tests inside this file. External coverage: crates/codecs/tests/h264_hw_path.rs (20 tests, executed 20/20 — would fail on real regressions in configure/decode/flush/assembly/staleness/damage; its own defects are logged in its audit: tautological bound assert, self-skipping corpus lane).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no software fallback anywhere in this file ... typed Unsupported ... never a silent skip, never a fabricated frame" | header L11-15 | REAL | configure L605-609 typed refusals; no sw path in file; hw_path tests pin codes |
| "Non-VCL units attach to the picture that follows them" | header L29-30 | FALSE (for the open-picture flow) | F-002 trace: close_picture consumes them into the previous AU |
| "a conformant picture is orders of magnitude below" MAX_PENDING_UNITS=4096 | L75-78 | REAL-UNPROVEN | cap enforced (L479/L711); conformance bound not re-derived (would need spec table check) |
| hw row "partial pending the Phase-7 runtime activation (ADR-013)" | docs/feature-matrix.toml:416 | REAL | matches F-001: no shipped activation |
| mod.rs: sw path is the shipped one, "codec-h264-sw, default-on" | h264/mod.rs L8-27 | REAL | pipeline_host.rs:346 constructs H264SwDecoder |

## 7. Dead / duplicated / unreachable code
- The entire H264Decoder hw path is UNREACHABLE from shipped entry points (F-001) — reachable only from tests (h264_hw_path.rs).
- parse_vcl ambiguity branch: dead (F-003).
- parse_slice_header_pub: trivial indirection.

## 8. Dependencies & flags
In-tree only: codec_api (Decoder, FrameSink, drop_stale_packet), media_types, super::parser, super::hw. No rust_h264 dependency here (that is sw.rs behind `codec-h264-sw`). hw lanes (videotoolbox/vaapi) are compile-verified only per matrix note — runtime hw decode never exercised in sandbox.

## 9. Open questions for the next agent
- Who owns Phase-7 activation (ADR-013)? If it lands, pipeline_host must read the policy — until then the settings knob is dead.
- h264/mod.rs truth table (hw/mod.rs) claims VA-API "compile-verified + failure-path runtime-tested"; my file 10 (hw/vaapi/library.rs) should verify that claim.
- A conformance check of MAX_PENDING_UNITS vs worst-case legal pictures (e.g., FMO many-slice) was not performed — UNVERIFIED.
