# AUDIT: crates/demux/src/wav/format.rs#part2 (lines 441–880)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 440 (of 881; part2) / parse_fmt tail + 1 helper + 10 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (parse_fmt is called by `WavDemuxer::parse_fmt_chunk` (wav/mod.rs:464) ← `parse_container` ← `Demuxer::start` ← `containers/wav.rs:50` ← `lumen` open path — see wav/mod.rs#part1 audit; executed: all 10 in-range tests ok in baseline `orchrun-test-workspace-nofailfast.log.txt:1912-1921` + corpus/fixture suites) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "test wav::format::" baseline log` (10/10 ok); `rg -n "from_wave_tag\|SUBFORMAT" crates/codecs/src/audio/pcm/decoder.rs` (mirror-claim spot check: same two GUIDs at :435-437); `sed -n` on quoted ranges; formula verification against IMA/MS ADPCM spec arithmetic |

## 1. One-paragraph verdict
The second half of `wav/format.rs` is the tail of `parse_fmt` — ADPCM
samples-per-block resolution (declared vs block-geometry derivation with
typed refusals on arithmetic lies), the valid-bits and block-align
consistency gates — plus a ten-test battery that is field-exact and
externally anchored: every happy-path shape mirrors a measured
ffmpeg-emitted fixture (pcm24-extensible, the 0016 FLAC 74-byte shape, the
0042 IMA / 0102 MS corpus shapes with their declared values verified against
corpus bytes), and every refusal path pins the exact typed error code. All
ten tests ran and passed in the baseline. The single finding is a
defense-in-depth gap with zero current blast radius: an ADPCM `fmt ` whose
declared `wSamplesPerBlock` is present but whose block geometry cannot
physically carry a preamble (`nBlockAlign ≤ 7·ch` / `≤ 4·ch`) is accepted
because derivation returns `None` and the declaration is trusted
unvalidated.

## 2. What actually works (verified)
- All 10 in-range tests ok in baseline (:1912-1921): `adpcm_tags_parse_their_block_framing`,
  `extensible_flac_guid_exposes_streaminfo_and_descriptor`,
  `extensible_float_guid_maps_to_f32`, `extensible_without_body_is_a_structural_lie`,
  `parses_extensible_pcm24_like_ffmpeg_writes`, `parses_plain_pcm16`,
  `short_fmt_is_structural`, `unknown_tags_and_depths_are_typed_refusals`,
  `valid_bits_below_container_bits_are_refused`, `zero_or_inconsistent_params_are_bad_fmt`.
- Formula correctness externally anchored: IMA `((1024−4·2)·2)/2+1 = 1017`
  and MS `(1024−7·2)/2·2+2 = 1012` match the ffmpeg-authored corpus
  declarations (0042/0102), with the MS value's provenance documented as
  "verified against the corpus bytes `f4 03`" (:722-723).
- The mirror claim spot-check: `codecs/src/audio/pcm/decoder.rs:427-437`
  resolves `0xFFFE` through the same SUBFORMAT_PCM/IEEE_FLOAT GUIDs the
  demuxer uses; the demuxer's FLAC GUID path returns early
  (wav/mod.rs#part1 :378-400) so the PCM decoder never receives it — the
  claimed one-accepted-set seam holds on inspection.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/wav/format.rs:445-450
Code:
```rust
            (Some(declared), None) => {
                if declared == 0 {
                    return Err(error::bad_format_params(offset));
                }
                declared
            }
```
What is wrong: `derived == None` means the block geometry cannot hold even
the per-channel preamble (`block_align ≤ channels·4` for IMA via
`ima_samples_per_block` :308-309, `≤ channels·7` for MS via
`ms_samples_per_block` :321-322) or the arithmetic overflowed. In that case
the declaration is accepted verbatim — a structurally impossible block size
(e.g. IMA, 2 channels, `nBlockAlign = 4`) yields `samples_per_block =
declared` and the demuxer frames 4-byte "blocks" each claiming the declared
sample count.
Why it matters: Every other impossible combination in this parse is a typed
refusal; this one silently mints container facts no ADPCM implementation can
honor. Blast radius today is zero — the engine carries no ADPCM decoder
(mod.rs:49-52: the decode gap is reported, never faked) — but the day one
lands, this is a decode-side crash/panic feed.
What correct looks like: `derived == None` with any `declared` should be
`error::bad_format_params` (geometry below the preamble floor is never a
variant), matching the `(Some(0), None) | (None, None)` arm's stance.
Confidence: HIGH — Verification: read (all three helper branches traced);
no run needed (pure control flow).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_fmt` ADPCM resolution tail (boundary: match opens :430 in part1) | 441-468 | REAL | S4 (F-001) | (Some,Some): refuse 0 or >derived, accept ≤derived (lenient but sane); (Some,None): F-001; (None,Some): derived; (None,None)/(0,None): typed refusal. Codec construction carries honest facts. |
| `parse_fmt` PCM path tail | 470-507 | REAL | — | `from_wave_tag` vocabulary gate (typed unsupported for 20-bit/float-8/unknown); valid-bits: 0 or >bits → bad-fmt, ≠bits → unsupported (the documented [VERIFY] 20-in-24 refusal); `nBlockAlign == channels × bytes_per_sample` exact (decoder re-checks the verbatim blob — spot-verified decoder.rs:427-437). |
| `tests::pcm16` helper | 513-522 | REAL | — | honest 16-byte PCMWAVEFORMAT builder. |
| `parses_plain_pcm16` | 524-544 | REAL | — | field-exact incl. descriptor string. |
| `parses_extensible_pcm24_like_ffmpeg_writes` | 546-576 | REAL | — | mirrors the committed `wav_pcm_s24le_48k_st.wav` fixture shape (cbSize 22, validBits 24, mask 0x3, PCM GUID). |
| `extensible_float_guid_maps_to_f32` | 578-602 | REAL | — | GUID → f32le descriptor. |
| `extensible_flac_guid_exposes_streaminfo_and_descriptor` | 604-645 | REAL | — | 74-byte shape with STREAMINFO [40..74] + the tailless variant (streaminfo None) — both pinned. |
| `extensible_without_body_is_a_structural_lie` | 647-659 | REAL | — | 0xFFFE + cbSize 0 → `demux.wav.bad-structure`. |
| `unknown_tags_and_depths_are_typed_refusals` | 661-692 | REAL | — | 20-bit int, float-8, unknown tag 0x0055 — all `demux.wav.unsupported-format`. |
| `adpcm_tags_parse_their_block_framing` | 694-803 | REAL | — | The strongest test: declared IMA 1017, MS 1012 (corpus shapes), the >derived lie (4000) refused `demux.wav.bad-fmt`, and BOTH derivation-only paths (18-byte fmt) pinned to the formulas. |
| `valid_bits_below_container_bits_are_refused` | 805-824 | REAL | — | 20-in-24 → `demux.wav.unsupported-format` (the [VERIFY] gap, honestly typed). |
| `zero_or_inconsistent_params_are_bad_fmt` | 826-870 | REAL | — | zero rate, zero channels, padded-24 align lie, channels 65 > MAX_CHANNELS 64 — all `demux.wav.bad-fmt`. |
| `short_fmt_is_structural` | 872-879 | REAL | — | 15-byte fmt → `demux.wav.bad-structure` (MIN_FMT_BYTES 16 boundary). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 10 | YES | Every test pins exact fields or exact error codes; mutation examples: flip `valid != bits` to `valid > bits` → `valid_bits_below…` fails; drop the align check → `zero_or_inconsistent…` third case fails; use derived unconditionally → `adpcm…` declared-1012 case fails. Baseline executed 10/10 ok (:1912-1921). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The PCM half of the `fmt ` parse mirrors `codecs::audio::pcm`'s blob rules exactly (tag/GUID resolution, width vocabulary, `nBlockAlign == channels × width`)" | wav/mod.rs:44-48 | REAL | Spot-verified: same two SubFormat GUIDs (decoder.rs:435-437), same width vocabulary (PcmSample::from_wave_tag :146-162 ↔ decoder descriptors), same align rule (:492-495, decoder re-check documented). Full decoder-side verification belongs to the codecs wave. |
| "wValidBitsPerSample < wBitsPerSample … typed refusals" (recorded gap) | wav/mod.rs:74-76 | REAL | :481-487 + test :805-824. |
| "ADPCM … frames at block granularity (one nBlockAlign block per unit, wSamplesPerBlock frames each)" | wav/mod.rs:22-25 | REAL | WavCodec variants + unit_size/samples_per_unit (part1 :217-241) + corpus-differential tests (corpus_wav.rs 0042/0102 lanes). |

## 7. Dead / duplicated / unreachable code
None in range. (The `PcmSample`/GUID vocabulary is demux-local by design;
the decoder keeps its own copy of the GUID constants — cross-crate
duplication inherent to the no-crate-edge layering, same class as the probe
mirror flagged in wav/mod.rs#part1 F-002.)

## 8. Dependencies & flags
No new deps; media_types only. No unsafe, no feature gates. Test module is
self-contained byte builders (no fs, no subprocess — unlike the corpus
suites).

## 9. Open questions for the next agent
- codecs wave: does `codecs::audio::pcm` reject `nBlockAlign`-lies on the
  verbatim blob with the same code the demuxer would have refused
  (`bad-fmt`), so a demuxer-side acceptance (F-001's class, for PCM) cannot
  reach a decoder panic? (Claimed at :342-344; verify the re-check exists.)
- part1 (A06) owns `ima/ms_samples_per_block` heads and `WavCodec::unit_size`
  — verify `unit_size` for ADPCM is `block_align` (the test asserts 1024)
  and for FLAC is 1 (asserted) — both confirmed by tests in my range.
