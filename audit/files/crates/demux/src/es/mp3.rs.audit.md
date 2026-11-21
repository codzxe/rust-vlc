# AUDIT: crates/demux/src/es/mp3.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 690 / 21 (incl. 5 pub consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/src/demux_host.rs:458 `containers::open` → apps/cli/src/containers/mod.rs:65-66 `"mp3"/"mp2" => es::open_mpeg_audio` → containers/es.rs:94 `RawMp3Demuxer::new`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | rg `RawMp3Demuxer` (callers: apps/cli/src/containers/es.rs:73,94,128; tests es/tests.rs, tests/es_corpus.rs) · rg `damage_total|frames_demuxed|id3_skipped` (mp3's `damage_total` has zero callers) · rg `EXTENSIONS` (`es::mp3::EXTENSIONS` zero refs) · sed -n verification of every quoted range · baseline log cites below |

## 1. One-paragraph verdict
The raw MPEG-audio demuxer is genuinely implemented and reachable from the shipped `lumen` CLI: strict ISO header parse with correct bitrate/samplerate/spf/coefficient tables, exact sample-clock pts, bounded open/resync scans, typed sticky truncation, Xing/Info/VBRI first-frame skip, and a real damage/event model. Baseline executed the demux suite green (462 passed / 0 failed, incl. 5 mp3-specific tests). Its defects: the documented "ID3v2.2/2.3/2.4" tag skip only implements the 10-byte v2.3/2.4 shape, so a real ID3v2.2 tag (8-byte header, 3-byte size) is decoded with garbage arithmetic that can refuse a valid file or silently skip audio; seek is refused outright where the sibling ADTS demuxer proves CBR seek is format-inherent arithmetic; a mid-stream sample-rate change keeps the first frame's timebase (pts drift in wall time); and two dead pub accessors/consts plus a swallowed read error round it out.

## 2. What actually works (verified)
- Frame walk + exact sample clock: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1689` `test es::tests::mp3_walks_frames_with_exact_sample_clock ... ok` (demux lib suite: 462 passed / 0 failed / 0 ignored, log lines 1489-1491ff).
- Truncation/refusal/resync policies executed: same log lines 1686-1688 (`mp3_midstream_garbage_resyncs_with_recovered_event`, `mp3_no_sync_is_typed_unsupported_after_scan`, `mp3_truncated_final_frame_is_typed_sticky` — all ok) plus `fuzz_lite_mutations_never_panic_or_hang` ok (line 1683).
- Corpus lane vs ffprobe packet counts: `test corpus_mp3_lanes_match_ffprobe_packet_counts ... ok` (test-workspace.log.txt:2066) — REAL-UNPROVEN at evidence level: the test body `else { eprintln!("SKIP es corpus…"); }` passes identically when the corpus dir is absent, and the log's `ok` does not distinguish ran vs skipped.
- Reachability: rg chain above — `RawMp3Demuxer` is constructed on the `mp3`/`mp2` probe lanes of the shipped CLI.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/es/mp3.rs:7-8 (claim), 352-377 (code)
Code:
```
//! The demux-side thin walk: an ID3v2.2/2.3/2.4 tag skip at the head
//! (and between frames), the 4-byte frame header (ISO/IEC 11172-3
```
```
        let mut size = u64::from(tag[6] & 0x7F) << 21
            | u64::from(tag[7] & 0x7F) << 14
            | u64::from(tag[8] & 0x7F) << 7
            | u64::from(tag[9] & 0x7F);
```
What is wrong: The doc claims ID3v2.2 support, but the parser only implements the v2.3/v2.4 shape (10-byte header, flags at byte 5, 4×7-bit syncsafe size at bytes 6..10). ID3v2.2 has an 8-byte header (no flags byte; 3-byte size at bytes 5..8), so for a real v2.2 tag the code decodes two payload bytes (tag[8], tag[9]) as size bits: the computed skip is garbage. Additionally the `& 0x7F` masking silently accepts non-syncsafe size bytes that the probe's own `id3v2_end` (media-probe/src/signature/es.rs:160-162) refuses ("not syncsafe — not a tag size"), so demuxer and probe disagree on what a tag is.
Why it matters: A valid v2.2-tagged MP3 (pre-1999 encoders; ffmpeg handles it) either (a) gets `next > len` from the bogus size → `Err(truncated)` at line 373 — a valid file refused at open, or (b) lands mid-payload: the open scan then false-syncs on any 0xFF 0xFB-looking bytes inside the tag (an APIC image, lyrics), or silently skips real audio frames that get logged as "damage". The v2.2 lane is also untested: `adts/test_support.rs:109-118 id3v2_tag` only ever mints a v2.3 tag.
What correct looks like: Version-discriminate on tag[3] (0x02 → 6-byte header + 3-byte size at tag[5..8]; 0x03/0x04 → current path; footer bit only valid for v2.4), and refuse non-syncsafe size bytes instead of masking, matching the probe's gate.
Confidence: HIGH — Verification: read (spec arithmetic traced; cross-checked against probe's id3v2_end and test_support's v2.3-only fixture). Not executed (no cargo allowed).

### F-002 [S3]
Where: crates/demux/src/es/mp3.rs:680-689
Code:
```
    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "demux.es.mp3.seek.unsupported",
        ))
    }
```
What is wrong: Unconditional typed refusal; `seekability()` is not overridden (default `SeekSupport::None`). The host's justification ("no index exists to map a target onto", apps/cli/src/containers/es.rs:63-66) is false for CBR streams — the common case — where byte offset is exact slot arithmetic, exactly the precedent the same crate's ADTS demuxer implements (`adts/mod.rs:1198-1229`: ADIF CBR seek = format-inherent arithmetic, `SeekSupport::Fast { indexed: false }`, and ADTS gets `SlowLinear` scan landing).
Why it matters: Every raw MP3/MP2 playback is unseekable in-session; the engine's seek flow (doc 03 §6) dead-ends at this lane while the strictly-analogous raw-AAC lane seeks. This is capability drift within one card, honestly typed but a functional gap in realistic use.
What correct looks like: CBR landing via `frame_index = target_ns × sample_rate / (spf × 1e9)`, `pos = first_frame_pos + Σ frame_size` (slot arithmetic handles padding); at minimum `SeekSupport::SlowLinear` with a header-scan landing like ADTS.
Confidence: HIGH — Verification: read; sibling implementation quoted at adts/mod.rs:1198-1231.

### F-003 [S3]
Where: crates/demux/src/es/mp3.rs:559-579
Code:
```
        let base = track.timing.base;
```
```
        let pts = Stamp::Present(Timestamp::new(self.samples_clock, base));
```
```
        self.samples_clock = self
            .samples_clock
            .saturating_add(i64::from(h.samples_per_frame));
```
What is wrong: On a mid-stream sample-rate change the code emits `DemuxEvent::Discontinuity` (line 575) but keeps minting pts on the first frame's `1/first_rate` base while adding the new frame's `samples_per_frame` ticks — a unit mismatch. After a 44.1k→48k change, playback time advances ~8.8% slower than wall time and the error grows without bound.
Why it matters: Timestamp drift / A-V desync on parameter changes is exactly the class the module's own docs claim is handled ("a parameter change mid-stream (rate) is surfaced as Discontinuity — the ADTS precedent", lines 22-24). Surfaced yes; timestamped correctly no.
What correct looks like: Rebase the clock at the discontinuity (new base = 1/new_rate, pts carries accumulated wall time), or track a running nanosecond clock and convert per-packet.
Confidence: MEDIUM — the arithmetic is certain; realistic frequency of mid-stream rate changes in raw mp3 files is low (pathological files), hence S3 not S2. Verification: read.

### F-004 [S4]
Where: crates/demux/src/es/mp3.rs:622-625
Code:
```
        let bytes = match self.reader.read_prefix_at(need, self.walk) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
```
What is wrong: A read error (I/O failure) is swallowed and reported as "not a metadata frame"; the Xing frame would then be emitted as an audio packet. The error is not lost (the packet read at lines 560-562 hits it) but the classification inversion is silent.
Why it matters: Swallowed-error pattern the checklist names; on a transient I/O error the observable behavior changes (a metadata frame becomes a packet) instead of failing at the first read.
What correct looks like: Propagate the error (`?`) or record it in the damage log.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/demux/src/es/mp3.rs:329-333
Code:
```
    /// Total bytes skipped by bounded recovery + tags.
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }
```
What is wrong: Dead public API — zero callers. rg over crates/ apps/ tests/ finds `damage_total` used only by the mkv/ps/annexb/ts/avi demuxers' own definitions; nothing calls the mp3 one (tests use `id3_skipped()`/`frames_demuxed()`).
Why it matters: Copy-paste sibling-symmetry accessor; maintenance surface asserting observability nobody consumes.
What correct looks like: Delete, or wire it into the CLI harvest facts like `head()` is.
Confidence: HIGH — Verification: rg (commands listed in header).

### F-006 [S4]
Where: crates/demux/src/es/mp3.rs:71-72
Code:
```
/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["mp3", "mp2"];
```
What is wrong: `es::mp3::EXTENSIONS` has zero references in the tree (rg `mp3::EXTENSIONS` → nothing); the probe registry carries its own extension tables (media-probe/src/signature/mod.rs:54). The doc comment claims it is "the R3 registry row" — nothing consumes it as such.
Why it matters: Dead constant whose comment asserts a wiring that does not exist.
What correct looks like: Either consume it from the probe registry round-trip or drop the claim/comment.
Confidence: HIGH — Verification: rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_OPEN_SCAN_BYTES`/`MAX_RECOVERY_SCAN_BYTES`/`MAX_ID3_BYTES`/`MAX_FRAME_BYTES` consts | 58-69 | REAL | — | Ceilings enforced at 365, 229, 419-456; 4096 > largest table frame 2881 (V2.5 L2/3 160k@8kHz = 2881, doc arithmetic checks out) |
| `EXTENSIONS` const | 71-72 | DEAD | S4 | F-006 |
| `MpegVersion`/`Layer`/`FrameHead` types | 74-121 | REAL | — | Bit assignments match ISO 11172-3 §3.1 |
| `table_bitrate` | 140-156 | REAL | — | Rows match ISO tables; rejects index 0 (free format — documented refusal) and 15 |
| `table_sample_rate` | 158-169 | REAL | — | V2.5/V2/V1 rows correct; index 3 reserved refused |
| `parse_head` | 180-244 | REAL | — | Strict: sync, reserved version/layer, emphasis 10 refused; coeff 48/144/72 correct; frame_size bounds-checked [4, 4096] before any allocation; no panic paths found |
| `descriptor` | 247-256 | REAL | — | Reserved→"mp3" arm unreachable (parse refuses); `bad_header(0)` fallback never fires (static ASCII) |
| `RawMp3Demuxer::new` | 293-309 | REAL | — | Delegates typed refusals to SourceReader::new (empty/unseekable refused — test `empty_source_refuses_at_construction` ok, baseline log line 1681) |
| `frames_demuxed` | 312-315 | REAL | — | Test/corpus-used |
| `head` | 318-321 | REAL | — | Used by CLI harvest (containers/es.rs:137) + tests |
| `id3_skipped` | 324-327 | REAL | — | Test/corpus-used |
| `damage_total` | 330-333 | DEAD | S4 | F-005 |
| `fail` / `sticky_err` | 335-349 | REAL | — | Sticky replay correct; None arm typed not_started (no panic) |
| `skip_id3` | 352-378 | DEFECTIVE | S2 | F-001 (v2.2 mis-parse, non-syncsafe masking, footer bit applied regardless of version) |
| `open_scan` | 383-414 | REAL | — | Bounded 64 KiB, 3-byte overlap windows, no-sync typed error with head sniff at 0 |
| `resync` | 417-469 | REAL | — | 4 MiB budget → typed ResourceLimit; EOS tail counted as damage + Recovered; no infinite loop traced (walk strictly advances) |
| `mint_track` | 472-500 | REAL | — | Direct `TimeBase{1, rate}` literal valid (rate>0 guaranteed by parse); span end Missing = documented no-extent honesty |
| `step` | 504-593 | REAL | S3 | F-003 pts base; truncation sticky divergence from ffmpeg documented at 539-541; Xing skip only when frames_out==0 (correct — mid-stream Xing is damage, not metadata) |
| `first_frame_is_metadata` | 599-633 | REAL | S4 | F-004 swallowed read error; side-info sizes 17/32 (V1), 9/17 (V2/2.5) and VBRI offset 36 match ISO/spec |
| `Demuxer::start` | 637-656 | REAL | — | Idempotent re-start, sticky replay, head-tag skip first |
| `Demuxer::next_event` | 658-678 | REAL | — | Loop terminates (walk strictly increases); None arm returns EOS not panic |
| `Demuxer::seek` | 680-689 | REFUSAL | S3 | F-002 |

## 5. Test honesty (tests live in es/tests.rs + tests/es_corpus.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `mp3_walks_frames_with_exact_sample_clock` (es/tests.rs:203-237) | YES | Asserts packet count, id3_skipped=16, frames_demuxed=3, per-packet pts ticks i×1152, keyframe, head rate/spf, descriptor "mp3". Mutation: break samples_clock accumulation → pts asserts fail. Executed ok in baseline (log line 1689). |
| `mp3_truncated_final_frame_is_typed_sticky` (es/tests.rs:239-261) | YES | Truncates inside frame 2, asserts 1 packet + typed Corrupt/Truncated + sticky replay. Break the overrun check → packets==2 fails. Baseline ok (line 1688). |
| `mp3_no_sync_is_typed_unsupported_after_scan` (es/tests.rs:263-273) | YES | 300×0x11 refuses with demux.es.no-sync. Baseline ok (line 1687). |
| `mp3_midstream_garbage_resyncs_with_recovered_event` (es/tests.rs:275-289) | YES | 7 garbage bytes between frames; asserts 2 packets + recovered:7 event. Break resync → event missing. Baseline ok (line 1686). |
| `fuzz_lite_mutations_never_panic_or_hang` (es/tests.rs:307ff) | WEAK-MED | LCG mutations over annexb+mp3 base; proves no-panic only on one seed sequence; executed ok (line 1683). |
| `corpus_mp3_lanes_match_ffprobe_packet_counts` (es_corpus.rs:170-229) | YES-if-corpus-present | ffprobe-pinned counts (0139: 79 packets, 576-sample frames, MPEG-2.5; 0145: 6 packets + tag skip). Skip-when-absent pattern makes baseline "ok" ambiguous — REAL-UNPROVEN (see §2). |
| Gap | — | No test for ID3v2.2 (helper mints v2.3 only — adts/test_support.rs:111 `vec![b'I', b'D', b'3', 3, 0, 0]`), no direct test of `first_frame_is_metadata` (Xing/VBRI skip), no test of the 4 MiB resync budget refusal. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "an ID3v2.2/2.3/2.4 tag skip at the head" | mp3.rs:7-8 module doc | FALSE for 2.2 | F-001: 10-byte-only decode at 361-364; v2.2's 3-byte size at tag[5..8] mis-read |
| "a file with no parseable header inside MAX_OPEN_SCAN_BYTES is the typed Unsupported-after-scan refusal" | mp3.rs:29-31 | REAL | open_scan 383-414 + test `mp3_no_sync…` ok (baseline log 1687) |
| "a frame whose computed size overruns end-of-stream is the typed sticky truncation" | mp3.rs:32-34 | REAL | step 537-542 + test ok (baseline log 1688); ffmpeg divergence (short packet) honestly documented at 539-541 |
| "ffmpeg additionally synthesizes an attached-picture video stream … deliberately not mirrored" | mp3.rs:39-44 | REAL | 0145 corpus lane pins audio-only 6 packets (es_corpus.rs:206-219) |
| Xing/Info/VBRI first frame skipped like ffmpeg | mp3.rs:543-548 | REAL-UNPROVEN | Code exists (599-633) with measured-comment rationale; no direct test; corpus 0139 asserts only packet counts — the skip branch has no executed proof |

## 7. Dead / duplicated / unreachable code
- `damage_total()` — dead (F-005). `EXTENSIONS` — dead (F-006).
- `skip_id3` is a near-verbatim duplicate of `adts/mod.rs:474-501` skip_id3 (same v2.2 bug — cross-file duplication; flagged here for mp3, the adts file belongs to another agent).
- `descriptor()`'s `Layer::Reserved` arm unreachable (parse refuses reserved layers).

## 8. Dependencies & flags
No feature flags gate this file (module `es` unconditional in the demux crate). No unsafe. Uses `avi::riff::{PayloadSlots, SourceReader}` and `avi::header::DamageLog` — cross-family reuse from the avi module (layering smell, works). Pool pre-allocates 2×4 KiB per demuxer instance — bounded.

## 9. Open questions for the next agent
- Does any shipped lane ever feed an ID3v2.2-tagged file (corpus 0071-style)? If yes, F-001 is user-visible; if the corpus lacks v2.2 entirely, it stays latent.
- adts/mod.rs skip_id3 shares F-001 verbatim — the adts-auditing agent should cross-link.
- `SelectDemuxer::new` skip-watermark path (containers/es.rs:114) drops frames by pts pre-filter — verify it handles the F-003 drifted clock.
