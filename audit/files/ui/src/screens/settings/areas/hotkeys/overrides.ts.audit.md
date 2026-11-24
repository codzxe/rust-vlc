# AUDIT: ui/src/screens/settings/areas/hotkeys/overrides.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 49 / 2 store impls (+1 type, 1 iface) |
| File verdict | REAL code, honestly-labeled MOCK persistence — and a DEAD WRITE: no runtime consumer ever loads the persisted hotkey overrides |
| Reachable from shipped entry point? | YES (HotkeysEditor.tsx:9/156 default store; controls.tsx:44/176 reset) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Grep `lumen.hotkeys.overrides|useKeymap|resolveBinding` (exit 0 — storage key appears ONLY in this file); Grep `HOTKEY_BINDINGS` (exit 0 — consumed only by editor + tests); Grep player keymap (exit 0 — keymap.ts is a 3-binding seam, "keymap LOADING pipeline has not landed") |

## 1. One-paragraph verdict
The localStorage round-trip is real and crash-proof (corrupt store -> {}, non-string/empty combos dropped, honest memory test double). But the feature it persists is a dead write: the storage key `lumen.hotkeys.overrides.v1` is referenced by exactly one file (this one); the ONLY loaders are the editor itself (to re-display drafts) and tests — no runtime keyboard dispatcher, no keymap engine, nothing applies an override to actual key handling. The player's keymap.ts is a three-binding seam whose own header says "The keymap LOADING pipeline has not landed". So the user can rebind 141 hotkeys in the settings editor, the bindings persist, and every actual keyboard action keeps firing its hardwired default. The header is honest about the MOCK store; the dead-write consequence is the finding.

## 2. What actually works (verified)
— verified by read: load/save round-trip, corrupt-store tolerance, entry validation (string + non-empty), memory test double labeled as test double.
— verified by Grep: consumers are HotkeysEditor (display) + controls.tsx reset (save({})) — both UI-only.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/hotkeys/overrides.ts:14 (and the consumer graph)
Code:
```
const STORAGE_KEY = "lumen.hotkeys.overrides.v1";
```
What is wrong: persisted hotkey overrides have ZERO runtime consumers — Grep proves the key is private to this module and `HOTKEY_BINDINGS` (141 rows) is imported only by the editor and tests. Rebinding a hotkey in the shipped app changes nothing about actual keyboard behavior (the player keymap seam is 3 bindings, handwired, loading pipeline not landed per its header). The editor + persistence is a complete-looking feature with no effector. Why it matters: dead-write persistence is the settings version of a dead button — the user believes their rebinding took. What correct looks like: the runtime key dispatcher reads this store (the seam's stated handoff), or the editor is disabled until it lands. Confidence: HIGH — Verification: ran Grep (three queries, all zero runtime consumers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HotkeyOverrides / HotkeyOverridesStore (types) | 7-12 | REAL | — | mirrors settings seam shape |
| localStorageHotkeyOverrides | 16-39 | REAL code / dead-write product | S3 | F-001 |
| memoryHotkeyOverrides | 41-48 | REAL | — | test double, labeled |

## 5. Test honesty
controls.test.tsx:121-130 exercises save->load round-trip + reset — it proves the STORE, not the feature (no test asserts a rebound key fires its action, because nothing could). UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a MOCK localStorage round-trip today, the real write path (keymap config over the client bridge) is the app layer's handoff" (L2-5) | header | REAL | honest label; handoff not landed (Grep evidence) |

## 7. Dead / duplicated / unreachable code
The persisted data is write-only at product level (F-001) — readable only by its own editor.

## 8. Dependencies & flags
window.localStorage only. No feature flags.

## 9. Open questions for the next agent
- keymap.ts owner: when the loading pipeline lands, wire this store into the dispatcher in the SAME change — until then consider labeling the editor's Save with the staged-marker pattern the tools use.
