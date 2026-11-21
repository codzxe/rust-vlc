# AUDIT: ui/src/screens/player/tools/video/video.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 461 / 34 rule blocks (+1 @media) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: video tools mount in the player's tools dialog (ToolsDialog → ColorsTool/CropTool/EssentialTool/HdrTool, each `import "./video.css"`; ToolsDialog reachable from PlayerScreen). NO shipped binary mounts it (W14: desktop-tauri E0425 — no GUI binary at HEAD). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | rg class census (!*.css) — 18/18 hooks used; token verification incl. precise WCAG math (python); hex-in-declaration scan (comments only); od -c byte-verification of the composed `[hidden]` rule in dynamics.css; rg `VideoSetCrop` repo-wide (catalog-only); rg `pendingCommand=` staging census. |

## 1. One-paragraph verdict
Styling sheet for the four staged video tools (Colors/Crop/Essential/HDR). Structure is honest and verified: every class hook is consumed by the four tool TSX files, every declaration is token-based (all frame hexes live in comments), the `--video-dropdown-cap`/`data-list-direction` contract matches what useDropdownScrollportFlip actually sets/removes (hook lines 173-175), the separator bleed (`margin: 0 calc(var(--space-inset) * -2)` = -16px) exactly cancels the dynamics body's 16px side insets (dynamics.css:99), and the composed `.dynamics-tool__body[hidden]` collapse rule was byte-verified intact (dynamics.css:102 — an initial rg/cat display garbled the bracketed selector; `od -c` proved the file correct; no finding). The tools themselves are staged-inert BY DESIGN and say so on screen (VideoCard.tsx:100-102 role="note" staged line + `data-pending-command`), styling local-state controls that send nothing to the engine — the hostile question "dead buttons?" answers: interactive-but-staged, visibly marked, exactly one of the awaited commands (`video.set-crop`) even exists in the Rust catalog (spec.rs:371, no executor anywhere). Two findings: the crop unit ink fails WCAG AA in both themes by a wide margin (1.92:1 / 1.58:1, computed), and the row-label comments cite the frame's #6B6B6B while the applied token is #828282.

## 2. What actually works (verified)
- Class census: `rg -l "<hook>" --glob '!*.css'` → all 18 video-* hooks consumed by ColorsTool/CropTool/EssentialTool/HdrTool (+video-sep--1..5 at ColorsTool.tsx:271-295, video-crop-* in CropTool, video-hdr* in HdrTool).
- Token discipline: hex scan — every `#…` in this file sits in a comment; declarations use tokens (verified values: --color-stroke-border #404040/:52, --opacity-emphasis 0.7/:86, --shadow-card 0.38α/:104, --color-surface-panel #191919/:60).
- Flip contract: `--video-dropdown-cap` fallback 320px (:446) + `data-list-direction` pins (:442-453) match useDropdownScrollportFlip.ts:173-175 (sets/removes both).
- Geometry: `.video-sep` -16px bleed = exact inverse of `.dynamics-tool__body` side padding `calc(var(--space-inset) * 2)` (dynamics.css:96-100).
- Collapse machinery (composed, not owned here): `.dynamics-tool__body[hidden] { display: none }` verified BYTE-LEVEL via `od -c` (dynamics.css:102) — the rg/`cat -A` display of that line was garbled by ANSI handling in my capture; the rule is intact, so collapsed cards do hide (VideoCard.tsx:97 `hidden={!open}`).
- Staging honesty: colors/crop/essential/HDR send nothing; each card carries `pendingCommand=` (video.set-colors / video.set-crop / video.set-essential — ColorsTool.tsx:224, CropTool.tsx:173, EssentialTool.tsx:133) rendered as a visible staged note (VideoCard.tsx:100-102, role="note"). Of the three named commands only `video.set-crop` exists in the Rust catalog (crates/commands spec.rs:371) and it has NO executor (rg VideoSetCrop over *.rs → definition sites only).
- Baseline: vite build compiles the sheet (INDEX.md row 22); vitest tool suites passed (row 23) asserting DOM structure (jsdom — no computed styles).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/video/video.css:286-289
Code:
```
.video-crop-input__unit {
  font-size: var(--font-size-section);
  color: var(--color-stroke-border); /* frame unit: #404040 (token-exact) */
}
```
What is wrong: The "px" unit ink is `--color-stroke-border` — #404040 on the #090909 well (dark) and #9a9a9a on the #c2c2c2 well (light). Precise WCAG contrast (computed): 1.92:1 dark, 1.58:1 light. AA requires 4.5:1 for normal text; this fails both themes by a factor >2. The stroke-border token is a BORDER color being used as TEXT ink.
Why it matters: The unit is meaningful text (it tells the user the crop fields are pixels, not percent); at ~1.6–1.9:1 it is unreadable for low-vision users and fails the repo's own axe-a11y discipline (playlist.css:51-54 documents tertiary 3.2:1 being bumped for exactly this reason). "Token-exact" to the frame is honored; readability is not.
What correct looks like: Use a content token for text ink (tertiary passes at 5.18:1 on the same well), and reserve stroke tokens for strokes; or file a frame deviation note.
Confidence: HIGH — Verification: computed contrast (python, formula-checked against known pairs); token values read.

### F-002 [S4]
Where: ui/src/screens/player/tools/video/video.css:45-47, 59-62 (same claim at :79, :97, :176, :196, :244)
Code:
```
/* Hex+swatch+toggle rows and plain toggle rows: one 32px control band, label
 * left (#6B6B6B ≈15px — the shared .dynamics-row__label grammar via
 * .video-row__label), control group right. */
```
What is wrong: The comments pin the label ink to the frame's #6B6B6B, but the rule applies `--color-content-tertiary` (#828282 — :62). A token matching the frame value exists (`--color-content-quiet: #6b6b6b`, tokens.css:40) and is not used. The applied value is the BETTER choice (4.58:1 on the surface-panel card vs the frame's 3.23:1) — but the comment claims frame fidelity it does not have, in five places.
Why it matters: Frame-fidelity comments are the audit trail to the SVGs; a reader diffing against "Video - Colors.svg" would flag the code as off-frame when the comment says it is on-frame (or vice versa trust a false pin).
What correct looks like: Comment states the substitution ("frame #6B6B6B → tertiary #828282 for AA; quiet token reserved") or the code uses content-quiet with a logged deviation.
Confidence: HIGH — Verification: read + token values + computed contrast (4.578:1 tertiary on #191919).

## 4. Per-function table (rule blocks)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.video-tool .dynamics-row__value` hide | 39-41 | REAL | — | Value column hidden, DOM kept, aria-valuetext carries values (disclosed) |
| `.video-row` (+label/spacer) | 48-70 | REAL | — | 32px band, 47px pitch (32+15 verified); F-002 comment drift |
| `.video-row + .dynamics-row` seam | 75-77 | REAL | — | 19.5px frame-measured seam |
| `.video-hex` (+input/swatch) | 81-120 | REAL | — | 116×32 well; swatch background set inline (ColorsTool.tsx:166-169) — CSS positions only, correct split; color-mix() inner stroke (modern-webview dependency, degrades to no stroke) |
| `.video-sep` (+colors per-gap margins --1..5, +dynamics-row) | 129-164 | REAL | — | Bleed −16px cancels body insets exactly; per-gap margins frame-measured (20/19, 20.5/19, 10/9, 10/14, 15/8.5+14) |
| colors body pad / first-row pull | 138-140, 168-170 | REAL | — | −5px header-band delta, 24px bottom pad (frame) |
| `.video-subdropdown` (+trigger rim) | 178-197, 376-390 | REAL | — | Two-layer rim via border on surface-panel fill; hover/open re-declared (specificity ascending, disclosed) |
| `.video-tool--crop` pulls | 205-211 | REAL | — | Same 54px band logic as Colors |
| `.video-crop-row` (+input/label/value/control/unit) | 215-289 | REAL | S3 | F-001 unit ink; `field-sizing: content` (:270) is Chrome123+/Safari17.4+ — documented 3ch fallback (graceful) |
| `.video-crop-link` (+hover/focus/aria-pressed) | 294-325 | REAL | — | ≥32px hit area; unlocked dims via opacity 0.7; focus-visible outline |
| `.video-hdr` (+trigger rim/hover/open, list shadow) | 338-394 | REAL | — | Two-layer rim token-exact; shadow-card 0.38α ≈ frame 0.35 (nearest-token, disclosed) |
| HDR check mask `::after` | 405-414 | REAL | — | 14×11 masked vector, accent-icon token (themeable) — replaces ✓ glyph |
| `.video-hdr__staged` | 416-420 | REAL | — | The staged note this sheet styles (honest marker) |
| Dropdown clipping caps | 442-453 | REAL | — | Contract with useDropdownScrollportFlip verified |
| `@media (prefers-reduced-motion)` | 457-461 | REAL | — | Guards own transition (:309) |

## 5. Test honesty
ColorsTool.test.tsx / CropTool.test.tsx / EssentialTool.test.tsx / HdrTool.test.tsx / useDropdownScrollportFlip.test.ts assert DOM/ARIA and the flip state machine — none can see CSS (jsdom). Would they fail if the CSS broke? No. Style application covered only by the UNRUN visual harness. The flip hook's own test is behavioral (state machine) and real.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (stylelint + eslint enforce); the frame hexes … live in comments" | video.css:28-30 | REAL | Hex scan: declarations token-only |
| Card shell "NOT re-declared here … imported from ../dynamics/dynamics.css" | :20-22 | REAL | VideoCard.tsx:39 imports dynamics.css; no .dynamics-tool__toggle/base re-declared here (only scoped overrides) |
| "no color-threshold/… engine stage or catalog command exists" (staging) | ColorsTool.tsx:26-27 (cross-ref) | REAL for colors/essential (catalog has no video.set-colors/set-essential) | rg over crates/commands — absent |
| Same claim in CropTool.tsx:28 | CropTool.tsx:28 (cross-ref) | FALSE on the catalog half | `video.set-crop` EXISTS in the catalog (crates/commands/src/catalog/spec.rs:371) though no executor handles it — the UI still correctly stages (nothing to call); comment overstates |
| "the slider's aria-valuetext still reports every value" | :37-38 | REAL | VideoSliderRow feeds percent() to aria-valuetext (ColorsTool.tsx:50) |

## 7. Dead / duplicated / unreachable code
None in this sheet: 18/18 hooks consumed; the reduced-motion block guards its own transition.

## 8. Dependencies & flags
tokens/generated/tokens.css (all resolve), dynamics.css (composed card grammar — collapse rule byte-verified), menu.css (Dropdown primitive; shared 44px rows claimed, cross-file), useDropdownScrollportFlip.ts (runtime contract). Modern-CSS dependencies: `color-mix()` (:119), `field-sizing: content` (:270) — both documented with fallbacks; UNVERIFIED on the shipped webview (no GUI binary; webkit2gtk version unknown in sandbox).

## 9. Open questions for the next agent
- dynamics.css is A01's file (this wave) — my byte-verification of its :102 `[hidden]` rule clears a suspicion worth re-checking with od if their tooling garbles brackets.
- CropTool.tsx:28's "no … catalog command exists" vs spec.rs:371 — the tool TSX audits (colors/crop/essential/hdr are split among A01/A09/A10 per the wave TSVs) should carry that as a comment-accuracy finding (S4); cross-ref from here, not double-counted.
- The a11y contrast failures (F-001) should be swept across sibling tool sheets (dynamics/equalizer/visualizer use the same quiet-ink grammar) by their owners.
