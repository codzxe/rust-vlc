# AUDIT: ui/src/components/list/Tabs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 123 / 5 (Tabs, TabsList, Tab, TabsPanel, useTabsContext) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: PlayerScreen `<ToolsDialogHost />` → ToolsDialog.tsx:107 `<Tabs defaultValue={defaultTab}>` — the Audio/Video tool tab pair; opened via `openToolsDialog` event from ControlBar's app menu / tools buttons. Sole production consumer) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file + List.test.tsx:28-115 (the Tabs suite) + ToolsDialog.tsx:99-169 (the consumer) · `rg -n "<Tabs\b" ui/src` (production usage: ToolsDialog:107 only; both test usages uncontrolled) · `rg -n "TabsList\|TabsPanel" ui/src` (consumer closure) |

## 1. One-paragraph verdict
A correct uncontrolled ARIA tabs primitive: the roving-tabindex, arrow/Home/End navigation, wrapping, selection-follows-focus, aria-selected/controls/labelledby wiring, and panel hiding are all implemented per the WAI-ARIA pattern and test-pinned (7 focused tests, all asserting real DOM behavior). The hostile finding is the API surface the file advertises but cannot deliver: the `value?: string` prop creates a CONTROLLED mode with no `onChange` (or any notification) — in controlled mode, clicks and arrow keys call `setValue`, which writes `internalValue`, which `controlledValue ?? internalValue` then masks, so the selection never moves and the parent is never told; the component's entire interaction is silently dead in the mode its own props invite. Every current consumer (one production + the tests) is uncontrolled, so the trap is latent. Second, smaller: omitting `defaultValue` yields an empty selection, and with no tab selected every tab is `tabIndex={-1}` — the tab list has no keyboard stop at all until something is clicked with a mouse.

## 2. What actually works (verified)
— ARIA anatomy, test-pinned: tablist/tab/tabpanel roles, aria-orientation, aria-selected, aria-controls ↔ panel id bidirectional wiring (List.test.tsx:44-53 asserts the id pair matches).
— Roving tabindex: only the selected tab is tabbable (:58 asserts [-1, 0, -1]).
— Arrow navigation with wrap, per orientation; Home/End; focus moves with selection (:68-88, :90-107 — selection AND `document.activeElement` asserted).
— Click selection with panel swap (:109-114).
— Context misuse fails loudly (`useTabsContext` throws with the caller name, :119-123).
— The keyboard handler gathers `:not([disabled])` tabs from the DOM (robust to conditional children) and guards the empty list (:57).

## 3. Findings

### F-001 [S3]
Where: ui/src/components/list/Tabs.tsx:29, 35-42, 95
Code:
```
  value?: string;
```
```
  const [internalValue, setInternalValue] = useState(defaultValue ?? "");
  const baseId = useId();
  const ctx: TabsContextValue = {
    value: controlledValue ?? internalValue,
    setValue: setInternalValue,
    orientation,
    baseId,
  };
```
What is wrong: the public API advertises a controlled mode (`value`), but there is NO change callback. `Tab.onClick` (:95) and the keyboard handler (:70) call `ctx.setValue` → `setInternalValue`; in controlled mode the rendered selection is `controlledValue ?? internalValue`, so the internal write is masked and the parent has no way to learn the user acted (no onChange, no event). Result: `<Tabs value={x}>` renders a tab strip whose clicks and arrow keys do nothing — the component's core interaction is dead in the mode its own prop signature invites.
Why it matters: this is a trap for the next consumer, not a live bug: the only production usage is uncontrolled (ToolsDialog.tsx:107 `defaultValue`), and both test usages pass `defaultValue`. But the defect is silent — no error, no warning, just dead tabs — the worst failure shape for a reusable primitive. The 7-test suite never exercises `value`, so CI would stay green while the controlled mode is broken.
What correct looks like: either delete the `value` prop (uncontrolled-only component, like the rest of this codebase's primitives) or add `onChange?: (value: string) => void` and call it from setValue with the controlled-source-of-truth rules.
Confidence: HIGH — Verification: read; masked-write path traced line by line; consumer + test grep confirms zero controlled usages.

### F-002 [S4]
Where: ui/src/components/list/Tabs.tsx:35, 92
Code:
```
  const [internalValue, setInternalValue] = useState(defaultValue ?? "");
```
```
      tabIndex={selected ? 0 : -1}
```
What is wrong: with no `defaultValue`, the initial value is `""`, no tab matches, so EVERY tab renders `tabIndex={-1}` and none is focusable — the tab list is unreachable by keyboard (Tab key skips it entirely) until a pointer user clicks a tab. The ARIA Tabs Pattern's default is that the FIRST tab is selected.
Why it matters: keyboard-only users get a dead region from the no-default mount. Latent today: ToolsDialog always passes `defaultTab` (defaulted to "audio", ToolsDialog.tsx:102) and both tests pass `defaultValue`.
What correct looks like: `useState(defaultValue ?? firstTabValue)` is not knowable at Tabs level (children opaque) — so either treat empty-string selection as "index 0" when rendering tabIndex (derive: if no tab is selected, the first `Tab` gets tabIndex 0), or document/make `defaultValue` required.
Confidence: HIGH — Verification: read; tabIndex logic traced for the no-match case; consumers verified to always pass a default.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Tabs | 22-48 | REAL | S3/S4 | uncontrolled path correct; F-001 severed controlled mode; F-002 empty-selection keyboard trap; ctx object recreated per render (harmless — provider re-renders) |
| TabsList | 50-79 | REAL | — | DOM-order roving nav, wrap, Home/End; disabled tabs excluded; empty-list guard; `values[nextIndex]!` safe behind the length check |
| Tab | 81-100 | REAL | — | id/aria-controls wiring; roving tabIndex; click → setValue |
| TabsPanel | 102-117 | REAL | — | hidden attr swap; aria-labelledby back-wiring; tabIndex 0 |
| useTabsContext | 119-123 | REAL | — | loud misuse error with caller name |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| List.test.tsx:44 aria wiring | YES | role + id-pair assertions |
| :55 roving tabindex | YES | tabIndex array assertion |
| :61 only the selected panel visible | YES | hidden-attr assertions |
| :68 arrows move selection + focus, wrapping | YES | selected role AND activeElement asserted both directions |
| :81 Home/End | YES | end jumps asserted |
| :90 vertical orientation uses Down/Up | YES | orientation attribute + both keys |
| :109 click selects | YES | selection + panel swap |
All green: baseline ui-vitest.log.txt (1705 passed). GAP: no test for controlled `value` (F-001) or the no-defaultValue mount (F-002) — both would fail today if written.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "doc 08 §4.5 anatomy: tablist/tab/tabpanel, roving tabindex, arrow-key navigation per orientation, aria-selected + aria-controls/aria-labelledby wiring. Selection follows focus … Home/End" | header :16-21 | REAL | implemented :53-72, :83-99, :104-116; all 7 tests green |
| (implied) controlled `value` prop works | prop signature :29 | FALSE | F-001 — no notification path; selection frozen |

## 7. Dead / duplicated / unreachable code
- The `value` prop is de-facto dead API surface (no consumer, broken semantics) — F-001.
- No other dead code.

## 8. Dependencies & flags
- React (createContext/useContext/useId/useState), list.css. No engine dependency (pure presentation), no feature flags. Consumer: ToolsDialog only.

## 9. Open questions for the next agent
- If any settings screen plans a controlled Tabs (URL-synced tabs), F-001 blocks it — fix or remove the prop first.
- list.css (the shared sheet) and the remaining List primitives were not in my file set.
