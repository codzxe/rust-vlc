# AUDIT: crates/codecs/src/audio/mp3/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 214 / 3 (fuzz_mp3_stream, CountingSink::accept) + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO by design (fuzz harness) — but wired: fuzz/fuzz_targets/fuzz_codec_mp3.rs:19 calls `codecs::audio::mp3::fuzz::fuzz_mp3_stream(data)`; logic proven by tests/mp3_fuzz_survival.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `target/debug/deps/mp3_fuzz_survival-a38f4eaafbfa5a92` → 2 passed (13.66s, real mutation campaign); `codecs-bae0f1756cc0a809 'mp3::fuzz'` → 2 passed; sed/rg verifications of fuzz/ wiring |

## 1. One-paragraph verdict
REAL. The harness runs all six documented passes (ID3 walk, header walk, Xing/VBRI, full decode_stream, contract path, seek planners) over arbitrary bytes with typed-error swallowing and bounded work (`take(64)` on the contract pass, saturating counters, `data.get()` bounds-checked slices). The panic-free claim is backed by an executable 13.7-second seeded-mutation + corpus-pool survival run that passes. The defect is documentation: the module's headline "Wiring status (honest)" section says the cargo-fuzz target file and the fuzz/Cargo.toml scaffolding are still Lead-owned proposals "until they land" — both HAVE landed (target file exists and calls this function; libfuzzer-sys 0.4.13 is a declared dependency), so the honest note is stale and understates what exists. Only the nightly libFuzzer campaign execution remains unwired, which the fuzz/README records correctly.

## 2. What actually works (verified)
— verified by `target/debug/deps/mp3_fuzz_survival-a38f4eaafbfa5a92` → `mutated_fixtures_never_panic_and_settle_typed ... ok`, `corpus_dirty_and_garbage_pools_survive ... ok` (2 passed, 13.66s).
— verified by `codecs-bae0f1756cc0a809 'mp3::fuzz'` → 2 passed (empty/garbage and header-only inputs).
— verified by reading fuzz/fuzz_targets/fuzz_codec_mp3.rs → `fuzz_target!(|data: &[u8]| { codecs::audio::mp3::fuzz::fuzz_mp3_stream(data); });` and fuzz/Cargo.toml:43 `libfuzzer-sys = "0.4.13"`.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mp3/fuzz.rs:22-33
Code:
```rust
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp3.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land
```
What is wrong: the "honest" wiring-status note is stale — it describes a state that no longer exists. `fuzz/fuzz_targets/fuzz_codec_mp3.rs` EXISTS (a complete libFuzzer shim calling this exact function), and `fuzz/Cargo.toml:43` declares `libfuzzer-sys = "0.4.13"`. The doc's "proposed … until they land" misleads any reader auditing coverage ("the campaign isn't wired") in the understating direction — the shims exist; what is actually owed is only the nightly campaign run (fuzz/README.md records that truth). Why it matters: wiring-status docs are the load-bearing honesty surface of this repo's fuzz story; a stale one erodes exactly the trust it exists to build. What correct looks like: update to "target + scaffolding landed (fuzz/README registry row `fuzz_codec_mp3`); campaign execution owed nightly wiring". Confidence: HIGH — Verification: read + ls fuzz/fuzz_targets.
F-002 [S4]
Where: crates/codecs/src/audio/mp3/fuzz.rs:136-141 and 97
Code:
```rust
        let codec = match first.header.layer {
            super::header::Layer::L1 => "mp1",
            super::header::Layer::L2 => "mp2",
            super::header::Layer::L3 => "mp3",
            super::header::Layer::Reserved => "mp3",
        };
```
What is wrong: `walk_frames` can never yield a `Reserved`-layer frame — the family parser refuses reserved layer bits typed before any frame is accepted (mp3/header.rs:291-293, code `mp3.header.layer_reserved`), so the `Reserved` arm is dead. Same block: `let _ = len;` (line 97) is a dead assignment, and `id3v2_len(data)` is computed twice (94 and 100) — redundant re-parse of the same bytes per fuzz call. Cosmetic. Confidence: HIGH — Verification: read + mp3/header.rs:291-293.
F-003 [S4]
Where: crates/codecs/src/audio/mp3/fuzz.rs:188-213 (in-file tests)
What is wrong: the two in-file tests use only inputs that walk ZERO frames (`[]`, `0xFF×4096`, a lone 4-byte header) — so harness passes 3-6 (Xing/TOC seek, decode_stream, contract path, CBR plan) never execute in the in-file suite; the assertions are satisfied trivially. The real coverage lives in tests/mp3_fuzz_survival.rs (verified passing), which the doc honestly points to — but a reader of this file alone would overestimate what its own tests prove. Confidence: HIGH — Verification: read + ran both.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_mp3_stream` | 85-182 | REAL | — | six passes; bounds-checked (`data.get(..)`, `take(64)`); typed errors swallowed by contract |
| `CountingSink::accept` | 60-65 | REAL | — | saturating counter |
| `FuzzOutcome` (struct) | 68-80 | REAL | — | observable digest |
| test `empty_and_garbage_inputs_never_panic` | 189-206 | REAL/shallow | S4 | passes 1-2 only (F-003) |
| test `a_header_only_buffer_is_survived` | 209-213 | REAL/shallow | S4 | zero frames walked |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `empty_and_garbage_inputs_never_panic` | panic YES / logic NO | full-outcome equality only for `[]`; garbage asserts `frames == 0`; deep passes never run |
| `a_header_only_buffer_is_survived` | panic YES / logic NO | asserts frames==0 only |
| tests/mp3_fuzz_survival.rs (ran) | YES | 6 fixtures × seeded LCG mutations + corpus dirty/garbage pools, per-class outcome tally — real survival campaign |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:17-20 | REAL | survival run passed; bounds math below consistent |
| "Memory is bounded by the input: … at most walked_frames × 1152 × 2 channels × 4 bytes … frame count bounded by input_len / frame_size (frame_size ≥ 24)" | fuzz.rs:35-42 | REAL-UNPROVEN | arithmetic self-consistent (LSF L3 min frame = 24); actual allocation inside decode_stream is another file's code — not re-derived here |
| "the cargo-fuzz target file is proposed … until they land" | fuzz.rs:24-29 | FALSE (stale) | F-001: target file + Cargo.toml dep both exist |
| fuzz/README registry row `fuzz_codec_mp3 → fuzz_mp3_stream` | fuzz/README.md:71 | REAL | verified the shim calls it |

## 7. Dead / duplicated / unreachable code
Dead `Layer::Reserved` arm (F-002); duplicate `id3v2_len` call; `let _ = len;`. No production entry point reaches this file — by design (fuzz harness wired to the cargo-fuzz package, not the shipped binaries).

## 8. Dependencies & flags
`pub mod fuzz` behind `codec-mp3` (mp3/mod.rs:60). The harness itself adds no deps; the fuzz package pins libfuzzer-sys 0.4.13 (fuzz/Cargo.toml:43) outside the workspace (root members = crates/* + tools/xtask).

## 9. Open questions for the next agent
- mp3/decoder.rs owner: confirm `decode_stream` preallocates from the walked frame count as the bounds doc claims (the "bounded by the input" guarantee lives there, not here).
- Whoever owns fuzz/README: the campaign (nightly `cargo fuzz run`) is still owed — that is the honest remaining gap F-001 should have stated.
