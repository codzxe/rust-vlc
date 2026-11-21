# AUDIT: ui/src/screens/player/dialogs/dialogIcons.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 73 / 2 (CloseGlyph, ChevronGlyph) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): consumers GotoTimeDialog.tsx:142 (Close 16), BookmarksDialog.tsx:278,297 (Close 12/16), ToolsDialogShell.tsx:111,180 (Close 16, Chevron 24) — the player tool dialogs, reached from the player screen; dialog wiring verified in my dialogs.css audit (file 2). Ceiling: no GUI binary at HEAD (W14). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read of the file + consumers; rg of design/figma "Audio - Equalizer.svg" Vector 27 (`M356 39L361 44L356 49`, 1.5 stroke, #6B6B6B) and "Open network stream.svg" Vector 20/21 (`M1018 20L1028 30` / `M1028 20L1018 30`, 1.5 stroke, #8F8F8F) — frame geometry extracted verbatim; rg of shared icon set (icons.tsx:111 "close" `M7 7L17 17`, :421 "chevron-right" `M9.5 7L14.5 12L9.5 17`); dialogs.css:427-436 chevron rotation read; python bbox math on both paths. |

## 1. One-paragraph verdict
CloseGlyph is a faithful, fully verified frame port: the frame's Vector 20/21 in "Open network stream.svg" is a 10×10 ✕ with 1.5 stroke and #8F8F8F ink (extracted verbatim), and the code's 15-of-24-unit geometry at size 16 renders exactly 10px with a 2.25-unit stroke rendering exactly 1.5px — the "w26.a18 pixel re-verify" comment's arithmetic checks out to the unit. ChevronGlyph does not: its cited frame vector (Audio - Equalizer.svg Vector 27) is a STEEP chevron 5 units wide × 10 tall, the shared icon set's chevron-right is the faithful 5×10 port, but this module's path is 8.5 wide × 7 tall — a SHALLOW chevron whose aspect ratio (w>h) inverts the frame's (h = 2w) — while its own comment claims "~10×6" dimensions and "14 units wide renders ≈9px" (the path is 8.5 units wide; nothing in the file is 14 units wide). The dialogs.css rotation rule (:433 "Collapsed points right (frame Vector 27)") compounds the false citation. Second finding: the module's premise — the shared icon set "does not carry yet" the ✕ and chevron — is false at HEAD: icons.tsx:111 carries "close" (documented as "identical to the Player.svg title-bar close (Vector 20/21)") and :122-125 carries all four chevrons; the honest rationale for the close duplicate is the render size (10-unit shared ✕ needs size 24 for 10px; the dialog grammar wants 10px at size 16 → 15-unit variant), which the header never states.

## 2. What actually works (verified)
- CloseGlyph frame fidelity: frame Vector 20/21 = 10×10 crossed lines, stroke-width 1.5, stroke #8F8F8F (verbatim from the design file). Code: 15 units of the 24 grid (M4.5→19.5) at size 16 → 15×16/24 = 10px rendered; strokeWidth 2.25 units → 1.5px. Both frame numbers reproduced exactly.
- Token discipline: stroke="currentColor" + no fill → the glyphs inherit token ink (#8F8F8F quiet class via --color-stroke-icon family); no raw hex in the module (stylelint/eslint clean by construction).
- Decorative-glyph a11y: aria-hidden + focusable="false" on both — correct for glyphs inside labeled buttons (the dialog close buttons carry their own accessible names — GotoTimeDialog/BookmarksDialog/ToolsDialogShell consumers).
- Chevron rotation contract: dialogs.css:427-436 rotates .tools-card__chevron 90° on [aria-expanded=true] — the "expanded points down" half of the comment is real CSS, not prose.
- Consumer wiring: all three dialog consumers import and render both glyphs at the sizes the comments were verified for (16 default; BookmarksDialog's size-12 close is a 7.5px ✕ — sub-10px variant, no frame claim attached).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/dialogs/dialogIcons.tsx:44-49 (comment) + :70 (path); frame evidence design/figma/Audio - Equalizer.svg Vector 27
Code:
```
 * Chevron — frame: "Audio - Equalizer.svg" ("Icon"/"Vector 27": right-pointing
 * 1.5-stroke chevron, #6B6B6B ink, right end of the card's toggle bar; the
 * expanded state points down — hosts rotate via CSS on aria-expanded).
 * Geometry: ~10×6 frame glyph on the shared 24-unit grid (14 units wide
 * renders ≈9px at size 16, stroke 2.25 units = the frame's 1.5px).
```
```
      <path d="M8 8.5L16.5 12L8 15.5" />
```
What is wrong: The cited frame vector is `M356 39L361 44L356 49` — 5 wide × 10 tall (a steep chevron, height twice the width). The shipped path is `M8 8.5L16.5 12L8 15.5` — 8.5 wide × 7 tall (shallow, wider than tall): the aspect ratio is inverted versus the frame. The comment's own numbers match neither: "~10×6 frame glyph" (frame is 5×10) and "14 units wide renders ≈9px" (the path spans 8.5 units; nothing in the glyph is 14 units). The shared icon set's chevron-right (icons.tsx:421 `M9.5 7L14.5 12L9.5 17` — 5×10) is the faithful port, proving the frame geometry was available and not used here.
Why it matters: This repo's design-honesty system is frame-pinned geometry with vector-citing comments and "pixel re-verify" claims; this component ships a visibly different shape (shallow vs steep chevron) on every tools-card toggle bar in the player dialogs while citing the exact vector it contradicts. dialogs.css:433 repeats the false citation ("Collapsed points right (frame Vector 27)").
What correct looks like: `M9.5 7L14.5 12L9.5 17`-class geometry (the frame's 5×10 proportions) at the size the bar needs, with the comment's numbers matching the path; or correct the citation to what was actually drawn.
Confidence: HIGH — Verification: frame SVG read verbatim (Vector 27 extracted), python bbox math on both paths, shared chevron compared.
### F-002 [S4]
Where: ui/src/screens/player/dialogs/dialogIcons.tsx:1-7 (module premise)
Code:
```
// Player-dialog glyphs (w26.a18) — the two small stroke glyphs the dialog
// grammar needs that the shared icon set (components/icon) does not carry
// yet: the header close ✕ and the tool-card chevron.
```
What is wrong: The premise is false at HEAD: icons.tsx:111 carries IconName "close" (comment: "the ✕; identical to the Player.svg title-bar close (Vector 20/21)" — the same vector family this module re-ports) and :122-125 carries chevron-up/down/left/right. Both glyph families the module claims are missing exist in the shared set. The genuine (unstated) rationale for the close duplicate is render size: the shared ✕ is 10-of-24 units (10px at size 24), the dialog grammar needs 10px at size 16 → the 15-unit variant here. The chevron duplicate has no size rationale at all (see F-001 — it is not even the faithful shape).
Why it matters: "Additive local module" framing hides that this is glyph-family duplication; a future reader hunting for the ✕/chevron finds two divergent sources. The promised Lead follow-up ("adding the glyphs to the shared icon set") is already half-obsolete.
What correct looks like: Either restate the premise honestly (size-variant ports pending shell support for per-instance scale) and de-duplicate the chevron onto the shared 5×10 geometry, or upstream both with a size prop.
Confidence: HIGH — Verification: icons.tsx IconName union + both path geometries read.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CloseGlyph | 18-42 | REAL | — | Frame-verified geometry (10×10 ✕, 1.5px stroke at size 16); currentColor ink; SVGProps passthrough minus width/height; size default 16 |
| ChevronGlyph | 51-73 | REAL | S3 | Renders and rotates (dialogs.css:435) but contradicts its cited frame vector and its own comment (F-001) |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no dedicated unit test for this module) | — | Consumers' tests exercise the dialogs end-to-end (dialog harness tests assert chrome presence), but NO test pins glyph geometry — F-001's drift is exactly what an untested decorative module accumulates |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Geometry is mined from the W25 frames (paths translated onto the shared 24×24 grid …)" | :3-5 | FALSE for the chevron / REAL for the close | Close: frame Vector 20/21 verified (10×10 → 15u@16=10px, 1.5px stroke). Chevron: frame Vector 27 is 5×10; path is 8.5×7 (F-001) |
| "the shared icon set (components/icon) does not carry yet: the header close ✕ and the tool-card chevron" | :1-3 | FALSE | icons.tsx:111 "close" + :122-125 chevrons exist (F-002) |
| "stroke via currentColor so tokens tint them — same conventions as components/icon/icons.tsx" | :5-6 | REAL | stroke="currentColor", fill="none" on both |
| "15 units wide (renders 10px at size 16) with a 2.25-unit stroke (renders the frame's 1.5px)" | :14-16 | REAL | 15×16/24 = 10px; 2.25×16/24 = 1.5px; frame stroke-width 1.5 verified in the SVG |
| "14 units wide renders ≈9px at size 16, stroke 2.25 units" | :48-49 | FALSE | Path spans 8.5 units; nothing is 14 units (F-001) |
| "the expanded state points down — hosts rotate via CSS on aria-expanded" | :47-48 | REAL | dialogs.css:435 rotate(90deg) on [aria-expanded=true] |
## 7. Dead / duplicated / unreachable code
- F-002: duplicates the shared icon set's "close" (size variant, unstated rationale) and "chevron-right" (no rationale; wrong shape — F-001).
- No dead exports: both glyphs consumed (three dialog files).
## 8. Dependencies & flags
React SVGProps only; no imports beyond react. Consumers: GotoTimeDialog, BookmarksDialog, ToolsDialogShell. Design sources: design/figma "Open network stream.svg" (Vector 20/21), "Audio - Equalizer.svg" (Vector 27). No feature flags. Lint coverage: .tsx under src → both local ESLint rules (no literals, no raw colors — clean).
## 9. Open questions for the next agent
- dialogs.css:433's "(frame Vector 27)" citation inherits F-001 — the CSS owner should re-cite or the glyph should be fixed; the rotation direction (90° from a shallow chevron) also differs visually from a 90° rotation of the steep frame chevron.
- If the Lead's follow-up (upstreaming these into components/icon) lands, F-002's duplication resolves — check the lane log claim ("recorded in the lane log") is real; I did not locate a lane log file for w26.a18 (UNVERIFIED — no such file found by that name; possibly internal to a wave log outside ui/).
