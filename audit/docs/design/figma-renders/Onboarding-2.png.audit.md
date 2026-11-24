# CLAIM AUDIT: design/figma-renders/Onboarding-2.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A04 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 57,487 B / a394f1579a94e0c764c603ff764cac6b7fda9adedb57fa0a24b5a16926dfb7f6 |
| Verdict | REAL (render of the onboarding SHORTCUTS/"Finish" step — frame `design/figma/Onboarding-2.svg`) |

## 1. Blunt verdict
Genuine render of the wizard's keyboard-shortcuts step: its vector source
`design/figma/Onboarding-2.svg` (1090×759; the 900×627 PNG family is the 0.825 export
scale) is the ONLY SVG in design/figma containing the heading "Learn the basic keyboard
shortcuts." (rg-verified, 1 hit) and the `id="Finish"` vectorized button text (:8) —
i.e. the final wizard frame. Consumer lanes grade the implementation's keycap grammar
against exactly this frame (gfix-D: "Onboarding-2 keycap grammar MATCH").

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Shortcuts step of the 11-frame onboarding set (welcome / appearance-orange / **shortcuts** / 8 accent variants) | whole | REAL | Family mapping (w26.e01.md frame→step table + w26.g02.md): the shortcuts slot is Onboarding-2 — verified by content: the heading lives in Onboarding-2.svg (rg), `id="Finish"` at :8. |
| 2 | "Onboarding-2 keycap grammar MATCH (shortcuts step shares `.onboarding__key`)" | w26.gfix-D.md:93 | REAL (consumer verdict) | gfix-D names this asset by its real filename; the i01/l01 keycap measurements (V 58.7/57, SHIFT 94.2/92, "," 35/35, fill #1B1B1B, stroke #373737, pitch 82px) are SVG-measured against this frame's source. |
| 3 | Uniform 900×627 family geometry (all 11 Onboarding renders identical size) | whole | REAL | `file design/figma-renders/Onboarding*.png` -> all 11 report "900 x 627 … RGBA" (checked this run). |

## 3. Contradictions
One consumer-side quirk, not an asset defect: w26.e01/g02/i01 tables render this file's
name as a mangled "`n`" / "n.png" / "n.svg" (e.g. e01: "`n.png` (shortcuts, \"Finish\")")
while the on-disk name is Onboarding-2 — the mapping is recoverable (the shortcuts
heading exists ONLY in Onboarding-2.svg), but the docs' filename column is corrupted.
Asset itself: coherent, single-purpose, correctly consumed.
