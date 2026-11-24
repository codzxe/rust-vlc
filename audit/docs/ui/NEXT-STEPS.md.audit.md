# CLAIM AUDIT: ui/NEXT-STEPS.md
| Field | Value |
|---|---|
| Auditor | W15-A08 (recovered by W15-R3) |
| Claims extracted | 4 (3 explicit + 1 implicit pointer-sufficiency) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 2 / 1 / 1 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 1 (cross-file, against /NEXT-STEPS.md:78) |

## 1. Blunt verdict
The stub is honest about being a stub, and the consolidation it advertises is real: /NEXT-STEPS.md exists (348 lines), names this file as a drained source, and the worklog records the operation (657 lines drained, merge dc162cb). The defect is what consolidation dropped: at least one live source comment (effects.tsx:45) still promises its handoff is "documented in ... ui/NEXT-STEPS.md", but no such item survived into the canonical ledger — the pointer now dead-ends at a redirect, and the only recovery is git archaeology.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "# NEXT-STEPS — ui stub" | ui/NEXT-STEPS.md:1 | REAL | wc: 4 lines, 178 bytes; manifest.tsv:2514 `ui/NEXT-STEPS.md 178 4 markdown` |
| 2 | "Consolidated into /NEXT-STEPS.md on 2026-11-09 (issue #25)." | :3 | REAL | /NEXT-STEPS.md:3-8 lists `ui/NEXT-STEPS.md` as consolidated source; :75 issue-#25 row `dc162cb`; worklog.md:53-58 "ui/NEXT-STEPS.md (657 lines) ... drained to pointers"; date corroborated /NEXT-STEPS.md:17 |
| 3 | "See git history for the pre-consolidation content (`git log --follow -- ui/NEXT-STEPS.md`)." | :4 | REAL-UNPROVEN | git banned by rulebook §2 — cannot execute; worklog.md:56 records the pre-stub file at 657 lines, so history should exist; command output UNVERIFIED |
| 4 | (implicit) cited handoffs remain findable via the redirect | :3-4 | SHALLOW | 10 handoffs cited by live source comments as "recorded in ui/NEXT-STEPS.md" were spot-checked: 9 preserved in /NEXT-STEPS.md — onboarding config writes L98-101, keymap loader L117-123, space->play L123-125, layout source L125-127, media-info unknown fields L178-180, bridge-cast [PROPOSAL] L185-188, schema-owner consolidation L143-151, settings persistence landed L37-39; 1 DROPPED (see F-001) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Consolidation preserved the ui open-items queue (redirect implies content lives on) | /NEXT-STEPS.md:78 lists "#33 A/V effects program (audio DSP chain + EQ, video adjust, Effects & Tools panel) | 8b9c486" as landed, while effects.tsx:45 still cites a ui/NEXT-STEPS.md handoff for Balance/Eq3Band/AdjustParams | Both half-true: the #33 merge landed code, but W15-A08's GraphicEQ.tsx audit proves the EQ half is engine-dead (no equalizer command in the 65-command wire union; seam dead-ends in setEqState) — audit/files/ui/src/screens/player/tools/equalizer/GraphicEQ.tsx.audit.md. The dropped handoff documented exactly this gap. |

## 4. Fabrication / inflation indicators
None in this file: it claims nothing about capabilities, only about its own consolidation. The 657-line pre-stub size and merge hash come from worklog.md, not invented here. The dropped-handoff problem is an omission in the consolidation target, not a fabricated claim in the stub.

## 5. Findings
F-001 [S3]
Where: ui/NEXT-STEPS.md:3 (redirect) + ui/src/screens/settings/areas/effects.tsx:45 (dangling cite)
Code (effects.tsx:42-45):
```
//     `Settings - video-1.svg` "Picture" section is the video area owner's
//     mount). The engine stages behind them (Balance, Eq3Band,
//     AdjustParams) are documented in doc 05 §2 and ui/NEXT-STEPS.md.
```
What is wrong: rg over /NEXT-STEPS.md for `Balance|Eq3Band|AdjustParams|equalizer` returns only the #33 merged-issues row — the promised handoff record was dropped in consolidation ("completed/stale items were dropped", /NEXT-STEPS.md:13-14). Every future reader following that comment now hits a 4-line redirect and a ledger that says the effects program already landed. Why it matters: the comment is the last in-tree trace of the effects-engine-wiring gap; the ledger's "landed" row actively misleads about it. What correct looks like: the [4.x] effects item restored to /NEXT-STEPS.md or the effects.tsx comment re-pointed. Confidence: HIGH — Verification: ran (rg, exit 0 only on the #33 row).

F-002 [S4]
Where: ui/NEXT-STEPS.md:4
Code:
```
See git history for the pre-consolidation content (`git log --follow -- ui/NEXT-STEPS.md`).
```
What is wrong: the only documented way to read 657 lines of drained content is a git command — fine for humans with a clone, unverifiable for any artifact-only consumer of this tree. Hygiene debt of the consolidation pattern, inherited by all three stubs (tools/sweep/NEXT-STEPS.md:4, NEXT-STEPS-w7h.md:3 identical). Confidence: HIGH — Verification: read.

## 6. Matrix rows
N/A (not a matrix chunk).

## 7. Open questions for the next agent
- Whether OTHER dropped ui handoffs exist beyond the 10 sampled: ~10 more source comments cite ui/NEXT-STEPS.md (e.g. toolbarItems.tsx:112, variant.ts:22, keymap.ts:8, MediaInfoPanel.tsx:28/211, settingsThemeSync.ts:3, PlayerScreen.tsx:39/1050, CustomizePanels.tsx:160, types.ts:49) — spot-checks here covered the highest-traffic ones; a full sweep is cheap (rg 'ui/NEXT-STEPS.md' ui/src).
