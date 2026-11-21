# AUDIT: ui/src/components/form/TextField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 55 / 3 (TextFieldProps, TextField component, displayName) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the dev/test shell — 9 production consumers (SettingField.tsx:254/278/324/349/368 — every settings text/duration/float/path row; OnboardingScreen, OpenMediaDialog, GotoTimeDialog, BookmarksDialog, areas/{extensions,subtitles,staged,controls}), all on mounted routes. No shipped binary exists (W14 pin). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -ln "<TextField" ui/src` (9 prod files + test); `rg -n -A6 "<TextField"` over aria-props (only staged.tsx:131 passes aria-describedby — to a BUTTON, not the field); form.css read (:53-100 control rules, :595-625 light-mode overrides); `rg -c "color-stroke-input|color-surface-input" tokens.css` → NOT DEFINED (fallback hex live, documented); TextField.test.tsx read in full; tokens.css light block read (content-bright #161616) |

## 1. One-paragraph verdict
Solid, well-tested primitive: label/error/hideLabel wiring is correct and pinned by real tests (including the describedby→error-node id round-trip), ref forwarding works, and the "pending input tokens" debt the header admits is accurately documented in form.css with stylelint-disabled fallbacks — including a light-mode re-skin (form.css:603-606) that fixed exactly the dark-wells-on-light-panels failure the raw fallbacks would otherwise cause. One latent a11y defect: `{...rest}` spreads LAST, so a caller's `aria-describedby` silently replaces the error-message linkage (rendering an orphaned, unannounced error `<p>`), and a caller's `aria-label` overrides the hideLabel name — the repo's own Tooltip/SettingInfoDot grammar merges describedby instead of clobbering it. No current caller triggers the path (verified), hence latent.

## 2. What actually works (verified)
- Label wiring: `htmlFor={inputId}` + `id={inputId}` with useId fallback (lines 25-27, 31, 37) — getByLabelText succeeds in tests; hideLabel mode swaps the visible node for `aria-label` (line 40), keeping one accessible name and one text node (the w26.f02 row-grammar contract, tested at TextField.test.tsx:50-56).
- Error state: `aria-invalid` + `aria-describedby={errorId}` pointing at the rendered `<p id={errorId}>` (lines 41-48) — test asserts the describedby value IS the error node's id (test :23-31), not just that the attribute exists.
- Ref forwarding: forwardRef + ref on the input (lines 23, 36) — test :58-62 asserts HTMLInputElement.
- The `Omit<…, "size">` (line 5) is deliberate: the native size attribute would fight the fixed 34px control height (form.css:57, frame Onboarding.svg 469×34).
- Token-seam honesty: form.css:62/68 use `var(--color-stroke-input, #3a3a3a)` / `var(--color-surface-input, #2a2a2a)` with inline stylelint-disable + "token seam pending" comments; `--color-surface-input`/`--color-stroke-input` are indeed absent from tokens.css (verified) — the comment in THIS file ("token seams pending the input tokens", line 19-20) is accurate, not aspirational cover.
- Light mode is safe: form.css:603-606 re-skins the control (`--color-surface-deep` fill, `--color-stroke-border` stroke) with a comment recording the vision-flagged dark-remnant fix; onboarding.css:251-255 steps one ramp further for the light wizard surface. The unreadable-#161616-on-#2a2a2a scenario the raw fallback implies in light mode does NOT occur.
- Baseline vitest 1705/1705 pass (INDEX #23) includes TextField.test.tsx.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/form/TextField.tsx:40-43
Code:
```tsx
          aria-label={hideLabel ? label : undefined}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
          {...rest}
```
What is wrong: The rest-spread lands after all three aria props, so any caller-supplied `aria-describedby` REPLACES the error linkage while the error message still renders (an orphaned `<p>` no AT will announce), and any caller-supplied `aria-label` overrides the hideLabel accessible name. There is no merge (e.g. `aria-describedby={[rest[\"aria-describedby\"], error ? errorId : undefined].filter(Boolean).join(\" \")}`).
Why it matters: The combination "field with a hint description AND a validation error" is the normal mature form; this primitive silently drops one of the two announcements. The repo already has the correct grammar — Tooltip.tsx merges describedby, SettingInfoDot rides description via a dedicated twin — so the shared text-input primitive is the inconsistent one. Latent today: the only aria-describedby passer near a TextField puts it on a sibling button (staged.tsx:131, verified), and no test covers the combined case.
What correct looks like: Compose describedby (caller + errorId) and only default aria-label from `label` when the caller supplies none — or at minimum document the override precedence in TextFieldProps.
Confidence: HIGH — Verification: read; all 9 consumers' prop usage checked via rg; test file read (no combined-case test).

### F-002 [S4]
Where: ui/src/components/form/TextField.tsx:18-20 (with form.css:62,68,205 and setting-field.css)
Code:
```tsx
 * TextField primitive (wave 26 input grammar: #2a2a2a-class fill, 1px
 * #3a3a3a-class border, r6, focus orange — form.css token seams pending the
 * input tokens).
```
What is wrong: Documented debt, but debt with two independent copies: `--color-surface-input`/`--color-stroke-input` are referenced with identical raw-hex fallbacks in BOTH form.css (:62,68) and setting-field.css (:205), and neither token exists — so the "single source" for the input fill/stroke is currently two manually-synced fallback literals per color, per file.
Why it matters: When the token lane lands the input tokens, any missed consumer keeps its fallback silently; until then, a frame re-pin of the input ink must be edited in two sheets.
What correct looks like: Land `--color-surface-input`/`--color-stroke-input` in tokens.css (the var() consumers then win automatically, as form.css:66-67 itself notes), or consolidate the fallback into one @layer import.
Confidence: HIGH — Verification: read; tokens.css grep (absent); second consumer located.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TextFieldProps` | 5-15 | REAL | — | Omit<"size"> deliberate (CSS-sized grammar); honest prop docs |
| `TextField` | 23-53 | REAL | S3/S4 | spread-last clobber (F-001); `type={rest.type ?? "text"}` at line 38 then re-spread of rest.type at 43 is harmless duplication (same value, just noise); errorId derives from caller id — duplicate caller ids would collide (caller error, not guarded); wrapper className join with filter(Boolean) correct |
| `displayName` | 55 | REAL | — | set for devtools/debugging — correct for forwardRef components |

## 5. Test honesty (if tests exist in/for this file)
TextField.test.tsx (67 lines) — substantive.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "wires label, input and typing" (:13-21) | YES | getByLabelText + change-event spy |
| "error state: aria-invalid + aria-describedby point at the message" (:23-31) | YES | asserts describedby value === error node's id — the full round-trip, not just attribute presence |
| "no error state by default" (:33-36) | YES | absence pins (no stray aria-invalid / error text) |
| "disabled means disabled" (:38-45) | WEAK-BUT-HONEST | asserts the native attribute only; the comment explicitly says fireEvent bypasses the browser gate, so the test honestly scopes itself to the attribute contract |
| "hideLabel keeps the accessible name" (:47-56) | YES | role query by name + asserts NO text node (one-text-node contract) |
| "forwards the ref + DOM snapshot" (:58-62) | YES (ref) | ref instanceof; snapshot pins the DOM shape (snapshot file audited by another agent) |
| Mutation check (reasoned): pass `aria-describedby="hint"` + `error="boom"` | NOT caught | no combined-case test — F-001's latency is unguarded |
| CSS state-matrix tests (:64-78) | YES | regex pins that hover/active/focus-visible/disabled rules consume semantic tokens in form.css — guards the token discipline, not this .tsx |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Error state wires `aria-invalid` + `aria-describedby` to the rendered error message (doc 08 §4.4/§5.4)" | TextField.tsx:20-21 | REAL | lines 41-48 + test :23-31 |
| "form.css token seams pending the input tokens" | TextField.tsx:19-20 | REAL | tokens absent from tokens.css (grep); fallbacks live in form.css with matching comments |
| "hideLabel … the reference mechanism for every field's hidden-label mode (w26.k01 j01-8: BrowseField/NumberField align to THIS)" | TextField.tsx:10-14 | REAL-UNPROVEN (spot-check) | NumberField uses labelHidden (SettingField.tsx:300 call) — the alignment claim is about the one-text-node mechanism; full BrowseField/NumberField audit belongs to their files |

## 7. Dead / duplicated / unreachable code
- None in this file. The duplicated token fallbacks live in form.css/setting-field.css (F-002).

## 8. Dependencies & flags
Imports: react (useId, forwardRef), form.css. No engine/bridge usage — a pure input primitive; every value it carries flows through host onChange handlers. No feature flags. No i18n (label/error are pre-resolved strings by contract).

## 9. Open questions for the next agent
- F-001's clobber pattern (`{...rest}` after aria defaults) — worth a cross-primitive rg (`rg -n "aria-describedby=.*\n.*\{\.\.\.rest\}"` multi-line): Dropdown/Switch/NumberField may share the same latent hole.
- The pending `--color-surface-input`/`--color-stroke-input` tokens have a ready-made consumer pair; the token lane could land them mechanically (fallbacks already document the intended values).
