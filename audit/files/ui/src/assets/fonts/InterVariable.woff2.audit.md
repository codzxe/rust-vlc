# AUDIT: ui/src/assets/fonts/InterVariable.woff2 (assignment chunks #part1-#part13, whole file)
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 352,240 bytes / n-a (binary) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (main.tsx:5 `import "./assets/fonts/fonts.css"` -> fonts.css:33 `src: url("./InterVariable.woff2")`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` (Web Open Font Format 2, TrueType, 352240 bytes); python3 fontTools parse (exit 0) |

## 1. One-paragraph verdict
Genuine Inter Variable webfont (WOFF2), not a stub or renamed blob. fontTools
parses it: family "Inter Variable", PostScript "InterVariable", version "Version
4.001;git-9221beed3", fvar axes opsz 14-32 and wght 100-900, 2937 glyphs.
Provenance is legitimate: SIL OFL 1.1 license (LICENSE.txt in same dir, name ID
0 "Copyright 2016 The Inter Project Authors", license URL scripts.sil.org/OFL).
The 13 assignment chunks are byte-slices of this one file; one audit covers all.

## 2. What actually works (verified)
— `file ui/src/assets/fonts/InterVariable.woff2` -> "Web Open Font Format
(Version 2), TrueType, length 352240, version 4.66".
— python3 fontTools: name IDs 1/2/4/6 = Inter Variable/Regular/Inter Variable/
InterVariable; fvar axes [('opsz',14,14,32),('wght',100,400,900)]; numGlyphs
2937; version 4.001;git-9221beed3.

## 3. Findings
F-001 [S4]
Where: ui/src/assets/fonts/fonts.css:19
Code: `* Inter v4.1 variable (opsz+wght 100..900, latin+latin-ext coverage),`
What is wrong: The comment says "Inter v4.1"; the font's own version string is
"Version 4.001" ("v4.1" reads as 4.1, the file is 4.001). Trivial provenance
doc imprecision, not a licensing or functional problem.
Confidence: HIGH — Verification: ran fontTools name ID 5.

## 4. Per-function table
Not applicable (binary asset).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "repo-bundled type — no host font dependency" | e2e/visual/MATRIX-20261113.md:110 | REAL | main.tsx:5 imports fonts.css which @font-face's this file with font-display: block |
| "SIL OFL 1.1, LICENSE.txt" alongside | MATRIX-20261113.md:110 | REAL | LICENSE.txt present in dir; name ID 14 = scripts.sil.org/OFL |

## 7. Dead / duplicated / unreachable code
None — the font is loaded first thing in main.tsx (line 5) and used by the
--font-family-sans token stack.

## 8. Dependencies & flags
None. Pure asset. ~344 KiB ships in the bundle (web-budget impact noted by
orchestrator baseline's red web-budget section; not re-measured here).

## 9. Open questions for the next agent
None.
