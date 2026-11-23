# AUDIT: crates/subtitles/src/cc/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 99 / 2 (fuzz_sub_cc, feed) |
| File verdict | UNREACHABLE (harness logic real and survival-tested; its cargo-fuzz entry point was never created) |
| Reachable from shipped entry point? | NO — crate-level wall (only fuzz/Cargo.toml:50 links `subtitles`); `fuzz_sub_cc` is called only by tests/cc_acceptance.rs:452,487,489; `fuzz/fuzz_targets/` contains fuzz_sub_{srt,ass,vtt}.rs but NO fuzz_sub_cc.rs, and fuzz/README.md's registry (:81-83) lists exactly those three — CC is unregistered |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | ls fuzz/fuzz_targets/ (no fuzz_sub_cc.rs); Grep `fuzz_sub_cc` repo-wide (mod.rs doc refs + cc_acceptance.rs callers only); sed quote verifications (exit 0); baseline citation |

## 1. One-paragraph verdict
The harness body is a competent multi-surface pass (GA94 SEI, MPEG-2 GA94, SCTE-20, raw-triplet shapes, reset-relock) with deterministic survival proof — but the coverage story has two holes the doc paper-clips over: (1) the claimed "registry name `fuzz_sub_cc`" does not exist — the Lead wired cargo-fuzz targets for srt/ass/vtt and never for CC, so the most stateful decode surface in the crate has zero coverage-guided fuzzing; (2) the doc claims "the stateful cross-payload paths are covered by the multi-chunk splits", but each `feed()` constructs a fresh `CcDecoder`, so chunk splits never carry state across payloads — cross-payload state is exercised ONLY by the four sequential decodes inside a single `feed()`. Plus the usual crate-level unreachability.

## 2. What actually works (verified)
— Baseline (audit/runs/baseline/cargo-test-workspace.txt:6022-6035): all 11 cc_acceptance tests pass, including `fuzz_lite_survival_seeded_mutations ... ok` (feeds fuzz_sub_cc over >100 KiB of mutated bytes and asserts the damage-ledger cap per pass) and `systematic_truncation_sweep_survives ... ok` (every truncation of every seed through every surface). The chunking splitter and truncation-sweep logic read as panic-free (no indexing beyond slice bounds; `saturating_sub` on the cut).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/cc/fuzz.rs:1-2 and 17-25
Code:
```
//! The CC fuzz harness (card 5.N3 `+FB`; doc 04 §5 — registry name
//! `fuzz_sub_cc`).
```
What is wrong: the registry name is not in the registry. `ls fuzz/fuzz_targets/` shows fuzz_sub_srt.rs, fuzz_sub_ass.rs, fuzz_sub_vtt.rs — no fuzz_sub_cc.rs; fuzz/README.md:81-83 lists exactly those three. The doc's own "Wiring status (honest)" paragraph (lines 17-25) admits the target file is missing and defers to "the Lead", but the header still asserts the registry name as if registered, and the deferral has a concrete cost: every text-subtitle format got a libFuzzer entry point; the CC path — two carriage grammars + a stateful 608 machine + a 708 reassembler, i.e., the surface that benefits most from coverage guidance — got none. Only the fixed seeded-LCG mutations and truncation sweeps run (cc_acceptance.rs:451-489). What correct looks like: a 6-line fuzz/fuzz_targets/fuzz_sub_cc.rs mirroring fuzz_demux_ps.rs's shape, plus the README row. Confidence: HIGH — Verification: read (ls + greps, exit 0).

F-002 [S3]
Where: crates/subtitles/src/cc/fuzz.rs:66-70 vs 59-63
Code:
```
/// One payload through every surface (fresh decoder state each
/// time — the stateful cross-payload paths are covered by the
/// multi-chunk splits and the survival suite).
fn feed(payload: &[u8]) {
    let mut decoder = CcDecoder::new();
```
What is wrong: the claim is contradicted by the code. The multi-chunk branch (lines 59-63) calls `feed(chunk)` per chunk, and `feed` starts with a fresh `CcDecoder::new()` — so no state ever crosses a chunk boundary. Stateful cross-payload behavior (channel latch persistence across payloads, roll-up scroll carryover, XDS skip state, pop-on double-buffer half-fills across SEI payloads) is exercised only by the four sequential `decoder.decode(...)` calls inside one `feed()` (lines 71-98), i.e., only when the four grammars all parse the same payload. The splits — the natural tool for cross-payload state fuzzing — cover none of it. What correct looks like: a persistent decoder threaded across the chunk loop in `fuzz_sub_cc` (a second pass with shared state). Confidence: HIGH — Verification: read.

F-003 [S2]
Where: crates/subtitles/src/cc/fuzz.rs:36-39
Code:
```
/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the decoder's bounds-checked paths.
```
What is wrong: same crate-level wall as every other file in this assignment — no shipped app links the crate, so this harness (and the CC decoder behind it) is exercised only by the crate's own test binary. The bounds claim itself is spot-checked by the survival test's damage-cap asserts (baseline :6032,6026 `fuzz_lite_survival_seeded_mutations`/`systematic_truncation_sweep_survives` ok) but the per-decoder allocation ceilings (MAX_DTVCC_PACKET etc.) are owned by cc/cea708.rs and cc_data.rs — UNVERIFIED here (other agents' files). Confidence: HIGH (reachability) / UNVERIFIED (decoder-internal bounds).

## 4. Per-function table (file ≤ 100 LOC — condensed)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_cc` | 39-64 | REAL | — | Chunk splitter + truncation sweep; panic-free by read; only test callers. |
| `feed` | 69-99 | REAL | S3 | Four grammars + raw triplets + reset-relock on one fresh decoder per call (F-002). |

## 5. Test honesty
| Test (cc_acceptance.rs, other agent's file) | Would it fail if code were broken? | Reason |
|---|---|---|
| fuzz_lite_survival_seeded_mutations | YES (panics/caps only) | Feeds >100 KiB mutated bytes, asserts damage cap per pass — catches panics and ledger overflow, not correctness. |
| systematic_truncation_sweep_survives | YES (panics only) | Every truncation of every seed. |
Both pass in baseline (:6032,6035 region). Neither can catch F-001/F-002 (coverage-shaped holes, not crashes).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "registry name `fuzz_sub_cc`" | fuzz.rs:1-2 | FALSE (unregistered) | fuzz/README.md:81-83 lists only srt/ass/vtt (F-001). |
| "the stateful cross-payload paths are covered by the multi-chunk splits" | fuzz.rs:67-68 | FALSE | Fresh decoder per feed (F-002). |
| "One pass, output ≤ O(input) + the fixed 15×32 grids" | fuzz.rs:29-30 | REAL-UNPROVEN | Decoder-internal caps not audited here (UNVERIFIED). |
| "until the Lead wires it, the harness logic is proven on stable by the deterministic survival runs" | fuzz.rs:22-25 | REAL | baseline :6026/:6032 both ok. |

## 7. Dead / duplicated / unreachable code
The module is reachable only from the crate's own test binary. No internal dead code. The ass/srt/vtt fuzz modules have cargo-fuzz shims; this one does not (F-001).

## 8. Dependencies & flags
crate::cc (CcDecoder, CcCueBuilder, cc_data parsers). No unsafe, no I/O, no features. The `+FB` card tag and doc 04 §8.5 registration are doc references only.

## 9. Open questions for the next agent
- cc/cea708.rs + cc/cc_data.rs auditors: verify MAX_DTVCC_PACKET and the reassembler ceiling actually bound allocation on hostile input (the fuzz harness's "never allocates without a ceiling" rests on them).
- dvb/fuzz.rs + pgs/fuzz.rs + vobsub/fuzz.rs: same unwired-target question (no fuzz_sub_dvb/pgs/vobsub in fuzz/fuzz_targets/ either — check whether their docs make the same unfulfilled registry claim).
