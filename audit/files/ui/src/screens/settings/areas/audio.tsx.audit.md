# AUDIT: ui/src/screens/settings/areas/audio.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 282 / 5 exported+inner (defaultDevice, reasonFor, useAreaMsg factory, AudioDeviceControl, AudioArea, hintMsg) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — App route /settings/audio -> SettingsScreen -> AudioArea (areas/index.tsx registry) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n` on audio.test.tsx (12+ tests incl. dispatch/revert/refusal); `sed -n 175,182p` + `od -c` SettingField.tsx:178 (syntax OK); `rg` hint-key registration in messages.ts (registered L1337-1340, 1924-1927); read of shared.tsx/persistence.ts/config-settings.ts/SettingsScreen.tsx:461,490 |

## 1. One-paragraph verdict
The most genuinely-wired settings surface in my assignment: AudioDeviceControl sends the real `audio.list-devices` on mount and `audio.set-device` on selection, with optimistic-accept + error-revert, honest disabled-with-reason states for every failure mode (loading, feature-not-exposed, no-devices, generic error), and 12+ tests asserting the dispatches and the refusal rendering. The schema rows (audio.enable … replay-gain-preamp) render from the GENERATED settings seed (generated/schema.ts, "do not edit", drift-guarded by npm run settings:check) and persist through SettingsScreen's injected persistence — the desktop shell's real `config.set-setting` bridge (SettingsScreen.tsx:461), localStorage only in the browser dev shell. One defect: neither send has a `.catch` — a rejecting transport leaves the row stuck in "loading" forever plus an unhandled promise rejection.

## 2. What actually works (verified)
— verified by audio.test.tsx:64-180 (queries engine, default preselect, set-device dispatch with real id, failed-set revert, feature-not-exposed disabled-with-reason, inert-when-disabled, loading→final reason, empty enumeration, non-feature errors) and SettingsScreen.tsx:461/490 (desktop = createConfigSettingsPersistence over the client bridge; browser = localStorage mock, honestly disclosed in persistence.ts header). Baseline vitest 1705 passed.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/audio.tsx:130-149
Code:
```
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.list-devices" })
      .then((response: CommandResponse) => {
        if (cancelled) return;
```
What is wrong: no `.catch` on either send (also L160-166 `chooseDevice`). If the transport rejects (desktop shell host unreachable), the promise rejects unhandled and `setQuery` never runs — the Output device row stays `status: "loading"` with the loading reason indefinitely, never reaching an honest unavailable state. Why it matters: this control's entire design is honest-failure states; the one failure mode it cannot represent is the transport itself dying. What correct looks like: `.catch(() => setQuery({ status: "unavailable", reason: "settings.capability-unavailable" }))`. Confidence: HIGH — Verification: read (both call sites); no test covers a rejecting transport (test matrix uses resolving fakes only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| defaultDevice | 68-69 | REAL | — | isDefault ?? first ?? "" |
| reasonFor | 71-75 | REAL | — | feature-not-exposed → exact reason |
| useAreaMsg (makeAreaMsg) | 80-83 | REAL | — | catalog-first seam, fallback to lane keys |
| AudioDeviceControl | 120-238 | REAL | S3 | F-001 missing catch; data-capability attrs; operable ⓘ (j13 P2-3 fix) |
| AudioArea | 246-280 | REAL | — | generated seed order; hand row spliced after audio.enable (frame order) |
| hintMsg + AUDIO_HINTS | 86-100 | REAL | — | all 9 hint keys registered in messages.ts (verified L1337+) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :64 enumeration query renders fixture | YES | asserts cmd sent + options rendered |
| :89 set-device dispatch with engine id | YES | payload assertion |
| :103 failed set-device reverts | YES | truthful accepted-vs-completed |
| :119 feature-not-exposed disabled+reason | YES | the config-never-invents AC |
| :166 empty enumeration no invented devices | YES | honest-empty |
| :182 every schema row bound to key | YES | seed-order regression |
Gap: rejecting-transport case untested (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Output device is a CAPABILITY-BACKED control: audio.list-devices / audio.set-device feed it from engine enumeration — never invented options" (L20-22) | header | REAL | L131/L161 sends; tests :64-:103 |
| "When the capability is absent ... DISABLED WITH THE HONEST REASON" (L22-26) | header | REAL | test :119/:148 |
| "the generated section data stays the source of truth" (L5) | header | REAL | schemaSectionViews reads generated/schema.ts (1507+ entries verified) |

## 7. Dead / duplicated / unreachable code
None — all rows live, including the hand row (interleaved at frame order, invisible to search by design, L102-105).

## 8. Dependencies & flags
Generated settings seed (settings/generated/schema.ts — card 4.E1, schemas/settings.toml + product.toml feature exposure, drift-guarded). Feature gate `audio-device-enumeration` from product.toml (comment L23). This is the UI's generated-vs-handwritten boundary done right: seed + hand row + drift check.

## 9. Open questions for the next agent
- Does the ENGINE implement audio.list-devices with real device enumeration (crates/audio-output)? UI-side real; engine-side UNVERIFIED here.
- product.toml [features] audio-device-enumeration — is it true in the release lane? If false, this control ships permanently disabled on desktop (honest, but dead-by-config — worth a doc row).
