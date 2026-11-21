# AUDIT: crates/codecs/src/audio/flac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 239 / 11 (2 pub fuzz entries, `synthetic_facts`, `CountingSink`, 5 tests) |
| File verdict | SHALLOW (a real, well-bounded harness whose headline third pass — the codec-api contract path — is provably dead code) |
| Reachable from shipped entry point? | NO as shipped-binary code (fuzz harness), YES as the registered fuzz target body: `fuzz/fuzz_targets/fuzz_codec_flac.rs` shim calls `codecs::audio::flac::fuzz::fuzz_codec_flac(data)` (verified in the target file); the campaign itself is NOT run in CI (baseline #18 runs `scripts/fuzz_registry_check.sh`, which only diffs registry-vs-targets-vs-corpus — "35 targets, diff empty"). The in-crate smoke tests execute the harness (baseline #3, 1681 passed). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `sed -n` on all quoted ranges; `rg -n "STREAMINFO_LEN" crates/codecs/src/audio/flac/metadata.rs` → 34 (line 41), length gate at 158-167; `sed -n '520,545p' crates/codecs/src/audio/flac/decoder.rs` → configure requires extra_data → parse_streaminfo; `ls fuzz/fuzz_targets/` → target EXISTS; `cat fuzz/fuzz_targets/fuzz_codec_flac.rs` → shim calls this harness; `rg -n "fuzz_codec_flac" crates/codecs/tests/flac_fuzz_survival.rs` → survival suite reuses this entry; `rg -n "fn max_output_len" -A 8` + `sed -n '250,268p' flac/decoder.rs` → slot-short refusal `flac.output.slot_size` |

## 1. One-paragraph verdict
The harness is honestly bounded and panic-free in what it actually runs — metadata walk, free-frame decode, seek resolver, and a clamped-slot decode_into pass — and its smoke tests have real teeth for the survival contract. But the module's advertised Pass 3, "the codec-api contract path (configure → packet → sink → flush)", is dead code: the hand-rolled STREAMINFO blob it feeds `extra_data` is 28 bytes where `parse_streaminfo` hard-requires 34 (`STREAMINFO_LEN`, metadata.rs:41,158-167), so `FlacDecoder::configure` refuses with `flac.streaminfo.short` on every input and the harness returns before ever decoding a packet through the contract path. The serialization is wrong twice over — 6 bytes missing AND the packed field at offset 4 instead of the spec's offset 10. The FLAC contract path (the surface the shipped pipeline actually drives) therefore has zero fuzz or mutation coverage anywhere in the tree, while the module doc claims it is one of three fuzzed surfaces. The wiring-status paragraph is also stale: it says the cargo-fuzz scaffolding is "proposed"/not landed, but `fuzz/Cargo.toml` + `fuzz/fuzz_targets/fuzz_codec_flac.rs` + seed corpora all exist (card W1-12).

## 2. What actually works (verified)
— Passes 1, 2, 4 and `fuzz_demux_flac` run for real and are input-bounded: verified by reading `synthetic_facts` (86-100: channels 1..=8, bps 4..=32, block ≤ 32768 — all in the legal domain, pinned by the test at 228-238), the `cut` arithmetic (165: `% (data.len() + 1)` cannot divide by zero or overrun), and the 1 MiB slot clamp (182).
— Smoke tests execute the harness on empty/marker/short/0xFF/0x00/sync-flood inputs (191-225) and the survival suite (`crates/codecs/tests/flac_fuzz_survival.rs`) drives the same entry with seeded mutants — baseline #3 (`cargo test --workspace` 1681 passed) includes both.
— The corpus gate is real: baseline #18 confirms `fuzz/corpus/fuzz_codec_flac` has seeds (`seed_flac_mono16.flac` etc., verified by ls).

## 3. Findings

### F-001 [S2] Pass 3 (the codec-api contract path) can never execute: the facts blob is 28 bytes, configure demands 34
Where: crates/codecs/src/audio/flac/fuzz.rs:141-163
Code:
```rust
    let facts_bytes = {
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&facts.min_block_size.to_be_bytes());
        bytes.extend_from_slice(&facts.max_block_size.to_be_bytes());
        let packed: u64 = (u64::from(facts.sample_rate) << 44)
```
What is wrong: the blob is min_block(2) + max_block(2) + packed(8) + md5(16) = **28 bytes**. `FlacDecoder::configure` (flac/decoder.rs:530-543) passes `extra_data` to `parse_streaminfo`, which refuses anything shorter than `STREAMINFO_LEN = 34` (metadata.rs:158-167, "streaminfo body is {} bytes, spec requires 34") with `flac.streaminfo.short`. A real STREAMINFO carries two more u24 fields — min/max frame size at offsets 4-9 — which the serialization omits, so even padding to 34 would leave the packed field at the wrong offset (the parser reads `body[10..18]`, metadata.rs:173; this builder writes it at offset 4). `configure` therefore errors on every input, the `is_err()` arm returns at 161-163, and lines 165-178 (packet slicing, `decoder.decode`, `decoder.flush`, `CountingSink`) are unreachable — in the fuzz harness AND in the survival suite that calls the same entry (flac_fuzz_survival.rs:456,508,557).
Why it matters: the module doc (lines 10-12) advertises "3. the codec-api contract path (configure → packet → sink → flush)" as a fuzzed surface; it is not fuzzed anywhere. The contract path — the exact surface the shipped pipeline drives (`pipeline_host.rs:253-259` seeded `FlacDecoder`) — has zero mutation/fuzz coverage: a panic in `FlacDecoder::decode`'s pooled-slot or emission code would be invisible to the whole fuzz story. The smoke tests cannot catch it because the survival contract is "did not panic", and returning early IS not panicking.
What correct looks like: serialize the real 34-byte body (two u24 frame-size fields between max_block and packed, or zeros); assert in a smoke test that `configure` succeeds at least once (e.g. `assert!(matches!(...))` on one synthetic input) so the dead pass can never regress silently.
Confidence: HIGH — Verification: read; byte arithmetic re-derived (2+2+8+16=28); `STREAMINFO_LEN=34` and the configure path verified at metadata.rs:41,158-167 and decoder.rs:530-543.

### F-002 [S4] Stale "Wiring status (honest)" paragraph contradicts the landed fuzz scaffold
Where: crates/codecs/src/audio/flac/fuzz.rs:19-30
Code:
```rust
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_flac.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned dependency
//! decisions (AGENTS.md §3.2, C5) — so until they land the harness
//! logic is proven by the in-crate deterministic smoke tests here
```
What is wrong: the scaffolding HAS landed — `fuzz/Cargo.toml` (its own header: "delivered by card W1-12"), `fuzz/fuzz_targets/fuzz_codec_flac.rs` (a working `libfuzzer_sys::fuzz_target!` shim that calls this module), and the seed corpus. Only the second half of the paragraph (the campaign is not wired into CI, nightly pin) remains true, and the fuzz/README + the shim carry that truth correctly. The module doc describes a pre-W1-12 world.
Why it matters: "honest" status sections that drift are worse than none — a reader concludes the target file does not exist.
What correct looks like: update to "the scaffold landed (card W1-12); the nightly campaign is still unwired in CI (stable-pinned toolchain); harness proven by smoke tests + survival suite".
Confidence: HIGH — Verification: read + ls fuzz/ + cat of the shim and manifest.

### F-003 [S4] `fuzz_demux_flac` skips the seek resolver for streams with unknown total length
Where: crates/codecs/src/audio/flac/fuzz.rs:68-70
Code:
```rust
pub fn fuzz_demux_flac(data: &[u8]) {
    if let Ok(layout) = parse_stream(data)
        && let Some(total) = layout.stream_info.total_samples
```
What is wrong: `total_samples` is `None` when the STREAMINFO 36-bit field is 0 (metadata.rs:178) — a legal FLAC state (live recordings, streamed encodes). For those streams the planner/resolve half of the demux pass never runs, so the seek resolver's dirty-input exposure is sampled only from length-declared streams.
Why it matters: coverage bias inside a harness whose whole job is dirty-input exposure; the resolver's bisection over unknown-length streams is exactly the branch a fuzzer should hit.
What correct looks like: fall back to a derived target (`layout.frames_start`-relative, or the walked frame count) when `total_samples` is None.
Confidence: HIGH — Verification: read.

### F-004 [S4] Pass 4's 1 MiB clamp defeats its own "worst case" purpose for large-but-legal facts
Where: crates/codecs/src/audio/flac/fuzz.rs:180-182
Code:
```rust
    // Pass 4: decode with the maximum-slot sizing (the pooled path's
    // worst case: a frame that fills the whole declared maximum).
    let mut slot = vec![0u8; max_output_len(&facts).min(1 << 20)];
```
What is wrong: for a parsed (pass-1) STREAMINFO with `max_block_size = 65535` and ≥ 5 channels at wide output, `max_output_len` reaches 65535×8×4 ≈ 2 MiB; the clamp truncates the slot to 1 MiB and `decode_into` then refuses with `flac.output.slot_size` (decoder.rs:255-266) — the "frame that fills the whole declared maximum" the comment promises is never decoded for exactly the streams where it matters. (Synthetic facts cap at exactly 1 MiB — 32768×8×4 — so only pass-1-parsed facts hit this.)
Why it matters: the pass's stated purpose is the worst case; the clamp silently converts the worst case into a refusal. Bounded and typed, hence S4 — but the comment lies about what runs.
What correct looks like: cap the *facts* domain (clamp `max_block_size`/channels before computing) or size the slot to the true maximum with a documented harness memory budget.
Confidence: HIGH — Verification: read; `max_output_len` arithmetic verified at decoder.rs:125-128; short-slot refusal at decoder.rs:255-266.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CountingSink` + `impl FrameSink` | 50-60 | REAL | — | Saturating count; never panics. Currently dead in practice (only used by the dead pass-3 tail, F-001). |
| `fuzz_demux_flac` | 68-80 | REAL | S4 (F-003) | parse_stream → planner → resolve(total/2); all Results swallowed per the survival contract; planner only exercised when total_samples is Some. |
| `synthetic_facts` | 86-100 | REAL | — | Input-derived legal domain (ch 1..=8, bps 4..=32, block ≤ 32768, rate from 3-value table); empty-input safe via `unwrap_or(0)`. |
| `fuzz_codec_flac` | 105-185 | SHALLOW | S2 (F-001), S4 (F-004) | Passes 1/2/4 real and bounded; pass 3 dead at the configure gate (28 < 34 bytes). `cut` slicing safe; descriptor/generation early-outs safe. |
| 5 tests | 191-238 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `demux_pass_survives_hostile_input` (192) | YES (panic only) | Four hostile bins through the demux pass; a panic fails the test, wrong seek plans do not (survival contract — honest about that at lines 14-17). |
| `empty_input_survives_all_passes` (200) | PARTIAL | Runs the harness, but "all passes" is a misnomer — pass 3 early-returns at configure (F-001); the name promises the surface the code never reaches. |
| `marker_only_and_short_inputs_survive` (205) | YES (panic only) | 41 deterministic short inputs. |
| `hostile_wide_inputs_survive` (214) | YES (panic only) | 0xFF/0x00 floods + a sync-flood with plausible headers — good adversarial shapes. |
| `synthetic_facts_stay_in_the_legal_domain` (228) | YES | Real domain assertions incl. the slot bound ≤ 65535×8×4 — this one has teeth beyond survival. |
Mutation check (reasoned): breaking `synthetic_facts`'s domain fails test 228; introducing a panic in `parse_stream`/`decode_frame` fails 192/205/214 and the survival suite. Introducing a panic in `FlacDecoder::decode`/`flush` fails NOTHING in this file or the fuzz campaign (F-001) — only the decoder's own unit tests would catch it.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through all three decode surfaces: … 3. the codec-api contract path (configure → packet → sink → flush)" | module doc 4-12 | FALSE | F-001: configure refuses the 28-byte blob on every input (STREAMINFO_LEN=34, metadata.rs:158-167); the packet→sink→flush tail never runs. |
| "the libFuzzer campaign itself is owed CI wiring" | module doc 29-30 | REAL | Baseline #18 runs only the registry diff; `fuzz/Cargo.toml` header records the stable-toolchain nightly block. |
| "Memory is bounded by the input: the decode output is at most max_block_size (≤ 65 535) × channels (≤ 8) × 4 bytes" | module doc 32-39 | REAL | decode_into refuses short slots typed (decoder.rs:255-266); pass-4 clamp bounds the harness allocation; synthetic domain pinned by test 228. |

## 7. Dead / duplicated / unreachable code
- Lines 165-178 (packet construction, `decoder.decode`, `decoder.flush`, sink use) + `CountingSink` (50-60): dead on every input (F-001).
- The wiring paragraph (19-30) describes a superseded state (F-002).

## 8. Dependencies & flags
- No Cargo feature gates this module (`pub mod fuzz` in flac/mod.rs, unconditional under `codec-flac` which is default-on); depends only on codec-api/media-types/super. The fuzz campaign package (`fuzz/`) is a separate stable-checkable workspace root (nightly needed to run).

## 9. Open questions for the next agent
- flac/decoder.rs owner: does the decoder's own test module cover the pooled-slot emission path (the surface F-001 leaves unfuzzed)? The `body = [0u8; 34]` at decoder.rs:457 suggests configure is unit-tested with a CORRECT 34-byte blob — the harness could simply reuse that construction.
- Fuzz/CI wave: consider a gate that runs each in-crate harness once with a canary input asserting at least one non-early-return outcome per advertised pass — the registry check (name+corpus presence) cannot see dead passes.
