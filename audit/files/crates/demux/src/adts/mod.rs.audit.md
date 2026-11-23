# AUDIT: crates/demux/src/adts/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 1268 / 48 (incl. `Demuxer` impl, BitReader, PCE) |
| File verdict | REAL (with S3 recovery/doc defects) |
| Reachable from shipped entry point? | YES: `lumen` CLI → `apps/cli/src/containers/mod.rs:52 "adts" => adts::open` → `apps/cli/src/containers/adts.rs:66 AdtsDemuxer::new(stream)` → this file |
| Findings | S0: 0 S1: 0 S2: 0 S3: 6 S4: 5 |
| Commands run | `cargo test -p demux --lib adts` → 32 passed / 0 failed; standalone rlink probes (see F-001/F-003/F-004) executed against the built libdemux |

## 1. One-paragraph verdict
This is a genuinely implemented, heavily-tested ADTS/ADIF demuxer — header bit-extraction is correct against ISO 13818-7, timing is exact frame arithmetic, every error path is typed, and 32 unit tests + corpus + fuzz-lite pass (I ran them). The hostile focus areas hold up: frame sync/length math is checked (length ≥ header, ≤ stream end, sticky truncation), and no panic path exists (no unwrap/panic in non-test code; all bounds checked). The real defects are in the damage-recovery edge: a 6-byte unscanned hole at every 64 KiB resync window boundary silently drops a recoverable frame (executed proof below), a persistent mid-stream parameter change floods one `Discontinuity` event per frame forever, ADIF blocks larger than the 8200-byte pool cap allocate per-packet against the module's own zero-alloc claims, and the module docs make at least three claims the code contradicts (zero per-frame heap traffic, 9-byte stack CRC scratch, pool bound for ADIF).

## 2. What actually works (verified)
- `cargo test -p demux --lib adts` → `test result: ok. 32 passed; 0 failed` — covers header round-trips, clean-walk pts/duration arithmetic, all 14 typed error codes, ID3 head+interframe skip, ADIF CBR walk + cross-container essence agreement, seek round-trips, fuzz-lite fixtures.
- Bit-extraction audited manually against ISO 13818-7 §6.5.1 (`mpeg2`=0x08, layer=0x06, `profile`=bits 7-6, `sampling_index`=bits 5-2, 13-bit `frame_length` split 2/8/3, 11-bit `buffer_fullness` 5/6): all correct.
- Standalone binary linked against the built rlib (rustc 1.99.1) reproduced the walk, resync, ADIF lanes from public API — proving the module is real and reachable as documented.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/adts/mod.rs:694-731
Code:
```
            let window_end = (at + 64 * 1024).min(budget_end.max(at + 1));
            let window = self.reader.read_prefix_at((window_end - at) as usize, at)?;
            let mut found = None;
            for (i, pair) in window.windows(2).enumerate() {
                if pair[0] == 0xFF
                    && pair[1] & 0xF0 == 0xF0
                    && i + 7 <= window.len()
```
What is wrong: a syncword candidate needs `i + 7 <= window.len()`, so pair-start positions in the last 6 bytes of each 64 KiB window can never be validated; on `None` the scan advances `at = window_end` (line 731) and never revisits those 6 bytes. A real frame whose header starts there is silently skipped and the scan continues into its payload. Why it matters: recovery after sync loss can drop a perfectly valid next frame or resync into garbage; ~6 bytes of blind spot per 64 KiB of scan (up to 64 windows per bounded resync). What correct looks like: advance to `window_end - 6` (overlap the tail) or extend the last candidate's 7-byte read past the window edge. Confidence: HIGH — Verification: ran (standalone probe vs. built rlib: frame at window1 hole offset 65638 → `packets = 1` + "lost sync at end of stream", identical frame 8 bytes earlier → `packets = 2`).

F-002 [S3]
Where: crates/demux/src/adts/mod.rs:63-70 (claim) vs 654, 663, 761-763, crates/demux/src/avi/riff.rs:307-311 (reality)
Code:
```
//! decoder — zero per-packet allocation in steady state
//! ([`pool_bytes`](AdtsDemuxer::pool_bytes) is the observable). The
//! CRC scratch is a 9-byte stack array; the only per-frame heap
//! traffic is the event queue node.
```
vs `read_prefix_at`: `let mut out = vec![0u8; len];` (riff.rs:308) and `emit_adts_frame`: `let track = self.track.clone()` (mod.rs:761-763) where `Track.codec` is `CodecDescriptor(String)` (media-types/src/ids.rs:573).
What is wrong: the doc's allocation claim is false. Per frame the code heap-allocates: (1) `skip_id3`'s 10-byte Vec (mod.rs:479, every frame), (2) `read_frame_head`'s 7-byte Vec (mod.rs:654, every frame), (3) the 9-byte CRC Vec when protection is present (mod.rs:663 — there is no "9-byte stack array" anywhere; `crc16_header` takes a caller-provided slice), (4) `track.clone()`'s String per frame. Why it matters: the module sells itself on a zero-copy/zero-alloc contract ("notes for 5.X2") that its own hot path violates; the pool only covers payload bytes. What correct looks like: reuse a stack/scratch buffer for header reads (`read_prefix_into` into a `[u8; 10]`), store the parsed `TimeBase`/`TrackId` instead of cloning the Track. Confidence: HIGH — Verification: read (riff.rs:308 `vec![0u8; len]` cited; Track clone cited).

F-003 [S3]
Where: crates/demux/src/adts/mod.rs:783-793
Code:
```
        let reference = match self.head.as_ref() {
            Some(RawAacFormat::Adts(h)) => *h,
            _ => raw,
        };
        let param_change = raw.sampling_index != reference.sampling_index
            || raw.channel_config != reference.channel_config
            || raw.profile != reference.profile;
        if param_change {
            self.damage.note(0, "adts: mid-stream parameter change");
            self.pending.push_back(DemuxEvent::Discontinuity);
        }
```
What is wrong: `reference` is the *first* frame's header, never updated after a change. Every subsequent frame of a persistent change (e.g. two concatenated .aac files with different channel counts — a realistic user action) emits another `Discontinuity` event, marks the packet discontinuous, and logs another damage note, forever. Why it matters: event/damage spam for the whole remainder of the stream; the engine's discontinuity accounting is inflated per frame. What correct looks like: update `self.head` to the new header on change (emit the event once), or compare against the previously emitted frame. Confidence: HIGH — Verification: ran (standalone probe: 1 stereo + 5 mono frames → `packets=6 discontinuity_events=5`). Note the unit test `mid_stream_parameter_change_is_a_discontinuity` (tests.rs:339-353) only exercises ONE changed frame, so it cannot catch the flood.

F-004 [S3]
Where: crates/demux/src/adts/mod.rs:392 and 864-867 vs 444-445 (doc) and riff.rs:597/654-670
Code:
```
            pool: PayloadSlots::new(2, MAX_FRAME_BYTES as usize),
```
```
        let take = info.block_bytes.min(left);
        let data = self
            .pool
            .fill(take as usize, |buf| self.reader.read_prefix_into(buf, at))?;
```
What is wrong: the pool's slot cap is `MAX_FRAME_BYTES` (8200), but ADIF `block_bytes = bitrate × 128 / rate` is bounded only by the 23-bit bitrate — up to 145,986 bytes (8,388,607 bps @ 7,350 Hz); any legal ADIF with `block_bytes > 8200` (e.g. 4.5 Mbps @ 48 kHz → 12,000-byte blocks) takes the spill path (riff.rs:660 `let mut fresh = vec![0u8; want];`) — a fresh heap allocation per packet, never pooled, contradicting the `pool_bytes` doc "bounded by `2 × MAX_FRAME_BYTES`" (the pool holds zero ADIF bytes in that mode) and the module's zero-per-packet-allocation claim. Why it matters: silent per-packet allocation of up to ~146 KiB per block in the ADIF lane; the D-1 observable reads 0 and reports health. What correct looks like: size the pool for `max(MAX_FRAME_BYTES, block_bytes)` at ADIF open (block_bytes is known there). Confidence: HIGH — Verification: ran (standalone probe: `adif_stream(3,2,5,12000)` → 5 packets, `pool_bytes=0` all blocks).

F-005 [S3]
Where: crates/demux/src/adts/mod.rs:522-542
Code:
```
            // The first ADTS claim: strict, typed validation.
            if raw.sampling_index >= SAMPLE_RATES.len() as u8 {
                return Err(error::bad_sampling_index());
            }
            if raw.channel_config == 0 {
                return Err(error::bad_channel_config(at));
            }
```
What is wrong: the bounded open scan (`MAX_OPEN_SCAN_BYTES`, 64 KiB) aborts at the *first* syncword+layer-00 claim with a typed refusal instead of continuing the scan for a valid frame. A file whose first frame header is damaged (bad sampling index, channel config 0, CRC mismatch, or length < header — all checked with immediate `return Err`) fails to open even when a perfectly valid ADTS frame sits 10 bytes later inside the scan window. Why it matters: first-frame damage — a realistic torn-recording artifact — is fatal at open where ffmpeg's parser scans on and recovers; the 64 KiB scan budget exists but is thrown away on the first claim. What correct looks like: on strict-validation failure of a *candidate*, advance `at` and keep scanning; refuse typed only when the window is exhausted. (The doc at lines 43-49 documents this policy, so it is an honest-but-fragile divergence, not a hidden one.) Confidence: MEDIUM — Verification: read (all four early-return arms cited); behavior confirmed by unit tests asserting the refusals (tests.rs:237-278).

F-006 [S3]
Where: crates/demux/src/adts/mod.rs:669-677 and 704-715
Code:
```
        if self.walk + u64::from(raw.frame_length) > len {
            // Declared frame overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet instead — the
            // honestly-recorded divergence).
            return Err(error::truncated(
```
What is wrong: one flipped bit in the `frame_length` field of a frame near end-of-stream (within 8 KiB of EOS), or a garbage 0xFFFx candidate in the resync scan whose 13-bit length overruns, converts recoverable damage into total stream death: the error is terminal and replays (sticky) on every subsequent `next_event`. The resync path escalates identically (line 714) even for a candidate found in *garbage* — the scan cannot skip past an overrunning false claim. Why it matters: single-bit corruption near the tail destroys the entire remaining playback; ffmpeg emits a short final packet. What correct looks like: treat an overrunning mid-walk claim as damage (resync past it) and reserve the sticky truncation for the strict first frame; or at minimum don't abort resync on a candidate that was never verified beyond syncword+len. Confidence: MEDIUM — Verification: read (both arms cited; unit test truncated_frame_is_the_typed_truncation proves stickiness of this exact path).

F-007 [S4]
Where: crates/demux/src/adts/mod.rs:151
Code:
```
    header.len() >= 2 && header[0] == 0xFF && header[1] & 0xF0 == 0xF0
```
What is wrong: `probe` ignores the layer bits, so any MPEG audio frame (MP3 0xFFFB, MP2 0xFFF3) matches; only the CLI's media-probe lane refines (`prefix[1] & 0xF6 == 0xF0`, media-probe/src/signature/phase5.rs:141-143). The raw-ES lane (`es/aac.rs:21` calls this probe) inherits the layer-blind claim. Why it matters: a misnamed .aac MP3 file can be claimed as ADTS by any caller of this public probe without the phase5 refinement. What correct looks like: check `& 0xF6 == 0xF0` here (2 bits, no cost). Confidence: HIGH — Verification: read + grep of both call sites.

F-008 [S4]
Where: crates/demux/src/adts/mod.rs:553-556
Code:
```
        let head = self.reader.read_prefix_at(4096, at)?;
        if head.len() < 8 {
            return Err(error::not_adts(&head));
        }
```
What is wrong: a file that *is* ADIF-prefixed but shorter than 8 bytes (truncated header) is typed `demux.adts.not-adts` — the error says "not ADTS" about a file that declared ADTS; the honest code is a truncation. Same for `adif_no_data`'s `Truncated{expected:1, found:0}` shape (error.rs:182-192) which fabricates 1/0 instead of real offsets. Confidence: HIGH — Verification: read.

F-009 [S4]
Where: crates/demux/src/adts/mod.rs:104-106
Code:
```
pub mod error;
pub mod fuzz;
pub mod test_support;
```
What is wrong: `fuzz` (~97 loc harness) and `test_support` (~189 loc fixture builders) are un-gated public modules compiled into every release binary of the demux crate (lumen-cli depends on demux); "Test-support surface, not player surface — nothing in the demuxer uses it" (test_support.rs:6) is accurate about usage and exactly the problem: dead code shipped. Confidence: HIGH — Verification: read + grep (only test/doctest callers).

F-010 [S4]
Where: crates/demux/src/adts/mod.rs:561-569
Code:
```
            // Byte-terminated null string (the GPAC parser's
            // convention; bounded so a lying length cannot loop).
            let mut seen = 0u32;
            while bits.read_byte()? != 0 {
                seen += 1;
                if seen > 64 {
                    return Err(error::adif_bad_params());
                }
            }
```
What is wrong: the ADIF `copyright_id` is read as a NUL-terminated string (≤64 bytes). If ISO/IEC 14496-3's field is the fixed 72-bit `copyright_id` (as I recall the spec table), a spec-compliant 9-byte non-terminated ID would desynchronize every field after it. FAAD2 also reads NUL-terminated, and I cannot access the spec text here. UNVERIFIED (no spec source available); flag for a spec-owning agent. The 64-byte bound is also stricter than FAAD2's 72-80. Confidence: LOW — Verification: UNVERIFIED.

F-011 [S4]
Where: crates/demux/src/adts/mod.rs:1198-1220 and 921-941
Code:
```
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        let save = self.reader.position();
```
What is wrong: (a) `seek` before `start()` returns `demux.adts.seek.out-of-range` instead of `not-started` (reanchor at 1235-1237 guards properly) — inconsistent taxonomy; (b) `scan_landing_adts` omits the layer check (`head[1] & 0x06`) that `open_adts` applies (line 518), so garbage syncword candidates on a damaged stream skew the seek landing; (c) `self.reader.seek_to(save)?` (line 1205) propagates before `result?`, discarding the original landing error if the restore fails. All limited blast radius. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 147-152 | REAL | S4 | layer-blind (F-007) |
| AdtsHeader::parse | 193-214 | REAL | — | bit math verified vs spec |
| AdtsHeader::header_len | 218-220 | REAL | — | |
| AdtsHeader::samples | 224-226 | REAL | — | 1024×blocks |
| AdtsHeader::sample_rate | 230-236 | REAL | — | 13-entry table, 13-15→None |
| AdtsHeader::descriptor | 241-251 | REAL | — | profile map incl. SSR/LTP |
| AdifInfo::descriptor | 282-292 | REAL | — | duplicate of above (S4 dup) |
| AdifInfo::sample_rate | 296-298 | REAL | S4 | panics on externally-built idx>12 (only internal construction — unreachable) |
| StickyShape::of/mint | 333-343 | REAL | — | replayable error |
| AdtsDemuxer::new | 379-396 | REAL | — | pool cap 8200 (F-004) |
| format/stream_info/frames_demuxed/damage_total/id3_skipped/pool_bytes/track_duration | 399-463 | REAL | S3 | pool_bytes doc false for ADIF (F-004) |
| fail | 467-470 | REAL | — | sticky only via this; walk errors re-derive (consistent) |
| skip_id3 | 474-501 | REAL | — | syncsafe math correct; 16 MiB ceiling |
| open_adts | 505-547 | REAL | S3 | first-claim refusal (F-005), per-byte read alloc (F-002) |
| open_adif | 551-618 | REAL | S4 | copyright string UNVERIFIED (F-010), CBR/VBR/multi-PCE gates correct per spec shape |
| mint_track | 621-648 | REAL | — | direct TimeBase construction documented |
| read_frame_head | 652-679 | REAL | S3 | overrun→sticky death (F-006) |
| resync | 688-749 | REAL | S3 | window hole (F-001), overrun escalation (F-006) |
| fill_frame | 752-757 | REAL | — | |
| emit_adts_frame | 760-806 | REAL | S3 | param-change flood (F-003), Track clone (F-002) |
| step_adts | 809-845 | REAL | — | ID3/trailing-fragment handling verified by tests |
| step_adif | 848-902 | REAL | S3 | pool spill (F-004) |
| scan_landing_adts | 907-943 | REAL | S4 | no layer check (F-011b) |
| landing_adif | 946-962 | REAL | — | past-end refusal documented asymmetry |
| target_nanos | 964-972 | REAL | — | |
| crc16_header | 979-992 | REAL | — | CRC-16/MPEG-2 verified by protected-frame tests |
| be16 | 994-996 | REAL | — | |
| ProgramConfigElement::parse | 1007-1064 | REAL | — | bit-counts vs spec shape verified |
| BitReader (6 methods) | 1070-1125 | REAL | — | all reads length-checked |
| Demuxer::start | 1128-1164 | REAL | — | sticky via fail(); verified by tests |
| Demuxer::next_event | 1166-1196 | REAL | — | pending-first ordering correct |
| Demuxer::seek | 1198-1220 | REAL | S4 | F-011 |
| Demuxer::seekability | 1222-1233 | REAL | — | matches test assertions |
| Demuxer::reanchor | 1235-1267 | REAL | — | D4 commit-after-success correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_parse_roundtrips_the_field_set | YES | every field asserted incl. bit splits |
| clean_walk_stamps_frame_arithmetic | YES | exact pts/duration arrays incl. multi-block frame |
| id3_head_and_interframe_tags_are_skipped | YES | exact skipped byte counts |
| pool_memory_stays_bounded | YES (weakly) | asserts pool ceiling but ADTS-only; cannot catch F-004 (ADIF spill) |
| lost_sync_mid_stream_resyncs | YES | single-frame gap; cannot catch F-001 (window hole needs ≥64 KiB layout) |
| truncated_frame_is_the_typed_truncation | YES | code + replay asserted |
| sampling_index_fifteen/channel_config_zero/crc_mismatch/bad_header | YES | exact codes |
| mid_stream_parameter_change_is_a_discontinuity | PARTIAL | one changed frame only — blind to the per-frame flood (F-003) |
| adif_* (7 tests) | YES | VBR/multi-program/params/no-data/partial/seek all typed |
| adts_seek_roundtrip / past_the_end | YES | at-or-before semantics pinned |
| committed_mutation_fixtures... | NO (fuzz-lite) | absence-of-panic only, asserts nothing else (by declared design) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero per-packet allocation in steady state … only per-frame heap traffic is the event queue node" | mod.rs:67-70 | FALSE | riff.rs:308 `vec![0u8; len]` ×2+/frame; Track clone (F-002) |
| "The CRC scratch is a 9-byte stack array" | mod.rs:69-70 | FALSE | no stack array; 9-byte read is a heap Vec (mod.rs:663, riff.rs:308) |
| pool "bounded by 2 × MAX_FRAME_BYTES" | mod.rs:444-445 | FALSE for ADIF | F-004 executed: pool_bytes=0 with 12000B blocks |
| "truncated, sticky" for EOS overrun | mod.rs:48-49 | REAL (indirect) | not recorded in `self.sticky` but deterministically re-derived; test replays error (tests.rs:219-224) |
| ffmpeg corpus differential ("verified frame-by-frame") | mod.rs:36-37 | REAL-UNPROVEN | corpus suite exists (tests/adts_corpus.rs, 32 tests pass); I did not re-run the ffmpeg differential |
| "ID3-prefixed streams … the registry sniffs past the tag at open" | mod.rs:144-145 | REAL | phase5/containers confirmed + test |

## 7. Dead / duplicated / unreachable code
- `AdifInfo::descriptor` duplicates `AdtsHeader::descriptor` verbatim (241-251 vs 282-292) — copy-paste pair.
- `test_support` + `fuzz` modules: un-gated, shipped, production-unused (F-009).
- `probe`'s ADIF arm is dead from the CLI lane (media-probe checks ADIF itself) but live for `es/aac.rs`.

## 8. Dependencies & flags
- No external deps (std + workspace crates: media-types, plugin-api). No feature gates on this module — everything compiled in the release lane.
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` — cross-family coupling of ADTS to AVI internals (layering smell, works).

## 9. Open questions for the next agent
- F-010: does ISO/IEC 14496-3 define ADIF `copyright_id` as fixed 72 bits or NUL-terminated? Spec-owning agent must settle it.
- es/aac.rs uses layer-blind `adts::probe` (F-007) — the ES-lane auditor should check whether an MP3-named-`.aac` is mis-claimed there.
- Does the engine's discontinuity accounting degrade under F-003's flood (session.rs demux_discontinuities counter)? Worth an engine-side look.
