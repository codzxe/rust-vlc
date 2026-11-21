# AUDIT: crates/codecs/src/video/h264/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 101 / 4 (FUZZ_NAL_BUDGET, fuzz_codec_h264, 2 tests) |
| File verdict | UNREACHABLE (real harness + wired shim; fuzz campaigns never run; no shipped entry point) |
| Reachable from shipped entry point? | NO — fuzz infrastructure: the only callers are the in-crate tests (baseline test log :548-549) and `fuzz/fuzz_targets/fuzz_codec_h264.rs` (read: `fuzz_target!(\|data: &[u8]\| { codecs::video::h264::fuzz::fuzz_codec_h264(data); })`), whose campaign requires Lead-owned nightly wiring that does not exist (fuzz/README.md). Compiled unconditionally into the lib (`pub mod fuzz;` h264/mod.rs:52) — dead weight in shipped binaries. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read tool full file + fuzz/fuzz_targets/fuzz_codec_h264.rs + scripts/fuzz_registry_check.sh:1-40; `ls fuzz/corpus/fuzz_codec_h264/` → 2 seeds (seed_mkv_h264_aac.mkv, seed_mp4_h264_aac.mp4); Read parser/nal.rs:378-440 (AvcNals termination invariant — every path advances or ends); baseline log :548-549 (2/2 tests ok), INDEX run #18 (fuzz_registry_check PASS = names+corpora only) |

## 1. One-paragraph verdict
A well-shaped context-free fuzz harness: Annex-B walk with per-NAL SPS/PPS/SEI parses over a reused RBSP buffer, the avcC record walk, and the length-prefixed framing walk at all three legal length sizes, with a 4096-NAL parse budget and every error deliberately swallowed per the documented absence-of-panic contract. The termination story checks out (I verified AvcNals::next cannot spin: zero-length and truncation both set pos to end). The decisive defect is evidentiary, not structural: **no fuzzing has ever executed** — the cargo-fuzz campaign is un-wired (Lead-owned nightly), the baseline's "fuzz_registry_check PASS" validates only name/corpus consistency, and the entire panic-freedom guarantee for the H.264 parser — the parser the shipped CLI uses on every h264 MP4 — rests on three deterministic inputs in this file plus the unit suites. Docs elsewhere cite "the fuzz targets … hammer exactly this guarantee" as if it were an executed fact.

## 2. What actually works (verified)
— 2/2 deterministic tests ok in baseline (test-workspace.log.txt:548-549: `empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive`).
— Shim wiring real: fuzz/fuzz_targets/fuzz_codec_h264.rs calls exactly this harness; registry gate (baseline run #18, PASS, 35 targets) verified the four-way name/corpus consistency including this target's non-empty corpus (2 seeds on disk, verified).
— Termination audit (read): pass 4's exhaustion loop cannot hang — AvcNals::next (parser/nal.rs:404-440) terminates on empty, on `rest.len() < length_size` (pos→end), on zero-length prefix (pos→end, "refuse to spin"), and on truncation (pos→end); success advances by `len + length_size ≥ 1`. Pass 1-2's `take(FUZZ_NAL_BUDGET)` bounds parse count; pass 3 is a single typed parse.
— Error swallowing is deliberate and documented ("damage is expected to surface as a typed error and is swallowed here") — `let _ =` here is correct, not a checklist violation.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/fuzz.rs:15-16 (the contract claim), 30-35 (the honest wiring note); consumers of the guarantee: crates/codecs/src/lib.rs:50-52 and h264 parser docs
Code:
```
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here.
```
What is wrong: the contract is asserted but the instrument has never run. The cargo-fuzz target exists (fuzz/fuzz_targets/fuzz_codec_h264.rs) but its campaign requires the Lead's nightly wiring, which is absent; the baseline's only fuzz-related run (`scripts/fuzz_registry_check.sh`, run #18 PASS) verifies [[bin]]↔file↔README↔harness-fn name consistency plus non-empty seed corpora — it executes nothing. Executed evidence for "no panics on any input" for the H.264 parser = this file's 3 fixed inputs (`[]`, `[00 00 00 01]`, `[00 00 01 67]`) + 3 hostile shapes (all-00, all-FF, structured flood) + the unit suites. Meanwhile codecs/src/lib.rs:50-52 states the fuzz targets "hammer exactly this guarantee" — present tense for a campaign that has never fired.
Why it matters: the H.264 parser is on the SHIPPED decode path (sw decoder feeds through the in-house parser's framing/EPB bridge, codecs/Cargo.toml:169-173); its hostile-input guarantee is the strongest-sounding and least-executed claim in the codec stack. My hevc slice.rs audit (S2, ceil_log2 underflow) demonstrated concretely that this class of defect survives both the unit suites and release-profile "never panic" sweeps — the same blind spot applies to the h264 parser, which IS reachable.
What correct looks like: wire the campaign (one CI job: `cargo +nightly fuzz run fuzz_codec_h264 -- -max_total_time=60` across the 35 targets), or until then reword every "fuzz hammers/guarantees" doc claim to "fuzz targets exist; campaigns not yet run; deterministic sweeps only" — this file's own wiring note (:25-29) already has the honest wording; lib.rs and the parser docs do not.
Confidence: HIGH — Verification: read (shim, README, registry script, baseline INDEX run #18 semantics); no campaign artifacts anywhere in audit/runs/baseline/.
REQUEST-ORCH-RUN: `bash -c 'cd fuzz && cargo +nightly fuzz run fuzz_codec_h264 -- -max_total_time=60'` (or the orchestrator's equivalent) to convert the guarantee from asserted to executed.

### F-002 [S4]
Where: crates/codecs/src/video/h264/fuzz.rs:44-47
Code:
```
    // Pass 1 + 2: the Annex-B walk with per-NAL parameter-set/SEI
    // parses (reusable RBSP buffer — steady state performs no
    // allocation per NAL).
```
What is wrong: the "no allocation per NAL" claim is true only for the SPS/PPS sub-passes (`parse_with_buffer(&mut rbsp)`); the SEI sub-pass at :53-55 calls `unescape_rbsp(nal)`, which returns a fresh owned `Vec` per NAL — one heap allocation per NAL in steady state. The module-level bounds claim (:33-35 "the RBSP buffer is reused across NALs") has the same half-truth.
Why it matters: the file's §12 memory story is its headline; a reader optimizing or auditing allocation behavior would be misled about the SEI lane. Bounds themselves still hold (each allocation ≤ NAL size ≤ input).
What correct looks like: extend `parse_with_buffer`'s reuse to the SEI lane (an `unescape_rbsp_into(nal, &mut buf)` variant — the hevc parser already has exactly that shape, hevc/parser/nal.rs `unescape_rbsp_into`), or fix the comment to "SPS/PPS reuse the RBSP buffer; SEI unescapes into a per-NAL buffer".
Confidence: HIGH — Verification: read (unescape_rbsp signature/return in parser/nal.rs; contrast hevc's unescape_rbsp_into used at hevc/parser/slice.rs:179).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-39 | REAL | — | scope list verified against code; wiring-status note honest (the one place that is) |
| `FUZZ_NAL_BUDGET` | 40-43 | REAL | — | 4096, `[PROPOSAL]`-marked; applied to pass 1-2 only (pass 4 is input-bounded — verified) |
| `fuzz_codec_h264` | 46-65 | REAL | S3/S4 | F-001 (unexecuted guarantee), F-002 (SEI alloc comment); pass 4 walks framing only — NAL *contents* from length-prefixed framing are not parsed here, but the same parse surface is covered via pass 1-2 with equivalent bodies (no gap in substance); all errors deliberately swallowed per contract |
| test `empty_and_marker_inputs_survive` | 69-76 | REAL | — | 3 fixed inputs; would fail on a panic in any walked surface |
| test `hostile_wide_inputs_survive` | 78-90 | REAL | — | all-00/all-FF floods + structured start-code flood; would catch panics/hangs on these shapes only (see F-001) |

## 5. Test honesty
2/2 ok in baseline (:548-549).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES (for these inputs) | any panic/index error on empty or start-code-only buffers fails it |
| hostile_wide_inputs_survive | YES (for these shapes) | the all-00 flood drives maximal NAL splits through every parse; a bounds regression on wide inputs fails it — but 6 fixed shapes are not a fuzz campaign (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through every parser surface of the landed parser self-build" | fuzz.rs:5-7 | REAL | passes 1-4 verified against the parser's context-free surface; the slice-header seam's exclusion is documented and correct (needs active SPS/PPS) |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :15-16 | REAL-UNPROVEN | the contract is the design intent; the instrument has never executed (F-001) — deterministic sweeps only |
| "Memory is bounded by the input: NAL views are zero-copy, the RBSP buffer is reused across NALs" | :33-35 | SHALLOW | true for SPS/PPS; the SEI lane allocates per NAL (F-002); overall input-boundedness holds |
| "building the campaign needs the Lead's nightly wiring … the harness logic is meanwhile proven on stable by the deterministic sweeps" | :25-29 (and the shim's twin comment) | REAL | honest; this is the wording lib.rs:50-52 should have |
| codecs lib.rs: "the fuzz targets `fuzz_codec_mjpeg` / `fuzz_codec_pcm` … hammer exactly this guarantee" | crates/codecs/src/lib.rs:50-52 | FALSE (as executed evidence) | no campaign has ever run (F-001); registry check is name/corpus consistency only |
| fuzz registry "35 targets, diff empty" (baseline run #18) | audit/runs/baseline/INDEX.md #18 | REAL (for what it checks) | script read: four-way name consistency + non-empty corpora; zero execution |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped entry points by design (fuzz infra). No internal dead code. Near-duplication: this harness's shape (annexb walk + param parses + config-record walk + length-prefixed walk) is mirrored by hevc/fuzz.rs — acceptable, the codecs differ enough that a shared harness would be forced.

## 8. Dependencies & flags
In-crate only: `super::parser::{nal, sps, pps, sei}`. No unsafe, no feature gate (`pub mod fuzz` unconditional — compiled into shipped binaries as dead weight, ~100 lines). The fuzz/ package itself (libfuzzer-sys) is workspace-external and never built by release.yml.

## 9. Open questions for the next agent
- The orchestrator/Lead: 35 registered fuzz targets, zero executed campaigns — this is the single highest-leverage conversion of paper guarantees to executed evidence in the codec stack (one CI job). Logged as REQUEST-ORCH-RUN.
- parser/nal.rs owner: my termination read of `AvcNals::next` (404-440) found it sound — the zero-length "refuse to spin" guard at :424-430 is exactly the invariant the fuzz pass-4 comment relies on; please pin it with a unit test (zero-length prefix followed by more data → exactly one Err then None) if not already.
- Whoever owns sw.rs: the sw decoder is fed through "the in-house parser's AnnexBNals/AvcNals/unescape_rbsp_into bridge" (Cargo.toml:171-173) — note that bridge uses `unescape_rbsp_into` while this fuzz harness's SEI lane uses the allocating `unescape_rbsp`: two unescape entry points to keep in sync.
