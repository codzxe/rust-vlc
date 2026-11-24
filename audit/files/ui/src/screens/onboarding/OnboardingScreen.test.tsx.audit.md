# AUDIT: ui/src/screens/onboarding/OnboardingScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 543 / 28 tests + 3 helpers (renderWizard, goToStep/goToPrivacy, renderProbe) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest lane `npm run test` ran it: baseline ui-test.txt:36 `✓ src/screens/onboarding/OnboardingScreen.test.tsx (28 tests) 692ms`. Subject wired at ui/src/App.tsx:11,185,296 (RequireOnboarding gate + /onboarding route) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg (App.tsx wiring, test count), sed -n '397,401p;244,256p' (quote verify), python3 batchmeta n/a here |

## 1. One-paragraph verdict
This is a real, dense behavioral suite: it drives the actual OnboardingScreen DOM (28 tests over
the 8-step flow), asserts real localStorage store contents, real settings-override writes, real
transport commands, keyboard/roving-tabindex semantics and router redirects. All 28 tests executed
and passed in the orchestrator baseline (ui-test.txt:36; run summary 1705 passed / 1 skipped,
ui-test.txt tail). Test count on disk matches the baseline exactly (28 `it(` blocks). The only
defects are hygiene-level: one test's name promises proof it cannot deliver, and heavy render
boilerplate duplication.

## 2. What actually works (verified)
— verified by rg -c "it(" (28 = baseline's 28 tests) and by citation of the executed run
audit/runs/baseline/ui-test.txt:36 -> `✓ src/screens/onboarding/OnboardingScreen.test.tsx
(28 tests) 692ms`. All 28 passed in that run (final tally line: `Tests 1705 passed | 1 skipped`).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/onboarding/OnboardingScreen.test.tsx:397-401
Code:
    it("subtitle-languages placeholder flows through the catalog (no raw literal, j16 P3-13)", () => {
      renderWizard();
      goToStep(4);
      expect(screen.getByPlaceholderText("en, de")).toBeInTheDocument();
    });
What is wrong: the name claims the placeholder is catalog-sourced ("no raw literal") but the body
only asserts a rendered placeholder string; a raw literal `placeholder="en, de"` in the component
passes identically. Why it matters: the claim is unproven by this test — it records an intent, not
a guarantee. What correct looks like: assert the placeholder equals the value exported from the
catalog module (import and compare), or drop the parenthetical from the name. Test still fails if
the placeholder disappears — name-vs-body inflation only.
Confidence: HIGH — Verification: read (mutation reasoning; would pass either way).

F-002 [S4]
Where: ui/src/screens/onboarding/OnboardingScreen.test.tsx:244-255, 278-289, 312-323
Code: (three verbatim-identical 12-line blocks)
    it("appearance step live-previews the theme through ThemeProvider", () => {
      render(
        <ThemeProvider>
          <ClientProvider transport={new MockTransport()}>
            <MemoryRouter initialEntries={["/onboarding"]}>
              <Routes>
What is wrong: the ThemeProvider+ClientProvider+MemoryRouter render scaffold is copy-pasted three
times instead of a `renderThemedWizard()` helper next to `renderWizard()` (line 39).
Why it matters: pure duplication drift risk (one block gains a tweak, others silently don't).
What correct looks like: one helper parameterized by the tree.
Confidence: HIGH — Verification: read (sed -n verified).
No S0-S3 findings: no tautologies found; every test asserts observable DOM/store/transport state
that the component under test produces (e.g. lines 74-85 assert `aria-checked="false"` + empty
store; 204-232 assert `transport.sent[0]` matches `session.open` args and the honest error notice).

## 4. Per-function table (tests are the functions)
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| renders welcome + progress (66) | REAL | — | aria-label + aria-current step |
| fresh run privacy all OFF (74) | REAL | — | asserts store untouched (toEqual({})) |
| honest off-by-default note (87) | REAL | — | copy assertion |
| skip completes flags false (95) | REAL | — | full store object equality |
| skip never flips flag (104) | REAL | — | partial opt-in preserved |
| skip navigates to player (116) | REAL | — | router probe |
| explicit opt-ins persist (124) | REAL | — | only touched flags true |
| opt-in re-hydration (137) | REAL | — | resetOnboardingForTests + probe |
| corrupt store folds to defaults (150) | REAL | — | never "on" — good hostile case |
| Back/Next walk steps (160) | REAL | — | live-region announcer asserted |
| Back hidden on welcome (176) | REAL | — | class check (geometry kept) |
| Next→done, Skip disappears (181) | REAL | — | complete=true |
| library field + Browse submit (192) | REAL | — | accessible name |
| open-path through session.open (204) | REAL | — | transport.sent match + typed error notice |
| empty path rejected (234) | REAL | — | field copy, no notice |
| appearance live preview (244) | REAL | — | dataset.theme/accent |
| arrow keys roam radiogroup (278) | REAL | — | roving tabindex + wrap |
| appearance picks persist (312) | REAL | — | settings overrides store |
| fresh load hydrates picks (339) | REAL | — | ThemeProvider bootstrap |
| playback step writes store (357) | REAL | — | "playback.start-paused" |
| subtitle-mode segmented (364) | REAL | — | "playback.subtitle-mode" |
| subtitle-languages typing (373) | REAL | — | regression-proof of w26.j16 P1 |
| placeholder "catalog" (397) | SHALLOW | S4 | F-001 |
| default-rate clamp (405) | REAL | — | 9→4 clamp + empty-commit guard |
| skip leaves settings untouched (422) | REAL | — | covers prior gap |
| shortcuts keymap tour (434) | REAL | — | keymap data through DOM |
| gate redirects incomplete (457) | REAL | — | RequireOnboarding |
| gate after completion (478) | REAL | — | + bounce-back |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all except F-001 case | YES | DOM/store/transport assertions tied to component output, not mocks-of-self |
| placeholder-catalog (397) | only if placeholder vanishes | cannot distinguish catalog vs literal (F-001) |
Mocking posture: MockTransport mocks the transport seam (client command layer), not the screen —
correct seam for UI-logic tests; the backend truth of session.open is out of this suite's scope and
is covered elsewhere (rc-smoke engine lanes, baseline/rc-smoke.txt).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "8-step flow (welcome→…→done)" | test header comment lines 18-22 | REAL | progress labels "1 of 8"… "You are set" asserted (70,164,184) |
| "only explicit opt-ins persist" | header line 21-22 | REAL | lines 95-135 assert exactly that |

## 7. Dead / duplicated / unreachable code
None dead. Duplication documented in F-002. Helpers all used.

## 8. Dependencies & flags
Imports only in-repo modules + @testing-library/react + react-router-dom + vitest; no new deps; no
feature gates. Subject reachable in shipped app via App.tsx:296.

## 9. Open questions for the next agent
Does any E2E lane exercise the completed-wizard → player handoff with a REAL transport (not
MockTransport)? Baseline rc-smoke covers engine lanes only — the UI-side handoff is unit-proven
only.
