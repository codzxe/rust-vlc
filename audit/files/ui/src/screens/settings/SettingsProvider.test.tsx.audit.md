# AUDIT: ui/src/screens/settings/SettingsProvider.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 207 / 14 (2 describes) |
| File verdict | REAL (baseline-verified green; state-machine coverage incl. live-feed races) |
| Reachable from shipped entry point? | YES — SettingsProvider.tsx (159 LOC) is the settings state root every area screen composes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:90; wc source |

## 1. Verdict
Full state-contract suite for the settings draft/persisted model: clean start vs schema
defaults, unknown-key drop (config-never-invents), per-key dirty tracking, single-key and
full revert, apply committing only non-default overrides, revert-after-apply semantics,
field/area reset scoping, restart-required bookkeeping across applies, plus the live engine
feed: external config.setting-changed moves clean state but never clobbers a dirty draft, and
unmount unsubscribes. Baseline green: ui-test.txt:90 `✓ …SettingsProvider.test.tsx (14 tests) 42ms`.

## 2. What actually works (verified)
- Baseline 14/14 (cite above).
- Mutation reasoning: apply-not-committing fails :74-77 (persisted + saved payload equality);
  revert-to-pre-edit-instead-of-applied fails :80-87 (the exact regression the test names);
  area reset leaking fails :97-106; restart flag never clearing fails :119-126; engine feed
  clobbering user edits fails :167-182; leak on unmount fails :202-206.
- Persistence is a recorded fake at the SEAM (:9-20) — the provider logic is what's under
  test, and every save payload is asserted exactly (:75-77).
- The "ignored events" test proves non-movement off defaults, the honest form of an absence
  claim (:184-200).

## 3. Findings
None. Checked: all 14 tests read for tautologies (none — all assert moved/moved-not state);
act() discipline correct on every state mutation; the emit helper drives a captured listener
through act (no unawaited updates); mocks only at persistence/subscription seams; source
SettingsProvider.tsx 159 LOC exists and is composed by every settings area (per audio.test).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clean start | YES | dirtyCount 0 + product.toml defaults |
| unknown key drop | YES | draft undefined for bogus key |
| dirty + full revert | YES | counts + value round-trip |
| single-key revert | YES | second key untouched |
| apply | YES | persisted + exact saved array |
| revert-after-apply | YES | applied value, not pre-edit |
| resetField | YES | schema default + still dirty |
| resetArea | YES | area-only effect |
| restart bookkeeping ×2 | YES | pending list contents |
| feed moves clean value | YES | value+persisted+dirty 0 |
| dirty draft wins | YES | user value survives engine move |
| unknown/unrelated ignored | YES | nothing moves off defaults |
| unmount release | YES | unsubscribe called |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "config-never-invents (doc 05 §6.4)" | :42 | REAL | drop + ignore tests |
| live engine feed (issue #3) | :129 | REAL | 5 feed tests |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
renderHook + act; vi.fn only for unsubscribe spy. No component mocks.

## 8. Open questions for the next agent
None blocking.
