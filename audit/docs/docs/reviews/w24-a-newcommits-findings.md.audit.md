# CLAIM AUDIT: docs/reviews/w24-a-newcommits-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 2 / 1 / 0 / 0 / 0 / 1 / 2 |
| Internal contradictions found | 1 (R11's own sha quote vs the record it cites) |

## 1. Blunt verdict
An audit-of-an-audit whose headline (the audited report's "Chromium
deleted / NOT re-runnable" note was false) is coherent and was live-tested
by R11 — but R11's own evidence contains one hash citation that does not
match the committed record it cites: it writes the wasm artifact sha256 as
`aa70ec59…4a19e2` "the record's `artifact.sha256` exactly", while the
committed record's value is `4a0f33831d…4d87`. An auditor whose own
citation fails byte-verification is a defect worth recording (S3). The
re-runnability ingredients have since changed again (today: chromium
ABSENT; /tmp wasm artifact rebuilt, 1,302,846 B, different hash) —
environment drift, not a new claim.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Record fields: `chromiumExecutable` path, artifact 1294311 bytes, Playwright 1.64.1 | :36-44 | REAL | docs/web/browser-matrix-20261110.json:18 path matches; :27 `"bytes": 1294311`; md:174 `Playwright 1.64.1; Chromium 157.0.8103.50`. |
| 2 | "the wasm artifact … sha256 `aa70ec59…4a19e2` = the record's `artifact.sha256` exactly" | :42 | FALSE (citation mismatch) | The record's artifact block: `"sha256": "4a0f33831d6411dfef20b13925bb82a2f658d79c843dd1ec1ebfab78ca478d87"` — `aa70ec59` appears nowhere in the committed record (`rg 'aa70ec59' docs/` -> only this review file). |
| 3 | Headline P1: the audited report's "Chromium was deleted … NOT re-runnable" note was false (binary existed and ran, exit 0) | :13-15 | UNVERIFIED today | The binary is absent in today's sandbox (`ls` -> CHROME-ABSENT); R11's live run happened 2026-11-10 and its log is this file — internally consistent, not re-provable. |
| 4 | "Re-runnability ingredients … wasm artifact … 1294311 bytes" | :41-43 | REAL-UNPROVEN | A wasm artifact exists today at the recorded /tmp path but is 1,302,846 B with a different sha (`4ed2d7dd…`) — rebuilt since; the record's 1,294,311 B figure matches the record itself (:27). |
| 5 | "Deterministic re-render … the committed .md IS the renderer's output over the committed JSON" | :23-26 | REAL-UNPROVEN | The .md/.json pair exists and is internally consistent (spot-checked fields above); the byte-identical re-render itself was R11's session run. |
| 6 | "R8's P2-2 is CLOSED, P2-1 HALF-closed, 4/5 P3s fixed" + 3-commit delta list | :10-13 | UNVERIFIED | Git-lineage claims (SHAs) — git forbidden to this auditor. |

## 3. Contradictions
| This doc says | Committed record says | Truth |
|---|---|---|
| artifact sha256 `aa70ec59…4a19e2` "exactly" matches the record (:42) | browser-matrix-20261110.json artifact.sha256 = `4a0f3383…4d87` | The record is the authority; R11's hash citation is wrong or stale — either way false as committed. |

## 4. Fabrication / inflation indicators
One: the hash-citation above. The rest of the file deflates hard (it
caught a materially false claim in another report and re-ran the evidence
to prove it). The irony is that its strongest evidence line contains the
one unverifiable number.

## 5. Matrix rows
None. Touches docs/web/web-column-pass-20261110.md's evidence base (the
browser-matrix record).
