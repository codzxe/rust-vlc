# AUDIT: ui/src/client/app-transport.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 80 / 2 (selectAppTransport, isDesktopShell) + 2 types |
| File verdict | REAL — the single honest transport selector; real shell gets the real bridge, browser gets the badged demo |
| Reachable from shipped entry point? | YES — this IS entry wiring (main.tsx:22/56 `selectAppTransport()`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read tauri-transport.ts (194 lines, context); rg `selectAppTransport|isDesktopShell` (exit 0 — main.tsx:56, AppShell/App/SettingsScreen consumers); read main.tsx:1-60 |

## 1. One-paragraph verdict
Correct and honest: one probe (`detectTauriBackend()` on `window.__TAURI_INTERNALS__`), two outcomes — the desktop shell runs TauriTransport (every command through the shell's single `lumen_dispatch` tauri command; events on `lumen://event`), the browser runs DemoTransport, and exactly one console.debug line announces which. The label copy is accurate against the code I traced end-to-end: TauriTransport.send invokes `lumen_dispatch` with `{ request }` (matching the Rust `async fn lumen_dispatch(app, state, request: WireRequest)` at apps/desktop-tauri/src/app.rs:144-154) and subscribe listens on `lumen://event` (the shell's EVENT_CHANNEL, app.rs:100). The demo label honestly says "simulated engine ... no decoded media". The one caveat worth recording: the tauri event-frame shape carries an explicit [VERIFY] marker (tauri-transport.ts:93-97) — the wire shape is verified only by the CI lane (tauri-launch.yml), not in this sandbox (UNVERIFIED here, environment limit per spec §13).

## 2. What actually works (verified)
— verified by read + rg: probe single-sourced (isDesktopShell uses the same detectTauriBackend — L41); no double-probe drift (backend handed straight to the transport, L62-67); injectable browserFallback for tests; main.tsx:56 calls it exactly once.
— verified by read of tauri-transport.ts: the "real engine" label claim is truthful about the WIRE (single command, event channel); what the engine does with those commands (15-command SERVED surface, typed refusals elsewhere) is disclosed by the host, not hidden here.

## 3. Findings
F-001 [S4]
Where: ui/src/client/app-transport.ts:75 (and tauri-transport.ts:93-97 context)
Code:
```
  const transport = options.browserFallback?.() ?? new DemoTransport();
```
What is wrong: nothing functional — recording that the DEFAULT browser experience is the full simulation (playlists, bookmarks, volume, shuffle all "work" in the browser demo and are typed-refused in the desktop shell). The badge + debug line disclose it, but every demo-shell screenshot/video of "working" playlist/volume UI is unrepresentative of the product. This is the mock-as-real architectural split, honestly labeled at this seam and deceptive only if the label is ignored downstream (docs/matrix). Confidence: HIGH — Verification: read (both transports) + demo-transport simulation evidence from PlaylistPanel audit.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AppTransportOrigin / AppTransport (types) | 21-33 | REAL | — | |
| isDesktopShell | 40-42 | REAL | — | single probe source |
| SelectAppTransportOptions | 44-51 | REAL | — | injectable fallback/log/failure seam |
| selectAppTransport | 58-80 | REAL | S4 | honest labels; demo default (F-001) |

## 5. Test honesty
app-transport.test.ts exists (client dir); tests inject fake backends — they prove selection logic, not the webview wire (which the CI [VERIFY] marker owns). UNVERIFIED beyond baseline for the webview half.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "When that surface is present the app runs on the REAL transport — TauriTransport over the shell's single lumen_dispatch command and the lumen://event channel" (L5-8) | header | REAL | read tauri-transport.ts + app.rs:144-154/100 |
| "the choice is VISIBLE — exactly one debug line" (L12-13) | header | REAL | L61-79: one log call per branch |
| "real wire protocol, no decoded media" (demo label, L74) | header | REAL | demo-transport simulates command results (read) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
demo-transport, tauri-transport (detectTauriBackend). No feature flags; the environment probe IS the flag.

## 9. Open questions for the next agent
- tauri-transport.ts owner: has the [VERIFY] marker (frame shape/Any target) been cleared by a green tauri-launch.yml run? If yes, delete the marker; if no, the event path is REAL-UNPROVEN in production.
