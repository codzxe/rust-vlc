# AUDIT: ui/src/assets/fonts/LICENSE.txt (quick check — license file)
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 93 / 1 document (OFL 1.1: copyright line + preamble + definitions + conditions 1-5 + termination + disclaimer) |
| File verdict | REAL (verbatim SIL OFL 1.1 for Inter; copy requirement satisfied) |
| Reachable from shipped entry point? | YES — it ships beside InterVariable.woff2 (352,240 bytes, verified magic `wOF2`), which is the bundled face loaded first by main.tsx:5 → assets/fonts/fonts.css:29-35 (@font-face Inter, weight 100 900, font-display block), consumed by the token stack `--font-family-sans: "Inter"` (tokens.css). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read all 93 lines against the canonical OFL 1.1 structure (preamble/definitions/5 conditions/termination/disclaimer — all present, wording matches the 26 February 2007 text); ls ui/src/assets/fonts (LICENSE.txt + InterVariable.woff2 + fonts.css, no orphan files); head -c 4 on the woff2 → `wOF2` magic (real font, not a placeholder); Read fonts.css:1-35 (the bundling rationale + @font-face); rg font references (main.tsx:5, tokens.css --font-family-sans). Binary probe for version/coverage strings → nothing readable (Brotli-compressed tables) — UNVERIFIED (below). |

## 1. One-paragraph verdict
The license file is the standard SIL Open Font License 1.1 with the correct Inter copyright line ("Copyright (c) 2016 The Inter Project Authors", upstream URL), no Reserved Font Name declaration — matching upstream Inter, which does not reserve — and the full canonical text (all five permission conditions, termination, disclaimer). OFL condition 2 (the copy requirement: "each copy contains the above copyright notice and this license") is satisfied by shipping this file in the same directory as the woff2. OFL is permissive for bundling with the repo's MIT code (fonts stay OFL, app stays MIT — the license explicitly does not extend to documents/software using the font). Nothing is missing, truncated, or altered.

## 2. What actually works (verified)
— verified by reading: complete OFL 1.1 text, correct copyright attribution, no RFN (matches upstream Inter's own header).
— verified by ls + magic bytes: the licensed artifact exists (InterVariable.woff2, `wOF2` signature) and the license ships beside it (condition 2 satisfied).
— verified by rg: the font is genuinely consumed (main.tsx:5 imports fonts.css before anything else; tokens.css lists "Inter" first in --font-family-sans) — this is not an orphan license for a purged asset.

## 3. Findings
None. Checked: canonical text completeness, attribution, RFN status, copy-requirement satisfaction, artifact presence + signature, consumption by shipped code, MIT-repo compatibility.

## 4. Per-function table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Copyright + license pointer | 1-5 | REAL | — | Inter Project Authors, upstream URL, SIL pointer. |
| Preamble / Definitions | 11-44 | REAL | — | Canonical. |
| Permission and Conditions 1-5 | 46-77 | REAL | — | Canonical, incl. the bundling clause (2) and the name-use clauses (3-4). |
| Termination / Disclaimer | 79-93 | REAL | — | Canonical. |

## 5. Test honesty
Not applicable (license text). The visual-parity lanes (fonts.css header: bundling fixed 13/50 failing text-heavy screens, CI run 43378741271) are the executable evidence the font matters.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "SIL Open Font License 1.1 — LICENSE.txt alongside" | fonts.css:19-20 | REAL | This file, read. |
| "The OFL is OSI-approved and GPL-compatible; the copy requirement is satisfied … LICENSE.txt ships in the same directory" | fonts.css:20-22 | REAL | Read; condition 2 met. |
| "Inter v4.1 variable (opsz+wght 100..900, latin+latin-ext coverage)" | fonts.css:18-19 | UNVERIFIED | woff2 tables are Brotli-compressed — no readable version/axis strings; no font tooling in sandbox (no network/pip). The @font-face weight range 100 900 matches the wght claim; opsz axis and glyph coverage could not be checked. |

## 7. Dead / duplicated / unreachable code
None — the directory holds exactly the font, its license, and its loader CSS; all three consumed.

## 8. Dependencies & flags
None (a text file). Context: repo LICENSE is MIT; OFL applies to the font files only and does not contaminate the surrounding code license.

## 9. Open questions for the next agent
- If anyone regenerates or updates the font, re-verify the version/coverage claim (v4.1, opsz+wght, latin+latin-ext) with fonttools — my probe was blocked by compression (UNVERIFIED, §6).
