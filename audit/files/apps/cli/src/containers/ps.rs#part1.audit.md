# AUDIT: apps/cli/src/containers/ps.rs (part1: lines 1-550 of 1056 — all production code; part2 = the test module 551-1056, also mine)
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 550 / 21 items (module docs; consts ×4; fns ×10; struct ×2; `Demuxer` impl ×5) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.mpg` → parse.rs `CliCommand::Open` → run.rs → `demux_host::open_source` → probe → `containers/mod.rs:71 "mpegps" => ps::open` → `open()` → `PsDemuxer::new/start` → `SelectDemuxer(PsHostDemuxer)`. Executed with the REAL binary: `mpegps_container_opens_and_accepts_the_session ... ok` (orchrun-test-workspace-nofailfast.log.txt:3044); unit lane `containers::ps::tests::*` 4/4 ok (:2830-2833).) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | full Read of all 1056 lines; `rg "containers::ps\|ps::open\|mpegps" apps/cli/src` → mod.rs:71 the sole dispatch; `rg "MAX_PACKET_BYTES" crates/demux/src/ps/mod.rs` → 64 KiB (HOLD_CAP = 131 072); `rg "frame_size\|MAX_FRAME_BYTES" crates/demux/src/es/mp3.rs` → parse_head guarantees 4 ≤ frame_size ≤ 4096 (no zero-progress loop, hold bound sound); `rg "EndOfStream" apps/cli/src/demux_host.rs pipeline_host.rs bench.rs` → every CLI walk `break`s at EOS; `sed -n '2165,2195p' crates/playback-engine/src/pipeline.rs` → engine `return`s at EOS; `rg "REASON_TAIL\|REASON_RESYNC" apps/cli/src/containers/ps.rs` → defined+used, never asserted; `rg "reanchor" apps/cli/tests/*.rs` → no PS reanchor test; cross-read audit/files/crates/demux/src/ps/mod.rs.part{1,2}.audit.md (W05-A07/A06: demux half REAL, seek=pure query conformance); sed -n verification of every quote below. |

## 1. One-paragraph verdict
This is the CLI-side MPEG-PS wiring and it is genuinely engineered, not staged: the open lane (construct → pre-pass `start()` → fact harvest → `--start-time` clamp + pure seek query → assembler wrap → `SelectDemuxer`) is real, executed green by the shipped binary (nofailfast:3044), and the MPEG-audio streaming frame assembler is a correct implementation of its documented crossing/stamp-anchor design — I hand-verified the per-AU arithmetic against the test pins (anchor 9 000 + round(1 152 × 90 000 / 44 100) = 11 351; 1 152 × 1e9 / 44 100 floored = 26 122 448 ns) and the safety bounds (parse_head guarantees frame_size ≥ 4 so the mint loop always advances; hold cap 2 × 64 KiB > legal max 4 095 + 65 536). The one real defect is an event-ordering bug at end-of-stream: `next_event` queues the assemblers' tail notes and then immediately returns `EndOfStream`, but every consumer in the repository — the playback engine (pipeline.rs:2175-2180 `return`), the CLI's probe/bench walks (`break`), and this file's own tests — stops pulling at `EndOfStream`, so the promised "end-of-stream leftovers surface as a tail note — counted, never silently dropped" is structurally undeliverable and `report.damaged_demux` undercounts on truncated files. Two hygiene notes: spanning-frame AUs inherit the completing PES's keyframe/generation flags while their stamps anchor to the starting PES, and the harvest's parity pins are debug_assert-only.

## 2. What actually works (verified)
- The full open lane runs in the shipped binary: `mpegps_container_opens_and_accepts_the_session ... ok` (orchrun-test-workspace-nofailfast.log.txt:3044) and `mpegps_stub_is_refused_typed_by_the_wired_demuxer ... ok` (:3045) — probe refusal and open both exercised through the real `lumen` process.
- Per-AU stamp arithmetic (ticks_for 438-442): `(samples × 90 000 + rate/2) / rate` in u128 — hand-checked: 1 152 samples @ 44.1 kHz → 2 351 ticks; 9 000 + 2 351 = 11 351 == the test pin (ps.rs:730). Duration `1152 × 1e9 / 44100` floored = 26 122 448 ns == pin (:714). — verified by read + arithmetic.
- Mint-loop liveness: `parse_head` (crates/demux/src/es/mp3.rs:228-229) rejects `frame_size < 4`, so `end > at` always holds and the loop cannot spin without progress; `end` uses `saturating_add` (ps.rs:355). — verified by read of the callee's guard.
- Hold bound: HOLD_CAP_BYTES = 2 × 65 536 = 131 072 (ps.rs:115, MAX_PACKET_BYTES = 64 KiB at ps/mod.rs:102) > worst legal hold (4 095-byte partial frame + one ≤ 65 536-byte PES payload = 69 631). The "two ceilings of slack" doc claim holds. — verified by read + arithmetic.
- `resync_from` (456-472): ascending scan, `Found` only on a `parse_head`-valid header, `MaybeTail` only for a pair with < 4 bytes to judge (no judgeable header can exist after it), `Drain` otherwise; starts at i=1 (position 0 already failed). Correct by read.
- Crossing rule (402-409): a frame completing at/after `old_len` re-anchors to the current PES after being minted under the previous anchor — matches the documented "frame belongs to the PES whose bytes carry its FIRST byte". Traced through the synthetic test's spanning-frame scenario by hand. — verified by read.
- Queue ordering (next_event 500-530): minted packets are drained before the next inner pull, so inner damage notes (e.g. padding-stream) interleave correctly — confirmed by the mpg test's exact `notes` pin (ps.rs:820-824). — verified by read + passing test.
- `Demuxer::seek` forwarding is the pure query (demux crate conformance: W05-A06 audit, plugin-api trait contract "the pure query never moves"); `seekability` forwards `SlowLinear` (honest — no index in PS). — verified by cross-audit citation.

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/containers/ps.rs:519-526 (with 414-424)
Code:
```
                DemuxEvent::EndOfStream => {
                    // 2. The flush: leftovers become tail notes
                    //    before the honest end.
                    for (_, assembler) in &mut self.assemblers {
                        assembler.flush(&mut self.queue);
                    }
                    return Ok(DemuxEvent::EndOfStream);
                }
```
What is wrong: The flush pushes `Recovered { reason: REASON_TAIL }` notes into `self.queue` and then **returns `EndOfStream` on the same call** — the queued notes are only delivered by *subsequent* `next_event` calls, but every consumer in the repository terminates its walk at `EndOfStream` and never calls again: the playback engine (`crates/playback-engine/src/pipeline.rs:2175-2180` — `Ok(DemuxEvent::EndOfStream) => { … self.eof = true; … return; }`), the CLI probe/walk helpers (`demux_host.rs:717`, `demux_host.rs:769` — `Ok(DemuxEvent::EndOfStream) => break`), the bench lane (`bench.rs:380`), and this file's own four tests (ps.rs:722, 814, 954, 1042). The inline comment claims "leftovers become tail notes **before** the honest end" — in delivery order they come after it, i.e. never.
Why it matters: The module doc's damage contract (ps.rs:41-47: "end-of-stream leftovers surface as a tail note — counted, never silently dropped") is structurally undeliverable: the note is minted into a queue no one will ever drain. Realistic trigger: any `.mpg`/`.vob` whose final MP2 frame is truncated (cut download, growing file) — the leftover bytes vanish from accounting; `report.damaged_demux` (pipeline.rs:2161-2168 adds `Recovered.skipped` mid-stream only) undercounts, and the session report the user sees hides real damage the module promises to surface. This is a swallowed diagnostic, the exact "never silent" invariant the file stakes its honesty on.
What correct looks like: On `EndOfStream`, flush into the queue and keep looping: return queued notes first and return `EndOfStream` only once the queue is empty (e.g. an `eos_pending: bool` field so the terminal event is emitted after the notes).
Confidence: HIGH — Verification: read (all four consumer sites quoted/located by grep; engine return path read directly).

### F-002 [S4]
Where: apps/cli/src/containers/ps.rs:388-409
Code:
```
            queue.push_back(DemuxEvent::Packet(Packet {
                track,
                pts: au_pts,
                dts: au_dts,
                duration: Some(Duration::Known(Nanoseconds(duration_ns))),
                data: SharedBytes::from_vec(self.buffer[at..end].to_vec()),
                keyframe,
                discontinuity,
                generation,
            }));
```
What is wrong: A frame spanning a PES boundary is minted while processing the *completing* PES, so it inherits that PES's `keyframe` and `generation` (destructured at 296-306) while its stamps are computed from the *starting* PES's anchor (the crossing rule). If a seek/reanchor bumped `generation` between the two PES, the minted AU carries the new generation despite starting in the old era; `keyframe` likewise reflects the wrong parent PES.
Why it matters: Mixed parentage on boundary-spanning frames; today `keyframe`'s only production consumer is the (unreachable) transcode muxer (per W05-A06's audit) and generation invalidation is coarse, so the blast radius is latent — but the asymmetry with the carefully-specified stamp attribution is undocumented.
What correct looks like: Capture `keyframe`/`generation` alongside the anchor at crossing time (per-byte provenance), or document the completing-PES rule next to the crossing-rule comment at 307-310.
Confidence: MEDIUM — Verification: read (no execution; no test covers a generation change across a spanning frame).

### F-003 [S4]
Where: apps/cli/src/containers/ps.rs:218-227
Code:
```
    let descriptors = demuxer.stream_descriptors();
    debug_assert_eq!(
        descriptors.len(),
        tracks.len(),
        "stream_descriptors parallels the track list"
    );
    for (track, (codec, kind)) in tracks.iter().zip(descriptors) {
        debug_assert_eq!(track.codec.as_str(), codec, "descriptor vocabulary parity");
        debug_assert_eq!(track.kind, kind, "track kind parity");
    }
```
What is wrong: The parity pins the module doc advertises ("pinned equal by a debug assert", ps.rs:63-65) are `debug_assert!` — compiled out of the shipped release binary; in release a divergence would pass silently (the `zip` at 224 simply truncates). Facts correctness is unaffected (the harvest loop at 229-258 reads `tracks`, not `descriptors`), so this is a dev-only tripwire, not a production check.
Why it matters: The doc presents the pin as an invariant guarantee; it exists only in debug builds. Acceptable engineering, misleading prose.
What correct looks like: Either state "debug-only tripwire" in the doc, or hoist the check into a typed error at open time (cost: one comparison per track).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-93 | REAL | S2 | The damage-honesty contract (41-47) is FALSE as delivered (F-001); the measured fixture claims (24-31) are pinned by tests; "no bridge" design accurate |
| `HOLD_CAP_BYTES` | 109-115 | REAL | — | 2 × 64 KiB = 131 072 ≥ legal max hold 69 631 — sound |
| `REASON_RESYNC` | 117-119 | REAL | — | used at 336; never asserted by any test (part2 F-001) |
| `REASON_DRAIN` | 120-122 | REAL | — | used at 348; asserted once (synthetic test) |
| `REASON_TAIL` | 123-125 | REAL | S2 | used at 420; undeliverable post-EOS (F-001); never asserted |
| `open` | 139-198 | REAL | — | clamp → pure seek query → watermark; `unwrap_or(i64::MAX/4)` fallback sane; µs→ns via `saturating_mul`; `skip_until_ns` dies at reanchor (SelectDemuxer, demux_host.rs:616) |
| `is_mpeg_audio` | 203-205 | REAL | — | mp1/mp2/mp3 split set — matches wired decoders |
| `harvest_facts` | 214-264 | REAL | S4 | debug-only parity pins (F-003); facts from `tracks` — correct source; span end via `.timestamp().ok()` |
| `MpegAudioAssembler` (struct) | 276-290 | REAL | — | Default derive; five fields all used |
| `MpegAudioAssembler::push` | 296-412 | REAL | S4 | core walk — correct (crossing, resync, drain, cap); F-002 flag parentage; `frame_size ≥ 4` callee guarantee keeps it live; buffer ops bounded |
| `MpegAudioAssembler::flush` | 416-424 | REAL | S2 | correct in isolation; stranded by F-001's return order |
| `timestamp_of` | 428-433 | REAL | — | trivial |
| `ticks_for` | 438-442 | REAL | — | u128 half-up rounding, saturating — verified by arithmetic |
| `enum Resync` | 447-454 | REAL | — | three-state result, sound |
| `resync_from` | 456-472 | REAL | — | ascending scan, correct MaybeTail window (i+4 > len ⇒ nothing later judgeable) |
| `PsHostDemuxer` (struct) | 487-491 | REAL | — | inner + assemblers + queue |
| `Demuxer::start` | 494-498 | REAL | — | forwards (idempotent cached list) |
| `Demuxer::next_event` | 500-530 | REAL | S2 | F-001 EOS ordering; queue-first interleave otherwise correct |
| `Demuxer::seek` | 532-534 | REAL | — | pure query forwarded (trait conformance per W05-A06) |
| `Demuxer::seekability` | 536-538 | REAL | — | forwards SlowLinear — honest |
| `Demuxer::reanchor` | 540-549 | REAL | S4 | forwards; the "assembler stays intact" claim (541-548) has NO test anywhere (part2 F-002) |

## 5. Test honesty
Tests live in part2 (lines 551-1056) — see `ps.rs#part2.audit.md`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "end-of-stream leftovers surface as a tail note — counted, never silently dropped" | ps.rs:44-45 (module doc) | FALSE | F-001: note queued then EOS returned; engine returns at EOS (pipeline.rs:2175-2180), all CLI walks break |
| "leftovers become tail notes before the honest end" | ps.rs:521-522 (inline) | FALSE | same — delivery order is after the end |
| "the hold is capped at one partial frame + one PES payload (a legal hold can never exceed it)" | ps.rs:45-47 | REAL | 4 095 + 65 536 = 69 631 < 131 072 (parse_head MAX_FRAME_BYTES=4096, MAX_PACKET_BYTES=64 KiB) |
| "each PES's PTS anchors the frames whose first byte rides in it" | ps.rs:33-37 | REAL | crossing logic 402-409; pinned by synthetic test stamps 9 000/11 351 (test passes, nofailfast:2832) |
| "an unparseable walk position re-syncs to the next judgeable frame header (a `Recovered` note)" | ps.rs:42-43 | REAL-UNPROVEN | code correct by read (330-340, 456-472) but no test drives the resync-Found path (part2 F-001) |
| "a reanchor marks the next packet discontinuous and the walk resumes at a pack boundary, so the next PES pushes into a still-valid buffer" | ps.rs:541-548 | REAL-UNPROVEN | forwarded to inner demuxer (W05-A06 verified the demuxer side); the assembler-intact half has zero test coverage repo-wide (`rg reanchor apps/cli/tests` → OGG only) |
| "`mp1`/`mp2`/`mp3` … the split set is exactly the decodable MPEG-audio descriptors" | ps.rs:49-50 | REAL | is_mpeg_audio (203-205); mpg test decodes mp2 end-to-end (44 928 frames == ffmpeg, nofailfast:2830) |
| "the family HAS a `duration_us` to clamp `--start-time` against (the AVI shape)" | ps.rs:77-79 | REAL | open() 153-154 clamps; start_time test (nofailfast:2831) pins the skip |
| PS family reachability "the registry's FINAL wired family" | ps.rs:1-2 | REAL | containers/mod.rs:71 dispatch; real-binary open test ok (nofailfast:3044) |

## 7. Dead / duplicated / unreachable code
None in part1. All four consts, all functions, and every `Demuxer` impl method have live callers (`open` via mod.rs:71; trait methods via the engine's `dyn Demuxer`). REASON_TAIL is "live" only in the mint sense — its delivery is dead per F-001.

## 8. Dependencies & flags
- Uses `demux::ps::PsDemuxer`, `demux::es::mp3::parse_head`, `demux::ps::MAX_PACKET_BYTES`, `media_types`, `plugin_api` (Demuxer/DemuxEvent/SeekSupport), `commands::CommandError`, `crate::demux_host` — all used, no unused imports, no feature gates, no unsafe. The `lumen-cli → streaming` architecture edge noted by the baseline (#11) does not touch this file.

## 9. Open questions for the next agent
- Does the engine's `SessionReport` surface `damaged_demux` to the CLI user (which command prints it)? That determines F-001's user-visible impact strength.
- The demux-crate half (W05-A06) found EOS-adjacent damage gaps of its own — someone should check whether the inner PsDemuxer also queues notes at EOS that the engine never reads (same class as F-001, one layer down).
- SelectDemuxer (demux_host.rs) `start()` filters `MediaKind::Subtitle/Data` tracks out (`_ => false`) — the VOB nav-track deselection the test at 926-930 observes; confirm the engine treats a nav-Data track identically when reached via the desktop bridge.
