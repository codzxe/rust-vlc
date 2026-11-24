# AUDIT: ui/src/components/form/Switch.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 46 / 1 (Switch forwardRef skin) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — SettingWidgets/SettingField + 5 settings areas (subtitles, controls, extensions, effects, staged) render switches in live settings rows |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers; verified switchCore.tsx exists; verified class grammar consumers (settings.css `.lumen-switch*` hooks, from that audit) |

## 1. One-paragraph verdict
A 4-line skin over the single shared SwitchCore (the j01-3 collapse of the
dual Toggle/Switch implementations), keeping the frozen `.lumen-switch*`
class grammar that settings CSS hooks. The public contract (role=switch,
aria-checked, keyboard via native button, disabled gating) lives in the core.
No findings.

## 2. What actually works (verified)
- One core, two skins: button/switchCore.tsx exists; Switch applies the
  form-family class set (16-21) that settings.css/effects.css/subtitles.css
  target (verified in the settings.css audit: `[data-staged] .lumen-switch:disabled`).
- 8+ production consumers (settings widgets + areas).
- forwardRef + displayName present.

## 3. Findings
None. Checked: API freeze (class names match the CSS hooks), dedup claim
(single core verified), a11y contract (documented, owned by core).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| switchSkin | 16-21 | REAL | frozen class grammar |
| Switch | 42-46 | REAL | thin forwardRef wrapper |

## 5. Test honesty
Switch.test.tsx exists (consumer scan). Core behavior tests live with
switchCore (j01-9 OFF-ink divergence fix referenced).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "collapses the dual implementations back to ONE core ... fixes land once" | Switch.tsx:28-33 | REAL | single switchCore.tsx; Toggle.tsx also skins it |

## 7. Dead / duplicated / unreachable code
None — this file IS the dedup residue (intentional frozen API).

## 8. Dependencies & flags
button/switchCore. None.

## 9. Open questions for the next agent
- None.
