# AUDIT: ui/src/screens/settings/areas/extensions.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 243 / 3 (ExtensionRow, ExtensionsArea, useAreaMsg const) + 2 module consts |
| File verdict | SHALLOW (honest shell over a fully unwired surface) |
| Reachable from shipped entry point? | YES — routed: SettingsScreen.tsx:81 group "system" -> areas ["…","extensions"] -> /settings/extensions (test at extensions.test.tsx:42 asserts the route) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "wasm-extensions" product.toml` -> :166 `false`; `sed -n '2466,2480p' generated/schema.ts` (featureExposed: false); `rg -n "list-plugins" ui/src/generated/commands.ts` + `crates/commands/src/catalog/command.rs:441-451`; test-file inspection |

## 1. One-paragraph verdict
The Extensions settings area is 100% backendless by design and says so: the
list seam is a hard-coded empty array, the three toolbar buttons
("Find more extensions"/"Add"/"Remove") render `disabled` from a verified
feature gate (product.toml:166 `wasm-extensions = false` -> schema
featureExposed: false), each carrying a staged tooltip + sr-only reason. As
honest degradation this is well executed — no fabricated rows, buttons never
look alive. But the file's justification comment is factually wrong: the
command catalog DOES contain plugin commands (`system.list-plugins`,
`system.plugin-enable`, `system.plugin-disable`), so "no extensions command
exists" is false, and the shipped Switch row is wired to an empty closure.

## 2. What actually works (verified)
— Gate chain verified end-to-end: product.toml:166 -> schema.ts:2472-2473
(featureGate "wasm-extensions", featureExposed: false) -> `actionsDisabled`
(L135) -> all three buttons `disabled` (L193/206/219). — Route verified:
SettingsScreen.tsx:81 + extensions.test.tsx:42-45. — The search TextField is
a working controlled input (query state L125-126) — real filter logic, over a
permanently empty array. — Tests assert routing, embed, gate, hidden row
(extensions.test.tsx:42-75): real assertions, would fail on regressions.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/extensions.tsx:19-21
Code:
```
//   • The add-on list is dynamic data. NO extensions command exists in the
//     command catalog and product.toml [features] wasm-extensions = false
//     (Phase 7, rule F2), so the list seam below is EMPTY — the well
```
What is wrong: the premise is false. The command catalog contains
`system.list-plugins`, `system.plugin-enable`, `system.plugin-disable`
(ui/src/generated/commands.ts:24; crates/commands/src/catalog/command.rs:441-451
"registry from manifests"). The UI never even ATTEMPTS `system.list-plugins`
(which, per the PlaylistProvider pattern, would honestly render
feature-not-exposed as a not-connected note if unimplemented). The dead surface
is justified by a non-existent gap.
Why it matters: the whole area is a dead settings surface on a false premise;
the honest-degradation doctrine is used to excuse not wiring commands that
exist in the contract.
What correct looks like: mount a list-plugins fetch on this area (or correct
the comment to "the plugin commands are catalog-only / not exposed by the
engine" — whichever the engine side proves).
Confidence: HIGH — Verification: read + rg (both catalogs show the commands).
Engine-side implementation status: UNVERIFIED (commands-crate domain).

F-002 [S3]
Where: ui/src/screens/settings/areas/extensions.tsx:161-164
Code:
```
              onToggle={() => {
                /* inert by design: the extensions store does not exist
                 * yet (Phase 7) — the list seam above owns the handoff. */
              }}
```
What is wrong: ExtensionRow renders a form Switch wired to a no-op closure —
a control that LOOKS alive and does nothing. Today unreachable (the list is
empty), but the file's own rule is "never dead controls that look alive"
(L30); the moment any data reaches this seam, users get a silent no-op toggle.
Why it matters: pre-planted dead control; contradicts the file's stated
discipline.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: ui/src/screens/settings/areas/extensions.tsx:31, 176-183
Code:
```
//   • The toolbar search is a real local filter over the list data.
```
What is wrong: "real local filter" over `INSTALLED_EXTENSIONS = []` (L74) —
typing in the search box can never change anything visible. Honest by comment,
functionally dead UI.
Confidence: HIGH — Verification: read (L74 + L127-134 filter over []).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| INSTALLED_EXTENSIONS (const) | 74 | STUB | — | hard-coded empty array, documented seam |
| useAreaMsg (const factory) | 79 | REAL | — | makeAreaMsg(registryKeys, laneKeys) — pre-registration pattern |
| ExtensionRow | 87-106 | REAL | S3 (F-002) | renders ListRow + Switch; onToggle is caller's problem |
| ExtensionsArea | 122-243 | REAL (shell) | S3 (F-001) | gate-verified disabled toolbar; empty well; embeds generated screen |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "is routed at /settings/extensions" | YES | route + registry assertions (L42-45) |
| "keeps the seeded row hidden + inert while wasm-extensions is off" | YES | asserts hidden state under the real gate (L75+) |
| "embeds GENERATED_SCREENS.extensions" | YES | embed presence (L51-62) |
Tests validate the HONEST SHELL, not any extension functionality — nothing to
test functionally; consistent with the surface being backendless. Not run here
(no node_modules; baseline vitest 1705 passed cited).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO extensions command exists in the command catalog" | extensions.tsx:19-20 | FALSE | commands.ts:24 + command.rs:441-451 list three plugin commands |
| "they render disabled-from-data ... never dead controls that look alive" | extensions.tsx:26-30 | SHALLOW | true for the toolbar; violated by the onToggle no-op (F-002) |
| "the b08 log never landed, agent died mid-wave" | extensions.tsx:24-25 | UNVERIFIED | docs/waves/w26/logs/w26.b08.md not checked (out of scope) |

## 7. Dead / duplicated / unreachable code
ExtensionRow is exported but rendered only from ExtensionsArea's empty map —
dead in production today (kept as the documented seam). Nothing else.

## 8. Dependencies & flags
form/Switch, form/TextField, list/Row, i18n key files w26.b02 + w26.b08,
generated Screens + schema, staged.css, extensions.css. Gate: product.toml
`wasm-extensions` (verified false). No transport import — this area is
backendless.

## 9. Open questions for the next agent
- Does the engine actually implement `system.list-plugins` (catalog entry
  command.rs:441 vs a real handler)? If yes, F-001 upgrades: the UI ignored a
  working backend command. If no, the comment should say "catalog-only".
