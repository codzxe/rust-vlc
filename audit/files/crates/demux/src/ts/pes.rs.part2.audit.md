# AUDIT: crates/demux/src/ts/pes.rs#part2 (lines 618–1235)
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-21 |
| Lines / functions | 618 (chunk 618–1235 of 1235) / 8 production + 2 test helpers + 19 tests |
| File verdict | REAL (flawed in EOS/damage-accounting corners) |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ts` → apps/cli/src/run.rs `lumen_dispatch` → containers/mod.rs:72 `"mpegts" => ts::open(stream, opts)` → containers/ts.rs `PesBridge::new` (assembler field, line 232) → `PesAssembler::push_packet/finish` at containers/ts.rs:255,265) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 1 |
| Commands run | `rg -n "PesAssembler" crates/ apps/` → shipped caller is ONLY apps/cli/src/containers/ts.rs (plus tests ts_pes_corpus.rs, mpeg12_corpus.rs). `rg -n "observe_pcr" apps/ crates/` → NO shipped caller (tests only). `rg -n "push_packet\("` → shipped call site containers/ts.rs:255 passes `discontinuity=false` hard-coded. `rg -n '"mpegts"' apps/cli/src/containers/mod.rs` → line 72 (reachability). `sed -n` on all quoted ranges (verified). `rg -n "ts::pes::tests::" audit/runs/baseline/test-workspace.log.txt` → 19/19 ok (log lines 1904–1922); `sed -n '2254,2262p'` → ts_pes_corpus 4/4 ok. |

Chunk boundary: part1 (W05-A05) = lines 1–617 (module docs, error constructors, `classify_stream_id`, `decode_ts5`/`encode_ts5`, `PesHeader::parse`, `mpeg_time_base`, `pts_stamp`, `PesPacket`, `TimingEvent`, `Assembly`, `PesAssembler::new/push_packet/observe_pcr`). Part2 (this audit) = 618–1235: `complete_one` … `Default`, plus the whole `#[cfg(test)] mod tests`.

## 1. One-paragraph verdict
This half of the PES layer is genuinely implemented and genuinely exercised: all 19 in-file unit tests and the 4-test corpus differential ran and passed in the Wave-0 baseline (`test-workspace.log.txt:1904-1922`, `2254-2262`), and the shipped `lumen open` path reaches every production function here via the `mpegts` container arm (containers/mod.rs:72). The assembly core (`try_complete`, bounded completion, overrun damage) is careful, typed, and tested. The defects cluster in the honesty contract the module brags about: the EOS flush aborts on the first truncated PID (stranding every other PID's tail packets behind a `?`, in HashMap-random order), a mid-stream short bounded PES is emitted as a complete packet with zero damage signal, the EOS overrun arm silently truncates where the mid-stream arm records damage, the entire PCR-regression lane (`observe_pcr`/`PcrRegressed`) is dead code in every shipped build because only tests call it, and the discontinuity lane the module documents is hard-coded off at its one shipped call site.

## 2. What actually works (verified)
— `rg -n "ts::pes::tests::" audit/runs/baseline/test-workspace.log.txt` → 19/19 `... ok` (lines 1904–1922), including `payload_ceiling_is_the_typed_refusal` and `truncated_bounded_pes_at_eos_is_typed`.
— `sed -n '2254,2262p' audit/runs/baseline/test-workspace.log.txt` → `tests/ts_pes_corpus.rs` 4 passed (corpus differential vs ffprobe truth), so the assembly math is differential-tested, not just round-tripped.
— `rg -n "PesAssembler" crates/ apps/` + `sed -n '255p;265p' apps/cli/src/containers/ts.rs` → shipped chain proves `lumen open` on a `.ts` drives `push_packet`/`finish` (via containers/mod.rs:72 `"mpegts"` arm).

## 3. Findings

### F-001 [S2] EOS flush aborts on the first truncated PID: other PIDs' tail packets are never flushed, and which tails survive is HashMap-random
Where: crates/demux/src/ts/pes.rs:760-791 (abort at 785-788)
Code:
```rust
            if let Some((expected, found)) = short {
                return Err(truncated(expected, found));
            }
            self.complete_one(pid);
        }
        Ok(())
```
What is wrong: `finish()` iterates `self.states.keys()` (a `HashMap<u16, Assembly>` — line 761: `let pids: Vec<u16> = self.states.keys().copied().collect();`) and `return Err` on the FIRST PID whose bounded PES is short. Every PID after it in iteration order never gets `complete_one`, so its final unbounded (video) PES is dropped entirely. The packets already pushed to `self.complete` for earlier PIDs are also stranded: the shipped caller (apps/cli/src/containers/ts.rs:264-267) is `self.assembler.finish()?; self.drain()` — the `?` propagates before `drain()` mints the queue into `DemuxEvent`s, and re-calling `finish()` re-raises on the same short buffer.
Why it matters: a truncated `.ts` (partial download / cut recording — the realistic damaged-input case) with one short audio PES loses the final video packet(s) of every other track at EOS, and because HashMap iteration order is randomized per process, the same file produces different tails across runs. The module's own contract ("emit every trailing PES honestly", lines 753-755) is violated for all PIDs after the damaged one.
What correct looks like: complete every PID first (collect the first truncation error), then return it after the loop; or have the caller drain the queue before propagating the error.
Confidence: HIGH — Verification: read (control flow traced through containers/ts.rs:264-267 `self.assembler.finish()?; self.drain()`).

### F-002 [S3] The PCR-regression lane is dead code in every shipped build; only tests call `observe_pcr`
Where: crates/demux/src/ts/pes.rs:1148-1159 (the test anchoring the lane), function at 592-613 (part1 range)
Code:
```rust
    #[test]
    fn pcr_regression_is_an_event_and_discontinuity_resets() {
        let mut asm = PesAssembler::new();
        asm.observe_pcr(100_000, 0, false);
        asm.observe_pcr(99_000, 0, false);
```
What is wrong: `rg -n "observe_pcr" apps/ crates/` returns exactly three call-site groups: `crates/demux/tests/ts_pes_corpus.rs:202,518` and pes.rs's own tests (1151-1164). No production code — including the shipped bridge `apps/cli/src/containers/ts.rs` — ever observes a PCR. The module doc (lines 28-29) claims "a … regressed PCR is a [`TimingEvent`] on the drain queue"; in the shipped binary that event is unreachable.
Why it matters: doc-vs-shipped drift and dead public API: the doc says PCR regression is surfaced "never silent, MP §8", but no shipped entry point can produce the event. Tests green-light a lane the product never runs.
What correct looks like: either the TS demuxer/bridge feeds `observe_pcr` from its adaptation-field PCR parse (ts/mod.rs parses the field but keeps PCR to itself), or the doc marks the lane as test-corpus-only.
Confidence: HIGH — Verification: read + `rg -n "observe_pcr"` over apps/ and crates/ (exhaustive).

### F-003 [S3] The discontinuity lane is hard-coded off at the only shipped call site; the module doc attributes behavior the shipped build cannot exhibit
Where: apps/cli/src/containers/ts.rs:254-256 (call site), crates/demux/src/ts/pes.rs:26-31 (claim)
Code (pes.rs:29-31):
```rust
//! `discontinuity_indicator`, or a regressed PCR is a
//! [`TimingEvent`] on the drain queue (mapped by the demuxer to
//! `DemuxEvent::Discontinuity` — never silent, MP §8).
```
Code (containers/ts.rs:255):
```rust
        self.assembler.push_packet(pid, pusi, false, payload)?;
```
What is wrong: the shipped bridge passes literal `false` for `discontinuity`, so `TimingEvent::DiscontinuityIndicator`, the `Assembly.disc` bookkeeping, and `PesPacket.discontinuity = true` can never occur in a shipped build. The transport layer covers the signal itself (crates/demux/src/ts/mod.rs:882-884 mints `DemuxEvent::Discontinuity` on the adaptation-field bit — verified by rg), so end behavior exists, but pes.rs's documented lane plus `PesPacket.discontinuity` are parallel dead plumbing in the shipped path, and the doc misattributes where the event comes from.
Why it matters: the seam is documented as receiving the bit ("the transport layer … hands this layer the `discontinuity_indicator` bit", pes.rs:60-62) — it does not; a reader auditing playback discontinuity handling following this doc lands on dead code.
What correct looks like: delete the third parameter at the shipped seam or fix the doc to state the demuxer owns the event and the assembler's flag is corpus-only.
Confidence: HIGH — Verification: read + rg (`push_packet(` call sites; ts/mod.rs:882-884).

### F-004 [S3] Mid-stream short bounded PES is emitted as a complete packet with no damage or truncation signal
Where: crates/demux/src/ts/pes.rs:662-676
Code:
```rust
                let payload = buf
                    .get(header.header_len..)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default();
                self.complete.push_back(PesPacket {
```
What is wrong: `complete_one`'s Ok arm never compares `buf.len()` against `header.total_len`. When the next `payload_unit_start` arrives before a bounded PES reaches its declared length (push_packet's `had_pending` path, lines 546-548), the short packet is pushed to `self.complete` as an ordinary `PesPacket` — no `Recovered` record, no `damage_bytes`, no event. Contrast `try_complete` (737-748), which scrupulously records a 4-byte overrun as `"pes.length-overrun"`, and `finish`, which types the identical short-bounded condition as `demux.ts.pes.truncated` (771-774).
Why it matters: a decoder receives a truncated access unit presented as whole; the "recovered-and-counted" damage contract (module doc lines 35-43) has exactly this shape missing. On lossy TS (continuity-counter breaks — realistic broadcast), mid-stream short bounded PES is the expected damage form and it is the one form with zero accounting.
What correct looks like: in `complete_one`'s Ok arm, when `header.total_len` is `Some(total)` and `buf.len() < total`, push a `Recovered` record (e.g. reason `"pes.length-under-run"`) or the typed truncation instead of the silent emit.
Confidence: MEDIUM — Verification: read (control-flow traced: push_packet 546-548 calls complete_one on had_pending with no length check in between).

### F-005 [S3] EOS overrun arm truncates silently where the mid-stream arm records damage — same anomaly, two accountings
Where: crates/demux/src/ts/pes.rs:771-779
Code:
```rust
                    Ok(header) => match header.total_len {
                        Some(total) if state.buf.len() < total => {
                            Some((total as u64, state.buf.len() as u64))
                        }
                        Some(total) if state.buf.len() > total => {
                            state.buf.truncate(total);
                            None
                        }
```
What is wrong: in `finish`, a bounded buffer LONGER than its declared total is truncated to `total` with no `damage_bytes` increment and no `Recovered` record. `try_complete` (737-748) handles the identical overrun by recording `damage_bytes += skipped` and `Recovered { reason: "pes.length-overrun" }`. The EOS copy dropped the accounting.
Why it matters: `damage_bytes()` undercounts real damage depending on whether an overrun is detected mid-stream or at EOS — the observability metric the demuxer surfaces is order-dependent, and the duplication is exactly the drift copy-paste seams produce.
What correct looks like: extract the overrun handling into one helper used by both, or at minimum record the skipped bytes in the EOS arm.
Confidence: HIGH — Verification: read (both arms quoted and compared).

### F-006 [S4] Legal padding-stream stuffing is counted into `damage_bytes` ("bytes dropped by recovery")
Where: crates/demux/src/ts/pes.rs:626-630
Code:
```rust
                if header.class == PesStreamClass::Padding {
                    // Stuffing bytes are not a packet.
                    self.damage_bytes += buf.len() as u64;
                    return;
                }
```
What is wrong: a `0xBE` padding PES is spec-legal stuffing, not damage; folding it into `damage_bytes` (documented at 810-811 as "Total bytes dropped by recovery so far") mislabels well-formed streams as damaged.
Why it matters: a conforming stream full of padding PES (rate-buffered broadcasts) reports massive "damage"; operators chasing that number find nothing wrong.
What correct looks like: count stuffing separately (e.g. `stuffing_bytes`) or not at all.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PesAssembler::complete_one` | 618-692 | REAL | S3 | Emits completed PES + timing events; no bounded-length validation → F-004; padding mislabel F-006; Err arm counts+records honestly (678-690). Reached via push_packet/try_complete/finish. |
| `PesAssembler::try_complete` | 694-751 | REAL | — | Bounded completion; overrun damage recorded (737-748); bad-start-code → dormancy without Recovered (deliberate, pinned by test at 1004). Double `PesHeader::parse` (here + complete_one) is O(1) each — acceptable. |
| `PesAssembler::finish` | 753-791 | REAL (flawed) | S2/S3 | F-001 abort-on-first-short + F-005 silent EOS truncation. Unbounded finals emitted correctly (test 929-934). |
| `PesAssembler::next_pes` | 793-796 | REAL | — | `pop_front`; trivial and correct. |
| `PesAssembler::take_recovered` | 798-802 | REAL | — | `drain(..).collect()`; trivial and correct. |
| `PesAssembler::take_timing_events` | 804-808 | REAL | — | `drain(..).collect()`; trivial and correct. |
| `PesAssembler::damage_bytes` | 810-813 | REAL | S4 | Metric mixes stuffing into "recovery" (F-006). |
| `Default for PesAssembler` | 816-820 | REAL | — | Delegates to `new()`; correct. |
| `tests::err_code` (helper) | 829-834 | REAL | — | Lint-legal unwrap_err replacement; correct. |
| `tests::video_pes` (helper) | 837-859 | REAL | — | Builds unbounded video PES vectors; flags/`encode_ts5` math sound. |
| `stream_id_classification_table` | 861-882 | REAL | — | Pins Table 2-18 mapping; would fail on classification change. |
| `ts5_roundtrip_and_marker_guard` | 884-903 | REAL | — | Roundtrip + marker-guard + wrong-prefix + short-field; symmetric-bug risk mitigated by the independent doctest vector (pes.rs:77-91). |
| `unbounded_video_assembles_across_packets` | 905-937 | REAL | — | 3-way split, asserts pts/dts/pes_len/payload + EOS flush; strong. |
| `bounded_audio_completes_at_declared_length` | 939-954 | REAL | — | Pins `6+length` arithmetic. |
| `tiny_bounded_pes_without_optional_header_completes` | 956-968 | REAL | — | 0xBF below the 9-byte optional-header minimum; covers the `<9` hold path. |
| `length_overrun_is_recovered_damage` | 970-985 | REAL | — | Pins overrun accounting of F-005's mid-stream arm. |
| `stray_continuation_before_any_start_is_counted` | 987-993 | REAL | — | Damage + Recovered pinned. |
| `non_pes_pid_goes_dormant_instead_of_accumulating` | 995-1005 | REAL | — | PAT-shaped start → dormancy; asserts NO Recovered (deliberate). |
| `truncated_bounded_pes_at_eos_is_typed` | 1007-1027 | REAL | — | expected=38/found=12 math verified (6+0x20=38, 9+3=12). |
| `bad_pts_marker_is_typed_damage_midstream` | 1029-1045 | REAL | — | Damaged PTS dropped+counted, good PES survives. |
| `forbidden_pts_dts_flags_refuse_typed` | 1047-1055 | REAL | — | 0b01 flags → typed refusal. |
| `header_swallowing_the_declared_packet_is_refused` | 1057-1067 | REAL | — | header_len ≥ total refusal pinned. |
| `discontinuity_surfaces_as_event_and_packet_flag` | 1069-1084 | REAL | — | Tests the lane F-003 shows is dead in shipped builds. |
| `dts_regression_and_jump_are_events` | 1086-1111 | REAL | — | Both events + ordering pinned. |
| `pts_regression_without_dts_is_an_event_bframe_pts_with_dts_is_not` | 1113-1146 | REAL | — | Three-scenario discrimination (reorder ≠ damage); matches doc contract. |
| `pcr_regression_is_an_event_and_discontinuity_resets` | 1148-1166 | REAL | — | Tests the lane F-002 shows is unreachable in shipped builds. |
| `payload_ceiling_is_the_typed_refusal` | 1168-1190 | REAL | — | Feeds 1 MiB chunks until `payload-over-ceiling`; ResourceLimit kind pinned. |
| `pts_stamp_maps_to_the_ninety_kilohertz_base` | 1192-1214 | REAL | — | Independent nanos arithmetic (126_000·1e9/90_000). |
| `error_kinds_are_stable_and_staged` | 1216-1234 | REAL | — | Codes + stage + `[demux] ` display prefix pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 19 in-file tests | YES | Each asserts concrete values (codes, event enums, payload bytes, exact expected/found), not tautologies. Mutation reasoning: breaking `decode_ts5` bit layout fails `ts5_roundtrip…` AND the doctest vector (pes.rs:77-91); breaking overrun accounting fails `length_overrun…`; breaking the ceiling fails `payload_ceiling…`. The two lane tests (`discontinuity…` 1069-1084, `pcr_regression…` 1148-1166) would fail on regression of lanes no shipped build reaches (F-002/F-003) — honest about the code, misleading about the product. |
| Baseline execution | — | 19/19 `ok` (test-workspace.log.txt:1904-1922); corpus differential 4/4 `ok` (:2254-2262). No `#[ignore]`, no cfg-gating beyond `#[cfg(test)]`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a … regressed PCR is a TimingEvent on the drain queue … never silent, MP §8" | pes.rs:26-31 (module doc) | UNREACHABLE (shipped) | `rg observe_pcr` → only tests call it; shipped bridge never does. |
| "The transport layer (I1a) … hands this layer the discontinuity_indicator bit and PCR observations" | pes.rs:58-62 | FALSE (shipped) | containers/ts.rs:255 passes hard-coded `false`; `observe_pcr` uncalled; ts/mod.rs:882-884 keeps both to itself. |
| "an unbounded one is emitted as the final packet (ffmpeg's own last-packet behavior, verified by the corpus differential)" | pes.rs:47-48 | REAL | `corpus_ts_lanes_match_the_ffprobe_truth … ok` (test-workspace.log.txt:2259). |
| "a bounded PES still short of its declared length is the typed sticky truncation" | pes.rs:44-46 | REAL (first damaged PID only) | Test 1007-1027 pins it; F-001 shows it also silently kills every other PID's flush. |
| "Mid-stream damage is recovered-and-counted" | pes.rs:35-43 | SHALLOW (gap) | F-004: mid-stream short-bounded PES is neither recovered nor counted. |

## 7. Dead / duplicated / unreachable code
- `observe_pcr` + `TimingEvent::PcrRegressed` + `last_pcr`: unreachable from shipped entry (F-002).
- `Assembly.disc`/`TimingEvent::DiscontinuityIndicator`/`PesPacket.discontinuity`: dead in shipped path (F-003); duplicated coverage at ts/mod.rs:882-884.
- Overrun handling duplicated between `try_complete` (737-748) and `finish` (775-778) with drifted accounting (F-005).

## 8. Dependencies & flags
No feature gates on this module (plain `pub mod pes;` under `pub mod ts;`, demux/src/lib.rs:1200-1206). No unsafe. Uses `media_types` error/time plumbing and `testkit::Must` (test-only). No cfg-gating that could hide the code from shipped builds — reachability is proven, not feature-conditional.

## 9. Open questions for the next agent
- Does the host's error handling after a demux EOS error consume already-queued events (mitigating or worsening F-001)? Trace `PesBridge::finish` consumers in demux_host.rs / SelectDemuxer.
- ts/mod.rs (W05-A09/A10): does the transport walk surface PCR at all, or is PCR parsing dead there too? If dead end-to-end, the "regressed PCR" promise fails at two layers.
- W05-A05 (part1): `push_packet`'s ceiling check clears the buffer BEFORE returning Err — combined with F-001's abort, verify whether a ceiling breach mid-file also strands other PIDs.
