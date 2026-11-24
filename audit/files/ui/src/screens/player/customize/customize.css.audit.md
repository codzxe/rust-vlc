# AUDIT: ui/src/screens/player/customize/customize.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 625 / 45 selector groups |
| File verdict | REAL (pure stylesheet, all selectors live, all vars resolve, layering claims verified true) |
| Reachable from shipped entry point? | YES — CustomizePanels.tsx and toolbarItems.tsx `import "./customize.css"` → CustomizePanels mounts from the player customize flow → PlayerScreen → App → main.tsx. Live in both lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python class/var sweep (45 classes, dead=NONE; 27 vars, missing=NONE); rg scrim z-index (surface.css:35 z-index:20 vs band 21 — claim TRUE); sed quote checks (exit 0) |

## 1. One-paragraph verdict
Real frame-measured stylesheet for the Customize Player modal + edit band. Every selector is referenced by TSX, every custom property resolves (locals `--customize-well-ink/size`, `--well-dash` defined in-sheet at L39-40, L178), and the two cross-sheet claims I could verify statically both check out: the band's z21 sits over the Dialog scrim's z20 (surface.css:35), and the light-mode remap closes the sheet. The only note is `:has()` at line 54 — a modern selector whose support floor (Chromium 105 / Safari 15.4+) is fine for current Tauri webviews but is the newest CSS feature in the sheet; if the scrim-neutralization silently fails, the modal dims the video contrary to the frame.

## 2. What actually works (verified)
— verified by python sweep: 45/45 `.lumen-*` classes referenced in src TSX/TS; 27/27 `var(--…)` names defined (sheet-local or shipped CSS).
— verified by rg: `.lumen-scrim { … z-index: 20 }` (src/components/surface/surface.css:27-35) — the comment "Above the Dialog scrim (z20) so drops reach it" (L294) and `z-index: 21` (L299) are consistent.
— verified by rg importers: CustomizePanels.tsx, toolbarItems.tsx.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/customize/customize.css:54-56
Code:
```
.lumen-scrim:has(> .lumen-dialog.lumen-customize__surface) {
  background: transparent;
}
```
What is wrong: Nothing functionally; flagging the compat floor. `:has()` is required for the "no dark scrim, video bleeds at full value" frame behavior (L45-53). In any webview older than Chromium 105/WebKit 15.4 the rule no-ops and the dialog dims — a silent visual deviation, not a crash. The repo's webview floor is not pinned in any browserslist/config I could find (UNVERIFIED — no browserslist file). Confidence: HIGH on the mechanics — Verification: read.

## 4. Per-function table (file >100 LOC — selector groups)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-customize/-band token pins | 36-41 | REAL | — | raw hex suppressed with frame citation |
| scrim :has() neutralization | 54-56 | REAL | S4 | F-001 |
| __surface deck (1048×655, blur 25) | 65-80 | REAL | — | min() clamps for small viewports |
| .lumen-customize shell + header/title/instruction/close | 82-124 | REAL | — | |
| __box grid (9 cols, 53px row gap) | 128-138 | REAL | — | |
| __tile / __tile-well / __tile-label + hover/drag/drop states | 140-246 | REAL | — | 4-4 dash painted via 4 repeating gradients (native dashed is 3-3) |
| __footer / __cancel / __save (clean-dim) | 250-288 | REAL | — | (0,3,0) override documented |
| .lumen-customize-band (fixed overlay) + __seekrow | 295-324 | REAL | — | z21; runtime-measured geometry, 45px fallback |
| band __time/__row/__cluster/__well family + play/wide/tall variants | 329-421 | REAL | — | |
| __volume-track / __slot / __seek preview (+buffered 7.9% / played 8px) | 423-488 | REAL | — | static frame-0 preview values, honestly labeled preview |
| drop-active / dragging states | 492-511 | REAL | — | |
| per-variant wells (flat/topbarrun/27/23/60/mark/low/ghost/right+shifted) | 518-586 | REAL | — | all 1:1 frame-cited |
| .lumen-toolbar-items (live bar consumer) + spacer/divider | 593-614 | REAL | — | |
| light-mode remap | 622-625 | REAL | — | closes sheet per cascade note |

## 5. Test honesty
No tests in this file. CustomizePanels tests exist (not assigned); visual harness is the fidelity gate — UNVERIFIED here (node_modules absent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "It sits ABOVE the Dialog scrim (z20)" | customize.css:25,294 | TRUE | surface.css:35 `z-index: 20`; band L299 `z-index: 21` |
| "Semantic tokens only … raw values … each carrying its frame citation" | customize.css:27-29 | TRUE | single raw hex (L39) carries citation + lint suppression |
| "the shared Dialog primitive renders .lumen-scrim with no className seam of its own, so this sheet reaches it with :has()" | customize.css:50-52 | TRUE | scrim styled only in surface.css; no className prop observed |

## 7. Dead / duplicated / unreachable code
None. 45/45 selectors live.

## 8. Dependencies & flags
Consumes surface.css (scrim/dialog), icon.css, token set. No feature flags.

## 9. Open questions for the next agent
- Does CustomizePanels.tsx actually measure the band geometry at runtime (comment at L292-294) or is the 45px constant the real value? Verify in the TSX.
