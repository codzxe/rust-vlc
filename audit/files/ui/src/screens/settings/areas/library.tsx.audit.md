# AUDIT: ui/src/screens/settings/areas/library.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 125 / 1 (LibraryArea) |
| File verdict | STUB (whole area staged-dead, honestly marked; zero live rows) |
| Reachable from shipped entry point? | YES — /settings/library via App.tsx:322 -> SettingsScreen -> areas registry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; header cross-check with staged.ts pattern (same-lane); grep registry |

## 1. One-paragraph verdict
The Library settings area is 100% staged: seven StagedRows (switches pinned on/off, one canned enum value) plus a staged Advanced bar, every one disabled with `data-staged="schema-seed"` and the honest tooltip, awaiting schemas/settings.toml keys that do not exist. The embedded PlaylistServicesArea renders nothing at all (the future-seam mount). One row even stages a switch ON ("Use media library", L98-105) — a toggle that displays an active state while controlling nothing. Honest, disclosed, and completely unwired: the master-question answer for /settings/library is zero backend.

## 2. What actually works (verified)
— verified by read: composition only; i18n labels resolve (staged.ts's useStagedMsg); structure matches the staged.tsx shared pattern used by the other staged areas (subtitles/effects audits).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/library.tsx:98-105 (representative; all rows L56-124)
Code:
```
        <StagedRow
          rowKey="library.use-media-library"
          label={t("settings.library.use-media-library")}
          control={{ kind: "switch", on: true }}
          seam={seam}
          reason={staged}
          info
        />
```
What is wrong: the entire area is inert UI — 7 disabled rows + staged Advanced bar; the "Use media library" row stages an ON state that suggests an active feature. Disclosure is present (data-staged markers, ⓘ reason "settings.library.staged"), so this is honest dead surface, not fraud. Census: /settings/library wires to NOTHING. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LibraryArea | 40-125 | STUB | S3 | 7 staged rows + staged advanced bar + empty PlaylistServicesArea fold |

## 5. Test honesty
library.test.tsx referenced in the header (registry embedding when generated screens land) — UNVERIFIED depth (not my file).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no schemas/settings.toml [[setting]] keys exist behind them yet" | header L6-8 | REAL (consistent) | rows are all StagedRow compositions, no SettingField imports |
| "Renders nothing today — the seam for its future schema rows" | L48-49 | REAL | PlaylistServicesArea mounted bare (L51) |

## 7. Dead / duplicated / unreachable code
The whole area is staged-dead by design (F-001); no other dead code.

## 8. Dependencies & flags
staged.ts shared primitives, playlist-services fold, staged.css. No feature flags.

## 9. Open questions for the next agent
- staged.ts (shared module, another agent's file): verify StagedRow disables honestly (assumed by pattern).
