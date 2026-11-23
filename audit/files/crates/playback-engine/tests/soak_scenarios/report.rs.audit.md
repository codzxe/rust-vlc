# AUDIT: crates/playback-engine/tests/soak_scenarios/report.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 576 / 6 production callables + 3 tests (+3 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A by design (test-support code) — but EXECUTED in the workspace test lane: unit tests + the full `soak_scenario_suite` run in the baseline; its rendered output is verbatim-committed under docs/soak/raw/ |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
The soak report renderer is genuine, disciplined, and evidenced end-to-end: it
renders only what the runners measured, labels every per-hour figure an
EXTRAPOLATION, prints `n/a` for absent instruments instead of zero, records
wall-cap shortfalls as notes, and its output format matches the committed raw
reports byte-for-byte in their distinguishing lines. The worst-slope headline
carries an honest regression record (a past `+INFINITY` fold bug that made the
line dead code, fixed to `NEG_INFINITY` with a pinning test). The civil-date
algorithm is Hinnant's, pinned against four known dates including pre-epoch.
Two hygiene defects: a `unwrap_or(0.0)` that would print a fabricated "0.0 B/s"
if two always-paired fields ever drift apart, and a decimal-vs-binary "MB"
unit mix between the live-bytes and RSS columns of the same table.

## 2. What actually works (verified)
- Baseline (audit/runs/baseline/cargo-test-workspace.txt:4523-4534):
  `report::tests::civil_dates_match_known_days ... ok`,
  `report::tests::utc_timestamp_has_the_documented_shape ... ok`,
  `report::tests::worst_slope_headline_prints_the_largest_finite_slope ... ok`,
  and the full suite `test soak_scenario_suite ... ok` (:4534, CI-short
  manifest tests/soak/scenarios.toml, ~1 min budget per its header).
- Output-reality chain: docs/soak/raw/scenarios-sandbox__play-loop.md:14
  carries the exact summary header written at report.rs:128
  (`| scenario | kind | wall | cycles | live slope (MB/h) | RSS slope (MB/h) | verdict |`)
  and :4-5 carry the `Profile: release build, linux/x86_64` / `Captured:
  ...Z (UTC)` lines from report.rs:95-96/:118 — the committed reports were
  produced by this renderer (via scripts/soak-scenarios.sh + SOAK_SCENARIOS_REPORT).
- Field-pairing consistency verified in runner.rs:1214-1223:
  `live_mb_per_h = live_slope_b_per_s.map(|slope| slope * 3600.0 / 1e6)` —
  Some⟺Some, same fit, so the renderer's paired reads cannot disagree today.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak_scenarios/report.rs:257-267
Code:
```
    match result.verdict.live_mb_per_h {
        Some(mb) => {
            let _ = writeln!(
                out,
                "- Live-bytes slope: {:.1} B/s = **{:.3} MB/h** (EXTRAPOLATION from a {:.1} s \
                 window) — {}",
                result.live_slope_b_per_s.unwrap_or(0.0),
```
What is wrong: if `verdict.live_mb_per_h` is `Some` while
`live_slope_b_per_s` is `None`, the line prints a fabricated "0.0 B/s" next to
a real MB/h figure — violating the module's own discipline ("every absent
instrument is labeled not-measurable, never zero", :6-7). Today unreachable
(runner.rs:1219 derives both from the same Option), so hygiene not a live bug.
What correct looks like: assert the pairing or print `n/a` for the B/s figure.
Confidence: HIGH — Verification: read + runner.rs:1214-1223.

F-002 [S4]
Where: crates/playback-engine/tests/soak_scenarios/report.rs:136-138 vs runner.rs:1219
Code:
```
        let rss = match result.rss_slope_kb_per_s {
            Some(slope) => format!("{:.3}", slope * 3600.0 / 1024.0),
```
What is wrong: the live-bytes column converts B/s → MB/h decimally
(runner.rs:1219: `slope * 3600.0 / 1e6`) while the RSS column converts kB/s →
MB/h binarily (`* 3600.0 / 1024.0`). Both columns are labeled "MB/h" in the
same summary table (:128). The mismatch is ~4.9% — small, but the budgets
(target 5 / ceiling 50 MB/h) live in the decimal convention while the adjacent
RSS number is MiB. Confidence: HIGH — Verification: read both conversions.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ScenarioResult (struct) | 19-83 | REAL | — | measured facts only; every field has a producer in runner.rs |
| suite_markdown | 87-198 | REAL | — | NEG_INFINITY fold (fixed W24-R4 P2-2, pinned by test); EXTRAPOLATION labels; PASS/FAIL verdict from failures |
| write_scenario | 201-362 | REAL | S4 | F-001 unwrap_or(0.0); wall-cap note; violation listing |
| slope_words | 365-381 | REAL | — | boundary semantics match runner's `>=` ceiling check |
| fraction_word | 384-386 | REAL | — | "15%" |
| utc_now_iso8601 | 399-412 | REAL | — | std-only; pre-epoch clock degrades to 1970-01-01T00:00:00Z (safe) |
| civil_from_days | 415-426 | REAL | — | Hinnant algorithm; pinned on 4 known dates incl. pre-epoch (:552-558) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| worst_slope_headline_prints_the_largest_finite_slope (:505) | YES | pins the exact rendered headline for mixed-sign, all-negative, and no-slope inputs — the +INFINITY regression would fail case 1 |
| civil_dates_match_known_days (:552) | YES | 4 hardcoded dates incl. negative days |
| utc_timestamp_has_the_documented_shape (:562) | WEAK-MED | shape+plausible-year only (a wrong-but-shaped timestamp passes); acceptable for a timestamp |
All 3 pass in baseline (:4523-4526). The heavier proof is the suite itself
(soak_scenario_suite, :4534) which drives the real engine stack and asserts
budgets; the renderer is exercised on real measured results there.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every absent instrument is labeled not-measurable, never zero" | report.rs:6-7 | REAL (one unreachable exception) | n/a branches at :134/:138/:269-292; F-001 is the exception |
| "every per-hour figure ... is an EXTRAPOLATION" | report.rs:4-5 | REAL | :107-108, :163, :261, :281 all label it |
| "the five-scenario bounded soak RAN for real ... verbatim report committed" | docs/soak/soak-report-20261110.md | REAL | raw reports match renderer output format (verified :128 header, :95-96 profile lines); suite green in baseline |
| "CI-short default runs as part of cargo test --workspace" | main.rs:15 | REAL | baseline :4502-4534 shows the binary ran and passed |

## 7. Dead / duplicated / unreachable code
None. All fields of ScenarioResult are rendered; helpers are used. The
`#[allow(dead_code)]` on the shared 3.E4 modules lives in main.rs (documented
subset-API rationale), not here.

## 8. Dependencies & flags
testkit (Must, alloc_count serialize), std only. No features. The counting
allocator serialization discipline (process-global instrument, :432-437) is
correctly applied to every test.

## 9. Open questions for the next agent
- runner.rs (not my file): does `least_squares_slope` guard against
  degenerate windows (n=1, zero x-variance)? The renderer trusts its output.
- The nightly 4 h tier (.github/workflows/soak-nightly.yml exists) is
  "billing-blocked" per docs/soak/soak-report-20261110.md — has any nightly
  run ever executed? (The committed evidence is the ~31 min sandbox tier
  only; the 4 h tier's reality is UNVERIFIED from my files.)
