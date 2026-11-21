# AUDIT: crates/demux/src/ps/mod.rs#part2 (lines 610-1219)
| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-21 |
| Lines / functions | 610 (chunk 610-1219 of 1219; part1 = 1-609 → W05-A07; boundary cuts through `parse_pes_full` 497-619 — tail 608-619 read in full for coherence) / 16 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.{mpg,mpeg,ps,vob,m2p}` → `crates/media-probe/src/signature/phase5.rs:183-190` `PsSignature` ("00 00 01 BA" + extensions) → `apps/cli/src/containers/mod.rs:71` `"mpegps" => ps::open(stream, opts)` → `apps/cli/src/containers/ps.rs:144` `PsDemuxer::new` → the `Demuxer` impl audited here. Baseline execution: 28 `ps::` unit tests + doctest ok (orchrun-test-workspace-nofailfast.log.txt:1833-1860, :6370), `tests/ps_fixtures.rs` 4/4 bit-exact differential ok (:2176-2184), `mpeg12_corpus.rs` 3/3 ok (:1142-1150), CLI `containers::ps::` 4/4 ok (:2830-2833). No `lumen open` smoke of a PS/VOB exists in baseline — samples are MP4/AVI only) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 2 |
| Commands run | full Read of 1-330 (part1 context) + 331-1219 (whole file); `sed -n` on every quoted range; `rg -n "fn seek|reanchor"` crates/plugin-api/src/demux.rs (trait contract: seek = pure query, reanchor = acting half — ps conforms); `rg "Mpeg12Facts|descriptor_gate"` → test-only consumers (crates/codecs/tests/mpeg12_corpus.rs), none in production; `rg "\.keyframe"` → sole production consumer `crates/transcode/src/mp4mux.rs:299`; `rg "next_event"` apps/cli + crates/playback-engine/src/pipeline.rs:2141-2185 (Err arm :2182-2185 → `report.fatal`, no retry loop); baseline log greps listed above |

## 1. One-paragraph verdict
Part 2 is the PS demuxer's working half — pre-scan, packet emission, seek — and it is genuinely implemented and executed: the fixture differential pins demuxed payloads bit-for-bit against ffmpeg extractions and all 28 unit tests pass in the baseline run. But the MPEG-1 PES lane is missing the header-vs-declared-length invariant its own MPEG-2 sibling enforces ten lines earlier, so a malformed MPEG-1 packet underflows `payload_len` (panic in debug profiles, hard typed error in release where the module's own design promises damage-noted skip); stamp attribution merges every private-1 substream into the first registered one, so the standard DVD shape (AC-3 + sub-picture) gets an UNKNOWN span on the second track; and the resync scanner's loop bound misses a start code occupying the last 3 bytes of its window, turning a recoverable 64 KiB damage region into a hard open failure. None of the three is covered by a test — the committed fixtures each carry a single private-1 stream, benign garbage, and well-formed MPEG-1 packets.

## 2. What actually works (verified)
- `scan_streams` walk machine (624-714): every branch advances `pos` monotonically (pack end / packet_end / +6+ext / resync next); no infinite-loop path found including the underflow case (mod-2^64 self-correction lands exactly on `packet_end`, 709); 0xB9 ends the scan; unclassified private-1 substreams and skipped stream ids are damage-noted, never silent — verified against tests `private1_unclassified_substream_is_skipped_loudly`, `padding_stream_and_system_header_are_skipped_positionally` (baseline :1851, :1850 ok).
- Private-1 routing peeks (675-696): 1-byte substream read, `PrivateKind::of_substream` ranges (0x80-87/0x88-8F/A0-A7/20-3F), strip constants (4/7/1) — pinned bit-exact by `private1_ac3_strips_substream_and_three_filler_bytes`, `private1_dts_lpcm_and_spu_route_with_their_strips`, and `lpcm_packets_strip_the_full_dvd_prefix` (baseline :1851-1853, :2180 ok).
- `probe_audio_codec` layer bits (798-805): `buf[1] >> 1 & 0x03` → 0b11=mp1, 0b01=mp3, else mp2 — matches the MPEG audio header Layer field mapping; conservative "mp2" fallbacks.
- `resync_from` allocation discipline (817): one bounded `vec![0u8; want]` per resync, window clamped by remaining length (812).
- `build_tracks` (849-877): honest `language: None` (contrast the MP4 lane's garbage `Some("voe")` — INDEX.md "Standing defects"); span only when `last >= first`; 90 kHz base consistent with CLI harvest invariant (`apps/cli/src/containers/ps.rs` debug-asserts `(1, 90_000)`).
- `Demuxer::start` (925-959): typed `not_ps` head refusal, sticky-fail wiring, pre-scan damage surfaced as `Recovered` events before packets — executed by `wrong_head_is_the_typed_not_ps`, `not_started_is_typed_and_start_is_idempotent` (baseline :1859, :1849 ok).
- `next_event` event machine (961-1114): EOS/EOF/0xB9 flush paths all drain pending `Recovered` notes; the 0xBF nav lane emits real Data-track packets (`private2_nav_packets_form_a_data_track`, `vob_nav_packets_are_real_and_zero_padded_by_ffmpeg` — baseline :1854, :2182 ok).
- `seek`/`reanchor` split conforms to the trait contract: `crates/plugin-api/src/demux.rs:36-39` "seek asks the container for its nearest…", `:97` "the pure query [`Demuxer::seek`] never moves" — ps `seek` (1116-1137) only clamps into the SCR extent; `reanchor` (1143-1218) does the compute-then-commit walk (`seek_clamps_into_the_scr_extent_and_reanchor_marks_discontinuity` baseline :1856 ok). `SeekSupport::SlowLinear` (1139-1141) is the honest claim.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ps/mod.rs:608-610 (contrast guard at 532-536)
Code:
```rust
            (at as u64, p, d)
        };
        let data_start = pos + 6 + header_len;
        let payload_len = u64::from(declared_len) - header_len;
```
What is wrong: The MPEG-2 branch enforces `want > usize::from(declared_len)` → typed `bad_pes_header` (532-536) before any subtraction; the MPEG-1 branch computes `header_len` (= `at`, up to 16 stuffing + 10 stamp bytes = 26, 559-607) with **no** check against `declared_len`. A malformed MPEG-1 PES whose header bytes exceed its u16 declared length (e.g. declared_len=1, payload bytes `FF 0F` → at=2) underflows `u64::from(declared_len) - header_len`.
Why it matters: In debug/test profiles (overflow-checks on) this is a panic — `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` would crash on a mutation it never happened to generate (flips are confined to the first 512 bytes, 8 per round, tests.rs:495-520). In the shipped release build it wraps: `scan_streams` survives by mod-2^64 accident (709 lands on `packet_end`), but `next_event` reaches `emit_pes_packet` with `want = payload_len.saturating_sub(strip)` ≈ 2^64 → the guard at 890-893 returns a hard `payload_too_large` error, which the pipeline treats as fatal (`report.fatal = Some(error); return;`, crates/playback-engine/src/pipeline.rs:2182-2185) — one damaged MPEG-1 packet kills the whole stream, contradicting the module's own damage-tolerance design ("damage-noted skip, never an unbounded read", 505-508).
What correct looks like: Mirror the MPEG-2 guard after the MPEG-1 header parse: `if at > usize::from(declared_len) { return Err(error::bad_pes_header(pos)); }` (or damage-note skip) before line 610.
Confidence: HIGH — Verification: read + arithmetic traced; profile behavior per Rust defaults (workspace sets no `overflow-checks` override).

### F-002 [S2]
Where: crates/demux/src/ps/mod.rs:731-744
Code:
```rust
        for stream in &mut self.streams {
            let matches = match stream.key {
                StreamKey::Video(id) | StreamKey::Audio(id) => id == sid,
                StreamKey::Private1(..) => sid == 0xBD,
                StreamKey::Private2 => false,
            };
            if matches {
                if stream.first_pts.is_none() {
                    stream.first_pts = Some(stamp);
                }
                stream.last_pts = Some(stamp);
                return;
            }
        }
```
What is wrong: `note_stamps_for` matches ANY `Private1` stream for every 0xBD PES and `return`s after the **first** registry hit. With two private-1 streams (AC-3 0x80 + sub-picture 0x20 — the standard retail-VOB shape) all 0xBD stamps — both substreams, including stamps from PES whose substream is not even registered — are merged into the first private-1 track's extent, and the second track never receives `first_pts`/`last_pts`.
Why it matters: `build_tracks` (854-860) turns that into `TimeSpan::UNKNOWN` for the SPU track and a polluted span for the AC-3 track — wrong track timing metadata on the most common DVD configuration. The substream is already read one line earlier in `scan_streams` (676-696) and again in `next_event` (1078-1084) but is not threaded into `note_stamps_for` — the signature (728) has no substream parameter at all.
What correct looks like: Pass the peeked substream into `note_stamps_for` and match `StreamKey::Private1(kind, sub)` exactly (as `stream_index_for` at 838-842 already does).
Confidence: HIGH — Verification: read; fixtures each carry a single private-1 stream (ps_fixtures.rs pins: nav+video+ac3, nav+video+dts, nav+video+pcm_dvd), so no test covers the multi-private-1 case.

### F-003 [S3]
Where: crates/demux/src/ps/mod.rs:819-825
Code:
```rust
        let mut i = 0usize;
        while i + 3 < got {
            if buf[i] == 0 && buf[i + 1] == 0 && buf[i + 2] == 1 {
                return Ok(Some(pos + i as u64));
            }
            i += 1;
        }
```
What is wrong: Off-by-one. Valid candidate positions run to `i = got-3` (buf[i+2] must be in bounds); the condition `i + 3 < got` stops at `i = got-4`, so a start code occupying the **final three bytes** of the read window is never examined.
Why it matters: When the full window was read (`got == want`, mid-file damage), the missed edge start code falls through to `Err(error::resync_failed(pos))` (831) — in `scan_streams` (636) that propagates and fails `start()` entirely: a file whose nearest recovery point sits at offset 65533-65535 of the 64 KiB window cannot be opened at all. In the EOF case (`got < want` → Ok(None)) the last 3 bytes are harmless (nothing follows). Probability per resync ≈ 3/65536, but the module advertises bounded recovery as a core damage feature (module docs line 68).
What correct looks like: `while i + 3 <= got` (equivalently `while i + 2 < got`).
Confidence: HIGH — Verification: read + boundary arithmetic; `damaged_bytes_resync_within_the_window` (tests.rs:412-424) uses 4 garbage bytes mid-stream — the start code is deep inside the window, so the test cannot catch this.

### F-004 [S3]
Where: crates/demux/src/ps/mod.rs:782-784
Code:
```rust
        // No sequence header visible: MPEG-2 is the overwhelmingly
        // common flavor; the ES parser re-classifies downstream.
        "mpeg2video"
```
What is wrong: The comment claims a downstream ES re-classification that does not exist in any production path. `demux/src/es/` has no MPEG-1/2 video module (h264/hevc/aac/mp3/annexb only); the only MPEG-1/2 re-classifier in the tree, `codecs::video::mpeg12::facts::Mpeg12Facts::scan`, is consumed exclusively by `crates/codecs/tests/mpeg12_corpus.rs` (rg: zero production callers). The CLI PS container harvests the demuxer's own descriptor verbatim (`apps/cli/src/containers/ps.rs` `harvest_facts`).
Why it matters: The fallback descriptor ("mpeg2video" for any payload without a visible sequence header — including a genuinely MPEG-1 stream whose first PES peek misses the sequence start, or a first-packet B-frame lead) is final: the track carries it to the codec registry, where MPEG-1/2 decode is a typed refusal (`mpeg12.decode.blocked`, codecs/src/video/mpeg12/mod.rs:9-21 — facts-only module, no decoder). A reader of this comment would wrongly assume misclassification self-heals.
What correct looks like: Delete the false claim or implement the re-classification seam it asserts.
Confidence: HIGH — Verification: read + `rg -n "Mpeg12Facts|descriptor_gate"` (test-only) + `rg "mpeg1video|mpeg2video"` over crates/ (production hits: descriptor literals only).

### F-005 [S3]
Where: crates/demux/src/ps/mod.rs:1088-1092
Code:
```rust
                            let (strip, keyframe) = match key {
                                StreamKey::Private1(kind, _) => (kind.strip(), true),
                                StreamKey::Audio(_) | StreamKey::Private2 => (0, true),
                                StreamKey::Video(_) => (0, false),
                            };
```
What is wrong: Every MPEG-PS video packet is emitted with `keyframe: false` — no I-frame detection exists, so the flag is a constant for the entire video lane (MPEG-1/2 I-frames exist in every GOP).
Why it matters: The only production consumer of `Packet::keyframe` is `crates/transcode/src/mp4mux.rs:299` (`sync: packet.keyframe`), whose stss writer (565-573) emits a **zero-entry sync table when no row is sync** — an MP4 remuxed from MPEG-PS would be unseekable in every conforming player. Today that consumer is unreachable from the shipped CLI (`rg transcode|mp4mux` over `apps/` and `playback-engine`: no matches), so the blast radius is latent; the moment the transcode lane ships, PS→MP4 output is broken.
What correct looks like: Either parse the picture start code's picture-type field (the mpeg12 `headers` module already parses picture headers) or default video packets to `true` (all-sync is the safe muxer default; all-non-sync is the worst).
Confidence: HIGH — Verification: read + rg consumer chain (mp4mux.rs:299, 567-573; pipeline.rs has no keyframe read).

### F-006 [S4]
Where: crates/demux/src/ps/mod.rs:944-945 (vs 1073)
Code:
```rust
        if let Err(err) = self.scan_streams() {
            return Err(self.fail(err));
```
What is wrong: The sticky-error discipline (`fail`/`replay`, 406-417) is wired only into `start` (942, 945, 949). `next_event` propagates parse errors raw (`self.parse_pes_full(self.walk, self.mpeg2)?`, 1073; same for `parse_pack_at` 1025 and `length_of` 1042) without recording a sticky shape — so after a mid-stream failure the demuxer is in neither the sticky-failed nor a progressed state; each retry re-reads and re-fails at the same offset.
Why it matters: Inconsistent error contract: the WAV/AVI precedent the module cites ("the WAV StickyShape precedent", 235) makes failures idempotent. The shipped pipeline happens to treat any demux Err as fatal (`pipeline.rs:2182-2185`), so no user-visible loop occurs today — this is contract drift, not a live hang.
What correct looks like: Route `next_event` errors through `self.fail(err)` too (the sticky check at 962 then short-circuits retries).
Confidence: HIGH — Verification: read; `rg "self.fail("` → 3 hits, all in `start`.

### F-007 [S4]
Where: crates/demux/src/ps/mod.rs:895, 917
Code:
```rust
        let at = start;
```
(and, 917:) `generation: GenerationId::FIRST,`
What is wrong: Three hygiene residues in `emit_pes_packet`: (a) `let at = start;` is a dead alias of `start` (895); (b) `generation` is always `GenerationId::FIRST` and the pipeline overwrites it at push anyway (`crates/playback-engine/src/pipeline.rs:2148` `packet.generation = self.generation;`) — dead data minted per packet; (c) damage notes taken between packets are flushed **after** the next packet (1105-1110), so a `Recovered` for bytes damaged before a packet is delivered after it (cosmetic event ordering, observable in the `Recovered`-before-packets contract the module otherwise maintains).
Why it matters: Noise and a misleading event order in the observability layer the module sells as its honesty mechanism.
What correct looks like: Drop the alias; let the driver own `generation` end-to-end; flush damage notes before pushing the packet they preceded.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_pes_full` (tail; spans 497-619, part1 owns 497-607) | 608-619 | REAL | S2 | payload_len subtraction lacks MPEG-1 guard (F-001); MPEG-2 sibling guard at 532-536 proves the omission is an oversight, not a choice |
| `scan_streams` | 624-714 | REAL | — | Monotone walk verified incl. underflow self-correction; damage-noted skips honest; pre-scan = full-file header pass at open (documented two-pass design, 18-26) |
| `length_of` | 718-725 | REAL | — | 2-byte u16 read at pos+4; typed truncation; lossless u64 |
| `note_stamps_for` | 728-745 | REAL | S2 | F-002: first-Private1-wins stamp merge; no substream param |
| `register_key` | 749-760 | REAL | — | Idempotent first-appearance registration; correct |
| `probe_video_codec` | 766-785 | REAL | S3 | Sequence-header/B5-extension sniff correct; false re-classification comment (F-004); read-Err fallback "mpeg2video" |
| `probe_audio_codec` | 789-806 | REAL | — | Layer-bit mapping verified against the MPEG audio header layout; conservative fallbacks |
| `resync_from` | 811-832 | REAL | S3 | Bounded window + single allocation; off-by-one at window edge (F-003) |
| `stream_index_for` | 835-845 | REAL | — | Exact Private1(kind,sub) matching — the very precision `note_stamps_for` lacks |
| `build_tracks` | 849-877 | REAL | — | honest `language: None`; span guard `last >= first`; `unwrap_or(u32::MAX)` handle can't fire realistically |
| `emit_pes_packet` | 881-921 | REAL | S4 | want-guard before pool fill correct; F-007 hygiene; empty/short payloads damage-noted not errored |
| `Demuxer::start` | 925-959 | REAL | — | Typed head refusal; idempotent replay; pre-scan damage surfaced pre-packets; sticky wiring (only here — F-006) |
| `Demuxer::next_event` | 961-1114 | REAL | S2/S3/S4 | Event machine sound (EOS/EOF/0xB9 flush paths); F-001 fatal propagation, F-005 keyframe constant, F-006 raw `?`, F-007(c) ordering |
| `Demuxer::seek` | 1116-1137 | REAL | — | Pure-query half per trait contract (plugin-api demux.rs:36-39,97); clamps into SCR extent; does not move walk — correct |
| `Demuxer::seekability` | 1139-1141 | REAL | — | `SlowLinear` — honest (no index in PS) |
| `Demuxer::reanchor` | 1143-1218 | REAL | — | Compute-then-commit (D4): walk moved only after scan; `eos=false`, `next_discontinuity=true`, `pending.clear()`; O(file) linear scan documented; unbounded-PES aborts scan conservatively |

## 5. Test honesty (tests in crates/demux/src/ps/tests.rs + tests/ps_fixtures.rs, all baseline-ok)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit` | YES | Bit-exact payload pinning against ffmpeg's own extraction — any strip/length arithmetic change on valid fixtures flips it |
| `lpcm_packets_strip_the_full_dvd_prefix` | YES | Pins the 7-byte LPCM strip on a real VOB fixture |
| `mpeg1_no_stamp_packet_carries_the_single_0x0f_byte` / `mpeg1_stamped_pes_roundtrips_pts` | YES | Synthetic MPEG-1 packets pin the at/header_len arithmetic for the cases they construct — but never `declared_len < header_len` (F-001 gap) |
| `damaged_bytes_resync_within_the_window` | YES (for mid-window codes) | 4 garbage bytes + both PES surviving; cannot catch the window-edge off-by-one (F-003) |
| `zero_pes_length_is_damage_noted_never_an_unbounded_read` | YES | Pins declared_len==0 damage note + no unbounded read |
| `seek_clamps_into_the_scr_extent_and_reanchor_marks_discontinuity` | YES | Asserts reanchor rewinds to first pack and 2 packets re-drain — real behavior pin |
| `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` | PARTIAL | Biased LCG flips in the first 512 bytes of 2 fixtures, 120 rounds — would catch a panic it happens to generate; demonstrably never generated the F-001 underflow pattern (it passed) |
| `fuzz_lite_clean_fixtures_drain_without_terminating_errors` | YES | Clean fixtures must drain with `terminal: None` |
| `containers::ps::vob_lanes_demux_and_refuse_undecodable_codecs_typed` (CLI) | YES | End-to-end through the wired container: demux + typed decode refusal for ac3/dts/lpcm lanes |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "MPEG program stream / VOB … implemented" (win/linux/macos) | docs/feature-matrix.toml:181-190 (`container-mpegps`) | REAL-UNPROVEN (open path) | Wiring exists (phase5.rs:183-190 → containers/mod.rs:71) and demux core executed via 28 unit + 4 fixture + 3 corpus + 4 CLI tests (baseline :1833-1860, :2176-2184, :1142-1150, :2830-2833) — but no baseline run ever exercised `lumen open` on a PS/VOB; the row's own evidence string admits "fixture-only evidence — ZERO ps/vob corpus lanes" (:190) |
| "private streams (AC-3/DTS/LPCM) exposed … honest descriptors; decode pending" | feature-matrix.toml:184 + module docs 47-54 | REAL | register_key + descriptor() (688-692); CLI test `vob_lanes_demux_and_refuse_undecodable_codecs_typed` ok (baseline :2833) |
| "recovery scans are window-bounded" | module docs :68 | REAL (with F-003 edge defect) | resync_from 811-832 clamps to MAX_RECOVERY_SCAN_BYTES |
| "damage-noted skip, never an unbounded read" for PES length 0 | module docs :74-76 | REAL (with F-001 caveat) | 505-517 + test :1860 ok — but the F-001 malformed-header case breaks the same promise |

## 7. Dead / duplicated / unreachable code
- `StickyShape::mint`/`replay` reachable only via `start`-initiated failures (F-006) — half-wired mechanism.
- `generation: GenerationId::FIRST` (917) — overwritten by the pipeline driver (pipeline.rs:2148); dead mint.
- `stream_descriptors`/`current_scr`/`scr_extent`/`is_mpeg2`/`damage_total`/`pool_bytes` accessors (368-403, part1) are consumed by the CLI container (`harvest_facts`, containers/ps.rs:214-263) — live observability, not dead.
- No unreachable code in my range; every match arm in `next_event`/`scan_streams` is exercisable.

## 8. Dependencies & flags
- Depends on `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` — cross-format reuse of the AVI I/O layer (recorded design, module docs 62-68); layering is demux-internal, no violation.
- No feature flags gate any of this code; it compiles in the default build the baseline exercised.
- `MAX_PACKET_BYTES` = 64 KiB ceiling matches the pool bound `PayloadSlots::new(2, …)` (359) and the CLI `HOLD_CAP_BYTES` (containers/ps.rs:115).

## 9. Open questions for the next agent
- F-001's debug-profile panic: one crafted 8-byte MPEG-1 PES (`00 00 01 C0 00 01 FF 0F …`) as a unit fixture settles it — the committed fuzz-lite demonstrably never generated that shape.
- Does any real retail VOB (AC-3 + SPU) exist in the corpus to demonstrate F-002's UNKNOWN span? Baseline corpus has zero PS/VOB lanes (feature-matrix.toml:190).
- The transcode crate's mp4mux (F-005 consumer) — its stss-empty-on-all-non-sync behavior deserves its own finding in whichever wave owns `crates/transcode/src/mp4mux.rs`.
