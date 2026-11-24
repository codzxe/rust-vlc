# AUDIT: ui/src/components/list/Tabs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 124 / 5 (Tabs, TabsList, Tab, TabsPanel, useTabsContext) |
| File verdict | REAL with a latent API trap — the controlled `value` prop is accepted but can never change |
| Reachable from shipped entry point? | YES (ToolsDialog.tsx:107-133 — the audio/video tools tabs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg `<Tabs` consumers (exit 0 — single production mount, uncontrolled); read full file |

## 1. One-paragraph verdict
A correct WAI-ARIA tabs primitive (roving tabindex, arrow/Home/End with wrap, aria-controls/labelledby wiring, orientation-aware keys) whose only defect is a controlled-mode illusion: `Tabs` accepts a `value` prop (L23, L29) but `setValue` mutates ONLY internal state (L39) and there is no `onChange` callback — so under a controlled `value`, clicking a tab or arrowing does nothing visible and notifies nobody. The single production consumer (ToolsDialog) uses `defaultValue` (uncontrolled), so the trap is latent. Everything else verified clean by read.

## 2. What actually works (verified)
— verified by read + rg: ARIA anatomy complete (roles, aria-selected, tabIndex roving at L92, panel wiring L108-110); keyboard wrap math correct (L63-66) including disabled-tab filtering (L56); context guard throws with caller name (L119-123).

## 3. Findings
F-001 [S3]
Where: ui/src/components/list/Tabs.tsx:23, 37-42
Code:
```
  const [internalValue, setInternalValue] = useState(defaultValue ?? "");
  ...
  const ctx: TabsContextValue = {
    value: controlledValue ?? internalValue,
    setValue: setInternalValue,
```
What is wrong: controlled mode is a silent no-op. With `value` passed, `ctx.value` is pinned to the prop; every Tab click (L95) and arrow key (L70) calls `setValue` → `setInternalValue`, which cannot affect the rendered selection, and no `onChange` exists to tell the controller. A future controlled consumer gets a dead tab bar with no error. Why it matters: public-component API that accepts and ignores a prop is exactly the dead-control pattern this audit hunts, one refactor away from production. What correct looks like: either drop the `value` prop or implement the controlled protocol (value + onChange + internal fallback). Confidence: HIGH — Verification: read + rg (no controlled consumer today).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Tabs | 22-48 | REAL w/ trap | S3 | controlled prop ignored (F-001) |
| TabsList | 50-79 | REAL | — | arrow/Home/End, wrap, disabled-skip |
| Tab | 81-100 | REAL | — | roving tabindex, aria wiring |
| TabsPanel | 102-117 | REAL | — | hidden + tabIndex 0 |
| useTabsContext | 119-123 | REAL | — | named guard |

## 5. Test honesty
Tabs tests exist in the list lane (not my assignment); they cover the uncontrolled path (the only path with behavior). No test can cover controlled mode meaningfully because it has no semantics — the suite cannot catch F-001 by construction.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "doc 08 §4.5 anatomy: role=tablist/tab/tabpanel, roving tabindex ... Selection follows focus" (L17-20) | doc comment | REAL | verified L56-71, L92, L108-110 |

## 7. Dead / duplicated / unreachable code
The `value` prop is dead-in-effect (F-001). Nothing else.

## 8. Dependencies & flags
react only + list.css. No transport, no feature flags. Presentation primitive.

## 9. Open questions for the next agent
- If any screen needs URL-driven tabs (settings areas are routes today), fix F-001 first — the trap will bite exactly there.
