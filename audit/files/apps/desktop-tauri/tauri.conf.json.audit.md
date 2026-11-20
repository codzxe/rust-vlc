# apps/desktop-tauri/tauri.conf.json + capabilities/default.json — the IPC surface (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| Files | `apps/desktop-tauri/tauri.conf.json` (35 lines) + `apps/desktop-tauri/capabilities/default.json` (14 lines) |
| Reachability | Compiled into the binary by `tauri-build` (`build.rs` standard embed; Cargo.toml:64-68) and read by `tauri::generate_context!()` at app.rs:319. This IS the shipped IPC surface. |
| Verdict | **Tight, documented IPC surface: exactly one custom command (`lumen_dispatch`), four window-control grants, `core:default`, no plugins, real CSP, `withGlobalTauri: false`. No allow-all found.** |

## The recorded surface (verbatim, sed-verified)

`tauri.conf.json`:
- `:10` `"withGlobalTauri": false`
- `:11-20` one window `main`, 1280×800, min 940×600, `"decorations": false` (the frameless shell — the four window grants below are its title-bar controls).
- `:22-23` CSP:
  ```
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob: data:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; worker-src 'self' blob:; base-uri 'self'; form-action 'none'; object-src 'none'; frame-src 'none'"
  ```
- `:26-33` bundle active, targets all.

`capabilities/default.json` permissions (lines 6-12, complete list):
```
"core:default",
"core:window:allow-minimize",
"core:window:allow-toggle-maximize",
"core:window:allow-close",
"core:window:allow-start-dragging"
```
The description (line 4) states the rule and is accurate to the code: "Every UI-originated command flows EXCLUSIVELY through the single lumen_dispatch command (doc 05 §5.1 single-command rule); engine events arrive on the lumen://event channel, covered by core:event:default. The window grants are the ONLY direct core commands the UI may invoke ... No plugin permissions are granted."

## Findings

- **POSITIVE — no allow-all, no plugins, no shell/fs/http capabilities.** The custom command surface is exactly one entry point, enforced at THREE layers: capabilities (this file), the registration (app.rs:291 `generate_handler![lumen_dispatch]`), and the anti-bypass test (tests/single_command.rs:39-64). The IPC attack surface for engine control is a single JSON envelope routed to the engine's typed command layer.
- **S3 — `style-src 'unsafe-inline'`** — the one documented relaxation ("the UI's dynamic inline styles (sized sliders, positioned overlays)", INFRA-BLOCKED.md §A). Style injection is a lower-risk class than script injection (`script-src 'self'`, no unsafe-eval), but it is the weakest point of the shipped CSP and no test pins it. UNVERIFIED whether the UI could drop it (w26 CSS was not audited here).
- **S4 — `media-src 'self' blob: data:`** — `data:` media URIs are reachable; the UI's `<video>` half consumes url sources only through `src={source.url}` (MediaSurface.tsx:85), and `connect-src` gates network to self+IPC. With url-source playback being a browser-shell/demo feature, the practical exposure is the webview fetching user-entered URLs... no: a url `session.open` goes through the ENGINE (RangedResource/ureq), not the webview's fetch — `connect-src 'self' ipc: http://ipc.localhost` blocks webview-side network fetches. No finding stands beyond the record.
- **S4 — the frame channel's raw-payload fetch (`plugin:__TAURI_CHANNEL__|fetch`)** is invoked by the UI shim only in the defensive token-string branch (frame-channel.ts:433-437); on the shipped tauri 2.12.5 the shell's own injected script performs the fetch first (frame-channel.ts:29-33). Whether the capability set would permit the UI-side invoke on a drifted shell is UNVERIFIED (no runtime). Dormant risk.
- **S4 — `frontendDist: "../../ui/dist"`** (line 7): the built UI is embedded at compile time — the binary the user runs contains the w31.4 canvas code iff CI built ui/dist from HEAD. Release lanes do (INFRA-BLOCKED.md §A CI lane builds ui/dist). Consistent.

## Test honesty

- No committed test asserts the capabilities file's contents (it is a build artifact input); the security review is explicitly deferred: INFRA-BLOCKED.md §B.5 "ADR-004 per-OS verdicts + the security review (TB3: webview sandbox posture, IPC surface, navigation/content isolation) — execute against the real launched app, then flip the ADR rows." The repo itself marks the security verdict as OPEN. UNVERIFIED here accordingly.

## Doc claims

- INFRA-BLOCKED.md §A "Real CSP (`tauri.conf.json`): ... `csp: null` is gone" — matches the shipped string. "Capabilities: unchanged `core:default` only (no plugins — the 4.H1 posture)" — INCOMPLETE in the doc: the four `core:window:allow-*` grants are ALSO present (added by w28 fix 3 per the capability file's own description). Doc understates the surface by omission (S4 drift; the capability file itself is accurate).

## Dead code / deps / flags

- None: every field is consumed by tauri's runtime/config codegen.

## Open questions

1. `windows: ["main"]` in the capability — single-window posture; the frame channel binds to `get_webview_window("main")` (frame_channel.rs:45). Consistent; no second window is created anywhere.
