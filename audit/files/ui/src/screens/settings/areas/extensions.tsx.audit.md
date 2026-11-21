# AUDIT: ui/src/screens/settings/areas/extensions.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 243 / 3 (ExtensionRow, ExtensionsArea, useAreaMsg factory use) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (area registry → /settings/extensions route; test 42 pins routing; SettingsScreen sidebar System group). ENGINE: NONE CLAIMED — wasm-extensions = false (product.toml:166, Phase 7); no extensions command in catalog. SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg "wasm-extensions" product.toml:166 + generated/schema.ts:2474 (featureGate); rg "featureExposed" schema.ts; extensions.test.tsx survey 41-152; read staged.css pattern refs |

## 1. One-paragraph verdict
The model citizen of honest staging in this repo: the add-on list seam is an
explicitly EMPTY const, the toolbar actions are disabled-from-data (the
schema's featureExposed gate, wasm-extensions=false in product.toml), every
staged control carries data-staged + tooltip + sr-only describedby reason,
and the tests pin all of it (empty well, no fabricated rows, staged
full-strength actions, real local search). No dead buttons that look alive,
no invented data. The single finding is a latent one: the exported
ExtensionRow renders an ENABLED Switch wired to a no-op handler — safe today
only because the list is empty.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): extensions.test.tsx
executes routing (42), the generated-screen embed (51), the schema seed
truth (65), the F2 gate hiding the row (75), the empty well (83), no
fabricated rows (93), staged actions (98, 124), the local search filter
(113), and the row anatomy (152).
— verified by rg: product.toml:166 `wasm-extensions = false  # Phase 7` and
generated/schema.ts:2474 `featureGate: "wasm-extensions"` — the gate the
buttons read is real product config, not a hard-coded lie.
— verified by read: `visible` filter is a genuine local search over the
(empty) list data; the query state actually works.

## 3. Findings
### F-001 [S4] ExtensionRow ships an ENABLED Switch wired to a no-op — a fake-control landmine behind an empty seam
Where: ui/src/screens/settings/areas/extensions.tsx:157-165
Code:
```
          {visible.map((info) => (
            <ExtensionRow
              key={info.id}
              info={info}
              onToggle={() => {
                /* inert by design: the extensions store does not exist
                 * yet (Phase 7) — the list seam above owns the handoff. */
              }}
```
What is wrong: The row's Switch is rendered enabled (ExtensionRow:102 —
no disabled prop) with a no-op onToggle. Today zero rows render
(INSTALLED_EXTENSIONS = []), so no fake control exists on screen. The moment
anyone feeds the seam (the documented handoff), every row shows a working-
looking toggle that silently does nothing — the exact "dead control that
looks alive" this header forbids.
Why it matters: The honesty of this surface depends on a list staying empty;
the component API invites the regression.
What correct looks like: `onToggle` required and real, or the Switch
disabled with the staged reason until the store exists — the same
disabled-from-data rule the toolbar already follows.
Confidence: HIGH — Verification: read; ExtensionRow consumers = this file +
tests only (rg).
Cross-reference: the engine side of Phase 7 is a dead crate — W11-A02:
"plugin-host-wasm (the only real applier) is a dead crate with zero
dependents" — so even flipping the product gate would expose no real store.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| INSTALLED_EXTENSIONS seam | 74 | REAL | — | honestly empty const, documented handoff |
| useAreaMsg (makeAreaMsg) | 79 | REAL | — | shared factory (j13 dedup) — no local copy |
| ExtensionRow | 87-106 | REAL | S4 (F-001) | [VERIFY] anatomy flag in own doc; Switch enabled + no-op handler |
| ExtensionsArea | 122-243 | REAL | — | filter honest; actions disabled-from-data; sr-only reason twin (j13 P2-2) correct a11y pattern |

## 5. Test honesty (extensions.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| routed at /settings/extensions (42-49) | YES | registry routing pinned |
| embeds GENERATED_SCREENS.extensions (51-63) | YES | the seeded row's only render path |
| schema truth: exactly one gated row (65-73) | YES | guards against schema drift |
| F2 gate keeps row hidden + inert (75-81) | YES | feature-gate hiding pinned |
| empty well, no fabricated rows (83-97) | YES | the honesty core is test-enforced |
| staged actions full-strength + tooltip (98-123) | YES | asserts disabled + data-staged + title |
| search is a real local filter (113-123) | YES | types a query over (empty) data |
| row anatomy (152+) | YES | name/version/Switch render |
Mutation check: put one fabricated row into INSTALLED_EXTENSIONS → tests
93-97 fail ("no fabricated add-on rows"). Delete the gate read → test 75
fails. Strong, honest suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NO extensions command exists … wasm-extensions = false … the list seam is EMPTY" | header 18-25 | REAL | product.toml:166; catalog rg; code |
| "toolbar actions … render disabled-from-data … never dead controls that look alive" | header 26-30 | REAL (toolbar) / LATENT (row, F-001) | toolbar verified; row Switch is the landmine |
| KNOWN Phase-7 debt: duplicate section header when gate flips | comment 235-239 | REAL (documented debt) | read |

## 7. Dead / duplicated / unreachable code
- ExtensionRow + the whole list well rendering path is unreachable today
  (empty seam) — deliberately, honestly, test-pinned. Counted in F-001's
  context, not double-counted.

## 8. Dependencies & flags
components/form (Switch, TextField), list/Row, generated Screens/schema,
i18n lane keys, staged.css. The only "flag" is the product.toml
wasm-extensions feature gate read through the generated schema — a real
config-driven gate (verified against product.toml).

## 9. Open questions for the next agent
- When Phase 7 flips the gate, does the generated screen's own section
  header duplicate the hand one (the documented KNOWN debt at 235-239)?
  Someone must own that fold.
- Is plugin-host-wasm (dead crate, W11-A02) scheduled for deletion or
  revival? The UI seam's future depends on it.
