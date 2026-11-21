# AUDIT: crates/codecs/src/audio/pcm/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 551 / 20 (11 prod items incl. enum/structs/impls + 1 free fn, 9 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.avi|wav|...` → demux mints `pcm_s16le`-family descriptor → `apps/cli/src/pipeline_host.rs:347` `starts_with("pcm_")` lane → `PcmDecoder::with_format` + `configure` → decoder.rs:683 `PcmFormat::from_descriptor` (this file, mod.rs:183); free fn `decode_pcm` reachable via `pub mod fuzz` harness + tests only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `decode_pcm\|PcmSource\|pcm::PcmFormat` (callers: pcm/fuzz.rs:104, decoder.rs tests, codecs/tests/pcm_fuzz_survival.rs — free surface is fuzz/test-only); rg `impl DecoderProvider` apps/cli (pipeline_host.rs:347-371 pcm lane); rg `"pcm_[a-z0-9_]*"` crates/demux/src (descriptor inventory vs vocabulary); sed -n avi/header.rs:110-180 (AVI vocabulary subset check); sed -n decoder.rs:90-231 (PcmPlan/decode_payload delegation); grep baseline test log (9/9 mod tests ok); sed -n docs/formats/audio-pcm.md; rg mulaw/alaw/lpcm docs+code |

## 1. One-paragraph verdict
The vocabulary layer of the in-house PCM decoder is real and exact: 11 formats (u8, s16/s24/s32/f32/f64 × LE/BE), byte-order normalization to host, planar-suffix descriptor grammar, typed config refusals, damage-as-dropped-tail. All 9 module tests executed and passed in the baseline run (`audit/runs/baseline/test-workspace.log.txt:489-497`), and the production lane is proven end-to-end by the RC smoke AVI fixture (60/60 audio frames decoded, baseline INDEX #20 + `mp4-smoke-bench-avi-mjpeg-pcm.txt`). The vocabulary is a strict superset of the AVI demuxer's 6 LE descriptors (verified against `avi/header.rs:146-171`). The one functional hole is at the ecosystem boundary, not in this file's logic: demuxers mint `pcm_mulaw`/`pcm_alaw` (mp4) and `pcm_dvd` (ps) descriptors that this vocabulary deliberately excludes and that no other decoder claims — those files demux then refuse at decode with a PCM-vocabulary error.

## 2. What actually works (verified)
— Vocabulary + descriptor round-trip + bytes/sample + endianness axes: verified by baseline execution `audio::pcm::tests::descriptor_vocabulary_roundtrips_every_format ... ok` (test-workspace.log.txt:491) and read of mod.rs:147-207 (all 11 mappings checked entry-by-entry against the module's own format table).
— Byte-order normalization semantics: `s24_groups_reverse_as_three_byte_units`, `f32/f64_values_survive...`, `u8_is_identity...` all ok in baseline (test-workspace.log.txt:493-497).
— End-to-end shipped-binary evidence: `audit/runs/baseline/rc-smoke-engine-only.log.txt` — AVI MJPEG+PCM first-vertical, 60/60 audio frames decoded (`v_frames=60 v_decoded=60 a_frames=60 a_decoded=60`, INDEX #20).

## 3. Findings
### F-001 [S3] Demux minted descriptors `pcm_mulaw`/`pcm_alaw`/`pcm_dvd` have no decoder — dead-end descriptors refused with a PCM-vocabulary error
Where: crates/codecs/src/audio/pcm/mod.rs:163-177 (the vocabulary) vs crates/demux/src/mp4/mod.rs:461-462 and crates/demux/src/ps/mod.rs:170
Code:
```rust
            PcmFormat::U8 => "pcm_u8",
            PcmFormat::S16Le => "pcm_s16le",
```
(the vocabulary has no mulaw/alaw/dvd entry) and
```rust
        b"ulaw" => Some("pcm_mulaw"),
        b"alaw" => Some("pcm_alaw"),
```
What is wrong: The MP4 demuxer maps `ulaw`/`alaw` AudioSampleEntries to `pcm_mulaw`/`pcm_alaw` and the PS demuxer maps LPCM to `pcm_dvd`. The CLI provider routes anything starting `pcm_` into `PcmDecoder` (pipeline_host.rs:347), whose `configure` then refuses with `pcm.codec_descriptor` ("not in the PCM vocabulary") — a µ-law MP4, A-law MP4, or VOB-LPCM file demuxes fully and then cannot play. VLC plays all three.
Why it matters: Functional dead end visible to users; the error text blames the PCM vocabulary for a codec that is simply absent. `pcm_dvd` is documented unscheduled (docs/formats/container-mpegps.md:92) and AIFF documents the same class of gap honestly (aiff/error.rs:81-82 "ulaw, alaw, ima4 and friends have no decoders"), but the MP4 ulaw/alaw mapping carries no such note and no matrix row owns it.
What correct looks like: Either a companded-audio decoder module (µ-law/A-law are ~30 lines each) or refusing at the demux descriptor mapping (`None`, like AIFF's unknown `sowt`-class handling) so the engine reports "no decoder for pcm_mulaw" instead of a PCM-vocabulary error.
Confidence: HIGH — Verification: read (call chain traced end to end; no execution of a µ-law MP4 — none in the baseline corpus).

### F-002 [S4] Module doc's "honest gaps" list omits the descriptor dead-ends of F-001
Where: crates/codecs/src/audio/pcm/mod.rs:59-67
Code:
```rust
//! layout), WAVE_FORMAT_EXTENSIBLE channel masks (doc 04's "rare
//! channel orders" gap — the demuxer resolves the SubFormat GUID to
//! a width, the mask→layout mapping is future work), and mid-stream
```
What is wrong: The declared gap list (bare `pcm`, EXTENSIBLE masks, mid-stream changes) is accurate but incomplete — mulaw/alaw/dvd landings are the largest real refusal class and are unlisted here.
Why it matters: A reader of this module believes the gap surface is smaller than it is; the honest-gaps posture this project advertises is undermined by omission.
What correct looks like: One added sentence naming the companded/DVD-LPCM descriptors the sibling demuxers already mint.
Confidence: HIGH — Verification: read.

### F-003 [S4] `decode_pcm` always copies the input even on the zero-work path
Where: crates/codecs/src/audio/pcm/mod.rs:281-289
Code:
```rust
    let payload = decoder::decode_payload(data, &plan);
    Ok(DecodedPcm {
        format: plan.output_format(),
        samples: payload.converted.unwrap_or_else(|| data.to_vec()),
```
What is wrong: When no swap and no truncation is needed (`payload.converted == None` — the native-endian clean case, e.g. s16le on an LE host), `decode_pcm` still materializes `data.to_vec()`. The zero-copy claim in the module header (lines 27-30) is true only for the contract path (decoder.rs:174-176 documents `None` as "the caller may reuse the packet buffer zero-copy"); the free surface always allocates.
Why it matters: Cosmetic/perf only — `decode_pcm` is the fuzz/differential entry, never the playback hot path (verified: callers are fuzz.rs:104 and tests). No user-visible effect.
What correct looks like: Either a `Cow<'_, [u8]>` return or a doc sentence scoping the zero-copy claim to the contract path.
Confidence: HIGH — Verification: read + rg of all callers.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum PcmFormat` | 104-128 | REAL | — | 11 variants, matches module table exactly |
| `PcmFormat::ALL` | 133-145 | REAL | — | 11 entries = variant count ✓ (round-trip test pins) |
| `PcmFormat::bytes_per_sample` | 148-157 | REAL | — | 1/2/3/4/8 correct; test recomputes independently |
| `PcmFormat::descriptor` | 163-177 | REAL | — | ffmpeg-compatible names; vocabulary boundary = F-001 |
| `PcmFormat::from_descriptor` | 183-188 | REAL | — | linear scan, 11 compares once per configure; `None` for foreign (test pins 7 refusals incl. `pcm_u16le`, `pcm_s20le`) |
| `PcmFormat::is_little_endian` | 193-207 | REAL | — | U8 → None (byte-order-free) ✓ |
| `PcmFormat::output_sample_format` | 212-221 | REAL | — | maps to media-types SampleFormat; no depth conversion (by design, doc lines 74-76) |
| `struct PcmSource` | 226-236 | REAL | — | channels ≥ 1 enforced downstream (PcmPlan::new) |
| `PcmSource::descriptor` | 242-250 | REAL | — | `_planar` suffix grammar; configure strips it (decoder.rs:679) — pinned by test |
| `struct DecodedPcm` | 255-267 | REAL | — | carries `dropped_tail_bytes` observability |
| `decode_pcm` | 281-289 | REAL | — | delegates to PcmPlan::new + decode_payload; config errors typed (`pcm.channels.zero`, `pcm.rate.zero` — pinned); F-003 copy |
| test `descriptor_vocabulary_roundtrips_every_format` | 295-326 | REAL | — | mutation: any wrong descriptor/width mapping fails ✓ |
| test `planar_suffix_descriptor_shape` | 328-342 | REAL | — | mutation: suffix grammar change fails ✓ |
| test `every_format_decodes_a_clean_buffer_value_exactly` | 344-400 | REAL | — | 11×2×3 sweep, expectations computed independently of decoder; proves both byte-order directions on any host ✓ |
| test `decode_pcm_rejects_broken_configurations_typed` | 402-430 | REAL | — | pins both error codes ✓ |
| test `s24_groups_reverse_as_three_byte_units` | 432-453 | REAL | — | byte-exact 3-byte group reversal, host-conditional ✓ |
| test `f32_values_survive_the_byte_order_normalization` | 455-485 | REAL | — | value-level assertion through `from_ne_bytes` ✓ |
| test `f64_values_survive_the_byte_order_normalization` | 487-518 | REAL | — | + frame_bytes()==16 ✓ |
| test `u8_is_identity_and_byte_order_free` | 520-535 | REAL | — | identity ✓ |
| test `damaged_tail_is_reported_not_failed` | 537-550 | REAL | — | 5-byte buffer → 4 bytes out + 1 dropped ✓ |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 9 tests | YES | Every expectation is hand-computed (byte groups reversed by hand, error codes named, tail counts exact); none derived from the subject. All executed in baseline (test-workspace.log.txt:489-497). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = implemented, full unpacked-PCM format matrix" | docs/formats/audio-pcm.md front matter | REAL | 11-format vocabulary + baseline AVI 60/60 lane (rc-smoke-engine-only.log.txt) |
| "s16/s24/s32/f32/f64 LE+BE, interleaved+planar, plus u8" | mod.rs:43-46 table | REAL | PcmFormat::ALL + round-trip test |
| "no depth conversion, no rate conversion, no channel remix: every decoded byte is a permuted input byte" | mod.rs:74-78 | REAL | decode_payload (decoder.rs:215-231) only swaps/truncates; f32/f64 pass through as raw IEEE bytes |
| "bit-exact against fixture audio_stream_sha256" | audio-pcm.md Evidence § | REAL-UNPROVEN here | sidecar hash asserted in AVI integration tests (not this file); baseline suite green (test-workspace #3, 1681 passed) |
| "honest gaps: bare pcm, EXTENSIBLE masks, mid-stream changes" | mod.rs:59-67 | SHALLOW (incomplete) | F-002: mulaw/alaw/dvd dead-ends unlisted |

## 7. Dead / duplicated / unreachable code
- `decode_pcm` + `DecodedPcm` are the fuzz/test free surface only (callers: pcm/fuzz.rs:104, decoder.rs tests, codecs/tests/pcm_fuzz_survival.rs) — production decode rides `PcmDecoder` (decoder.rs). Not dead (fuzz registry `fuzz_codec_pcm` is a shipped target per scripts/fuzz_registry_check.sh PASS, baseline #18), but not on the playback path.
- Vocabulary duplication risk: demux-side PCM descriptor vocabularies (avi/header.rs:146-171 — verified strict subset; wav/format.rs, aiff — not re-audited here, other agents' files) mirror these strings by design (boundaries.toml forbids the dep). AVI side verified consistent today.

## 8. Dependencies & flags
- `codec_api::prelude`, `media_types::frame::{AudioFormat, SampleFormat}` only; zero external deps, no feature gate (in-house decoder is unconditional per codecs/Cargo.toml comment); no `unsafe`.

## 9. Open questions for the next agent
- pcm/decoder.rs owner: the planar tail-drop rule (`whole_frame_prefix`, decoder.rs:198-209) assumes equal-length planes for planar sources; an out-of-contract planar packet with unequal planes gets a prefix cut slicing across the plane boundary — confirm audio-engine planar consumers never assume equal planes.
- pcm/decoder.rs owner: `MAX_PCM_CHANNELS = 64` (decoder.rs:91) caps the contract path but `PcmPlan::new` (via `decode_pcm`) accepts unbounded channel counts — deliberate per comment decoder.rs:128-135; confirm the fuzz harness sweeps that.
- F-001 routing: whoever owns apps/cli/src/pipeline_host.rs — `starts_with("pcm_")` catches `pcm_mulaw` and produces a misleading PCM-vocabulary refusal.
