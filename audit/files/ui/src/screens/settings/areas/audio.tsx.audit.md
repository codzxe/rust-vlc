# AUDIT: ui/src/screens/settings/areas/audio.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 282 / 8 (AudioDeviceControl, AudioArea, defaultDevice, reasonFor, useAreaMsg, hintMsg, AUDIO_HINTS, consts) |
| File verdict | REAL (code) — Output-device capability has NO runtime handler anywhere (W8 class); unhandled rejection path |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: composed via areas/index.tsx (`<AudioArea/>` + EffectsArea per l.16-17), route /settings/audio; baseline rows 22-23 green; audio.test.tsx 14+ behavioral cases. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg `AudioListDevices` across crates/ + apps/ → ONLY catalog (crates/commands/src/catalog/{command,ids,types,spec}.rs) and playback-engine TEST hosts (scenarios_b/) — ZERO production handlers; rg `audio-devices` result kind → 0 producers; rg `audio.list-devices` in apps/desktop/src, apps/cli/src, apps/control-server/src → 0; read TauriTransport.send rejection path (tauri-transport.ts:144-155); read audio.test.tsx test list (l.63-181) |

## 1. One-paragraph verdict
The best-implemented dead control in the repo: the Output-device row is a textbook capability-backed dropdown — engine enumeration, default-selection, optimistic-apply-with-revert, honest disabled states with per-state reasons, an operable ⓘ — and 14 behavioral tests drive every state through mock transports. But the capability it backs does not exist: `audio.list-devices`/`audio.set-device` are catalog ENTRIES with no runtime handler in any host (shell, CLI, control-server — verified by grep), so in every buildable artifact the row renders permanently disabled at "System default": the same UI-wired-to-nothing class as W8's audio.set-eq. Additionally, both `client.send` calls lack `.catch()`, so a transport-level rejection (which TauriTransport throws for non-command rejections) leaves an unhandled promise rejection and a row stuck on "loading" forever.

## 2. What actually works (verified)
— verified by reading l.128-153 → the enumeration state machine: completed+audio-devices → ready; empty → no-devices reason; error → reasonFor (feature-not-exposed distinguished); accepted-no-payload → honest stand-in. All four outcomes honest.
— verified by reading l.155-167 → optimistic selection with revert ONLY on error results (functional update guards against stale overwrites); `id === query.selected` no-op dispatch guard.
— verified by reading l.254-260 → device-row splice: if `audio.enable` is feature-gated out, `findIndex` returns −1, `deviceAt` becomes 0, and the hand row renders FIRST — a sane degradation, not a crash or a dropped row.
— verified by reading audio.test.tsx:63-181 → enumeration, default pre-select, set-device dispatch args, failed-set revert, disabled-with-reason (feature-not-exposed, pending, empty, non-feature) all asserted behaviorally; the disabled control asserted to dispatch nothing.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/areas/audio.tsx:130-131, 161 (the capability seam)
Code:
```
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.list-devices" })
```
What is wrong: The command the control is built around has NO production handler. rg `AudioListDevices` matches only the command CATALOG (crates/commands/src/catalog/{command.rs:197, ids.rs:121, types.rs, spec.rs}) and playback-engine TEST hosts (scenarios_b/host.rs, scenario_12_device_loss.rs); apps/desktop/src, apps/cli/src, apps/control-server/src contain zero references, and no code anywhere produces the `audio-devices` completed payload the UI unpacks at l.135. The schema entry itself says the command is "(S)"-class with "exactly one pseudo-device on web" — a web behavior that also does not exist in this tree.
Why it matters: The Output device row — the flagship "capability-backed hand row" of the audio area (l.19-26) — can never leave its disabled state in any buildable artifact. The UI handles the absence honestly (feature-not-exposed → reason → disabled), so no user is deceived at runtime, but this is the third instance of the W8 pattern (UI seams awaiting backends that were never written): settings rows that are real components around a permanently absent engine feature.
What correct looks like: an engine-side handler for AudioListDevices (even the honest single-pseudo-device answer the schema describes), or the row demoted out of the frame composition until the capability lands.
Confidence: HIGH — Verification: rg sweeps across all host crates; catalog-only matches enumerated.
Filed S3 (honest inert control; the UI code itself is correct).

### F-002 [S3]
Where: ui/src/screens/settings/areas/audio.tsx:128-153 and 160-166
Code:
```
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.list-devices" })
      .then((response: CommandResponse) => {
        if (cancelled) return;
```
What is wrong: Neither `client.send(...)` chain has a `.catch()`. TauriTransport.send THROWS for non-CommandError rejections (`throw new Error("lumen_dispatch IPC failed: …")`, tauri-transport.ts:153-155). On that path: (1) the rejection is unhandled (console noise, potential crash under strict handlers); (2) the enumeration effect never calls setQuery — the row stays `{status: "loading"}` forever, disabled with the "loading" reason; (3) the set-device path (l.160-166) has the same hole — a transport failure neither reverts the optimistic selection nor surfaces anything.
Why it matters: "Transport down" is the one failure mode every honest-UI seam must survive; this one hangs a row in a spinner-less "loading" state with no recovery path (no retry, no reason update). The file's own honesty doctrine (l.19: "config-never-invents", staged-inert grammar) is violated by its least-tested branch.
What correct looks like: `.catch(() => { if (!cancelled) setQuery({status: "unavailable", reason: "settings.capability-unavailable"}); })` on both chains (and the same revert for set-device).
Confidence: HIGH — Verification: read both chains + tauri-transport.ts rejection path. The MockTransport used in tests never rejects (transport.ts:63-87), which is exactly why no test catches this.
Mutation check (reasoned): make the test transport reject → currently no test fails (the suite's transports are all resolving mocks) — the gap is the proof.

### F-003 [S4]
Where: ui/src/screens/settings/areas/audio.tsx:174-180
Code:
```
      ? devices.map((device) => ({
          value: device.id,
          labelKey: device.isDefault ? `${device.name} — ${m("settings.audio.device-default")}` : device.name,
        }))
```
What is wrong: `labelKey` again receives a resolved label (device name, or a composed `name — default` string) — the same identity-resolver seam lie as SettingField.tsx:239-242. Works only because Dropdown's default resolver is identity.
Why it matters: A future host wiring a real resolver into this Dropdown double-resolves device names; the prop name keeps misleading.
What correct looks like: a `label` prop on options.
Confidence: HIGH — Verification: read Dropdown.tsx:123 (identity default).

### F-004 [S4]
Where: ui/src/screens/settings/areas/audio.tsx:86
Code:
```
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;
```
What is wrong: A double cast (`as string as MessageKey`) that erases both key-set membership checks: any i14 key compiles, whether or not the message catalog registers it. If a msgid is unregistered, the area-msg fallback chain renders the raw key as the user-visible hint.
Why it matters: The cast exists to paper over the i18n catalog/keys split (the "pre-registration seam") — the same W11 localization-orphan class; TypeScript could enforce registration but is told not to look.
What correct looks like: derive MessageKey from the i14 catalog type, or register the keys in the catalog and drop the cast.
Confidence: HIGH — Verification: read; fallback chain in makeAreaMsg (shared.tsx) ends in the raw key.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| wire types (Completed/AudioDevicesResult/AudioDevice) | 56-60 | REAL | — | derived through the transport seam (generated types not imported directly — the stated rule) |
| `DeviceQuery` | 63-66 | REAL | — | exhaustive 3-state machine, reason always present when unavailable |
| `defaultDevice` | 68-69 | REAL | — | isDefault → first → "" (unreachable empty; length 0 handled upstream) |
| `reasonFor` | 71-75 | REAL | — | feature-not-exposed distinguished from generic |
| `SYSTEM_DEFAULT`/`I14Key`/`useAreaMsg` | 77-83 | REAL | — | makeAreaMsg factory dedup (j13 ruling) |
| `hintMsg` | 86 | REAL | S4 | F-004 double cast |
| `AUDIO_HINTS` | 90-100 | REAL | — | 9 of 10 frame ⓘ rows mapped; Maximum-volume honestly omitted (l.88-89) |
| `AudioDeviceControl` (enumeration effect) | 128-153 | REAL | S3 | F-001 (no handler), F-002 (no catch); cancelled-flag cleanup correct |
| `chooseDevice` | 155-167 | REAL | S3 | F-002 (no catch); optimistic+revert correct for error RESULTS |
| render (row/ⓘ/reason/dropdown) | 195-236 | REAL | — | data-staged marker, role=note reason, disabled gated on !ready; operable ⓘ (j13 P2-3) |
| `AudioArea` | 246-280 | REAL | — | schema-driven sections + frame-order splice; findIndex−1 degradation sane |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| audio.test.tsx:64-77 enumeration → fixture devices | YES | asserts rendered options from a mock transport's audio-devices payload |
| :79-87 default pre-select | YES | asserts selected value |
| :89-102 set-device dispatch args | YES | asserts the exact command + id |
| :103-117 failed set reverts | YES | the truthfulness contract asserted |
| :119-180 disabled-with-reason ×5 | YES | feature-not-exposed / pending / empty / non-feature / inert-no-dispatch all behavioral |
| :181+ AudioArea sections | YES | seed-driven composition |
| MISSING | — | no test makes the transport REJECT (F-002's hole); no test can detect the absent engine handler (F-001) — that is a cross-layer fact, correctly outside a unit suite's reach but recorded nowhere in the repo's evidence |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Output device is a CAPABILITY-BACKED control: audio.list-devices / audio.set-device feed it from engine enumeration" | l.19-21 | FALSE (no enumeration exists) | F-001: zero handlers in any host; the schema itself describes an unimplemented "(S)" web behavior |
| "When the capability is absent … DISABLED WITH THE HONEST REASON" | l.22-26 | REAL | implemented + tested; and at HEAD this is the ONLY state the control can ever be in |
| "the frame hangs a ⓘ circle on 10 of the 11 rows" | l.34-36 | REAL | AUDIO_HINTS maps 9 schema rows + the hand row's own ⓘ = 10; Maximum-volume omitted honestly |
| "Optimistic accept; a failed apply reverts" | l.158 | REAL for error results / HOLE for rejections | l.155-167 + F-002 |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Unreachable capability: the enumeration path (F-001). Duplication: labelKey-as-label pattern (F-003, third occurrence repo-wide after SettingField and Dropdown hosts).

## 8. Dependencies & flags
Dropdown, useClient, transport types, i18n key catalogs (gfixB, i14), SettingField + SettingInfoDot, shared (makeAreaMsg, schemaSectionViews), audio.css. No feature flags read here (feature gating flows through schemaSectionViews/SettingField's featureExposed). Product.toml [features] audio-device-enumeration is documented (l.22-24) — its value only chooses WHICH honest-disabled variant renders.

## 9. Open questions for the next agent
- areas/index.tsx + effects.tsx owners: does EffectsArea carry the same awaiting-backend pattern (audio.set-eq per W8) with the same missing .catch holes?
- Whoever owns the command catalog: AudioSetDevice is documented "class A, no gen" — confirm no generated wire path exists either (my grep suggests none).
