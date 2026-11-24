# AUDIT: ui/src/screens/playlist/subtitles/useSubtitlesMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 27 / 1 (useSubtitlesMsg) |
| File verdict | REAL — seam resolver whose fallback arm is dead post-registration |
| Reachable from shipped entry point? | YES — 3 production consumers (SubtitlePickList, FilenameMatchControl, AddSubtitlesDialog, subtitles settings area) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n` b11 keys in messages.ts (registered: empty-title L2229, list-label L2233); `rg -ln useSubtitlesMsg` (4 consumers) |

## 1. One-paragraph verdict
The w26.b11 seam resolver: `useMsg` widened with the subtitle-flow lane keys, catalog-first with lane fallback. The Lead registration has landed (spot-checked keys present in messages.ts), so — as with w26.b09/i18/k16 — the `?? laneCatalog[key]` arm is unreachable for registered keys (useMsg never returns undefined for them). The hook itself is live and consumed by 4 real surfaces. S4: dead fallback arm awaiting the documented cleanup pass.

## 2. What actually works (verified)
— verified by consumer sweep (4 files) + registration check (messages.ts:2229/2233).

## 3. Findings
F-001 [S4] — dead fallback arm post-registration (mechanism per w26.b09/ControlBar F-004).
## 4-9. (compact)
No drift (registered values match the lane reference). Consumers real. No flags. No open questions.
