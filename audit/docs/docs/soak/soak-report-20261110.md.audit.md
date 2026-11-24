# CLAIM AUDIT: docs/soak/soak-report-20261110.md

| Field | Value |
|---|---|
| Auditor | W19-A09 (orchestrator completion, W19-S2) |
| Claims extracted | 14 |
| REAL | 9 | REAL-UNPROVEN | 4 | SHALLOW | 0 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 1 | UNREACHABLE | 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
The strongest single evidence document in the repo: a real in-sandbox
bounded soak with committed verbatim per-scenario reports, an honest
aarch64/x86_64 correction of its own briefing, and an explicit §8 list
of what did NOT run. Its one false claim is the "nightly 4h tier ...
wired and billing-blocked" line — the W17 evidence chain (worklog run
IDs) shows the billing story flipped on 2026-11-12.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the five-scenario bounded soak RAN for real, in-sandbox, all green" | :3-4 | REAL | raw reports committed (docs/soak/raw/scenarios-sandbox__*.md); W16 re-ran soak suite live 39s PASS |
| 2 | "the 5 MB/h design target is FROZEN on the measured numbers" | :4-5 | REAL-UNPROVEN | numbers committed; freeze decision Lead-only |
| 3 | "the nightly 4 h tier and the LSAN job are wired and billing-blocked" | :5-6 | FALSE | W17: billing unblocked 2026-11-12, run IDs 43378757248+ exist; header stale |
| 4 | "Every number below is from a real run whose verbatim report is committed" | :6-7 | REAL | raw files verified on disk by W16 |
| 5 | "Honest label: the wave briefing called this sandbox 'aarch64' — the machine is x86_64" | :14-16 | REAL | exemplary self-correction |
| 6 | "the MEASURED walls: 3 × 480.0 s + 216.4 s + 200.1 s = 1,856.5 s" | :18-20 | REAL | arithmetic: 3*480+216.4+200.1 = 1856.5 exact |
| 7 | "what did NOT run is labeled in §8 (blocked)" | :7-8 | REAL | honest-negative disclosure pattern |
| 8 | "Every op is a catalog WireRequest through the soak dispatch seam onto the real engine stack" | :~30 | REAL | W16-A07 executed soak_scenarios live (39s PASS) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "billing-blocked" (2026-11-10) | RELEASE.md: billing unblocked 2026-11-12; run IDs exist | Doc was true when written; stale at HEAD (also flagged by W14-A04, W17-A04) |

## 4. Fabrication / inflation indicators
None — the opposite: self-corrects its own briefing (aarch64), labels
unmeasured lanes, and commits raw evidence. Best-practice document the
rest of the corpus should imitate.
