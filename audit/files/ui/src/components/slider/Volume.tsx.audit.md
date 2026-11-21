# AUDIT: ui/src/components/slider/Volume.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 57 / 1 component (VolumeSlider) + props interface + formatPercent |
| File verdict | REAL as a component — but its ONLY shipped commit lane is refused by the host: `audio.set-volume` is unserved in the Rust dispatch table, so in the real desktop shell the slider can never move (permanently disabled); it works only on DemoTransport |
| Reachable from shipped entry point? | YES as web-UI source: ControlBar.tsx:477-484 (VolumeCluster) → PlayerScreen → app root; TauriTransport → `lumen_dispatch` (apps/desktop/src/shell.rs:163) → apps/cli/src/dispatch.rs `route()` → **`other => Err(unserved(other))`** for AudioSetVolume. No GUI binary exists at all (W14). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -n "VolumeSlider" src/` (sole consumer ControlBar:477); `rg -n "setVolume" src/` (context.tsx:389); `rg -n "SetVolume|set-volume" crates apps` (catalog HAS the command; NO execution handler anywhere: dispatch.rs route() lists session/playback/media-getinfo/tracks-list/engine-info/list-commands/config only, everything else `unserved`); `sed -n` verification of all quotes; baseline #23 (vitest 1705 pass incl. Volume.test.tsx 7 tests) |

## 1. One-paragraph verdict
The component itself is a clean thin wrapper: it pins the 0…1/0.01 horizontal grammar, merges the caller's className with `lumen-volume` (the w26.a04 clobber fix, regression-tested at Volume.test.tsx:96-101), forwards ref and rest, and keeps aria-label the caller's job. The hostile question for this wave — "is any UI element wired to a real backend?" — gets a decisive answer here: the control is wired to a REAL transport whose REAL backend refuses the command. The command catalog defines `audio.set-volume`, the UI dispatches it via TauriTransport to `lumen_dispatch`, and the host's route table never serves it — every volume commit in the shipped shell is answered `UnsupportedForHost`, `state.volume` never becomes non-null (both its sources — the `audio-volume` result and `state-snapshot` — require unserved commands), and ControlBar disables the slider (`disabled={volume === null}`). Volume works exclusively on the fake DemoTransport.

## 2. What actually works (verified)
— verified by Volume.test.tsx:37-111 (baseline #23, 1705 pass) → 0…1 range + percent text on the slider ARIA, keyboard live-then-commit order, Home/End clamping, pointer scrub mapping, compact anatomy + `--lumen-slider-fill: 25`, className-merge regression, disabled inertness.
— verified by read + cross-ref my Slider.tsx audit (file 15) → `toPercent` clamps the fill (interaction.ts:194), so an out-of-range paint cannot escape the track.
— verified by sed → className merge (:47) survives the rest spread because `className` is destructured out of rest; the same destructuring protects `value`/`disabled`/`onChangeLive`/`onCommit` from caller clobber.

## 3. Findings

### F-001 [S2]
Where: ui/src/components/slider/Volume.tsx:22 (onCommit contract) + apps/cli/src/dispatch.rs:154-155 (the lane it feeds)
Code:
```
  /** Fires on release / keyboard set — the value callers act on. */
  onCommit?: (value: number) => void;
```
```
        // ---- everything else: the honest typed refusal ----
        other => Err(unserved(other)),
```
What is wrong: the sole production consumer wires `onCommit={actions.setVolume}` (ControlBar.tsx:482), which dispatches `audio.set-volume` (context.tsx:390). The desktop shell forwards every command to `lumen_dispatch` (apps/desktop/src/shell.rs:163), whose `route()` serves only session/playback/media-getinfo/tracks-list/engine-info/list-commands/config — `Command::AudioSetVolume` falls into the `unserved` arm and is refused `UnsupportedForHost`. `state.volume` is non-null only from the `audio-volume` result (context.tsx:236-237, the response to set/get-volume) or `state-snapshot` (:286, SystemGetStateSnapshot) — both equally unserved — so in the shipped shell `volume` stays `null`, ControlBar renders the slider `disabled={volume === null}` (ControlBar.tsx:481), and the volume control is dead chrome. The only transport that serves the command is the DemoTransport simulation (demo-transport.ts:482).
Why it matters: the wave's core question — dead buttons — answered with a dead slider: a visible, fully-styled, tested control whose real-backend lane is a typed refusal, and whose only working lane is a fake. Users of the real shell can never change volume (or mute: `audio.set-mute` is equally unserved).
What correct looks like: serve `audio.set-volume`/`get-volume` (and `system.get-state-snapshot` to seed `volume`) in the host's route table, or render the control disabled-by-host-capability with a "not served by this host" affordance instead of relying on the null-seeded disable.
Confidence: HIGH — Verification: read of dispatch.rs:75-157 (full route table), shell.rs:153-167, context.tsx:233-298/389-391, ControlBar.tsx:449-487, demo-transport.ts:480-486; rg confirms zero `AudioSetVolume` execution handlers outside the catalog/policy layers.

### F-002 [S4]
Where: ui/src/components/slider/Volume.tsx:25
Code:
```
const formatPercent = (value: number): string => `${Math.round(value * 100)}%`;
```
What is wrong: no clamping of the incoming `value` before formatting (and Slider passes `shown` raw to `aria-valuenow`/`aria-valuetext`, Slider.tsx:92-93). A host reporting volume >1 (doc 05 range 0..1, but the catalog's `Volume(f32)` is range-unenforced — crates/commands catalog audit F-002) would produce `aria-valuenow="1.5"` (invalid per ARIA: must lie within valumin/valuemax) and "150%" text while the fill clamps to 100% — two surfaces disagreeing about one value.
Why it matters: currently unreachable (the real host never answers volume at all, F-001; DemoTransport clamps at demo-transport.ts:482), but the moment the command is served this becomes the honest-display path.
What correct looks like: clamp in the wrapper (`Math.min(1, Math.max(0, value))`) or in Slider before the ARIA write.
Confidence: HIGH (code read) / reachability LOW (blocked by F-001) — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VolumeSliderProps` | 15-23 | REAL | — | `Omit<…,"onChange">` correct (native onChange must not leak); all props consumed |
| `formatPercent` | 25 | REAL | S4 | F-002 no clamp; Math.round fine in-range |
| `VolumeSlider` | 27-55 | REAL | S2 | pins 0…1/0.01 horizontal grammar; className merge + destructured-prop clobber protection; ref forwarded; F-001 is the wiring verdict on its commit contract |
| `displayName` | 57 | REAL | — | forwardRef hygiene |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ARIA 0…1 + percent text (:37-46) | YES | attribute assertions incl. aria-valuetext "50%" |
| keyboard live-before-commit order (:48-63) | YES | exact call-order array |
| Home/End clamp (:65-73) | YES | commit args asserted |
| pointer scrub mapping (:75-85) | YES | pinned rect + clientX math |
| compact anatomy + fill CSS var (:87-94) | YES | class + inline-var assertions |
| className-merge regression (:96-101) | YES | pins the w26.a04 clobber fix |
| disabled inert (:103-111) | YES | no commit + aria-disabled |
| — gap | All 7 tests run against the raw component or DemoTransport-fed state; NONE can catch F-001 (dead lane) — that requires the real transport/host, which no committed UI test touches. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Engine-reported volume, 0…1 (doc 05 command layer range)" | Volume.tsx:16 | REAL-UNPROVEN | range matches doc 05 §2.6 table; but no shipped host ever reports volume (F-001) — the contract is doc-true, runtime-dead |
| "the value callers act on" (onCommit) | Volume.tsx:21-22 | SHALLOW | the one production caller's action leads to a typed refusal (dispatch.rs:155) |

## 7. Dead / duplicated / unreachable code
Nothing duplicated. The whole control is production-dead-end (F-001): rendered, styled, disabled forever in the shipped shell.

## 8. Dependencies & flags
Slider (file-15 audit), interaction.toPercent (clamped), demo-transport (fake lane). No feature flags.

## 9. Open questions for the next agent
- Slider.tsx:92-93 passes the RAW `shown` to `aria-valuenow`/`aria-valuetext` (only the CSS var goes through clamping `toPercent`) — my file-15 audit missed this; a Seekbar fed `value > duration` would publish invalid ARIA too. Fold into any Slider follow-up.
- If the host ever serves `system.get-state-snapshot`, check it seeds `volume` non-null (context.tsx:286) — that alone would un-disable the slider while commits still fail (worse UX than fully disabled: a movable slider whose writes all error).
