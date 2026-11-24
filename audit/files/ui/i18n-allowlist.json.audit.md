# AUDIT: ui/i18n-allowlist.json
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 47 / 6 allowlist entries |
| File verdict | REAL (every entry verified against live code) |
| Reachable from shipped entry point? | Config plane: enforced at lint time by ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:33 (reads this file); runtime reachability n/a |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg/sed verification of all 6 literals at their cited files (exit 0) |

## 1. One-paragraph verdict
Every allowlisted literal is real and present at the cited locations — I verified all six against the source (several are JSX text nodes, not quoted strings, which is why naive `"lit"` greps miss them): the em-dash separator SettingsScreen.tsx:393 `... —{" "}`, the unsaved-count parens SettingsScreen.tsx:416 `({settings.dirtyCount})` and HotkeysEditor.tsx:244 `({dirtyIds.length})`, the position/duration slash GotoTimeDialog.tsx:125 `... /{" "}`, the onboarding shortcut separator OnboardingScreen.tsx:875-876 (aria-hidden `/` span), and the two frame-pinned decorative timecodes in CustomizePanels.tsx:692+ (aria-hidden preview chrome, explicitly not the live control bar). The allowlist is file-scoped and enforced by the local eslint rule that reads it. Note: the baseline `lint-literals` gate failure (full_gate.txt:59-63) is a Rust-side identity literal in apps/cli/src/host.rs:1799 — NOT this file.

## 2. What actually works (verified)
— 6/6 entries match live code (command list above; each cited path+line confirmed).
— Enforcement: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:31-33 loads this exact file.
— The e2e visual matrix doc (ui/e2e/visual/MATRIX-20261109.md) cites it for the pseudo-locale hardcode proof.

## 3. Findings
None. Checked: every entry's file scoping, the "decorative/aria-hidden" claims for the timecodes (true — customize band preview), and the data-not-copy rationale for parens/slashes (counts and timecodes are values).

## 4. Per-function table
N/A (JSON config): 6 entries each verified individually.

## 5. Test honesty
The eslint rule's own tests (i18n-no-literals.test.js) exercise allowlist matching; baseline UI lint passes (ui-test/typecheck/build green in baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| each entry's "files" scoping | this file | TRUE | all 6 verified present at cited files |
| "the real control bar renders the live localized timecodes" (timecode entries) | L38-39, L43-44 | TRUE | CustomizePanels band is aria-hidden preview chrome; ControlBar renders engine-truth time |

## 7. Dead / duplicated / unreachable code
None — no stale entries (a risk I specifically checked: every allowlist row has a live literal).

## 8. Dependencies & flags
Consumed by the local eslint plugin at lint time; no runtime cost.

## 9. Open questions for the next agent
- None.
