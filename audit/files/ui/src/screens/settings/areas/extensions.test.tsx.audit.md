# AUDIT: ui/src/screens/settings/areas/extensions.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 167 / 11 tests across 4 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped settings Extensions area; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "extensions.test" audit/runs/baseline/ui-test.txt` → `97: ✓ src/screens/settings/areas/extensions.test.tsx (11 tests) 231ms`; full file read |

## 1. One-paragraph verdict
Honest tests of a STAGED surface: the empty well is asserted to carry NO invented copy ("No add-ons installed" absent, L89-90), toolbar buttons are staged-disabled with the real tooltip text and data-staged marker, the schema truth (exactly one seeded row, gated wasm-extensions) is asserted against the generated schema itself, CSS pins are read from the real staged.css/extensions.css with regex grammar, and ExtensionRow behavior is tested with a fixture through the real switch role. The one defect: the search-filter test's NAME promises filter behavior its body cannot prove (the list is empty by design).

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L97 `✓ ... (11 tests) 231ms` (inside 1705 passed).
- Absence-of-fabrication pins: L89-90 (no invented empty-state), L93-96 (no fabricated rows), L109-110 (no invented gate note).
- Real schema assertion: L65-71 `expect(keys).toEqual(["extensions.load-on-startup"])` + featureGate === "wasm-extensions" for every entry.
- CSS pins: L127-140 regexes over the real stylesheets (opacity:1 for staged disabled actions, theme-token placeholder/well rules).
- ExtensionRow: switch role + aria-checked + onToggle(false) on click (L152-165).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/extensions.test.tsx:113-122
Code:
```
  it("toolbar: the search input is a real local filter over the list", () => {
    mountArea();
    const search = screen.getByPlaceholderText("Search");
    expect(search).toHaveValue("");
    fireEvent.change(search, { target: { value: "demo" } });
    expect(search).toHaveValue("demo");
    // The (empty) list filters without crashing — still the empty well.
    expect(screen.getByTestId("extensions-list")).toBeInTheDocument();
```
What is wrong: the test NAME claims "a real local filter over the list", but the body only proves the input holds a value and the empty list survives the change event — with an always-empty list the filtering logic is never exercised (nothing can be filtered, and no row is present to be hidden/kept). The in-test comment admits it ("The (empty) list filters without crashing"). Why it matters: name-vs-body inflation; the next reader believes the filter behavior is covered when zero filter logic is. What correct looks like: render ExtensionRow fixtures (as the L144-150 describe already does) under the area with a populated list seam, then assert filtering hides non-matching rows. Confidence: HIGH — Verification: read.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| routing + sibling contract | 41-72 | REAL | — | route, embedded generated screen, schema truth |
| F2 feature gate | 74-80 | REAL | — | hidden + inert while gate off |
| hand Add-ons surface | 82-142 | REAL | S4 | honest-empty pins; search test name overclaims (F-001); CSS pins real |
| ExtensionRow anatomy | 144-166 | REAL | — | switch semantics + toggle payload |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 10 of 11 | YES | route/gate/schema/CSS/row assertions bind to real output |
| "search input is a real local filter" | NO (for filter logic) | empty list ⇒ no filter path exercised (F-001) |
Verified green baseline L97.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the frame's well is empty — the invented empty-state copy is gone" | comment L87-88 | REAL | L89-90 absence assertions |
| "The staged grammar is CSS-carried (jsdom will not compute it) — pin the sheet" | comment L124-126 | REAL | L127-140 regex pins over real CSS |

## 7. Dead / duplicated / unreachable code
None (EXTENSIONS_ENTRIES, helpers all used).

## 8. Dependencies & flags
@testing-library/react, vitest, react-router-dom MemoryRouter, ClientProvider/MockTransport (backend seam only), real generated schema + Screens, node:fs CSS reads.

## 9. Open questions for the next agent
- Is the list data seam intended to be populated anywhere (an engine add-on event)? If the seam is permanently empty, the F-001 filter path is dead code in the product, not just the test.
