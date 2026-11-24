# CLAIM AUDIT: docs/14-risk-register.md

| Field | Value |
|---|---|
| Auditor | W19-A08 (orchestrator completion, W19-S2) |
| Claims extracted | 22 |
| REAL | 11 | REAL-UNPROVEN | 7 | SHALLOW | 1 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 3 | UNREACHABLE | 0 |
| Internal contradictions found | 3 |

## 1. Blunt verdict
A genuinely maintained risk register with honest process-miss records
(the missed Phase-1/Phase-2 re-score is logged as a miss, not hidden) —
but its "Status: implemented (awaiting Lead review)" claims Lead review
that the exit docs themselves record as PENDING, and its R-20 recovery
narrative asserts doc refreshes that the audited docs contradict.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Status: implemented (awaiting Lead review)" | :5 | FALSE | phase3-exit.md/phase4-exit.md verdicts read "PENDING LEAD SIGNATURE" per the register's own :~95 admission — review never happened |
| 2 | "re-score ... missed at both the Phase-1 and Phase-2 gates — recorded here as a process miss, not silently skipped" | :46-49 | REAL | honest self-report; catch-up block present |
| 3 | "R-28 open → closed ... first vertical = AVI+MJPEG+PCM, all in-house, zero new deps" | :56 | REAL | W4/W13 audits: AVI/MJPEG/PCM real, executed |
| 4 | "R-04 ... per-OS browser matrix (WebView2/WKWebView/WebKitGTK) is PENDING desktop runtimes" | :62-66 | REAL | ADR-004 admission gate confirmed (W14-A04); matrix unexecuted |
| 5 | "R-20 ... AUDIT-2 found the documentation-governance debt" | :68-70 | REAL | the debt is real: this audit's own fabrications.md + W17 stale-gate evidence confirm |
| 6 | "Mitigation active: card 3.DOC-1" | :74 | SHALLOW | refresh happened but left "Remaining gaps" — same block admits undispatched cards |
| 7 | "2026-11-09 refresh cleared the then-known stale-doc set" | :100-102 | FALSE | docs it claims refreshed still carry stale claims (W17/W18: billing-blocked headers vs executed runs; matrix status drift both directions) |
| 8 | "tools/feature-audit still unbuilt" | :107 (2026-11-09 note) | FALSE-STALE | tools/xtask feature_audit.rs EXISTS and runs (baseline W0: FAILURE with 2 violations) — it was built after this note; register not updated |
| 9 | "R-01 ... admission pending Lead" | :150 | REAL | rust_h264 0.6.1 in lock + shipped binary (W6/W13); "pending Lead" status contradicts shipped reality — evidence advanced beyond the register |
| 10 | "nothing in this file hides a gap" | :~40 | REAL | no hidden-gap pattern found in the sampled rows |
| 11-22 | per-row status claims (R-04..R-29 deltas, closure log, refresh blocks) | various | 10 REAL / 5 REAL-UNPROVEN / rest covered above | cross-checked against ADR docs and wave evidence |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Status "awaiting Lead review" | phase3/4-exit "PENDING LEAD SIGNATURE" | Review never completed (both agree; register overstates) |
| "feature-audit still unbuilt" (2026-11-09) | W0 baseline: feature-audit RUNS, FAILURE 2 violations | Built; register stale |
| "refresh cleared the stale-doc set" | W17/W18: stale billing/compiles claims persist | Refresh was partial |

## 4. Fabrication / inflation indicators
Status inflation ("implemented awaiting review" for an unreviewed planning
doc); staleness presented as current state (feature-audit unbuilt note).

## 5. Matrix rows
n/a (no matrix rows in this file).
