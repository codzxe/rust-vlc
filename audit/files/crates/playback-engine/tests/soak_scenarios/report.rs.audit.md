# AUDIT: crates/playback-engine/tests/soak_scenarios/report.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 576 / `ScenarioResult` (data) + `suite_markdown` + `write_scenario` + `slope_words`/`fraction_word` + `utc_now_iso8601`/`civil_from_days` + 3 unit tests |
| File verdict | REAL (the triage artifact renderer; executed end-to-end in the W00 baseline via the CI-short suite run) |
| Reachable from shipped entry point? | N/A (test-harness module of the soak_scenarios binary). It ran for real: `soak_scenario_suite` (main.rs:184) renders this report on every merge-gate run. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | Read (full file); unit-conversion cross-check against runner.rs:1219 (`* 3600.0 / 1e6`); sed verification of quoted ranges; cross-ref W03-A04's soak/report.rs audit (the sibling file's RSS unit bug — distinct file, not contradicted). No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The markdown report renderer for the scenario suite, with a genuinely
honest reporting discipline: every per-hour figure is labeled an
EXTRAPOLATION from its measured window, absent instruments print
"n/a" or a NOTE (never zero), the wall-cap shortfall is reported
rather than hidden, and the worst-slope headline refuses to render
when no slope is definable — with the W24-R4 P2-2 regression (a
`+INFINITY` fold identity that made the headline dead code) documented
at the fold site AND pinned by a three-case unit test. The std-only
UTC clock (Hinnant's `civil_from_days`) is tested against four known
dates including pre-epoch. The renderer ran for real in the W00
baseline (the suite test prints this report for the CI-short
profile). Three hygiene findings: a latent fabrication seam
(`unwrap_or(0.0)` in an artifact whose motto is "nothing here is
invented"), a mixed MB/MiB convention between the live and RSS
columns (both labeled MB/h, ~5% apart), and untested rendering paths
(the FAIL verdict block and the growth-number lines).

## 2. What actually works (verified)

— Executed in the W00 baseline: `soak_scenario_suite` calls `suite_markdown` (main.rs:222) and prints it — the renderer's happy path (PASS verdict, all scenario sections) ran against real measurement data, not only unit tests.
— The W24-R4 P2-2 regression is fixed and pinned (:151-169): the fold over `live_mb_per_h` starts at `NEG_INFINITY`, so the headline renders the largest finite slope; the unit test (:504-546) covers mixed signs (max = the only positive), all-negative (max of negatives is finite and must render), and no-slope-anywhere (headline must stay absent — "never invent a number").
— Honest-absence discipline: `n/a` for missing instruments (:134, :288-292), NOTE lines for absent platforms and wall-cap shortfalls (:225-233), and the count scenarios' span line states exactly which cycles the budgets cover (:296-303 — the W24-R4 P2-3 fix's reporting half).
— The verdict block mirrors the test's own assert: failures collected and listed (:177-196) — the report and the pass/fail decision cannot diverge in structure.
— `civil_from_days` tested against the epoch, 2024-01-01, 2026-11-11, and pre-epoch −1 (:551-558); timestamp shape pinned (length 20, separators, Z, plausible year range :561-575).
— Mutation reasoning (UNVERIFIED, no cargo): restore the `INFINITY` fold → `worst_slope_headline_prints_the_largest_finite_slope` fails (headline absent); break the date algorithm → `civil_dates_match_known_days` fails; swap the verdict word bands → the headline/section tests would catch the band text for the three synthetic slopes.

## 3. Findings

### F-001 [S4] A latent fabrication seam: `unwrap_or(0.0)` for the B/s figure in an artifact whose contract is "nothing here is invented"

Where: crates/playback-engine/tests/soak_scenarios/report.rs:263
Code:
```rust
                "- Live-bytes slope: {:.1} B/s = **{:.3} MB/h** (EXTRAPOLATION from a {:.1} s \
                 window) — {}",
                result.live_slope_b_per_s.unwrap_or(0.0),
```
What is wrong: if `verdict.live_mb_per_h` is `Some` but `live_slope_b_per_s` is `None` (an inconsistent `ScenarioResult`), the report prints "0.0 B/s = X.XXX MB/h" — a fabricated 0.0 next to a real extrapolated figure, in the triage artifact. Today the invariant holds structurally (runner.rs:1213-1225 derives `live_mb_per_h` from the slope; `None` slope ⇒ `None` verdict), so the seam is unreachable — but the module's own header (:17-18 "nothing here is invented") and the headline test's comment (:535-536 "never invent a number") are the stated contract, and this default invents a number the moment a future caller breaks the invariant.
Why it matters: the report is the evidence artifact cited in soak docs; a contradictory 0.0-B/s-with-nonzero-MB/h line would poison triage silently.
What correct looks like: derive both prints from one source — print the B/s from the same value the verdict was computed from, or assert the consistency (`debug_assert!`/`must`).
Confidence: HIGH — Verification: read (both fields' producers in runner.rs).

### F-002 [S4] Two MB conventions in one report: live slopes use decimal MB (÷1e6), RSS slopes use binary MiB (÷1024) — both labeled "MB/h"

Where: crates/playback-engine/tests/soak_scenarios/report.rs:137 (and :284) vs runner.rs:1219
Code:
```rust
            Some(slope) => format!("{:.3}", slope * 3600.0 / 1024.0),
```
```rust
                slope,
                slope * 3600.0 / 1024.0
```
vs runner.rs:1219:
```rust
    let live_mb_per_h = live_slope_b_per_s.map(|slope| slope * 3600.0 / 1e6);
```
What is wrong: the live-bytes column converts B/s → MB/h with 1e6 (decimal MB); the RSS column converts kB/s → "MB/h" with 1024 (binary MiB). Both columns and both the summary table and the per-scenario section label the result "MB/h". The same nominal unit thus means two things ~4.9% apart across columns of the same table.
Why it matters: anyone comparing the live slope against the RSS slope (the report exists to be compared — "kernel truth, arenas included") compares mismatched units; the target/ceiling budgets (5.0 / 50.0 MB/h, decimal) are only meaningful against the live column.
What correct looks like: one conversion constant for the report (either both decimal or both binary, with the unit string matching).
Confidence: HIGH — Verification: read (all three conversion sites).
Cross-ref: W03-A04's audit of the SIBLING file `tests/soak/report.rs` found an RSS-unit defect there ("RSS MB/h 1024× off") — a different, larger bug in a different file; this file's issue is the mixed convention, not a 1024× error. The two findings are consistent (the family's RSS unit hygiene is sloppy in both).

### F-003 [S4] The FAIL path and the growth-number lines have no rendering test

Where: crates/playback-engine/tests/soak_scenarios/report.rs:188-196, 296-352
Code:
```rust
        let _ = writeln!(
            out,
            "\n## Verdict\n\nFAIL — {} violation(s):\n",
            failures.len()
        );
```
What is wrong: the three unit tests cover the headline, the dates, and the timestamp shape. Nothing tests: the FAIL verdict block (violation listing), the wall-cap NOTE, the fd/RSS/live growth lines, the measured-span line, the drift-mean line, or the ops/decoding lines. The synthetic-result helper (`result_with_slope`, :462-496) hardcodes all of those to `None`/defaults, so even the suite's own unit tests never render them.
Why it matters: the report is the triage artifact; a formatting regression in the FAIL block or the growth lines would ship silently (the PASS path is exercised by every merge-gate run, the FAIL path only by an actual violation — exactly the moment triage needs it). Severity stays S4 because the pass/fail DECISION is the test's assert (main.rs:237-244), not the report — only the artifact's usefulness is at risk.
What correct looks like: one render test with a synthetic violating result asserting the VIOLATIONS section, the growth lines, and the wall-cap NOTE appear.
Confidence: HIGH — Verification: read (all three tests + the helper's defaults).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ScenarioResult` | 19-83 | REAL | — | pure data carrier; every field documents its None semantics |
| `suite_markdown` | 87-198 | REAL | S4 | header (profile/OS/arch/UTC/wall/method/instruments), summary table, worst-slope headline (P2-2 fix pinned), verdict block (F-003 untested path) |
| `write_scenario` | 201-362 | REAL | S4 | the per-scenario facts; F-001 seam at :263; F-002 conversions; wall-cap and NOTE lines honest |
| `slope_words` | 365-381 | REAL | — | four bands incl. negative-is-returning |
| `fraction_word` | 384-386 | REAL | — | "15%" |
| `utc_now_iso8601` | 399-412 | REAL | — | pre-epoch clock reads as 1970-01-01T00:00:00Z (`unwrap_or(0)`) — impossible in practice, visibly wrong if it ever happened |
| `civil_from_days` | 415-426 | REAL | — | Hinnant's algorithm; 4 known dates pinned |
| test `worst_slope_headline_prints_the_largest_finite_slope` | 504-546 | REAL | — | three cases incl. both edges |
| test `civil_dates_match_known_days` | 551-558 | REAL | — | epoch/2024/2026/pre-epoch |
| test `utc_timestamp_has_the_documented_shape` | 561-575 | REAL | — | shape + plausible year |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| worst_slope_headline... | YES | exact "**0.060 MB/h**" / "**-0.058 MB/h**" / absence asserts |
| civil_dates_match_known_days | YES | exact tuples |
| utc_timestamp_has_the_documented_shape | YES (shape-level) | length/separators/Z/year range — would not catch a wrong-but-plausible time |

All three ungated → ran in the W00 baseline. The renderer itself also ran end-to-end (PASS path) inside `soak_scenario_suite`.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every number the run actually measured … nothing here is invented" | header :2-4, :17-18 | REAL with one latent seam | F-001: the `unwrap_or(0.0)` default contradicts the motto if the invariant ever breaks |
| "Every per-hour figure … labeled an EXTRAPOLATION" | :4-7 | REAL | :107-108, :261-262, :281-282 |
| "every absent instrument is labeled not-measurable, never zero" | :6-7 | REAL | n/a strings + NOTE lines; the one exception is the impossible-clock 1970 default (table note) |
| unit "MB/h" | summary table :128, sections :262/:284 | SHALLOW | two conventions under one label (F-002) |

## 7. Dead / duplicated / unreachable code

- None dead. `fraction_word` has a single call site (fine for a formatting helper).

## 8. Dependencies & flags

No feature flags, no env knobs. Part of the soak_scenarios binary (default CI-short profile runs it in every merge gate; W00 baseline). Std-only (`SystemTime` + hand-rolled civil calendar — the C5 zero-dep discipline holds; `std::fmt::Write` into a `String`).

## 9. Open questions for the next agent

- F-002's mixed convention: before any soak doc compares live vs RSS slopes, pick one unit — the claim ledger should check docs/soak/soak-report-*.md for figures quoted from the RSS column (they are MiB/h labeled MB/h).
- The FAIL-path rendering (F-003) will first execute in anger during a real violation — a formatting bug found then is a triage-delayed bug.
