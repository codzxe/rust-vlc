# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__open-close-churn.md (35 lines — soak capture)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 REAL / 0 / 0 / 0 / 0 / 0 / 1 UNVERIFIED (measurement values) |
| Internal contradictions found | 1 (header vs summary scope) |

## 1. Blunt verdict
A raw soak-capture: 2,000,000 open/close cycles in 216.4 s with flat live-bytes
(295,117 -> 295,117; +0.0 B/cycle), fd count 4 -> 4 (budget 4), VmRSS 1720 -> 1972 kB
(budget 8 MB), verdict PASS. The honesty markers are strong: warmup exclusion disclosed,
"Every MB/h figure below is an EXTRAPOLATION from that window, not a 4 h measurement",
decoding 0 frames disclosed (open-close scenario never plays — the D-6 liveness axis),
and the ceiling-vs-target distinction (50 checked vs 5 reported-not-asserted with a
pointer to the freeze decision). One sloppiness: the header says the manifest has
"5 scenario(s)" and the verdict line says "every scenario stayed inside its ...
budgets" while this file's tables cover only the one scenario (open-close-churn) —
per-scenario capture files with a manifest-level verdict sentence.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "2,000,000 cycles ... wall 216.4 s ... 2,000,000 opens (0 error-lane) ... 2,000,000 closes" | scenario § | UNVERIFIED (measurement) | Raw capture; cannot re-run (cargo-build budget). Numbers internally consistent (4090 samples). |
| 2 | "Live bytes: 295117 -> 295117 ... Mean net live-bytes drift: +0.0 B/cycle" | scenario § | REAL (as data) | Flat-value pair arithmetic consistent (0 delta). |
| 3 | "fd count: baseline 4 -> final 4 (max 4); budget 4" | scenario § | REAL (as data) | The fd-budget assertion format matches the soak framework's design (soak-report-20261110.md). |
| 4 | "Fixtures ... read from their recorded expectation TOMLs, never transcribed" | method § | REAL (framework) | tests/fixtures/expectations/ exists (verified in doc-10 audit). |

## 3. Contradictions
| Header says | Summary shows | Truth |
|---|---|---|
| "Manifest ... 5 scenario(s)" + verdict "every scenario stayed inside budgets" | One scenario row (open-close-churn) | Per-scenario file; the manifest has 5, this file covers 1 — the verdict sentence is scoped to this scenario's budget axes. Sloppy phrasing, not a false measurement claim. |

## 4. Fabrication / inflation indicators
None — the capture discloses what it does NOT measure (no plays, no decode, 0 frames).
