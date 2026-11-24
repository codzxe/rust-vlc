# AUDIT: ui/src/components/button/IconButton.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 58 / 1 (IconButton, forwardRef) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — used across ControlBar, dialogs, AppShell window controls, CustomizePanels close (ref used), settings |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; cross-check button.css spinner claim (comment w26.k01) |

## 1. One-paragraph verdict
Correct primitive: mandatory `label` → aria-label (icon-only contract), forwardRef (the CustomizePanels focus-recovery relies on it), loading = inert + aria-busy + disabled, type="button" default (no accidental form submits), rest-spread AFTER className so host overrides survive, `lumen-icon-host` class wiring the icon-state protocol. No findings.

## 2. What actually works (verified)
— verified by read: all branch logic; `disabled={disabled || loading}` keeps loading buttons inert (the h07 defect-2 focus story in CustomizePanels depends on Save NOT being an IconButton — consistent).

## 3. Findings
Zero findings. Checked: a11y contract, ref forwarding, prop spread order, defaults.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| IconButton | 33-57 | REAL | forwardRef + displayName |

## 5. Test honesty
Button lane tests (not in my list); UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Pending state — inert + aria-busy + the CSS spinner ... the promised hook is real now" | L15-17 | REAL-UNPROVEN | button.css spinner not read (other lane); component half verified |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
button.css, icon.css protocol. No env vars.

## 9. Open questions for the next agent
- None.
