# AUDIT: crates/codecs/src/audio/adpcm/mod.rs#part2 (lines 687-1386)
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 700 (of 1386; chunk part2 of 2 — boundary: part1/W07-A03 owns 1-686 through `Decoder::flush`; my range opens at the `Stats` ledger doc, line 686/687) / 19 units (9 production: `Stats`, `parse_raw_facts`, `wav_out_bound`, `raw_out_bound`, `duration_nanos`, `pooled_slot`, `AdpcmError::into_engine_error`, `Detail::fmt`+`Error`, `unsupported`; 7 test helpers + 12 tests in `mod tests`) |
| File verdict | REAL (implemented, reachable, executed — baseline unit + differential + CLI open-path lanes) |
| Reachable from shipped entry point? | YES for the WAV pair, NO for the breadth five. `lumen open x.wav` → `apps/cli/src/containers/wav.rs` → demux `wav/format.rs:207-208` mints `adpcm_ima_wav`/`adpcm_ms` (ASF: `asf/header.rs:406-409`) → `apps/cli/src/pipeline_host.rs:296-300` `codec if codec.starts_with("adpcm_")` → `AdpcmDecoder`. **Correction after yamaha.rs audit (grep `adpcm_yamaha|adpcm_ima_oki|adpcm_swf|adpcm_ima_qt|adpcm_ima_ws` over crates/demux + apps → ZERO hits): no demuxer mints any breadth descriptor, so this part's `parse_raw_facts` and the five raw plans run only under tests/differential, never in the shipped binary.** Executed (test lane): baseline `test-workspace.log.txt:330-341` (12 `audio::adpcm::tests::*` ok), `orchrun-...:2838` `containers::wav::tests::adpcm_extra_data_flows_into_the_decoder ... ok`, `:2841` `corpus_wav_lanes_decode_through_the_open_path ... ok`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Read 1-686 (part1 context for callers of my helpers), 686-1386 (my range, full); rg function map (`rg -n "^(pub )?(fn|struct...)"`); rg `adpcm` over crates/demux, apps/cli (descriptor minters → reachability); rg baseline logs (`test-workspace.log.txt:292-341`, `orchrun-...:781,2838-2843`); rg `push|extend` in adpcm flavor modules (intermediate-Vec proof); Read media-types/src/packet.rs:63-216 (SharedBytes Arc semantics); sed verification of every quoted range. |

## 1. One-paragraph verdict
Part2 is the plumbing tail of an otherwise honest in-house ADPCM decoder: correct raw-facts skeleton parsing with typed refusals, worst-case output bounds I re-derived by hand for every flavor geometry, a clean nanosecond duration helper, and an Arc-based pooled-slot rotation. The two real defects are both about the pool: the "allocation-free per packet" claim is false because `decode`'s fill closure (part1 lines 580-639) builds an intermediate `Vec` via `extend_from_slice` growth on every packet before copying into the pooled slot — the rotation eliminates only the *second* allocation — and the two tests named "pooled_rotation_is_reused_across_packets" would still pass if the rotation were deleted outright. Execution evidence is strong: bit-exact ffmpeg differentials for all seven flavors, 12 unit tests, fuzz survival, and the CLI open-path lane all ran green in baseline.

## 2. What actually works (verified)
— All seven flavors bit-exact vs ffmpeg 7.1.5: baseline `test-workspace.log.txt:822-837` — `adpcm_breadth_differential::breadth_corpus_is_bit_exact_against_ffmpeg_reference ... ok`, `adpcm_corpus_differential::real_corpus_decodes_bit_exact ... ok` + `committed_fixtures_decode_bit_exact ... ok`; `adpcm_fuzz_survival` 4/4 ok (mutated fmt headers and blocks → typed, never panic — exercising my `parse_raw_facts`/error plumbing).
— My range's 12 unit tests executed and passed: `test-workspace.log.txt:330-341`, including `yamaha_packets_decode_with_persistent_state` (asserts cross-packet state continuity by value, line 1282) and `ms_packet_decodes_stereo_interleaved` (asserts seed sample values `[50,-50,100,-100]`, line 1233).
— `wav_out_bound` re-derived by me: tightest geometry (mono MS, block_align=8 → 8 output bytes/block) vs bound 8·4+4+2=38/block — safe by ≥3.4×; stereo MS 4·ba−32 ≤ 4·ba+4+4; IMA mono 4·ba−14 ≤ 4·ba+6. `raw_out_bound` (8×payload+64) covers the expansive SWF-stereo-width-2 shape (exactly 8×/byte, headers displace bits → strictly less) and every other flavor (≤4×; IMA4 128/34≈3.76×). Saturating arithmetic throughout — no overflow path.
— `duration_nanos` (765-770): u128 intermediate, rate==0 guard; correct ns math for the `Duration::Known(Nanoseconds(...))` frame field.
— `pooled_slot` Arc discipline: `get_mut` (Arc::get_mut) correctly refuses shared slots (media-types/src/packet.rs:213-216), `slice(0..used)` is a zero-copy view (packet.rs:170-178), slot eviction bounded to `OUT_SLOTS=4`.
— CLI end-to-end lane through my helpers: `adpcm_extra_data_flows_into_the_decoder ... ok`, `corpus_wav_lanes_decode_through_the_open_path ... ok` (orchrun log :2838, :2841).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/audio/adpcm/mod.rs:115-117 (claim) + 580-639 (violating caller; part1 range, cross-boundary)
Code:
```rust
/// Pooled output slots (the PCM `CONVERT_SLOTS` precedent): the
/// rotation reuses a freed slot via `get_mut`, so steady-state decode
/// is allocation-free per packet.
```
and (caller, lines 580-581 + 637-638):
```rust
        let samples = pooled_slot(&mut pool, &mut slot_next, bound, |bytes| {
            let mut buf = Vec::new();
```
```rust
            bytes[..buf.len()].copy_from_slice(&buf);
            buf.len()
```
What is wrong: the fill closure passed to `pooled_slot` decodes into a fresh intermediate `Vec<u8>` that grows by `extend_from_slice`/`push` (verified: `ima.rs:203-204` `push_sample` → `out.extend_from_slice(&sample.to_ne_bytes())`; same shape in ms/oki/yamaha/misc/swf), then copies into the pooled slot. Every packet therefore performs ≥1 heap allocation (Vec growth: 0→4→8→… elements) plus a full memcpy, even when the rotation hits.
Why it matters: the stated invariant ("allocation-free per packet") is false; the pool halves the cost instead of eliminating it, and nothing tests the claim — the repo's zero-alloc lane covers MJPEG only (`crates/codecs/tests/zero_alloc_decode.rs` single test `mjpeg_contract_path_allocates_nothing_per_frame_after_warm_up`, baseline :1296-1300) while PCM has `pcm_zero_alloc.rs` (:1247-1250); ADPCM has none.
What correct looks like: decode directly into the `&mut [u8]` slot (flavor functions take `&mut [u8]` + return written length), or at minimum strike the "allocation-free" claim and add an ADPCM zero-alloc test alongside `pcm_zero_alloc.rs`.
Confidence: HIGH — Verification: read (caller + all five flavor modules' push sites) + baseline log cited; not executed with an allocator probe.

### F-002 [S3]
Where: crates/codecs/src/audio/adpcm/mod.rs:1346-1385
Code:
```rust
    fn wav_pooled_rotation_is_reused_across_packets() {
        ...
        for i in 0..OUT_SLOTS * 3 {
            let outcome =
                match decoder.decode(packet(generation, block.clone()), &mut Capture::default()) {
```
```rust
        // The rotation stays bounded.
        assert!(decoder.slots.len() <= OUT_SLOTS);
```
What is wrong: both "pooled_rotation_is_reused_across_packets" tests assert only (a) each decode returns `Emitted { frames: 1 }` and (b) `decoder.slots.len() <= OUT_SLOTS`. If `pooled_slot` were replaced by `SharedBytes::from_vec` per packet (rotation deleted), `slots.len()` stays 0 ≤ 4 and every assertion still passes. The tests verify the pool exists at all only in the negative (they would catch unbounded slot growth, nothing else).
Why it matters: the one perf property the pool exists for (reuse) has no regression guard; F-001's false claim ships precisely because these tests cannot see allocations.
What correct looks like: assert reuse directly — e.g. after warm-up, `Arc::as_ptr` identity of the returned sample buffer's allocation across packets (the PCM `convert_pool` precedent), or an allocation-counting sink like `zero_alloc_decode.rs`.
Confidence: HIGH — Verification: read + mutation reasoning (delete rotation → tests pass unchanged).

### F-003 [S4]
Where: crates/codecs/src/audio/adpcm/mod.rs:637 (panic site, part1 range) guarded only by part2's bounds at 743-761
Code:
```rust
            bytes[..buf.len()].copy_from_slice(&buf);
            buf.len()
```
What is wrong: if any flavor decoder ever emits more than the computed worst-case bound, this is a panic (slice range out of order), and `SharedBytes::slice`'s `assert!` (media-types/src/packet.rs:171-175) is the second panic in the same chain. The crate convention is "No `unsafe`, no panics on any input" (crates/codecs/src/lib.rs:52-54).
Why it matters: the bound functions are the only guard between a future flavor-geometry change and a decode-stage panic on valid input; there is no debug assertion or truncation defense at the seam.
What correct looks like: `let used = buf.len().min(bytes.len());` + a debug_assert, or have flavor decoders write into the slot directly so bounds are structural.
Confidence: HIGH — Verification: read; current bounds proven safe by my re-derivation (§2), so this is latent, not live.

### F-004 [S4]
Where: crates/codecs/src/audio/adpcm/mod.rs:748-750
Code:
```rust
    blocks
        .saturating_mul(block_align.saturating_mul(4).saturating_add(4))
        .saturating_add(usize::try_from(channels.max(1)).unwrap_or(1) * 2 * blocks)
```
What is wrong: the seed term `channels * 2 * blocks` counts seed *samples* while the rest of the expression is in *bytes* (seeds cost `channels * 2 * 2 * blocks` bytes). The undercount is harmless because the `4·block_align + 4` per-block term already dominates every seed case (verified §2), but the unit mix makes the formula's safety argument harder than it should be.
Why it matters: maintenance hazard — anyone "tightening" the main term per the comment ("the exact worst case is 4 output bytes per input byte") could strip the slack that makes the seed undercount safe.
What correct looks like: `channels * 4 * blocks` with a unit comment, or drop the seed term and note it is dominated.
Confidence: HIGH — Verification: read + arithmetic re-derivation.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Stats` (struct) | 687-691 | REAL | — | trivial damage ledger; Copy/Default correct |
| `parse_raw_facts` | 698-737 | REAL | — | 16-byte floor, tag==0, 1..=2 channels, rate≠0 — all typed `unsupported`; LE field offsets match WAVEFORMATEX; exercised by `missing_facts_and_skeleton_damage_are_typed_unsupported` (5 refusal codes, baseline :333) |
| `wav_out_bound` | 743-751 | REAL | S4 | safe bound (re-derived); seed-term unit mix F-004; saturating |
| `raw_out_bound` | 759-761 | REAL | — | 8×+64 covers SWF-stereo expansive shape; saturating |
| `duration_nanos` | 765-770 | REAL | — | u128 intermediate, rate-0 guard; call-site `as u64` truncation unreachable for sane frames·ns |
| `pooled_slot` | 776-811 | REAL | S3 (via F-001) | Arc-correct rotation; exact-length matching means >4 distinct payload lengths → cold alloc per packet (CBR ADPCM unaffected — final short block only); `None => 0` branch at :799 dead (fresh Arc always unique) |
| `AdpcmError` (enum) | 818-833 | REAL | — | two-variant taxonomy; matches module docs |
| `AdpcmError::into_engine_error` | 835-860 | REAL | — | stage `Decode`, `Unsupported`/`Decoder{BadState}` mapping with context codes + `Detail` source; asserted by 6 unit tests |
| `Detail` + `Display` + `Error` | 863-872 | REAL | — | chain-root carrier; trivial |
| `unsupported` | 875-880 | REAL | — | constructor; trivial |
| `tests::ima_fmt` | 890-901 | REAL | — | helper; real WAVEFORMATEX layout |
| `tests::ms_fmt` | 905-921 | REAL | — | helper incl. standard 7-pair table |
| `tests::raw_facts` | 925-934 | REAL | — | helper; tag-0 skeleton |
| `tests::configured` | 936-951 | REAL | — | helper; wraps configure |
| `tests::Capture` (+`FrameSink`) | 954-975 | REAL | — | collecting sink; refuses non-audio typed |
| `tests::packet` / `empty_packet` | 977-992 | REAL | — | helpers; `Stamp::Missing` pts |
| `gate_accepts_exactly_the_seven_flavors` | 995-1017 | REAL | — | would fail on accepts() regression; asserts refusal code too |
| `lettered_oki_is_the_honest_typed_gap` | 1020-1031 | REAL | — | asserts code + `ErrorKind::Unsupported` |
| `missing_facts_and_zero_block_align_are_typed_unsupported` | 1034-1051 | REAL | — | two codes |
| `missing_facts_and_skeleton_damage_are_typed_unsupported` | 1054-1101 | REAL | — | five codes incl. unknown descriptor |
| `tag_mismatch_and_coef_damage_are_typed_unsupported` | 1104-1133 | REAL | — | three codes; byte-offset surgery on fmt |
| `unconfigured_decode_is_typed_bad_state` | 1136-1143 | REAL | — | BadState path |
| `ima_packet_decodes_and_recovers_typed` | 1146-1193 | REAL | — | decode + damage + stale-generation drop; count-level only (values pinned in differential suites) |
| `ms_packet_decodes_stereo_interleaved` | 1196-1234 | REAL | — | value assertion on seed order |
| `yamaha_packets_decode_with_persistent_state` | 1237-1296 | REAL | — | cross-packet state by value; empty/stale handling |
| `swf_and_qt_packet_damage_recovers_typed` | 1299-1344 | REAL | — | skip + full-chunk counts |
| `wav_pooled_rotation_is_reused_across_packets` | 1347-1367 | REAL (weak) | S3 | F-002: cannot detect removal of the rotation |
| `raw_pooled_rotation_is_reused_across_packets` | 1370-1385 | REAL (weak) | S3 | same |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| gate/typed-gap/facts/tag/coef tests (7) | YES | assert exact `err.context.code` strings and kinds against real parse paths |
| `unconfigured_decode_is_typed_bad_state` | YES | exercises `BadState` → `adpcm.not_configured` |
| `ima_packet_decodes_and_recovers_typed` | YES | sample-count (33·2), format fields, Recovered{skipped:1}, stale drop — but NOT decoded values (values covered by differential suites) |
| `ms_packet_decodes_stereo_interleaved` | YES | asserts seed sample values and interleave order |
| `yamaha_packets_decode_with_persistent_state` | YES | asserts `first[0] != second[0]` — breaks if state resets per packet |
| `swf_and_qt_packet_damage_recovers_typed` | YES | skip-vs-emit + 64-sample IMA4 count |
| pooled_rotation tests (2) | NO (for their named subject) | F-002: only `slots.len() <= 4` + outcomes; rotation deletion passes |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exactly against the ffmpeg 7.1 reference on the real corpus — the differential tables live in tests/adpcm_corpus_differential.rs … adpcm_breadth_differential.rs" | mod.rs:33-37 (module docs) | REAL | baseline :822-837 — both suites ok incl. `breadth_corpus_is_bit_exact_against_ffmpeg_reference`, `real_corpus_decodes_bit_exact` |
| "steady-state decode is allocation-free per packet" (OUT_SLOTS) | mod.rs:115-117 | FALSE (as a property of `decode`; true only of the rotation itself) | F-001: intermediate `Vec::new()` + `extend_from_slice` growth at :581/:637 callers; no zero-alloc test for adpcm exists |
| "Pooled output slots (the PCM CONVERT_SLOTS precedent)" | mod.rs:115 | REAL-UNPROVEN | PCM precedent exists (pcm_zero_alloc.rs :1247-1250 ok) but the adpcm side has no equivalent probe — not executed |

## 7. Dead / duplicated / unreachable code
- `pooled_slot` :797-800 `None => 0` arm is dead (a freshly created `SharedBytes` always has a unique Arc) — harmless, S4-grade, folded into F-001 discussion; not separately counted.
- No unreachable production code in part2: every helper is called from part1's `configure`/`decode` (verified by reading 305-676) or the error plumbing at configure.
- Duplication note: `pooled_slot` is a copy of the PCM `pooled_convert_slot` pattern (per its own doc) — accepted, cross-referenced, not counted.

## 8. Dependencies & flags
- No new deps; `adpcm` is unconditional (`codecs/Cargo.toml` — in-house set, no `codec-adpcm` feature), compiled in every shipped build including `--no-default-features`.
- Relies on `media_types::packet::SharedBytes` (Arc semantics verified, packet.rs:63-216) and `codec_api::prelude` — same-crate/workspace, no layering violation.

## 9. Open questions for the next agent
- part1 (W07-A03): the `decode` function's `filled = samples.len() / 2 / channels.max(1)` (line 650) truncates odd trailing sample counts — benign for all current flavor geometries (all emit whole instants); confirm no flavor can emit a partial instant.
- If any future container carries ADPCM with variable packet sizes (raw flavors from e.g. a Sega .adp demuxer), revisit `pooled_slot`'s exact-length matching (see F-001 note).
- The `assert!` in `SharedBytes::slice` (media-types packet.rs:171) is a shared panic seam for every codec using this pattern — a media-types-wave item, not adpcm's.
