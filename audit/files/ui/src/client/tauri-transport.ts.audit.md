# ui/src/client/tauri-transport.ts — the UI's command transport (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `ui/src/client/tauri-transport.ts` (194 LOC) |
| Reachability | lumen-tauri binary → ui/dist → `ClientProvider` (built from `detectTauriBackend`, line 113-118) → every UI command (`session.open`, `playback.play`, `session.set-frame-channel`, settings, ...) rides `TauriTransport.send` → `invoke("lumen_dispatch", { request })` (line 143) → the shell's single command. Events: `subscribe` → `plugin:event|listen` on `lumen://event` (lines 91-108, 160-182) → the UI state machines. |
| Verdict | **Real transport with the GUI's single most load-bearing unresolved marker: the event-listen frame shape is `[VERIFY at the ADR-004 admission build]` — the repo's own words, still unresolved at HEAD per INFRA-BLOCKED.md §B.2.** The command half is symmetric, honest, and committed-tested against mocks. |

## Per-function table

| Unit | Lines | What it does (sed-verified) |
|---|---|---|
| `LUMEN_DISPATCH_COMMAND` / `LUMEN_EVENT_CHANNEL` | 28 / 31 | `"lumen_dispatch"` / `"lumen://event"` — the shell's app.rs:97 mirror. |
| `internalsBackend` | 86-110 | binds `invoke` + a `listen` built on `internals.invoke("plugin:event|listen", { event, target: { kind: "Any" }, handler: transformCallback(...) })` (lines 91-102). |
| `detectTauriBackend` | 113-118 | `__TAURI_INTERNALS__` presence probe. |
| `TauriTransport.send` | 138-158 | `invoke(LUMEN_DISPATCH_COMMAND, { request })` (line 143 — the payload key IS the parameter name); a rejected promise that carries a typed `CommandError` is normalized into the same error envelope (lines 150-152, gen 0 = unset); anything else throws. |
| `TauriTransport.subscribe` | 160-182 | listen on the event channel; unlisten wiring with the unsubscribed-before-listen race handled (lines 178-181); listen failure → `onListenFailure` (never swallowed, lines 173-177). |
| `createTauriTransport` | 186-194 | throws `TauriTransportUnavailableError` outside the shell (63-71). |

## Findings

- **S2 — the event-listen shape carries the repo's own unresolved `[VERIFY]` marker.** Sed-verified lines 92-98:
  ```
  // [VERIFY at the ADR-004 admission build] the frame shape and the
  // "Any" default target are per Tauri 2.x api/event; unverifiable
  // without a webview here (INFRA-BLOCKED.md §unblock step 3).
  // (CI half: tauri-launch.yml — clears when its no-emit-failure +
  // clean-exit proof lands on a real run; the GUI half stays human.)
  ```
  Every GUI behavior downstream (state changes, track lists, position ticks, EOS, and thus the MediaSurface mount that binds the frame channel) rides this one invoke's runtime shape. INFRA-BLOCKED.md §B.2 confirms it is still open at HEAD: "the tauri-launch.yml deep verdicts (window/file/playback/settings per OS, the event-frame round-trip) remain manual." The command half (`lumen_dispatch` invoke) is under the same runtime cloud but its shape is enforced by serde on the Rust side and exercised by the CI launch smokes' stderr receipts only — no round-trip proof either.
- **S3 — `gen: 0` fabrication on the error path** (line 151): a shell-level rejection is answered with `gen: 0` — the comment says "0 = 'unset', never presented as engine truth" (lines 148-149), but the envelope shape is indistinguishable from a real generation-0 response to any consumer that does not know the provenance. Typed-comment honesty, not type-system honesty. Recorded.
- **S4 — `subscribe` before the listen resolves** (lines 163-181): listeners registered between subscribe and the async listen completion miss events (the closure holds none); the shell emits events under its lock immediately, so early engine events (the open's own burst) can be lost if the UI subscribes concurrently with an invoke. The app entry's mount order (ClientProvider before screens) mitigates; no committed test pins the race. Recorded.
- **S4 — unlisten failure is fire-and-forget** (lines 105-108): `void internals.invoke(...)` — an unlisten rejection is dropped on the floor at cleanup time. Leak-only risk (a dead listener id server-side), cosmetic.

## Test honesty

`tauri-transport.test.ts` (264 LOC): mocked `TauriBackend` — send routes through the single command with `{ request }` payload shape (81-190), rejection normalization, the probe/constructor refusals (191-264). Mocked posture is stated in the file header (lines 12-13). Same ceiling: no webview anywhere.

## Doc claims

- Line 27's claim "The one and only registered shell command (doc 05 §5.1; dispatch.rs)" matches the Rust side exactly (app.rs:291 + tests/single_command.rs).
- The `[VERIFY]` marker itself is the doc honesty — the file names its own unproven half. No overclaim found.

## Dead code / deps / flags

- No dead exports. No `@tauri-apps/api` import (lines 6-13) — the documented allowlist posture.

## Open questions

1. Does `plugin:event|listen`'s `target: { kind: "Any" }` need a capability grant beyond `core:default`? `core:event:default` is included in `core:default` (the capability file's own description asserts exactly this). Static reading agrees; runtime UNVERIFIED.
