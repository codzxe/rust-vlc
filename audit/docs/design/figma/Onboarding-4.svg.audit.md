# CLAIM AUDIT: design/figma/Onboarding-4.svg (text asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | SVG, ASCII text with very long lines (`file`) |
| Size / Lines | 73,480 B / 241 lines |
| Dimensions | 1090×759 (`width="1090" height="759" viewBox="0 0 1090 759"`, :1) |
| sha256 | 1baab0423927c08ed49e32532800da73b4e21946ab09277857b9cd092cf93ef1 |
| Verdict | REAL (design source asset; step 4 of the 11-frame onboarding flow) |

## 1. Blunt verdict
Genuine Figma-export vector frame (`g id="Onboarding"`, rx14 #0E0E0E window on drop-shadow
filter), 131 element ids, pure static paint — no script, no foreignObject, no external href.
Distinct from Onboarding-7.svg (sha256 differs; byte sizes 73,480 vs 73,479) — a separate
wizard step, not a duplicate export.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the 11-frame onboarding step flow (`Onboarding-1..10.png` family) | whole | REAL | docs/waves/w26/CONTRACT.md:45 lists the Onboarding 11-frame flow; sibling SVG exists (Onboarding-7.svg, distinct sha256 verified). |
| 2 | Vector source for the wizard step the w26 loop audited | whole | REAL-UNPROVEN | Consumers w26.g02.md:16 / w26.h02.md:127 cite the .png renders of the same family (Onboarding-7.png); no doc cites this .svg by filename (rg "Onboarding-4" -> only the PNG-suffix hits) — source-vs-render mapping is by family convention, not a recorded citation. |

## 3. Contradictions
None. No doc makes a pixel claim about this specific .svg that the file contradicts.
