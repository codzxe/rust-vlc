# AUDIT: ui/src/components/surface/Surface.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 530 / 28 tests across 10 describes + 3 harness components |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped UI component (ui/src/components/surface/Surface.tsx); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "Surface.test.tsx" audit/runs/baseline/ui-test.txt` → `37: ✓ src/components/surface/Surface.test.tsx (33 tests) 371ms`; full file read (530 lines) |

## 1. One-paragraph verdict
A genuinely rigorous component test: focus-trap semantics with Tab/Shift+Tab wrap cycling, focus restore to the opener on close, Escape-ownership between popups/in-flow listboxes/out-of-tree portals, IME composition guard, hidden-subtree skipping, natively-focusable contenteditable/summary cycling, dev-warning assertion with spy restore, plus the full presentational surface set (Sheet/Panel/Card/Well/Separator/SectionHeader/HeroPill tones and variants). No mocks of the subject, no tautologies, no skipped tests; the only approximation (a second RTL render standing in for a portal popup) is documented in-test as the honest shape (L234-236).

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L37 `✓ ... Surface.test.tsx (33 tests) 371ms` (part of 1705 passed / 1 skipped; the 1 skip is dialogs/harness.test.tsx L156, not this file).
- Read of full file: every `expect` binds to rendered DOM roles/attributes/classes or spy call arguments produced by the real Dialog/Sheet/Panel/... components; harnesses (L7-19, L134-176, L274-288, L358-368) reproduce real host usage patterns (state-driven open/close) instead of mocking the component.
- Not re-run here (no npm per load discipline); baseline citation above is the execution evidence.

## 3. Findings
None. Zero-findings checklist (what was checked, how):
- Tautology scan: every assertion consumes output of the component under test (roles, attributes, className, activeElement, onClose payloads). No assertion compares a value to itself or to an input constant of the harness.
- Mock-of-subject scan: `rg -n "vi.mock|jest.mock" ui/src/components/surface/Surface.test.tsx` → none; only vi.spyOn(console, "warn") with mockRestore (L331-341, restored).
- Skipped/ignored scan: no `it.skip`/`describe.skip`/`.todo` in file; baseline shows 33 passed 0 failed for this file.
- Name-vs-body scan: each it() name matches what its assertions prove (e.g. "traps Shift+Tab: wraps from first back to last" L86-92 asserts exactly the last-first wrap).
- Approximation honesty: portal simulated by second render (L236-240) — in-test comment L234-236 states it ("A second RTL render mounts OUTSIDE the dialog's DOM tree — the honest shape of a portal/fixed popup").
- Mutation reasoning: breaking the trap wrap breaks L78-91; breaking focus restore breaks L62-66; breaking the IME guard breaks L257-269; breaking hidden-skip breaks L291-295.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Dialog focus trap | 26-124 | REAL | — | role/aria-modal, labelledBy, closed renders nothing, focus in/out, Tab wrap both directions, Escape reason, scrim gated by closeOnOutsideClick |
| Escape ownership (popups vs in-flow vs portal) | 126-270 | REAL | — | popup-yield, in-flow no-wedge, out-of-tree portal yield, IME composition ignore |
| Trap edge cases | 272-343 | REAL | — | [hidden] skip, zero-focusable surface holds focus, native focusables, dev warn for missing label |
| Sheet | 345-388 | REAL | — | dialog semantics + side class + default right |
| Dialog/Sheet tones | 390-419 | REAL | — | panel default, deep tone data attr |
| Panel | 421-445 | REAL | — | section default, `as` semantics, edges/tone attrs |
| Card | 447-469 | REAL | — | flat default, three variants, `as="li"` |
| Well / Separator / SectionHeader / HeroPill | 471-529 | REAL | — | smoke-level but honest (each asserts concrete rendered output) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 28 (groups above) | YES | assertions bind to real rendered output; verified green in baseline L37 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness reproduces both DOM shapes of the real app inside ONE dialog" | comment L126-133 | REAL | PickerHarness L134-176 renders both shapes as described |
| "jsdom: click alone does not focus (browsers do)" | comment L59 | REAL-UNPROVEN | standard jsdom behavior claim; consistent with explicit .focus() calls |

## 7. Dead / duplicated / unreachable code
None. Harnesses are minimal and each used by the tests that need them.

## 8. Dependencies & flags
@testing-library/react, vitest, React useState; subject: ./Surface (real component). No feature flags.

## 9. Open questions for the next agent
- None.
