# AUDIT: ui/src/screens/player/dialogs/GotoTimeDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 156 / 2 (GotoTimeDialog, onSubmit) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:793 `setGotoOpen(true)` (hotkey Ctrl+T `uiAction === "goto-time"`, has-media-gated) and ControlBar.tsx:404 -> player screen -> App -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '370,400p' ui/src/screens/player/context.tsx` (actions.seek body); `sed -n '788,796p' PlayerScreen.tsx`; `od -c` byte-verify of parseTimeInput.ts:40 (display-artifact clear); test-file scans (parseTimeInput.test.ts 7 cases; GotoTimeDialog.test.tsx behavioral cases) |

## 1. One-paragraph verdict
This dialog is a master-question POSITIVE: a UI element genuinely wired to
the real backend command path. Submit parses the time input (parseTimeInput),
cross-checks the engine-reported duration (null = live stream = no invented
ceiling), then calls `usePlayerControls().actions.seek(seconds)` — the SAME
seam the seekbar uses — which dispatches `playback.seek` with
`{ target: { kind: "absolute", time: secondsToUs(seconds) } }`
(context.tsx:378-386, verified quote below). Prefill is the engine-reported
position at open; failures surface as field errors; the dialog is honest about
pending (closes on submit; seek truth arrives via timing.seek-completed/failed
in the seam). Not executable in this sandbox (no node_modules; desktop build
blocked at gdk-sys per baseline) — hence REAL-UNPROVEN, not REAL.

## 2. What actually works (verified)
— Wiring chain verified by read: GotoTimeDialog.tsx:84 `actions.seek(seconds)`
-> context.tsx:378-386 (quote in §3 context) -> transport dispatch
(`cmd: "playback.seek"`) -> TauriTransport in the desktop shell (app-transport
probe). — parseTimeInput.ts grammar verified (incl. od byte-check of the
destructure line after a display artifact suggested corruption — intact):
bare seconds w/ fraction, mm:ss, hh:mm:ss; rejects negatives, >3 fields,
non-numeric, mm/ss ≥ 60. 7 unit tests (parseTimeInput.test.ts).
— GotoTimeDialog.test.tsx:124 asserts submit produces `playback.seek` with
ABSOLUTE µs on a spy transport — the wiring is test-pinned, not just claimed.
— Hotkey path verified: PlayerScreen.tsx:790-793 (quote below) opens the
dialog on the keymap's `goto-time` uiAction, has-media-gated.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/dialogs/GotoTimeDialog.tsx:60
Code:
```
  // Prefill: the engine-reported position, captured at mount (= open).
  const [value, setValue] = useState(() => formatSeekTime(state.position));
```
What is wrong: purely cosmetic — if the dialog stays open while playback
continues, the input prefill goes stale (the live position/duration hint at
L124-127 stays live, so the mismatch is visible but harmless). Mount-per-open
makes the window small.
Confidence: HIGH — Verification: read.

Zero S0-S3 findings. Checked and how: fake seams (none — every control wired:
form submit, Cancel, ✕, scrim/Escape via Dialog); mock data (none — position
and duration come from the engine seam, "--:--" for unknown); dead buttons
(none); units (seconds->µs conversion verified at the dispatch site);
swallowed errors (invalid input -> field error, no silent paths); i18n (all
strings via useDialogsMsg); a11y (labelledBy, form submit button, inputMode).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GotoTimeDialog | 50-156 | REAL-UNPROVEN | S4 (F-001) | full dialog; wiring verified to dispatch site |
| onSubmit | 65-86 | REAL-UNPROVEN | — | parse -> duration cross-check (null-aware) -> actions.seek -> close |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "submits mm:ss as playback.seek ABSOLUTE µs (wire) and closes" (GotoTimeDialog.test.tsx:124) | YES | spy transport asserts cmd + target kind + µs value |
| "initial focus lands on the Time input, never the header ✕" | YES | focus assertion (L76) |
| "prefills with a nonzero engine-reported position" | YES | emits timing event, asserts formatted prefill (L100-112) |
| parseTimeInput.test.ts (7 cases) | YES | table-driven grammar incl. rejections |
Not run here (no node_modules; baseline vitest 1705 passed / 1 skipped cited).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`playback.seek` with an ABSOLUTE target through PlayerControlsProvider's `actions.seek`" | GotoTimeDialog.tsx:5-7 | REAL | context.tsx:378-386 dispatch (read) |
| "The engine side of a goto is just the existing seek; no new command exists (nor is one needed)" | GotoTimeDialog.tsx:14-15 | REAL | playback.seek in catalog (commands.ts:24) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Button/IconButton/TextField/Dialog, formatSeekTime, usePlayerControls (the
REAL transport seam), parseTimeInput, useDialogsMsg, dialogs.css. No feature
gates. No direct transport import — correctly goes through the seam.

## 9. Open questions for the next agent
- Engine side: confirm `playback.seek` absolute targets are implemented (not
  catalog-only) — the MP4 smoke baseline exercised `lumen open`, not seeks.
- ControlBar.tsx:404 also mounts this dialog — same wiring, no divergence.
