# AUDIT: ui/src/assets/fonts/fonts.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 34 / 0 (single @font-face + header) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by main.tsx (`rg -ln "fonts.css"` -> ui/src/main.tsx), the app entry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `ls -la ui/src/assets/fonts/` (3 files); `file InterVariable.woff2` -> "Web Open Font Format (Version 2), TrueType, length 352240, version 4.66"; importer scan |

## 1. One-paragraph verdict
A single correct @font-face bundling Inter v4.1 variable (opsz+wght 100..900)
as a real 352KB WOFF2 asset that exists on disk and is imported at the app
entry. The header documents a real incident (k-wave font purge broke visual
baseline parity, CI run 43378741271, 13/50 failed) and the license story
(OFL 1.1, LICENSE.txt ships alongside — verified present). `font-display:
block` is deliberate and justified (baseline pinning; no fallback-face first
paint).

## 2. What actually works (verified)
— `file` on the woff2: genuine WOFF2, 352240 bytes (matches the CSS src).
— LICENSE.txt present in the same directory (OFL copy requirement satisfied
per header). — Import chain: main.tsx -> fonts.css -> woff2 url.

## 3. Findings
None. Checked and how: asset existence (file command); license compliance
(LICENSE.txt listed); format correctness (woff2-variations with weight range
100 900 matches the variable font claim); reachability (entry import).

## 4. Per-function table
N/A (binary-asset-adjacent stylesheet; short audit per spec §12 binary-asset
rule).

## 5. Test honesty
N/A (font asset). Visual-baseline suites pin the rendered face (per the
header's incident note — those suites are other agents' scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Bundling restores the documented parity guarantee ... 'repo-bundled type — no host font dependency'" | fonts.css:13-17 | REAL (asset bundled + imported at entry) | file + importer verification; MATRIX-20261109.md claim itself UNVERIFIED (docs domain) |
| "SIL Open Font License 1.1 — LICENSE.txt alongside" | fonts.css:19-22 | REAL | ls shows LICENSE.txt (4380 bytes) in the directory |
| "CI run 43378741271, visual PR-lite lane: 13/50 failed" | fonts.css:8-11 | UNVERIFIED | historical CI log not accessible; incident narrative plausible and consistent |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (asset + one rule). No feature gates.

## 9. Open questions for the next agent
- Confirm the token stack still lists "Inter" first
  (ui/src/tokens/generated/tokens.css --font-family-sans) — the bundling fix
  presumes it; if the stack changed, this @font-face could be dead weight.
