# AUDIT: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 112 / 8 `it` blocks in 2 describes |
| File verdict | REAL — executed, 8/8 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:118) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
Real editor-flow test. Conflict detection is pinned on BOTH rows of a
duplicate combo (both `data-conflict` attributes AND two inline alerts),
clearing is verified when fixed (57-71); Apply persists through an
injected store and Revert returns to the PERSISTED override, not the
default (82-92); disabled-from-data bindings render inert (94-99). The
second describe mounts the editor against the REAL generated bindings
module (>100 unique ids, row count equality) — the fixture data and the
shipped keymap truth are both covered.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:118 (verbatim):
```
 ✓ src/screens/settings/areas/hotkeys/HotkeysEditor.test.tsx (8 tests) 249ms
```
- Conflict pin (57-70):
```
    fireEvent.change(stop, { target: { value: "Space" } });
    ...
    expect(ppRow?.dataset.conflict).toBe("true");
    expect(stopRow?.dataset.conflict).toBe("true");
    expect(screen.getAllByRole("alert")).toHaveLength(2); // inline warning on both rows
```
- Real-data coverage (103-110):
```
    const ids = new Set(HOTKEY_BINDINGS.map((b) => b.id));
    expect(ids.size).toBe(HOTKEY_BINDINGS.length);
    expect(HOTKEY_BINDINGS.length).toBeGreaterThan(100);
    render(<HotkeysEditor store={memoryHotkeyOverrides} />);
    const rows = document.querySelectorAll(".hotkeys-row");
    expect(rows.length).toBe(HOTKEY_BINDINGS.length);
```

## 3. Findings
None. Zero-findings justification: conflict both-sides + clear-on-fix,
persist/revert semantics against an injected store (not localStorage
guessing), inert-disabled assertions, and a real-generated-data smoke with
uniqueness + row-count equality. All assertions exact.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| FIXTURE + recordingStore | 10-27 | REAL | injected store seam (load/save) |
| render + command column | 30-46 | REAL | command/uiAction id shown, "—" when none |
| edit draft | 48-55 | REAL | value + data-changed + apply bar |
| conflict | 57-71 | REAL | both rows, clears on fix |
| reset | 73-80 | REAL | restores default S |
| apply/revert | 82-92 | REAL | persisted override survives revert |
| disabled | 94-99 | REAL | inert input |
| real generated data | 102-112 | REAL | >100 unique ids, exact row count |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 8 | YES | exact input values, dataset attributes, saved-map equality, row-count equality |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "conflict detection fires + clears, reset-to-default per binding, apply persists" | comment :7-8 | REAL (tested) | 57-92 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vitest, @testing-library/react, generated bindings module. No gates.

## 9. Open questions
None.
