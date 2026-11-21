# AUDIT: ui/src/screens/settings/areas/effects.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 44 / 3 rule groups (eq-preset trigger base + hover, staged full-strength overrides) |
| File verdict | UNREACHABLE (from a shipped binary — W14: tauri host cannot compile; reachable in the UI tree: /settings/audio effects area mounts these classes) |
| Reachable from shipped entry point? | NO (no GUI binary exists); UI-tree chain: areas/index → effects.tsx → `.effects-eq-preset` (effects.tsx renders the class once, verified) and `[data-settings-area="effects"]` with disabled Switch (effects.tsx:142-147) + disabled Dropdown (effects.tsx:213-222). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg "surface-panel:\|surface-hover:\|radius-control:" themes/generated/tokens.css` → all exist (:58,:60,:96 dark; light variants :134,:136); `rg "disabled\|<Dropdown\|<Switch" effects.tsx` → both :disabled rules have live consumers; `rg -c "effects-eq-preset" effects.tsx` → 1; frame-math check 316+2·2=320 / 32+2·2=36 under border-box (two-layer citation is internally consistent); read of the whole file. |

## 1. One-paragraph verdict
A small, disciplined sheet: every color flows through a semantic token (all four referenced tokens exist in the generated cascade), the only raw values are frame-measured structural px that each carry their frame citation as the header promises, both rule groups have live consumers in effects.tsx, and the staged full-strength override (`opacity: 1` on disabled staged controls) implements the disclosed honesty pattern (disabled WITH reason + markers, never dimmed). One no-op declaration: the hover rule re-states `border-color: var(--color-surface)`, identical to the base rule — dead weight, however self-documenting.

## 2. What actually works (verified)
— verified by `rg` in tokens.css: --color-surface-panel (:60), --color-surface-hover (:58), --radius-control (:96) all exist (plus light-mode variants); --color-surface is the sheet-wide default (present, used across all audited sheets).
— verified by `rg` in effects.tsx: the disabled Switch (:142-147) consumes `.lumen-switch:disabled`; a staged disabled Dropdown (:213-222) consumes `.lumen-dropdown__trigger:disabled`; the interactive preset Dropdown (:278) consumes `.effects-eq-preset`. Zero dead selectors.
— verified by arithmetic: outer 320×36 rx6 / inner 316×32 rx4 with a 2px ring ⇒ 316+4=320, 32+4=36 under border-box — the comment's two-layer geometry is consistent.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/effects.css:30-33
Code:
```
.effects-eq-preset .lumen-dropdown__trigger:hover {
  background: var(--color-surface-hover); /* house hover (a19 B14) */
  border-color: var(--color-surface);
}
```
What is wrong: `border-color: var(--color-surface)` re-states the base rule's value exactly (:26 `border: 2px solid var(--color-surface)`), so the declaration is a no-op — the hover changes only the background.
Why it matters: Noise in a sheet whose discipline is "every declaration carries its frame citation"; a reader hunts for the hover border change that does not exist.
What correct looks like: delete the line, or comment it as "ring unchanged on hover (frame)" if the intent is documentation.
Confidence: HIGH — Verification: both rules read; values compared.

## 4. Per-function table (rule groups)
| Rule group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .effects-eq-preset trigger base | 21-28 | REAL | — | tokens + cited px; two-layer math consistent |
| trigger hover | 30-33 | REAL | S4 | F-001 no-op border-color |
| staged full-strength (:disabled opacity 1) | 40-43 | REAL | — | both selectors consumed (Switch :147, Dropdown :222); implements the disclosed never-dimmed staging |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| effects.test.tsx (adjacent, exists per areas listing) | not re-audited here | owned by the effects.tsx audit (another agent); this sheet has no dedicated css assertions — theIconButton-style css-contract test pattern is not applied to settings sheets |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only; raw px below are frame-measured… each carries its frame citation" | :12-13 | REAL | every color is var(); every px has an inline frame note |
| "the local .effects-info ⓘ rules are GONE — all six ⓘ ride the shared SettingInfoDot grammar" | :6-10 | REAL | zero .effects-info rules in this sheet; SettingInfoDot imported by effects.tsx (:46-47) |
| "staged rows render FULL-STRENGTH while inert" | :36-38 | REAL | the opacity:1 overrides; consumers verified |

## 7. Dead / duplicated / unreachable code
F-001's border-color is the only dead declaration. No dead selectors.

## 8. Dependencies & flags
Tokens only. No imports. Unreachable from a shipped binary (W14 ceiling), as with all ui sheets.

## 9. Open questions for the next agent
- The effects area's staged pattern hinges on commands (audio.set-eq / set-spatializer / set-stereo-widener) that do not exist in crates (W8 + my CompressorTool F-001) — the effects.tsx audit should book that S2 if not already.
