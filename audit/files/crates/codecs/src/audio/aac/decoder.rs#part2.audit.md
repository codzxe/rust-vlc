# AUDIT: crates/codecs/src/audio/aac/decoder.rs#part2 (lines 421-894)
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 474 (421-894) / 13 (3 `Decoder` impl methods, `pooled_sample_slot`, 9 tests + 1 helper). Part boundary: part1 (W07-A02) = 1-420 (module docs, gates, `decode_one`, `decode_stream`, struct); part2 (MINE) = 421-894 (`impl Decoder` = the shipped packet path, the output-slot pool, the test module). |
| File verdict | REAL (the shipped AAC lane; executed end-to-end in baseline) |
| Reachable from shipped entry point? | YES. Call chain: `lumen open file.mp4` → demux mp4 (`mp4a`→`"aac"`, crates/demux/src/mp4/mod.rs:449, esds blob → `DecoderConfig::extra_data`) → `apps/cli/src/pipeline_host.rs:288-292` `self.seeded(track, config, AacDecoder::new)` → `configure` (ASC mode, 422-487) → per-packet `decode` (489-593) → `out.accept`. EXECUTED: baseline INDEX.md §CORRECTION v2 — `sample-aac.mp4` (audio-only AAC) "Buffering → **Playing** → Stopped" and the 4:2:0 H.264+AAC MP4 "PLAYS" (audio into the null device). ADTS mode reachable via the raw-ADTS demuxer (`"aac-lc"`, crates/demux/src/adts/mod.rs:246,287) with `extra_data: None` → lazy engine build at 525-529. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "AacDecoder" crates apps` → only pipeline_host.rs:55,291 + apps/web/src/host.rs (both shipped hosts); `rg -n "esds\|extra_data" crates/demux/src/mp4/mod.rs` → ASC seeding; `sed -n` on all quoted ranges; `rg -n "sample_rate\|frame_len" crates/demux/.../adts.rs` (wrong — crates/codecs/src/audio/aac/adts.rs:145-172) → rate index validated 0..=12, frame_len ≥ header_len; no cargo registry in sandbox → symphonia internals UNVERIFIED |

## 1. One-paragraph verdict
This is the half of the AAC module that the shipped binary actually runs, and the baseline proves it works end to end: an MP4's esds AudioSpecificConfig seeds `configure` (ASC mode), packets decode through the pinned symphonia engine, and `sample-aac.mp4` reaches `Playing` (null device — no sound output path exists anywhere, per baseline §CORRECTION v2). The gate discipline is honest — profile/channel/HE refusals are typed with exact codes, `configure` fully resets state, the ADTS per-packet path re-validates every header and refuses parameter switches. The weaknesses: the emitted-frame arithmetic trusts three independently-sourced channel counts to agree without checking (silent stale-sample emission if the engine ever disagrees with the header/ASC — untestable here because the symphonia source is not present in the sandbox); reserved ASC channel configs 8-15 skip the F6 gate and surface as the opaque `aac.engine.init` the F6 fix was filed to eliminate; and the entire packet-emit path (pts/duration/pool) has zero in-tree test coverage — the only configure-level tests never feed a real packet through `decode`, leaving the baseline smoke as the sole execution evidence for the shipped lane.

## 2. What actually works (verified)
— End-to-end AAC-in-MP4 decode: `audit/runs/baseline/INDEX.md` §CORRECTION v2 — `sample-aac.mp4` → "Buffering → **Playing** → Stopped" (audio-only AAC MP4) and sample-420.mp4 (H.264+AAC) "PLAYS"; both exit 0. The lane is `pipeline_host.rs:288-292` → this part's `configure`+`decode`.
— `configure` gate chain: descriptor (424-430), ASC length (438-444), HE/profile (445), channel gate (452-456) — each refusal path asserted by in-tree tests at 652-824 (codes `aac.codec_descriptor`, `aac.asc.length`, `aac.he_sbr`, `aac.profile`, `aac.channel_count`, `aac.channel_config_pce`), and the clean paths (stereo@12kHz, mono@44.1kHz) configure a REAL engine (777-781).
— Baseline run #3 (INDEX.md): `cargo test --workspace` 1681 passed — includes this module's tests.

## 3. Findings

### F-001 [S3] Channel-count truth is triple-sourced and cross-assumed; a disagreement silently emits stale/truncated samples
Where: crates/codecs/src/audio/aac/decoder.rs:546-552
Code:
```rust
        let (frame_samples, channels) = decode_one(engine, payload, &mut self.scratch)?;
        if self.out_channels == 0 {
            self.out_channels = channels as u32;
            self.out_rate = engine.last_decoded().spec().rate();
        }
        let channels = self.out_channels as usize;
        let interleaved = &self.scratch[..frame_samples * channels];
```
What is wrong: `decode_one` (part1:233-278) returns the engine's *per-decode* channel count and sizes `scratch` to `frames × that count` (part1:271-275). `decode` then slices with the *configured* `out_channels` (from the ADTS header at 529, or the first packet's engine spec at 548) with no equality check. If the engine ever decodes a block whose channel elements disagree with the configured count (adversarial or mis-muxed input the engine still decodes): fewer channels → `interleaved` takes a prefix of the previous frame's leftovers (stereo→mono emits 1024 fresh + 1024 stale samples); more channels → `interleaved` takes half the decoded frame. `scratch` never shrinks so the slice itself cannot panic (sizes traced), but the emission is silent garbage — exactly what crates/codecs/src/lib.rs:53-56 forbids ("never a silent partial frame"). Relatedly, `pooled_sample_slot`'s `fill` writes `dst[cursor..cursor+4]` (558-561) into an 8192-byte slot sized for 1024×2 — engine output beyond 2 channels would panic there; today that is guarded only by symphonia's own >2-channel init refusal plus the sniffed gate (configs 1-7), NOT by any check in this file.
Why it matters: the code's own shape (`if out_channels == 0` capture, `channels` return value) admits the engine's count is authoritative per decode, then ignores it for every packet after the first. Whether symphonia 0.6.1 can vary `spec().channels()` across packets could not be verified — no cargo registry in the sandbox — so this is a defensive-gap finding, not a demonstrated failure.
What correct looks like: after `decode_one`, `if channels as u32 != self.out_channels { return Err(corrupt("aac.adts.param_switch", ...)) }` (or update out_channels and re-emit honestly); same for rate.
Confidence: MEDIUM — Verification: read + index arithmetic; symphonia internals UNVERIFIED (no registry source). If symphonia can vary spec per packet, escalate to S2 (silent wrong audio in the shipped lane).

### F-002 [S3] The shipped packet path has zero in-tree test coverage; only the baseline smoke executes it
Where: crates/codecs/src/audio/aac/decoder.rs:647-894 (test module) vs 489-593 (`decode`)
Code:
```rust
    #[test]
    fn decode_before_configure_is_typed_bad_state() {
```
What is wrong: the test module covers the descriptor/ASC/configure gates and one negative decode (BadState, 827-855). No test constructs a real ADTS frame or raw data block and drives `AacDecoder::decode` to an emitted `AudioFrame` — so lines 505-592 (ADTS per-packet header gate, truncated-frame refusal, lazy engine build, param-switch refusal, pts/duration/pool emission) are never executed by any test. The in-tree suites (`crates/codecs/tests/aac/*.rs`, fuzz target) all use `decode_stream` (ADTS walk), a different code path that shares only `decode_one`. A regression in, say, the `header_len..frame_len` payload slice or the `samples_fed` pts base would pass the entire suite.
Why it matters: the one lane the shipped binary runs is the one lane tests do not touch; the baseline smoke (`sample-aac.mp4` plays) is a single happy-path sample, not a regression net.
What correct looks like: one test per gate (truncated ADTS frame → `aac.adts.truncated`; mid-stream rate switch → `aac.adts.param_switch`; a two-packet decode asserting pts/duration/layout advance by 1024).
Confidence: HIGH — Verification: read of the test module + `rg -n "AacDecoder" crates apps` (no other test constructs it).

### F-003 [S4] Reserved ASC channel configs (8-15) bypass the F6 gate and surface as the opaque engine error F6 was filed to eliminate
Where: crates/codecs/src/audio/aac/decoder.rs:452-456
Code:
```rust
                if let Some(chcfg) = sniff_asc_channel_config(asc)
                    && let Some(channels) = asc_channel_count(chcfg)
                {
                    gate_channels(chcfg, channels)?;
                }
```
What is wrong: `asc_channel_count` returns `None` for indices 8-15 (part1:128-135), so the gate is skipped entirely for reserved configs. The F6 comment block (446-451) says the point of the gate is that channel damage surfaces as typed `aac.channel_count`, "never surfacing the engine's opaque 'aac too complex' as `aac.engine.init`" — but exactly that opaque surfacing happens for reserved indices, assuming symphonia refuses them (UNVERIFIED: no registry source; if symphonia ACCEPTS a reserved config, this becomes the F-001 panic precondition instead).
Why it matters: error-taxonomy inconsistency on the documented path; a user with a corrupt ASC gets an opaque message the module's own issue history (F6/#16) calls unacceptable.
What correct looks like: `None => return Err(unsupported("aac.channel_config_reserved", ...))` for sniffable-but-reserved configs.
Confidence: MEDIUM — Verification: read; engine behavior for 8-15 UNVERIFIED.

### F-004 [S4] `Emitted { frames: 1 }` is returned even on the (currently dead) zero-sample branch
Where: crates/codecs/src/audio/aac/decoder.rs:554-592
Code:
```rust
        let emitted_samples = interleaved.len() / channels.max(1);
        if emitted_samples > 0 {
```
…
```rust
        self.samples_fed += frame_samples as u64;
        Ok(DecodeOutcome::Emitted { frames: 1 })
```
What is wrong: if `emitted_samples == 0` the method still reports one emitted frame while sinking nothing. Today unreachable (`decode_one` refuses `frames == 0` at part1:263-270 and `channels ≥ 1` is guaranteed by 529/548), but the guard's existence shows the author considered it possible — and the outcome would then lie to the engine's frame accounting. `samples_fed` also advances by the full `frame_samples` regardless, which is correct only because the branch is dead.
Why it matters: dead-branch dishonesty; if F-001's mismatch ever materializes with a zero-length slice, the pts ledger and the emitted count disagree silently.
What correct looks like: `if emitted_samples == 0 { return Ok(DecodeOutcome::NeedMoreInput) }` (or drop the guard and document the invariant).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part2 only)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Decoder::configure` | 422-487 | REAL | S4 (F-003) | Descriptor/ASC/HE/channel gates all typed; full state reset at 476-485 (clean reconfigure across generations); engine built eagerly in ASC mode, deferred in ADTS mode. |
| `Decoder::decode` | 489-593 | REAL | S3 (F-001), S3 (F-002 coverage), S4 (F-004) | Generation gate + stale-packet drop (codec-api decode.rs:203); empty→NeedMoreInput; ADTS: per-packet header parse, profile/channel gates, truncated refusal (514-524), lazy engine build once (525-529), param-switch refusal (530-539); payload slice `get(header_len..frame_len)` safe (adts.rs guarantees frame_len ≥ header_len, adts.rs:156-162); emit: pooled slot, exact-byte slice (564), pts from `samples_fed` on 1/rate base, ns duration floor-rounded (≤1 ns/frame drift); saturating casts (577, 581-585). |
| `Decoder::flush` | 595-603 | REAL | — | BadState before configure; `Drained` immediately — honest for AAC-LC (each packet's 1024 samples emit fully; IMDCT overlap is intra-frame engine state). |
| `pooled_sample_slot` | 608-645 | REAL | S3 (F-001 panic sink) | 5.A1 rotation, depth 4; exclusivity via `get_mut` (619); fresh-slot typed refusal (628-635); pool cap then round-robin overwrite (637-643); no div-by-zero (loop only when count>0). `dst[cursor..cursor+4]` (559) trusts the 1024×2 slot bound. |
| `descriptor_gate_accepts_aac_only` | 651-661 | REAL | — | Would fail on accepts() regression. |
| `asc_object_type_sniffing_spots_the_he_family` | 663-674 | REAL | — | AOT bit math incl. 1-byte short. |
| `asc_bytes` (helper) | 680-699 | REAL | — | MSB-first ASC builder; honest (not generated by the code under test). |
| `asc_channel_config_sniffing_handles_index_and_escape` | 701-721 | REAL | — | Escape window + count table edges. |
| `six_channel_asc_refuses_typed_not_engine_opaque` | 729-782 | REAL | — | The F6 regression test: 5.1/7.1/escaped/PCE refused with exact codes; stereo/mono configure the real engine. Strong. |
| `he_asc_refuses_typed_and_never_reaches_the_engine` | 784-824 | REAL | — | AOT 5 → `aac.he_sbr`; AOT 4 → `aac.profile`; 1-byte → `aac.asc.length`. |
| `decode_before_configure_is_typed_bad_state` | 826-855 | REAL | — | Real Packet + sink; asserts `aac.not_configured`. |
| `configure_rejects_foreign_descriptors` | 857-878 | REAL | — | flac → `aac.codec_descriptor`. |
| `empty_buffer_and_garbage_refuse_typed_not_panicking` | 880-893 | REAL | — | decode_stream-level; not the contract path (F-002). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 652 `descriptor_gate_accepts_aac_only` | YES | Direct accepts() table. |
| 664 `asc_object_type_sniffing_spots_the_he_family` | YES | Six AOT mappings + short input. |
| 702 `asc_channel_config_sniffing_handles_index_and_escape` | YES | Bit-window math incl. 24-bit escape. |
| 730 `six_channel_asc_refuses_typed_not_engine_opaque` | YES | Mutating the F6 gate to pass-through flips the code assertions; clean configs prove the engine really initializes. |
| 785 `he_asc_refuses_typed_and_never_reaches_the_engine` | YES | Three distinct typed codes. |
| 827 `decode_before_configure_is_typed_bad_state` | YES | BadState ordering. |
| 858 `configure_rejects_foreign_descriptors` | YES | Descriptor gate. |
| 881 `empty_buffer_and_garbage_refuse_typed_not_panicking` | PARTIAL | Real refusals, but tests `decode_stream` (part1), not this part's `decode` — the truncated/param-switch/emit arms of `decode` (511-592) have NO test. |
Mutation check (reasoned): deleting the truncated-frame guard (514) or the param-switch check (530) would NOT fail any in-tree test — both are untested (F-002). Deleting an ASC gate DOES fail (tests above). The asymmetry is the finding.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "AAC in MP4 vs ADTS" — both configure shapes work | module doc 350-363; doc 04 §3.1 | REAL | ASC: baseline sample-aac.mp4 plays (INDEX.md §CORRECTION v2). ADTS shape: code path 505-541 + adts demuxer mints "aac-lc" (demux/src/adts/mod.rs:246,287) — ADTS *packet* decode itself has no executed evidence in baseline (REAL-UNPROVEN for that half). |
| "HE-AAC honestly partial … never a silent downgrade" | module doc 33-52 (part1), gate at 445 | REAL | Test 785; gate before any engine call. |
| "Damage is never a panic" | doc block 284-291 (part1, applies to shared path) | REAL-UNPROVEN | Holds for every path I could trace (all indexing via `get`, saturating casts); the F-001 pool-write panic precondition could not be excluded without symphonia source. |
| `lumen engine-info` decoders list | baseline INDEX.md §CORRECTION v2 standing defects | FALSE (the tool) | Hard-coded literal under-reports: AAC decodes in the shipped build (baseline smoke) while engine-info says mjpeg+pcm only. Not this file's defect. |

## 7. Dead / duplicated / unreachable code
- Nothing dead in part2. The whole part IS the reachable surface (unusually for this wave).
- Duplication: `pooled_sample_slot` is described as "the 5.A1 pattern, re-authored" — i.e., a near-copy exists in flac (5.A1) and mp3 (`pooled_sample_slot` in mp3/decoder.rs). Three hand-copied pools; a shared helper in codec-api would kill the drift risk (the identical `fresh_slot_not_exclusive` error shape suggests copy-paste). S4-class; noted for the cross-codec owner.
- mp3/decoder.rs `decode` (read for contrast) has the same `out_channels`-after-first-decode shape as F-001 — flag to the mp3 owner (W07-A04).

## 8. Dependencies & flags
- Feature `codec-aac` (default on; codecs/Cargo.toml:48,60) → compiled into shipped CLI/web.
- symphonia-codec-aac =0.6.1 + symphonia-core =0.6.1 (exact pins; admission records in docs/dependencies/records/symphonia-aac-0.6.1.md). No unsafe in this part.

## 9. Open questions for the next agent
- W07-A02 (part1 owner): does `decode_stream`'s param-switch gate (317-329) have a corpus test? If not, the same coverage gap as F-002 exists on the ADTS-walk path.
- Anyone with registry access: does symphonia-codec-aac 0.6.1 (a) reject channel configs 8-15 at `try_new`, (b) ever return a per-decode `spec().channels()` different from the configured one? F-001/F-003 severity hinges on both.
- AC-3/TrueHD (my wave's domain): the demuxer mints `ac3`/`eac3` descriptors for MP4 (mp4/mod.rs:451-452) but NO decoder exists anywhere in crates/codecs/src/audio/ (verified: `ls` + `rg "ac3\|truehd\|midi"` → nothing) — pipeline_host falls to `cli.codec_unavailable`. Dead descriptors at the demux layer; belongs to the demux wave's ledger.
