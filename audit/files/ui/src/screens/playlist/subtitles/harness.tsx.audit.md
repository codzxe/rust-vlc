# AUDIT: ui/src/screens/playlist/subtitles/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 152 / 5 harness mounts + 4 fixture arrays |
| File verdict | REAL (dev-only vision harness — never on app routes) |
| Reachable from shipped entry point? | NO by design — only consumer is harness.test.tsx (rg); header states "never imported by app routes" and the scan confirms it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers (harness.test.tsx only); rg AddSubtitlesDialog onSave semantics |

## 1. One-paragraph verdict
A screenshot harness for the subtitle-flow design comparison loop. It mounts
the REAL production components (AddSubtitlesDialog, FilenameMatchControl,
SubtitlePickList, SubtitlePickRow) with fixture data — it does not re-implement
or fake them, so the vision loop validates shipped markup. The noop
`onSave`/`onClose` props are honest seams; notably the harness passes onSave
deliberately so the dialog's Save button renders its wired state rather than
honest-disabled (comment 50-53). No leakage into app routes.

## 2. What actually works (verified)
- Consumer scan: only harness.test.tsx imports this module — "never imported by app routes" claim holds.
- The mounted components are the production ones (imports 14-18), with the dialog-scoped `lumen-plsubs` class replicating shipped CSS scoping (comment 98-108) — the harness explicitly fixed the earlier flaw of validating a control "that doesn't ship".
- i18n-literal eslint disables are scoped and justified per-line (113, 118, 134).

## 3. Findings
None. Checked: production leakage (none), fixture/real conflation (fixtures are
obviously synthetic filenames), dead code (all five mounts referenced by the
test), copy discipline (eslint disable comments).

## 4. Per-function table (file >100 LOC)
| Export | Lines | Verdict | Notes |
|---|---|---|---|
| HARNESS_* fixture arrays | 22-48 | REAL | synthetic episode/subtitle names |
| FolderPairsHarness | 54-65 | REAL | matched-pairs review state |
| FolderMultiHarness | 69-81 | REAL | multi-subtitle pick |
| FolderOnlineHarness | 84-96 | REAL | online pick |
| SubFilenameHarness | 109-126 | REAL | dropdown closeup, scope class applied |
| SubtitleRowsHarness | 130-152 | REAL | row fragments at 519px |

## 5. Test honesty
harness.test.tsx renders these mounts to static HTML for screenshots — the
harness itself is test infrastructure; its "test" is the visual comparison
loop run by the lane (recorded in lane log per header). UNVERIFIED: the
screenshot comparison outputs (not in sandbox scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | harness.tsx:1-2 | REAL | rg: sole importer is harness.test.tsx |
| "the mounts pass `onSave` ... Save gates on the commit seam and would render honest-disabled without it" | harness.tsx:50-53 | REAL | AddSubtitlesDialog.tsx:29-34 documents the no-fake-persistence contract |

## 7. Dead / duplicated / unreachable code
None within its dev-only purpose.

## 8. Dependencies & flags
Production subtitle components + harness.css. No gates.

## 9. Open questions for the next agent
- None (the AddSubtitlesDialog write path is that component's audit owner's
  question, not the harness's).
