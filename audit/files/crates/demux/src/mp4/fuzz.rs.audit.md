# AUDIT: crates/demux/src/mp4/fuzz.rs (whole file, 84 lines)
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-21 |
| Lines / functions | 84 / 2 functions + 3 budget constants |
| File verdict | REAL (harness executed in baseline; the coverage-guided campaign it fronts is PAPER-ONLY, scaffolded-not-wired) |
| Reachable from shipped entry point? | NO — deliberate: test/fuzz infrastructure, not linked into the `lumen` binary. Executed instead by `cargo test` lanes (baseline: `fuzz_lite_mutations_never_panic … ok`, `fuzz_lite_fragment_mutations_never_panic … ok` at test-workspace.log.txt:1816-1817; `malformed_mp4_fixtures_survive_the_fuzz_harness … ok` at :2202) and wired as libFuzzer shim `fuzz/fuzz_targets/fuzz_demux_mp4.rs` (campaign never run — F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -ln "fuzz_demux_mp4" crates/ apps/ scripts/ .github/` → 3 callers (mp4/tests.rs, tests/mp4_fixtures.rs, fuzz shim). `sed -n '1,40p' scripts/fuzz_registry_check.sh` → registry/corpus presence gate only. `rg -n "fuzz" .github/workflows/ci.yml` → CI step 6 runs only the registry check (:244-246). `rg -n "Status" fuzz/README.md` → "SCAFFOLDED, NOT WIRED — one Lead decision outstanding" (:17). `rg -n "MAX_MOOV_BYTES\|MAX_MOOF_BYTES\|MAX_SAMPLE_BYTES\|MAX_FRAGMENT_SCAN_BOXES" crates/demux/src/mp4/mod.rs` → all ceiling claims verified (:109,113,117,129,614,717). Baseline cites below. All quotes verified with `sed -n`. |

## 1. One-paragraph verdict
The harness body is real, bounded, and executed: construct → `start` → drain under a 500 000-event budget → three data-derived seek probes each followed by a 64-event drain, with typed errors deliberately swallowed per the documented contract. Its bounds claims all check out against mod.rs's actual ceilings (16 MiB moov, 1 MiB moof, 16 MiB samples through the 2-slot pool, 65 536-box fragment scan). It ran green in the baseline on two in-crate fuzz-lite sweeps and on 11 committed malformed fixtures. The problem is what "fuzz_demux_mp4" suggests vs. what runs: the coverage-guided libFuzzer campaign is scaffolded-not-wired (nightly Lead decision outstanding), CI only checks that the registry and seed corpus EXIST, and the five committed corpus seeds are consumed by no automated run — so the mutation coverage actually executed is stride-37/53 single-bit flips over two tiny fixtures plus truncations. Real harness, paper campaign.

## 2. What actually works (verified)
— `sed -n '2196,2205p' audit/runs/baseline/test-workspace.log.txt` → `malformed_mp4_fixtures_survive_the_fuzz_harness ... ok` (11 committed malformed derivatives through this harness) and `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed ... ok`.
— `rg -n "fuzz_lite" audit/runs/baseline/test-workspace.log.txt` → lines 1816-1817 both ok — the harness survives truncation prefixes, stride bit-flips, and a garbage tail.
— `rg -n "MAX_" crates/demux/src/mp4/mod.rs` → every bound the module doc claims is enforced in the demuxer: `MAX_SAMPLE_BYTES`/`MAX_MOOV_BYTES` = 16 MiB (:109,113), `MAX_MOOF_BYTES` = 1 MiB (:117), `PayloadSlots::new(2, MAX_SAMPLE_BYTES …)` (:614), `boxes_seen > MAX_FRAGMENT_SCAN_BOXES` (:717).
— Event-budget arithmetic hand-checked: `events += 1` before the cap check bounds every loop at `FUZZ_EVENT_BUDGET` iterations; the post-seek drain shares the same budget so total work per pass is bounded. ✓

## 3. Findings

### F-001 [S3] The registered campaign never executes: CI gates only registry/corpus presence; the committed seed corpus is consumed by nothing
Where: crates/demux/src/mp4/fuzz.rs:1-2 (registry claim), .github/workflows/ci.yml:244-246 (CI), fuzz/README.md:17 (status)
Code (fuzz.rs:1-2):
```rust
//! The MP4 fuzz harness (card 5.C1 `+FB`; doc 04 §8.5 registry name
//! `fuzz_demux_mp4`).
```
Code (fuzz/README.md:17):
```rust
## Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding
```
What is wrong: the fuzz target `fuzz/fuzz_targets/fuzz_demux_mp4.rs` is a libFuzzer shim that has never been run by any automated process: CI step 6 executes `bash scripts/fuzz_registry_check.sh`, which verifies names, shim→harness wiring, and that each target has a non-empty seed corpus — it never executes the harness on those seeds (`sed -n '1,40p' scripts/fuzz_registry_check.sh` confirms: four set-identity walks + coverage gate). The five committed MP4 seeds (`fuzz/corpus/…/seed_mp4_h264_aac.mp4`, `seed_mp4_frag.mp4`, `seed_mp4_chapters.mp4`, `seed_mp4_trunc0.mp4`, …) are therefore dead weight in every run. The executed mutation coverage is: stride-37 `0x80` flips + 7 truncations on one 4-sample progressive fixture, stride-53 `0x01` flips + 5 truncations on one 2-fragment fixture (mp4/tests.rs:1077-1112), and the 11 static malformed fixtures (mp4_fixtures.rs:243-248) — a vanishingly small slice of the mutation space a real campaign would explore.
Why it matters: the repo's defensive posture for the MP4 parser (the owner's core format) reads as "fuzzed"; the truth is "harness code executed on ~30 hand-picked inputs per test run". A panic reachable only via a two-byte interaction (e.g., a size field mutation plus a dependent offset) is invisible to stride single-bit flips.
What correct looks like: until the nightly decision lands, add a stable deterministic loop that at minimum RUNS THE SEED CORPUS through the harness in CI (the AVI crate already has the pattern — `tests/fuzz_loop_stable.rs` runs `fuzz_demux_avi` over seeded pseudo-random inputs on stable); MP4 has no such loop.
Confidence: HIGH — Verification: read (CI yml, registry script, README, callers) + baseline cites (1816-1817, 2202).

### F-002 [S4] `span()`'s silent unit fallback: a failed ns-base construction would seek in 1/1 ticks, not refuse
Where: crates/demux/src/mp4/fuzz.rs:35-41
Code:
```rust
fn span(nanos: u64) -> TimeSpan {
    let base = TimeBase::new(1, 1_000_000_000).unwrap_or(TimeBase { num: 1, den: 1 });
```
What is wrong: if `TimeBase::new(1, 1_000_000_000)` ever errored, the fallback silently builds a 1/1 base by literal struct construction (bypassing `TimeBase::new`'s validation) — seek targets would then be interpreted as 1-tick units, nine orders of magnitude off, with no signal. The fallback is dead in practice (a constant valid rational never fails), which makes it pure risk-shaped noise rather than defense.
Why it matters: swallow-shaped defensive code in a fuzz harness masks exactly the class of misbehavior the harness exists to surface; a future edit that breaks the base construction would quietly neuter every seek probe instead of failing loudly.
What correct looks like: `TimeBase::new(1, 1_000_000_000).unwrap_or_else(|e| unreachable!("ns base: {e}"))` — precondition panic in test infrastructure, or propagate the error.
Confidence: HIGH — Verification: read.

### F-003 [S4] Seek probes can only target 0–2.55 s: late-file and out-of-range seek paths are unprobed for longer inputs
Where: crates/demux/src/mp4/fuzz.rs:66-71
Code:
```rust
    // Seek probes at data-derived targets.
    for probe in 0..FUZZ_SEEK_PROBES {
        let at = data
            .get(data.len().saturating_sub(probe + 1))
            .map_or(0, |b| u64::from(*b) * 10_000_000);
```
What is wrong: targets are `last-3-bytes × 10 ms` → max 255 × 10 000 000 ns = 2.55 s. Any mutated input whose (often attacker-controlled) declared duration exceeds ~2.6 s never exercises a late-file seek landing, and the `demux.mp4.seek.out-of-range` refusal is only reachable for inputs that parse with duration < 2.55 s. The probe derivation is bounded and deterministic (good) but structurally front-loaded.
Why it matters: the seek machinery (keyframe lookup, fragment scan positioning) is the stateful half of the demuxer; the harness systematically under-fuzzes its back half.
What correct looks like: derive wider targets, e.g. scale a probe byte by the input's own length or mix in `u64::from(b) << (b % 40)` style magnitudes, still bounded.
Confidence: HIGH — Verification: read (arithmetic: 255 × 10 000 000 = 2 550 000 000 ns).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `span` | 35-41 | REAL | S4 | F-002 unit fallback; otherwise correct ns→Timespan. |
| `fuzz_demux_mp4` | 45-84 | REAL | S3/S4 | Full pass: construct→start→drain→3 seek probes; budgets correct (F-001 is the campaign gap around it, F-003 the probe narrowness); `Err(_)` swallows are the documented contract (:7-10). |
| `FUZZ_EVENT_BUDGET` (500 000) | 26-27 | REAL | — | Bounds both loops; shared across probes. |
| `FUZZ_POST_SEEK_DRAIN` (64) | 29-30 | REAL | — | Small bounded post-seek drain. |
| `FUZZ_SEEK_PROBES` (3) | 32-33 | REAL | — | Three probes; see F-003 for their reach. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fuzz_lite_mutations_never_panic` / `fuzz_lite_fragment_mutations_never_panic` (mp4/tests.rs:1077-1112, part2/W05-A05) | ONLY on panic/hang | The contract is survival; a budget that shrinks to 1 or a probe removed passes silently — no coverage assertion exists (inherent to fuzz harnesses, noted not filed). |
| `malformed_mp4_fixtures_survive_the_fuzz_harness` (mp4_fixtures.rs:243-248) | ONLY on panic/hang | Same contract; the companion `…only_ever_refuse_or_demux_typed` pins the typed outcomes separately (:250-276). |
| Baseline execution | — | All three ok: test-workspace.log.txt:1816-1817, 2202. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "registry name fuzz_demux_mp4" (doc 04 §8.5) | fuzz.rs:1-2 | REAL (registered) / PAPER-ONLY (campaign) | Registry gate PASS (baseline INDEX #18: 35 targets, diff empty); but no CI step or script executes the target — F-001. |
| "Structural reads are ceiling-checked inside the demuxer (moov/moof ≤ 16 MiB/1 MiB; samples ≤ 16 MiB each through the 2-slot pool)" | fuzz.rs:14-17 | REAL | mod.rs:109,113,117,614 (constants + `PayloadSlots::new(2, MAX_SAMPLE_BYTES…)`). |
| "The fragment scan is bounded by MAX_FRAGMENT_SCAN_BOXES" | fuzz.rs:18-19 | REAL | mod.rs:129 constant, enforced :717 (`boxes_seen > MAX_FRAGMENT_SCAN_BOXES`). |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-8 | REAL | Harness swallows `Err(_)` by design (:62,78); the typed flavors are pinned by the conformance suite (mp4_fixtures, baseline ok). |
| "Never panics by construction of the demuxer and the budgets" | fuzz.rs:43-44 | REAL-UNPROVEN | Executed on ~30 deterministic inputs per run (baseline ok); "by construction" is a claim about the whole demuxer that only a real campaign could support. |

## 7. Dead / duplicated / unreachable code
- The five committed MP4 corpus seeds are consumed by no automated run (F-001) — present for a campaign that never happens.
- The `unwrap_or` fallback in `span` is dead-in-practice (F-002).
- No duplication; the shim (fuzz/fuzz_targets/fuzz_demux_mp4.rs) correctly re-uses this harness verbatim rather than rewriting it.

## 8. Dependencies & flags
No feature gates; `#[cfg]`-free module under `pub mod mp4` (demux/src/lib.rs:51). No unsafe. Depends on `super::test_support::mem_stream` (test-support reuse in a fuzz module — fine) and `super::MAX_FRAGMENT_SCAN_BOXES` (verified). The libFuzzer shim lives in the workspace-excluded `fuzz/` package (nightly, not built by baseline — its `cargo check`-on-stable claim is UNVERIFIED here; no build ran).

## 9. Open questions for the next agent
- Who owns the "Lead decision outstanding" on nightly fuzz wiring (fuzz/README.md:17-37)? If the decision is months old, the scaffold is shelfware — worth an S2 at repo level once wave 15/17 (CI/tooling) weighs in.
- The AVI crate has a stable-toolchain seeded loop (`tests/fuzz_loop_stable.rs`); MP4, MKV, TS do not — check whether those crates' loops exist elsewhere (mkv_corpus/ts corpus tests are differential-only, not mutation loops).
- W05-A05 (tests.rs part2): the fuzz-lite sweeps live in your range — consider asserting event-count > 0 on the valid fixture pass so a silently-short-circuiting harness fails loudly.
