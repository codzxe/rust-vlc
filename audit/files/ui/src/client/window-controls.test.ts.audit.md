# AUDIT: ui/src/client/window-controls.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 116 / 1 installInternals helper + 1 describe block (6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:140 `✓ src/client/window-controls.test.ts (6 tests) 6ms`; subject detectOsWindowBackend is the desktop shell's window seam (consumed by the app chrome in the Tauri build — desktop lane UNVERIFIED in sandbox per baseline cargo-build-desktop-tauri.txt) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n window-controls audit/runs/baseline/ui-test.txt` -> :140; read-through of all 6 tests |

## 1. One-paragraph verdict
Clean seam test: a scriptable `__TAURI_INTERNALS__` double (the injected shape — boundary double, not a subject mock) drives the real `detectOsWindowBackend` and asserts the exact Tauri command vocabulary (`plugin:window|minimize/toggle_maximize/close/is_maximized`) with the RUNTIME label from internals.metadata, the conventional-`main` fallback when metadata is absent, a second-window label driving that window, is_maximized true/false, the null probe outside the desktop shell, and the tauri://resize listen/unlisten lifecycle including the captured callback actually firing twice. afterEach removes the internals global — no leakage.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:140 (6/6 pass; lane 1705 passed :178-179). Mutation reasoning: a hardcoded label fails :90-95; a wrong command name fails :68-73 (toEqual of the whole invoke array); a swallowed unlisten fails :112-114; a false null detection fails :51-53.

## 3. Findings
None. Checked for: subject mocking (none — the double is the OS boundary), tautologies (none — whole-array equality on invokes), leakage (afterEach deletes the global :46-48), unwired tests (6/6 run per baseline). Zero findings is the honest result.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| installInternals | 9-44 | REAL | — | records invokes; is_maximized answer; transformCallback captures the resize handler |
| null probe | 51-53 | REAL | — | honest null outside desktop |
| command shapes suite | 55-95 | REAL | — | 4-command array equality; label from metadata / fallback / second-window |
| resize lifecycle | 97-115 | REAL | — | event name + eventId round-trip + double fire |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| command array :55-74 | YES | exact equality incl. order and payloads |
| second window :90-95 | REAL | label from metadata, not hardcoded |
| resize lifecycle :97-115 | YES | fired counter + unlisten command/eventId |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 6 tests run | baseline ui-test.txt:140 | REAL | `✓ ... (6 tests) 6ms` |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Same Fake-internals posture as tauri-transport.test.ts (referenced in header :3) — consistent boundary-double strategy.

## 9. Open questions for the next agent
None.
