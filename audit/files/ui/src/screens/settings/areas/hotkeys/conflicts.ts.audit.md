# AUDIT: ui/src/screens/settings/areas/hotkeys/conflicts.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 38 / 1 (detectConflicts) |
| File verdict | REAL-UNPROVEN (correct conflict detection; test file exists) |
| Reachable from shipped entry point? | YES — HotkeysEditor (settings/controls dialog) calls it to mark conflicting rows; editor data = bindings.ts (my audit: 141 rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; consumer grep (HotkeysEditor); normalizeBindingKey cross-read (keymap.ts:110-135, audited clean) |

## 1. One-paragraph verdict
Correct group-bucketing conflict detection: (mode, normalized key, sorted modifiers) group key; unparseable spellings never guessed as equal (null -> skip); all members of a multi-member group get flagged. Reuses the single normalizeBindingKey seam. The shipped data itself contains same-mode duplicate combos (e.g. bindings.ts "Up" used by audio.volume-up AND navigation.nav-up, both player-focused) — the detector would honestly flag them; whether the editor renders those warnings is the editor lane's concern. No findings in this file.

## 2. What actually works (verified)
— verified by read: bucket logic, override-or-default combo resolution (`keys[binding.id] ?? binding.key`), set-of-all-members result. conflicts.test.ts exists (same dir).

## 3. Findings
None. Checked: normalization null path, modifier order insensitivity (sorted join), mode partition, empty-mode default, no mutation of inputs.

## 4. Per-function table
n.a. — <100 LOC single function.

## 5. Test honesty
conflicts.test.ts — UNVERIFIED depth (not my file), but the function is pure and trivially testable.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two bindings conflict when they claim the SAME normalized key combination in the SAME activation mode" | L3-5 | REAL | groupKey composition L28 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
keymap.ts normalizeBindingKey. No feature flags.

## 9. Open questions for the next agent
None.
