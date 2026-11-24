# AUDIT: ui/src/components/appshell/AppShell.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 855 / 5 (useShellMsg, loadSidebarOpen, persistSidebarOpen, useShellFullscreen, AppShell) |
| File verdict | REAL-UNPROVEN (logic verified by read + committed tests; not executed in a browser by me) |
| Reachable from shipped entry point? | YES — main.tsx -> App (App.tsx:3,179 `import { AppShell }`; `<AppShell demo={DEMO_SHELL} provideMain={...}>` inside GatedShell) -> every non-onboarding route |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n '710,721p'`, `sed -n '412,420p'` AppShell.tsx (quote verify, exit 0); `rg` AppShell/test/i18n/transport traces (exit 0) |

## 1. One-paragraph verdict
This is the app chrome and it is genuinely wired: rail NavLinks route to real screens, the sidebar toggle persists real state (localStorage `lumen.shell.sidebar.v1`), window "close" dispatches a real wire command `playback.stop` over the ClientProvider transport, OS window controls call a probed `plugin:window|…` backend (desktop-only, gated by `isDesktopShell()`), and the playlists sidebar reads the same `playlistsStore` /playlist uses. The master question's dark side is confirmed here but honestly labeled: in a browser the transport IS the DemoTransport simulation (App.tsx:135 `const DEMO_SHELL = !isDesktopShell()`; app-transport.ts:75), so every "real" command in browser context is simulated — the shell badges this with a visible "Demo" chip (L566-570) and `data-demo`. The one piece of dead chrome is the sidebar "…" button, permanently disabled.

## 2. What actually works (verified)
— verified by `sed` quote checks + committed test suite read (AppShell.test.tsx, 792 LOC, ~30 its covering badge on/off, OS cluster gating, sidebar persistence/route defaults, create-flow store persistence, `playback.stop` dispatched on close at test line 624). Not executed by me — UI test runs cited from orchestrator baseline: "UI: npm ci/typecheck/test/build all pass (vitest 1705 passed, 1 skipped)" (audit/waves/wave-00/AGENT-SPEC.md §13).

## 3. Findings
F-001 [S3]
Where: ui/src/components/appshell/AppShell.tsx:713-720
Code:
```
            <IconButton
              label={t("shell.sidebar.more")}
              size="sm"
              disabled
              title={t("shell.sidebar.more.hint")}
            >
              <GlyphMore size={16} />
            </IconButton>
```
What is wrong: the sidebar header's "…" (More playlist options) button is unconditionally `disabled` — dead chrome shipped in the primary navigation surface. Why it matters: it is one of four header controls; users click "…" expecting a menu. Mitigation: the tooltip is honest — messages.ts:2325 `"shell.sidebar.more.hint": "Playlist actions are not exposed by this build yet."` — so this is a labeled dead button, not a fake one. What correct looks like: omit the button until actions exist. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: ui/src/components/appshell/AppShell.tsx:301-309
Code:
```
      void client.send({ ...request, requestId: nextRequestId.current } as CommandRequest).catch(() => {
        // Transport unreachable — screens surface their own honest errors.
      });
```
What is wrong: fire-and-forget dispatch swallows every transport error with an empty catch; the comment's claim ("screens surface their own honest errors") is wrong for the one command this shell dispatches (`playback.stop` on window-close, L418) — no screen surfaces its failure; the user gets silence. Blast radius: one command, desktop+demo both. Confidence: HIGH — Verification: read (no error path renders for this dispatch).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useShellMsg | 126-133 | REAL-UNPROVEN | — | `m(key) ?? laneCatalog[key]` fallback; keys verified registered in messages.ts (L562-563, 846-847 exist), so fallback is inert passthrough today (comment at 134-138 admits this) |
| loadSidebarOpen | 210-231 | REAL-UNPROVEN | — | localStorage read with try/catch fallback open; route-aware defaults; sound |
| persistSidebarOpen | 233-239 | REAL-UNPROVEN | — | guarded localStorage write; no-op on unavailable host |
| useShellFullscreen | 246-268 | REAL-UNPROVEN | — | document fullscreen API + optional-chained no-op on hosts lacking it; honest pressed state |
| AppShell | 274-855 | REAL-UNPROVEN | — | full chrome; all interactive elements wired (see §7 for the one exception); `data-tauri-drag-region` inert in browsers by design |

## 5. Test honesty
AppShell.test.tsx (792 LOC, not assigned to me but read for context): tests assert DOM structure, aria state, localStorage persistence, store persistence after dialog save, and transport sends (`request.cmd === "playback.stop"`, line 624). Would fail if code were broken: YES for structure/aria/persistence; the transport assertion uses a spy transport — honest for "shell dispatches the command", says nothing about engine execution (out of UI scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no dead chrome, no second data source" (file header L20-23) | AppShell.tsx comments | MOSTLY TRUE | Sidebar rows = playlistsStore (same as /playlist); exception: disabled "…" button (F-001) |
| Browser runs DemoTransport, "badged Demo by the AppShell" | App.tsx:130-135 | REAL | app-transport.ts:75 `?? new DemoTransport()`; badge rendered AppShell.tsx:566-570 |
| OS window controls drive plugin:window|… desktop-only | AppShell.tsx:422-431 | REAL-UNPROVEN | Gated by `isDesktopShell()` + `detectOsWindowBackend()`; desktop build UNVERIFIABLE in sandbox (baseline: gdk-sys failure) |

## 7. Dead / duplicated / unreachable code
- Disabled "…" button (F-001) — the only unwired interactive element in the file.
- `useShellMsg` lane-catalog fallback is dead weight while keys are registered (self-documented, S4-grade convention debt).

## 8. Dependencies & flags
Imports ClientProvider (`useClient`), app-transport (`isDesktopShell`), window-controls, OpenMediaDialog, NewPlaylistDialog, playlistsStore, list primitives. No feature flags. `demo` prop comes from App.tsx `DEMO_SHELL`.

## 9. Open questions for the next agent
- Does the desktop TauriTransport actually deliver `playback.stop` to the engine (client/tauri-transport.ts + Rust lumen_dispatch)? Wave 12/16 territory.
- playlistsStore persistence is localStorage-only ("no engine seam exists for saved playlists" L86-87) — see my playlistsStore.ts audit.
