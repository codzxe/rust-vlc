# AUDIT: ui/src/screens/settings/areas/updates-about.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 103 / 6 it() tests, 2 describe blocks |
| File verdict | REAL (6/6 pass in baseline; version-truth + schema-truth pins) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 127 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest area test: the About panel's version comes from the build-injected
UI_PACKAGE_VERSION/UI_PACKAGE_NAME (asserted equal + semver-shaped — the component carries
no drift-able literal); the about→global fold redirect lands behind the Advanced
disclosure (asserted both collapsed-absent and revealed-present); the schema truth pins
global.update-checks as opt-in default-false via telemetry.update_checks; and the
config-never-invents rule asserts NO update button exists without a catalog command.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 127 `✓ src/screens/settings/areas/updates-about.test.tsx (6 tests) 288ms`.
- Read-verified: `expect(screen.getByTestId("about-version").textContent).toBe(UI_PACKAGE_VERSION);`
  + `expect(UI_PACKAGE_VERSION).toMatch(/^\d+\.\d+\.\d+/);`
- Read-verified no-invention: `expect(screen.queryByRole("button", { name: /check for updates/i })).toBeNull();`
- Exactly-one-row dedup pin: `querySelectorAll('[data-setting="global.update-checks"]')).toHaveLength(1)`.

## 3. Findings
F-001 [S4] the semver check `/^\d+\.\d+\.\d+/` is prefix-anchored only — "0.0.0-placeholder"
would pass. Cosmetic given the equality assertion against build-info. Confidence HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| renderRouted | 24-32 | REAL | routed mount |
| describe About panel | 34-71 | REAL | build-info equality; fold redirect; row grammar |
| describe update checks | 73-103 | REAL | schema-truth data; single-row dedup; no invented actions |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| version injected from the manifest | YES | toBe(build-info value) — hardcoded literal fails |
| about→global fold redirect | YES | absence-before/presence-after disclosure clicks |
| update row OFF by default, exactly once | YES | aria-checked + selector count |
| no update action without catalog command | YES | absence query |

## 6. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "REAL version data imported from the package manifest (never a hardcoded literal)" (header lines 1-2) | file itself | REAL | toBe(UI_PACKAGE_VERSION) |
| 6 tests pass | baseline ui-test.txt:127 | REAL | vitest line |

## 7-9. Dead code / deps / open questions
None / vitest + build-info + generated schema imports / none.
