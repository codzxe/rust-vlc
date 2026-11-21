# AUDIT: ui/src/components/form/Switch.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 46 / 1 (Switch) + 1 const (switchSkin) + 1 interface |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web build): form/index.ts:11 re-exports; consumed by settings rows (SettingWidgets.tsx:220, staged.tsx:101, effects/subtitles/extensions areas), onboarding (OnboardingScreen.tsx:411,845). Baseline #22 build OK; #23 vitest aggregate green (Switch.test.tsx's 9 tests in the run). No GUI binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` Switch.tsx (complete) + button/switchCore.tsx (the core) + button/useSwitchLabels.ts (i18n seam); `rg -ln "<Switch" src --glob '*.tsx'` (consumers: SettingWidgets, staged, onboarding, effects, subtitles, extensions, controls, SettingField); `rg -n "lumen-switch" src/components/button/button.css` (skin rules :357-454, shared selector lists confirmed verbatim at :356-357 and :430-431); `rg -c "lumen-switch" settings.css effects.css subtitles.css` (frozen-hook claim: 2/1/1 hits — real); `rg -n '"switch.on"\|"switch.off"' src/i18n/messages.ts src/i18n/keys/w26.k01.ts` (registered in shared catalog :2974-2975 AND lane :15-16); read Switch.test.tsx (103 lines) + __snapshots__/Switch.test.tsx.snap (snapshot audited as its own file); spot-check of three consumers' aria labeling. |

## 1. One-paragraph verdict
A model thin-skin: 3 lines of runtime code (forwardRef → SwitchCore with the frozen `.lumen-switch*` class grammar) over the shared switch core, and every claim in its 25 lines of comments verifies — the class hooks really are consumed by settings.css/effects.css/subtitles.css, the skin really is styled by one shared selector-list block in button.css (j01-3 dedup, confirmed verbatim), the j01-9 OFF-ink contrast fix really is in both grammars, and the ON/OFF micro labels really flow through the i18n seam (registered in messages.ts, with a lane fallback). The a11y contract (role=switch, aria-checked, native button keyboard, disabled gating) is real and test-pinned, and the core's `if (disabled) return` guard is load-bearing in jsdom, not dead code. One hygiene defect: the frozen class grammar inverts the meaning of `__fill` across the two switch families — in `.lumen-toggle*` it names the 33×26 ON segment, in `.lumen-switch*` it names the 26×26 knob — so every shared button.css rule pairs deliberately non-equivalent names, a trap the file documents but perpetuates.

## 2. What actually works (verified)
— Verified by `rg -c "lumen-switch"`: the frozen-hook claim (:13-14 "settings.css, effects.css, subtitles.css") — settings.css:2, effects.css:1, subtitles.css:1 rule hits; the grammar really is load-bearing screens-side.
— Verified by sed of button.css:349-360, :424-435: the shared selector lists exist verbatim (`.lumen-toggle,\n.lumen-switch {` … `.lumen-toggle__knob,\n.lumen-switch__fill {`), matching every regex Switch.test.tsx pins — one stylesheet, two grammars, no divergence possible.
— Verified by rg of i18n: `switch.on`/`switch.off` are registered in the shared catalog (messages.ts:2974-2975, "ON"/"OFF") with the identical-copy lane keys as fallback (w26.k01.ts:15-16) — the j01-4 seam claim holds, and the test's mock ("AN"/"AUS") proves the catalog wins when it serves values.
— Verified by read of consumers: every spot-checked usage provides an accessible name (aria-label at staged.tsx:102, SettingWidgets.tsx:221; aria-labelledby at OnboardingScreen.tsx:414) — the component itself is nameless by design (labels are the row's job) and its consumers honor that.
— Verified by baseline #23 aggregate (EXIT=0): the 9-test suite (semantics, disabled gating, micro-label classes, i18n seam, snapshot, 4 css-skin tests) is in the green run.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/form/Switch.tsx:16-21
Code:
```ts
const switchSkin: SwitchSkin = {
  root: "lumen-switch",
  onSegment: "lumen-switch__text lumen-switch__text--on",
  knob: "lumen-switch__fill",
  offSegment: "lumen-switch__text lumen-switch__text--off",
};
```
What is wrong: the knob is classed `lumen-switch__fill`, but in the sibling grammar `.lumen-toggle__fill` is the 33×26 ON TEXT SEGMENT — the same suffix names different anatomy in the two families. button.css must compensate in every shared rule by pairing non-equivalent names: `.lumen-toggle__knob` (knob) shares a rule with `.lumen-switch__fill` (knob), while `.lumen-toggle__fill` (segment) shares a rule with `.lumen-switch__text--on` (segment) — verified verbatim in button.css and pinned by the test's regexes (Switch.test.tsx:69,75,95).
Why it matters: "naming that lies" inside the one place (the skin map) a maintainer reads to learn the grammar; a future contributor "fixing" the switch knob by copying toggle rules keyed on `__fill` styles the wrong element, and the error is invisible until pixel review.
What correct looks like: this is frozen public API by declared design ("the class set is frozen", :14-15 — screens' CSS hooks depend on it), so the fix is a comment in the skin map itself stating the inversion (`// __fill = knob in this grammar; = ON segment in .lumen-toggle* — do not copy rules across`), retiring the trap without a breaking rename.
Confidence: HIGH — Verification: read of both skins + button.css selector lists + Toggle.tsx sibling.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `switchSkin` (module const) | 16-21 | REAL | F-001 | Pure data; the four class strings all exist as rules in button.css (:357-454) — no dead hooks. |
| `Switch` (forwardRef) | 42-46 | REAL | — | 3-line composition: ref + skin + prop spread. The core's `if (disabled) return` guard (switchCore.tsx onClick) is load-bearing under jsdom's click-on-disabled quirk, not dead code. No ignored params. |
| `SwitchProps` | 5-10 | REAL | — | Re-declares `checked`/`onChange` identical to SwitchCoreProps — redundant but contract-pinning (an incompatible core change errors HERE first, which is the point of a frozen public API). Not a defect. |

## 5. Test honesty (Switch.test.tsx, green in baseline #23 aggregate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "exposes switch semantics with checked state" | YES | role=switch + aria-checked pinned directly. |
| "changes through onChange; disabled means disabled" | YES | Mutation: delete the core's disabled guard → fireEvent.click on the disabled control fires onChange(false) → `not.toHaveBeenCalledWith(false)` fails. The guard is what the test proves. |
| "carries both decorative ON/OFF micro labels" | YES | Class grammar + aria-hidden + aria-checked trio pinned; mutation of the skin map fails the `toHaveClass` assertions. |
| "j01-4: labels flow through useMsg()" | YES | Mocked catalog ("AN"/"AUS") + absence of lane fallback copy — deleting the useMsg seam (hardcoding lane keys) fails it. |
| "DOM snapshot" | YES (as change-detector) | See the snapshot's own audit file — pins the full anatomy incl. the mocked labels. |
| 4 css-skin tests (:66-102) | YES | Regex-pin button.css shared blocks and form.css's NON-possession of switch rules — mutation (a second skin in form.css) fails the negative assertions (:98-102). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "settings/onboarding row CSS keeps its `.lumen-switch*` hooks (settings.css, effects.css, subtitles.css)" (:13-14) | this file | REAL | rg counts 2/1/1 in those three sheets. |
| "role="switch" + aria-checked, onChange on click/keyboard, native disabled gating" (:26-27) | this file | REAL | switchCore.tsx read; Space/Enter free via native button; tests pin all three. |
| "the two exported names stay (frozen public API), the implementation and the CSS skin are shared" (:30-32) | this file | REAL | button.css shared selector lists verified; form/index.ts:11 + button/Toggle both re-export the core. |
| "Geometry is an honest port of Toggle.svg … 68×32 rx6 track; ON = 33×26 … 26×26 fill" (:35-38) | this file | REAL (as pinned) | The css-skin tests pin 68×32/33×26/26×26 in button.css; pixel truth belongs to the visual lane (harness/playwright), not asserted here. |
| "localized (useSwitchLabels → i18n keys, j01-4)" (:39-40) | this file | REAL | useSwitchLabels.ts read; keys registered in messages.ts:2974-2975. |

## 7. Dead / duplicated / unreachable code
- None dead: skin strings all consumed by button.css rules; both public names (Switch/Toggle) have distinct consumer sets (frozen API by Lead ruling, documented — not flagged).
- Duplication is the design (two names, one core) and is test-enforced against re-divergence (form.css negative assertions).
- useSwitchLabels' lane-key fallback is now belt-and-braces (keys registered in the shared catalog with identical copy) — the comment itself predicts this ("lane keys win while unregistered, the shared catalog wins after"); harmless, noted for the file's owner.

## 8. Dependencies & flags
- SwitchCore + SwitchSkin (button family), useSwitchLabels → i18n messages + lane keys. No flags. No engine dependency (pure presentational). Consumers own the accessible name — verified they provide one.

## 9. Open questions for the next agent
- SettingWidgets.tsx:220-224 mounts this Switch with INVERTED polarity (`checked={value(key) !== true}` / `setValue(key, !checked)`) — whether that widget's label copy communicates the inversion ("Disable …") is that file's audit question; if the label is a plain positive phrasing, the a11y name lies about state.
- Toggle.tsx (button family twin) was not my assignment — its skin should be checked for the mirror of F-001.
