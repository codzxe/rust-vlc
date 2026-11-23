# AUDIT: crates/codecs/src/audio/adpcm/misc.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 413 / 9 prod + 10 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs dispatches `codec if codec.starts_with("adpcm_")` → codecs adpcm mod.rs:518 `"adpcm_ima_qt" => Plan::ImaQt` / :630 `misc::decode_ima_ws(...)` (this file). MOV/M4A and raw-AUD lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 5 |
| Commands run | `cargo test -p codecs --lib adpcm::misc` → 10 passed; `cargo test -p codecs --test adpcm_breadth_differential` → 4 passed in 1.39s (real work: `breadth_corpus_is_bit_exact_against_ffmpeg_reference` green over committed fixtures); `ffmpeg -version` → 7.1.5-0+deb13u1 (matches the doc's claimed oracle version exactly). |

## 1. One-paragraph verdict
A tight, correct pair of IMA decoders whose central risk — the QuickTime 9-bit seed predictor reconstruction (`sign_extend(16) & !0x7F`, i.e. signed-9-bit × 128) and ffmpeg's cross-chunk continuation heuristic — is pinned BIT-EXACT against a real ffmpeg 7.1.5 encoder-produced MOV fixture (173 × 68-byte stereo chunks, reference MD5 `a2cb46ce...`) and against ffmpeg's forced decoder for the hand-crafted Westwood mono/stereo vectors; the differential ran green here in 1.39 s, and the sandbox ffmpeg version matches the doc's oracle claim verbatim. The IMA math is bounds-safe by construction (nibble-indexed 16-entry table, step index clamped 0..=88 before every 89-entry lookup, predictor clamped to i16 range), and the two reconstruction forms (partial-sum vs multiply) are both implemented and pinned to disagree exactly where the spec says they do (test asserts 11 vs 13 on the same nibble). All findings are hygiene-grade: state mutation before a whole-packet concealment skip, a defensive channels=0 branch with lying stats, native-endian output that contradicts convert.rs's little-endian assumption, and two per-packet heap allocations.

## 2. What actually works (verified)
— `cargo test -p codecs --lib adpcm::misc` → 10/10 passed (seed-word split, continuation keep/re-seed, wild-index skip, stereo interleave, WS mono/stereo packing, cross-call state, odd-payload tail).
— `cargo test -p codecs --test adpcm_breadth_differential` → 4/4 passed, incl. `breadth_corpus_is_bit_exact_against_ffmpeg_reference` — this file's two flavors decode the committed ffmpeg-oracle fixtures to the pinned PCM MD5s (IMA4: 173 real encoder chunks — a ×128 seed-scale error or a wrong heuristic direction could not pass this lane).
— Mutation reasoning: the differential compares content MD5s over committed bytes; a broken predictor/index update changes the MD5 and fails — the test is not vacuous.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/adpcm/misc.rs:171-186
Code:
```rust
        if state[ch].step_index == step_index {
            let diff = (predictor - state[ch].predictor).abs();
            if diff > 0x7F {
                state[ch].step_index = step_index;
                state[ch].predictor = predictor;
            }
        } else {
            state[ch].step_index = step_index;
            state[ch].predictor = predictor;
        }
        if state[ch].step_index > MAX_STEP_INDEX {
```
What is wrong: on a multi-channel packet where channel N's header index is > 88, the whole packet is skipped — but channels 0..N-1 have ALREADY had their state adopted/re-seeded AND fully evolved through their 64 nibbles, while the packet's output is discarded. Those channels' running state is now 64 samples ahead of anything ever emitted; the next packet decodes from a desynced predictor (persistent per-channel glitch until a re-seed header happens to land). Corrupt-input-only, drift not crash, and ffmpeg's own behavior (hard frame error) is no gold standard here — but the skip path leaves inconsistent state where a rollback (or skip-per-channel) would be defensible.
Confidence: HIGH — Verification: read; not executed (would need a crafted 2ch stream with a bad ch1 header).

F-002 [S4]
Where: crates/codecs/src/audio/adpcm/misc.rs:217
Code:
```rust
    let pairs = data.len() / channels.max(1);
```
What is wrong: the channels=0 defensive branch lies: `pairs` becomes `data.len()`, every pair iterates zero channels but still does `stats.frames += 2`, so the stats claim 2×len decoded frames while `out` stays empty — any caller advancing pts by `stats.frames` would desync. Unreachable through the Decoder contract (mod.rs parse_raw_facts bounds channels to 1..=2, verified at adpcm/mod.rs:721-729), pub(crate) misuse hazard only.
Confidence: HIGH — Verification: read + caller's bounds check read.

F-003 [S4]
Where: crates/codecs/src/audio/adpcm/misc.rs:195, 228, 231
Code:
```rust
            out.extend_from_slice(&chunk[instant].to_ne_bytes());
```
What is wrong: output is native-endian i16 (`to_ne_bytes`), consistent with media-types' "native endian" doc but INCONSISTENT with audio-engine convert.rs, which decodes S16 strictly little-endian (`from_le_bytes`). Identical on every shipped (LE) target; the two modules encode opposite assumptions about the same `SampleFormat::S16` contract. Cross-file hygiene defect (see convert.rs F-003).
Confidence: HIGH — Verification: read of both files.

F-004 [S4]
Where: crates/codecs/src/audio/adpcm/misc.rs:162, 218-219
Code:
```rust
    let mut scratch = vec![[0i16; 64]; channels];
```
(and `let mut lows = vec![0i16; channels]; let mut highs = vec![0i16; channels];`)
What is wrong: per-packet heap allocations in the decode hot path (one 64×ch scratch for IMA4, two ch-length vecs for WS) — bounded and tiny, but the sibling flavors (yamaha/oki/swf, per mod.rs) take caller-provided state/buffers; these could live in the decoder struct like `ima_state` does.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/codecs/src/audio/adpcm/misc.rs:155-157
Code:
```rust
    if data.len() < chunk_total {
        stats.skipped_packets = u32::from(!data.is_empty());
        return stats;
    }
```
What is wrong: an empty packet is not counted as skipped (deliberate: no bytes, no crime) but a 1..33-byte packet is — the asymmetry is undocumented in the "Damage model" doc block (:52-56), which says "shorter than 34·channels is skipped" without the empty-packet exemption. Doc precision nit.
Confidence: HIGH — Verification: read; empty-packet shape untested (no test feeds `&[]`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `STEP_TABLE` | 61-67 | REAL | — | 89 entries; deliberate duplication documented (:58-60) |
| `INDEX_TABLE` | 70-73 | REAL | — | ffmpeg's 16-entry table |
| `qt_expand` | 84-105 | REAL | — | partial-sum form; pinned vs multiply form by test; bit-exact vs ffmpeg via breadth lane |
| `ws_expand` | 109-122 | REAL | — | multiply form; same |
| `WsChannel` | 127-132 | REAL | — | shared state carrier |
| `MiscStats` | 136-141 | REAL | S4 | frames lies on channels=0 (F-002) |
| `decode_ima_qt_packet` | 147-200 | REAL | S4 | seed+heuristic bit-exact vs ffmpeg; F-001 state-rollback gap, F-004 alloc |
| `decode_ima_ws` | 206-236 | REAL | S4 | packing verified vs ffmpeg forced decoder; F-002/F-003/F-004 |
| `SignExtend` | 240-252 | REAL | — | correct arithmetic-shift semantics, edge bits 0/32 guarded, tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sign_extend_helper_matches_c_semantics | YES | 4 boundary pins |
| partial_and_multiply_forms_disagree_on_odd_steps | YES | pins 11 vs 13 — would catch a form swap |
| qt_header_word_splits_predictor_and_index | YES | pins seed adoption on first output sample |
| qt_continuation_heuristic_keeps_running_state | YES | pins keep AND re-seed branches with concrete predictors |
| qt_short_packet_and_wild_index_are_skipped | YES | both skip shapes |
| qt_stereo_chunks_are_per_channel_then_interleaved | YES | interleave order pinned sample-by-sample |
| ws_mono_is_two_samples_per_byte_low_first | YES | nibble order + values pinned |
| ws_stereo_packs_one_byte_per_channel_per_pair | YES | 4-sample pin incl. step-table walk to 16 |
| ws_state_persists_across_calls | YES | cross-call predictor equality |
| ws_trailing_byte_of_an_odd_stereo_payload_is_left_unconsumed | YES | odd-tail shape |
| (integration) breadth_corpus_is_bit_exact_against_ffmpeg_reference | YES | MD5 over committed ffmpeg-oracle fixtures; ran green here |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "be16 = (predictor << 7) \| step_index ... top 9 bits / low 7" | :11-13 | REAL | code :167-168 + bit-exact ffmpeg differential over 173 chunks |
| "ffmpeg applies a continuation heuristic ... within 0x7F ... running state kept" | :15-19 | REAL | code :171-180 + breadth differential green (ffmpeg C source not on box — UNVERIFIED directly, but 173-chunk bit-exactness would fail under a wrong heuristic direction) |
| "Oracle: ffmpeg 7.1.5 ... MOV-carried ... bit-exact" | :25-28 | REAL | breadth test executed; sandbox ffmpeg is 7.1.5-0+deb13u1, matching the doc |
| "no container in ffmpeg 7.1.5 muxes adpcm_ima_ws ... verified bit-exact against ffmpeg's forced decoder" | :44-48 | REAL-UNPROVEN | ws lanes pass against pinned MD5s (doc says derived from forced-decoder session; I did not re-derive the MD5s — UNVERIFIED executably) |
| "An IMA4 packet shorter than 34·channels is skipped whole" | :52-54 | SHALLOW | true except the undocumented empty-packet exemption (F-005) |
| "The Westwood stream has no headers to damage — every byte decodes" | :56 | SHALLOW | odd stereo payloads leave a trailing byte unconsumed (tested, documented in test names but contradicts "every byte" literally) |
| "the vqa3 shape is a documented gap" | :40-42 | REAL | honest gap, no code pretends otherwise |

## 7. Dead / duplicated / unreachable code
None dead. STEP_TABLE/INDEX_TABLE duplicated across flavor modules deliberately (documented self-containment policy, :58-60).

## 8. Dependencies & flags
None (std only). No unsafe. No features.

## 9. Open questions for the next agent
- adpcm/mod.rs (another agent's file): how is `MiscStats.skipped_packets` surfaced — does the IMAQt skip emit a `Recovered` outcome per packet, and does the ch0-state desync of F-001 self-heal on the next header re-seed in practice?
- The WAV-flavor IMA (ima.rs) shares STEP_TABLE — the partial-sum/multiply split documented here suggests ima.rs uses one of them; consistency check belongs to that file's auditor.
