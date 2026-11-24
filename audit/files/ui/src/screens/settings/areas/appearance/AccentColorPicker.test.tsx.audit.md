# AUDIT: ui/src/screens/settings/areas/appearance/AccentColorPicker.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 64 / 3 (1 describe) |
| File verdict | REAL with one dishonest test name (2 strong tests + 1 constants-only test mislabeled as wiring proof) |
| Reachable from shipped entry point? | YES — AccentColorPicker is the special widget for the global.accent-color row (specialWidgets.ts:34-37) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg baseline ui-test.txt:159; rg specialWidgets registration |

## 1. Verdict
The two real tests are strong: the 9-swatch frame order with orange default-selected, and
click-to-draft + live preview through the ThemeProvider accent seam (dataset.accent). The
third test's comment claims end-to-end wiring proof but its body asserts only schema
constants — it proves nothing about the SPECIAL_FIELD_WIDGETS registration it names.
Baseline green: ui-test.txt:159 `✓ …(3 tests) 125ms`.

## 2. What actually works (verified)
- Baseline 3/3 (cite above).
- Mutation reasoning: swatch order regression fails :31-32 (exact 9-element array); default
  selection fails :33; draft binding fails :51-53; live preview seam fails :54 (dataset.accent
  change — the real preview mechanism, same seam visual.spec.ts drives).
- The real registration exists outside this test: specialWidgets.ts:34-37 maps
  ACCENT_SETTING_KEYS (incl. global.accent-color) to the picker (rg verified).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/appearance/AccentColorPicker.test.tsx:57-63
Code:
  it("registers as the special widget for the accent row wherever the schema seeds it", () => {
    // The areas registry derives the special-widget key from the schema
    // (SPECIAL_FIELD_WIDGETS, areas/index.tsx) — assert the wiring end-to-end:
    // the entry's key is the registered key.
    expect(SETTING_GLOBAL_ACCENT_COLOR.key).toBe("global.accent-color");
    expect(SETTING_GLOBAL_ACCENT_COLOR.defaultValue).toBe("orange");
  });
What is wrong: the test name and comment claim it asserts the SPECIAL_FIELD_WIDGETS wiring
"end-to-end"; the body asserts two schema constants only. Nothing imports or reads the
registry; a total regression in specialWidgets.ts (picker never registered) passes this test
green. The claim about what this test proves is false.
Why it matters: a reader trusts the suite to have covered the registration seam; it has not.
What correct looks like: import SPECIAL_FIELD_WIDGETS (now exported from
../specialWidgets.ts:301 area index) and assert the entry's widget resolves to
AccentColorPicker.
Confidence: HIGH — Verification: read + rg (registration lives in specialWidgets.ts; no
reference from this file).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nine accents | YES | exact order + checked |
| click binding + preview | YES | draft value + dataset.accent |
| "registers as the special widget" | NO (not what it checks) | constants only — F-001 |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "wired through the existing SwatchGroup … with live preview through the ThemeProvider accent seam" | :9-13 | REAL | test 2 asserts the seam |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
memorySettingsPersistence — in-memory seam, appropriate; ThemeProvider real.

## 8. Open questions for the next agent
Cover the SPECIAL_FIELD_WIDGETS resolution as F-001 suggests.
