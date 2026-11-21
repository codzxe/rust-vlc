# AUDIT: crates/codecs/src/video/av1/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 115 / 5 (2 production items + sink + 2 tests) |
| File verdict | SHALLOW (the harness's decode pass structurally cannot exercise inputs beyond a 255-byte prefix — its stated contract "one untrusted byte buffer through both surfaces" is false for surface 2) |
| Reachable from shipped entry point? | The harness itself: NO (a lib function called only by fuzz/fuzz_targets/fuzz_codec_av1.rs — not shipped — and its in-file tests). Its SUBJECT is shipped-reachable: `Av1Decoder` is wired at apps/cli/src/pipeline_host.rs:304-308 (`"av1" \| "av01"`), the IVF demuxer mints `"av1"` for the `AV01` fourcc (crates/demux/src/ivf/mod.rs:211), and `codec-av1` is a default feature (crates/codecs/Cargo.toml:50) — so `lumen open file.ivf` (AV1) reaches the code this harness is supposed to protect. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `sed -n` quote verification (56-58, 78-84); `cat fuzz/fuzz_targets/fuzz_codec_av1.rs`; `rg -n "fuzz_codec_av1" fuzz/ scripts/` (target exists + registered, fuzz/Cargo.toml:223-224, README:78); `rg -n "av1" audit/runs/baseline/test-workspace.log.txt` (fuzz tests :523,:571 ok; conformance/differential suites ran ok); `rg -n "av01" crates/demux/src/ivf/mod.rs` |

## 1. One-paragraph verdict
A two-pass AV1 fuzz harness: pass 1 drives the in-house OBU parser with the full input; pass 2 drives the rav1d contract path (configure → decode → flush) — except it does not. The packet length is `first_byte % (len+1)`, which for any input longer than 255 bytes caps the decoded prefix at 255 bytes: the entire tail — the bulk of a real temporal unit, where tile data and deep decoder state live — never reaches the decoder. The module doc claims "the whole buffer as one temporal unit", the fuzz-target shim claims "one full untrusted pass over the bytes", and the hostile-input test feeds 4096-byte buffers believing it exercises them; all three claims are gutted by the truncation. The shipped, P0, default-enabled AV1 decoder therefore has a fuzz safety net that is structurally blind to >255-byte hostile inputs. Pass 1 is honest; the in-file tests ran green in baseline; the separate conformance/differential suites (which do exercise real frames) also ran green — the differential work is what actually protects the decoder today, not this harness.

## 2. What actually works (verified)
— Both in-file tests `ok` in baseline run #3 (test-workspace.log.txt:523 `hostile_wide_inputs_survive`, :571 `empty_and_short_inputs_survive`).
— Pass 1 is a full-input walk: `super::obu::parse_temporal_unit(data)` receives the complete buffer (line 54, read).
— Slice safety: `cut ∈ [0, data.len()]` by construction (`% (len+1)`), so `data[..cut]` cannot panic; `to_vec` allocation ≤ input size — the stated input-derived memory bound holds (read).
— The fuzz target exists and is registered (fuzz/fuzz_targets/fuzz_codec_av1.rs, fuzz/Cargo.toml:223-224, fuzz/README.md:78) — the registry check passed in baseline run #18 (35 targets, diff empty).

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/video/av1/fuzz.rs:78-84 (with doc claims :4-11, 56-58; fuzz/fuzz_targets/fuzz_codec_av1.rs:6-7)
Code:
```rust
    let cut = usize::from(data.first().copied().unwrap_or(0)) % (data.len() + 1);
    let packet = Packet {
```
and
```rust
        data: media_types::packet::SharedBytes::from_vec(data[..cut].to_vec()),
```
What is wrong: `usize::from(u8)` is at most 255, so for every input with `len ≥ 255` the decode pass receives at most the first 255 bytes. A fuzzer growing inputs past 255 bytes (libFuzzer's default behavior) gains nothing on surface 2: the rav1d contract path only ever sees tiny prefixes. Panics requiring deeper input — mid-tile corruption, multi-OBU sequences, state built across a real-sized frame — are unreachable by this campaign.
Why it matters: the module doc states the contract as "one untrusted byte buffer through **both** surfaces" (:4-5) and "the whole buffer as one temporal unit" (:57-58) — both false for surface 2 on realistic inputs. The fuzz target repeats it ("one call = one full untrusted pass over the bytes"). The subject decoder is shipped (P0 matrix row `codec-video-av1`, status implemented on all three desktop platforms; wired at pipeline_host.rs:304-308; IVF lane mints the descriptor), so this is the safety net for production code with a structural hole and overstated evidence. The in-file "hostile wide" test's 4096-byte buffers decode ≤255-byte prefixes — its comment ("maximal OBU-header false positives") overstates what was exercised.
What correct looks like: derive the cut from a value that can exceed 255 (a hash of the whole buffer, or `usize::from(byte) * (len+1) / 256`), or feed the full buffer on a rotating subset of passes; fix the two comments to match reality.
Confidence: HIGH — Verification: read + arithmetic (`255 % 4097 = 255`; `cut ≤ 255` whenever `len ≥ 255`).

### F-002 [S4]
Where: crates/codecs/src/video/av1/fuzz.rs:42-47
Code:
```rust
impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}
```
What is wrong: the sink always returns `Ok(())` — the decoder's handling of a failing sink (backpressure/error propagation from `out.accept`) is never exercised by the harness.
Why it matters: the codec-api contract includes sink errors; a panic-on-Err path in the decoder would survive this fuzz campaign untouched.
What correct looks like: an input-derived sink that returns `Err` on some frames (e.g., every Nth), mirroring the counting sink otherwise.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/codecs/src/video/av1/fuzz.rs:85
Code:
```rust
        keyframe: true,
        discontinuity: false,
```
What is wrong: every fuzz packet is declared a keyframe with no discontinuity — the decoder's non-keyframe-entry and discontinuity-flush paths are never fuzzed through this harness.
Why it matters: AV1 inter frames reference prior state; a real stream's second packet is exactly the `keyframe: false` case. Narrow fuzz breadth on a shipped decoder.
What correct looks like: derive both flags from input bytes (like the cut), covering all four combinations.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-29 | SHALLOW | S2 | contract claim false for pass 2 (F-001); bounds claim holds |
| `CountingSink` + `FrameSink::accept` | 37-47 | REAL | S4 | F-002 always-Ok sink |
| `fuzz_codec_av1` | 52-92 | SHALLOW | S2 | F-001 255-byte decode cap; F-003 fixed flags; pass 1 full-input; slice-safe |
| tests: `empty_and_short_inputs_survive` | 98-106 | REAL | — | 34 inputs ≤ 32 bytes — inside the only size class the decode pass fully covers |
| tests: `hostile_wide_inputs_survive` | 108-114 | SHALLOW | — | 4096-byte buffers, decode pass sees ≤255 bytes (F-001) — comment overstates |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_short_inputs_survive | PANIC class only | survival contract; asserts nothing about outcomes (by design) |
| hostile_wide_inputs_survive | PANIC class only, and only on ≤255-byte prefixes | F-001 — the wide part of "wide" never reaches the decoder |
Both ran `ok` in baseline run #3 (:523, :571). The outcome-level protection comes from the separate suites: `av1_conformance` (1 test / 15 rows pixel-MD5, `ok`, 14.05s) and `av1_differential` (6 tests, `ok`) — both in the same baseline log (:880-895).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through **both** surfaces" | fuzz.rs :4-5 | FALSE | decode pass sees ≤255 bytes for len ≥ 255 (F-001) |
| "the whole buffer as one temporal unit — invalid as a TU or not" | fuzz.rs :56-58 | FALSE | `data[..cut]`, not the whole buffer |
| "one call = one full untrusted pass over the bytes" | fuzz/fuzz_targets/fuzz_codec_av1.rs:6-7 | FALSE | same truncation |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs :13-14 | REAL | read — outcomes swallowed by design |
| "Memory is bounded by the input" | fuzz.rs :25-29 | REAL | to_vec ≤ input; decode path trivially bounded by the 255-byte cap |
| matrix row `codec-video-av1` evidence: conformance 15 rows pixel-MD5 PASS | docs/feature-matrix.toml:468 | REAL | baseline av1_conformance `ok` (test-workspace.log.txt:880-889) — note: the row does NOT cite the fuzz harness, so no evidence-string fraud here |

## 7. Dead / duplicated / unreachable code
No dead code. The harness function is unshipped (fuzz-target + tests only) but its subject is shipped — the reason F-001 matters.

## 8. Dependencies & flags
Uses `codec-av1`-gated `Av1Decoder` (video/mod.rs:22-23); the fuzz module itself compiles under the same gate. `codec-av1` is default-on (Cargo.toml:50) — so the harness's code is compiled into shipped builds too (as uncalled pub fn). rav1d pinned asm-off (Cargo.toml:125), consistent with the module doc's "rav1d engine (asm-off)".

## 9. Open questions for the next agent
- av1/decoder.rs auditor: does the decoder hold cross-packet state that a >255-byte input could corrupt into a panic? If yes, F-001 upgrades (the harness is the only randomized defense for that class).
- fuzz/ wiring owner: the README says the nightly campaign is Lead-owned and not yet run ("until then this package compiles on stable") — meaning NO libFuzzer campaign has ever executed this harness; the only executions are the two in-file deterministic tests. Worth stating in the fuzz README's honesty table.
