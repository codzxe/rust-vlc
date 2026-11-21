# AUDIT: ui/src/screens/settings/areas/playback.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 103 / 3 |
| File verdict | SHALLOW (fully live-looking schema-driven settings that persist to a store nothing consumes — write-only settings with no staged markers) |
| Reachable from shipped entry point? | YES (web app build): /settings/playback route (areas/index.tsx registry; areas.test.tsx asserts the area mounts). Baseline #22 build OK, #23 vitest 1705 passed incl. areas.test.tsx (the 13-area registry + the playback ⓘ wiring case at 166-176). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `Read` playback.tsx 1-103 (complete); `rg -ln "playback.start-paused|playback.resume|playback.seek-precisely|playback.repeat-mode" src --glob '!**/settings/**' --glob '!**/onboarding/**' --glob '!**/*.test.*'` (ONLY i18n files — zero behavior consumers); `rg -n "start-paused|startPaused" src/client src/screens/player` (demo-transport reads the OPEN-REQUEST option `args.options?.startPaused`, which openPathRequest never sets from settings); `head -30 src/client/config-settings.ts` (the "REAL write path": config.* over the command transport — desktop shell only, which cannot compile per W14-A01; the demo transport answers feature-not-exposed for config.*); `rg -ln "PlaybackArea" src --glob '*.test.tsx'` + areas.test.tsx inspection (registry + ⓘ coverage). |

## 1. One-paragraph verdict
The most dangerously convincing surface in my settings sample: unlike effects.tsx (staged, disabled, marked) this area renders fully LIVE schema-driven SettingFields — toggles, sliders, dropdowns, ⓘ hints, working persistence into `lumen.settings.overrides.v1` — and every one of them is write-only. No code outside the settings/onboarding screens reads a single `playback.*` key: the demo transport's start-paused behavior keys off the open-request's `options.startPaused`, which the ONE open funnel never populates from settings; the "REAL write path" (config-settings.ts, `config.set-setting` over the transport) exists but targets the desktop shell that cannot compile at HEAD (W14-A01), and the demo transport honestly answers `feature-not-exposed` for config.*. So "Start playing as soon as a file opens" toggles, persists, and changes nothing — with no staged marker anywhere to say so. This is W11's "settings never persist (to any behavior)" instantiated as the product's most central settings page.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): the area mounts on /settings/playback, the 13-area registry includes it, and the ⓘ wiring case (areas.test.tsx:166-176) asserts 12-of-13 rows carry the hint with the slider row honestly bare.
— Verified by read: `schemaSectionViews("playback", { hiddenKeys })` composition is correct — hidden rows (preferred languages, subtitle-mode) stay live for onboarding and cross-area search per the comment; empty sections render null; keys are stable.
— Verified by read: PLAYBACK_HINTS covers exactly the 12 frame rows; SettingField receives the hint through its prop contract.
— Verified by read: the persistence round-trip itself works in the browser (localStorage overrides store — my ThemeProvider audit chain), so the toggles genuinely hold their values across reloads. They just drive nothing.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/areas/playback.tsx:83-101 (the live area); evidence chain: `rg` over src for the four headline keys (only i18n hits), demo-transport.ts:411 (open-request option, settings-independent), open-path.ts:58-69 (session.open args carry no options), config-settings.ts:1-8 (the engine bridge — desktop-only, unbuildable)
Code:
```tsx
      {schemaSectionViews("playback", { hiddenKeys: PLAYBACK_HIDDEN_KEYS }).map(
        ({ section, visible }) => {
          if (visible.length === 0) return null;
```
What is wrong: the area renders live, enabled, schema-driven controls with zero staged markers, but no buildable configuration consumes any of them. (1) Browser (the only buildable shell): the overrides store persists the values, and `rg` proves no UI/client code outside settings/onboarding reads any `playback.*` key — the demo transport's paused-on-open behavior derives from `request.args.options?.startPaused` (demo-transport.ts:411), an option the ONE open funnel (open-path.ts) never sets. (2) Desktop (the intended consumer via config-settings.ts → `config.set-setting`): the host crate cannot compile at HEAD (W14-A01), and even mounted, the ENGINE side of that bridge is the W11 finding's subject (settings never persist to behavior). The module's own comment calls the hidden rows "REAL, LIVE settings" — they are real, live *storage*, dead *behavior*.
Why it matters: wrong behavior in the most realistic case possible: a user sets "Start playing as soon as a file opens" → OFF, opens a file, playback starts anyway; sets Repeat → "All", playback does not repeat; "Seek precisely" toggles nothing in the seek path. Every control on the page silently lies by omission — worse than effects.tsx, whose controls at least render disabled with reasons.
What correct looks like: either wire the bridge (map the persisted overrides onto openPathRequest's `options` for the browser shell — startPaused/startPosition are right there in the open options — and onto `config.set-setting` for the desktop), or mark the area staged with the data-pending-command grammar the sibling areas use, so the page stops pretending.
Confidence: HIGH — Verification: read (consumer search exhaustive over src; the open-funnel args traced; the config bridge's shell-gating read; W14/W11 cross-cited).

### F-002 [S4]
Where: ui/src/screens/settings/areas/playback.tsx:43-44
Code:
```tsx
/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;
```
What is wrong: the double cast (`as string as MessageKey`) is the most aggressive variant of the lane-msg pattern I have audited (ToolsDialog F-002, MusicMode F-002) — it launders an unregistered lane key into the central `MessageKey` type at compile time, relying on `useMsg`'s runtime fallback to resolve it.
Why it matters: same silent-break risk as the sibling findings, maximally un-typechecked; if the lane key and the registered key ever diverge in spelling, the hint renders `undefined`-adjacent output with no compiler help.
What correct looks like: the makeAreaMsg factory shape (settings/areas/shared) which the effects area already uses, or a `MessageKey | LaneKey` union on SettingField's `hint` prop.
Confidence: HIGH — Verification: read (pattern compared across four files).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `hintMsg` | 44 | REAL | F-002 | Double-cast lane seam. |
| `PLAYBACK_HINTS` (const) | 48-61 | REAL | — | 12-of-13 rows per frame circle extraction; matches areas.test.tsx:166-176. |
| `PLAYBACK_HIDDEN_KEYS` (const) | 70-74 | REAL | — | Three rows hidden from THIS composition, live for onboarding/search (verified: onboarding writes playback.subtitle-mode — OnboardingScreen.tsx:482). |
| `PlaybackArea` | 83-102 | SHALLOW | F-001 | Schema-driven live rendering; write-only settings; no staged markers. |

## 5. Test honesty (areas.test.tsx — the coverage this area has; ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 13-area registry (43-60) | YES | The area id set is pinned. |
| playback ⓘ wiring (166-176) | YES | 12 info dots + the slider row's absence asserted. |
| NOT covered: any behavior assertion for a playback setting | — | No test opens media with `playback.start-paused` set and asserts paused start — the write-only gap (F-001) is invisible to the suite, which is exactly why it ships. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the hidden rows below stay REAL, LIVE settings (the onboarding wizard writes them; the shell's cross-area search still finds them)" (29-32) | this file | REAL (storage) / FALSE (behavior) | Onboarding writes them (OnboardingScreen.tsx:482); no consumer reads them — F-001's evidence chain. |
| "the screen stays schema-driven: sections/entries/order come from the seed" (22) | this file | REAL | schemaSectionViews composition read + registry test. |
| "Default playback speed → slider (fill 139/320, 6 ticks)" (frame truth, 7-8) | this file | REAL | Rendered via SettingField from the schema; ⓘ absence test-pinned. |

## 7. Dead / duplicated / unreachable code
- No dead code in the file; the deadness is one level up (the settings' behavior, F-001).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- SettingField, schemaSectionViews (shared areas layer), i14 lane keys. No feature flags. The schema seed (`generated/schema.ts`) is the data source — cliFlag "--playback-start-paused" exists in the schema (line 1002) as documentation of the CLI flag the engine exposes; the UI does not consume it either way.

## 9. Open questions for the next agent
- The schema seed's `cliFlag` fields imply the ENGINE has a config layer for these keys — whoever audited the engine config (W11) can confirm whether `config.set-setting` would even reach playback behavior, or whether the write-only gap is engine-side too.
- open-path.ts owner (w15-g003): F-001's minimal browser fix is one function — map `playback.start-paused`/`playback.resume` overrides onto `session.open`'s `options` (the demo transport already honors `options.startPaused` at :411).
