# CLAIM AUDIT: docs/adr/ADR-003-video-in-webview.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A04) |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 1 |

## 1. Blunt verdict
Decision record with full referential integrity: every evidence file, card,
risk row, and prototype doc it names exists, and the downstream state machine
it set up (2.D3 prototype -> PROVISIONAL-B -> R-04 mitigating -> ADR-004
admission gate) is coherent with its "B on Linux" landing. Only the upstream
issue-tracker citations (wry #1540, tauri #14924) are offline-unverifiable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Evidence: `docs/inventory/platform-synthesis.md` ... platform-{windows,macos,linux}.md" | :5-6 | REAL | all 4 files on disk |
| 2 | "doc 13 §5 cards 2.D3 / 2.E2-V" exist | :7 | REAL | doc 13 :348 card 2.D3 ("Video-in-webview prototype per doc 08 §7 (ADR-003 input): variant B first ..."), :355 card 2.E2-V gate |
| 3 | "Phase 2 prototype gates ... A platform whose prototype fails flips to its documented fallback" | :37-41 | REAL (executed) | docs/prototypes/video-in-webview.md:72 "**PROVISIONAL-B stands** (doc 08 §7's B-first + ADR-003's per-OS landing)" |
| 4 | "R-04 becomes bounded-and-planned" | :50 | REAL | 14-risk-register.md:95 R-04 row: "P2 gate passed 2026-10-21 ... PROVISIONAL-B ... re-scored 2026-10-22 §0.1" (status mitigating) |
| 5 | "SV-1 (Linux WebCodecs default-enabled) is ... assigned to the first Phase 2 wave" | :54-55 | REAL | platform-synthesis.md:102 "[VERIFY] (Linux default-enabled) [L §2.1 → SV-1]" |
| 6 | wry #1540 / tauri #14924 / wry #1731 upstream citations | :18,24-25 | UNVERIFIED | external tracker facts; no network in sandbox |
| 7 | "the active presentation path is a queryable engine fact" (truthful reporting) | :42-44 | REAL-UNPROVEN* | ADR-004:22-72 records the shell decision consistent w/ this clause; engine-side query not executed in this recovery |

## 3. Contradictions
None. The per-OS landing (A macOS / A-or-B Windows / B Linux) matches the
prototype trail's PROVISIONAL-B outcome exactly.

## 4. Fabrication / inflation indicators
None — the ADR encodes uncertainty honestly ([VERIFY SV-1], prototype gates,
fallback design). Findings: none.
