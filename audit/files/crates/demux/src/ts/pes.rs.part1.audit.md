# AUDIT: crates/demux/src/ts/pes.rs#part1 (lines 1-617)
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-21 |
| Lines / functions | 617 (chunk 1-617 of 1235; part2=618-1235 is W05-A04's; `mod tests` at 822-1235 lies in part2) / 31 items |
| File verdict | REAL (assembly core genuine + baseline-executed; discontinuity/PCR surfaces unreachable from the shipped wiring) |
| Reachable from shipped entry point? | YES for the assembly core (call chain: `lumen open x.ts` → `apps/cli/src/demux_host.rs:458 containers::open` → `apps/cli/src/containers/mod.rs:66 "mpegts" => ts::open` → `apps/cli/src/containers/ts.rs:112 open` → `PesBridge::new` (ts.rs:241) → `PesBridge::push` (ts.rs:258) → `PesAssembler::push_packet`). NO for `observe_pcr` and the `discontinuity=true` paths (F-002/F-004) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 6 S4: 2 |
| Commands run | full Read of pes.rs 1-1235 (context; audited 1-617); `rg -n "PesAssembler"` → 3 referencing files (apps/cli/src/containers/ts.rs, crates/demux/tests/ts_pes_corpus.rs, crates/codecs/tests/mpeg12_corpus.rs); `rg -n "data_alignment"` repo-wide → only consumer is ts.rs:341 `data_alignment: _` (discarded); `rg -n "observe_pcr"` → only test callers; `rg -n "last_pcr"` in ts/mod.rs → set at :880, dead getter :647; `rg -n "Doc-tests demux"` + `sed -n '6362,6375p'` on orchrun-test-workspace-nofailfast.log.txt → pes.rs doctest line 69 **ok**; `sed -n` on every quoted range; hand-verification of 33-bit PTS/DTS reassembly arithmetic and of Table 2-18/2-17 bit layouts |

## 1. One-paragraph verdict
This is the PES half of the TS stack and the parse/assembly core is real and baseline-executed (18/18 unit tests ok, doctest ok at nofailfast:6371, 4/4 corpus differential tests ok at nofailfast:2213) — but three of its advertised honesty mechanisms are theater in the shipped build. The 33-bit PTS/DTS decode and Table 2-18 stream classification check out by hand arithmetic, yet `data_alignment_indicator` is read from the wrong flag byte (byte 7 bit 2 = `DSM_trick_mode_flag`'s position, not byte 6 bit 2 where ISO/IEC 13818-1 §2.4.3.7 puts it), the test factory bakes the same wrong byte in so the suite pins the error instead of the spec, and the PCR-regression watchdog (`observe_pcr`/`PcrRegressed`) plus the entire `discontinuity_indicator` parameter path have zero production callers — the one shipped caller hard-codes `false`, so the module doc's "a regressed PCR … is a TimingEvent … never silent, MP §8" promise is false for every binary this repo ships. A mid-PES discontinuity flag is additionally clobbered by an overwrite ordering bug before the packet it belongs to is completed.

## 2. What actually works (verified)
- `decode_ts5` (253-266): hand-verified the 33-bit reassembly `(b0&0x0E)<<29 | b1<<22 | (b2&0xFE)<<14 | b3<<7 | (b4&0xFE)>>1` — bit positions land exactly at 32..30, 29..22, 21..15, 14..7, 6..0; marker bits b0/b2/b4 bit0 and the 4-bit prefix are all validated before use; short slice → typed `demux.ts.pes.short-header`. `encode_ts5` (271-280, test-only) is the exact inverse — roundtrip-pinned by `ts5_roundtrip_and_marker_guard` (baseline ok, nofailfast:1880).
- `PesHeader::parse` structural spine (312-358): start-code check guarded by `len < 6` short-circuit (no slice panic); `total_len = 6 + length` with 0 = unbounded (None) per §2.4.3.7; PTS-only `0b10` → field at byte 9, PTS+DTS `0b11` → DTS at byte 14; forbidden `0b01` refused (356); header-data-length completeness checked (343-345) before any field read.
- `classify_stream_id` (234-247) vs ISO/IEC 13818-1 Table 2-18 base entries: 0xBC/0xFF StreamMap, 0xBD Private1, 0xBE Padding, 0xBF Private2, 0xC0..=0xDF Audio, 0xE0..=0xEF Video, 0xF0/0xF1 Entitlement (ECM/EMM), 0xF2/0xF3 Interactive (DSM-CC/MHEG) — all match; pinned by `stream_id_classification_table` (baseline ok).
- `push_packet` assembly spine (524-584): stray continuation before any start → damage + `pes.stray-continuation` Recovered record (558-566); accumulation ceiling `MAX_PES_BYTES` = 8 MiB enforced with `checked_add` on buf+payload (567-577) → typed `demux.ts.pes.payload-over-ceiling` (ResourceLimit, never an unbounded read); bounded/unbounded completion semantics (unbounded completes only at next start / EOS flush) — all pinned by baseline-ok tests (`payload_ceiling_is_the_typed_refusal`, `non_pes_pid_goes_dormant…`, `stray_continuation…`, `unbounded_video_assembles_across_packets`).
- DTS/PTS regression watchdogs: the `last_dts`/`last_pts`/`seen_dts` state and event types are part1; emission in `complete_one` (631-660, part2) is wired in the shipped build — the bridge drains `take_timing_events` into `DemuxEvent::Discontinuity` (containers/ts.rs:286-289), and `pipeline.rs:2169-2174` counts them. Baseline-ok unit pins: `dts_regression_and_jump_are_events`, `pts_regression_without_dts_is_an_event_bframe_pts_with_dts_is_not` (a genuinely subtle B-frame-vs-damage policy, correct).
- Doctest (69-94): executed and passed — `test crates/demux/src/ts/pes.rs - ts::pes (line 69) ... ok` (orchrun-test-workspace-nofailfast.log.txt:6371).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/ts/pes.rs:339-342
Code:
```rust
        let flags2 = bytes[7];
        let data_len = usize::from(bytes[8]);
        header.header_len = 9 + data_len;
        header.data_alignment = flags2 & 0x04 != 0;
```
What is wrong: `data_alignment_indicator` is read from **byte 7** (the PTS_DTS/ESCR/ES_rate/DSM_trick flag byte) with mask `0x04` — byte 7 bit 2 is `DSM_trick_mode_flag`. Per ISO/IEC 13818-1 §2.4.3.7 (Table 2-17 syntax), `data_alignment_indicator` is bit 2 of **byte 6** ('10' + scrambling + priority + data_alignment + copyright + original_or_copy). `bytes[6]` is never read anywhere in `parse`. A real muxed PES header `00 00 01 E0 … 84 80 05 …` (ffmpeg's aligned-video convention: byte 6 = 0x84, byte 7 = 0x80) parses `data_alignment = false`; it is only ever `true` when a DSM trick-mode flag is set.
Why it matters: the field is public API on `PesHeader` and `PesPacket`, documented as "the payload starts at an access-unit boundary (a decoder-side hint, surfaced honestly)" (291-293) — the surfaced value is the wrong bit of the wrong byte. The unit-test factory bakes the error in: `let mut flags2 = 0x04u8; // data_alignment_indicator set` (pes.rs:839, part2) places the bit in byte 7, so the suite pins the misunderstanding instead of the spec. The only consumer today discards it (`apps/cli/src/containers/ts.rs:341 data_alignment: _`), which is the sole reason this is S3 and not S2: the wrong value flows nowhere — until any decoder consumes the documented hint, at which point every stream with trick-mode-flagged packets reports spurious alignment.
What correct looks like: `header.data_alignment = bytes[6] & 0x04 != 0;` (plus optionally validating the `'10'` marker in byte 6 bits 7-6), and a test fixture with byte 6 = 0x84 / byte 7 = 0x80 (the real-world shape) asserting `data_alignment == true`.
Confidence: HIGH — Verification: read; spec layout hand-checked against §2.4.3.7; consumer discard verified by `rg -n "data_alignment"` (single consumer, destructured `_`).

### F-002 [S2]
Where: crates/demux/src/ts/pes.rs:592-613 (code), 26-31 (doc claim), evidence: rg "observe_pcr" → only crates/demux/tests/ts_pes_corpus.rs:202,518 and pes.rs's own tests
Code:
```rust
//! `discontinuity_indicator`, or a regressed PCR is a
//! [`TimingEvent`] on the drain queue (mapped by the demuxer to
//! `DemuxEvent::Discontinuity` — never silent, MP §8). B-frame PTS
```
```rust
        if let Some(prev) = self.last_pcr.replace(pcr).filter(|prev| pcr < *prev) {
            self.timing.push_back(TimingEvent::PcrRegressed {
                from: prev,
                to: pcr,
            });
        }
```
What is wrong: `observe_pcr` has **zero production callers**. `rg -n "observe_pcr"` across the repo returns only `crates/demux/tests/ts_pes_corpus.rs:202,518` (the test-only transport walker) and pes.rs's own unit test (1151-1164). The shipped TS path (`apps/cli/src/containers/ts.rs`) never feeds PCR into the assembler; the demuxer's transport walk stores `self.last_pcr = af.pcr` (crates/demux/src/ts/mod.rs:880) with no regression check, and its getter `last_pcr()` (ts/mod.rs:647-648) has no callers either. So `PcrRegressed` can never fire in any shipped binary, and the module doc's promise that a regressed PCR is "a TimingEvent on the drain queue … never silent, MP §8" is false for the shipped wiring — a clock-damaged TS (regressed PCR, no discontinuity flag) produces zero observability.
Why it matters: the doc's headline honesty contract ("never silent") is the file's stated reason to exist (§"Timing (the honest contract)"); an auditor or maintainer reading it would believe PCR regression is detected at runtime. It is detected only inside a test harness. The corpus test itself confirms the lanes are all-clean ("no `discontinuity_indicator` bits set, PCR monotone", ts_pes_corpus.rs:38-39), so even the test evidence never exercises a real regression.
What correct looks like: either the shipped `PesBridge` hands the assembler PCR observations from the demuxer's adaptation-field walk (the demuxer already parses `af.pcr` at ts/mod.rs:879-881 — one call site), or the doc claims only what the shipped wiring delivers.
Confidence: HIGH — Verification: read + repo-wide caller greps; no build run needed to prove absence of callers.
(PCR wraparound note, folded here: `pcr < prev` is not modular — the 2^33×300-tick PCR clock wraps every ~26.5 h, firing one spurious `PcrRegressed` per wrap. S4-grade given ~26 h exposure, and moot while the function is unwired.)

### F-003 [S3]
Where: crates/demux/src/ts/pes.rs:535-548 with 578-581
Code:
```rust
            if start {
                state.dormant = false;
                state.disc = discontinuity;
                !state.buf.is_empty()
            } else {
                false
            }
        };
        let dormant = self.states.get(&pid).map(|s| s.dormant).unwrap_or(false);
        if had_pending {
            self.complete_one(pid);
        }
```
```rust
        state.buf.extend_from_slice(payload);
        if discontinuity {
            state.disc = true;
        }
```
What is wrong: a discontinuity flagged on a **continuation** packet sets `state.disc = true` for the PES currently being assembled (line 580). But when that PES is later closed by the **next start** (the only way an unbounded video PES completes mid-stream), line 539 `state.disc = discontinuity` executes *before* `complete_one` (547) reads the flag (`std::mem::replace(&mut state.disc, false)` at 623) — so a closing start packet with `discontinuity == false` clobbers the in-flight `true` and the completed video PES is emitted with `discontinuity == false`. The 579-581 write is only ever observed on the bounded-completion (`try_complete`) and EOS-flush (`finish`) paths.
Why it matters: the unbounded PES is "the video convention" (doc, line 10-11) — the most common packet shape in the layer — and a splice flagged mid-packet is exactly the clock-reset boundary the field documents ("timestamps before it and after it must not be arithmetically mixed", 402-405). The only unit test (`discontinuity_surfaces_as_event_and_packet_flag`, 1070-1084) covers discontinuity-on-start only, so the loss is unpinned. Mitigations that keep this at S3: the `TimingEvent::DiscontinuityIndicator` still fires independently (531-534) on every flagged packet, and in the shipped build this entire path is dead anyway (F-004).
What correct looks like: on start, fold rather than overwrite (`state.disc |= discontinuity`) and let `complete_one` take it — or move the completion read before the new-packet state reset.
Confidence: HIGH on the mechanism (traced both completion paths); MEDIUM on real-world impact (no shipped caller passes `true`).
Verification: read + path trace; unit coverage gap verified by reading test 1070-1084.

### F-004 [S3]
Where: crates/demux/src/ts/pes.rs:60-63 (doc), 531-534, 539, 579-581 (machinery); shipped caller: apps/cli/src/containers/ts.rs:258-259
Code:
```rust
//! the adaptation field are `00 00 01 E0`). The transport layer (I1a)
//! parses the adaptation field and hands this layer the
//! `discontinuity_indicator` bit and PCR observations
//! ([`PesAssembler::observe_pcr`]). The deliberately small
```
```rust
        self.assembler.push_packet(pid, pusi, false, payload)?;
```
What is wrong: the doc's seam description — "The transport layer (I1a) parses the adaptation field and hands this layer the `discontinuity_indicator` bit and PCR observations" — describes the **test harness's** walker (ts_pes_corpus.rs:14-16, 202-211), not the shipped one. The single production caller hard-codes `discontinuity = false` (containers/ts.rs:258, with its own comment explaining the demuxer owns that event). Consequence: `push_packet`'s `discontinuity` parameter, the `DiscontinuityIndicator` timing event (531-534), `Assembly.disc`, and `PesPacket.discontinuity` are unreachable-as-`true` from the shipped CLI. The redundancy is deliberate and defensible (the demuxer emits its own `DemuxEvent::Discontinuity` on `af.discontinuity`, ts/mod.rs:882-885 — no duplicate events), but `PesPacket.discontinuity` is then minted into `Packet.discontinuity` (containers/ts.rs:452) where it is **always false** for TS, and that engine-level field has no readers (rg `.discontinuity` over playback-engine/media-clock/video-engine/transcode → none; only `DemuxEvent::Discontinuity` is consumed, pipeline.rs:2169).
Why it matters: two parallel discontinuity mechanisms (demuxer-owned event vs assembler-owned flag), of which the shipped build activates only one; the module doc documents the inactive one as the seam contract. Dead parameter plumbing invites the F-003 class of bug to survive forever because no shipped path can ever expose it.
What correct looks like: either delete the assembler's discontinuity surface (the demuxer owns it) and fix the doc, or pass the bit through the `PesConsumer` seam so the flag is live and tested against the shipped wiring.
Confidence: HIGH — Verification: read + grep (sole caller hard-codes false; `Packet.discontinuity` consumers: none).
Verdict for the surface: UNREACHABLE (from shipped entry point).

### F-005 [S3]
Where: crates/demux/src/ts/pes.rs:359-367
Code:
```rust
        // The optional header must not swallow the whole declared
        // packet: a header-only "elementary stream" PES is
        // structurally broken (stuffing belongs on padding_stream).
        if header
            .total_len
            .is_some_and(|total| header.header_len >= total)
        {
            return Err(short_header());
        }
```
What is wrong: a bounded PES whose declared length exactly covers the optional header (`header_len == total`, i.e. zero elementary-stream payload) is refused as `demux.ts.pes.short-header`. The spec sets no minimum payload for PES packets, and PES-header stuffing is its own mechanism (0xFF stuffing bytes inside the `PES_header_data_bytes` area, §2.4.3.7) — distinct from `padding_stream` (0xBE) packets, which the comment conflates ("stuffing belongs on padding_stream"). Mid-stream, such a packet is dropped and counted as `pes.header-damage` (complete_one's Err arm, 678-689) rather than emitted as a legitimate zero-payload timestamp carrier.
Why it matters: legal-but-rare shapes (splice-point marker PES carrying only PTS, audio muxers emitting header-only packets at AU boundaries) are silently reclassified as damage. The `>` case of `>=` is genuinely broken (header alone exceeds the declared packet) and should stay; the `==` case is the overreach.
What correct looks like: refuse only `header_len > total`; emit zero-payload PES for the `==` case (or at minimum count it under a distinct reason string so a corpus differential could distinguish spec-legal markers from damage).
Confidence: MEDIUM — the spec text sets no minimum and defines in-header stuffing, but I have no executed corpus lane containing a zero-payload PES to prove real-world incidence. The pinned test `header_swallowing_the_declared_packet_is_refused` (1057-1067) only proves the refusal happens for the genuinely-broken `>` shape (header_len 19 > total 12) — it never even constructs the `==` case.
Verification: read; spec reasoning; test body read.

### F-006 [S4]
Where: crates/demux/src/ts/pes.rs:137-147
Code:
```rust
pub fn short_header() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 0,
                found: 0,
            },
        },
        "demux.ts.pes.short-header",
    )
}
```
What is wrong: the error detail carries placeholder zeros — `expected: 0, found: 0` for every short-header occurrence (structurally-short header, forbidden flags, header-swallows-packet), so the typed `Truncated` detail conveys no information. The `truncated(expected, found)` constructor (163-170) exists and is used properly elsewhere; this one just fills the shape.
Why it matters: the error taxonomy's selling point is context (MP §8 honesty); a `Truncated` detail that always says 0/0 is a lie of precision — it claims to know the extent and reports nothing.
What correct looks like: parameterize `short_header(have: usize, need: usize)` at the four call sites, or use `InvalidStructure` instead of `Truncated` for the non-length failures.
Confidence: HIGH — Verification: read; all call sites in-file (255, 337, 344, 356, 366).

### F-007 [S3]
Where: crates/demux/src/ts/pes.rs:546-548 (trigger; the missing check lives in `complete_one`, 618-692, part2/W05-A04's chunk — cross-chunk finding, flagged for A04)
Code:
```rust
        let dormant = self.states.get(&pid).map(|s| s.dormant).unwrap_or(false);
        if had_pending {
            self.complete_one(pid);
        }
```
What is wrong: when a bounded PES is closed early by the next start (declared `total_len` never reached — the transport lost packets), `complete_one` parses the buffer, emits the packet with whatever partial payload arrived, and **never compares `buf.len()` against `total_len`**; no `Recovered` record, no damage count, no packet marker. The shortfall check exists only in the EOS flush (`finish`, 770-787: `Some(total) if state.buf.len() < total` → typed truncation) and the overrun check only in `try_complete` (737-749). So: lost-packet truncation mid-stream = silent; the same condition at EOS = typed sticky error. The same silent path applies to an unbounded video PES whose bytes were lost (gap inside the payload, no marker).
Why it matters: realistic damage (terrestrial/capture loss) yields a short access unit passed downstream as a complete packet. Mitigations that hold this at S3: the demuxer's continuity-counter jump check (ts/mod.rs:861-863: `cc_jumps`, `DemuxEvent::Discontinuity`) surfaces the *transport* loss, and the ADTS AU-split consumer independently handles trailing partial AUs (`end > payload.len()` → honest stop, containers/ts.rs) — but the whole-packet lane (AC-3/private, and all video) still hands the decoder a truncated AU with no demux-level marker.
What correct looks like: in `complete_one`, when `header.total_len == Some(t)` and `buf.len() < t`, count the shortfall and emit a `Recovered { reason: "pes.short-of-declared" }` (or mark the packet) before emitting it.
Confidence: HIGH on the mechanism (all three completion paths traced); MEDIUM on incidence (needs a corpus lane with real packet loss — the corpus lanes are clean).
Verification: read + path trace across the 547/750/788 call sites.

### F-008 [S3]
Where: crates/demux/src/ts/pes.rs:244-245 (with 226-228)
Code:
```rust
        0xF4..=0xFE => PesStreamClass::Reserved,
        0x00..=0xBB => PesStreamClass::Reserved,
    }
```
What is wrong: `0xFD` (extended_stream_id — extended video/SVC-MVC streams per the H.222.0 amendment lineage; ffmpeg's mpegts/mpegpes treat 0xBD and 0xFD as the private/extended pair and parse full optional headers on both) is classified `Reserved`, so `has_optional_header()` returns false and a 0xFD PES is parsed as headerless: `header_len = 6`, `pts = None`, and the optional-header bytes (flags, `PES_header_data_length`, the 5-byte PTS fields) are delivered **as payload**. Against the base 13818-1 Table 2-18 the mapping is defensible (0xFD sits in the reserved band) — the doc comment (231-233) explicitly cites Table 2-18 — but every modern muxer that emits 0xFD emits it with the optional header.
Why it matters: an extended-video TS lane demuxes to garbage payload offsets and lost timestamps. Rare format, so S3 not S2 — the shipped PMT routing would still route the PID, and the mis-parse is silent (no error; PTS bytes in payload).
What correct looks like: `0xFD => PesStreamClass::Video` (or a dedicated `Extended` class with `has_optional_header() == true`), matching ffmpeg's observed behavior on extended streams.
Confidence: MEDIUM — Table 2-18 base-band conformance is exact; the 0xFD-with-optional-header behavior is established ffmpeg practice, but I could not execute an 0xFD lane to prove it in this sandbox (no corpus).
Verification: read; spec/ffmpeg reasoning; no run.

### F-009 [S4]
Where: crates/demux/src/ts/pes.rs:549-554 (with test 1004 as the documenting pin)
Code:
```rust
        if dormant {
            // Not a PES PID (or the current start is dead): the
            // payload is someone else's — counted, not accumulated.
            self.damage_bytes += payload.len() as u64;
            return Ok(());
        }
```
What is wrong: dormancy damage is added to `damage_bytes` but never surfaced as a `Recovered` record, unlike every other damage path (stray continuation 561-565, overrun 742-745, header damage 682-689). The bridge drains `take_recovered()` into `DemuxEvent::Recovered` (containers/ts.rs:276-281) — so a dormant PID's bytes are visible only through the aggregate `damage_bytes()` accessor, which the shipped bridge never reads (rg: no callers of `damage_bytes()` outside tests).
Why it matters: the module doc's damage policy (38-43) says damage is "recovered-and-counted … surfaced via `take_recovered`" — the dormancy path is counted-only. Minor observability asymmetry, pinned as intended by `non_pes_pid_goes_dormant_instead_of_accumulating` (`assert!(asm.take_recovered().is_empty(), "dormancy, not recovery")`, pes.rs:1004).
What correct looks like: either a `Recovered { reason: "pes.dormant-pid" }` per dormancy episode, or doc that names dormancy as the counted-only exception.
Confidence: HIGH — Verification: read; test assertion quoted at 1004; accessor callers grepped.

## 4. Per-function table (part1 items; `complete_one`/`try_complete`/`finish`/`next_pes`/`take_*`/`damage_bytes` + `mod tests` are part2 = W05-A04)
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + doctest | 1-94 | REAL | — | claims mostly verified (unit+corpus+doctest baseline-ok); seam doc at 60-63 describes test-only wiring (F-004); doctest executed: nofailfast:6371 ok |
| `MPEG_CLOCK_HZ` | 106 | REAL | — | 90_000 ✓ §2.4.2 |
| `MAX_PES_BYTES` | 111 | REAL | — | 8 MiB; enforced 567-577 with checked_add; test-pinned |
| `MAX_SANE_STEP_TICKS` | 115 | REAL | — | 10 s; consumed in complete_one (part2) |
| `STAGE` | 117 | REAL | — | Demux |
| `mint` | 119-121 | REAL | — | trivial constructor |
| `bad_start_code` | 125-132 | REAL | — | typed Corrupt/InvalidStructure |
| `short_header` | 137-147 | REAL | S4 | placeholder 0/0 detail (F-006) |
| `bad_pts` | 151-158 | REAL | — | typed |
| `truncated` | 163-170 | REAL | — | carries real expected/found; used by finish() |
| `payload_over_ceiling` | 173-182 | REAL | — | ResourceLimit/Memory, budget+attempted |
| `time_base_unavailable` | 186-193 | REAL | — | defensive arm for constant 1/90000 (unreachable by construction — honest) |
| `PesStreamClass` (enum) | 198-220 | REAL | S3 | 0xFD → Reserved misses extended streams (F-008) |
| `has_optional_header` | 226-228 | REAL | — | Video/Audio/Private1 ✓ Table 2-16 |
| `classify_stream_id` | 234-247 | REAL | S3 | base Table 2-18 exact; 0xFD drift (F-008); test-pinned |
| `decode_ts5` | 253-266 | REAL | — | 33-bit reassembly hand-verified; marker+prefix guards; short slice typed |
| `encode_ts5` (test-only) | 271-280 | REAL | — | exact inverse; roundtrip-pinned |
| `PesHeader` (struct) | 283-304 | REAL | — | fields honest; `data_alignment` doc lies about which bit (F-001) |
| `PesHeader::parse` | 312-369 | REAL | S3/S3 | F-001 (wrong byte), F-005 (zero-payload refusal); PTS/DTS arm correct; no panic paths (len guards before every index) |
| `mpeg_time_base` | 376-378 | REAL | — | constant base, honest error map |
| `pts_stamp` | 381-385 | REAL | — | i64::try_from defensive (33-bit value cannot fail) |
| `PesPacket` (struct) | 388-410 | REAL | S3 | `discontinuity` field always-false in shipped build (F-004) |
| `PesPacket::presentation` | 415-420 | REAL | — | Missing when no PTS ✓; test-pinned |
| `Recovered` (struct) | 425-431 | REAL | — | plain data |
| `TimingEvent` (enum) | 435-453 | REAL/UNREACHABLE | S2 | DiscontinuityIndicator + PcrRegressed unreachable shipped (F-002/F-004); Dts* / PtsRegressedNoDts live |
| `Assembly` (struct) | 456-472 | REAL | S3 | `disc` overwrite hazard (F-003) |
| `Assembly::new` | 475-485 | REAL | — | trivial |
| `PesAssembler` (struct) | 489-497 | REAL | — | per-PID HashMap + deques; PIDs bounded by 13-bit PID space; per-PID buf ceiling-checked |
| `PesAssembler::new` | 502-511 | REAL | — | trivial |
| `push_packet` | 524-584 | REAL | S3×3 | F-003 (disc clobber), F-004 (param dead shipped), F-007 trigger (silent short bounded emit); ceiling/dormancy/stray paths correct |
| `observe_pcr` | 592-613 | UNREACHABLE | S2 | F-002; arithmetic itself correct (checked mul/add, 9-bit ext mask, discontinuity reset) |

## 5. Test honesty (part1's only in-chunk test is the doctest; `mod tests` 822-1235 belongs to part2/W05-A04, referenced as evidence where noted)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doctest `ts::pes` (line 69) | YES (for the assembly spine) | constructs a real bounded audio PES byte-by-byte, asserts class/PTS/payload round-trip through `push_packet`+`finish`+`next_pes`; executed ok (nofailfast:6371) |
| `ts5_roundtrip_and_marker_guard` (884-903, part2) | NO for F-001 — it would PASS with the wrong flag byte | `encode_ts5`/factory share the same misunderstanding; marker-bit and prefix guards are genuinely mutation-sensitive (mentally flipped a marker bit → `bad_pts` fires) |
| `discontinuity_surfaces_as_event_and_packet_flag` (1070-1084, part2) | NO for F-003 — only covers discontinuity-on-start | mutation: set discontinuity on a continuation of packet 1, close with clean start → flag lost, test still green |
| `pcr_regression_is_an_event_and_discontinuity_resets` (1149-1166, part2) | YES for the function's logic | real regression + reset asserted; but the function has no production caller (F-002) — it proves the dead code works, not the shipped behavior |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a regressed PCR is a TimingEvent on the drain queue … never silent, MP §8" | pes.rs:26-31 (module doc, Timing contract) | FALSE (for shipped builds) | observe_pcr zero production callers (rg); shipped bridge hard-codes discontinuity=false (containers/ts.rs:258); demuxer's own last_pcr unchecked (ts/mod.rs:880) |
| "The transport layer (I1a) … hands this layer the discontinuity_indicator bit and PCR observations" | pes.rs:60-63 | FALSE (for shipped builds) | same evidence; the described wiring exists only in tests/ts_pes_corpus.rs:202-211 |
| "data_alignment_indicator — the payload starts at an access-unit boundary (a decoder-side hint, surfaced honestly)" | pes.rs:291-293 | SHALLOW | surfaced, but from the wrong byte (F-001); then discarded by the sole consumer (ts.rs:341) |
| "B-frame PTS reordering with a monotone DTS is not damage (verified against the corpus's B-frame-storm lane)" | pes.rs:31-33 | REAL | corpus tests baseline-ok (nofailfast:2213, `bframe_reorder_pts_series_matches_ffprobe_exactly ... ok`) |
| PES timing model (ticks-in-base, 1/90000) | pes.rs:20-26 | REAL | `pts_stamp_maps_to_the_ninety_kilohertz_base` ok (nofailfast:1875); TimeBase::new(1,90_000) verified |
| "Accumulation is ceiling-checked … never an unbounded read" | pes.rs:48-50 | REAL | 567-577 checked_add + typed refusal; test-pinned (`payload_ceiling_is_the_typed_refusal` ok, nofailfast:1872) |
| bounded complete at `6 + PES_packet_length`; unbounded at next start / honest flush | pes.rs:16-18 | REAL | traced try_complete/finish (part2); tests ok (nofailfast:1864,1881) |

## 7. Dead / duplicated / unreachable code
- `observe_pcr` + `TimingEvent::PcrRegressed` + `PesAssembler.last_pcr` (494-495): test-only (F-002).
- `push_packet`'s `discontinuity` parameter as live input, `TimingEvent::DiscontinuityIndicator`, `Assembly.disc`, `PesPacket.discontinuity`: dead in shipped wiring (F-004); `Packet.discontinuity` (media-types/packet.rs:283) into which it is minted has no engine readers.
- `damage_bytes()` accessor: read only by tests (the bridge never surfaces it).
- Demuxer's `last_pcr()` getter (ts/mod.rs:647-648) + its `last_pcr` field: set, never read — parallel dead PCR state at the transport layer.
- Duplication: two discontinuity mechanisms (assembler flag vs demuxer event) and two PCR states (assembler `last_pcr` vs demuxer `last_pcr`), of which the shipped build uses exactly one of the first and none of the second.

## 8. Dependencies & flags
- No feature gates: `crates/demux/src/lib.rs:54 pub mod ts;` → ts/mod.rs:1206 `pub mod pes;` — unconditionally compiled into the demux crate; no cfg gating anywhere in pes.rs except `#[cfg(test)] encode_ts5` (270) and `mod tests` (822). Shipped `lumen engine-info` features (first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor — lumen-engine-info.txt) do not gate this file; the TS lane is reachable in the shipped binary regardless.
- Imports (96-101): std HashMap/VecDeque + media_types error/time — no external crates; no unsafe.

## 9. Open questions for the next agent
- For W05-A04 (pes.rs#part2): `complete_one` (618-692) never checks bounded `buf.len() < total_len` — see F-007; the `mod tests` factory `video_pes` (837-859) encodes `data_alignment` into byte 7 (F-001 corroboration at 839); the doc comment at 42-43 lists damage surfaces but not dormancy (F-009).
- Does any corpus-v4 lane carry 0xFD stream ids or zero-payload bounded PES? (No corpus in this sandbox — both F-005/F-008 stay MEDIUM confidence.)
- REQUEST-ORCH-RUN (optional, would firm F-001): `xxd` dump of one real PES header from any corpus .ts (or any ffmpeg-muxed .ts: `ffmpeg -y -f lavfi -i testsrc=duration=1:size=64x64:rate=10 -c:v libx264 -muxdelay 0 /tmp/pesprobe.ts && xxd -l 32 -s 189 /tmp/pesprobe.ts`) — byte 6 will be 0x84 (aligned) with byte 7 = 0x80/0xC0, proving the shipped parse reads false on real files.
