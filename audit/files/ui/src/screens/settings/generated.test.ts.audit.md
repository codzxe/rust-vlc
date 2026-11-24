# AUDIT: ui/src/screens/settings/generated.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 266 / 11 |
| File verdict | REAL — the strongest kind of test in the tree: it executes the real generator in --check mode and proves drift detection on mutated copies |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:64 `✓ src/screens/settings/generated.test.ts (11 tests) 277ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | ls ui/scripts/gen-settings-screens.mjs (exists); rg baseline |

## 1. One-paragraph verdict
This is a genuine drift gate with a positive control: the committed
generated screens must pass `gen-settings-screens.mjs --check` (exit 0), and
four MUTATED schema copies must FAIL it (default change, restart-flag change,
entry removal, and the msgid-collision guard — with the failure MODE asserted
by stderr content, "collide on msgid", not just exit 1). The mutation helper
guards itself: `expect(mutated).not.toBe(original)` (L94) turns a stale
regex into a loud failure, not a silently-green no-op. The committed snapshot
is additionally sanity-checked: 10 areas, 102 entries pinned, unique keys,
range/enum-default rules, default_ref resolution through product.toml,
feature-gate exposure, restart flags, and full i18n coverage including a
catalog-vs-lane-file byte-equality pin (the j14 #8 three-copy divergence fix).

## 2. What actually works (verified)
— Baseline: ui-test.txt:64 — 11 tests, 277 ms, green.
— The generator script exists: `ls ui/scripts/gen-settings-screens.mjs` →
present (the subprocess target is real, not a phantom).
— Mutation reasoning: each drift test FAILS the suite if the generator's
--check stops detecting that mutation class — this is a test of the TEST
INFRA, the rare meta-test that would catch a neutered drift gate.
— The i18n coverage test (L209-265) asserts a msgid for every entry
description, every enum option, every section title, with named failure
messages; the lane-file equality loop (L236-244) pins catalog==lane copy.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/generated.test.ts:119
Code:
      s.replace(/\[\[setting\]\]\nkey = "playback\.start-paused"[\s\S]*?since = "0\.1\.0"\n/, ""),
What is wrong: the removal mutation assumes `since` is the last field of the
`playback.start-paused` entry — field-order drift in schemas/settings.toml
makes the mutation a no-op; the file's own guard (L94) then fails the test
loudly, so this is self-defusing brittleness, not silent rot.
Confidence: HIGH — Verification: read.
Zero S0-S3 findings. Checked and found clean: subprocess exit-code contract
(L56-68), stderr capture (L73-88), temp-dir mutation isolation (L90-98 — the
real schema is never touched), path slicing vs Vite URL rewriting (L48-54),
area/entry counts (L153-165), key uniqueness + min/max/enum-default rules
(L168-180), default_ref/featureGate/restartRequired pins (L182-207), msgid
count 121 + no-dup + coverage (L245-263).

## 4. Per-function table (describe/it groups)
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| loadNode / runGenerator / runGeneratorOutput | 38-88 | REAL | dynamic node builtins, typed locally; exit-code contract |
| mutateSchemaCopy | 90-98 | REAL | self-guarding mutation on temp copy |
| drift detection (4.E1 AC) | 102-140 | REAL | 4 failing modes incl. stderr-asserted collision |
| committed snapshot sanity | 142-266 | REAL | counts, uniqueness, ranges, refs, gates, restart, i18n |

## 5. Test honesty
All 11 tests fail on their named regression (§2). The four mutation tests are
positive controls for the drift gate itself.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The generator is re-run in --check mode (byte-compare…)" | file L2-3 | REAL | runGenerator(["--check"]) exit-0 test + script exists |
| "nothing needs restoring" (mutation on temp copy) | file L6-7 | REAL | mkdtempSync path, real schema read-only (L92-97) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Node builtins only (fs/os/child_process) via dynamic import; no new deps
(card rule cited L9-11); @types/node now a devDependency (L39-40 note).

## 9. Open questions for the next agent
- Is scripts/gen-settings-screens.mjs itself audited by another agent?
  (Not in my assignment list; its drift gate is exercised from here.)
