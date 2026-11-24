# AUDIT: ui/src/components/button/__snapshots__/IconButton.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A04) |
| Date | 2026-11-24 |
| Lines / functions | 19 / 1 snapshot entry (DOM snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test artifact for IconButton — production-mounted broadly, e.g. toolbarItems.tsx:291, ColorsTool; snapshot consumed by IconButton.test.tsx:82-86) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (snapshot-producing test, Glyph fixture, lumen-icon-host wiring, consumers, baseline) -> all cited; sed -n '3,8p' snap -> quote verified |

## 1. One-paragraph verdict
One clean snapshot pinning IconButton's default DOM contract: `aria-label` from the `label` prop, the dual class grammar (`lumen-icon-button` + `lumen-icon-host` — the latter is the icon.css hover/active tint protocol hook, IconButton.tsx:44), native `type="button"`, and verbatim child passthrough (the fixture Glyph svg serialized with zero wrapper). It matches the live component (baseline 12/12 green) and has no orphan entries.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:117 `✓ src/components/button/IconButton.test.tsx (12 tests) 132ms` — snapshot matched live output.
— verified by rg: `lumen-icon-host` is a real IconButton behavior (IconButton.tsx:44) with real CSS consumers (Icon.test.tsx:125-129 pins the tint protocol against it) — the snapshot pins a load-bearing class, not decoration.

## 3. Findings
None. Checked: orphan entries (0 — one entry, one toMatchSnapshot call at IconButton.test.tsx:86), stale drift (baseline green), pinned-contract value (aria-label wiring, class grammar, child passthrough, type=button all pinned), fixture leakage (the pinned svg IS the test's own Glyph fixture (IconButton.test.tsx:15-19) — intentional: proves children render untouched, no cloning/wrapping), useId-style brittleness (none — no generated ids in this snapshot, unlike TextField's).

## 4. What the snapshot pins (mutation reasoning)
| Component change | Snapshot fails? | Why |
|---|---|---|
| aria-label not wired from `label` prop | YES | `aria-label="Snapshot"` (line 5) |
| `lumen-icon-host` dropped (breaks icon tint protocol) | YES | pinned at line 6 |
| children wrapped/cloned (extra span, injected attrs) | YES | fixture svg serialized byte-identical (lines 9-17 vs test :15-19) |
| type changed/removed | YES | line 7 |
| disabled/aria-busy/loading/focus styling | NO | attribute-state and CSS concerns — covered by the other 11 tests (e.g. :70-79 busy+disabled) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| IconButton.test.tsx:80-87 (produces this snap) | YES | full-DOM diff of the real render |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Carries `lumen-icon-host` so nested `.lumen-icon` [glyphs get state tints]" | IconButton.tsx:29 | REAL | pinned class + Icon.test.tsx:125-129 CSS protocol tests |

## 7. Dead / duplicated / unreachable code
None — single live entry.

## 8. Dependencies & flags
None. Vitest snapshot v1 format; no env/config keys.

## 9. Open questions for the next agent
None.
