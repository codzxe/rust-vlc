# AUDIT: ui/src/screens/settings/areas/effects.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 397 / 7 (useEffectsMsg, stagedNote, StagedToggleRow, StagedDropdownRow, PresetRow, StagedNote, EffectsArea) |
| File verdict | SHALLOW (interactive-looking local state, zero backend wiring by design) |
| Reachable from shipped entry point? | YES — App.tsx:322 `/settings/:area` -> SettingsScreen -> areas/index.tsx registry (imports EffectsArea; audio area embeds the Effects section) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg for `audio.set-eq` across player tools (EqualizerTool.tsx:47-49 same pending marker); read of full file; grep GraphicEQ usages |

## 1. One-paragraph verdict
The Effects settings area is a staged diorama: four rows (Equalizer enable, Two-pass, Spatial audio, Stereo widening) are disabled switches/dropdowns with empty onChange handlers; the "Equalizer preset" row and the embedded GraphicEQ fader panel are fully INTERACTIVE but write only React local state — no transport call exists anywhere in the file, and no `audio.set-eq` command exists in the product (the player's own EqualizerTool.tsx:47-49 carries the identical `PENDING_COMMAND = "audio.set-eq"` staged marker). So a user can drag EQ faders and pick presets that change nothing about audio. Disclosure is real (ⓘ notes, `data-pending-command` markers, a section-level staged note) but weaker than the subtitles area: these controls LOOK live because they respond to input.

## 2. What actually works (verified)
— verified by rg: no `client.send`/`useClient`/transport import in this file; `audio.set-eq` absent from the command surface (equalizer lane carries the same pending marker). Local state machine (preset -> gains reshape) works as UI-only behavior.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/effects.tsx:318-331 and 349-357
Code:
```
  // The EQ state is AREA-LOCAL: the preset row and the fader panel share it
  // (the preset pick re-shapes the faders; a fader commit keeps the last
  // applied preset label — VLC behavior). Never a dead control, never an
  // engine write (the staged markers name the awaited `audio.set-eq`).
  const [eqState, setEqState] = useState<EqualizerState>(DEFAULT_EQUALIZER_STATE);
```
and
```
          <GraphicEQ defaultOpen state={eqState} onChange={setEqState} />
```
What is wrong: the comment claims "Never a dead control" — but a fader panel that accepts drags and writes zero engine state IS a dead control wearing an interactive costume; the honest disclosure lives in data attributes and an ⓘ tooltip, not in the control's behavior. No `audio.set-eq` command exists anywhere (EqualizerTool.tsx:47-49), so EQ settings cannot reach the engine's Eq3Band stage from ANY surface. Why it matters: master-question census — /settings effects rows and the EQ panel are 0% wired; users can "tune" a graphic equalizer that does nothing. What correct looks like: disabled faders (subtitles-area pattern) until the command lands. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useEffectsMsg | 83 | REAL-UNPROVEN | — | makeAreaMsg factory; keys registered |
| stagedNote | 86-87 | REAL-UNPROVEN | — | resolves i18n key |
| StagedToggleRow | 100-159 | STUB | S3 | disabled switch, empty onChange, honest ⓘ |
| StagedDropdownRow | 172-233 | STUB | S3 | single canned option, disabled |
| PresetRow | 243-296 | SHALLOW | S3 | interactive dropdown -> local state only |
| StagedNote | 303-309 | REAL-UNPROVEN | — | visible disclosure line |
| EffectsArea | 316-397 | SHALLOW | S3 | composes staged rows + local-only EQ (F-001) |

## 5. Test honesty
No dedicated effects.test.tsx found in the area dir during scan (tests for the EQ live in the equalizer lane). The area's zero-transport contract is untested here — weaker than subtitles.test.tsx's explicit assertion. UNVERIFIED: whether a test elsewhere covers EffectsArea (rg `EffectsArea` in test files not run exhaustively).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "interactive LOCAL state ... never a dead control" | L11-12, L320 | MISLEADING | local-only with no consumer = dead effect; see F-001 |
| "No schemas/settings.toml entries exist for effects yet" | L46-47 | REAL (consistent) | consistent with generated schema absence; settings lane owns final proof |

## 7. Dead / duplicated / unreachable code
None beyond the staged rows themselves.

## 8. Dependencies & flags
GraphicEQ + EQ_PRESETS from player tools/equalizer; form/menu primitives; makeAreaMsg; effects.css. No feature flags.

## 9. Open questions for the next agent
- EqualizerTool.tsx (player Tools dialog) same staging — that lane's audit should confirm no dispatch path exists.
- Does the engine's Eq3Band stage get configured by ANY non-UI path (profile/config file)? (Rust settings lane)
