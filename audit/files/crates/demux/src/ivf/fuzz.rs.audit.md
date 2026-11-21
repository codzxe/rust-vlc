# AUDIT: crates/demux/src/ivf/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 98 / 4 (1 harness fn, 3 budget consts; no in-file tests — the proof suites live in ivf/tests.rs + tests/ivf_fixtures.rs by design) |
| File verdict | UNREACHABLE (real, executed harness; callers: the ivf test suites and the never-built cargo-fuzz shim) |
| Reachable from shipped entry point? | NO for the harness (test/fuzz only). The `IvfDemuxer` it drives IS reachable: probe names `ivf` (signature/mod.rs:140) → `containers/mod.rs:56` `"ivf" => ivf::open(...)`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full Read; `cat fuzz/fuzz_targets/fuzz_demux_ivf.rs` (shim exists, calls this fn, own honest status header); `sed -n '470,560p' crates/demux/src/ivf/tests.rs` (the three fuzz-lite sweeps); `sed -n '285,305p' crates/demux/tests/ivf_fixtures.rs` (integration mutant/truncation sweep); `rg "MAX_FRAME_BYTES\|FRAME_CEILING_BYTES" ivf/mod.rs` (4 MiB pool / 16 MiB refusal — bounds claims verified); baseline `orchrun-test-workspace-nofailfast.log.txt:1696-1698` (3/3 fuzz-lite ok), `:2078-2086` (ivf_fixtures 5/5 ok, incl. `fuzz_lite_over_the_real_ffmpeg_bytes`) |

## 1. One-paragraph verdict
The strongest fuzz surface of the three demux harnesses in my set: the module's "proven by the deterministic in-crate sweeps" claim is backed by executed, load-bearing tests — 512 seeded bit-flip rounds with a determinism cross-check (two demuxers over the same bytes must agree on the terminal error code), an exhaustive every-prefix truncation sweep, 256 wild-size rounds that **assert the pool stays ≤ 2 × MAX_FRAME_BYTES** (an allocation-bound check, not just survival), and an integration suite that replays 512 mutants + step-7 truncations over real ffmpeg bytes — all green in the baseline. The harness body itself is clean: budget-before-call drain (≤500k), saturating seek-target arithmetic derived from the observed extent, non-negative clamps, no unwrap/index. Two documentation-level defects: the wiring-status paragraph still frames the cargo-fuzz target file as an unlanded Lead decision when the file exists with its own (clearer) status header, and the shim's "compiles on stable (`cargo check` in fuzz/)" claim has no committed run behind it. As with all 35 targets: the libFuzzer campaign has never executed.

## 2. What actually works (verified)
— Executed and passing in `orchrun-test-workspace-nofailfast.log.txt`: `ivf::tests::fuzz_lite_bit_flips_never_panic_or_hang` (:1696), `fuzz_lite_truncations_never_panic_or_hang` (:1697), `fuzz_lite_wild_declared_sizes_are_refused_not_allocated` (:1698), and integration `ivf_fixtures` 5/5 (:2078-2086) including `fuzz_lite_over_the_real_ffmpeg_bytes` and `truncated_derivatives_are_typed_never_panics`.
— The allocation-bound claim is asserted, not just stated: tests.rs:535-555 runs 256 rounds of wild declared sizes / u64::MAX pts and asserts `demuxer.pool_bytes() <= 2 * super::MAX_FRAME_BYTES` after each — the harness contract's "never allocates without a ceiling" has a test that would fail on a pool regression.
— Bounds claims verified in source: `MAX_FRAME_BYTES = 4 MiB` (pool, mod.rs:118), `FRAME_CEILING_BYTES = 16 MiB` (typed refusal before allocation, mod.rs:122), `PayloadSlots::new(2, MAX_FRAME_BYTES)` (mod.rs:338).
— Seek-target arithmetic re-derived: `ticks = last_ticks.saturating_mul(fraction)/3` for fraction 1..=3 → thirds of the observed extent; `ticks.max(0)` clamps before `Timestamp::new`; `i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3)` cannot fall back. Budget checked after each event with a hard 500k break.

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/ivf/fuzz.rs:12-19
Code:
```rust
//! The cargo-fuzz target file is a Lead-owned dependency decision
//! (`libfuzzer-sys`), exactly as the AVI/WAV/Ogg/ADTS cards recorded;
//! until it lands the harness logic is proven by the deterministic
//! in-crate sweeps (bit-flip/truncation families over the committed
//! ffmpeg-generated fixtures — under 200 KB total, MP §12's fuzz
//! budget — run by the unit + integration suites).
```
What is wrong: the target file EXISTS — `fuzz/fuzz_targets/fuzz_demux_ivf.rs` (added with the W1-12 scaffold, commit 5d34901e) calls `demux::ivf::fuzz::fuzz_demux_ivf(data)` and carries its own, clearer status header ("this package compiles on stable (`cargo check` in fuzz/) … the harness logic is proven by the in-crate deterministic hostile-input sweeps"). The in-crate paragraph still reads as if the target file itself were pending; the only outstanding item is the nightly campaign wiring. Secondary: the shim's "compiles on stable" claim has no committed run behind it (no baseline/gate entry compiles `fuzz/`; `fuzz_registry_check.sh` checks names/corpus/harness-fn existence only), and the "under 200 KB total" fixture-budget figure is asserted without a manifest sum.
Why it matters: same stale-wiring family as avi/fuzz.rs F-001 — the in-crate doc is the first thing an auditor reads for the fuzz posture; it under-reports landed state and carries two unverifiable figures.
What correct looks like: mirror the shim's status (target exists + compiles-on-stable claim either proven by a gate step or dropped; nightly wiring outstanding); cite the fixture sizes from a manifest.
Confidence: HIGH on the existence/staleness (cat + git); UNVERIFIED on "compiles on stable" and the 200 KB total (no committed run; no cargo allowed).

### F-002 [S4]
Where: crates/demux/src/ivf/mod.rs (`pub mod fuzz;`) consuming crates/demux/src/ivf/fuzz.rs:49
Code:
```rust
pub fn fuzz_demux_ivf(data: &[u8]) {
```
What is wrong / why / fix: identical to my avi/fuzz.rs F-002 and es/fuzz.rs pattern note — the fuzz module (and ivf's `test_support`) compile into every release binary while only tests and the never-built shim call them. Gate behind `#[cfg(any(test, feature = "fuzz"))]`.
Confidence: HIGH — Verification: read + caller census (ivf/tests.rs:501/526/529-531, tests/ivf_fixtures.rs:72/297/301, shim:19 — no other callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 36-44 | REAL | — | 500k / 64 / 3 with `[PROPOSAL]` markers; 500k rationale (frame counts) stated. |
| `fuzz_demux_ivf` | 49-98 | REAL | — | Budget-after-count break (≤ BUDGET+1 calls — off-by-none that matters); `last_ticks` tracked from Packet pts only (Stamp::Present); seek probes at 1/3, 2/3, 3/3 of observed extent via `reanchor` (the seek-family method the ivf demuxer exposes — aiff twin uses the same); every arithmetic step saturating/clamped; no unwrap in reachable paths (the single `unwrap_or` guards an infallible conversion). |

## 5. Test honesty (no in-file tests; the documented proof suites exist and ran)
| Test (elsewhere) | Would it fail if the code were broken? | Reason |
|---|---|---|
| ivf::tests::fuzz_lite_bit_flips_never_panic_or_hang (:491) | YES | 512 rounds × 1-8 seeded flips over `demo_stream` through the harness; PLUS every 64th round a determinism cross-check — two fresh demuxers over identical bytes must produce the same terminal error code (catches hidden state, not just panics). |
| ivf::tests::fuzz_lite_truncations_never_panic_or_hang (:523) | YES | Every prefix 0..len of the seed + `[]`, `b"DKIF"`, zeros, 0xFF×128 — exhaustive truncation class. |
| ivf::tests::fuzz_lite_wild_declared_sizes_are_refused_not_allocated (:535) | YES — and it asserts a BOUND | 256 rounds of 0xFF frame-size bytes + u64::MAX pts, asserting `pool_bytes() ≤ 2 × MAX_FRAME_BYTES` — fails on an allocation regression, the only fuzz-lite test in the demux crate that checks memory, not just survival. |
| tests/ivf_fixtures.rs::fuzz_lite_over_the_real_ffmpeg_bytes | YES | 512 mutants (1-8 flips, seeded LCG) + step-7 truncations over the real ffmpeg fixture through the harness. |
| tests/ivf_fixtures.rs::truncated_derivatives_are_typed_never_panics | YES | Truncation derivatives assert TYPED outcomes (conformance + survival together). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :6-10 | REAL (and exceeded — the wild-sizes test also asserts the pool bound) | Tests cited in §5, baseline lines. |
| "The cargo-fuzz target file is a Lead-owned dependency decision … until it lands" | :14-16 | STALE/misleading | Target file exists (cat; commit 5d34901e); only nightly wiring outstanding (F-001). |
| "proven by the deterministic in-crate sweeps (bit-flip/truncation families over the committed ffmpeg-generated fixtures … run by the unit + integration suites)" | :16-19 | REAL | ivf/tests.rs:491/523/535 + ivf_fixtures.rs — all executed ok in baseline (:1696-1698, :2078-2086). |
| "under 200 KB total, MP §12's fuzz budget" | :18-19 | UNVERIFIED | No manifest sum committed; fixtures are small (spot-checked sizes) but the figure has no proof artifact. |
| "Packet memory by MAX_FRAME_BYTES (pool) and FRAME_CEILING_BYTES (typed refusal before any allocation)" | :26-29 | REAL | mod.rs:118/122/338 + the pool assertion in the wild-sizes test. |
| "the walk is strictly positional and monotone" | :23-24 | REAL (cross-check) | ivf frame walk is positional (header 32 B + 12 B per frame — mod.rs read during bounds verification). |

## 7. Dead / duplicated / unreachable code
F-002 (release-lib shipping). No in-file duplication; the LCG in ivf/tests.rs is a commented twin of the integration file's — named provenance, acceptable.

## 8. Dependencies & flags
Crate-internal only (media-types time types, plugin_api, super::IvfDemuxer + test_support). No unsafe, no unwrap/expect, no panics. Consistent house style with the avi/es harnesses.

## 9. Open questions for the next agent
- The "compiles on stable" shim claim (fuzz_demux_ivf.rs:10) is testable by one `cargo check --manifest-path fuzz/Cargo.toml` gate step — cheap to prove, currently asserted on faith; suggest to the Lead/tooling wave.
- `fuzz_lite_wild_declared_sizes_are_refused_not_allocated`'s pool assertion is the model the avi/es harnesses lack — porting it to `MAX_IDX1_ENTRIES`/lane budgets would close the gap my avi/fuzz.rs F-001-adjacent note describes.
