# AUDIT: crates/codecs/src/audio/aac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 94 / 6 units: module docs+decls, `pub use` re-export, `is_adts_stream`, tests (`probe_accepts_validated_headers_only`) |
| File verdict | REAL (the module's surface — engine-backed AAC-LC decoder — is implemented, wired into the shipped MP4/M4A/ADTS lanes, and executed end-to-end in baseline); two doc/dead-code defects inside the module file itself |
| Reachable from shipped entry point? | YES. Full AAC-in-MP4 chain verified link by link: `demux/src/mp4/mod.rs:449` `b"mp4a" => Some("aac")` → `apps/cli/src/containers/mp4.rs:229-232` `config_blob("aac", entry)` = `asc_of_entry(entry)` (esds walked to the bare DSI/ASC bytes) → host facts pre-pass → `apps/cli/src/pipeline_host.rs:288-292` `"aac" \| "aac-lc" => self.seeded(track, config, AacDecoder::new)` → `AacDecoder` (symphonia-codec-aac 0.6.1). ADTS lane: `demux/src/adts/` (5.C2b landed) → `containers/adts.rs`. Executed: baseline INDEX.md corrected matrix — `sample-aac.mp4` Buffering → **Playing** → Stopped, `sample-420.mp4` (H.264+AAC) **PLAYS**; `containers::mp4::tests::h264_aac_fixture_opens_with_expected_facts ... ok`, `six_channel_aac_geometry_flows_but_the_decoder_refuses ... ok`, `adts_aac_family_lane ... ok`, `es::tests::aac_lane_is_the_landed_adts_demuxer ... ok` (orchrun :2818, :2822, :2938, :1680); codecs suites: adts/decoder/error/fuzz unit tests + `aac_bitexact` 2/2 + `aac_corpus_differential` ok + `aac_fuzz_survival` 2/2 (baseline :276-293, :792-812). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:0 |
| Commands run | Read 94 lines (full); link-by-link wiring trace (mp4 demuxer → CLI containers/mp4.rs config_blob/asc_of_entry → pipeline_host dispatch); grep `is_adts_stream` over the whole tree (zero callers outside own test); grep demux/src/adts (own `probe` at mod.rs:147); grep `half-rate\|implicit\|HE` in tests/aac_corpus_differential.rs + tests/data/aac_bitexact_reference.txt (no degradation row); Read reference-data fixture rows (0059/0110/0029/0094); grep docs/formats/audio-aac.md + feature-matrix rows 682/685; baseline log greps (:276-293, :792-812, orchrun :1680/:2023/:2818-2822/:2938); sed verification of quoted ranges. |

## 1. One-paragraph verdict
This module file answers the wave's hostile question affirmatively: the AAC-in-MP4 path is REAL and works end to end in the shipped build — I traced every link (mp4a → `"aac"` descriptor, esds → ASC harvest, seeded configure, symphonia engine) and the baseline proves execution (audio-only AAC MP4 plays; H.264+AAC MP4 plays; CLI mp4/adts lanes green; corpus differential vs ffmpeg green with ±1 LSB). The `engine-info` "decoders: mjpeg, pcm_s16le" literal is the misleading part, already a recorded standing defect — the real registry includes this decoder. Inside the file itself, two defects: `is_adts_stream` is a dead public probe whose doc claims it "complements" the 5.C2b demuxer row — that row landed with its OWN probe and never calls this one; and the HE-AAC honesty section claims "the differential records the degradation" for implicit-HE streams — no differential row, fixture, or reference annotation records any such degradation, and the docs (formats/audio-aac.md:43,79) repeat the false claim.

## 2. What actually works (verified)
— AAC-in-MP4 end to end (the hostile question): chain cited in the header row; executed evidence: `sample-aac.mp4` → Playing (INDEX.md corrected matrix), `sample-420.mp4` PLAYS, `h264_aac_fixture_opens_with_expected_facts ... ok` (orchrun :2818). Audio renders into the null device only (no sound output path — standing defect, INDEX.md).
— ADTS lane: `adts_aac_family_lane ... ok`, `es::tests::aac_lane_is_the_landed_adts_demuxer ... ok` (orchrun :2938, :1680); ADTS parse/resync unit tests 4/4 ok (baseline :276-279).
— HE/profile/channel gates exist with the exact documented codes: `aac.he_sbr`, `aac.profile`, `aac.channel_config_pce`, `aac.channel_count` (decoder.rs:159-198); executed: `he_asc_refuses_typed_and_never_reaches_the_engine ... ok`, `six_channel_asc_refuses_typed_not_engine_opaque ... ok`, `asc_object_type_sniffing_spots_the_he_family ... ok` (baseline :280-281, :285, :290); the 6-channel corpus row `0094` asserts the typed refusal (aac_corpus_differential.rs:23-25) — honest partial, REAL.
— "no panic on any input": `aac_fuzz_survival` 2/2 ok (sync storms, garbage interleaves, mutated fixtures — baseline :792-812).
— "ffmpeg decodes exactly `frames × 1024` samples": `aac_bitexact::lc_frames_are_exactly_1024_samples ... ok` + `fixtures_decode_the_recorded_window_exactly ... ok` (equal-length, ±1 LSB windows, baseline :792-798); corpus rows 0059/0110 record both MD5s with `max_sample_delta = 1` (reference data lines 42-63).
— Fuzz wiring honesty: mod.rs:41-42 says the `fuzz_codec_aac` wiring status "documented there, honestly" — verified TRUE (fuzz.rs:29 admits the campaign "is owed CI wiring").
— `probe_accepts_validated_headers_only` is a real test: positive header (MPEG-4/no-CRC/LC/44.1k/stereo) plus four refusals (layer-1, no sync, `fLaC`, empty) — ran ok (baseline :291).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/audio/aac/mod.rs:72-79
Code:
```rust
/// Whether `data` begins like a raw ADTS AAC stream: a valid fixed
/// header (full validation — sync, layer, tables) at offset 0 (the
/// codec-side probe the 5.C2b demuxer row complements; doc 04 §2.2
/// "ADTS AAC" row).
#[must_use]
pub fn is_adts_stream(data: &[u8]) -> bool {
    adts::parse_adts_header(data).is_ok()
}
```
What is wrong: `is_adts_stream` has ZERO callers in the entire tree (grep over crates/ + apps/ — only its own test at :86-92). The 5.C2b demuxer row it claims to "complement" LANDED (`crates/demux/src/adts/`, card 5.C2b, with its own `pub fn probe` at adts/mod.rs:147 and its own header validation) and does not call the codec-side probe.
Why it matters: dead public API whose doc implies a live architectural role; two independent ADTS header validators now exist with nothing keeping their acceptance sets in sync — a stream the demuxer accepts but `parse_adts_header` rejects (or vice versa) would mis-frame exactly where the doc claims the two complement each other.
What correct looks like: delete `is_adts_stream` (its test with it), or make the demuxer's probe delegate to it (one validator).
Confidence: HIGH — Verification: read + exhaustive caller grep; demux-side probe read at adts/mod.rs:147.
### F-002 [S3]
Where: crates/codecs/src/audio/aac/mod.rs:26-31 (claim) vs crates/codecs/tests/aac_corpus_differential.rs + tests/data/aac_bitexact_reference.txt
Code:
```rust
//! ADTS HE signaling is undetectable from the header (the SBR
//! extension hides behind full SCE/CPE syntax) — such streams decode
//! as **half-rate LC**, and the differential records the degradation
//! rather than silently passing it.
```
What is wrong: no differential row records any implicit-HE degradation. The fixture set is: `sine441_lc_stereo`, `sine48_lc_mono` (LC), corpus `0059`/`0110` (both mono, 88.2/96 kHz — the implicit-HE signaling *shape*, yet both rows assert EQUAL windows with `max_sample_delta = 1`, i.e. both sides decoded identically — no degradation recorded), `0029` (m4a bridge), `0094` (6-channel typed refusal). No HE fixture, no half-rate row, no degradation annotation anywhere in the reference data. docs/formats/audio-aac.md:43 ("decode as half-rate LC and the degradation is recorded, never silently passed") and :79 repeat the claim.
Why it matters: the module's entire HE-AAC honesty posture rests on this evidence claim; it is absent. In reality an implicit-HE file plays as bandwidth-limited LC with no report anywhere in playback (admitted: undetectable at the header) AND no recorded evidence trail in the differential. The claim is FALSE, not "honestly partial".
What correct looks like: add one implicit-HE fixture (ffmpeg `-profile:a aac_he` cannot target implicit signaling directly, but an SBR stream with the doubled-rate header works) with a row that asserts the half-rate window and annotates the delta in the reference file — or delete the "records the degradation" sentence and the doc echoes.
Confidence: HIGH — Verification: read of the differential source + reference data + fixture list; no HE row exists.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + decls (`adts`/`decoder`/`error`/`fuzz`) | 1-68 | REAL | — | map accurate; gates verified against decoder.rs:159-198; layering claim ("never touches crates/demux") TRUE (no demux import) |
| `pub use decoder::{AacDecoder, DecodedAac, decode_stream}` | 70 | REAL | — | re-export of the executed surface |
| `is_adts_stream` | 76-79 | REAL (dead) | S3 (F-001) | correct validation via `parse_adts_header`; zero callers |
| tests: `probe_accepts_validated_headers_only` | 85-93 | REAL | — | would fail on validation loosening; pins dead code only |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_accepts_validated_headers_only` | YES | one positive + four distinct refusal classes, hand-built headers |
| (module's real evidence lives in siblings/integration) | YES | adts 4/4, decoder 9/9, bitexact 2/2, corpus differential, fuzz 2/2, CLI mp4/adts lanes — all green in baseline (cited in header) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the lossy-audio workhorse for the MP4/M4A and ADTS lanes" | mod.rs:2-3 | REAL | full chain + lanes cited in header row; sample-aac.mp4 plays |
| ASC mode: "`extra_data` = the AudioSpecificConfig, the MP4/esds path" | mod.rs:13-14 | REAL | containers/mp4.rs:229-232 `asc_of_entry`; `h264_aac_fixture_opens_with_expected_facts` ok |
| HE explicit-ASC refusal codes | mod.rs:23-26 | REAL | decoder tests :280-285, :290 ok; codes at decoder.rs:159-198 |
| "such streams decode as half-rate LC, and the differential records the degradation" | mod.rs:28-30; docs/formats/audio-aac.md:43,79 | **FALSE** (the recording part) | F-002: no HE fixture/row/annotation exists |
| "no panic on any input … never a crash" | mod.rs:46-51 | REAL | aac_fuzz_survival 2/2 ok |
| "verified: ffmpeg decodes exactly `frames × 1024` samples" | mod.rs:57-59 | REAL | `lc_frames_are_exactly_1024_samples ... ok` |
| "the codec-side probe the 5.C2b demuxer row complements" | mod.rs:73-75 | FALSE (stale) | F-001: 5.C2b landed with its own probe; this one is uncalled |
| (matrix note, context) "142 concealed (damaged) packets on VALID corpus files of codec `aac` — the decode is not clean" | feature-matrix row 682 notes | REAL-UNPROVEN (open triage, matrix's own note) | not this file's claim; decoder/adts owners (W07-A02/A09) should root-cause — concealment on VALID input contradicts a clean-damage-model reading |

## 7. Dead / duplicated / unreachable code
- `is_adts_stream` (F-001) — dead pub fn + its test.
- Duplication: two independent ADTS header validators (codecs::audio::aac::adts vs demux::adts) — deliberate layering (L3 vs L1), but with the "complement" probe dead nothing pins their acceptance sets together.

## 8. Dependencies & flags
- `codec-aac = ["dep:symphonia-codec-aac", "dep:symphonia-core"]` (codecs/Cargo.toml:60), in `default` (:48) — shipped. Engine pinned =0.6.1, purity record referenced at Cargo.toml:104-109.
- Module compiles behind `#[cfg(feature = "codec-aac")]` (audio/mod.rs:28-29).

## 9. Open questions for the next agent
- aac/decoder.rs owners (W07-A02/A09): the corpus differential's own doc (aac_corpus_differential.rs:14-15) still says "Lumen has no MP4 demuxer on main yet (5.C1 is branch-pending)" — STALE now that demux/src/mp4 + containers/mp4.rs exist; the m4a stream-copy bridge should be replaced by the native MP4 lane (or the comment corrected).
- Matrix row 682's "142 concealed packets on valid AAC corpus files" — triage owed; if real, the damage-model honesty claims in this module's docs need a caveat.
- The `aac-lc` descriptor refusal recorded in row 682's compat-matrix note ("1 corpus tracks of codec `aac-lc` REFUSED (no decoder in this build)") contradicts pipeline_host.rs:288-292 which dispatches both `aac` and `aac-lc` — which build was that matrix run on? Worth a cross-check against the feature set of that lane.
