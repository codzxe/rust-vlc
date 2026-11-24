# AUDIT: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 267 / 6 (comboOf, labelPrefix, trailingNumber, orderBindingsNumericAware, HotkeysRow, HotkeysEditor) |
| File verdict | SHALLOW (polished editor wired to a MOCK store; no runtime consumer) |
| Reachable from shipped entry point? | YES (controls area dialog — areas/controls.tsx "Edit Keyboard Shortcuts" action row -> HotkeysEditor; comment 22-24) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | rg localStorageHotkeyOverrides consumers (editor + controls.tsx:176 only); rg runtime keymap override reads (NONE); sed quote verification |

## 1. One-paragraph verdict
A beautifully engineered dead end. The editor renders 152 real bindings from
generated keymap data, with memoized rows, numeric-aware ordering, live
conflict detection, per-row reset, a dirty-gated apply bar, and a beforeunload
guard — and every edit lands in `localStorageHotkeyOverrides` ("a MOCK
localStorage round-trip today", overrides.ts:1-2), which NOTHING in the
runtime reads. The player's actual key handling matches against
DEFAULT_KEYMAP_TOML, a hard-coded 3-binding excerpt (keymap.ts:68-74:
fullscreen/exit-fullscreen/show-controls), and never consults the overrides
store. Rebinding a key in this editor has ZERO effect on the shipped app.
The write path to the engine ("keymap config over the client bridge") is an
unlanded handoff in ui/NEXT-STEPS.md.

## 2. What actually works (verified)
— orderBindingsNumericAware (56-77): real run-detection + numeric sort; only
consecutive same-prefix runs reorder.
— Memoization claims (w26.k13) are real: conflicted (175), ordered (180-183),
stable callbacks (187-196) — verified by reading deps arrays.
— Apply/Revert/dirty counting logic is internally consistent (165-172, 240-264).
— beforeunload guard (198-205) uses preventDefault() — sufficient per HTML
spec (sets returnValue).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx:154-163, 254 (store) + overrides.ts:1-4 + keymap.ts:68-74
Code:
```
export function HotkeysEditor({
  bindings = HOTKEY_BINDINGS,
  store = localStorageHotkeyOverrides,
}: {
```
and (Apply) `              store.save(next);`
What is wrong: The default store is the localStorage MOCK. rg proves the ONLY
consumers of localStorageHotkeyOverrides are this editor and
controls.tsx:176 (`localStorageHotkeyOverrides.save({})` — reset-all). The
runtime keymap (src/screens/player/keymap.ts) parses DEFAULT_KEYMAP_TOML — a
verbatim 3-binding excerpt — and has no override layer. Editing any of the
152 rows changes nothing the user's keyboard does. Why it matters: this is a
full settings surface (dialog, table, conflict UX) that performs a no-op against
the product; users will believe rebinding worked. What correct looks like:
either the engine keymap seam (bridge write + runtime read) or an honest
read-only table until it lands.
Confidence: HIGH — Verification: read + rg consumer scan.
F-002 [S3]
Where: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx:251
Code: `                if (combo !== b.key && combo !== "") next[b.id] = combo;`
What is wrong: Clearing a hotkey (empty input) is silently dropped on Apply —
the binding reverts to its default. There is no way to unassign a key, and the
UI gives no signal that empty was rejected (the row just snaps back after
Apply). Combine with F-001 and the entire edit grammar is theater, but even in
a wired world this would be a hidden data-loss behavior.
Confidence: HIGH — Verification: read + traced comboOf fallback (37-38).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| comboOf | 37-38 | REAL | - | draft ?? default (overrides pre-seeded into draft at 163) |
| labelPrefix/trailingNumber | 41-47 | REAL | - | regex-based label math |
| orderBindingsNumericAware | 56-77 | REAL | - | exported; tested elsewhere |
| HotkeysRow (memo) | 84-152 | REAL | - | stable-callback contract holds |
| HotkeysEditor | 154-267 | SHALLOW | F-001, F-002 | mock store; no runtime effect |

## 5. Test honesty
HotkeysEditor tests exist under areas/hotkeys (owned by another agent) — they
can only test the editor against its own store seam, so they cannot catch
F-001 (nothing asserts a runtime consumer).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bindings from the REAL keymap data (keymaps/default.toml via the generated module)" | HotkeysEditor.tsx:14-16 | REAL (read side) | generated/bindings.ts:29-30 cites toml ids |
| "the real write path (keymap config over the client bridge) is the app layer's handoff" | overrides.ts:2-4 | PAPER-ONLY | no bridge keymap write exists in ui/src (rg config.set-setting consumers: settings only) |
| "Editing writes DRAFT state; Apply commits ... to the persistence seam" | HotkeysEditor.tsx:18-19 | REAL (to the mock seam) | lines 247-256 |

## 7. Dead / duplicated / unreachable code
The editor itself is the dead surface (reachable UI, effect-free output).
memoryHotkeyOverrides (overrides.ts:39-46) is a labeled test double living in
production code — acceptable, explicitly commented.

## 8. Dependencies & flags
generated/bindings (152 rows), conflicts.ts, overrides.ts, hotkeys.css,
Button, i18n. No flags.

## 9. Open questions for the next agent
Does the desktop shell (crates/ command layer) have ANY keymap override
command? If the bridge protocol has one unexposed in ui/src, F-001 downgrades
to a wiring gap; if not, it is a full dead feature.
