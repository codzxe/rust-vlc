# AUDIT: crates/codecs/src/audio/adpcm/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 159 / 9 (1 pub harness fn, `raw_facts_skeleton`, `CountingSink`, `FLAVORS` const, 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO as shipped-binary code (fuzz harness). YES as the registered fuzz target body: `fuzz/fuzz_targets/fuzz_codec_adpcm.rs:19` calls `codecs::audio::adpcm::fuzz::fuzz_codec_adpcm(data)` (verified). The ADPCM lanes themselves ARE shipped-reachable (`pipeline_host.rs:298-302` `codec.starts_with("adpcm_")` → seeded `AdpcmDecoder`). In-crate smoke tests execute the harness (baseline #3, 1681 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` on all quoted ranges; `rg -n "fn accepts" -A 8 adpcm/mod.rs` → `adpcm_oki` IS in accepts (line 295) so the dedicated `adpcm.oki.no_reference` refusal (mod.rs:481-491) is reachable; `rg -n "parse_raw_facts" adpcm/mod.rs` + `sed -n '693,735p'` → 16-byte tag-0 skeleton is the exact accepted shape; `sed -n '511,525p'` → raw flavors route through `parse_raw_facts`; `rg -n "fuzz_codec_adpcm" crates/codecs/tests/adpcm_fuzz_survival.rs` → NOT called there (standalone mutation suite); `sed -n '1302p;1372p' adpcm/mod.rs` → positive configure tests exist for swf/ima_ws |

## 1. One-paragraph verdict
The best-behaved of the three codec fuzz harnesses I audited this wave: the contract path is unconditionally live for the five raw flavors (the synthesized 16-byte tag-0 skeleton is provably `parse_raw_facts`'s exact accepted shape, so `configure` succeeds and decode/flush run on every input), conditionally live for the two WAV flavors (raw input bytes as the `fmt` blob — the fuzzer must earn a valid tag/block-align, which is correct fuzzing), and the eighth descriptor is the documented honest refusal (`adpcm.oki.no_reference`, verified reachable through `accepts`). The wiring-status paragraph is the only one of the three harnesses that is current (target exists; nightly campaign unwired). What is thin: the module's own tests are pure survival with no liveness canary (a configure regression would silently no-op the harness here, caught only transitively by adpcm/mod.rs's own configure tests), and the smoke inputs skip the damaged-tail/fixture-shaped cases the sibling harnesses carry; the standalone `adpcm_fuzz_survival.rs` mutation suite does not call this entry at all (unlike flac/pcm where the survival suites route through the fuzz fn).

## 2. What actually works (verified)
— Contract-path liveness for raw flavors: traced `raw_facts_skeleton` (73-82: tag 0 at bytes 0-1, channels `1 + a%2` ∈ 1..=2 at 2-3, table rate at 4-7) against `parse_raw_facts` (mod.rs:698-735: ≥16 bytes, tag == RAW_TAG == 0, channels/rate checks the skeleton always satisfies) → `configure` succeeds for `adpcm_yamaha`/`adpcm_ima_oki`/`adpcm_swf`/`adpcm_ima_qt`/`adpcm_ima_ws` on every input; decode+flush run.
— The `adpcm_oki` refusal claim: `accepts` includes it (mod.rs:295) → the dedicated gate at mod.rs:481-491 fires with `adpcm.oki.no_reference` — the module doc's "the configure error is the expected outcome" is exact, and the harness swallows it via `continue`.
— Skeleton legality pinned by a real test (150-157: channels 1..=2, rate 8000..=48000 decoded back out of the bytes).
— Baseline #3 (`cargo test --workspace`, 1681 passed) executes the smoke tests; the shipped ADPCM lanes are compiled (adpcm unconditional, audio/mod.rs:37) and dispatched (pipeline_host.rs:298).

## 3. Findings

### F-001 [S4] No liveness canary: the module's tests cannot distinguish "harness ran" from "harness no-op'd"
Where: crates/codecs/src/audio/adpcm/fuzz.rs:129-158 (test module) vs 109-112
Code:
```rust
        if decoder
            .configure(&DecoderConfig::new(
                descriptor,
                generation,
                ThreadBudget::default(),
                None,
                Some(extra),
            ))
            .is_err()
        {
            continue;
        }
```
What is wrong: every `continue` path (descriptor, generation, configure refusal) is indistinguishable from a full pass under the survival contract. If `parse_raw_facts` or `configure` regressed to refusing the skeleton, `fuzz_codec_adpcm` would become an eight-iteration no-op and all three smoke tests would still pass. Mitigation exists but is indirect: adpcm/mod.rs's own tests positively configure raw flavors (`configured(&mut decoder, "adpcm_swf", Some(raw_facts(2, 22_050)))` at mod.rs:1302; `"adpcm_ima_ws"` at 1372), so the regression would be caught there, not here — the fuzz module itself never asserts any pass completed.
Why it matters: the flac harness (this wave, S2) proves this failure mode is real, not hypothetical; a one-line canary would make the class impossible.
What correct looks like: one test asserting `configure` succeeds for at least one raw flavor with the skeleton (or exposing a tiny `fn pass_reached() -> bool` debug hook).
Confidence: HIGH — Verification: read + mod.rs test grep.

### F-002 [S4] Smoke coverage thinner than the sibling harnesses; the mutation suite bypasses this entry
Where: crates/codecs/src/audio/adpcm/fuzz.rs:133-147 and crates/codecs/tests/adpcm_fuzz_survival.rs
Code:
```rust
    #[test]
    fn empty_and_short_inputs_survive() {
        fuzz_codec_adpcm(&[]);
        fuzz_codec_adpcm(&[0x00, 0x00, 0x01, 0x00]);
        for length in 1..=32usize {
```
What is wrong: (a) no fixture-shaped or damaged-tail/dangling-tail input (pcm's harness has both; flac's has marker+sync-flood shapes) — the ADPCM block-tail truncation paths (`adpcm.*` block geometry) are never exercised through this entry on stable; (b) `adpcm_fuzz_survival.rs` is a standalone fmt/block mutation suite that never calls `fuzz_codec_adpcm` (verified by grep), so unlike flac/pcm the fuzz entry's only stable executions are these 3 tests.
Why it matters: the entry the campaign will run is the least-exercised entry of the three harnesses; its shape coverage on stable is 34 fixed buffers.
What correct looks like: route the survival suite's mutant corpus through `fuzz_codec_adpcm` (the flac/pcm precedent), add a truncated-block and a dangling-tail case.
Confidence: HIGH — Verification: read + grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FLAVORS` const | 46-55 | REAL | — | 8 descriptors, matches the module doc's cycle exactly; `adpcm_ima` (headerless DVI) honestly absent per decoder docs. |
| `CountingSink` + `impl FrameSink` | 58-68 | REAL | — | Saturating count; live target (5 flavors reach decode every call). |
| `raw_facts_skeleton` | 73-82 | REAL | — | Input-derived, always-legal 16-byte tag-0 skeleton; no panics (`unwrap_or(0)` for short input); pinned by test 150. |
| `fuzz_codec_adpcm` | 87-127 | REAL | S4 (F-001 context) | 8 iterations; WAV flavors take input as `extra_data` (95-96); raw flavors the skeleton (98); `adpcm_oki` swallowed at the configure gate as documented; packet is a full input clone (118, input-bounded); all outcomes swallowed per the survival contract. |
| 3 tests | 133-158 | REAL | S4 (F-001, F-002) | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_and_short_inputs_survive` (134) | YES (panic only) | Empty + 4-byte + 32 deterministic short buffers through all 8 flavors. |
| `hostile_wide_inputs_survive` (144) | YES (panic only) | 0xFF/0x00 floods. |
| `raw_facts_skeleton_stays_legal` (150) | YES | Real assertions: decodes channels/rate back out of the skeleton bytes and bounds them — would fail if the skeleton builder regressed. |
Mutation check (reasoned): a panic in `AdpcmDecoder::decode` for any raw flavor fails all three tests (configure provably reaches decode); a configure regression no-ops everything and fails NOTHING here (F-001, caught only by mod.rs's own tests).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the raw breadth flavors take a synthesized 16-byte WAVEFORMATEX skeleton … so the decode surface is exercised even when no container is present" | module doc 13-16 | REAL | §2 trace: skeleton == `parse_raw_facts`'s accepted shape; 5 flavors decode unconditionally. |
| "`adpcm_oki` is the lettered-variant typed refusal — the configure error is the expected outcome and is swallowed" | module doc 17-18 | REAL | `accepts` includes it (mod.rs:295); dedicated `adpcm.oki.no_reference` (mod.rs:481-491); harness `continue`s. |
| "The cargo-fuzz target is fuzz/fuzz_targets/fuzz_codec_adpcm.rs (Lead-owned nightly wiring…) the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's breadth differential suite" | module doc 25-30 | REAL | Target exists (verified); nightly campaign unwired (fuzz/Cargo.toml header); the "sweeps below" are the 3 smoke tests (accurate — it does NOT overclaim a survival-suite link, unlike the flac/pcm paragraphs' stale text). |
| "Memory is bounded by the input: … worst-case output (≤ 8 output bytes per input byte)" | module doc 32-36 | REAL | Nibble→s16 is 4× per byte in every flavor (mono or stereo); 8× is conservative; per-call allocations ≤ 16 input clones (bounded by constant × input). |

## 7. Dead / duplicated / unreachable code
- None dead. `CountingSink` is a third near-identical copy (flac/pcm/adpcm) — the shared-testkit-sink note from the pcm audit applies.
- No unsafe.

## 8. Dependencies & flags
- No feature gate (adpcm unconditional, audio/mod.rs:37 — the registration-card note); std-only. Fuzz package separate (nightly-only).

## 9. Open questions for the next agent
- adpcm/mod.rs owners (W07-A03/A04): the WAV-flavor fuzz pass only exercises `configure_wav` when the fuzzer synthesizes a valid tag 2/0x11 fmt — consider seeding the campaign corpus with one valid `adpcm_ms` fmt (7-pair coefficient table) + one `adpcm_ima_wav` fmt so the block-decode paths are reachable early.
- CI/fuzz wave: same liveness-canary recommendation as pcm/flac.
