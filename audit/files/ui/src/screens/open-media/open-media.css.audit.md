# AUDIT: ui/src/screens/open-media/open-media.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 221 / 12 rule groups (≈20 selectors) |
| File verdict | UNREACHABLE from shipped binaries; REAL as a stylesheet (cleanest sheet audited this wave: token-only colors, zero stylelint disables, all selectors emitted) |
| Reachable from shipped entry point? | NO at HEAD (host crate unbuildable, W14-A01 E0425). Imported by OpenMediaDialog.tsx (mounted by AppShell's `lumen:open-media` listener); compiled + jsdom-tested in baseline #22/#23. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | class census `rg -l "<class>" src --glob '!*.css'` for all 18 class families → every one emitted by OpenMediaDialog.tsx (+test); `rg -n "lumen-dialog" src/components/surface/Surface.tsx` → :245 `className={`lumen-dialog ${className}`}` — the compound `.lumen-dialog.open-media__surface` anchor is real; `rg -c "frame:" open-media.css` → 11 frame pins; `rg -n "stylelint-disable" open-media.css` → none; read of OpenMediaDialog.tsx header/wiring. |

## 1. One-paragraph verdict
A small, disciplined sheet: every selector matches a class the dialog really renders, every color rides a semantic token (the only [PROPOSAL] is a font-size token seam where the used token carries the frame's exact 15px value), the divider's negative-margin bleed is documented, and — uniquely among the sheets audited this wave — reduced-motion is handled with the `@media (prefers-reduced-motion: no-preference)` wrapper (motion only when allowed) instead of a redundant `reduce` block, and there are zero inline stylelint disables. The one caveat is inherited from the component, not the CSS: the "Open Media" dialog this sheet skins has no native file picker — it is a paste-a-path form plus the demo catalog and remembered openings, so the desktop user's most basic expectation (an OS file chooser) is absent while the sheet styles the substitute with frame-perfect fidelity.

## 2. What actually works (verified)
— verified by class census: zero orphan selectors (18/18 emitted).
— verified by `rg -n "lumen-dialog" Surface.tsx:245`: the `.lumen-dialog.open-media__surface` compound targets a class the Dialog primitive really applies.
— verified by token discipline scan: no hex literals, no `stylelint-disable`, no color functions — consistent with the sheet header's "Semantic tokens only" claim (this header is TRUE here, unlike variants.css).
— executed: baseline #22 compile, #23 jsdom tests over OpenMediaDialog.test.tsx.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/open-media/open-media.css:1-17 (header, "the app-shell file dialog") + OpenMediaDialog.tsx:25-28
Code (open-media.css:1-5):
```
/* Open-media dialog — W26 Figma language (w26.b13; no direct frame —
 * CONTRACT §3). The surface primitive (components/surface) owns the chrome:
 * scrim (z-index 20 — the modal-layer fix lives there since w26.a08), r14
```
and (OpenMediaDialog.tsx:25-28):
```
// The dialog offers the DemoTransport clip catalog plus the transport's
// remembered openings ("Recent"), and opens the picked SourceSpec through
// the ClientProvider transport (`session.open` / `playlist.load` — the ONE
// funnel, client/open-path.ts), then routes to the player/playlist.
```
What is wrong: The styled surface is a paste-a-path form + pick list; no native OS file chooser exists anywhere in the dialog (no tauri dialog-plugin call — the W14 capabilities audit grants no dialog plugin permission either). "Open Media" in a desktop player means a file picker; here the user must type or paste an absolute path. Honest (blank path honestly invalid, no fabricated sources), but a frame-perfect skin over a reduced interaction model.
Why it matters: The single most-used entry point of a media player (open a file) has no file browser; the meticulous 560px dialog geometry styles manual path entry.
What correct looks like: a `plugin:dialog` open call in the tauri host (capability + one button), with the paste form as the secondary path.
Confidence: HIGH — Verification: OpenMediaDialog.tsx read (no dialog plugin usage); W14-A08 capabilities audit cross-ref (no dialog grant).
Severity: S4 for the CSS file (the sheet itself is correct; the gap is the component's interaction model — recorded here because the header sells it as "the app-shell file dialog").

## 4. Per-function table (rule groups)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.open-media__surface` | 21-26 | REAL | — | compound anchor verified; 560px column; overflow:hidden pins header/footer |
| `.open-media` root | 28-36 | REAL | — | flex column |
| header (strip/title/close) | 40-70 | REAL | — | 50px strip, ellipsized title, [PROPOSAL] font token seam disclosed |
| body | 74-84 | REAL | — | thin scrollbar + token scrollbar-color |
| path form + field | 91-100 | REAL | — | footer-Open-as-submit grammar (form association) |
| hint | 103-107 | REAL | — | |
| full-bleed rule | 110-112 | REAL | — | negative-margin bleed documented |
| sections + titles + list | 116-136 | REAL | — | |
| item rows (+hover/focus) + icon/text/title/source | 143-193 | REAL | — | wrap-anywhere source (audit-note: ellipses hid data); activation buttons, no focus-as-selection |
| empty | 195-199 | REAL | — | |
| actions footer | 204-215 | REAL | — | 100×32 frame pins |
| reduced-motion wrapper | 217-221 | REAL | — | the CORRECT `no-preference` pattern — contrast with controls.css:832 (redundant `reduce` block) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| OpenMediaDialog.test.tsx (baseline #23) | partial | behavioral assertions on rows/forms/routes (mutation-sensitive); geometry pins (560px, 50px, 100px) untested — jsdom has no layout |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (contract §4); raw px appears solely for frame-measured structural dimensions" | open-media.css:13-15 | REAL | zero hex/color literals in the sheet; 11 `frame:` pins |
| "the app-shell file dialog" | open-media.css:2 (via AppShell ownership) | SHALLOW | F-001: no native picker; paste-path + demo catalog + recents |

## 7. Dead / duplicated / unreachable code
None. No orphans, no duplication. Unreachable from shipped binaries (W14-A01).

## 8. Dependencies & flags
Consumes surface primitive classes (`lumen-dialog`), form/button primitives, design tokens (all defined — spot-checked in tokens.css). No flags, no disables.

## 9. Open questions for the next agent
- Tauri lane: was a `plugin:dialog` grant ever proposed (W14 capabilities review)? If yes, F-001 is a regression; if no, it is an unbuilt seam.
- Whoever audits surface.css: confirm the scrim z-index 20 claim (w26.a08 fix) still holds — this sheet depends on it for stacking.
