# AUDIT: ui/src/client/window-controls.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 111 / 3 (detectOsWindowBackend, backendFromInternals, error class) + 2 interfaces |
| File verdict | REAL — a genuinely wired OS-window bridge (the − / □ / × title-bar controls) over `window.__TAURI_INTERNALS__` |
| Reachable from shipped entry point? | YES (AppShell.tsx:81/430: `isDesktopShell() ? detectOsWindowBackend() : null` — the banner controls in the desktop shell) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | cat apps/desktop-tauri/capabilities/default.json (exit 0 — all four window grants present); rg `detectOsWindowBackend|data-tauri-drag-region` (exit 0 — AppShell.tsx:430/541) |

## 1. One-paragraph verdict
This is one of the few UI surfaces that is REAL end-to-end in the shipped desktop shell: the frameless window's custom title bar drives the actual OS window via tauri's `plugin:window|minimize|toggle_maximize|close` invokes, with permissions actually granted (capabilities/default.json: `core:window:allow-minimize/allow-toggle-maximize/allow-close/allow-start-dragging` — read and verified), the drag region attribute present on the banner (AppShell.tsx:541), and a clean browser-mode fallback (null backend → banner renders browser mode). The master question gets a YES here: minimize/maximize/close/drag are wired to a real backend (the tauri runtime, deliberately NOT the engine command path — doc 05 §5.1 separation is respected). Two small defects: the label fallback contradicts its own comment, and the resize-listener leaks its JS callback on unlisten.

## 2. What actually works (verified)
— verified by cat capabilities/default.json: all four invoked permissions are granted to window "main"; `core:default` covers is_maximized + event listen (as the header claims).
— verified by rg: AppShell mounts the backend only in the desktop shell (AppShell.tsx:430) and carries `data-tauri-drag-region="deep"` (AppShell.tsx:541); the error class is exported for honest non-desktop refusal.

## 3. Findings
F-001 [S4]
Where: ui/src/client/window-controls.ts:62-65
Code:
```
  // The label the runtime stamped into THIS webview — never a hardcoded
  // "main" (a second window would silently drive the wrong one).
  const label =
    internals.metadata?.currentWindow?.label ?? "main";
```
What is wrong: the comment forbids a hardcoded "main" and the code then hardcodes "main" as the fallback. If `metadata.currentWindow` is ever absent in a multi-window future, the controls silently drive the main window from another window — exactly the failure the comment describes. Today metadata is always stamped (tauri 2.x injects `__TAURI_INTERNALS__` unconditionally — W14-A08 vendored-source proof), so no live impact. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/src/client/window-controls.ts:85-94
Code:
```
          handler: internals.transformCallback(() => handler()),
        })
        .then((eventId) => {
          ...
          return () => {
            void internals.invoke("plugin:event|unlisten", {
```
What is wrong: `transformCallback` registers a permanent JS-side callback id; the unlisten path removes the tauri event listener but never releases the transformCallback registration (no `delete` on the internals' callback table). One leaked closure per mount cycle of the banner. Micro-leak, desktop shell only. Confidence: MEDIUM — Verification: read (tauri internals API shape from tauri-transport.ts pattern; UNVERIFIED against vendored tauri source).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OsWindowBackend (iface) | 28-41 | REAL | — | injectable seam |
| TauriInternals (iface) | 44-48 | REAL | — | mirrors tauri-transport.ts |
| OsWindowControlsUnavailableError | 51-59 | REAL | — | honest browser-mode refusal |
| backendFromInternals | 61-97 | REAL | S4 | label fallback contradiction (F-001); callback leak (F-002) |
| detectOsWindowBackend | 105-110 | REAL | — | same presence probe family as isDesktopShell |

## 5. Test honesty
window-controls.test.ts exists (client dir listing); the backend is injectable so tests run without a webview — the seam design exists precisely for honest tests. Baseline vitest covers. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the seam that drives the REAL window from the shell's custom banner" (L1-2) | header | REAL | invoke targets + capabilities verified |
| "NEVER through lumen_dispatch (doc 05 §5.1: engine commands only...)" (L10-12) | header | REAL | no lumen_dispatch reference in file (read) |
| permissions list (L14-18) | header | REAL | capabilities/default.json matches exactly (cat) |
| "chrome reads THIS for the controls so the two never drift" (L102-103) | header | REAL | AppShell.tsx:430 gates on isDesktopShell() then detects (rg) |

## 7. Dead / duplicated / unreachable code
None. (`start_dragging` permission is consumed via the injected drag.js/data-tauri-drag-region path, not this module — not dead, different half.)

## 8. Dependencies & flags
window.__TAURI_INTERNALS__ only (no @tauri-apps/api import — dependency posture preserved). No feature flags; browser shell gets null.

## 9. Open questions for the next agent
- tauri-transport.ts owner (not my assignment): does its listen/unlisten share the transformCallback leak (F-002)? If yes, one shared fix.
