# AUDIT: ui/src/components/form/__snapshots__/BrowseField.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 29 / 1 snapshot export (BrowseField DOM snapshot 1) |
| File verdict | REAL (honest golden, faithful to the component) — but the audit surfaced that the component it pins has ZERO shipped consumers (F-001) |
| Reachable from shipped entry point? | N/A (test golden). The pinned COMPONENT: NO — `rg "BrowseField" ui/src` (excluding its own files) hits only the form module's index/css/states.md and a SettingField.css comment; no screen renders it. Executed in baseline #23. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | read of the snapshot + BrowseField.tsx (69 lines: label→inputId htmlFor wiring :36, useId :30-31, optional onAction `onClick={() => onAction?.()}` :54) + BrowseField.test.tsx assertions (4 behavioral tests + snapshot); `rg "BrowseField" ui/src` → zero component consumers; `sed -n '305,345p' SettingField.tsx` → the SHIPPED path rows (skin file, log file) use SettingField's own inline TextField + disabled-Browse combo, not this component; `rg "browse" ui/src/i18n/messages.ts` → settings.field.browse msgid used by SettingField's button. |

## 1. One-paragraph verdict
The golden itself is unimpeachable: 29 lines pinning the label/for wiring, the text input, and the trailing "Browse" button — every attribute traces to BrowseField.tsx, and the test file carries four real behavioral tests (typing flows, onAction fires, disabled disables both, error aria wiring) so the snapshot is corroboration, not sole evidence. The problem is what the snapshot silently documents: a component nobody ships. No screen imports BrowseField — the actual settings path rows (Skin file, Log file) are rendered by SettingField.tsx:311-340 with its OWN inline TextField-plus-Browse combo (Browse disabled with an honest "no file-dialog command in the catalog" note). So this golden pins dead code that duplicates a live pattern, and its default render — a Browse button wired to nothing (`onAction?.()`, omitted in the golden's own render) — is exactly the unwired state the shipped variant deliberately avoids by being disabled-with-reason.

## 2. What actually works (verified)
— verified by attribute trace: `for="_r_6_"` ↔ `id="_r_6_"` (BrowseField.tsx:30-31,36,43 — label/input id wiring is real), classes `lumen-field`/`lumen-browse`/`lumen-browse__action` (:34,40,54), button `type="button"` (:54), caption from `actionLabel` (:55).
— verified by reading BrowseField.test.tsx: the golden is backed by explicit tests — typing → onChange (:17), click → onAction (:24), disabled → both inert + no fire (:30-34), error → aria-invalid + aria-describedby → error id (:40-42). Snapshot failure cannot silently erase behavioral coverage.
— executed: baseline #23 (vitest, 1705 passed) includes this comparison.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/form/BrowseField.tsx (surfaced by this snapshot; 0 consumers) + ui/src/screens/settings/SettingField.tsx:311-340
Code:
```
        <button
          type="button"
          className="settings-action settings-field__browse-action"
          disabled
          aria-describedby={browseNoteId}
```
What is wrong: BrowseField is dead in the shipped UI: `rg` finds no component consumer — only the form module's own exports, css comments, and states.md documentation. The path rows it was built for (frame: "Settings - interface.svg 'Skin file' / Settings - global.svg 'Log file'", BrowseField.tsx:22-23) are implemented by SettingField's own inline combo instead — a duplicate of the same input+button grammar. The shipped Browse button is honestly disabled (no file-dialog command in the catalog, SettingField.tsx:315-318); the unused component's default state is a live-looking button wired to an optional callback nobody passes anywhere.
Why it matters: Dead component + duplicated pattern: two implementations of the settings path-row grammar will drift (one disabled-with-reason, one onAction-optional), and the dead one is the one whose name promises the feature. Tests and a snapshot keep the corpse green (baseline #23 passes while nothing ships it) — the "1705 passed" count includes coverage of code with zero reachability.
What correct looks like: either SettingField's path row consumes BrowseField (single implementation, staged-disabled via a prop), or BrowseField is deleted with its tests/snapshot/css block.
Confidence: HIGH — Verification: consumer rg; SettingField path-row code read.
Severity: S3 (dead code + duplication per the scale's examples); booked here because this snapshot audit is what surfaced it — a BrowseField.tsx owner should inherit it.

### F-002 [S4]
Where: ui/src/components/form/__snapshots__/BrowseField.test.tsx.snap:9,18
Code:
```
    for="_r_6_"
```
What is wrong: Same useId leak as the SwatchGroup golden: React's internal id prefix is baked into the snapshot; a React upgrade changing useId formatting breaks the golden with zero product change, training `-u` muscle memory.
Why it matters: Golden noise on library upgrades.
What correct looks like: render the snapshot case with an explicit `id` (the prop exists, :31 `id ?? autoId`).
Confidence: HIGH — Verification: component id fallback read.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| snapshot export "DOM snapshot 1" | 3-29 | REAL | S4 | faithful DOM golden (label/for, input, action button); F-001 dead component; F-002 useId leak |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| DOM snapshot | YES (structure drift) | byte-compare of the serialized combo |
| wires the label + typing flows | YES | getByLabelText + onChange spy |
| action fires onAction + caption | YES | spy + name query |
| disabled disables both | YES | toBeDisabled ×2 + no fire |
| error aria wiring | YES | aria-invalid + describedby → error element id |
| gap | — | the suite proves the component works — nothing proves anything USES it (F-001); no reachability test exists for any form component |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "BrowseField (settings path combo, wave 26)" — describe title | test :3 | STALE | it is a settings path combo nothing in settings uses (F-001) |
| "frame: Settings - interface.svg 'Skin file' / Settings - global.svg 'Log file'" | BrowseField.tsx:22-23 | REAL (frame provenance) / FALSE (usage) | the frames carry those rows; the rows are implemented elsewhere |
| states.md documents BrowseField's aria-label mechanism | states.md:52-54 | REAL (mechanism) | component read — but documents an unconsumed component |

## 7. Dead / duplicated / unreachable code
F-001 is the finding: the audited golden exists to pin a component with zero shipped reachability. The golden itself is live (executed by the suite).

## 8. Dependencies & flags
Vitest Snapshot v1. Generated by the component under test; behavioral claims carried by explicit tests. Cross-ref: the shipped Browse (disabled, honest note) is the file-dialog absence my open-media.css audit also hit — no native picker exists anywhere in the UI.

## 9. Open questions for the next agent
- If a BrowseField.tsx file audit exists in a later wave, F-001 belongs to it; the dead-component census (form/index.ts exports vs consumers) is worth one sweep for the whole form module (TextField/NumberField/Switch consumers vs BrowseField's zero).
