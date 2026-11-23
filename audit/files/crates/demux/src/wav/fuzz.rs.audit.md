# AUDIT: crates/demux/src/wav/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 251 / 8 (1 pub harness fn, 3 consts, 1 scrambler, 5 #[test]]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `pub mod fuzz` (wav/mod.rs:92) is compiled into the lib but nothing in production calls it; callers are integration tests (corpus_wav.rs:356,377; wav_aiff_fixtures.rs:714) and the unwired libFuzzer target fuzz/fuzz_targets/fuzz_demux_wav.rs:19. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `sed` fuzz/fuzz_targets/fuzz_demux_wav.rs (shim exists, nightly-unwired); `rg` callers (3 integration-test call sites); baseline cargo-test-workspace.txt:2135-2139 (all 5 sweep tests ok). |

## 1. One-paragraph verdict
A real deterministic fuzz harness with honest labeling: bounded event budget,
bounded post-seek drains, seek probes gated on seekability and a known rate,
all errors swallowed per the absence-of-panic contract. The in-crate sweeps
(empty/tiny, 64 escalating scrambles over a structured WAV, 64 scrambles + 9
truncations over a WAVEFORMATEXTENSIBLE file, 8 extreme size-field patches,
13 truncation cuts) all pass in the baseline, and three integration suites
route their mutated fixtures through the same harness. The coverage-guided
campaign itself has never run (libFuzzer nightly wiring is a pending Lead
decision — stated plainly in both this file and the target shim). Defects are
comment-vs-code mismatches: the seek-probe comment describes targets the
arithmetic does not produce, the probe count is off by one, and a dead
`landing` binding.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:2135-2139: all five `wav::fuzz::tests::*`
  sweeps "ok" — the harness executed over ~160 deterministic hostile inputs.
- read: budgets are real (`FUZZ_EVENT_BUDGET` checked at loop head 66-68,
  post-seek drain bounded at 106-111, no unbounded loop); `span / 4` divides
  by the constant 4 — no division-by-zero path.
- read: fuzz/fuzz_targets/fuzz_demux_wav.rs:19 routes `data` into this exact
  function — the shim and the module agree (one harness, not two).

## 3. Findings
F-001 [S4]
Where: crates/demux/src/wav/fuzz.rs:81-95
Code:
```
    // Seek probes at data-derived targets: zero, a quarter of the
    // observed pts span, its middle and a past-the-end stamp. Only
    // meaningful when the demuxer claims a seekability.
```
```
    let span = last_pts_ticks.saturating_mul(4);
    for quarter in 0..=FUZZ_SEEK_PROBES {
        let ticks = span / FUZZ_SEEK_PROBES as i64 * quarter as i64;
```
What is wrong: the comment promises probes at "a quarter of the observed pts
span, its middle and a past-the-end stamp", but `span = last×4` and
`ticks = span/4 × quarter` produce probes at 0, 1×, 2×, 3×, 4× the last
observed pts — four probes at or past the end, none inside the span. The
described in-span coverage (25%, 50%) does not exist. The harness still
exercises the seek surface, but the comment misdescribes what is exercised.
Confidence: HIGH — Verification: read (arithmetic).

F-002 [S4]
Where: crates/demux/src/wav/fuzz.rs:47-48, 94
Code:
```
/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 4;
```
What is wrong: the loop is `0..=FUZZ_SEEK_PROBES` — 5 probes, not the 4 the
constant documents.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/wav/fuzz.rs:101-105
Code:
```
        if let Ok(landing) = demuxer.seek(target) {
            // A landing that maps commits through reanchor, then
            // drains a bounded window from the new position.
            if demuxer.reanchor(target).is_ok() {
                let _ = landing;
```
What is wrong: `landing` is bound and immediately discarded — the seek
landing value (which conformance tests elsewhere pin) contributes nothing
here; binding it only to drop it is noise. Also note seek-then-reanchor
drives the target twice per probe — intentional surface expansion, but
undocumented.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/wav/fuzz.rs:12-18 (and fuzz/fuzz_targets/fuzz_demux_wav.rs:7-10)
Code:
```
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_wav.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI card recorded; until it lands the harness logic is proven
//! by the deterministic in-crate sweeps here and by the integration
//! corpus run over the mutated fixture family.
```
What is wrong: the coverage-guided campaign has never executed — "proven" is
too strong a word for ~160 deterministic passes over ≤1 KiB inputs plus
fixture-derived mutations; no corpus, no coverage feedback, no crash
regression. The honesty labeling (Lead-owned, pending) keeps this out of
fraud territory, but the gap is real: hostile WAV input space is sampled at
handful-of-points depth only.
Confidence: HIGH — Verification: read + fuzz/README.md:21-27 (nightly
opt-in pending); campaign run would be required to upgrade the verdict.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_* consts | 37-48 | REAL | S4 | F-002 off-by-one doc |
| fuzz_demux_wav | 53-115 | REAL | S4 | F-001 probe comment, F-003 dead binding; budgets real |
| Scrambler (xorshift64*) | 124-144 | REAL | — | deterministic, no dep |
| empty_and_tiny_inputs_survive | 146-152 | REAL | — | |
| scrambled_structured_files_survive | 154-168 | REAL | — | 64 escalating scrambles |
| scrambled_extensible_headers_survive | 170-215 | REAL | — | 64 scrambles + 9 fmt-region cuts |
| adversarial_size_fields_survive | 217-240 | REAL | — | u32::MAX/0/1 at 3 size sites |
| truncations_at_every_header_site_survive | 242-250 | REAL | — | 13 cuts |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all five | YES (for panics/hangs) | any panic in the demuxer surfaces as test failure; budgets catch hangs. NO for wrong-but-quiet behavior — the contract is absence-of-panic only (documented) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never panics, never hangs, never allocates without a ceiling" | fuzz.rs:50-52 | REAL | budgets at 66/106; demuxer ceilings cited; baseline sweeps ok |
| "proven by the deterministic in-crate sweeps" | fuzz.rs:16-17 | SHALLOW | ~160 deterministic passes ≠ proof; no campaign ever ran |
| probe targets "zero, a quarter ... its middle and a past-the-end" | fuzz.rs:81-82 | FALSE | arithmetic yields 0,1×,2×,3×,4× last pts (F-001) |

## 7. Dead / duplicated / unreachable code
`fuzz_demux_wav` is unreachable from shipped entry points (test-only
surface). `let _ = landing` dead binding. No duplication.

## 8. Dependencies & flags
No external deps (hand-rolled scrambler). The libFuzzer target needs nightly
+ libfuzzer-sys (Lead decision pending, fuzz/README.md:21-27). No feature
gates in this file.

## 9. Open questions for the next agent
- Once nightly wiring lands, does the campaign find anything in the first
  hour? (The deterministic sweeps found nothing; that is weak evidence.)
- Why do `seek` and `reanchor` both get called per probe — does reanchor
  double the work per landing, and is that the engine's real call pattern?
