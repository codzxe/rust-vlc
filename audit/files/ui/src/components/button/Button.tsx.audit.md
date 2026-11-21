# AUDIT: ui/src/components/button/Button.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 71 / 2 types + props interface + BASE_VARIANTS + Button (forwardRef) + displayName |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): 9 files import it (census: rg "components/button/Button" — ConvertScreen, OpenMediaDialog, dialogs, settings, onboarding surfaces). The app's primary action control. Behaviorally tested (Button.test.tsx, 20 tests, baseline vitest row 23). Ceiling: no GUI binary at HEAD (W14). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read of Button.tsx + button.css (512 lines, full) + states.md excerpts + Button.test.tsx; ls docs/waves/w26/logs/w26.a03.md (EXISTS — the cited lane log is real); rg consumer census (9 importers); rg `<Button[^>]*loading` (1 production consumer: OpenMediaDialog.tsx:288). |

## 1. One-paragraph verdict
The Button primitive is the most disciplined component audited so far in this wave: every doc comment in the file is machine-checkable against button.css and every one checks out — the five variants map to real CSS (default/secondary share the base class by construction via BASE_VARIANTS, and the test pins that "one secondary look"; primary/tinted/ghost modifiers all exist with token-only styling), the 28/32/36 size ladder matches .lumen-button--sm/--lg heights exactly, the loading contract (aria-busy + disabled + 0.7-opacity spinner, motion-off under prefers-reduced-motion) is implemented and behaviorally tested (click on a loading button fires nothing), and the frame citations are backed by a real lane log (docs/waves/w26/logs/w26.a03.md — exists, unlike dialogIcons' unverifiable one) with the one frame deviation (#1B1B1B dialog fill vs shipped #222222 raised) explicitly disclosed in states.md:131-133 rather than hidden. The `type="button"` default kills the implicit-submit trap. The single finding is an a11y trade-off in the loading design: `disabled={disabled \|\| loading}` means a focused button that enters loading immediately loses focus to `<body>` (disabled elements cannot hold focus) — with a live consumer (OpenMediaDialog's submit button, loading={opening}), so every keyboard open flow drops focus mid-operation.

## 2. What actually works (verified)
- Variant↔CSS parity: BASE_VARIANTS ["default","secondary"] render no modifier class (:39, :57) — "explicit alias of default (renders the same classes)" is enforced by code, pinned by test :44-51; --primary/--tinted/--ghost rules exist (button.css:92, 124, 174) with per-state (:hover/:active/:focus-visible) coverage including specificity management for the nested --color-stroke-icon remaps (:112-119).
- Size ladder: base 32px (button.css:42), --sm 28px (:193-195), --lg 36px (:197-199) — matches the tsx comment "28 (play pill/compact rows) / 32 (dialog buttons) / 36 (segmented + footer bars)"; test :125 pins it.
- Loading contract REAL: aria-busy={loading \|\| undefined} (:63), disabled={disabled \|\| loading} (:64), CSS 0.7 opacity (--opacity-emphasis) + spinner ::before + pointer-events:none (button.css:215-229) + motion-off (:508-511). Test :85-97 asserts aria-busy + disabled + click-dead.
- Default type="button" (:49) — no implicit form submission from stray Buttons; consumers opt into submit explicitly (OpenMediaDialog passes type="submit" + form=).
- Native semantics: no role/button ARIA reimplementation — Enter/Space free (test :60 acknowledges it honestly rather than faking keydown tests).
- Focus-visible: 2px accent outline at 2px offset (button.css:83-88); disabled = tertiary on muted well + pointer-events none (:203-212).
- Light-theme contrast sweeps documented with computed ratios (tinted 1.75:1 → 7.8:1 fix, button.css:152-170; IconButton stroke-icon 2.1:1 → 4.5:1, :284-298) — disclosed out-of-lane fixes with lane-log flags.
- Cited artifacts exist: docs/waves/w26/logs/w26.a03.md (ls verified), states.md carries the anatomy + the #1B1B1B frame-residue disclosure (:131-133).
- ref forwarding + displayName (:70); test :99-103 asserts the ref lands.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/button/Button.tsx:63-64
Code:
```
      aria-busy={loading || undefined}
      disabled={disabled || loading}
```
What is wrong: Entering the loading state flips `disabled` on. A disabled HTML button cannot hold focus, so the moment a focused (clicked or Enter-activated) button transitions to loading, the browser moves focus to `<body>`. When loading resolves, focus does not return — keyboard and screen-reader users are dropped to the top of the tab order mid-flow.
Why it matters: Live consumer: OpenMediaDialog.tsx:288 (`<Button type="submit" … loading={opening}>`) — every keyboard-driven open flow loses focus the instant the pending state starts. The "fully inert" contract (:32-34) is honored, but inertness could keep focus by suppressing via aria-busy + pointer-events + a keydown guard instead of the disabled attribute (the CSS already ships pointer-events:none for aria-busy, so the disabled flip only adds the keyboard block and the focus loss).
What correct looks like: Keep the button enabled while loading with aria-busy + pointer-events:none (CSS already present) + onClick guard `if (loading) return`, preserving focus; or restore focus in a loading-end effect at the consumer.
Confidence: HIGH on the mechanism (disabled-focus behavior is browser-native; disabled + focus = blur is well-defined), MEDIUM on impact depth (no focus-management test exists; jsdom tests never assert focus after loading).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ButtonVariant type | 20 | REAL | — | 5 variants, all with CSS counterparts |
| ButtonSize type | 24 | REAL | — | sm/md/lg = 28/32/36 (CSS-verified) |
| ButtonProps | 26-36 | REAL | — | Extends ButtonHTMLAttributes; variant/size/loading documented |
| BASE_VARIANTS | 39 | REAL | — | The alias mechanism; test-pinned |
| Button (forwardRef) | 47-68 | REAL | S4 | Spread order: rest last (only aria-busy is overridable — benign); type default "button"; F-001 loading-disabled |
| displayName | 70 | REAL | — | Forward-ref naming |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders a native button (role + default type)" :27 | YES | type=button default pinned |
| "applies variant and size modifier classes" :34 / "base variants … one secondary look" :44 | YES | Class composition + the alias contract |
| "fires click handlers (mouse)" :53 / keyboard note :60 | YES | Honest: keyboard activation is browser-native, test does not fake it |
| "disabled means disabled" :72 / "loading means inert" :85 | YES | aria-busy + disabled + handler-silence all asserted |
| "forwards the ref" :99 / "DOM snapshot" :105 | YES | Ref lands; snapshot pins class anatomy |
| CSS-source suite :115+ (rest/hover/pressed/focus/disabled/loading/primary/tinted heights…) | YES | Asserts button.css rules consume the named tokens — the tsx doc comments are test-enforced, not prose |
| (gap) | — | No focus-retention test across loading (F-001 untested) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "default — raised fill + 1px border, r6 … frames: 100x32 r6, fill #1B1B1B, label #8F8F8F" | :6-9 | REAL (frame cite) / DISCLOSED DEVIATION | CSS ships surface-raised #222222 + stroke-border; states.md:131-133 discloses the frame residue ("this lane ships raised #222222 + 1px stroke-border per the task card") |
| "secondary — explicit alias of default (renders the same classes)" | :10 | REAL | BASE_VARIANTS + test :44 |
| "primary — accent fill, on-accent (dark) glyph, r6 … No frame shows a pill-radius (r30) button" | :11-14 | REAL | button.css:92-119; test :160; the r30 negation is the lane log's ruling |
| "tinted — dialog confirm … #371F06 bg + #D37918 text" | :15-17 | REAL | --color-accent-tinted + --color-accent (tokens.css values match the cited hexes); light-theme contrast fix disclosed at button.css:152-157 |
| "ghost — transparent, 1px transparent border, hover surface-hover" | :18-19 | REAL | button.css:174-189 (border-color:transparent keeps box stability) |
| "loading … 0.7 opacity + CSS spinner + aria-busy. A loading button never fires click handlers — it is fully inert" | :31-34 | REAL | CSS :215-229 + disabled flip + test :85; side effect F-001 |
| "anatomy in button.css + docs/waves/w26/logs/w26.a03.md" | :42-43 | REAL | Both exist (ls verified) |
## 7. Dead / duplicated / unreachable code
- None in this file. The switch/toggle CSS in button.css (Sibling components Toggle/Switch) is out of scope here (their .tsx files are separate; button.css itself is another agent's assignment).
## 8. Dependencies & flags
React only (forwardRef, ButtonHTMLAttributes). Consumers: 9 files. Companion files: button.css (skin), states.md (anatomy log), index.ts (barrel). The i18n rules: no literals in this file (label copy is consumer-supplied children). IconButton/Toggle share button.css but are separate components (own test files, real).
## 9. Open questions for the next agent
- OpenMediaDialog owner: F-001's focus-drop is observable in the open-media flow — if any e2e a11y lane (axe) runs on a loading state, it would surface there; none does today (loading is exercised only in unit tests).
- button.css (unassigned to me): the switch-core duplication story (.lumen-toggle* vs .lumen-switch* shared selectors) and the two light-sweep blocks are documented; whoever audits it should verify Toggle.tsx/switchCore.tsx against those selectors.
