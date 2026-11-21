# AUDIT: crates/codecs/src/audio/aac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 163 / 6 items |
| File verdict | UNREACHABLE from shipped entry points (fuzz/test infrastructure; real, executed in-crate) |
| Reachable from shipped entry point? | NO for this file (harness called only by `fuzz/fuzz_targets/fuzz_codec_aac.rs` (libFuzzer target, campaign not run — stable-pinned toolchain) and `crates/codecs/tests/aac_fuzz_survival.rs`). The module is `pub mod fuzz;` (aac/mod.rs:68), so it compiles into every shipped binary as dead weight. CONTEXT for the wave question: the AAC decoder this harness exercises IS reachable and works — `lumen open` on an MP4 AAC track → mp4 demuxer mints "aac" → apps/cli/src/pipeline_host.rs:288-292 `\"aac\" \| \"aac-lc\" => self.seeded(track, config, AacDecoder::new)`; baseline `mp4-smoke-open-session-aac.txt`: Buffering → **Playing** → Stopped (null device); `decode-bench` alone reports "no decoder in this build" because the bench lane is hard-coded to MJPEG+PCM (baseline INDEX "Standing defects"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ls fuzz/fuzz_targets (fuzz_codec_aac.rs EXISTS, calls `codecs::audio::aac::fuzz::fuzz_aac_stream`); head fuzz/Cargo.toml (scaffold exists, self-contained workspace, stable-checkable, not gated); rg "aac" audit/runs/baseline/test-workspace.log.txt:276-293,792-810 (fuzz module tests + bitexact + corpus_differential_vs_ffmpeg + fuzz_survival all ok); rg "fuzz-registry" full-gate.log.txt:46-48 (CLEAN — 35 targets, corpora present); sed -n quote verification (19-31, 91-97, 130-132); hand-decode of the ADTS header in the header-only test (frame_len=255 > 7-byte buffer → 0 frames ✓). |

## 1. One-paragraph verdict
A competent four-pass hostile-input harness (header parse, resync walk, full stream decode, contract path over ≤64 walked frames) with honest absence-of-panic semantics, bounded memory, checked slicing (`data.get(...).unwrap_or(&[])`), and no unwrap/panic of its own; its determinism is proven by the in-crate tests and the seeded-mutation survival suite, all green in the committed baseline. Two hygiene defects: the "Wiring status (honest)" section is no longer honest — the libFuzzer target file and the fuzz/Cargo.toml scaffolding it calls "proposed"/"Lead-owned … until they land" both exist in the tree today (registry check passes with 35 targets) — and the `if let Some(first)` binding is a dead store (`let _ = first;`) used only as an existence check. The harness itself never lies about what it proves: no coverage claims, damage swallowed by contract.

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:289,293 → `aac::fuzz::tests::{a_header_only_buffer_is_survived, empty_and_garbage_inputs_never_panic}` ok; :806-810 `aac_fuzz_survival::{sync_storms_and_garbage_interleaves_never_panic, mutated_fixtures_never_panic_and_settle_typed}` ok (seeded LCG mutants of a real ffmpeg-authored LC fixture through this exact function); :796-802 `aac_bitexact` (2 ok) + `aac_corpus_differential_vs_ffmpeg` ok — the decode surface under test is bit-exact against ffmpeg.
— verified by read: every indexing path is checked (`data.get(frame.offset..frame.offset + frame.header.frame_len).unwrap_or(&[])`, 111-113); the contract pass is bounded (`take(64)`, 110); `CountingSink` saturates; all fallible steps early-return the partial `FuzzOutcome` rather than unwrapping (94-101).
— verified by hand: the header-only test's 7 bytes decode to ADTS frame_len = ((0x08&0x3)<<11)|(0x1F<<3)|(0xFC>>5) = 255 > 7, so the walk correctly reports 0 frames — the test's claim is arithmetically true.

## 3. Findings

### F-001 [S4] — "Wiring status (honest)" is stale: the "proposed" libFuzzer target and scaffolding both exist in the tree
Where: crates/codecs/src/audio/aac/fuzz.rs:19-31
Code:
```rust
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_aac.rs` exactly as doc 04 §3.1
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land the
```
What is wrong: `fuzz/fuzz_targets/fuzz_codec_aac.rs` EXISTS (a real `fuzz_target!` shim calling `codecs::audio::aac::fuzz::fuzz_aac_stream`), and `fuzz/Cargo.toml` EXISTS (a self-contained scaffold that compiles on stable, per its own header). The registry gate passes: "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present" (audit/runs/baseline/full-gate.log.txt:46-48). What remains true is only the narrower claim that the *campaign* is not wired into any gate (toolchain stable-pinned 1.99.1, cargo-fuzz needs nightly).
Why it matters: A doc section titled "(honest)" that misstates the tree teaches readers to distrust the real honesty markers. Same stale status is repeated in `crates/codecs/tests/aac_fuzz_survival.rs:5` ("the proposed fuzz/fuzz_targets/fuzz_codec_aac.rs target").
What correct looks like: "The libFuzzer target and fuzz/Cargo.toml scaffold are committed (registry gate CLEAN); running the campaign still awaits the Lead's nightly toolchain decision — until then the harness logic is proven by …".
Confidence: HIGH — Verification: read (files listed) / ran-baseline-cite (registry gate).

### F-002 [S4] — `if let Some(first)` binds a value only to discard it
Where: crates/codecs/src/audio/aac/fuzz.rs:92 and 131
Code:
```rust
    if let Some(first) = frames.first() {
```
```rust
        let _ = first;
```
What is wrong: `first` is never used; the binding exists only so `if let` can express "is non-empty", and the trailing `let _ = first;` silences the unused-variable lint while making the reader hunt for a use that does not exist.
Why it matters: Dead store + misleading name in a file whose whole point is to be legible to auditors of the fuzz surface.
What correct looks like: `if !frames.is_empty() { … }` and delete both lines 92's binding and 131.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-40 | REAL | S4 | harness contract accurate; bounds section accurate but does not name the 64-frame contract-pass cap ("a fixed number of passes" — loose); wiring status stale (F-001) |
| `CountingSink` + FrameSink impl | 50-60 | REAL | — | saturating counter; accepts any Frame |
| `FuzzOutcome` | 63-71 | REAL | — | observable digest; asserted by tests |
| `fuzz_aac_stream` | 76-134 | REAL | — | 4 passes; all fallible steps early-return; checked slicing only; no unwrap/panic; per-frame `to_vec` bounded by take(64) × frame_len ≤ 8191; stale/`first` dead store (F-002) |
| test `empty_and_garbage_inputs_never_panic` | 140-155 | REAL | — | pins exact outcome for empty; frames==0 for 0xFF×4096 |
| test `a_header_only_buffer_is_survived` | 157-162 | REAL | — | hand-verified frame_len=255 arithmetic |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_garbage_inputs_never_panic | YES | exact-struct assert for empty; a walk that accepted 0xFF garbage as frames would report frames>0 and fail; any panic fails the test by definition |
| a_header_only_buffer_is_survived | PARTIAL | pins frames==0 for a header whose frame_len exceeds the buffer — catches a walk that emits short frames, but a single 7-byte input is a narrow pin (the survival suite widens it) |
| (external) aac_fuzz_survival 2 tests | YES | seeded LCG mutants of a real LC fixture through this function, tallying Decoded vs Typed codes — a panic or untyped settle fails |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:14-17 | REAL | code shape (swallowed errors, bounded passes) + survival tests ok (baseline :806-810) |
| "Memory is bounded by the input: output at most walked_frames × 1024 × 2ch × 4B, frames bounded by input_len/7" | fuzz.rs:33-40 | REAL-UNPROVEN (decoder-side bound owned by aac/decoder.rs — not this wave's file) | consistent with pass structure; `take(64)` cap additionally bounds pass 4 |
| "the libFuzzer campaign itself is owed CI wiring" | fuzz.rs:29-31 | REAL | fuzz/Cargo.toml header: scaffold "NOT wired into any gate"; toolchain stable-pinned |
| "The cargo-fuzz target file is proposed … until they land" | fuzz.rs:21-26 | FALSE (stale) | target + scaffold exist; registry gate CLEAN 35 targets (F-001) |

## 7. Dead / duplicated / unreachable code
- The module is unreachable from shipped entry points (fuzz/test only) yet compiled unconditionally (`pub mod fuzz;` aac/mod.rs:68) — dead weight in every shipped binary, same as the codec tree's other fuzz modules.
- `first` binding dead (F-002).

## 8. Dependencies & flags
- `codec_api`, `media_types`, crate-internal `super::adts`/`super::decoder` — no external deps in this file. The libFuzzer sys dep lives only in the separate fuzz/ workspace (not the shipped graph).

## 9. Open questions for the next agent
- aac/decoder.rs owner: confirm the pooled "4 output slots of 1024×2×4" bound quoted in this file's docs matches the decoder's actual pool geometry.
- If the Lead ever wires the nightly campaign, the stale "(honest)" sections here and in aac_fuzz_survival.rs:5 should be rewritten in the same change (F-001).
