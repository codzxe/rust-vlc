# AUDIT: ui/src/screens/settings/areas/playlist-services.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 44 / 3 `it` blocks in 2 describes |
| File verdict | REAL — executed, 3/3 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:167) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
An honesty test for an intentionally-empty settings area
("config-never-invents"): zero rows, no area-root claim (the library area
owns `[data-settings-area="library"]`), no invented service-discovery
options (no UPnP/podcast/SAP text), and a real routing check through the
actual SettingsScreen at /settings/library with a MockTransport. All
assertions are negative queries except the route rendering — which is the
correct shape for "this surface must render nothing".

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:167 (verbatim):
```
 ✓ src/screens/settings/areas/playlist-services.test.tsx (3 tests) 145ms
```
- Never-invent pin (18-21):
```
    expect(container.querySelector('[data-settings-area]')).toBeNull();
    expect(container.querySelector('[data-testid="settings-area-empty"]')).toBeNull();
    expect(container.querySelectorAll(".settings-field")).toHaveLength(0);
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
```
- Invention ban (24-27): `expect(screen.queryByText(/UPnP|podcast| SAP /i)).toBeNull();`
- Routing through the real screen (32-43): MemoryRouter /settings/:area → SettingsScreen → `[data-settings-area="library"]` present.

## 3. Findings
None. Zero-findings justification: the file's contract is absence — proven
with 4 negative DOM queries + a text-content ban; the routing proof mounts
the real SettingsScreen (not the area in isolation). Executed green.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| never-invent test | 13-22 | REAL | 4 negative queries |
| invention-ban test | 24-28 | REAL | content regex ban |
| routing test | 31-44 | REAL | real SettingsScreen mount |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 3 | YES | any invented control/text or lost route fails a query |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "schema-missing options render NOTHING (config-never-invents)" | comment :1-2 | REAL (tested) | 13-28 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vitest, react-router-dom, ClientProvider + MockTransport. No gates.

## 9. Open questions
None.
