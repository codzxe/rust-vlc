# AUDIT: ui/src/components/surface/surface.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 342 / 30 rule blocks (+2 @media, 4 @keyframes) |
| File verdict | REAL |
| Reachable from shipped entry point? | PARTIAL: Dialog/Card/Panel/Separator/SectionHeader/scrim/pane grammar is production-reached (AppShell.tsx:68 Panel; OpenMediaDialog.tsx:46 Dialog+Separator; ConvertScreen.tsx:49 Card+SectionHeader; DiscoveryScreen, MediaInfoPanel, all player dialogs). Sheet/Well/HeroPill are NOT (see F-001). No shipped binary mounts any of it (W14: desktop-tauri E0425). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | NO_COLOR=1 rg production-consumer census per primitive (excluding *.test.*); lumen.css separator/panel rules read; z-index cross-check (menu.css); token verification; vite.config.ts read (no CSS purge). |

## 1. One-paragraph verdict
The surface primitive sheet: scrim, Dialog, Sheet, Panel(.lumen-pane), Card, Well, Separator, SectionHeader, HeroPill. The reached half is correct and disciplined — scrim z-index 20 really does outrank the menu popup layer's 10 (menu.css:85), tokens all resolve, sheet edge-anchoring geometry is right, animations are gated behind `prefers-reduced-motion: no-preference` (the positive form). The problem is dead weight: THREE of the nine primitives (Sheet, Well, HeroPill) have ZERO production consumers — no screen imports them; only their own unit test (Surface.test.tsx) mounts them — and nothing purges CSS (vite.config.ts has no purge plugin; the sheet imports wholesale), so ~140 lines of unreachable styling ship in every bundle. Sheet's `--top` side is dead even in its own test. Two comment falsehoods: the `.lumen-pane` naming rationale cites a `.lumen-panel` rule that lumen.css explicitly REMOVED (j18 F7), and the `.lumen-separator` comment claims "ONLY the orientation geometry" is layered while the rule re-declares `background`.

## 2. What actually works (verified)
- Production-reach census (NO_COLOR rg, non-test TSX): Dialog ← OpenMediaDialog, NewPlaylistDialog, AddSubtitlesDialog, GotoTime/Bookmarks/Tools dialogs, NetworkStreamScreen, controls.tsx, Dropdown.tsx; Panel ← AppShell.tsx:68 (rail, edges="end"); Card ← ConvertScreen/DiscoveryScreen/PlaylistScreen; Separator ← NewPlaylistDialog, NetworkStreamScreen, OpenMediaDialog; SectionHeader ← ConvertScreen, DiscoveryScreen, MediaInfoPanel. The sheet's core grammar is load-bearing.
- Scrim layering claim (:31-35): menu.css:85 `z-index: 10` — scrim's 20 wins; the "modal never loses the paint race" fix is real in the cascade.
- Tokens: --radius-card 14px, --radius-card-lg 30px (the HeroPill "r30" claim), --stroke-ring 4px, --motion-duration-fast 120ms, --opacity-scrim-strong 0.24, --shadow-overlay, --radius-window 14px — all present in tokens/generated/tokens.css.
- Dialog chrome (:45-59): max-width min(92vw, 480px) / max-height min(86vh, 720px), surface-panel fill + hairline + r14 + overlay shadow — matches the figma-fidelity citations; dialogs.css (my file 2) correctly rides this primitive instead of re-specing.
- Sheet geometry (:108-156): per-side fixed anchoring, hairline only on free edges, inner-corner radii — correct; the four entrance animations run ONLY under `prefers-reduced-motion: no-preference` (:158-174) — correct positive gating.
- `.lumen-separator[data-orientation]` cascade-order independence: data-attribute selectors (0,2,0) outrank lumen.css's class rule (0,1,0) regardless of import order — the claim's mechanism is sound; vertical hairline margin:0 override disclosed (:297).
- Baseline: vite build compiles the sheet (INDEX.md row 22); Surface.test.tsx in the 1705-test vitest pass (row 23).

## 3. Findings
### F-001 [S3]
Where: ui/src/components/surface/surface.css:83-202 (Sheet), :268-273 (Well), :330-342 (HeroPill)
Code:
```
.lumen-sheet {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  overflow: auto;
```
What is wrong: The Sheet (4 side variants + 4 entrance animations + 4 keyframes, ~120 lines), Well (6 lines) and HeroPill (+ring, 13 lines) primitives have NO production consumer: `rg -l` over non-test TSX finds Sheet/Well/HeroPill only inside Surface.tsx itself; the sole mount site of all three is Surface.test.tsx. Nothing tree-shakes CSS (vite.config.ts: no purge/minify-css purging of unused rules — the sheet is imported wholesale for Dialog), so ~140 lines of unreachable styling ship in the production bundle. Additionally `.lumen-sheet--top` (:122-129, animation :167-169, keyframes :190-195) is exercised by NOTHING, not even the test (Surface.test.tsx uses side="left"/"bottom"/default only, :348/:363/:411).
Why it matters: Dead-primitive weight in every bundle, and a false affordance for contributors: three exported "primitives" suggest an established grammar when no screen uses them — the exact pattern lumen.css j18-F7 condemned and fixed for `.lumen-panel`/`.lumen-row` ("removed rather than advertised as a grammar nothing uses", lumen.css:152-155).
What correct looks like: Either delete the three (and their exports/tests) until a consumer exists, or keep them as documented staging primitives with an explicit "awaiting consumer" note — the video-tools staging pattern (visible markers) applied at the primitive layer.
Confidence: HIGH — Verification: NO_COLOR rg census (test vs non-test), barrel read (components/surface/index.ts exports all 9), vite.config.ts read.
### F-002 [S4]
Where: ui/src/components/surface/surface.css:205-210
Code:
```
/* Class is .lumen-pane, NOT .lumen-panel: the theme base sheet
 * (theme/lumen.css, theme lane) ships .lumen-panel as the r12–14 CARD
 * grammar. Window-level app panels are a different concept (square, flush
```
What is wrong: lumen.css does NOT ship `.lumen-panel` — its j18-F7 pass REMOVED `.lumen-panel`/`.lumen-row` as never-adopted (lumen.css:152-155), and rg confirms no `.lumen-panel` rule exists anywhere. The collision-avoidance rationale for the `.lumen-pane` name cites a rule that was deleted; the comment is stale in the opposite direction of the truth (the name is now collision-free because the OTHER name is gone).
Why it matters: A reader auditing the "flagged to the Lead for cross-lane unification" handoff will chase a nonexistent rule.
What correct looks like: Update the comment: ".lumen-panel was removed by lumen.css j18 F7; .lumen-pane stands alone; unification flag resolved."
Confidence: HIGH — Verification: rg `.lumen-panel` (rule) over ui/src → comments only.
### F-003 [S4]
Where: ui/src/components/surface/surface.css:277-286
Code:
```
/* The theme base sheet (theme/lumen.css) owns the shared .lumen-separator
 * hairline (hr grammar: border:none + quiet inset margins). This module
 * layers ONLY the orientation geometry on top — ...
.lumen-separator {
  flex: none;
  background: var(--color-stroke-border);
}
```
What is wrong: "ONLY the orientation geometry" — but the rule also re-declares `background: var(--color-stroke-border)`, a paint property lumen.css:158-163 already sets identically. Harmless duplication (same value), but the comment misdescribes what the layer does, and the duplicated paint is exactly the kind of drift the layering note exists to prevent.
Why it matters: If lumen.css ever retunes the separator ink, this hidden duplicate keeps the old value alive for one import order — a stale-paint trap documented as impossible.
What correct looks like: Drop the background line (lumen.css owns paint) or amend the comment to "geometry + a defensive background re-pin".
Confidence: HIGH — Verification: read both rules.

## 4. Per-function table
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-scrim` | 27-41 | REAL | — | z-index 20 verified vs menu.css 10; raw-black-over-token-opacity disclosed with stylelint disable + [PROPOSAL] token note |
| `.lumen-dialog` (+tone deep, focus-visible) | 45-68 | REAL | — | Production-reached; chrome matches frames; tone=deep used by customize modal |
| `.lumen-surface__sr-title` | 71-81 | REAL | — | Synthesized a11y title (Surface.tsx); clip-path modern equivalent noted |
| `.lumen-sheet` + 4 side variants + anims | 83-202 | UNREACHABLE (prod) | S3 | F-001; geometry itself correct; --top dead everywhere |
| `.lumen-pane` (+tone, data-edges) | 212-235 | REAL | — | AppShell rail/sidebar consumers; square edges per W26 grammar; F-002 stale comment |
| `.lumen-card` (+raised/interactive/floating) | 239-266 | REAL | — | Consumers verified; hover transition under no-preference gate |
| `.lumen-well` | 270-273 | UNREACHABLE (prod) | S3 | F-001 |
| `.lumen-separator` (+orientation) | 283-298 | REAL | — | F-003 comment; cascade-order-independence mechanism sound; vertical margin:0 disclosed |
| `.lumen-section-header` (+title/trailing/data-rule) | 302-328 | REAL | — | Consumers: Convert/Discovery/MediaInfoPanel |
| `.lumen-hero-pill` (+ring) | 332-342 | UNREACHABLE (prod) | S3 | F-001; r30 verified (--radius-card-lg 30px); color-mix ring |

## 5. Test honesty
Surface.test.tsx (in baseline vitest pass): Dialog/Sheet trap and close-reason tests are behavioral (would fail on regression). The Sheet/Well/HeroPill coverage tests only the never-mounted primitives — they verify library behavior, not product reachability; nothing tests that a real screen uses them (nothing does). Would CSS break them? No (jsdom).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "scrim veil + modal layer z-index 20, Escape + scrim-click close" | dialogs.css:3-4 (cross-ref from my file 2) | REAL | z-index 20 here (:35) vs menu.css:85 z-index 10; escape/outside-click semantics in Surface.tsx (closeOnOutsideClick prop, GotoTimeDialog.tsx:94) |
| "theme base sheet … ships .lumen-panel" | surface.css:206-207 | FALSE | lumen.css:152-155 removed it (j18 F7) — F-002 |
| "This module layers ONLY the orientation geometry" | surface.css:279-280 | FALSE | background re-declared — F-003 |
| "Dialog/sheet surfaces r19–21 via --radius-window" | :12-13 | REAL | --radius-window: 14px — the "r19–21" frame range is approximated by the 14px token (documented nearest-token policy); mild overstatement of fidelity, not hidden |
| "NO drop shadows except where a frame shows one" | :19-21 | REAL | Only --shadow-card (floating card) + --shadow-overlay (modals) appear |

## 7. Dead / duplicated / unreachable code
- F-001: Sheet (~120 lines incl. 4 animations + 4 keyframes), Well (6), HeroPill (13) — production-unreachable; `.lumen-sheet--top` unreachable even in tests.
- F-003: duplicated `background` on `.lumen-separator`.

## 8. Dependencies & flags
tokens/generated/tokens.css (all resolve), theme/lumen.css (separator/panel shared grammar — F-002/F-003 seams), components/menu/menu.css (z-index 10 layer contract). Vite build has NO CSS purge — dead rules ship. `color-mix()` in the hero-pill ring (modern webview; unreachable primitive anyway).

## 9. Open questions for the next agent
- Surface.tsx is A01's file this wave — F-001's reachability census was CSS-driven; the TSX audit should confirm no dynamic Sheet/Well/HeroPill mounts (rg shows none, but a string-built component reference would evade my census).
- Whoever audits the web-budget gate (baseline row 16 passed): ~140 dead CSS lines are inside the budget that passed — the budget does not catch dead code.
- j18 F7's lumen.css cleanup precedent suggests the fix pattern for F-001; the "flagged to the Lead" note in F-002 should be resolved in the same pass.

## ADDENDUM (added during the themes/generated/tokens.css audit, same session)
The §2 line "overlay shadow — matches the figma-fidelity citations" is imprecise for the SHADOW value specifically: the header comment (:12-14) cites "0 8px 15px rgb(0 0 0 / 46%)" as the frame measurement mapping to `--shadow-overlay`, but the generated token is `0px 4.76px 5.9px 0px rgba(0, 0, 0, 0.7)` (themes/generated/tokens.css:105; source-measured per themes/tokens.json's shadow.overlay "floating-window shadow — neutral black, measured"). The comment cites a different measurement than the token it names — a comment-vs-token drift of the same class as F-002/F-003 (would be an S4 if counted; recorded here rather than renumbering the file's findings). The token itself is correct per its source; the surface.css citation prose is stale.
