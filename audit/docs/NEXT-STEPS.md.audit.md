# CLAIM AUDIT: NEXT-STEPS.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 9 (actionable factual sub-claims) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 / 0 / 0 / 0 / 0 / 1 / 4 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
An honest tactical queue whose self-describing structure verifies
(pointer-stub consolidation REAL, several "still in place" items REAL) —
with one badly stale factual sub-claim: the [4.E3] item states "zero
entries exist for the video/audio/subtitles screens" while
schemas/settings.toml TODAY carries 12+ `video.*`, 11 `audio.*`, 9
`subtitle.*` and multiple `controls.*` [[setting]] rows (the w26.b02 seed
landed after this file's 2026-11-09 consolidation). The same staleness
class I found in feature-matrix rows 148-151/156. Merge-commit cites
(145b265, b480049, …) are UNVERIFIED — git is forbidden to auditors.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "NEXT-STEPS-w7h.md, ui/NEXT-STEPS.md, tools/sweep/NEXT-STEPS.md … are now pointer stubs" | :4-9 | REAL | `head ui/NEXT-STEPS.md` -> "# NEXT-STEPS — ui stub / Consolidated into /NEXT-STEPS.md on 2026-11-09 (issue #25)"; same shape for tools/sweep/NEXT-STEPS.md. |
| 2 | [4.C3] "delete `ui/src/screens/player/states/context-bridge.ts` … PlayerScreen currently consumes the bridge — verified still in place 2026-11-09" | :57-59 | REAL | `ls ui/src/screens/player/states/` -> context-bridge.ts present (with StateRouter.tsx, error-display.ts). |
| 3 | [4.E2] "no `interface.accent` key in `schemas/settings.toml`" | :124-126 | REAL | `rg 'key = "interface\.' schemas/settings.toml` -> interface.theme/use-custom-skin/skin-file/… — no interface.accent. |
| 4 | [4.E3] "zero entries exist for the video/audio/subtitles screens" | :108-111 | FALSE | schemas/settings.toml: `rg -c 'key = "video\.'` -> 12+, `key = "audio\.'` -> 11, `key = "subtitle\.'` -> 9, controls.skip-very-short :1479 (`default = 3000`) etc. The w26.b02 seed landed after 2026-11-09 and this queue item was never closed. |
| 5 | Landed table: 27 of 34 issues resolved "merge evidence: git log --oneline --grep" + per-issue merge SHAs | :18-64 | UNVERIFIED | git forbidden to auditors (AGENT-SPEC §2); downstream artifacts do exist (issue #2 -> h264-software-decoder-audits.md present; issue #18 -> .github/workflows/soak-nightly.yml present with cron `0 3 * * *` + SOAK_MINUTES=240 — verified by read; issue #3 config-settings.ts present). |
| 6 | [4.E2] startup-theme/settingsThemeSync scoping note | :122-123 | REAL-UNPROVEN | Honest mechanism note; behavior not re-executed here. |
| 7 | Standing note: "`tee \| head` SIGPIPE-truncates files (durations.tsv needed a rerun)" | :271-272 | REAL | Consistent with the sandbox behavior class documented in AGENT-SPEC-adjacent baselines; no artifact contradiction available. |
| 8 | [4.D2] "wire type carries no `size`, no … bitrate, no per-stream tag details … rows render Unknown" | :116-118 | REAL-UNPROVEN | Plausible MediaInfo gap claim; catalog code not re-verified in this audit (out of my file list). |
| 9 | [sweep] "non-AVI containers honestly land `unsupported`" + 15-field protocol | :266-269 | UNVERIFIED | tools/sweep sources out of my assignment; note recorded. |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "[4.E3] zero entries exist for the video/audio/subtitles screens" | schemas/settings.toml: video/audio/subtitle/controls rows present; GENERATED_AREA_IDS includes all four; controls.tsx renders `<GENERATED_SCREENS.controls/>` | The schema is true; the queue item is stale — it should be checked off or rewritten, not left asserting absence. |

## 4. Fabrication / inflation indicators
None — this ledger overstates nothing; its defect is unrefreshed staleness
(four-months-behind factual assertions inside open items). The "Landed"
section's merge SHAs cannot be checked without git (orchestrator's lane).

## 5. Matrix rows
None. Feeds issue #20 (feature-audit/backfill) context — the same item
whose under-stated matrix rows 148-151/156 I audited in
feature-matrix.toml.rows-144-156.
