# CLAIM AUDIT: design/figma/Player-6.svg (text asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | SVG, ASCII text with very long lines (`file`) |
| Size / Lines | 160,005 B / 211 lines |
| Dimensions | 1920×1080 (`width="1920" height="1080" viewBox="0 0 1920 1080"`, :1) |
| sha256 | 14f02f77ca5627c58b9e8b2247424e1f388cc7afce613d84b6937688614d3bac |
| Verdict | REAL (the Player frame variant WITHOUT icon rail and sidebar — canvas full-bleed) |

## 1. Blunt verdict
Genuine Figma-export vector variant of the Player window in its most-reduced
composition: `id="topbar"` + full-window-width control strip
`<rect width="1920" height="54" transform="translate(0 1026)" fill="#1B1B1B"/>` (:85 —
byte-verified) with no icon-rail/sidebar groups. It has a DIRECT code consumer: a CSS
composition block named after it.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Player-6 is the app's sidebar-toggled analogue" (rail and sidebar hidden, canvas spans under the titlebar; same full-width strip) | whole | REAL | w26.e02.md:19 (my e02 ledger) + element-id inventory this run (topbar/frames only — no rail/sidebar groups); strip rect :85 full-width. |
| 2 | Strip `translate(0,1026)` **width=1920** — window-wide band shared with Player.svg | g03.md:86 | REAL | :85 `<rect width="1920" height="54" transform="translate(0 1026)" fill="#1B1B1B"/>` — identical geometry to Player.svg:254 (verified in my e02 ledger). |
| 3 | Code consumer: a Player-6-modeled composition exists in the app CSS | whole | REAL | ui/src/screens/player/player.css:584: "/* ---- Player-6 composition: rail + sidebars all gone (w26.f09, e02 F8) ----" — the frame's composition is implemented and credited to e02's F8 finding; player.css:2 lists Player-6.svg among the composition frames. |

## 3. Contradictions
None. e02's F8 (rail kept after sidebar toggle) was a design-call flag; the CSS block
shows the lane later implemented the Player-6 geometry as the toggle's target state.
