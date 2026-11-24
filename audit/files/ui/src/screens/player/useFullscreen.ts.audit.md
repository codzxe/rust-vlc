# AUDIT: ui/src/screens/player/useFullscreen.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 79 / 1 (useFullscreen) + FullscreenUnsupportedError |
| File verdict | REAL — web-API path only; the documented shell-command path is not implemented |
| Reachable from shipped entry point? | YES — PlayerScreen (F key fullscreen-toggle -> toggleFullscreen, PlayerScreen.tsx:824-825) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln "useFullscreen"` (PlayerScreen); `rg -rn "video.set-fullscreen" src/` (catalog + comments only — no dispatcher); `sed -n 215,230p keymap.ts` (target:shell note) |

## 1. One-paragraph verdict
Honest-state fullscreen hook: isFullscreen derived only from `fullscreenchange` events (never assumed from request success), live-document toggle (no stale React cache), typed unsupported error, capability probe. Implemented exclusively via the web Fullscreen API — the header's claim that the desktop path is the `video.set-fullscreen` SHELL command (doc 05 §2.7) is not implemented anywhere: `rg video.set-fullscreen src/` finds the generated catalog entry and two comments, zero dispatches. On the desktop webview the element-fullscreen API still functions (WebKitGTK/WebView2 support it), so the F key works — but the documented shell window-fullscreen path is dead protocol from the UI side.

## 2. What actually works (verified)
— verified by useFullscreen.test.tsx (per file list; baseline vitest 1705 passed) + PlayerScreen consumer trace. Event-synced re-derivation reviewed (L40-47, L68, L73).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/useFullscreen.ts:3-4
Code:
```
// `video.set-fullscreen` is a SHELL-target command (doc 05 §2.7): desktop
// Tauri window fullscreen; web Fullscreen API on the player container.
```
What is wrong: comment describes a dual-path; only the web path exists. `video.set-fullscreen` sits in the generated CommandRequest catalog un-dispatched by any UI code. Blast radius: none today (element fullscreen works in webviews); the drift matters if the shell's window-fullscreen behavior differs from element-fullscreen (e.g. multi-monitor macOS mismatches). Confidence: HIGH — Verification: read + rg sweep (quoted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FullscreenUnsupportedError | 14-19 | REAL | — | typed refusal |
| useFullscreen | 34-79 | REAL | S4 | F-001 comment drift; revision-counter re-render pattern |

## 5. Test honesty
useFullscreen.test.tsx exists (file list); would fail on event-sync/toggle regressions. Not individually executed — baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "desktop Tauri window fullscreen; web Fullscreen API" (L3-4) | header | HALF-FALSE | only web API implemented; command never sent (rg) |

## 7. Dead / duplicated / unreachable code
None in the file; `video.set-fullscreen` is a dead catalog command from the UI's perspective.

## 8. Dependencies & flags
Browser Fullscreen API only. No transport import.

## 9. Open questions for the next agent
- Does apps/desktop implement a video.set-fullscreen handler that can never be reached from the UI? (Shell-side audit; would upgrade this to a cross-layer dead seam.)
