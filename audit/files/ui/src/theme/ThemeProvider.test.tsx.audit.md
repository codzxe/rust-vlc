# AUDIT: ui/src/theme/ThemeProvider.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A01) |
| Date | 2026-06-14 |
| Lines / functions | 256 / installMatchMedia + ModeProbe + ContextCapture + seedThemeOverride + seedAccentOverride + renderProbe + 2 describes (15 it blocks) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:93 `✓ src/theme/ThemeProvider.test.tsx (15 tests) 79ms`; subject ThemeProvider is mounted at the shipped UI entry ui/src/main.tsx:60 (`<ThemeProvider>` inside StrictMode createRoot render) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg -c "^\s*it\(" -> 15; sed -n quotes verified; rg ThemeProvider in ui/src (mounted: main.tsx, 4 screens); read persistence.ts + settingsThemeSync.ts + schemas/settings.toml + generated schema.ts to verify the seeding comment |

## 1. One-paragraph verdict
A genuinely hostile test suite for the theme bootstrap: 15 tests that pin the exact
contract of ThemeProvider — default dark/orange, attribute-only switching on
document.documentElement, ACCENTS guard throw, auto-follow of a *controlled*
matchMedia stub (including the flip event), first-render hydration of persisted
`interface.theme`/`global.accent-color` (the w26.f08 regression), saved-choice-wins
over OS, and total-fallback behavior for absent/unknown/corrupt localStorage stores.
All 15 pass in the orchestrator baseline. The only defects are honesty gaps at the
margin: two claims in the test header/provider comments are untestable in jsdom and
are not pinned by any assertion, and one test is a verbatim duplicate of the first.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:93 (`15 tests, 79ms`, in a lane whose totals are
1705 passed / 1 skipped per spec §13). Mutation reasoning (no code edited):
removing the bootstrap read (provider useState lazy-init ThemeProvider.tsx:69-77)
fails the 6 hydration tests; removing the matchMedia change listener
(ThemeProvider.tsx:81-93) fails both auto-flip tests; removing the setAccent guard
(:112-117) fails the toThrow test; corrupt-store tolerance is pinned by
`{not json` / `{corrupt` seeds (:211, :252).

## 3. Findings
F-001 [S4]
Where: ui/src/theme/ThemeProvider.test.tsx:1-7 (header claim) and ui/src/theme/ThemeProvider.tsx:97-106
Code: `// attributes (doc 08 §3.3: "no JS style recalculation beyond attribute set").`
 + provider `useLayoutEffect(() => { const root = document.documentElement; root.dataset.theme = resolvedMode; root.dataset.accent = accent; }, [resolvedMode, accent]);`
What is wrong: neither suite pins the two load-bearing claims it advertises. (a) The
"useLayoutEffect commits before paint so the first painted frame carries the mode"
claim is untestable in jsdom (no paint) — mutating `useLayoutEffect` → `useEffect`
passes all 15 tests; the flash-guard regression the comments describe is not
guarded by any test. (b) The doc 08 §3.3 "no JS style recalculation *beyond* the
attribute set" negative half has no negative assertion (e.g. style.background or
class mutations asserted absent after a switch).
Why it matters: the exact regression class the file says it exists to prevent
(flash / style recalculation) could return with the suite green.
What correct looks like: a jsdom-visible negative assertion (root inline style +
class list unchanged across a switch) and an honest UNVERIFIED note in the header
for the paint-timing claim.
Confidence: HIGH — Verification: read (mutation reasoning; jsdom has no paint pipeline).

F-002 [S4]
Where: ui/src/theme/ThemeProvider.test.tsx:221-225
Code: `it("still defaults to dark + orange with an empty store (the product defaults stand)", () => { renderProbe(); expect(root().dataset.theme).toBe("dark"); expect(root().dataset.accent).toBe("orange"); });`
What is wrong: verbatim duplicate of the first test (:91-96) — beforeEach already
clears localStorage for every test, so "empty store" is the default state of the
entire suite; the second describe re-proves the same 2 assertions minus the
resolved-mode probe.
Why it matters: zero marginal coverage; padding the count.
What correct looks like: delete or fold into describe 1.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: ui/src/theme/settingsThemeSync.ts:22-29 (consumed contract; not this file)
Code: `// the settings PICKER already renders the third cell (Light · Dark · System, f03 lane) writing "auto"; the schema enumeration (schemas/settings.toml `interface.theme`) is still dark/light only`
What is wrong: stale — schemas/settings.toml:76-80 NOW carries the comment "the k12
value-validation drop of a persisted `auto` was a regression — the schema now
carries it" with `enumeration = ["dark", "light", "auto"]`, and generated
schema.ts:96 enumerates dark/light/auto. The hydration tests for persisted "auto"
(tests :184-203) PASS *because* the enum grew; the seam comment still tells the
reader the engine write path rejects auto.
Why it matters: a reader of the seam would wrongly conclude test 9/10 cannot pass
honestly.
What correct looks like: refresh the comment (cross-file debt, logged here because
the test file's correctness story depends on it).
Confidence: HIGH — Verification: read (rg interface.theme in schemas/settings.toml:74-93 + generated schema.ts:86-104).

## 4. Per-function table
File is 256 LOC of test scaffolding; the meaningful units:
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| installMatchMedia | 16-34 | REAL | — | controllable stub; flip() drives the real listener path |
| ModeProbe | 36-54 | REAL | — | renders all 5 context values + 3 action buttons |
| ContextCapture | 59-63 | REAL | — | sidesteps React's async re-throw; toThrow still fails if capture is null (optional chain yields non-throwing fn) |
| seedThemeOverride / seedAccentOverride | 154-167 | REAL | — | seeding shape matches the real seam's save() exactly (persistence.ts:65-68 writes JSON.stringify(overrides) to "lumen.settings.overrides.v1") — the "exactly like the persistence seam writes it" comment is TRUE |
| describe §3.3 (6 tests) | 90-149 | REAL | — | all mutation-sensitive |
| describe hydration (9 tests) | 151-256 | REAL | S4 dup | :221-225 duplicate of :91-96 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| defaults to dark + orange (:91) | YES | dataset assertions fail if defaults or attribute effect removed |
| switching mode (:98) | YES | click→state→attribute chain fully asserted |
| switching accent (:107) | YES | same |
| rejects accent outside the 9 (:115) | YES | toThrow(/unknown accent "magenta"/) fails if guard removed; also pins state-not-updated |
| auto follows OS + reacts (:127) | YES | flip() only reaches the assertion through the real change-listener wiring |
| auto falls back without matchMedia (:141) | YES | typeof guard at provider :57 → dark; crash-on-missing-matchMedia would error the test |
| hydrate LIGHT first render (:169) | YES | fails if bootstrap read removed (would boot dark) |
| hydrate DARK over OS-light (:177) | YES | pins explicit mode ≠ auto-follow |
| hydrate SYSTEM + follow both ways (:184) | YES | valid because schema enum now includes auto (settings.toml:80) |
| hydrate SYSTEM vs dark OS (:197) | YES | same |
| absent/unknown/corrupt → dark (:205) | YES | three sub-cases: enum-drop of "solarized", JSON catch of "{not json", absent key |
| empty store defaults (:221) | YES | but duplicate of :91 (F-002) |
| hydrate persisted ACCENT (:227) | YES | fails if accent bootstrap removed |
| theme AND accent together (:236) | YES | independence pinned (theme stays dark) |
| unknown/corrupt accent → orange (:246) | YES | enum-drop + isAccentId belt-and-suspenders |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "must land on the root <html> element as data-theme/data-accent" | test header :1-3 | REAL | 15/15 baseline ui-test.txt:93; provider :102-106 does exactly this |
| "no JS style recalculation beyond attribute set" | test header :3 citing doc 08 §3.3 | REAL-UNPROVEN (negative half unpinned) | F-001 |
| "hydrate on the FIRST render" (w26.f08) | test header :4-7 | REAL | tests assert post-mount dataset without any user action |
| seeding "exactly like the settings screen's persistence seam writes it" | comment :152-153 | REAL | persistence.ts:65-68 save() shape + STORAGE_KEY :16 identical |
| "schema enumeration ... still dark/light only" (stale) | settingsThemeSync.ts:27-29 | FALSE comment (schema now has auto) | schemas/settings.toml:80; generated schema.ts:96 |

## 7. Dead / duplicated / unreachable code
F-002 duplicate test. ContextCapture (:59-63) is used by exactly one test — alive.

## 8. Dependencies & flags
Vitest + @testing-library/react (jsdom); imports only shipped modules (ThemeProvider,
useTheme, generated AccentId). No feature gate touches this lane.

## 9. Open questions for the next agent
- Can any e2e lane pin the layout-effect paint claim (Playwright visual baselines
  capture first-frame)? The visual lane exists (ui/e2e/visual) — worth one check.
- settingsThemeSync.ts:27-29 stale comment should be re-audited by the settings lane
  owner (cross-file, S4).
