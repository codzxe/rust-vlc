# AUDIT: ui/src/assets/fonts/LICENSE.txt
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 92 / n/a (SIL OFL 1.1 license text for the Inter variable font) |
| File verdict | REAL (correct, complete OFL 1.1 text accompanying the shipped font) |
| Reachable from shipped entry point? | YES as a shipped asset — fonts.css:20-22 cites it ("LICENSE.txt ships in the same directory"); the font it licenses (InterVariable.woff2, 352,240 bytes, woff2) is bundled via fonts.css:33, imported first in main.tsx:5. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file (ASCII text); head (copyright + OFL header verified); file on InterVariable.woff2 (valid woff2); rg citations (fonts.css:20-33; e2e/visual/MATRIX-20261113.md names it as the license for the bundled font) |

## 1. One-paragraph verdict
The standard SIL Open Font License 1.1 text for Inter (copyright 2016 The Inter Project Authors), unmodified preamble through the standard terms. The companion binary is a valid woff2 and is bundled in the shipped app; OFL 1.1 requires the license accompany redistribution, and it does (same directory, cited by fonts.css and the visual matrix doc). Zero findings — binary/asset short audit per spec §12.

## 2. What actually works (verified)
— `file` → ASCII text; header lines match the canonical OFL 1.1 text; woff2 companion is a real font (file: "Web Open Font Format (Version 2), TrueType, length 352240").

## 3. Findings
None. Checked: text is the real OFL 1.1 (not a placeholder), license/binary co-location, citation in fonts.css and the visual matrix, no license mismatch with the shipped face ("Inter Variable").

## 4. Per-function table
n/a (license text; <100 LOC short-audit rules).

## 5. Test honesty
n/a. (The woff2 itself was audited separately: audit/files/ui/src/assets/fonts/InterVariable.woff2.audit.md.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SIL Open Font License 1.1 — LICENSE.txt alongside" | fonts.css:20-22 | TRUE | file present, canonical text |

## 7. Dead / duplicated / unreachable code
n/a.

## 8. Dependencies & flags
n/a.

## 9. Open questions for the next agent
- None.
