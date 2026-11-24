# AUDIT: tools/mem-audit/src/report.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 361 / 9 fns + ClassSummary + 3 tests |
| File verdict | REAL (report formatting/aggregation; 3/3 unit tests green in committed baseline) |
| Reachable from shipped entry point? | NO (lumen-mem-audit dev tool; not in the release lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
Clean report layer: the MEMAUDIT/MEMSUM/MEMNOOP line protocols, the path
auto-classifier, per-class aggregation with medians/maxima, and honest "-" for
unavailable values (no packets fed / no /proc RSS). All three unit tests ran
green in the committed baseline. Nothing fake; two hygiene-grade nits only.

## 2. What actually works (verified)
- Baseline (cargo-test-workspace.txt:3286-3288): `test report::tests::
  classifier_rules ... ok / entry_parsing ... ok / median_shape ... ok` (inside
  PACKAGE lumen-mem-audit, 9 passed / 0 failed, :3293).
- Read end-to-end; the classifier test cases were checked against the rules
  table (report.rs:47-56): all 8 assertions match the documented order
  (malformed/`__` before valid before aux-*).

## 3. Findings
F-001 [S4] Where: report.rs:60 `if lower.contains("/malformed/") || lower.contains("__")`.
What is wrong: the `__` rule matches the whole path, not the file base —
`media/my__file.avi` or a parent dir like `/data/__archive/x.avi` classifies as
`fixture-malformed` even for pristine media. Documented as "contains __", so
behavior = doc; the label just becomes a lie for such paths. Confidence: HIGH —
read.
F-002 [S4] Where: report.rs:246 `values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))`.
What is wrong: NaN compares Equal — a NaN entry makes the sort a stable no-op
for that element and the median semi-meaningless. Inputs are alloc/packet
ratios (never NaN unless fed==0, excluded at :272-275), so unreachable in
practice; noted per checklist 6.2. Confidence: HIGH — read.
Zero-defect verification for the rest: checked every function for fakeness
(constant returns / ignored params / stub seams — none), arithmetic overflow
(u64 counters, no casts on hostile input — `as f64` only for ratios),
format-injection (error text appended verbatim by design, `error=` last —
documented at :30), aggregation correctness (max over open+walk peaks = union
property stated at :14-18 and implemented at :113, :280-282), and test honesty
below.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| classify | 58-77 | REAL | S4 | `__` whole-path match (F-001) |
| parse_entry | 82-99 | REAL | — | CLASS=path w/ charset check; invalid falls to whole-arg classify |
| memaudit_line | 104-159 | REAL | — | 25-field protocol, "-" placeholders, error last |
| verdict_name | 162-168 | REAL | — | 3 verdicts |
| memnoop_line | 173-175 | REAL | — | floor line |
| ClassSummary + memsum_line | 178-238 | REAL | — | aggregate line, Option "-" |
| median | 242-253 | REAL | S4 | NaN-Equal sort nit (F-002) |
| summarize | 258-311 | REAL | — | refused runs counted, never dropped; distinct files not inflated |
| tests (3) | 318-360 | REAL | — | green in baseline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| classifier_rules | YES | 8 exact rule assertions incl. priority order |
| entry_parsing | YES | CLASS=path split + invalid-class fallback |
| median_shape | YES | odd/even/empty exact values |
## 6. Doc claims: MEMAUDIT field order (:9-13) implemented verbatim at :120-151 — read match; "peak_B = max(open,walk)" (:14-18) — implemented :113. REAL.
## 7. Dead code: none.
## 8. Dependencies: crate::walk::WalkReport only.
## 9. Open questions: none.
