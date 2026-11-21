# AUDIT: ui/src/screens/player/controls/controls.css#part2 (lines 419-836)
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 418 (chunk 2 of 2; part1=1-418 is A02's) / 24 rule groups (≈60 selectors) |
| File verdict | UNREACHABLE from shipped binaries; REAL as a stylesheet (every selector emitted by SettingsPanel.tsx / ControlBar.tsx; all 20 tokens verified defined) |
| Reachable from shipped entry point? | NO at HEAD (host crate unbuildable, W14-A01 E0425; baseline #2 exit 101). Sheet imported by ControlBar.tsx (compiled in baseline #22, rendered in #23's jsdom tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | selector census: `sed -n '419,836p' … | rg -o '^\.[a-z-_]+'` → 25 top-level classes; each `rg -l "<class>" src --glob '!*.css'` → all emitted (panel family → SettingsPanel.tsx:153-339, ControlBar.tsx:538,677; quiet button/label → SettingsPanel.tsx:392; error chip → ControlBar.tsx:1102-1105; menu-popup → ControlBar.tsx); token loop over 20 names → all defined in src/tokens/generated/tokens.css; `rg -n "prefers-reduced-motion" src/theme/lumen.css` → global rule at 171-180; `rg -n "panel-tab-label" controls.css` → no rule (F-002); sed quote verification. |

## 1. One-paragraph verdict
The popover half of the control-bar sheet (settings panels, app-menu popup, error chip) is consistent, token-clean, and fully wired: all ~60 selectors in the range match classes the components really render, including the honest inert reason row (`.lumen-controls__panel-reason`) and the engine-truth check column (`[data-checked]` from SettingsPanel.tsx:242). Two small defects: the sheet's closing `prefers-reduced-motion` block disables transitions for exactly ONE selector (`.lumen-controls__play` — a part1 class) while part2 itself defines six more transitions, and the whole thing is redundant with the app-wide reduced-motion rule in lumen.css:171-180 — a block that looks like sheet-wide policy but is neither; and SettingsPanel emits a `.lumen-controls__panel-tab-label` class no rule in either half of the sheet styles. Unreachable from shipped binaries like all of ui/src.

## 2. What actually works (verified)
— verified by class census (commands above): zero orphan selectors in 419-836.
— verified by reading ControlBar.tsx:1102-1105: the error chip renders from real engine error state (`error.code` → `data-error-code`, role="status", title+detail+Dismiss) — the one part2 surface that carries live backend truth end-to-end (when a transport exists).
— verified by token loop: all 20 custom properties referenced (surface*, content*, stroke*, accent*, radius*, space*, font-*, motion-*, opacity-disabled, shadow-overlay) exist in the generated token sheet.
— executed: baseline #23 (ui-vitest 1705 passed) includes SettingsPanel.test.tsx / ControlBar tests that render this range's DOM; #22 compiles it.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/controls/controls.css:832-836 (vs src/theme/lumen.css:171-180)
Code:
```
@media (prefers-reduced-motion: reduce) {
  .lumen-controls__play {
    transition: none;
  }
}
```
What is wrong: The sheet-final motion block covers exactly one selector — `.lumen-controls__play`, a part1 class — while part2 itself declares transitions on `.lumen-controls__panel-tab` (444), `.lumen-controls__panel-row` (552), `.lumen-controls__quiet-button` (685-687), `.lumen-controls__quiet-label` (726). It is also fully redundant: lumen.css:171-180 already sets `transition-duration: 0.01ms` for `*, *::before, *::after` under the same media query.
Why it matters: A reader concludes this sheet manages its own reduced-motion coverage; it does not (and does not need to). Dead rule + misleading signal.
What correct looks like: delete the block (the global rule governs), or make it a deliberate sheet-wide `none` if the 0.01ms global is considered insufficient.
Confidence: HIGH — Verification: read both files.

### F-002 [S4]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:158 vs controls.css (no rule)
Code (SettingsPanel.tsx:157-158):
```
              <Icon name={tab.icon} size={14} />
              <span className="lumen-controls__panel-tab-label">{tab.label}</span>
```
What is wrong: `.lumen-controls__panel-tab-label` is emitted by SettingsPanel.tsx:158 but no rule in controls.css part1 or part2 (or any other sheet) targets it — `rg -n "panel-tab-label" controls.css` → no match. The span is styled purely by inheritance from `.lumen-controls__panel-tab`.
Why it matters: A named class hook with no styling contract is dead weight and invites future rules that silently never apply (or get "fixed" by styling a class that exists for no reason).
What correct looks like: drop the class, or give it the rule it exists to host.
Confidence: HIGH — Verification: rg across all CSS in ui/src.
Severity: S4.

## 4. Per-function table (rule groups)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| panel-header | 419-429 | REAL | — | 50px header, token colors; SettingsPanel.tsx:153 tabs |
| panel-tab (+svg/hover/focus/active/underline) | 431-482 | REAL | S4 | F-002: emitted label sub-span unstyled; token-lane handoff noted at 440 |
| panel-body / panel-section / section-title / section-icon | 488-531 | REAL | — | section-icon is quiet + inert (no seam) — disclosed at 526-528 |
| panel-row (+hover/focus/disabled) + row-label + row-value | 535-586 | REAL | — | disabled → opacity token + pointer-events none; tabular durations |
| panel-check + [data-checked] ✓ | 590-601 | REAL | — | engine-reported selection only (SettingsPanel.tsx:242 data-checked); ✓ as glyph content |
| panel-reason | 604-613 | REAL | — | the honest inert row (doc 08 §4.4), emitted at SettingsPanel.tsx:339 |
| panel-field (+label/value) | 617-637 | REAL | — | defaults rows |
| panel-segmented | 642-646 | REAL | — | ControlBar.tsx:538 |
| panel-meta | 650-656 | REAL | — | ControlBar.tsx:677 |
| panel-footer + quiet-button (+hover/focus) | 663-701 | REAL | — | Button.png grammar; UA-font pitfalls documented |
| quiet-label (bare footer face) | 710-736 | REAL | — | pixel-caught live fix documented (panel-gear.png evidence) |
| menu-popup anchor + width + submenu-left + hover repin | 749-783 | REAL | — | static-wrapper fix (w26.i03) + right-edge submenu flip; hover via --color-surface-selected per gfix-C note; runtime position UNVERIFIED (jsdom no layout) |
| error chip family | 790-828 | REAL | — | wired to real error state (ControlBar.tsx:1102-1105); ellipsized detail; r12 chip |
| reduced-motion block | 832-836 | REAL (redundant) | S4 | F-001 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| SettingsPanel.test.tsx / ControlBar tests (baseline #23) | partial | behavioral DOM assertions (tab order, checked rows, reason rows, error chip) are mutation-sensitive; all geometry this part pins (44px rows, 375px menu, 20px floats) is untested — jsdom has no layout, Playwright visual runs exist but are not in the baseline set (playwright.visual.config.ts; no baseline run) |
| baseline #22 | — | sheet compiles |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "inert (no seam)" for the section-header rescan glyph | controls.css:526-528 | REAL | no onClick in SettingsPanel.tsx:198 usage — icon renders without interaction |
| "Stage/code detail stays in the DOM (single line, honest)" (error chip) | controls.css:787-789 | REAL | ControlBar.tsx:1102-1105 renders code+title+detail unconditionally |
| "--color-surface-selected carries exactly #1f1f1f (tokens.json)" | controls.css:777-780 | REAL | verified in themes/tokens.json (gen-tokens pipeline; W15-A01 part1/2 auditors' range) |

## 7. Dead / duplicated / unreachable code
F-001 (redundant motion block), F-002 (unstyled emitted class). No other orphans. Whole range unreachable from shipped binaries (W14-A01).

## 8. Dependencies & flags
Consumes shared menu/slider/segmented classes + 20 design tokens (all verified). No hex literals in this range (all colors via tokens — the cleanest sheet audited so far). No feature flags.

## 9. Open questions for the next agent
- A02 (part1): the `.lumen-controls__play` transition your range declares is the only one the sheet-local reduced-motion block covers — see F-001; consider joint cleanup.
- Visual-lane owner: do the Playwright visual snapshots (dev-shots) pin this range's popover geometry? Baseline runs #22-24 do not execute them (REQUEST-ORCH-RUN candidate for the wave gate if popover fidelity matters).
