# CLAIM AUDIT: design/solstice-v2-brief.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07; R3's abandoned tail file #43) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 3 / REAL-UNPROVEN 1 / FALSE 1 / PAPER-ONLY 2 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The 2026-11-09 "Solstice v2" redesign brief, still headed "Status: ACTIVE (Lead-approved)".
FALSE today: the entire Solstice system it specifies is RETIRED — solstice.css is gone
(lumen.css:19 "Replaces solstice.css (sunset stage, taped notes, keycaps, Nunito...)"),
no data-backdrop tier exists anywhere in ui/src, and three later waves grep-verified "zero
Solstice remnants". The brief is a dead design doc with a never-updated ACTIVE header —
exactly the class of stale-status debt the audit exists to catch. Its surviving real
content: the `.lumen-button--primary` button language and the `lumen:open-media` flow
seam, both still live code.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: ACTIVE (Lead-approved)" | :3 | FALSE | ui/src/theme/lumen.css:19 "Replaces solstice.css..."; ls ui/src/theme/ -> no solstice.css; j13/j10 (this batch) grep 0 sunset/tape/keycap/Nunito hits |
| 2 | "html[data-backdrop='vivid']... html[data-backdrop='calm'] — new in solstice.css v2" | :32,37 | PAPER-ONLY (retired) | rg data-backdrop/dataset.backdrop in ui/src -> 0 hits; solstice.css absent |
| 3 | "primary = accent pill (.lumen-button--primary)" | :72 | REAL | ui/src/components/button/button.css:92 `.lumen-button--primary` exists (onboarding.css:632 consumes it) |
| 4 | "Empty state CTA → opens the same dialog (already wired via lumen:open-media)" | :102-103 | REAL | App.tsx:199 + PlayerScreen.tsx:43,69 — `lumen:open-media` CustomEvent seam alive today |
| 5 | "Contrast floor: body text ≥ 4.5:1... (WCAG AA)" | :62 | REAL-UNPROVEN | rule text; the WCAG arbitration in tokens.json:718 shows the same law enforced later; per-screen values not re-measured |
| 6 | Ownership map dirs (onboarding/not-found/playlist/settings/...) | :120-122 | REAL | ls ui/src/screens/ -> media-info, network-stream, not-found, shortcuts all exist |
| 7 | "Supersedes the 2026-11-08 first pass wherever they conflict" | :3-4 | UNVERIFIED | first-pass doc not present under design/ (ls); history claim |
| 8 | "10 screens found... 60-90% dead space" (the v2 motivation) | :10-24 | UNVERIFIED | no committed audit artifact of the 2026-11-08 pass; worklog archive cites the brief (docs/worklog/archive-2026-10.md:10910) |
| 9 | "A screen sets its tier in a mount effect (root.dataset.backdrop...)" | :47-48 | PAPER-ONLY (retired) | no onboarding .tsx sets dataset.backdrop today (rg 0 hits) |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "Status: ACTIVE" design system | lumen.css:19 replaces solstice.css; w26 waves audit "zero Solstice remnants"; NotFoundScreen.test.tsx "retires the Solstice stage" | The retirement is the truth; the brief's status header is a stale S3 lie-by-aging |

## 4. Fabrication / inflation indicators
Not fraud at authoring time — but the doc is the origin of an entire aesthetic that was
later fully removed, and its ACTIVE header was never struck. Cross-references: W01-A04's
audit flagged this file for identity literals (:72, :103) — both confirmed present.

## 5. Notes for next agents
- This file was W19-R3's #43 (R3 died at 20/50 logged, audits present through its #40;
  files 41-50 were abandoned — this recovery covers them).
- Anyone tracing "why does the UI have no Solstice tokens" should cite lumen.css:19 and
  this audit, not the stale brief.
