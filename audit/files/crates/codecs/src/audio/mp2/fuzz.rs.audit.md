# AUDIT: crates/codecs/src/audio/mp2/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 190 / 1 prod fn (`fuzz_mp2_stream`) + `FuzzOutcome` struct + `CountingSink`; 3 tests |
| File verdict | REAL (deterministic hostile-input harness, genuinely exercising every module surface incl. engine decode; mutation sweeps executed, pass) — but NO libFuzzer campaign has ever run |
| Reachable from shipped entry point? | NO (by design: fuzz infrastructure). Reachable from the `fuzz/` workspace target `fuzz_codec_mp2` (fuzz/fuzz_targets/fuzz_codec_mp2.rs calls `codecs::audio::mp2::fuzz::fuzz_mp2_stream`) and from `tests/mp2_fuzz_survival.rs` (ran, passes). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p codecs --test mp2_fuzz_survival --offline` -> 3 passed / 0 failed (4.85 s); `cargo test -p codecs --lib mp2 --offline` -> 27 passed (includes this file's 3); `ls fuzz/fuzz_targets/`, `ls fuzz/corpus/fuzz_codec_mp2/` -> 1 seed; sed verifications |

## 1. One-paragraph verdict
The harness itself is real and reasonably deep: one call drives the Layer II walk (resync/layer gate), the CBR geometry gate, both-direction seek arithmetic, the raw 4-byte header parse, the full `decode_stream` pipeline *including the symphonia engine decode of every walked frame*, and the codec-api contract path (configure → up to 64 packets → flush), with the contract honestly scoped to absence-of-panic/hang/unbounded-allocation. The committed mutation sweeps (which I ran: 3 tests pass, ~4.9 s) prove that contract on seeded mutants of real fixtures. Three defects, all documentation/hygiene: (1) the "Wiring status (honest)" section is now false — it says the cargo-fuzz target file is only "proposed" and that the `libfuzzer-sys` dependency and `fuzz/Cargo.toml` scaffolding have not landed, but all three exist in the tree today (the target is a real `fuzz_target!` shim); (2) the memory-bounds note cites "frame_size ≥ 17 … the 8 kbps/48 kHz corner" — an impossible bitrate/rate combination whose real arithmetic gives 48 bytes, not 17 (the bound still holds, conservatively); (3) the module compiles unconditionally into the release library (`pub mod fuzz`, mod.rs:69) with no feature gate, shipping harness code in the binary. What remains TRUE and matters: no actual libFuzzer campaign has ever executed (stable-pinned toolchain, nightly required, zero gate wiring, corpus = 1 seed file) — all "fuzz" evidence for MP2 is the in-crate deterministic sweeps.

## 2. What actually works (verified)
— `cargo test -p codecs --test mp2_fuzz_survival --offline` -> `the_fuzz_harness_survives_the_same_mutations ... ok`, `mutated_fixtures_never_panic_and_settle_typed ... ok`, `corpus_dirty_and_garbage_pools_survive ... ok` (3 passed).
— `cargo test -p codecs --lib mp2 --offline` -> 27 passed (this file's 3 smoke tests included).
— Verified by read: the harness covers every surface it claims (walk 86-89, geometry + `cbr_seek`/`sample_at_byte`/`duration_nanos` 95-102, raw header parse at the boundary 104-107, full engine decode via `decode_stream` 109-111, contract path with pooled slots and flush 113-150); the contract-path pass is bounded (`take(64)`); errors are swallowed by contract, outcomes surfaced in `FuzzOutcome`.
— `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exists and calls exactly this harness (verified by cat): `fuzz_target!(|data: &[u8]| { codecs::audio::mp2::fuzz::fuzz_mp2_stream(data); });`.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/mp2/fuzz.rs:18-28
Code:
```
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exactly as doc 04 §8.5
//! registers it; building it needs the `libfuzzer-sys`
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
```
What is wrong: the statement is false against the current tree. `fuzz/fuzz_targets/fuzz_codec_mp2.rs` EXISTS (a real `#![no_main]` + `fuzz_target!` shim), `fuzz/Cargo.toml` EXISTS with `libfuzzer-sys = "0.4.13"` and a `[[bin]]` target for `fuzz_codec_mp2`, and `fuzz/corpus/fuzz_codec_mp2/` exists with one seed. The doc understates what has landed (conservative direction, not inflation) and misleads a reader about the infrastructure state. The substantive claim that survives: the campaign has never RUN — the toolchain is stable-pinned 1.99.1, cargo-fuzz needs nightly, and no gate builds it (fuzz/Cargo.toml says so itself). What correct looks like: update the section to "target + scaffold landed; campaign blocked on nightly wiring; evidence = in-crate deterministic sweeps". Confidence: HIGH — Verification: read (`ls fuzz/`, `cat fuzz/fuzz_targets/fuzz_codec_mp2.rs`, `rg libfuzzer fuzz/Cargo.toml`).
F-002 [S4]
Where: crates/codecs/src/audio/mp2/fuzz.rs:30-38
Code:
```
//! Memory is bounded by the input: `decode_stream`'s output is at
//! most `walked_frames × 1152 × 2 channels × 4 bytes` with the frame
//! count bounded by `input_len / frame_size` (frame_size ≥ 17 for
//! any Layer II table bitrate at the LSF floor — the 8 kbps/48 kHz
//! corner), a fixed number of passes per call. The contract path
```
What is wrong: the cited corner does not exist. 8 kbps appears only in the MPEG-2/2.5 LSF Layer II table (crates/codecs/src/audio/mp2/header.rs:57-59), whose sample rates top out at 24 kHz; 48 kHz is MPEG-1, whose Layer II floor is 32 kbps. Real corners: LSF 8 kbps @ 24 kHz → 144×8000/24000 = 48 bytes; MPEG-1 32 kbps @ 48 kHz → 96 bytes. The minimum Layer II frame size is therefore 48 bytes, not 17 — the bound still holds (48 > 17), so the memory-boundedness conclusion is safe, but the stated justification is wrong arithmetic a reader cannot reproduce. What correct looks like: "frame_size ≥ 48 (8 kbps @ 24 kHz LSF corner)". Confidence: HIGH — Verification: read (tables in header.rs:50-59 + the spec frame-size formula quoted in header.rs:30-33).
F-003 [S4]
Where: crates/codecs/src/audio/mp2/mod.rs:69 (gating this file)
Code (mod.rs:69):
```
pub mod fuzz;
```
What is wrong: the harness module is compiled unconditionally into the release library — no `cfg(test)`, no feature gate. `fuzz_mp2_stream` + `CountingSink` + `FuzzOutcome` ship in every binary that links `codecs` (the CLI does), unreachable from any shipped entry point. The intended consumer (fuzz/ workspace) could equally reach it behind a `fuzz` feature. Blast radius: a few hundred bytes of dead code in the binary. Confidence: HIGH — Verification: read (rg for callers: only tests/ and fuzz/).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_mp2_stream` | 78-152 | REAL | — | All five surfaces driven; typed errors swallowed by contract; contract path bounded to 64 frames |
| `FuzzOutcome` | 62-73 | REAL | — | Observable digest asserted by the survival suite |
| `CountingSink` | 48-59 | REAL | — | Saturating counter, accepts all |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_garbage_inputs_never_panic | YES (shallow) | Empty + 4096×0xFF: pins zero-frame walk outcome |
| a_header_only_buffer_is_survived | YES (shallow) | Valid header, no body → zero frames |
| a_layer_iii_frame_is_refused_not_decoded | YES (shallow) | Layer gate via outcome flags |
| (external) mp2_fuzz_survival.rs `the_fuzz_harness_survives_the_same_mutations` | YES | Seeded-LCG flips/truncations/garbage-prefixes of real fixtures driven through `fuzz_mp2_stream` — ran, passes; this is the real evidence, the in-file smoke tests are only a sanity layer |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:13-16 | REAL | Read: all errors swallowed; survival suite pins typed outcomes |
| "the cargo-fuzz target file is proposed … until they land the harness logic is proven by the in-crate deterministic smoke tests" | fuzz.rs:18-28 | FALSE (stale) | F-001: target + Cargo.toml + corpus exist in-tree; only the campaign-not-run part is still true (fuzz/Cargo.toml states stable-pinned toolchain, no gate wiring) |
| "frame_size ≥ 17 for any Layer II table bitrate at the LSF floor — the 8 kbps/48 kHz corner" | fuzz.rs:34-36 | FALSE (arithmetic) | F-002: combination impossible; true minimum is 48 bytes (LSF 8 kbps @ 24 kHz) |
| docs/formats/audio-mp3-mp2.md: "fuzz_target = fuzz_codec_mp2 (in-crate harness logic; libFuzzer CI wiring Lead-owned)" | docs/formats/audio-mp3-mp2.md:11-12 | REAL | Matches reality: harness logic in-crate + sweeps; no campaign run |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from the shipped CLI (by design); shipped in the binary anyway (F-003).
- No duplication: it composes the module's own surfaces rather than re-implementing them.

## 8. Dependencies & flags
- No new deps (uses codec-api/media-types/symphonia via the module). The `fuzz/` scaffold depends on `libfuzzer-sys = "0.4.13"` — a separate workspace, not built by any gate. Not feature-gated (F-003).

## 9. Open questions for the next agent
- The same stale "wiring status" text pattern exists in the sibling harnesses (`fuzz_codec_mp3` "and its siblings" per fuzz.rs:27-28) — the adpcm fuzz file (my next file) and the mp3/vorbis/aac/flac ones likely carry the identical stale paragraph; whoever audits them should check `fuzz/fuzz_targets/` the same way.
- Has ANY libFuzzer campaign in this repo ever run (any target), or is every "fuzz" evidence string in the matrix actually a deterministic sweep? The corpus dirs' contents (seed-only vs grown) are the tell.
