# AUDIT: crates/codecs/src/audio/aac/decoder.rs#part1 (lines 1-487)
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-487 (module docs, ASC/ADTS gates + sniffs, `adts_engine`, `decode_one`, `decode_stream`, `AacDecoder` struct, `accepts`, `Decoder::configure`). part2 = lines 488-895 (`decode`, `flush`, `pooled_sample_slot`, the whole `#[cfg(test)] mod tests` at 647-895) — W07-A09. Boundary set at the end of `configure` (a complete coherent unit, 487/408 split); documented here so part2 does not re-audit 1-487. The tests that exercise part1 functions (651-721: descriptor/ASC sniff tests; 723-878: configure tests) are part2's line range — I verified them green in baseline and cross-reference, but the per-test honesty table belongs to part2. |
| Lines / functions | 487 / 20 |
| File verdict | REAL (part1: implemented, reachable, engine-backed; the free `decode_stream` surface is UNREACHABLE-in-production test tooling) |
| Reachable from shipped entry point? | YES for `AacDecoder::{new, configure}` — call chain: `lumen open` → apps/cli/src/pipeline_host.rs:288-291 `"aac" \| "aac-lc"` arm → `AacDecoder::new` → `configure` (ASC mode for MP4/esds); executed by baseline smoke `sample-aac.mp4` (Buffering → Playing, no error events, exit 0 — INDEX.md corrected matrix). NO for `decode_stream`/`DecodedAac` — callers: aac/fuzz.rs:88, streaming/tests/hls_vod_e2e.rs:300, in-crate tests only (rg over repo). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read decoder.rs 1-895 (structure) · `rg -n "AacDecoder\|decode_stream" repo` (callers: CLI dispatch + test/fuzz only) · `rg -n "aac" apps/cli/src/pipeline_host.rs` (dispatch 288-291; bench.rs hard-coded lane) · Read adts.rs 60-228 (walker guarantee: pushed frames always `end ≤ data.len()`; channel_count mapping) · Read pinned engine source `/tmp/.../symphonia-codec-aac-0.6.1/src/aac/mod.rs` (try_new gates: non-LC/sbr/>2ch/≠1024 → "aac too complex"; output buffer allocated ONCE with fixed AudioSpec → channel count immutable; decode_ref errors clear buf, never panic) + `symphonia-common/src/mpeg/audio/mod.rs:198-215,454-463` (ASC channel-config 8-15 → Invalid decode_error; 0 → Escape→None) + `symphonia-core/src/audio/util.rs:130-136` (`copy_to_slice_interleaved` asserts exact dst size) · `rg -n "aac" audit/runs/baseline/test-workspace.log.txt` (12 unit + 2 bitexact + 1 differential + 2 fuzz-survival tests green) · `sed -n` quote verification below · `ls docs/dependencies/records/` (symphonia-aac-0.6.1.md exists, header verified) |

## 1. One-paragraph verdict
This is the strongest kind of wrapper code in the tree: the ASC bit-sniffing (5-bit AOT at `asc[0]>>3`, channel config at bits 9-12 or 33-36 under the 24-bit rate escape) is bit-exactly correct — I re-derived every field offset by hand; the gate ordering in `configure` (object-type gate at 445 BEFORE the channel sniff at 452) is load-bearing and correct, because an SBR ASC (AOT 5) interleaves an extension-frequency field that would poison the channel-config sniff — and every refusal is typed with the documented code. The wave question's answer lands here: **AAC-in-MP4 works end to end in the shipped build** — the MP4/esds lane mints descriptor `aac` + the ASC blob, the CLI dispatch table (pipeline_host.rs:288-291, default features include `codec-aac`) constructs this decoder, and baseline evidence shows `sample-aac.mp4` opening to Playing with zero error events plus `fixtures_decode_the_recorded_window_exactly` (bitexact) and `corpus_differential_vs_ffmpeg` (differential) both green. I additionally verified the wrapper's trust in the pinned engine from the crate source itself: the engine's output buffer is allocated once with a fixed channel spec (channel count can never vary mid-stream, killing the mislabel scenario I went hunting for), `copy_to_slice_interleaved`'s exact-size assert is always satisfied by `need = frames × channels` computed from the same buffer, and in-band SBR extension payloads are skipped (the "half-rate LC" honesty claim is exactly what the engine does). The one finding is a defensive-shape nit. The free `decode_stream` surface is honest conformance tooling that no shipped entry point reaches.

## 2. What actually works (verified)
— ASC sniff bit math re-derived by hand: AOT bits 0-4 → `asc[0] >> 3` (109-122); freq index bits 5-8 → `((asc[0] & 0x07) << 1) \| (asc[1] >> 7)`; channel config bits 9-12 → `(asc[1] >> 3) & 0x0F`; escaped (index 0xF) rate bits 9-32 → config at bits 33-36 → `(asc[4] >> 3) & 0x0F` (142-151). All correct, and pinned by tests `asc_object_type_sniffing_spots_the_he_family` + `asc_channel_config_sniffing_handles_index_and_escape` (baseline test-workspace.log.txt:280-281 green).
— Engine invariants verified from the pinned crate source (not from docs): `symphonia-codec-aac-0.6.1/src/aac/mod.rs:87-93` refuses non-LC/sbr_present/>2 channels/≠1024 samples; the output `AudioBuffer` is built once at `try_new` with the fixed spec → `decoded.spec().channels().count()` is immutable per engine; `decode_ref` on error clears and returns Err (no panic); `symphonia-core/src/audio/util.rs:133-136` asserts `dst.len() == num_planes * bound.len()` — always satisfied because `decode_one` computes `need` from the same decoded buffer (271-276).
— Walker guarantee: `adts.rs:214-219` pushes only frames with `pos + frame_len ≤ data.len()` (trailing partial breaks), so `decode_stream`'s `data.get(start..end).unwrap_or(&[])` (332) never falls back — the "trailing partial dropped" doc claim (290-291) is the walker's verified behavior.
— End-to-end AAC-in-MP4 in the shipped build: baseline INDEX.md corrected matrix row `sample-aac.mp4` → "Buffering → **Playing** → Stopped" with no `decode.unsupported`/error events (the yuv444-h264 and HEVC samples in the same run DID error), plus `aac_bitexact.rs: fixtures_decode_the_recorded_window_exactly ok` and `aac_corpus_differential.rs: corpus_differential_vs_ffmpeg ok` (test-workspace.log.txt:795-805).
— `codec-aac` is in the codecs crate's `default` feature set (crates/codecs/Cargo.toml:60,67) — the shipped CLI build compiles and dispatches this decoder; `lumen engine-info`'s "decoders: mjpeg, pcm_s16le" literal under-reports it (standing INDEX.md defect).

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/aac/decoder.rs:209-214
Code:
```
fn adts_engine(header: &super::adts::AdtsHeader) -> Result<AacEngine, AacError> {
    let channels = if header.channels == 1 {
        CHANNEL_LAYOUT_MONO
    } else {
        CHANNEL_LAYOUT_STEREO
    };
```
What is wrong: the layout selection tolerates every `channels != 1` as stereo, silently. Today the call sites are gated (`gate_channels` refuses config 0 → channels 0, and > 2; adts.rs `channel_count` maps 1..=6→config, 7..=15→8, so reachable values are exactly {1,2}), but the function itself neither documents that precondition nor refuses a violation — a future relaxation of `gate_channels` (e.g., admitting 5.1) would silently initialize a STEREO engine for a 6-channel header and decode garbage rather than refusing typedly.
Why it matters: this is the exact class of silent-degradation the module's own HE-honesty section (33-56) exists to prevent; the guard is one `match` away and absent.
What correct looks like: `match header.channels { 1 => MONO, 2 => STEREO, other => return Err(unsupported("aac.channel_count", …)) }` — making the local invariant local.
Confidence: HIGH — Verification: read; both call sites (307-308, 512-513) traced to gated paths; the unsafe value is unreachable today (hence S4, not higher).

## 4. Per-function table (part1: lines 1-487)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-56 | REAL | — | claims verified: engine exact-pin (`=0.6.1` in Cargo.toml:60s, record exists), purity record exists (docs/dependencies/records/symphonia-aac-0.6.1.md), ADTS/ASC packet model matches code, "half-rate LC" for implicit HE verified in engine source (ID_FIL 0xd/0xe sets sbr_present but payload ignored — mod.rs:192) |
| `SAMPLES_PER_FRAME` | 73 | REAL | — | 1024; matches engine gate (mod.rs:90) |
| `DecodedAac` | 77-86 | UNREACHABLE (production) | — | test/fuzz-only result type; `frames` = ADTS frame count (documented — no unit lie) |
| `AscObjectType` | 91-106 | REAL | — | 5-bit AOT taxonomy incl. 5=SBR, 29=PS |
| `sniff_asc_object_type` | 109-122 | REAL | — | bit math hand-verified; <2 bytes → Other(0) — defensive only: `configure` pre-refuses short ASCs as `aac.asc.length` (438-444), so this branch never fires there |
| `asc_channel_count` | 128-135 | REAL | — | 0→0 (PCE), 1..=6→config, 7→8, 8-15→None (deferred to engine's Invalid decode_error — verified symphonia-common:458-461) |
| `sniff_asc_channel_config` | 142-151 | REAL | — | bit math hand-verified incl. 24-bit escape window; None on short ASC defers to engine (documented 140-141) |
| `gate_object_type` | 155-170 | REAL | — | LC ok; SBR/PS → `aac.he_sbr`; others → `aac.profile`; pinned by he_asc test (baseline:285) |
| `gate_adts_profile` | 173-184 | REAL | — | ADTS 2-bit profile field (0=Main,1=LC,2=SSR,3=reserved — matches adts.rs:138-143) |
| `gate_channels` | 188-206 | REAL | — | config 0 → `aac.channel_config_pce`; >2 → `aac.channel_count`; applied on BOTH paths (the F6/issue #16 fix at 446-456) |
| `adts_engine` | 209-227 | REAL | F-001 | mono/stereo layout from header; engine init errors typed `aac.engine.init` |
| `decode_one` | 233-278 | REAL | — | empty payload typed; engine rejection typed; zero frames/channels typed; `need` from the same decoded buffer → exact-size assert always satisfied (verified against symphonia-core util.rs:133); scratch grow-only (D-1) |
| `decode_stream` | 292-348 | UNREACHABLE (production) / REAL as test tooling | — | no-frames typed; first-frame gates; per-frame param-switch gate (rate/channels/profile) typed; walker guarantee makes the `unwrap_or(&[])` dead; `out_channels` per-frame update (334) is a no-op in practice (engine spec immutable — verified from engine source) |
| `AacDecoder` struct + fields | 365-386 | REAL | — | state inventory sane; pooled slots + counters |
| `Debug` impl | 388-396 | REAL | — | `finish_non_exhaustive` — engine has no Debug; correct |
| `SAMPLE_SLOTS` / `MAX_SLOT_BYTES` | 399-402 | REAL | — | 4 × 8192 B; arithmetic 1024·2·4 correct; consumed in part2 |
| `AacDecoder::new` | 407-409 | REAL | — | Default |
| `accepts` | 416-418 | REAL | — | `"aac" \| "aac-lc"` case-sensitive; pinned by descriptor_gate test (baseline:284) |
| `Decoder::configure` | 422-487 | REAL | — | descriptor gate; extra_data None/empty → ADTS mode, ≥2 bytes → ASC mode (mode selection BY PRESENCE — matches containers/adts.rs:19-27 wiring note "None is the correct wiring"); gate order object-type→channel (load-bearing for SBR ASC layout); full state reset on reconfigure (477-485); pinned by configure tests (baseline:282,285,290) |

## 5. Test honesty
Tests live at 647-895 (part2's range). The ones covering part1 functions, verified green in baseline (test-workspace.log.txt:280-291): `asc_object_type_sniffing_spots_the_he_family`, `asc_channel_config_sniffing_handles_index_and_escape`, `descriptor_gate_accepts_aac_only`, `six_channel_asc_refuses_typed_not_engine_opaque`, `he_asc_refuses_typed_and_never_reaches_the_engine`, `configure_rejects_foreign_descriptors`, `empty_buffer_and_garbage_refuse_typed_not_panicking`. Mutation reasoning: flipping `asc[0] >> 3` to `>> 4` breaks the AOT sniff tests; removing the F6 channel gate (452-456) flips `six_channel_asc_refuses_typed_not_engine_opaque` from `aac.channel_count` to `aac.engine.init` (exact-code assert); swapping the gate order (channel before object-type) makes the SBR ASC (AOT 5 + extension freq field) sniff a wrong channel config — the he_asc test would catch the misroute only if it changed the code; noted for part2's per-test table. All are discriminating asserts on exact error codes — none tautological.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "AAC-LC … symphonia-codec-aac 0.6.1" engine choice, exact-pinned, purity-verified | module docs 5-18; docs/04 §3.1 row 186 | REAL | `=0.6.1` pins in codecs/Cargo.toml; record file exists; engine source matches described gates |
| "HE-AAC … decode as half-rate LC (bandwidth-limited); the differential records the degradation" | module docs 45-52; doc 04 §3.1 "AAC honesty note" 205-213 | REAL | engine mod.rs:192-199: in-band EXT_SBR_DATA sets sbr_present but `ignore_bits` skips the payload — core LC only; he_asc refusal test green |
| "AAC in MP4 vs ADTS" (both lanes) | doc 04 §3.1 row 187; matrix row codec-audio-aac-lc (665-682) | REAL (MP4 lane) / REAL-UNPROVEN (ADTS lane in shipped binary — demuxer exists, `es` demuxer test green baseline:1721, but no shipped-binary ADTS smoke in baseline runs I can cite) | MP4: sample-aac.mp4 Playing (INDEX corrected matrix); dispatch pipeline_host.rs:288-291 |
| matrix codec-audio-aac-lc linux = `partial` ("142 concealed packets on VALID aac corpus files; 1 aac-lc track REFUSED (no decoder in this build) vs 4 decoded") | feature-matrix.toml:675-682 | SHALLOW-evidence lane | the "no decoder in this build" refusal string is the KNOWN hard-coded decode-bench fall-through (INDEX.md standing defects; apps/cli/src/bench.rs) — the matrix's own evidence lane is polluted by broken tooling; the 142-concealed-packets item is a real recorded engine-concealment triage, not verified here (part2/engine territory) |
| "per-frame isolation … after a typed Corrupt the next packet decodes unchanged" | module docs 29-31 | REAL (wrapper) / nuance | wrapper state resets nothing between packets (no shared scratch poison); the ENGINE keeps IMDCT overlap delay buffers across a mid-frame error (decode_ref clears buf only) — next frame decodes but may carry one frame of tainted overlap: standard concealment class, consistent with the recorded 142-concealed triage, NOT a silent-failure bug |
| `lumen engine-info` → "decoders: mjpeg, pcm_s16le" | baseline lumen-engine-info.txt (standing defect) | FALSE as capability report | this decoder + flac/mp3/mp2/vorbis/opus/alac/adpcm are compiled (default features) and dispatched (pipeline_host.rs:259-302) — the smoke run proves AAC decodes |

## 7. Dead / duplicated / unreachable code
- `decode_stream` + `DecodedAac` (292-348, 77-86): UNREACHABLE from shipped entry points — callers are aac/fuzz.rs, streaming/tests/hls_vod_e2e.rs, in-crate tests. Honest conformance/differential tooling, documented as such (mod.rs:37-39); not fraud.
- `AscObjectType::Main/Ssr/Ltp/Sbr/Ps` variants used in gates/tests — all reachable via configure paths. No dead arms found in part1.
- Duplication: the two gate paths (ADTS per-packet at 512-513 in part2, ASC at configure) share `gate_channels` correctly (the F6 fix) — no copy-paste drift.

## 8. Dependencies & flags
- `symphonia-codec-aac` + `symphonia-core` `=0.6.1`, optional behind `codec-aac`, which is in `default` (crates/Cargo.toml:60,67) — shipped builds include it; `--no-default-features` builds drop it (the wasm tree-shake lane).
- Cargo.lock pins checksums for all five symphonia crates (130-134, 1004-1060) — consistent single-version train 0.6.1.
- No `unsafe` in this file (module claim; rg-verified none in 1-487).
- The registry sources are checksum-pinned and present in the local cargo registry — I verified the engine claims from the actual pinned source, not from docs.

## 9. Open questions for the next agent
- **W07-A09 (part2)**: (a) `decode` line 559 writes output with `v.to_ne_bytes()` while audio-engine's convert.rs:88-90 reads F32 with `from_le_bytes` — native vs LE producer/consumer mismatch, harmless on all shipped LE targets (x86_64/aarch64/wasm32), latent on BE; the same `to_ne_bytes` convention is used by opus (decoder.rs:450) and adpcm (ima.rs:204, misc.rs:195) — a crate-wide decision someone should write down. (b) The pooled-slot fill closure (556-563) writes `emitted_samples*channels*4` bytes into an 8192-byte slot — safe only under the engine's ≤2ch/1024-frame gate (verified from engine source); a defensive length check is absent. (c) The 142-concealed-packets triage (matrix notes) — where does concealment surface (DecodeOutcome::Recovered?) in this decoder, if at all?
- Matrix/doc owner: codecs/src/lib.rs module map (lines 33-40) lists only mjpeg/pcm/flac — aac, alac, adpcm, mp2, mp3, opus, vorbis, h264, hevc, mpeg12, av1, vp9, dnx are all absent from the map while being compiled+dispatched; the crate `description` field likewise stops at FLAC. Doc drift, S4.
- Demux wave: confirm the MP4/esds ASC handoff is byte-verbatim (the smoke proves it works for one file; the corpus's 6ch refusal proves the gate fires through the same lane).
