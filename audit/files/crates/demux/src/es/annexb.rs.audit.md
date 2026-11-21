# AUDIT: crates/demux/src/es/annexb.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 539 / 24 |
| File verdict | SHALLOW (H.264 lane REAL and corpus-pinned; HEVC AU-boundary rule broken on spec-valid streams — see F-001) |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/src/demux_host.rs:458 `containers::open` → apps/cli/src/containers/mod.rs:69-70 `"hevc"/"h264" => es::open_annexb` → containers/es.rs:73 `AnnexbVideoDemuxer::new`) |
| Findings | S0: 1 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg `AnnexbVideoDemuxer` (callers: containers/es.rs; tests es/tests.rs, tests/es_corpus.rs) · rg `hevc|corpus_hevc` in tests/es_corpus.rs (HEVC corpus lane exists, `0044-raw-video.265`, 56-packet pin) · ls /tmp/my-project/corpus-v4/media (ABSENT in this sandbox — corpus NAL layout not inspectable) · sed -n verification of every quote · baseline log cites below |

## 1. One-paragraph verdict
The H.264 half of this demuxer is real: the `first_mb_in_slice` probe reads the correct byte, the prefix-class rule matches ffmpeg's grouping, and the corpus differential pins 26 packets with exact byte sizes (baseline-executed, corpus-presence ambiguity noted). The HEVC half is broken: `starts_new_picture` reads `head[1] & 0x80` where `head` is only the 2-byte NAL header — per H.265 §7.3.1.2 that bit is `nuh_layer_id`'s MSB (zero on every base-layer stream ever produced), while `first_slice_segment_in_pic_flag` (§7.3.2.8) is bit 7 of the byte AFTER the header, which `classify_len(Hevc)=2` never reads. On a spec-valid `.265` file (parameter sets once at head, then consecutive pictures), VCL-driven AU boundaries never fire: the whole file demuxes as one packet, or refuses typed `au_too_large` past 16 MiB. The module doc claims the rule is "pinned empirically against the corpus differential, not guessed", and the in-crate test locks the bug in with fixtures that are themselves invalid HEVC (layer_id 16, temporal_id_plus1 0). The walk engine underneath (windowed scan, overlap carry, zero-run walk-back, floor clamps) is sound — I traced every index path and found no panic or infinite loop.

## 2. What actually works (verified)
- H.264 lane executed: baseline `orchrun-test-workspace-nofailfast.log.txt:1686-1687` — `test es::tests::h264_aus_follow_the_nal_type_rule ... ok`, `h264_no_sync_is_typed_unsupported_after_scan ... ok`; corpus lane `test corpus_h264_lane_matches_ffprobe_packet_geometry ... ok` (log line 2022; asserts 26 packets, per-packet ffprobe sizes, `pts == Missing`) — REAL-UNPROVEN only insofar as the corpus-absent skip makes "ok" ambiguous.
- Typed construction refusals: `empty_source_refuses_at_construction` ok (log line 1681) — SourceReader::new refuses unknown-length/unseekable/empty (riff.rs:227-235).
- Windowed scan walk: hand-traced `next_boundary` (lines 248-309) — overlap carry can neither underflow (`win_end < len` implies window > WINDOW ≥ OVERLAP) nor loop forever (pos strictly advances; `last_code` dedup); walk-back index `at - origin - 1` provably ≥ 0 (guarded by `at > lo ≥ origin`).
- Memory bounds: `PayloadSlots::new(2, …)` is lazy (riff.rs:542-549 `arenas: Vec::new()`), so no eager 32 MiB; `emit_au` refuses `size > MAX_AU_BYTES` typed (line 450-452).

## 3. Findings

### F-001 [S0]
Where: crates/demux/src/es/annexb.rs:141-162
Code:
```
    /// Whether a VCL NAL header opens a new picture: H.264
    /// `first_mb_in_slice == 0` (the first slice-header payload bit —
    /// `ue(0)` is a lone `1`); H.265
    /// `first_slice_segment_in_pic_flag` (bit 15 of the 2-byte NAL
    /// header). Both bits are read from `head[1]` — H.264's classify
    /// window extends one payload byte past the 1-byte header, and a
    /// VCL header byte is never `0x00` in either codec, so emulation
    /// prevention cannot have altered the bit.
    #[must_use]
    fn starts_new_picture(self, head: &[u8]) -> bool {
        head.len() > 1 && head[1] & 0x80 != 0
    }
```
```
    fn classify_len(self) -> usize {
        match self {
            AnnexbCodec::H264 => 2,
            AnnexbCodec::Hevc => 2,
        }
    }
```
What is wrong: For HEVC, `head` is the 2-byte NAL unit header (`classify_len(Hevc) = 2`). Per ITU-T H.265 §7.3.1.2 those 16 bits are `forbidden_zero_bit(1) | nal_unit_type(6) | nuh_layer_id(6) | nuh_temporal_id_plus1(3)`; `head[1] & 0x80` is therefore the MSB of `nuh_layer_id` — 0 on every base-layer stream (layer_id 0 ⇒ head[1] ≤ 0x07). `first_slice_segment_in_pic_flag` is the FIRST bit of the slice-segment header (§7.3.2.8), i.e. bit 7 of the byte at NAL offset 2 — a byte this code never reads. The comment's "bit 15 of the 2-byte NAL header" is a spec misreading baked into both the code and the tests.
Why it matters: On a spec-valid, ordinary `.265` file — VPS/SPS/PPS once at the head, then consecutive pictures with no AUD/param-set/SEI NALs between them — `starts_new_picture` is always false, so the VCL boundary rule never fires. Every picture glues into one access unit: the demuxer emits ONE packet for the entire file, or, past `MAX_AU_BYTES` (16 MiB), refuses a valid file with the typed `au_too_large` error (line 450-452). The module doc (lines 10-12) claims the rule is "the rule ffmpeg's own raw-ES demux path exercises, pinned empirically against the corpus differential, not guessed", and apps/cli/src/containers/es.rs:22-24 claims "packet-exact demux of the hw-path codecs" — both are false for HEVC. The corpus pin cannot catch it: the only HEVC corpus file (`0044-raw-video.265`, 56-packet assertion) is an all-IRAP stream whose first-packet size (19745 vs ~14k for the rest) indicates parameter-set groups before each picture, so the prefix-class rule alone produces the 56 packets — the VCL rule is never exercised on a spec-valid shape. The in-crate test is worse than useless — it locks the bug in:
```
    // 2-byte HEVC headers: (type << 1) | 0, flags byte with the
    // first_slice_segment_in_pic_flag in bit 7.
    push_nal(&mut stream, &[0x26, 0x80, 0x55]); // IRAP_W_RADL (19): AU 1
```
(es/tests.rs:110-116) — `0x80` in the second NAL-header byte means layer_id 16 and temporal_id_plus1 0 (forbidden; §7.3.1.2 requires tid+1 ≥ 1), and the actual slice-header byte `0x55` has bit 7 = 0 (a second segment!). A spec-correct implementation would FAIL this test. This is the "test proving nothing used as evidence" pattern.
What correct looks like: `classify_len(Hevc) = 3`; `starts_new_picture` reads `head[2] & 0x80` for HEVC (and `head[1] & 0x80` for H.264 as today); regenerate the HEVC test fixtures with layer_id 0, temporal_id_plus1 1, and the first-slice flag in the first payload byte.
Confidence: HIGH on the bit layout (spec §7.3.1.2/§7.3.2.8 are unambiguous; every real base-layer HEVC encoder emits head[1] ≤ 0x07). MEDIUM on the corpus-file shape explanation (the corpus directory does not exist in this sandbox — see REQUEST-ORCH-RUN). Verification: read + spec trace; not executed (no cargo allowed).

### F-002 [S4]
Where: crates/demux/src/es/annexb.rs:390-395
Code:
```
    /// Total bytes logged as damage (always 0 today — Annex-B damage
    /// is not byte-attributable; kept for parity with the siblings).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }
```
What is wrong: Dead public API that can only ever return 0: `DamageLog::note` is never called anywhere in this module (rg `damage.note` in annexb.rs → no hits), no test or caller invokes `damage_total()` on this type, and the doc comment admits it.
Why it matters: Sibling-parity copy-paste surface asserting observability that does not exist.
What correct looks like: Delete both the accessor and the `damage` field (the struct is private state), or actually attribute skipped-garbage bytes.
Confidence: HIGH — Verification: rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `MAX_AU_BYTES`/`WINDOW`/`OVERLAP`/`MAX_OPEN_SCAN_BYTES` | 74-84 | REAL | — | OVERLAP 32 ≥ start code 4 + header 2 + slack — no straddle can be missed (traced) |
| `AnnexbCodec::header_len`/`nal_type` | 98-112 | REAL | — | H.264 `&0x1F`, HEVC `>>1 &0x3F` — correct per both specs |
| `AnnexbCodec::is_vcl` | 116-121 | REAL | — | H.264 1..=5, HEVC ≤31 — correct |
| `AnnexbCodec::is_irap` | 125-130 | REAL | — | H.264 IDR 5, HEVC IRAP band 16..=23 — correct |
| `AnnexbCodec::is_prefix_class` | 134-139 | REAL | — | H.264 {6..11,13..15}, HEVC {32..37,39}; filler (12/38) and suffix-SEI (40) correctly excluded — matches doc lines 26-29 |
| `AnnexbCodec::starts_new_picture` | 141-152 | DEFECTIVE | S0 | F-001 — correct for H.264 (head[1] IS the first slice byte), wrong bit for HEVC |
| `AnnexbCodec::classify_len` | 154-162 | DEFECTIVE | S0 | F-001 — HEVC needs 3 bytes, reads 2 |
| `AnnexbCodec::descriptor`/`format` | 165-182 | REAL | — | Static ASCII names; unreachable error arm documented |
| `probe` (free fn) | 189-208 | REAL | — | Bounded plausibility scan (H.264 1..=9, HEVC ≤34); used by start + media-probe lane |
| `AuScan::new` | 230-240 | REAL | — | Trivial |
| `AuScan::next_boundary` | 248-309 | REAL | — | Walk engine sound (see §2); emits `(at, closed_irap)` with correct IRAP handoff via `mem::replace`; the DECISION input is F-001-poisoned for HEVC only |
| `StickyShape::of`/`mint` | 321-331 | REAL | — | Replayable error shape |
| `AnnexbVideoDemuxer::new` | 364-382 | REAL | — | Typed refusals delegated; pool lazy |
| `aus_demuxed` | 386-388 | REAL | — | Corpus/test-used |
| `damage_total` | 393-395 | DEAD | S4 | F-002 |
| `fail` | 397-400 | REAL | — | Sets sticky |
| `AnnexbVideoDemuxer::probe` | 404-414 | REAL | — | 64 KiB open scan, typed no-sync (tested, baseline ok) |
| `mint_track` | 417-441 | REAL | — | 1/25 base + both stamps Missing — the documented timing honesty, asserted by corpus test |
| `emit_au` | 444-470 | REAL | — | Size guard → typed ResourceLimit; pool fill; keyframe = IRAP-presence (documented ffmpeg recovery-point divergence) |
| `take_pending` | 476-478 | REAL | — | Single-packet queue |
| `Demuxer::start` | 482-493 | REAL | — | Probe → track; sticky replay on second call (tested) |
| `Demuxer::next_event` | 495-526 | REAL | — | Final partial AU emitted at EOF (documented ffmpeg divergence); EOS latch correct |
| `Demuxer::seek` | 528-538 | REAL | — | Honest typed refusal — raw Annex-B has no timestamps to map onto (unlike mp3 CBR, this one is justified) |

## 5. Test honesty (es/tests.rs + tests/es_corpus.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `h264_aus_follow_the_nal_type_rule` (es/tests.rs:72-104) | YES | 4 AUs from 10 NALs incl. multi-slice AU, mid-SPS regroup, contiguous byte coverage, keyframe flags, Missing stamps. Break `first_mb` probe → count/size asserts fail. Baseline ok (log 1686). |
| `hevc_aus_group_on_first_slice_flag_and_vps` (es/tests.rs:107-136) | NO — IT ENCODES THE BUG | Fixtures put `0x80` in the NAL header's second byte (layer_id 16 / tid+1 0 — invalid HEVC) instead of the slice-header byte; asserts 3 AUs on that invalid shape. A spec-correct implementation fails this test. Baseline ok (log 1687 region) — proves nothing about real HEVC. |
| `h264_no_sync_is_typed_unsupported_after_scan` (es/tests.rs:139-157) | YES | 512×0xA5 → typed no-sync + sticky. ok (log 1687). |
| `empty_source_refuses_at_construction` (160-172) | YES | Both annexb and mp3 constructors refuse empty. ok (log 1681). |
| `next_event_before_start_is_typed` (174-183) | YES | Contract order. ok (log 1685). |
| `fuzz_lite_mutations_never_panic_or_hang` (307ff) | WEAK-MED | No-panic/no-hang on one LCG seed sequence. ok (log 1683). |
| `corpus_h264_lane_matches_ffprobe_packet_geometry` (es_corpus.rs:98-137) | YES-if-corpus-present | 26 packets, exact per-packet ffprobe sizes, byte-truth of packet 2's first 5 bytes. ok (log 2022); skip-when-absent ambiguity. |
| `corpus_hevc_lane_matches_ffprobe_packet_geometry` (es_corpus.rs:139-168) | NO for the VCL rule | 56 packets on an all-IRAP file with parameter-set groups (first packet 19745 B vs ~14k rest) — the prefix-class rule alone explains the count; the broken `first_slice` bit is never exercised. ok (log 2024); cannot detect F-001. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NAL-type-aware boundary rule — the rule ffmpeg's own raw-ES demux path exercises, pinned empirically against the corpus differential, not guessed" | annexb.rs:10-12 | FALSE for HEVC | F-001: the HEVC half reads the wrong bit; the corpus pin covers only a shape where the prefix rule suffices |
| "HEVC `first_slice_segment_in_pic_flag` (bit 15 of the 2-byte NAL header)" | annexb.rs:23-24 | FALSE | §7.3.2.8: the flag is bit 7 of the byte AFTER the 2-byte header; "bit 15" is `nuh_layer_id`'s MSB |
| "Packets therefore carry `Stamp::Missing` pts/dts … the track time base is minted at ffmpeg's documented raw-video fallback (1/25)" | annexb.rs:31-40 | REAL | mint_track:421-437; corpus asserts `pts == Missing` (es_corpus.rs:126) |
| "only NAL headers are classified (a multi-megabyte NAL body is skipped over without reading it)" | annexb.rs:44-48 | REAL | classify_len ≤ 2; window reads are bounded by WINDOW |
| "a larger access unit is a typed `ResourceLimit` refusal, never an unbounded allocation" | annexb.rs:49-51 | REAL | emit_au:450-452; pool slot_cap = MAX_AU_BYTES |
| "the final partial access unit is emitted (ffmpeg's behavior)" | annexb.rs:55-60 | REAL | next_event:514-519; corpus asserts contiguous whole-file coverage |
| "packet-exact demux of the hw-path codecs with no silent wrong decode" (HEVC lane) | apps/cli/src/containers/es.rs:22-24 | FALSE for HEVC | F-001 — one packet for a whole typical file is not packet-exact |

## 7. Dead / duplicated / unreachable code
- `damage_total()` + `damage` field — dead, always 0 (F-002).
- `StickyShape` duplicates the WAV/Ogg/ADTS sticky precedent by design (documented).
- No other dead code found; every method is called.

## 8. Dependencies & flags
No feature flags, no unsafe. Reuses `avi::riff::{PayloadSlots, SourceReader}` and `avi::header::DamageLog` (cross-family layering, same as mp3.rs). No tests are cfg-gated; the corpus suite needs `input` (dev-dependency) and an optional corpus dir.

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN: `python3 -c` script over `/tmp/my-project/corpus-v4/media/0044-raw-video.265` printing, for the first ~20 and a mid-file sample of Annex-B NALs, `(start_offset, head[0], head[1], head[2])` — to confirm head[1] ≤ 0x07 (base layer) and per-picture VPS/SPS/PPS groups, i.e. that the corpus pass never exercises the VCL rule. Exact command logged in W04-A05.log.md.
- containers/es.rs:22-31 also claims "H.264/HEVC have no software decoder in this tree" — refuted for H.264 by baseline INDEX.md CORRECTION v2 (sw.rs exists, 25,964 bytes). The containers/es.rs auditor should reconcile.
- If F-001 is fixed, `hevc_aus_group_on_first_slice_flag_and_vps` and the HEVC corpus expectations must be regenerated — flag to the fix wave.
