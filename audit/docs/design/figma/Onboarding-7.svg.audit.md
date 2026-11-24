# CLAIM AUDIT: design/figma/Onboarding-7.svg (text asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Type | SVG, ASCII text with very long lines (`file`) |
| Size / Lines | 73,479 B / 241 lines |
| Dimensions | 1090×759 (`width="1090" height="759" viewBox="0 0 1090 759"`, :1) |
| sha256 | b648bce8b81ec9fc19b0c696a6ddc70c16ce1fa70319aa90a1799055cc294db5 |
| Verdict | REAL (design source asset; the "appearance, blue" wizard step) |

## 1. Blunt verdict
Genuine Figma-export vector frame of the appearance/blue wizard step — the exact frame
w26.g02 and w26.h02 grade the implementation against. Static paint only; no script/href
payload. Distinct from Onboarding-4.svg (sha256/byte-size differ).

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Onboarding-7 (appearance, blue)" step frame; graded vs shot `04-appearance-blue-1440.png` | whole | REAL | w26.g02.md:16: "Onboarding-7 (appearance, blue) … Next fill #185DD3 ✓ … ring missing [P1-1]" — the frame is the oracle for an honestly-published GAPS verdict; blue accent #185DD3 exists in tokens (tokens.css `--color-accent: #185dd3` under `[data-accent="blue"]`). |
| 2 | "blue (frame Onboarding-7.png)" rated 8.2 ALMOST | w26.h02.md:127,:204 | REAL | h02's rating row exists (rg-verified :204 "appearance · blue | Onboarding-7.png | 8.2 | ALMOST") — the SVG is the vector source of that render family. |

## 3. Contradictions
None. The consumer docs publish implementation GAPS against this frame (ink/ring deltas)
— it functions as a gap witness, not an achievement prop.
