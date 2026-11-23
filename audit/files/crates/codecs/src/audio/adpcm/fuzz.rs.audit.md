# AUDIT: crates/codecs/src/audio/adpcm/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 159 / 1 prod fn (`fuzz_codec_adpcm`) + `raw_facts_skeleton` + `CountingSink` + `FLAVORS` const; 3 tests |
| File verdict | REAL (8-flavor contract-path harness with sound skeleton synthesis) — but with the weakest evidence in the family: the survival suite never drives this harness, and the harness returns nothing observable |
| Reachable from shipped entry point? | NO (by design: fuzz infrastructure). Reachable from `fuzz/fuzz_targets/fuzz_codec_adpcm.rs` (verified: `fuzz_target!(\|data: &[u8]\| { codecs::audio::adpcm::fuzz::fuzz_codec_adpcm(data); });`) and the 3 in-file unit tests. The `AdpcmDecoder` it drives IS shipped (apps/cli/src/pipeline_host.rs:300 `codec.starts_with("adpcm_")`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p codecs --test adpcm_fuzz_survival --offline` -> BLOCKED (linker `ld terminated with signal 7 [Bus error]` — disk 100% full, 19M free; environment limit, not a repo defect). Cited instead: baseline `audit/runs/baseline/cargo-test-workspace.txt:937-943` — `Running tests/adpcm_fuzz_survival.rs` … `empty_and_tiny_payloads_are_honest ... ok`, `mutated_fmt_headers_fail_typed_not_panicking ... ok`, `unmutated_substrate_decodes_deterministically ... ok`, `mutated_blocks_survive_with_typed_outcomes ... ok`. In-file unit tests: baseline lines 390-393 (`empty_and_short_inputs_survive`, `raw_facts_skeleton_stays_legal`, `hostile_wide_inputs_survive` all ok). sed verifications. |

## 1. One-paragraph verdict
The harness design is sound: each call cycles all 8 ADPCM descriptors through configure → packet → sink → flush, feeding the WAV flavors (`adpcm_ms`, `adpcm_ima_wav`) the raw hostile bytes as `extra_data` and the raw breadth flavors a synthesized 16-byte WAVEFORMATEX skeleton whose tag/channels/rate I verified against `parse_raw_facts` (adpcm/mod.rs:698-737: ≥16 bytes, tag 0x0000, 1..=2 channels, rate ≠ 0 — the skeleton always passes, so those flavors' decode paths really run), with `adpcm_oki` as the expected typed refusal. Unlike the mp2 sibling's stale wiring note, the "Wiring status (honest)" here is accurate (target exists, nightly wiring Lead-owned). Two real weaknesses: the committed survival suite `tests/adpcm_fuzz_survival.rs` never calls `fuzz_codec_adpcm` — it imports only `AdpcmDecoder` (line 21) and drives its own mutation loop — so the harness's flavor-cycle and skeleton logic is exercised only by 3 shallow unit tests over ~35 benign inputs, a fraction of the mp2 sibling's 120 mutants through its harness; and the harness returns `()` — no outcome digest — so nothing can distinguish "all 8 flavors decoded" from "all 8 refused at configure" without instrumenting the sink, and a silent total-refusal regression would pass every existing test.

## 2. What actually works (verified)
— Baseline (cited, disk-full prevented my run): `audit/runs/baseline/cargo-test-workspace.txt:940-943` — all 4 survival tests ok; `:390-393` — this file's 3 unit tests ok.
— Verified by read: the skeleton satisfies `parse_raw_facts` for every input (`out[0..2]` = tag 0 = `RAW_TAG` mod.rs:123; channels `1 + a%2` ∈ {1,2}; rate from a 5-entry table of real clocks, never 0) — the raw flavors' configure succeeds and their decode surfaces execute.
— Verified by read: `fuzz/fuzz_targets/fuzz_codec_adpcm.rs` exists and calls this harness (cat, quoted in reachability row).
— Bounds claim "≤ 8 output bytes per input byte" is conservative, not false: the decoder's own bound is ~4 output bytes per input byte plus block-header rounding (`wav_out_bound`, adpcm/mod.rs:739-743: "every nibble byte yields two samples … plus the 2 seed samples per block").

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/adpcm/fuzz.rs:22-23, 29-30; crates/codecs/tests/adpcm_fuzz_survival.rs:21
Code (fuzz.rs:22-23):
```
//! typed error and is swallowed here; the AC suites pin the specific
//! outcomes.
```
Code (fuzz.rs:29-30):
```
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's breadth differential suite.
```
Code (adpcm_fuzz_survival.rs:21):
```
use codecs::audio::adpcm::AdpcmDecoder;
```
What is wrong: the "AC suites" that pin outcomes never touch this harness. `tests/adpcm_fuzz_survival.rs` imports only `AdpcmDecoder` (line 21) and runs its own LCG mutation loop against the decoder directly — `fuzz_codec_adpcm` appears nowhere in the file (rg-verified). The mp2 sibling's survival suite explicitly drives its harness ("The registered-target logic itself (fuzz_codec_mp2) over the …" — mp2_fuzz_survival.rs:142, 120 mutants through `fuzz_mp2_stream`). Consequently the adpcm harness's flavor cycle, its `index < 2` extra-data split, and the skeleton synthesis are executed only by the 3 in-file unit tests over ~35 inputs (empty, 4-byte, 1..=32-byte ramps, two 4096-byte constant fills). The doc sentence is literally defensible ("the deterministic sweeps below" = the unit tests) but reads as parity with the sibling that does not exist. What correct looks like: add one survival test driving `fuzz_codec_adpcm` over the mutated fixtures, as mp2 does. Confidence: HIGH — Verification: read (rg for `fuzz_codec_adpcm` in tests/: zero hits); baseline run cited for the tests that do exist.
F-002 [S4]
Where: crates/codecs/src/audio/adpcm/fuzz.rs:87, 124-125
Code (87):
```
pub fn fuzz_codec_adpcm(data: &[u8]) {
```
Code (124-125):
```
        let _ = decoder.decode(packet, &mut sink);
        let _ = decoder.flush(&mut sink);
```
What is wrong: the harness returns unit — no `FuzzOutcome`-style digest (the mp2 harness returns frames/decoded/contract-frames flags its suite asserts). With `let _ =` on both decode and flush, a regression that made every flavor refuse at configure — e.g. a `parse_raw_facts` or descriptor gate change — would keep every existing test green (the unit tests assert only absence of panic). The harness cannot fail observably except by panicking. What correct looks like: return a small outcome struct (flavors configured, frames emitted) as the mp2 harness does, and assert it in the survival suite. Confidence: HIGH — Verification: read (mutation reasoning; no execution — cargo blocked by disk-full).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_codec_adpcm` | 87-127 | REAL | S4 | 8-flavor cycle, correct extra-data split, bounded work; zero observability (F-002), weak suite coverage (F-001) |
| `raw_facts_skeleton` | 73-82 | REAL | — | Always-legal WAVEFORMATEX floor; only first 2 input bytes influence geometry (bounded determinism, fine for fuzz) |
| `CountingSink` | 58-68 | REAL | — | Saturating counter; never read by any assertion |
| `FLAVORS` | 46-55 | REAL | — | 8 descriptors incl. the deliberate refusal flavor |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_short_inputs_survive | NO (only panics) | Calls the harness on ~35 inputs, asserts nothing — a configure-always-fails regression passes |
| hostile_wide_inputs_survive | NO (only panics) | Two 4096-byte constant fills, asserts nothing |
| raw_facts_skeleton_stays_legal | YES (narrow) | Asserts skeleton channels/rate legality — would catch a skeleton-synthesis bug, not a harness/decoder bug |
| (external) adpcm_fuzz_survival.rs 4 tests | YES (for the decoder) | Real LCG mutations with typed-outcome assertions — but they bypass this harness entirely (F-001); baseline: all ok |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through the codec-api contract path for every accepted flavor" | fuzz.rs:4-8 | REAL | Read: the FLAVORS cycle; skeleton verified legal against parse_raw_facts |
| "the AC suites pin the specific outcomes" | fuzz.rs:22-23 | SHALLOW | F-001: the AC survival suite pins decoder outcomes, not harness outcomes; the harness has none to pin |
| "The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_adpcm.rs` (Lead-owned nightly wiring …); the harness logic is meanwhile proven on stable by the deterministic sweeps below" | fuzz.rs:27-30 | REAL (accurate, unlike mp2's stale note) | Target file exists and calls the harness (cat); "sweeps below" = the 3 unit tests — weak but truthfully scoped |
| "Memory is bounded by the input: … ≤ 8 output bytes per input byte" | fuzz.rs:34-35 | REAL (conservative) | Decoder's own bound is ~4×+rounding (wav_out_bound, mod.rs:739-743) |

## 7. Dead / duplicated / unreachable code
- Module unreachable from shipped entry by design but compiled into the release library ungated (`pub mod fuzz`, adpcm/mod.rs:95) — same shipped-dead-code pattern as mp2's fuzz module.
- `CountingSink.frames` is written and never read (no assertion anywhere uses it).

## 8. Dependencies & flags
- No new deps; the ADPCM flavors are self-built (no feature gate needed); `fuzz/` scaffold carries libfuzzer-sys 0.4.13 for the shim, not built by any gate.

## 9. Open questions for the next agent
- The disk hit 100% (19M free) during this audit — the orchestrator should clear `target/` before the next wave or more baseline-citing will be needed.
- Does the `adpcm_ima` (headerless DVI/raw) gap noted at fuzz.rs:44-45 have a tracking card, or is a real-world flavor simply unimplemented? The decoder module documents it — whoever audits adpcm/mod.rs should confirm the gap is registered, not hidden.
