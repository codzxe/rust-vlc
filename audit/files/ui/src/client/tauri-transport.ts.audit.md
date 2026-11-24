# AUDIT: ui/src/client/tauri-transport.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 194 / 7 (isCommandError, internalsBackend, detectTauriBackend, TauriTransport.send/.subscribe, createTauriTransport, TauriTransportUnavailableError) |
| File verdict | REAL-UNPROVEN (unit-proven against a mocked seam; live webview round-trip still INFRA-BLOCKED) |
| Reachable from shipped entry point? | YES — main.tsx:56 selectAppTransport() -> TauriTransport when `window.__TAURI_INTERNALS__` present (app-transport.ts:63-71); the shipped webview app registers the matching backend |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "tauri::command|generate_handler|lumen_dispatch" apps/desktop-tauri/src/app.rs` (L144-145 command, L293 registration); `rg` single_command.rs:39-61 (anti-bypass test); `rg -n "it\(" tauri-transport.test.ts` (11 tests); read of apps/desktop-tauri/INFRA-BLOCKED.md |

## 1. One-paragraph verdict
The real bridge client. Every UI command funnels through the single `lumen_dispatch` Tauri command (L143), typed rejections are normalized into the uniform error envelope (L150-151), unexpected IPC failures rethrow with cause instead of inventing responses (L153-155), and event subscription rides the `lumen://event` channel with correct subscribe-before-resolve/unsubscribe-after-resolve race handling (L160-182). The backend registration is REAL: apps/desktop-tauri/src/app.rs:144-145 defines `#[tauri::command] async fn lumen_dispatch` and L293 registers exactly one handler, enforced by tests/single_command.rs:39-61; INFRA-BLOCKED.md records the webview app shipping in every release payload (w30.8/w30.9). What remains unproven is the one thing the file itself flags: the `plugin:event|listen` frame shape + `{ kind: "Any" }` target (L93-98) is `[VERIFY at the ADR-004 admission build]` — no webview exists in this sandbox, and the launch-lane event-frame round-trip is still manual.

## 2. What actually works (verified)
— verified by tauri-transport.test.ts:81-212 (11 tests: single-command routing, verbatim response, typed-error normalization, rethrow-not-invent, event forwarding, unsubscribe races incl. release-exactly-once, listen-failure surfacing, out-of-shell typed error, internals binding for listen/unlisten) — all against a mocked TauriBackend (correct unit scope). Baseline vitest 1705 passed. Registration chain verified by rg against apps/desktop-tauri/src/app.rs (L144/293).

## 3. Findings
F-001 [S4]
Where: ui/src/client/tauri-transport.ts:93-98
Code:
```
          // [VERIFY at the ADR-004 admission build] the frame shape and the
          // "Any" default target are per Tauri 2.x api/event; unverifiable
          // without a webview here (INFRA-BLOCKED.md §unblock step 3).
```
What is wrong: the event-listen frame contract is assumed, not proven — if the shape is wrong, EVERY engine event silently never arrives (send still works; the UI would show commands that appear accepted with no state ever updating). The file marks it honestly; the launch lane that would clear it (tauri-launch.yml deep verdicts) remains manual per INFRA-BLOCKED.md §B.2. Confidence: HIGH (that it is unproven) — Verification: read + INFRA-BLOCKED.md:66-77.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isCommandError | 73-83 | REAL | — | structural guard on rejection payload |
| internalsBackend | 86-110 | REAL-UNPROVEN | S4 | F-001: plugin:event|listen shape unverified; unlisten by numeric id |
| detectTauriBackend | 113-118 | REAL | — | probe of __TAURI_INTERNALS__ |
| TauriTransport.send | 138-158 | REAL | — | single-command funnel; typed-error normalization; gen:0 documented as "unset" |
| TauriTransport.subscribe | 160-182 | REAL | — | race-safe subscribe/unsubscribe; failures surfaced not swallowed |
| createTauriTransport | 186-194 | REAL | — | typed unavailable error |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :82 routes every send through lumen_dispatch | YES | asserts command name + payload key |
| :104 typed rejection normalization | YES | envelope shape + request echo |
| :130 rethrows unexpected failures | YES | anti-fabrication guard |
| :163 unsubscribe-before-resolve releases once | YES | race regression |
| :212 internals binding (listen/unlisten ids) | YES | issue #3 regression |
All tests use a mocked seam — they prove the transport's contract, NOT the webview wire (correctly disclosed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the REAL CommandTransport that routes every UI-originated command through the shell's single Tauri command `lumen_dispatch`" (L1-4) | header | REAL | app.rs:293 single registration + test :82 |
| "The backend seam ([TauriBackend]) is injectable so unit tests never need a webview (this sandbox cannot run one)" (L11-13) | header | REAL | test file structure |
| [VERIFY] event frame shape (L93-94) | comment | UNVERIFIED (disclosed) | INFRA-BLOCKED.md §B.2 launch lane manual |

## 7. Dead / duplicated / unreachable code
None. `gen: 0` sentinel on the normalized-error path documented (L148-149).

## 8. Dependencies & flags
No @tauri-apps/api import (ADR-004: internals binding instead — dependency record docs/dependencies/records/tauri-wry-2.x.md). Generated wire types from src/generated/commands.ts. No feature flags.

## 9. Open questions for the next agent
- CRITICAL for the master question: if the event frame shape is wrong, the entire UI is a command blaster with no feedback. The desktop-tauri agent should confirm whether ANY automated lane exercises a real event round-trip today (tauri-launch.yml status). UNVERIFIED in this sandbox.
- Does the demo-transport (browser fallback) share the wire envelope exactly? (demo-transport.ts not in my assignment.)
