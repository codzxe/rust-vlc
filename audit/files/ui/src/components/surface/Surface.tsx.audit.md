# AUDIT: ui/src/components/surface/Surface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 473 / 12 (2 hooks, 2 module helpers, 8 components) |
| File verdict | REAL (code) — 3 of 8 components DEAD (no production consumer) — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | Dialog/Panel/Card/Separator/SectionHeader: YES in dev/web build (12/5/6/4/4 consumer files, e.g. `<Dialog` in screens/*); NO in any shipped product (sole host desktop-tauri cannot compile, W14 SUMMARY l.19). Sheet/Well/HeroPill: NO anywhere outside their own unit test. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg `<Dialog|<Sheet|<Panel|<Card|<Well|<Separator|<SectionHeader|<HeroPill` across src (consumer counts 12/1/5/6/1/4/4/1 — the 1s are the test file itself); rg `lumen-well|lumen-sheet|lumen-hero-pill` outside components/surface → 0 hits; rg surface.css selectors → all emitted classes styled incl. 4 sheet sides; read Surface.test.tsx test list (529 lines, 30 cases) |

## 1. One-paragraph verdict
The focus-trap engineering is real and unusually well-reasoned — Escape ownership across popups (open-state detection, not bare role presence), IME composition guard, nested-trap stopPropagation, jsdom-vs-browser Tab semantics, focus restore to the opener — and 30 unit tests exercise exactly those branches. But three of the eight exported components (`Sheet`, `Well`, `HeroPill`) have zero consumers outside their own test file: 40+ lines of component and their CSS rules ship as dead weight, kept "green" by tests that mock the world they claim to serve. The trap's visibility filter is also deliberately degraded for jsdom: only `[hidden]`-attribute subtrees are skipped, so CSS-hidden focusables stay in the Tab cycle in real browsers.

## 2. What actually works (verified)
— verified by rg consumer sweep → Dialog (12 files), Panel (5), Card (6), Separator (4), SectionHeader (4) are live in screens; all emitted class names have rules in surface.css (checked `.lumen-scrim`…`.lumen-hero-pill--ring`, incl. all four `.lumen-sheet--{side}` variants).
— verified by reading Surface.test.tsx:56-105 → focus-in/restore, Tab wrap both directions, Escape reason, scrim-click gating are behaviorally tested (fireEvent + focus assertions), not string pins.
— verified by reading the trap code paths (l.145-190): nextIndex arithmetic correct at boundaries (−1 → 0 forward, ≤0 → last backward); empty-items path focuses the tabIndex=-1 surface.
— baseline rows 22-23: the module compiles and its suite runs green.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/surface/Surface.tsx:266-305 (Sheet), 384-386 (Well), 453-473 (HeroPill)
Code:
```
export function Sheet({
  open,
  onClose,
```
What is wrong: `Sheet`, `Well`, and `HeroPill` are exported (index.ts) but have ZERO production consumers: rg across ui/src for `<Sheet`/`<Well`/`<HeroPill` matches only `components/surface/Surface.test.tsx`; rg for their emitted classes (`lumen-sheet`, `lumen-well`, `lumen-hero-pill`) outside components/surface/ returns nothing. Their CSS (surface.css:85-158 sheet+sides, 270-281 well, 332-343 hero-pill) is equally dead.
Why it matters: ~120 lines of component + CSS ship in the bundle and must be maintained against token/cascade changes for no user. Worse for honesty: HeroPill's doc comment sells it as the Onboarding-frame surface ("In the Onboarding frames the 60×60 rx30 shape is the hero disc") — onboarding does not use it. Their unit tests (Surface.test.tsx:345-388, 471-477, and the hero-pill cases) pass while exercising nothing any user can reach — the textbook "test keeps dead code green".
What correct looks like: delete the three components, their tests, and their CSS; or wire Sheet/Well/HeroPill into the screens the comments claim they serve.
Confidence: HIGH — Verification: rg sweeps (commands above), zero non-test hits.

### F-002 [S3]
Where: ui/src/components/surface/Surface.tsx:164-175
Code:
```
      // The trap takes over Tab entirely (preventDefault + explicit move) so
      // cycling is deterministic in browsers *and* jsdom (which applies no
      // default Tab movement). Visibility check is structural because jsdom
      // has no layout (offsetParent is always null): skip `hidden` subtrees.
      // w26.k04 (j04-10): stopPropagation keeps a nested surface's own trap
      // from double-handling the same Tab press.
      event.preventDefault();
      event.stopPropagation();
      const items = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.closest("[hidden]") === null,
      );
```
What is wrong: The cyclable set is filtered only by the `[hidden]` ATTRIBUTE. In real browsers, focusables hidden by CSS (display:none via state classes, `visibility:hidden` — e.g. the player chrome's `data-chrome="hidden"` rules in states.css:65-70 use `visibility: hidden`) remain in the list; `items[nextIndex].focus()` happily focuses a `visibility:hidden` control, so Tab focus silently vanishes mid-cycle. The comment states the tradeoff was chosen because jsdom has no layout — test convenience degraded production behavior.
Why it matters: A modal dialog is the one place keyboard users cannot escape by clicking; losing visible focus inside it is a hard a11y failure (WCAG 2.4.3 focus visibility). Any dialog that conditionally hides controls with CSS classes (collapsed sections, step forms) instead of the `[hidden]` attribute hits this.
What correct looks like: dual check — `[hidden]` attribute for jsdom AND a real-geometry check (`el.offsetParent !== null` or `getClientRects().length > 0`) that jsdom stubs can bypass via a test-only escape hatch (e.g. skip geometry when `navigator.userAgent` indicates jsdom, or expose the filter for tests).
Confidence: HIGH on the mechanism (visibility:hidden elements are focusable — standard DOM behavior); MEDIUM on real-world frequency inside current dialogs (no CSS-hidden focusable inside a live dialog verified — the chrome-hidden case is outside dialogs).
Note: the same limitation is explicitly acknowledged in-code, i.e. known-and-shipped.

### F-003 [S4]
Where: ui/src/components/surface/Surface.tsx:140-143
Code:
```
    const opener = document.activeElement instanceof Element ? document.activeElement : null;
    // Move focus into the surface (first focusable, else the surface itself).
    const focusables = node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    (focusables[0] ?? node).focus();
```
What is wrong: `focusables[0]` is the first focusable in DOM order with no visibility filtering — the initial focus can also land on a CSS-hidden control (same root cause as F-002, at open time rather than cycle time). Additionally, `opener` restore in the cleanup (l.195) fires even when the surface closes because its ANCESTOR unmounted (screen switch): `opener.focus()` on a detached node is a silent no-op — acceptable, but focus is then simply lost with no compensation.
Why it matters: Initial-focus and restore edge cases are where focus traps usually fail users; the code handles the happy paths only.
What correct looks like: reuse F-002's corrected filter for the initial focus; on unmount-close, fall back to `document.body` when the opener is detached.
Confidence: HIGH on code reading; behavior verified by tests only for the happy path (Surface.test.tsx:56-68).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FOCUSABLE_SELECTOR` (const) | 31-42 | REAL | — | includes contenteditable/audio[controls]/video[controls]/summary (j04-11) — verified against MDN focusability |
| `OPEN_POPUP_TRIGGER_SELECTOR`/`POPUP_ROLE_SELECTOR` | 73-75 | REAL | — | open-state detection, dialog-trigger exclusion |
| `popupOwnsFirstEscape` | 77-83 | REAL | — | document-wide trigger check + focused-popup-outside check; both branches tested (test:179-256) |
| `warnUnnamedSurface` | 90-96 | REAL | — | DEV-gated; honest about empty names |
| `useSurfaceFocusTrap` | 122-198 | REAL | S3 | F-002 (visibility filter), F-003 (initial focus/restore); Escape IME guard + stopPropagation correct; onCloseRef sync-in-effect correct |
| `useSurfaceA11y` | 201-207 | REAL | — | labelledBy-preferred, useId fallback |
| `Dialog` | 219-257 | REAL | — | 12 consumers; scrim click gated; sr-title only when synthesized |
| `Sheet` | 266-305 | DEAD (production) | S3 | F-001; only consumer is its own test |
| `Panel` | 319-342 | REAL | — | 5 consumers; emits `.lumen-pane` (name-collision note in doc — honest) |
| `Card` | 356-376 | REAL | — | 6 consumers; variants map 1:1 to surface.css:239-268 |
| `Well` | 384-386 | DEAD (production) | S3 | F-001 |
| `Separator` | 394-410 | REAL | — | 4 consumers; aria-hidden decorative default correct |
| `SectionHeader` | 419-445 | REAL | — | 4 consumers; host-chosen heading level |
| `HeroPill` | 453-473 | DEAD (production) | S3 | F-001; doc comment implies onboarding usage that does not exist |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| test:56-68 focus in/restore | YES | real focus() assertions on rendered DOM |
| test:69-92 Tab wrap / Shift+Tab wrap | YES | simulates keydown, asserts document.activeElement |
| test:94-104 Escape reason + scrim gating | YES | behavioral |
| test:179-270 popup Escape ownership (4 cases) | YES | renders real popup trigger states, asserts surface survives/dies correctly — this is the file's best work |
| test:273-309 `[hidden]` skip + zero-focusables | YES | but only the `[hidden]` attribute path — the CSS-hidden hole (F-002) is exactly what the test does NOT cover |
| test:311-328 contenteditable/summary cycling | YES | behavioral |
| test:345-388 Sheet cases / 471-477 Well / hero-pill cases | YES (they test the code) | but the code under test is unreachable in production (F-001) — tests validate dead software |
Mutation check (reasoned): break `nextIndex` wrap arithmetic → tests 69-92 fail; delete `popupOwnsFirstEscape` yield → tests 179-256 fail. The suite is genuine; its blind spot is production visibility semantics (F-002) and production reachability of 3 components (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Esc closes overlays" (doc 08 §4.5) | l.8-9, 116-119 | REAL | implemented l.146-163 + tests 94-104 |
| "focus is restored to the opener" | l.119-120 | REAL | l.193-196 + test:56-68 |
| "In the Onboarding frames the 60×60 rx30 shape is the hero disc" (HeroPill rationale) | l.448-451 | TRUE of the FRAMES, FALSE as a usage claim | onboarding screens contain zero `HeroPill`/`lumen-hero-pill` references (rg) |
| "Sheet = edge-anchored dialog" (doc 08 §4) | l.5-6, 259-265 | REAL (code) / UNREACHABLE | implementation + 4-side CSS exist; zero production consumers |

## 7. Dead / duplicated / unreachable code
Dead: Sheet (40 lines + surface.css:85-158), Well (3 lines + css:270-281), HeroPill (21 lines + css:332-343) — F-001. Duplicated: none significant (Dialog/Sheet share the trap and markup shape intentionally). Unreachable: everything, in the shipped sense (W14).

## 8. Dependencies & flags
React only (`useEffect/useId/useRef`), `import.meta.env.DEV` gate for the warning, `./surface.css` for chrome. No feature flags. No token literals in this file (all visual grammar lives in surface.css).

## 9. Open questions for the next agent
- surface.css owner (not in my list): confirm the dead sheet/well/hero-pill rules and consider whether `--shadow-card`/`--opacity-scrim` tokens exist solely for dead rules.
- Whoever audits AddSubtitlesDialog/SettingsScreen: report any CSS-hidden (non-`[hidden]`) focusables inside live dialogs — that converts F-002 from risk to observed defect.
