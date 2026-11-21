# AUDIT: crates/demux/src/ps/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-21 |
| Lines / functions | 84 / 4 (3 consts, `fuzz_demux_ps`) |
| File verdict | REAL (harness code executes in baseline tests) — its "never panics/hangs/over-allocates" contract over ARBITRARY input is REAL-UNPROVEN: no coverage-guided campaign has ever run it |
| Reachable from shipped entry point? | COMPILED-IN BUT NOT EXECUTED at runtime: `ps/mod.rs:93 pub mod fuzz;` un-gated (ships in the release binary); callers are `ps/tests.rs:512,515,530` (in-crate `#[cfg(test)]` sweeps) and `fuzz/fuzz_targets/fuzz_demux_ps.rs:17` (a libFuzzer shim in the separate `fuzz/` workspace that NO gate builds or runs — see F-001). No `lumen`/`lumen-tauri` runtime path calls it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "fuzz_demux_ps" crates apps scripts .github` (callers: tests + shim only); `sed -n '93,94p' ps/mod.rs` (un-gated); `rg -n "MAX_RECOVERY_SCAN_BYTES\|MAX_PACKET_BYTES" ps/mod.rs` (exist + used, lines 102,107,359,812); `sed -n '495,535p' ps/tests.rs` (the actual sweep family); `rg -n "ps::tests::fuzz_lite" audit/runs/baseline/test-workspace.log.txt` (both green, lines 1880-1881); `.github/workflows/ci.yml:243-247` + `fuzz/Cargo.toml` head (campaign not wired into any gate); `sed -n` verification of every quote. |

## 1. One-paragraph verdict
The harness is competently shaped: one pass = construct + start + bounded drain (500k-event cap) + three data-derived seek probes with a 64-event post-seek drain each, exercising the post-seek and reanchor surfaces; every loop has a hard budget, and the demuxer-side ceilings it leans on (`MAX_PACKET_BYTES` 64 KiB × the 2-arena pool at mod.rs:359, `MAX_RECOVERY_SCAN_BYTES` window at mod.rs:812) really exist. But the name "fuzz" overstates the evidence: no fuzzer has ever executed this function. The libFuzzer scaffold is explicitly unwired ("building the campaign needs the Lead's nightly wiring" — the repo's toolchain is stable-pinned 1.99.1), and CI's "fuzz-registry" gate only checks name/file/README/harness consistency plus non-empty seed corpora — it runs zero fuzzing. What actually executes is a deterministic sweep: 2 fixtures × 120 rounds × 8 LCG byte-flips biased into the first 512 bytes, plus a truncation every 4th round (ps/tests.rs:495-518), green in baseline. That is a mutation test, not a fuzz campaign, and its flip-bias leaves corruption beyond byte 512 covered only by truncations.

## 2. What actually works (verified)
— Harness structure, read: all three loops budgeted (`FUZZ_EVENT_BUDGET` 500k at fuzz.rs:56-58, `FUZZ_POST_SEEK_DRAIN` 64 at 75-79, `FUZZ_SEEK_PROBES` 3 at 64); every error path breaks or returns (42-44, 53, 67-68, 77); seek probes are data-derived (fractions of the observed SCR extent, fuzz.rs:60-73) and exercise `seek` + `reanchor` + post-seek `next_event` — surface a pure walker would miss.
— The bounds the doc cites exist and are wired: `MAX_PACKET_BYTES = 64 * 1024` (ps/mod.rs:102) feeding a 2-arena pool (mod.rs:359), `MAX_RECOVERY_SCAN_BYTES = 64 * 1024` windowing recovery (mod.rs:812) — rg-verified.
— The sweep ran green in baseline: `ps::tests::fuzz_lite_mutations_of_the_committed_fixtures_never_panic ... ok` and `ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors ... ok` (test-workspace.log.txt:1880-1881). Mutation check: if `fuzz_demux_ps` panicked on any of the 270 mutant/truncated inputs, the first test fails — panic-freedom is genuinely proven for that family.
— Registry consistency is real: baseline row 18 (`fuzz_registry_check.sh` → PASS, "35 targets, diff empty") proves shim↔harness↔README agreement and non-empty seed corpora.

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/ps/fuzz.rs:33-34 (contract claim) + fuzz/fuzz_targets/fuzz_demux_ps.rs:7-10 (admission) + .github/workflows/ci.yml:244-246 (the gate that proves nothing about execution)
Code (fuzz.rs):
```
/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling.
pub fn fuzz_demux_ps(data: &[u8]) {
```
(shim):
```
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
```
(ci.yml):
```
      - name: "6. fuzz-registry (full_gate.sh #6; doc 04 §8.5)"
        if: always()
        run: bash scripts/fuzz_registry_check.sh
```
What is wrong: the doc-comment asserts an unbounded-input contract ("never panics, never hangs, never allocates without a ceiling") that nothing has ever tested against unbounded input. No coverage-guided fuzzer has executed `fuzz_demux_ps`: the `fuzz/` cargo-fuzz package is its own workspace, requires nightly (repo is stable-pinned 1.99.1), and is "NOT wired into any gate" (fuzz/Cargo.toml's own header). CI's fuzz step runs the registry-consistency script only — it validates names, files, README rows, harness existence and seed-corpus non-emptiness, and executes zero bytes of fuzzing. The deterministic in-crate family that DOES run is: 2 fixtures × 120 rounds × 8 flips biased to `((state >> 33) as usize) % 512.min(bytes.len())` (ps/tests.rs:508-510 — first 512 bytes only) + 30 truncations + 4 clean passes ≈ 274 executions, all derived from two known-good fixtures.
Why it matters: a hostile-input contract established on 274 fixture-derived mutants from one authoring tool (ffmpeg) is a smoke test wearing a fuzz harness's name. Panic/hang/OOM classes reachable only through deep-file corruption (the region beyond byte 512 is flip-untested), SCR/timestamp abuse, or pack-header shapes ffmpeg never emits are unproven. Anyone reading "fuzz target registered, gate green" concludes PS is fuzz-hardened; it is mutation-swept.
What correct looks like: either wire the campaign (nightly job or a stable-compatible harness runner with a time-boxed loop, e.g. the xtask pattern the repo already uses for benches) or restate the doc as what it is: "deterministic hostile-input sweep; campaign pending nightly wiring" — and widen the flip bias beyond the first 512 bytes.
Confidence: HIGH — Verification: read + rg (callers, gate script, CI workflow, fuzz/Cargo.toml) / ran (none — no campaign exists to run; that is the finding).

### F-002 [S4]
Where: crates/demux/src/ps/fuzz.rs:65
Code:
```
            let ticks = (last_scr.saturating_mul(fraction as u64) / FUZZ_SEEK_PROBES as u64) as i64;
```
What is wrong: unchecked `u64 → i64` cast on a data-derived value. `last_scr` is file-controlled; `saturating_mul(3)` can reach `u64::MAX`, and the `as i64` then wraps negative. Downstream this either fails `Timestamp::from_parts` (continue) or seeks toward a pre-zero target (typed refusal) — no panic — but the wrap is silent in a file whose entire purpose is bounds discipline on hostile values.
Why it matters: the harness means to probe odd targets; a wrapped negative target is fine surface, but an implicit wrap hides intent and would break loudly if `from_parts` ever accepted it as a valid pre-epoch stamp.
What correct looks like: `i64::try_from(...).unwrap_or(i64::MAX)` or clamp before the cast, with a comment that negative targets are deliberately out of range.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/demux/src/ps/mod.rs:93 (governs this file)
Code:
```
pub mod fuzz;
pub mod test_support;
```
What is wrong: same class as ts/test_support (my file 3, F-003): the fuzz harness and the test-support builders it consumes are compiled unconditionally into every shipped `lumen` binary, though nothing at runtime calls them.
Why it matters: dead payload in release builds; a dev-feature gate would keep the in-crate tests and drop the weight.
What correct looks like: `#[cfg(any(test, feature = "fuzz-support"))]` with the fuzz package enabling the feature.
Confidence: HIGH — Verification: read + rg (no runtime callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET (const) | 23-25 | REAL | — | 500k, `[PROPOSAL]`-marked rationale; cap enforced at 56-58 |
| FUZZ_POST_SEEK_DRAIN (const) | 27-28 | REAL | — | 64; enforced at 75-79 |
| FUZZ_SEEK_PROBES (const) | 30-31 | REAL | — | 3; fractions 1/3, 2/3, 3/3 of SCR extent |
| fuzz_demux_ps | 33-84 | REAL / contract REAL-UNPROVEN | S3, S4 | bounded on every path; seek+reanchor surface good; F-001 (no campaign), F-002 (cast) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ps::tests::fuzz_lite_mutations_of_the_committed_fixtures_never_panic (tests.rs:495-518) | YES — for its 274-input family | a panic in fuzz_demux_ps on any mutant/truncation fails the test; verified green in baseline (log:1880) |
| ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors (tests.rs:520-535) | YES | asserts the 4 clean fixtures drain without terminal errors (log:1881) |
| The "fuzz" gate in CI (ci.yml:244-246) | NO | proves registry consistency only; executes zero fuzzing (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Never panics, never hangs, never allocates without a ceiling" | fuzz.rs:33-34 | REAL-UNPROVEN (arbitrary input) / REAL (274-input deterministic family) | family: baseline log:1880-1881; campaign: none exists (fuzz/Cargo.toml header, shim :7-10) |
| "Recovery is bounded by MAX_RECOVERY_SCAN_BYTES; packet memory by MAX_PACKET_BYTES" | fuzz.rs:11-16 | REAL | ps/mod.rs:102, 107, 359, 812 (rg-verified wiring) |
| fuzz gate PASS = fuzzing happens | baseline INDEX row 18 ("35 targets, diff empty") | SHALLOW | gate script walks 4 name sets + corpus non-emptiness; runs no targets |

## 7. Dead / duplicated / unreachable code
- The whole module is runtime-dead in shipped builds (F-003); exercised only by in-crate tests.
- The libFuzzer shim (`fuzz/fuzz_targets/fuzz_demux_ps.rs`) is buildable-but-never-built by any gate — the scaffold's own header says so.
- No in-file duplication.

## 8. Dependencies & flags
- No feature flags, no `unsafe`, no external crates; depends on `plugin_api` (Demuxer/DemuxEvent), `media_types::time`, `super::{PsDemuxer, test_support, MAX_*}` — all in-crate.

## 9. Open questions for the next agent
- fuzz/Cargo.toml's header says "three registered fuzz targets" while the registry check counts 35 (baseline row 18) — stale scaffold doc; the fuzz/README owner should reconcile.
- If a campaign is ever wired, the flip-bias window (first 512 bytes, ps/tests.rs:508-510) should widen — deep-file corruption is currently only truncation-covered.
- ps/mod.rs owner: confirm `reanchor` semantics — the harness only reaches it after a successful `seek` (`is_ok() && ...` short-circuit, fuzz.rs:74); failed-seek reanchor paths are unfuzzed.
