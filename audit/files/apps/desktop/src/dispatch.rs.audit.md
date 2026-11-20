# apps/desktop/src/dispatch.rs — the single command's engine half (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `apps/desktop/src/dispatch.rs` (401 LOC; 261 production / 140 test) |
| Reachability | lumen-tauri binary → `lumen_dispatch` (app.rs:142-152) → `ShellRuntime::dispatch` (shell.rs:153-167) → `lumen_dispatch` HERE (line 211) → `lumen_cli::dispatch::lumen_dispatch` → the engine's command layer. Every GUI command, including file-open, crosses this file. |
| Verdict | **Real and provably the same command layer as the CLI.** No desktop-private engine path exists; the anti-bypass discipline (one command) is enforced by test in two places. |

## Per-function table (production half)

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `ShellHost::new` / `engine_host` | 82-94 | wraps `lumen_cli::host::CliHost` + a forwarded-event cursor (line 70). |
| `ShellHost::forward_new_events` | 100-109 | drains `host.events()[forwarded..]` in engine order, exactly once each — the `lumen://event` channel's webview-free half. |
| `EventQueueStats` | 122-142 | offered/taken/dropped_overflow/dropped_stale + `conserves()` identity. |
| `BoundedEventSink` | 151-203 | bounded queue, drop-OLDEST on full (lines 190-196), counted. |
| `lumen_dispatch` | 211-213 | `lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)` — the whole routing body is a delegation. |
| `open_file_request` | 220-231 | builds `Command::SessionOpen { source: SourceSpec::File { path } }`. |
| `playlist_load_request` | 238-246 | `Command::PlaylistLoad { path, append: false }`. |
| `open_path_request` | 253-261 | playlist → load; media → open; else `None` (honest no-op). |

Key quote (sed-verified, line 211-213):
```
pub fn lumen_dispatch(shell: &mut ShellHost, request: &WireRequest) -> WireResponse {
    lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)
}
```

## The file-open trace (trace question, GUI side)

Three acquisitions, ONE funnel:
1. **In-webview dialog** (the only UI-reachable native flow): PlayerScreen/ControlBar dispatch `window.dispatchEvent(new CustomEvent("lumen:open-media"))` (PlayerScreen.tsx:471, ControlBar.tsx:958) → AppShell listens (`AppShell.tsx:83` imports `OpenMediaDialog, OPEN_MEDIA_EVENT`; line 851 mounts it) → the dialog's `openSource` sends `openPathRequest(source, requestId)` (OpenMediaDialog.tsx:148) → cmd `"session.open"` with `SourceSpec::File { path }` (open-path.ts:58-69) → `TauriTransport.send` → `invoke("lumen_dispatch", { request })` (tauri-transport.ts:143) → HERE → engine.
2. **Drag-drop**: `on_window_event` → `ShellRuntime::on_files_dropped` → same `dispatch` (app.rs:303-318).
3. **argv at startup**: `open_argv_files` → `ShellRuntime::open_path` (app.rs:269-283).

The native `pick_and_open` dialog is deliberately unwired (app.rs:75-79: "it needs a dialog plugin admission of its own (capabilities stay `core:default` only, no plugins)").

## Findings

- **S4 — `forward_new_events` clones every event into the sink** (`sink.forward(event)` with `WireEvent: Clone` queue, line 194) — bounded at 64, so the allocation churn is capped. Consistent design.
- **S4 — `BoundedEventSink::offer` pops the front BEFORE pushing** (lines 190-196): at capacity, the oldest is dropped to admit the newest — matches the doc'd "drop-oldest, order of survivors never violated" and pinned by test (shell.rs:817-842).

## Test honesty

Tests (263-401) exercise the real engine: unserved commands refuse typed with echo (292-...), real fixture opens, event-order drains, stats conservation. The §7.4 anti-bypass halves live in `apps/desktop-tauri/tests/single_command.rs` (registration scan) and the repo's architecture checks.

## Doc claims

- Module doc lines 1-35 state the single-command rule and the tauri-attribute deferral honestly ("The `tauri` attribute itself is NOT applied here: importing tauri would link webkit2gtk-4.1"). Matches the split actually shipped (app.rs in the excluded crate).

## Dead code / deps / flags

- `open_file_request`/`playlist_load_request` are consumed via `open_path_request` (and files.rs's classifier twin); no dead exports.
- Deps: `commands` + `lumen_cli` only; no tauri, no unsafe.

## Open questions

- None beyond those already recorded on shell.rs (the event cursor's once-only drain assumes `host.events()` is append-only; the CLI's contract guarantees it).
