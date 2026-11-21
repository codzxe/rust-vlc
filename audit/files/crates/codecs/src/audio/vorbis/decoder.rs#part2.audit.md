# AUDIT: crates/codecs/src/audio/vorbis/decoder.rs#part2 (lines 570-1140)
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 571 (chunk part2 of 1140; part1=1-569 is W07-A07's) / 12 units in range |
| File verdict (part2) | REAL-UNPROVEN (genuine, heavily engineered, reachable by call chain; no baseline run plays a Vorbis lane in the shipped binary) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open → playback-engine `pipeline.rs:2362 limb.decoder.decode` → `DecoderProvider::decoder_for` apps/cli/src/pipeline_host.rs:271-275 `"vorbis" => self.seeded(track, config, VorbisDecoder::new)`; also apps/web/src/host.rs:641). Execution UNPROVEN: baseline rc-smoke is AVI-only (INDEX.md #20); the codec's own suites ran green in baseline #3 (1681 passed). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 4 S4: 1 |
| Commands run | `rg -n "VorbisDecoder::new\|next_first_sample\|timeline_shift\|emitted_total\|release_head_hold\|drain_head_hold"` (callers; found zero non-test callers for the observables) · `rg -n "HeadHold\|head_hold"` (test coverage: none touch it directly) · read codec-api/src/decode.rs:86-144 + contract.rs:281-314,400-429 (outcome contract) · `rg "decode_outcome_agrees_with_sink"` in codecs tests → only contract.rs runs it, against MjpegDecoder only · read media-types/src/packet.rs SharedBytes (Arc-backed, `get_mut` = `Arc::get_mut`) · `sed -n` quote verification |

## 1. One-paragraph verdict
Part 2 is the decoder's real engine room — configure/decode/flush, the granule tail clamp, the amortized head-hold drain (issue #38), and the pooled slot rotation — and the sample-clock algebra (pts = raw − shift, valid_len = granule − valid_start clamped to emission) is internally consistent and pinned end-to-end by the bitexact suite (head-trim, un-trimmed flush, corruption recovery). But it violates the codec-api `DecodeOutcome` contract twice on realistic paths: `decode` returns `Emitted { frames: 0 }` for every packet of a stream's first data page (the head hold withholds output), and `flush` pushes frames to the sink and still returns `Drained` — both are violations the project's own contract checker (`contract.rs:294-296`, `:309-311`) would flag as lies, and neither is caught because the contract suite runs only against MjpegDecoder (crates/codecs/tests/contract.rs:10). Add a sink-error path that silently discards the withheld head, a pooled-slot rotation that re-allocates per packet under the shipped 16-packet decode-ahead bursts (contradicting the module's zero-alloc claim), and three public observables with zero callers, and the picture is: correct sample math, sloppy contract edges, unproven in-product playback.

## 2. What actually works (verified)
— Head-trim end-to-end: `crates/codecs/tests/vorbis_bitexact.rs:339 stream_head_encoder_delay_is_trimmed_decoder_side` asserts `total == granule` (delay trimmed decoder-side) — ran green in baseline #3 (INDEX.md: 1681 passed / 1 unrelated failure). This exercises my range's hold-append (846-849), trim resolution (867-885), amortized drain (778-785, 570-646), and tail clamp (822-831).
— Granule tail clamp algebra verified by hand: `valid_len = (granule − (raw_first − shift)) clamped to [.., emitted]` (822-831); `measure_shift` (vorbis/seek.rs) requires the same `ts.base == base`, so stamp-base handling is consistent between clamp and shift measurement.
— `drain_head_hold` trim/cursor math: `start = max(released, trim)` skips exactly the delay bytes; first-chunk pts `trim − fresh` = 0 for a positive (head) shift and `|fresh|` for a negative (mid-stream landing) shift — matches the module doc's two landing shapes.

## 3. Findings
### F-001 [S2] `decode` returns `Emitted { frames: 0 }` — violates the codec-api outcome contract on every stream's first page
Where: crates/codecs/src/audio/vorbis/decoder.rs:905-907 (also 925-927)
Code:
```
            self.packet_index += 1;
            let frames = u32::try_from(drained_frames).unwrap_or(u32::MAX);
            return Ok(DecodeOutcome::Emitted { frames });
```
What is wrong: in the head-hold branch `drained_frames` is 0 until the first stamp resolves the trim (drain runs only when `hold.trim_from.is_some()`, 778-785), and the current block is withheld into `hold.data` (846-849) — so zero `accept` calls happen, yet the outcome is `Emitted { frames: 0 }`. The contract (codec-api/src/decode.rs:125-126 "`frames >= 1`; nothing to emit is `NeedMoreInput`"; machine-checked at contract.rs:294-296 "Emitted { frames: 0 } lies") requires `NeedMoreInput` here. The pure-padding tail branch (925-927) repeats the same lie after the hold is gone. The reference fixture's first page carries ~44 packets (module doc, line 136-137) — every one of them returns the lying outcome.
Why it matters: any consumer relying on the documented invariant (progress accounting, decode-park heuristics, the `pipeline_accounting` CLI tests) mis-reads these calls; the violation is invisible only because the contract suite is run solely against MjpegDecoder (crates/codecs/tests/contract.rs:10-11) — a coverage hole, not compliance.
What correct looks like: return `DecodeOutcome::NeedMoreInput` when nothing was accepted this call (the packet was consumed and buffered); keep `Emitted { frames: n }` for n ≥ 1 only.
Confidence: HIGH — Verification: read + traced both paths; baseline #3 ran the vorbis suites (no outcome-agreement assertion exists for them).

### F-002 [S2] `flush` pushes frames to the sink and returns `Drained` — the second outcome-contract violation
Where: crates/codecs/src/audio/vorbis/decoder.rs:984-992
Code:
```
            if started {
                self.drain_head_hold(out, generation, usize::MAX)?;
            } else {
                self.release_head_hold(out, generation, None)?;
            }
        }
        // The overlap tail was already emitted with the last block
        // (each packet emits its own half); nothing is buffered.
        Ok(DecodeOutcome::Drained)
```
What is wrong: both branches push ≥1 frame through `out.accept` (the un-started hold holds a whole stream head; the started one holds the drain residue), then the call answers `Drained`. The contract checker (contract.rs:309-311) makes "Drained after N accept(s)" a violation, and `flush_drains` (contract.rs:415-416) asserts exactly this. Every other audio decoder (aac:602, adpcm:682, pcm:785, mp3:610, mp2:407, alac:707, opus:496, flac:611 — same tree) is stateless in flush, so vorbis is the lone violator and the lone untested one.
Why it matters: same class as F-001 — the documented invariant is broken on realistic paths (single-data-page streams; mid-drain EOS), unpinned by tests. The engine today ignores flush outcomes beyond `Drained => break` (playback-engine pipeline.rs:2457), so nothing regresses *yet*; the first consumer that trusts the contract will.
What correct looks like: `Emitted { frames: n }` when frames were pushed, `Drained` only once nothing remains (and sticky).
Confidence: HIGH — Verification: read; cross-checked all 8 sibling flushes.

### F-003 [S3] Sink-refusal mid-drain silently discards the withheld head — contract says flush must still answer afterwards
Where: crates/codecs/src/audio/vorbis/decoder.rs:599 and 634 (with 536-543)
Code:
```
            out.accept(frame)?;
            frames += 1;
            pending_at += 1;
        }
        hold.pending.drain(..pending_at);
```
What is wrong: `drain_head_hold` does `self.head_hold.take()` (536) and restores it only on the two early typed-error paths (540, 549); if `out.accept` fails at 599 or 634 the `?` returns with the `hold` value dropped — all remaining head audio and every pending block are destroyed and `self.head_hold` stays `None`. codec-api/src/decode.rs:76-78 explicitly requires the decoder to remain flushable after `Err` ("After returning `Err`, the instance may still be asked to `flush` (which must answer, not panic)"); here flush answers `Drained` over a destroyed head.
Why it matters: an error-path data loss of up to `HEAD_HOLD_MAX_SAMPLES` (120,000) samples that the contract's post-error flush was designed to recover; narrow (sink refusals are fatal-class today) but it is exactly the resilience seam the contract documents.
What correct looks like: restore `self.head_hold = Some(hold)` before every `?`-propagation (or restructure so the hold lives in `self` for the whole call).
Confidence: HIGH — Verification: read; all exit paths enumerated (only 540/549/643 restore).
### F-004 [S3] Pooled-slot rotation re-allocates per packet under the shipped decode-ahead regime — contradicts the module's zero-alloc claim
Where: crates/codecs/src/audio/vorbis/decoder.rs:1031-1052 (with 1009-1030)
Code:
```
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; slot_len]);
```
What is wrong: a slot is reusable only when `Arc::get_mut` succeeds (1021, media-types packet.rs:213-214) — i.e. no downstream frame still references it. `SAMPLE_SLOTS = 4` (114), but the shipped first-vertical decode-ahead regime stages "16-packet bursts into a 3 s ring" (apps/cli/src/pipeline_host.rs:603-605) and the amortized drain additionally pins slots via `PendingBlock { bytes: bytes.clone() }` (860) — with ≥4 frames outstanding, every decode takes the cold path: a fresh `vec![0; slot_len]` per packet (64 KB stereo, 256 KB 8-channel at max blocksize) plus rotation eviction. The module doc's claim "steady-state decodes refill a slot in place, zero per-block allocation after warm-up" (decoder.rs:92-97) holds only in the not-yet-shipped paced regime.
Why it matters: per-packet 64-256 KB allocation churn in the decode hot path of the one lane (Ogg/Vorbis) this decoder exists for; the D-1 discipline claim is false in the shipped build's actual regime.
What correct looks like: size `SAMPLE_SLOTS` ≥ the decode-slice packet budget (16) + pending depth, or have the park/backpressure seam bound outstanding frames to < `SAMPLE_SLOTS`.
Confidence: MEDIUM (allocation behavior reasoned from `Arc::get_mut` semantics and the documented burst regime; not measured — no allocation-counting test covers the burst shape; REQUEST-ORCH-RUN not needed, the logic is static).
### F-005 [S3] Empty packet drops its container stamp before `measure_shift` — legal Ogg zero-length packets break the head trim
Where: crates/codecs/src/audio/vorbis/decoder.rs:754-756
Code:
```
        if packet.data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
```
What is wrong: Ogg lacing permits zero-length packets (RFC 3533 §2); the demuxer stamps the page's last-completing packet — if that packet is empty, its granule (the only stamp on the page) is discarded here without `measure_shift` (which runs only at 867/922/949). The shift never resolves from that page: the head hold keeps withholding until the next stamped packet or the 120,000-sample give-up bound (886-904), and mid-stream the tail clamp (822-831) skips that packet's granule too.
Why it matters: a legal-if-rare container shape degrades head-trim and pts accuracy on an otherwise-correct path; the fix is one line (run `measure_shift`/raw-counter accounting for empty packets before returning).
What correct looks like: advance `raw_next` (0 emission) and run `measure_shift` on the empty packet's stamp, then return `NeedMoreInput`.
Confidence: MEDIUM — Verification: read; the demuxer's stamping of empty packets is UNVERIFIED (demux/ogg is another agent's file; if it never emits empty packets the bug is unreachable — flag for the demux agent).
### F-006 [S3] Three public observables have zero callers; the one doc claim about them is false; one is also wrong for negative shift
Where: crates/codecs/src/audio/vorbis/decoder.rs:672-689
Code:
```
    pub fn next_first_sample(&self) -> u64 {
        self.raw_next
            .saturating_sub_signed(self.shift.unwrap_or(0).max(0))
    }
```
What is wrong: `next_first_sample`, `timeline_shift`, `emitted_total` have no callers anywhere in the repo (rg across all *.rs: definitions + one doc mention only). vorbis/seek.rs:36-38 claims `next_first_sample` "is the trim coordinate the playback layer uses to land on the container timeline" — the playback layer uses no such thing. Additionally `next_first_sample` clamps the shift at 0 (`.max(0)`) while the actual pts math (857-858, 910-914) subtracts the raw (possibly negative, mid-stream-landing) shift — for negative shift the observable under-reports the next sample index by `|shift|`, so it is not even a correct mirror of the pts math it documents.
Why it matters: dead public API whose documented consumer does not exist, one method semantically divergent from the code it claims to expose — the exact "naming that lies" pattern the checklist targets.
What correct looks like: delete them, or wire the seek landing surface the docs promise and drop the `.max(0)` (report `raw − shift` as i64).
Confidence: HIGH — Verification: rg caller trace (zero non-definition hits).
### F-007 [S4] `release_head_hold`'s trimmed-release path is dead code
Where: crates/codecs/src/audio/vorbis/decoder.rs:472-494 (callers at 900, 987)
Code:
```
        // The release window in raw samples: [trim_from, held) for a
        // trimmed release, [0, held) un-trimmed. The trim is bounded
        // by the hold (a lying granule cannot cut past the emission).
        let start_sample = trim_from.map_or(0, |trim| trim.min(hold.samples));
```
What is wrong: both live call sites pass `trim_from: None` (900 give-up, 987 flush); the `Some(_)` trim window (475) and the `Present` pts branch (486-494) are unreachable since issue #38 moved trimmed releases into `drain_head_hold`. Leftover pre-#38 machinery.
Why it matters: 23 lines of unreachable branch in the trickiest state machine in the file; a future edit can "fix" the dead branch and diverge from the live one.
What correct looks like: delete the `trim_from` parameter and the `Some` branches, or make the give-up path assert `None`.
Confidence: HIGH — Verification: rg both call sites (only 900, 987).

## 4. Per-function table (part2 range; part1 units listed only where part2 depends on them)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `drain_head_hold` (tail of fn begun part1:530) | 570-646 | REAL | F-003 | head-chunk loop 570-602 + pending FIFO 608-638; budget respected; `hold.released` cursor correct; shared-view memoization correct; error path drops hold |
| `VorbisDecoder::accepts` | 650-652 | REAL | — | exact-match "vorbis" |
| `output_format` | 657-667 | REAL | — | called by tests only (bitexact:263); mirrors decode's format literal (3 copies of the AudioFormat literal in this file: 495-501, 616-622, 929-935 — duplication, S4-adjacent) |
| `next_first_sample` | 672-675 | REAL / dead | F-006 | zero callers; `.max(0)` diverges from pts math |
| `timeline_shift` | 681-683 | REAL / dead | F-006 | zero callers |
| `emitted_total` | 687-689 | REAL / dead | F-006 | zero callers |
| `Decoder::configure` | 693-736 | REAL | — | typed descriptor/extra_data gates; all state reset BEFORE the new install (718-734 run only on success — failed reconfigure preserves old state, correct ordering) |
| `Decoder::decode` | 738-955 | REAL | F-001, F-005, F-004(feed) | stale drop first (747); discontinuity resets engine overlap (764-768); tail clamp 822-831 (backwards granule → full emission via `unwrap_or(usize::MAX)` then `.min(emitted)` — hostile-stamp-safe); hold append 846-849; trim resolution 867-885; give-up 886-904 (bound 120k, `shift_now.is_none()` guard correct); steady-state frame 929-947 |
| `Decoder::flush` | 957-993 | REAL | F-002 | `started` predicate (980-983) correctly distinguishes conflated-delay single-page heads from mid-drain residues; uncapped drain 985 |
| `struct PooledBlock` | 998-1001 | REAL | — | |
| `pooled_sample_slot` | 1009-1057 | REAL | F-004 | scan-reuse-else-allocate; `len() != slot_len` guard (1018) handles stale slots after reconfigure; fresh-slot `get_mut` failure typed (1035-1044) — unreachable by construction, honestly surfaced |
| test `descriptor_gate_accepts_only_vorbis` | 1063-1070 | REAL | — | would fail if gate widened |
| test `decode_before_configure_is_typed_bad_state` | 1072-1101 | REAL | — | asserts code + kind; pins the doctest claim |
| test `configure_rejects_foreign_descriptors_and_short_chains` | 1103-1138 | REAL | — | two typed refusals asserted by code |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| in-file 3 (table above) | YES | narrow but assert typed codes, not just Ok/Err |
| bitexact `stream_head_encoder_delay_is_trimmed_decoder_side` (tests/vorbis_bitexact.rs:339) | YES | `total == granule` + no-sample-past-EOS pin the trim, the amortized drain (44 frames must all leave at ≤4/call), and the tail clamp together |
| bitexact un-trimmed-flush case (:384-405) | YES | pins the single-data-page flush shape — but NOT the outcome value (would not catch F-002) |
| bitexact `mid_stream_corruption_is_typed_and_recovery_scoped` (:219) | YES | overlap-scoped recovery via engine.reset (280 in part1) |
| NOT COVERED: per-call budget 4 (`RELEASE_FRAMES_PER_CALL`), give-up bound 120k, pending FIFO order under mixed valid/padding blocks, contract outcome agreement | — | F-001/F-002 exist precisely because no test runs `decode_outcome_agrees_with_sink` against vorbis |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "steady-state decodes refill a slot in place, zero per-block allocation after warm-up" | decoder.rs:92-97 (module doc) | SHALLOW | F-004: false under the shipped 16-packet burst regime; true only in the unserviced paced regime |
| "an un-trimmed release … emits `Missing` pts, the pinned pre-fix shape" | decoder.rs:176-180 | REAL | release_head_hold `_ => Stamp::Missing` (493); bitexact:384-405 |
| "the release is amortized one RELEASE_FRAMES_PER_CALL frame per call" | decoder.rs:196-199 | REAL | 570, 608 budget checks; indirectly pinned by head-trim test |
| "`next_first_sample` … is the trim coordinate the playback layer uses" | vorbis/seek.rs:36-38 | FALSE | F-006: zero callers in the repo |
| "4 divides the default 16 (and every 4-aligned budget)" (pre-roll hold cap safety) | decoder.rs:145-152 | REAL-UNPROVEN | coupling to the engine's `hold_cap` constant asserted by comment only; no test pins a non-4-aligned cap failing safe |

## 7. Dead / duplicated / unreachable code
F-006 (three dead pub observables), F-007 (dead trimmed path in release_head_hold). The `AudioFormat` literal is hand-copied three times in decode paths (495-501, 616-622, 929-935) plus `output_format` — four copies. `MAX_MAPPED_CHANNELS` gate is honest (typed refusal before engine build).

## 8. Dependencies & flags
symphonia-codec-vorbis 0.6.1 exact-pinned (module doc; admission record docs/dependencies/records/symphonia-vorbis-0.6.1.md). No feature gate — the vorbis module compiles in the shipped build and is dispatched at pipeline_host.rs:271-275. No unsafe. No panics reachable from parsed input that I could find (all slice bounds derive from `hold.samples`/`emitted`, both bounded; `usize::try_from` defensive).

## 9. Open questions for the next agent
- demux/ogg: does it ever emit zero-length packets, and does it stamp them (F-005 reachability)? Does it stamp with `TimeBase::new(1, rate)` (any other base silently disables both the tail clamp and shift measurement — 823, seek.rs measure_shift)?
- Who runs the codec-api contract suite per codec? contract.rs:10 runs only MjpegDecoder — the per-codec contract coverage hole (F-001/F-002) belongs to whichever agent owns tests/contract.rs.
- engine seek pre-roll `hold_cap`: is the product default still 16 and is it guaranteed 4-aligned (decoder.rs:145-152 couples a codec constant to it)?
