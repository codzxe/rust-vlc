# AUDIT: crates/codecs/src/audio/pcm/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 191 / 10 (2 pub fn incl. `fuzz_codec_pcm`, `configurations`, `CountingSink`, 5 tests, 1 const) |
| File verdict | REAL (both advertised passes execute; contrast with the FLAC harness whose contract pass is dead) |
| Reachable from shipped entry point? | NO as shipped-binary code (fuzz harness); YES as the registered fuzz target body: `fuzz/fuzz_targets/fuzz_codec_pcm.rs:37` calls `codecs::audio::pcm::fuzz::fuzz_codec_pcm(data)` (verified). The libFuzzer campaign is not run in CI (baseline #18 checks registry/corpus only); in-crate smoke tests execute the harness (baseline #3, 1681 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `sed -n` on all quoted ranges; `sed -n '600,710p' crates/codecs/src/audio/pcm/decoder.rs` → `with_format` seeds (channels, rate), `configure` derives format from the descriptor and accepts the vocabulary; `rg -n "fn new" -A 12 decoder.rs:111` → `PcmPlan::new` refuses only zero channels/rate; `sed -n '227,295p' pcm/mod.rs` → `PcmSource::descriptor()` produces exactly the string configure parses (incl. `_planar` suffix); `rg -n "WAVEFORMATEX" crates/codecs/tests/pcm_fuzz_survival.rs` → the lying-WAVEFORMATEX surface IS covered by the survival suite (lines 29,50,139,637,766,812,872) |

## 1. One-paragraph verdict
This is the honest sibling of the FLAC harness: both advertised passes genuinely execute. Pass 1 drives the free `decode_pcm` entry across a deterministic 3-4-config spread (fixture s16le stereo, a data-derived format/planar/channel combo, an odd-width planar S24Be 3ch stress shape, an F32Be mono shape); Pass 2 drives the full codec-api contract cycle per config, and — unlike FLAC — it provably gets past `configure`: the descriptor string the harness builds is exactly what `configure` parses (`PcmSource::descriptor()`/`strip_suffix("_planar")` verified on both sides), `with_format(channels, rate)` seeds nonzero facts, and `PcmPlan::new` refuses only zero values the harness never produces. Bounds are input-derived as documented. The three findings are hygiene: a stale wiring-status paragraph (the fuzz scaffold landed with card W1-12), the harness never feeds `extra_data`/WAVEFORMATEX bytes (that file-controlled surface is exercised only by the deterministic survival suite, not by the fuzz entry the future campaign will run), and the "adds no buffers" bounds sentence is literally false (up to four input-sized packet clones per call, still input-bounded).

## 2. What actually works (verified)
— Pass 2 liveness (the FLAC failure mode ruled out): traced end to end — `source.descriptor()` (mod.rs:241-247) → `CodecDescriptor::new` → `configure` strips `_planar` (decoder.rs:683-686) → `PcmFormat::from_descriptor(base)` → `resolve_facts((Some(seeded), None)) → Ok(seeded)` (decoder.rs:648-651) → `PcmPlan::new` accepts (only zero channels/rate refuse, decoder.rs:111-123; the harness derives channels 1..=8 at line 78 and rates from `RATES`). The decode→flush tail runs for every config.
— Configuration-spread determinism and bounds: `configurations` (64-94) produces 3-4 configs, fixture-first, deduplicated; pinned by the test at 178-190. `PcmFormat::ALL` indexing is modulo-`len` (76); `channels = 1 + (b >> 1) % 8` ∈ 1..=8 (78).
— Smoke tests execute the harness on empty, sub-frame (1..=9 bytes — the dropped-tail path), fixture-shaped, dangling-tail, 0xFF-flood and 0x00-flood inputs; baseline #3 (`cargo test --workspace`, 1681 passed) includes them.

## 3. Findings

### F-001 [S4] Stale "Wiring status (honest)" paragraph — the scaffolding it says is missing has landed
Where: crates/codecs/src/audio/pcm/fuzz.rs:14-26
Code:
```rust
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_codec_pcm.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys`
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5), so until
//! they land the harness logic is proven by the in-crate
```
What is wrong: same drift as the FLAC harness: `fuzz/Cargo.toml` (self-described "delivered by card W1-12"), `fuzz/fuzz_targets/fuzz_codec_pcm.rs` (working `fuzz_target!` shim) and the seed corpus all exist; only the nightly-toolblock/CI-wiring half of the paragraph is still true. The paragraph's "until they land" describes a superseded state.
Why it matters: a doc section headed "(honest)" that misstates the present is exactly the trust-erosion this audit tracks.
What correct looks like: mirror fuzz/README.md's current status (scaffold landed; campaign requires nightly, unwired in CI).
Confidence: HIGH — Verification: read + ls fuzz/fuzz_targets + cat of the shim.

### F-002 [S4] The fuzz entry never feeds `extra_data` — the WAVEFORMATEX facts channel is outside the future campaign's surface
Where: crates/codecs/src/audio/pcm/fuzz.rs:117-124
Code:
```rust
        let Ok(mut decoder) = PcmDecoder::with_format(source.channels, rate) else {
            continue;
        };
        let config =
            DecoderConfig::new(descriptor, generation, ThreadBudget::default(), None, None);
```
What is wrong: every pass-2 configuration supplies `extra_data: None`, so `parse_waveformatex` and the `pcm.stream_format_conflict` cross-check (decoder.rs:648-671) — the decoder's one surface that consumes raw FILE bytes in the shipped pipeline (WAV/AVI `fmt`/`strf` blobs) — are never reached by `fuzz_codec_pcm`, the single entry the cargo-fuzz campaign will run once wired. Coverage exists today only via the deterministic survival suite (`crates/codecs/tests/pcm_fuzz_survival.rs`: lying-WAVEFORMATEX cases at its lines 29/50/139/637/766/812/872), which is a fixed set of hand-built mutants, not a fuzzer.
Why it matters: when the campaign lands, a panic in the WAVEFORMATEX parse would be invisible to it while the module doc's "both decode surfaces" framing implies the contract path is fuzzed as a whole.
What correct looks like: add a fifth configuration arm that builds a data-derived 16..40-byte `extra_data` blob alongside the seeded facts (exercising both the parse and the conflict gate).
Confidence: HIGH — Verification: read + survival-suite grep.

### F-003 [S4] "this harness adds no buffers" is false — up to four input-sized clones per call
Where: crates/codecs/src/audio/pcm/fuzz.rs:30-32 vs 130
Code:
```rust
//! Memory is bounded by the input itself: every PCM conversion is a
//! width- and order-preserving permutation of the packet's own bytes
//! (output ≤ input; see the decoder module docs) — this harness adds
//! no buffers.
```
and line 130:
```rust
            data: media_types::packet::SharedBytes::from_vec(data.to_vec()),
```
What is wrong: pass 2 clones the entire input once per configuration (3-4 clones per call). The MP §12 substance (memory bounded by the input, constant factor) holds, but the literal sentence "adds no buffers" is wrong by a factor of ≤4×input.
Why it matters: bounds-section precision; a future reader scaling the campaign (libFuzzer `-len_control`, RSS limits) will trust the sentence.
What correct looks like: "adds only O(configurations) clones of the input (≤4×)".
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RATES` const | 45 | REAL | — | 3 legal rates, includes the fixture's 44100. |
| `CountingSink` + `impl FrameSink` | 49-59 | REAL | — | Saturating count; live target (unlike FLAC's dead sink). |
| `configurations` | 64-94 | REAL | — | Deterministic 3-4-config spread; fixture-first; dedup vs configs[0]; modulo indexing safe; pinned by test 178. |
| `fuzz_codec_pcm` | 99-139 | REAL | S4 (F-002, F-003) | Pass 1: free entry × spread (104). Pass 2: full contract cycle × spread — verified live through configure (see §2); `CodecDescriptor::new`/`with_format`/`configure` failures `continue` (typed-refusal-safe); generation from a constant (no exhaustion); outcomes swallowed per the survival contract. |
| 5 tests | 145-190 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_input_survives_both_passes` (146) | YES (panic only) | Empty buffer through both passes; survival contract is honest about scope (lines 8-12). |
| `sub_frame_lengths_survive_both_passes` (151) | YES (panic only) | 1..=9 bytes — hits the whole-sample-frame-prefix/dropped-tail paths for every width including S24Be planar 3ch (4-byte frame... 9-byte cut). |
| `fixture_shaped_buffer_survives_both_passes` (158) | YES (panic only) | 5880-byte AVI chunk shape + dangling-tail variant. |
| `hostile_wide_input_survives_both_passes` (170) | YES (panic only) | 0xFF/0x00 floods. |
| `configurations_spread_is_deterministic_and_bounded` (178) | PARTIAL | Real teeth for spread size/order/determinism (would fail if the spread regressed to 1 config or reordered); does NOT assert channel/rate legality of the derived config — a regression emitting an invalid derived config (e.g. 0 channels) would silently skip that config's decode in both passes without failing anything. |
Mutation check (reasoned): a panic introduced in `decode_pcm`, `PcmDecoder::decode`, or `flush` fails all four survival tests; a panic in `parse_waveformatex` fails NOTHING in this file (F-002) — it would have to be caught by pcm_fuzz_survival.rs's fixed mutants or the decoder's unit tests.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through both decode surfaces … the codec-api Decoder contract path (configure → packet → sink → flush)" | module doc 4-8 | REAL | Verified live: configure provably accepts every harness config (§2 trace); contrast FLAC (S2 there). |
| "Building it needs the libfuzzer-sys dev-dependency plus the fuzz/Cargo.toml scaffolding — both Lead-owned … so until they land" | module doc 18-21 | FALSE (stale) | fuzz/Cargo.toml + target + corpus landed (card W1-12); only nightly/CI wiring is outstanding. |
| "this harness adds no buffers" | module doc 30-32 | FALSE (literally) | F-003: ≤4 input-sized clones per call; bounded-by-input substance holds. |
| "Runtime is structurally bounded: one pass over the bytes per configuration, a fixed number of configurations per call" | module doc 33-36 | REAL | Spread is 3-4 configs (pinned by test 178); each pass is linear. |

## 7. Dead / duplicated / unreachable code
- None dead. `CountingSink` is live (used at 135-137).
- Duplication: the `CountingSink` type is byte-identical in spirit to flac/fuzz.rs:50-60 (and likely the other codec harnesses) — per-harness copy-paste; a shared testkit sink would remove the Nth copy. S4-class; noted, not counted again.

## 8. Dependencies & flags
- No feature gate (`pub mod pcm` unconditional in audio/mod.rs:61); std-only; depends on codec-api/media-types/super. The fuzz package is separate (own workspace root, nightly-only to run).

## 9. Open questions for the next agent
- pcm/decoder.rs owners (W07-A05/A04): is `parse_waveformatex` reachable with fully-arbitrary bytes in any fuzz target today? If not, F-002 is the campaign's biggest hole — one data-derived `extra_data` arm closes it.
- CI/fuzz wave: the registry gate (baseline #18) proves name/corpus presence, not pass liveness — the FLAC S2 shows why a liveness canary per harness is needed.
