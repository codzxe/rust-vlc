# AUDIT: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 164 / 4 (EqualizerTool, applyPreset, commitFader, presetOptions + EqualizerState interface) |
| File verdict | REAL (code) — honest ghost: awaits `audio.set-eq` (absent from catalog, W8) and its persistence seam has NO consumer |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: mounted by ToolsDialog.tsx:122 (the player tools dialog, `lumen:open-tools` seam); EqualizerTool.test.tsx 12 behavioral cases; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `set-eq` in generated/commands.ts → 0 hits (absent from the CommandRequest union — W8 pin verified at this component); rg `EqualizerTool` consumers → ONLY ToolsDialog.tsx:36,122 + its test (effects.tsx:356 uses `GraphicEQ` directly); read ToolsDialog.tsx:110-128 (bare mount, wrapper note); read effects.tsx:322-356 (its own eqState, separate from this tool); read presets.ts:66-79 |

## 1. One-paragraph verdict
The component is a well-built, honestly-staged ghost: preset selector + 11 vertical faders, frame-true collapse grammar, 12 behavioral tests — and every control writes to LOCAL state that no backend and no persistence seam ever receives. `PENDING_COMMAND = "audio.set-eq"` names a command that does not exist in the generated catalog (verified — W8's pin, this is its UI half), the tool self-marks `data-staged-tool`/`data-pending-command` with an in-body note, and the sibling dialog adds a second staged note (documented transitional redundancy). The one finding unique to this file: the `onChange` "wire seam for the effects settings area" is dead — the effects area mounts GraphicEQ directly with its own state, and the only production consumer mounts this tool bare, so `onChange`, `defaultState`, and `className` are props no caller ever passes and every fader edit evaporates when the dialog closes.

## 2. What actually works (verified)
— verified by rg → `audio.set-eq` appears in ZERO lines of generated/commands.ts: the staged marker's claim ("no `audio.set-eq` command exists yet", l.47) is TRUE at HEAD.
— verified by reading l.98-112 → applyPreset/commitFader both `setState` + `onChange?.(next)` with the full state object — the local state machine is correct (unknown preset id degrades to NEUTRAL_GAINS, l.102).
— verified by reading presets.ts:66-79 → default preamp 12.0 dB, ±20 dB range, per-frame band labels — the values the tests assert ("preamp 12.0 dB, bands 0.0 dB", EqualizerTool.test.tsx:77).
— verified by reading EqualizerTool.test.tsx:16-97 → collapse/expand, named region, fader columns, chips, preset panel are behavioral (render + role/label assertions); would fail on regression.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:69-76 (and 85-122)
Code:
```
  /**
   * Fires on every committed change (fader commit, preset pick) with the
   * tool's full state — the wire seam for the effects settings area.
   */
  onChange?: (state: EqualizerState) => void;
```
What is wrong: The seam is dead. The ONLY production consumer mounts the tool bare — ToolsDialog.tsx:122 is literally `<EqualizerTool />` (no onChange, no defaultState, no className, not even defaultOpen). The effects settings area — the very consumer the comment names — does not use this component at all: effects.tsx:356 mounts `GraphicEQ` directly with its own `useState` (effects.tsx:322). So `onChange` never fires in any shipped path, `defaultState`/`className` are never passed, and every fader/preset edit lives exactly as long as the dialog.
Why it matters: The doc comment claims a wiring that does not exist (the "wire seam for the effects settings area" is a second, independent state island in effects.tsx). Two copies of EQ state machinery (this tool's and effects.tsx's) with zero shared persistence — the definition of the seam the comment says this is. Users tune an 11-fader EQ in the player dialog, close it, and lose everything.
What correct looks like: either the dialog lifts state (passes onChange and persists through the settings seam) or the component drops the misleading comment and the dead props until a consumer exists.
Confidence: HIGH — Verification: both consumer files read; rg sweep complete.
(Note: even with a consumer, the state could only reach localStorage at HEAD — the settings persistence dead-end, SettingField F-003.)

### F-002 [S4]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:152-161 (vs ToolsDialog.tsx:118-126)
Code:
```
      {open ? (
        <p className="eq-tool__staged" role="note">
          {eq("player.tools.equalizer.staged")}
        </p>
      ) : null}
```
What is wrong: The staged note exists TWICE for the dialog host: this in-body note (w26.k10) and the dialog wrapper's own note (ToolsDialog.tsx:123-125, same message key). equalizer.css suppresses this one inside the wrapper's `:has(.eq-tool--open)` scope — "until its owner deletes it" (l.155-156). The transitional pair ships both a rule to hide a note and the note it hides.
Why it matters: Self-acknowledged transitional debt with no expiry: the wrapper, the wrapper note, the tool note, and the suppression rule must be deleted together; any partial cleanup either double-renders the note or loses it.
What correct looks like: one note, one owner — delete the wrapper (its own module header says it is redundant now).
Confidence: HIGH — Verification: read both sites; the equalizer.css audit (sibling file) verified the `:has()` suppression rule exists.

### F-003 [S4]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:98-104
Code:
```
    const next: EqualizerState = {
      ...state,
      gains: spec === undefined ? NEUTRAL_GAINS : spec.gains,
      preset: id,
    };
```
What is wrong: An unknown preset id sets `preset: id` — a value not present in `presetOptions` — so the Dropdown's trigger would render an empty label (value not matching any option). Reachable only via `defaultState.preset` misuse (no production caller passes defaultState at all), hence hygiene.
Why it matters: The API accepts states it cannot render.
What correct looks like: coerce unknown ids to the no-preset id (or validate in the initializer).
Confidence: HIGH on the code path; LOW real-world reachability.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PENDING_COMMAND` | 45-49 | REAL | — | "audio.set-eq" — absent from catalog (verified); the honest-staging marker's claim TRUE |
| `EqualizerState` | 52-59 | REAL | — | preamp/gains/preset; docs match presets.ts defaults |
| `EqualizerToolProps` | 61-76 | REAL (dead seam) | S3 | F-001: onChange/defaultState/className have zero production callers |
| `EqualizerTool` (state) | 91-96 | REAL | — | local uncontrolled state, defaultState spread |
| `applyPreset` | 98-107 | REAL | S4 | F-003 unknown-id edge |
| `commitFader` | 109-112 | REAL | — | state + onChange pass-through |
| `presetOptions` | 114-122 | REAL | — | structural `data.preset` marker (css keys glyphs on it — verified by the sibling equalizer.css audit) |
| render (root + markers) | 124-151 | REAL | — | data-staged-tool/data-pending-command honest; preset dropdown only when open (frame-true) |
| staged note | 152-161 | REAL | S4 | F-002 transitional duplicate |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :17-25 collapsed ships "Open Equalizer" bar | YES | render + role assertions |
| :27-30 named Equalizer region hosted | YES | region query |
| :32-44 expand/collapse + defaultOpen | YES | behavioral toggle |
| :56-95 fader columns / chips / ±20 range | YES | label-verbatim + value-chip assertions |
| :97+ preset panel | YES | options from presets.ts |
| MISSING | — | no test asserts onChange fires with the full state (the dead seam F-001 is exactly why: nothing exercises the prop); no test covers defaultState |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no `audio.set-eq` command exists yet" | l.46-47 | REAL | rg generated/commands.ts → 0 hits (W8 pin) |
| "onChange … the wire seam for the effects settings area" | l.70-73 | FALSE | effects.tsx uses GraphicEQ directly; ToolsDialog mounts bare — F-001 |
| "all state LOCAL to the tool … the frozen w26.c1 contract" | l.14-15 | REAL | useState only; and with F-001 it stays local forever |
| "the tools dialog … and the settings Effects embed BOTH mount the A2 vertical-fader anatomy" | l.4-6 | REAL | both do — via different components (this tool vs GraphicEQ), which is how F-001 happened |
| "compose, don't duplicate" (ValueChip) | l.25-26 | REAL | chip composed; equalizer.css scopes anatomy (sibling audit verified) |

## 7. Dead / duplicated / unreachable code
Dead props: onChange/defaultState/className (F-001). Duplicated: the staged note (F-002) and, structurally, EQ state machinery (this tool vs effects.tsx's). Unreachable in shipped product (W14).

## 8. Dependencies & flags
Dropdown (+DropdownOption with `data`), presets (EQ_PRESETS/NEUTRAL_GAINS/DEFAULT_EQUALIZER_STATE), GraphicEQ, useEqMsg, equalizer.css. No transport import — the tool cannot even attempt a dispatch (the staging is honest about that). No feature flags.

## 9. Open questions for the next agent
- effects.tsx owner: does the settings Effects embed persist its own eqState anywhere (settings seam → localStorage), and does it also carry the audio.set-eq staged marker? If yes, the TWO EQ state islands should be unified behind this tool's seam.
- GraphicEQ.tsx owner: does its fader commit clamp to ±20 dB, and does the preamp fader map 12.0 dB to ≈82.6% as presets.ts:77-79 derives?
