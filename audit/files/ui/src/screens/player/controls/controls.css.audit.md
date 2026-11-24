# AUDIT: ui/src/screens/player/controls/controls.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 836 / 33 selector groups (no functions; pure stylesheet) |
| File verdict | REAL (real stylesheet, every selector live, every var resolvable; 2 token-hygiene defects) |
| Reachable from shipped entry point? | YES — ui/src/screens/player/controls/ControlBar.tsx:80 `import "./controls.css";` → ControlBar renders inside PlayerScreen → App → main.tsx. Both lanes (browser dev shell and desktop webview) load it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg class-usage sweep (exit 0, 33/33 classes referenced in TSX); python var-resolution sweep (exit 0, 37/37 vars defined or runtime-set); rg --gradient-volume-level tree-wide (exit 0 → consumed, never defined); sed quote checks (exit 0) |

## 1. One-paragraph verdict
A real, dense, frame-measured stylesheet for the ControlBar; no dead selectors (all 33 `lumen-controls__*` classes are referenced by non-CSS sources), all 37 custom properties resolve (35 defined in shipped CSS, `--lumen-slider-fill`/`--lumen-slider-thumb` set at runtime by Slider.tsx:99-100), focus-visible and prefers-reduced-motion are honored. Two defects: the volume signal gradient exists twice — once as the phantom token `--gradient-volume-level` (consumed with fallback at slider.css:275, defined nowhere in the tree, promised as "handoff … token lane" here at line 366) and once as raw duplicated hex here — and the wedge reveal formula `* 0.71%` contradicts the sheet's own measured 50% anchor by ~1.4px. Styling only; nothing here wires or unwires a backend.

## 2. What actually works (verified)
— verified by class sweep: every `lumen-controls__[a-z-]+` selector (33 distinct) appears in ≥1 .ts/.tsx file (e.g. `lumen-controls__panel-reason` → ControlBar.tsx/SettingsPanel.tsx) — zero dead selectors.
— verified by python sweep over all ui CSS: all 37 `var(--…)` names are defined somewhere in shipped CSS or set at runtime (Slider.tsx:99-100 sets the two slider vars; verified by rg).
— verified by sed: reduced-motion block (L832-836) kills the only transition-heavy control's animation.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/controls/controls.css:366-367 (with ui/src/components/slider/slider.css:275, ui/src/components/slider/Volume.tsx:8-12)
Code:
```
  /* stylelint-disable-next-line color-no-hex -- frame: volume.svg — the measured signal gradient #95D318→#B2A918(57.9%)→#D37918(79.2%)→#FF3737(100%). ... handoff: --gradient-volume-level (slider.css seam, token lane). */
  background: linear-gradient(90deg, #95d318 0%, #b2a918 57.9%, #d37918 79.2%, #ff3737 100%);
```
What is wrong: `--gradient-volume-level` is consumed at slider.css:275 (`background: var(--gradient-volume-level, var(--color-accent));`) but DEFINED NOWHERE in the repo (rg over src + tokens: only comments/usages). The promised "token lane" never landed, so the token's fallback (flat accent) is the only behavior, and Volume.tsx:10-11's doc claim "the level fill resolves to the frame-measured green→amber→red signal gradient through the --gradient-volume-level token when present" describes a state that never occurs. The gradient values now live as an unsynchronized raw-hex copy in this sheet — one edit in either place silently drifts the two volume surfaces. Currently invisible only because the sole production mount (ControlBar.tsx:477-483) hides the base fill (`controls.css:375-377 display: none`) and paints the raw copy. What correct looks like: define the token once (tokens.css) and consume it in both sheets. Confidence: HIGH — Verification: ran rg tree-wide (`NO-DEFINITION-FOUND`).
F-002 [S4]
Where: ui/src/screens/player/controls/controls.css:369-371
Code:
```
  /* 100% volume ends at ≈71% of the track (measured: 50% = 33px of 89px,
   * 100% = 63px) — the level never reaches the red zone in the 0–1 range */
  clip-path: inset(0 calc(100% - var(--lumen-slider-fill, 0) * 0.71%) 0 0);
```
What is wrong: The linear formula reproduces the 100% anchor (89px × 0.71 = 63.2px ≈ 63px) but not the sheet's own 50% anchor: fill=50 reveals 50 × 0.71% = 35.5% = 31.6px, not the measured 33px (37.1%). ~1.4px error at midpoint; the two measured points are not collinear through 0. Cosmetic; the comment presents both anchors as satisfied. Confidence: HIGH — Verification: read + arithmetic.

## 4. Per-function table (file >100 LOC — selector groups instead of functions)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-controls (+clusters/seek) | 34-80 | REAL | — | min-width:max-content anti-overlap documented |
| __time (+button faces, [data-editing]) | 88-152 | REAL | — | hover chip, caret; matches Go-to-Time seam |
| __play (+[data-glyph], :disabled, idle overrides, ink pins) | 158-274 | REAL | — | (0,6,0) specificity ladder internally documented and consistent |
| [data-playback-state] blocks | 224-279 | REAL | — | idle saturated vs buffering fade |
| seekbar track/fill/thumb/buffer | 287-335 | REAL | — | thumb flush formula documented |
| __volume-slider (wedge) | 342-381 | REAL | S3/S4 | F-001/F-002 |
| __panel family (header/tab/body/section/row/field/check/meta/footer) | 402-736 | REAL | — | 20 structural px constants, all frame-cited |
| __quiet-button / __quiet-label | 669-736 | REAL | — | bare-footer face override |
| __menu-popup overrides | 749-783 | REAL | — | 375px floor, submenu left-open, hover repin |
| __error chip | 790-828 | REAL | — | honest error surface (ControlBar error path) |
| @media reduced-motion | 832-836 | REAL | — | only .lumen-controls__play has transitions |

## 5. Test honesty
No tests in this file. ControlBar.test.tsx (baseline PASS) exercises the DOM this sheet styles; visual harness (scripts/visual-run.mjs) is the actual fidelity gate — not re-run here (UNVERIFIED, node_modules absent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "handoff: --gradient-volume-level (slider.css seam, token lane)" | controls.css:366 | FALSE (never landed) | rg: token consumed slider.css:275, defined nowhere |
| "Semantic tokens only (CONTRACT §4.4); raw px below are structural" | controls.css:26-28 | MOSTLY-TRUE | 2 raw px colors total: the documented gradient (suppressed lint) — everything else is var() (37/37 resolve) |
| "measured: 50% = 33px of 89px, 100% = 63px" | controls.css:369-370 | TRUE as measurements, FALSE as formula result at 50% | F-002 arithmetic |

## 7. Dead / duplicated / unreachable code
No dead selectors. Duplication: the volume gradient (F-001). `.lumen-volume .lumen-slider__fill` (slider.css:274-276) is display:none under the only production mount — dormant, not dead (VolumeSlider is a public primitive).

## 8. Dependencies & flags
Consumes tokens from tokens.css/lumen.css, slider.css grammar (`.lumen-slider__*`), menu.css grammar (`.lumen-menu*`), button.css (`.lumen-icon-button`). No feature flags. stylelint config bans raw hex (stylelint.config.js:52) — the single violation is consciously suppressed at line 366.

## 9. Open questions for the next agent
- Does the visual harness (ui/scripts/visual-run.mjs) have a golden for the volume wedge at 50%? If yes it should have caught F-002; if no, the wedge is untested visually.
