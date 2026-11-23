# AUDIT: crates/codecs/src/audio/adpcm/ima.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 300 / 5 production fns (header_bytes, ChannelState::expand, decode_ima_blocks, push_sample; STEP/INDEX consts) + 5 tests |
| File verdict | REAL (bit-exact vs ffmpeg 7.1.5 on clean input, mono+stereo, committed+fresh; one false doc attribution on the damage path) |
| Reachable from shipped entry point? | YES — `AdpcmDecoder` Plan::ImaWav (mod.rs:592-600) ← apps/cli/src/pipeline_host.rs:298-302 ← WAV demuxer `adpcm_ima_wav` (crates/demux/src/wav/format.rs:207) — the full `lumen open x.wav` lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::ima` → 5 passed (exit 0); python3 table diff (STEP_TABLE == canonical 89-entry ladder: equal; INDEX_TABLE == [-1,-1,-1,-1,2,4,6,8]: equal); python3 full port: committed fixture → 12_246 frames / `8a8146087f7573fa71963c3ae2c65f3b` == ffmpeg; fresh stereo encode → `f2e8673552515dbe541d19d212b29dce` == ffmpeg; ffmpeg damage probes (index 200, short block) |

## 1. One-paragraph verdict
The core decoder is correct and proven: the 89-entry step table and index table are byte-identical to the canonical IMA tables, and the multiply-then-shift diff form (`(step·(2δ+1))>>3`) — which the module correctly says differs from the textbook partial-sum form — reproduces ffmpeg 7.1.5's PCM bit-for-bit on the committed mono fixture and a fresh stereo encode. The block grammar (4-byte/channel headers first, 8-sample channel-major groups, low nibble first) is exactly what ffmpeg reads. The one real defect is documentation, not code: the claim that clamping a header step index >88 is "ffmpeg's own healing, kept for reference parity" is false — ffmpeg 7.1.5 hard-errors on such a header (I reproduced it), so the repo's clamp-and-continue is a deliberate divergence misattributed to the reference.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::ima` → `test result: ok. 5 passed` (exit 0).
- Committed fixture `adpcm_ima_mono.wav`: python3 port of this file → 12_246 frames, MD5 `8a8146087f7573fa71963c3ae2c65f3b`; sandbox ffmpeg → same MD5; repo differential (executed) → same.
- Fresh ffmpeg stereo encode (`adpcm_ima_wav`, 44.1 kHz, block_align 1024): port MD5 `f2e8673552515dbe541d19d212b29dce` == ffmpeg MD5 — stereo group interleave verified beyond the committed corpus.
- The pinned unit ladder [2314, 2301, 2331, 2268, 2132] reproduces under the multiply form in python (and differs from the partial-sum form — the test genuinely discriminates).
- `ffmpeg -i <committed fixture> -f md5 -` → pin match (see mod.rs audit §2).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/adpcm/ima.rs:46-49 (doc) vs 150-155 (code)
Code:
```
//! A block shorter than its 4·channels header is **block-skipping**
//! damage ([`ImaStats::skipped_blocks`]); a step index above 88 is
//! clamped to 88 with the concealment flagged ([`ImaStats::clamped_index`]
//! — ffmpeg's own healing, kept for reference parity); a short group
```
What is wrong: the attribution is false. ffmpeg 7.1.5's adpcm_ima_wav does NOT heal an out-of-range header step index — it refuses the frame: I built a one-block WAV with header index 200 and ran `ffmpeg -i ima_badidx.wav -f s16le -` → `ERROR: step_index[0] = 200` / `Invalid data found when processing input`, exit 69, zero output bytes. The code (clamp to 88 + `clamped_index` flag, decode continues) is a defensible concealment policy, but it is this repo's invention for this flavor, not reference parity; the doc misleads the next maintainer about what the oracle does.
Why it matters: anyone auditing "parity with ffmpeg" from the docs would wrongly conclude damaged-index behavior is pinned by the reference; it is unpinned (no differential covers damaged headers — the corpus is clean).
What correct looks like: doc says "deliberate divergence: ffmpeg errors on step_index > 88; we clamp and conceal (see damage model doc 03 §4.5(d))".
Confidence: HIGH — Verification: ran (ffmpeg probe above; code read).

F-002 [S4]
Where: crates/codecs/src/audio/adpcm/ima.rs:183-186 (tail-group "honest prefix")
Code:
```
            let per_group = (0..channels)
                .map(|ch| 2 * group_bytes.len().saturating_sub(4 * ch).min(4))
                .min()
                .unwrap_or(0);
```
What is wrong: nothing functionally (this is the documented short-tail concealment and it is panic-free), but note the behavior diverges from ffmpeg on damaged tails: ffmpeg emits only the block's seed sample for a short block ("invalid number of samples in packet", 2 output bytes in my probe) while this code emits seed + whole available nibble bytes. This divergence is documented (mod.rs:83-84) — recorded here so the next agent does not re-flag it as a bug.
Confidence: HIGH — Verification: ran (ffmpeg short-block probe).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TAG/STEP_TABLE/INDEX_TABLE consts | 53-68 | REAL | — | byte-diffed vs canonical: equal; 89 entries, [0]=7, [88]=32767 |
| header_bytes | 72-74 | REAL | — | 4·ch |
| ChannelState::expand | 83-100 | REAL | — | multiply form verified bit-exact vs ffmpeg (mono+stereo); predictor clamp ±32767; index clamp 0..88 |
| ImaStats / ImaFacts | 105-122 | REAL | — | data carriers |
| decode_ima_blocks | 128-199 | REAL | S3 (F-001 doc) | block loop `.chunks(max(1,ba))`; skip <header; groups 4·ch; tail prefix min-form correct; executes in tests + differential |
| push_sample | 203-205 | REAL | S4 | native-endian (LE-host assumption, shared with family) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| step_table_is_the_published_89_entry_ladder | YES | length + endpoints + monotonicity; a single wrong value in the middle passes monotonicity but fails the corpus MD5s — and I verified the values independently |
| mono_block_decodes_the_pinned_ladder | YES | exact 5-sample ladder, discriminates multiply vs partial-sum forms |
| header_index_above_88_clamps_and_flags | YES | pins the clamp policy (note: this is repo policy, not ffmpeg parity — F-001) |
| block_smaller_than_header_is_skipped_not_panicked | YES | skip count + empty out + zero-length no-op |
| stereo_group_interleave_keeps_channels_separate | YES | exact 4 sample values proving no cross-wiring of granules |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "89-entry step table" / values | ima.rs:55-62 | REAL | python diff vs canonical == equal |
| "The multiply folds `step>>3` in before the shift, so odd steps round differently" (multiply ≠ partial) | ima.rs:29-32 | REAL | step 7 nibble 7: 13 vs 11 (computed); ffmpeg output matches the multiply form (fixture + fresh-encode MD5s) |
| "decodes `1 + (block_align − 4·channels)·2 / channels` sample instants" | ima.rs:16 | REAL | 1024/mono → 2041/block ⇒ 12_246 total (executed); demuxer formula (format.rs:304-312) agrees |
| index >88 clamp is "ffmpeg's own healing, kept for reference parity" | ima.rs:47-49 | FALSE (attribution) | ffmpeg errors (exit 69, `ERROR: step_index[0] = 200`) — F-001 |
| "low nibble first" packing | ima.rs:3,15 | REAL | bit-exact MD5s would fail on nibble swap |
| raw DVI flavor is a typed gap | ima.rs:34-42 | REAL | mod.rs gate test executed (`adpcm.codec_descriptor` refusal) |

## 7. Dead / duplicated / unreachable code
- STEP_TABLE duplicated in swf.rs (byte-equal — checked); deliberate per-flavor isolation.
- `ImaStats::frames` is produced but unconsumed by mod.rs (only skipped/concealed mapped) — harmless telemetry.

## 8. Dependencies & flags
- std-only; no feature gates.

## 9. Open questions for the next agent
- None for this file. mod.rs F-001 (channels 3..=64 vs ffmpeg refusal) is the related open envelope question.
