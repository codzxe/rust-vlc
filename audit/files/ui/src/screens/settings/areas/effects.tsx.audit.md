# AUDIT: ui/src/screens/settings/areas/effects.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 397 / 7 |
| File verdict | SHALLOW (honest inert staging: every real control disabled awaiting commands that exist nowhere; the one interactive control is local-only state, discarded on unmount) |
| Reachable from shipped entry point? | YES (web app build): settings router → `areas/index.tsx:15,165` mounts `<EffectsArea />` (the Audio settings area's Effects section, composed by audio.tsx per its header note). Baseline #22 build OK, #23 vitest 1705 passed incl. effects.test.tsx (12 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `Read` effects.tsx 1-397 (complete); `rg -n "set-eq" src/generated/commands.ts` (NO MATCH — `audio.set-eq` absent from the command catalog); `rg -n "audio.set-eq" crates/commands/src -l` (NO MATCH — no such command in the commands crate); audit-runs cross-refs: W07-A06 log F-001 ("Eq3Band constructed by nothing in any shipped build; UI EQ staged-inert awaiting nonexistent audio.set-eq; matrix row 'partial' over-credits"), W8 prior (audio.set-eq awaits nonexistent backend), W11 (settings never persist); `rg -n "describe|it(" effects.test.tsx` (12 behavioral cases incl. "staged controls never send anything"); `rg -n "EffectsArea" areas/index.tsx audio.tsx` (mount chain); `sed -n` verification of quoted ranges. |

## 1. One-paragraph verdict
The most disciplined inert surface in the repo — and still a surface where nothing works: every toggle and the mode dropdown render disabled with `data-pending-command` markers naming commands (`audio.set-eq`, `audio.set-spatializer`, `audio.set-stereo-widener`) that do not exist in the generated command catalog, the commands crate, or any backend (W7-A06: the engine's Eq3Band stage is "constructed by nothing in any shipped build"; W8: the awaited backend does not exist). The two "interactive" controls — the preset dropdown and the embedded GraphicEQ fader panel — drive an area-local `useState` that is discarded the moment the area unmounts: a user who shapes an EQ, picks a preset, and returns to the player has configured nothing and saved nothing. The honesty apparatus is real (staged markers, operable ⓘ reveals, one section note, and a test that proves "staged controls never send anything"), which keeps this out of fraud territory — but the product behavior is: Settings → Audio → Effects is a picture of a settings page. Cross-cutting pins already on record: W11 (settings never persist), W7/W8 (the awaited command/backend never existed).

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 12 effects.test.tsx cases — section header, expanded GraphicEQ embed, preset row outside the panel, 8 presets, preset→fader reshaping (shared state), six ⓘ affordances, staged-inert rows naming their commands, and the "staged controls never send anything" no-dispatch assertion.
— Verified by read: `applyPreset` (324-331) correctly preserves the rest of the EQ state (spread) and falls back to NEUTRAL_GAINS for an unknown id; the preset row and the panel genuinely share one state object (the test at :90 pins the reshaping).
— Verified by read: the staging grammar is consistent — `data-setting-key="pending.<cmd>"`, `data-staged-row`, `data-pending-command` on every inert row and the panel wrapper; the ⓘ (SettingInfoDot) is operable with aria-expanded and a caption reveal.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/areas/effects.tsx:316-331 (local state), 340-390 (five inert rows), 349-357 (panel wrapper)
Code:
```tsx
  // The EQ state is AREA-LOCAL: the preset row and the fader panel share it
  // (the preset pick re-shapes the faders; a fader commit keeps the last
  // applied preset label — VLC behavior). Never a dead control, never an
  // engine write (the staged markers name the awaited `audio.set-eq`).
  const [eqState, setEqState] = useState<EqualizerState>(DEFAULT_EQUALIZER_STATE);
```
What is wrong: the entire Effects area is behaviorally a toy. (1) All five staged rows await commands that exist nowhere: `audio.set-eq` / `audio.set-spatializer` / `audio.set-stereo-widener` return zero hits in `ui/src/generated/commands.ts` AND zero hits in `crates/commands/src` — the awaited catalog command was never added on any side; W7-A06 additionally proved the engine's Eq3Band is constructed by nothing in any shipped build. (2) The one interactive path — preset pick → fader shape — writes only to the area-local `useState` above: navigating away from Settings discards the configuration with no persistence, no warning, and no effect on playback. A user who "sets an equalizer" here has set nothing.
Why it matters: realistic wrong behavior (S2): the surface presents itself as the Effects settings (frame-faithful rows, real dropdown, real faders) and silently discards every interaction. The staging markers are visible in the DOM and the ⓘ reveals say so, but the primary affordances (a working preset dropdown, draggable faders) invite configuration that cannot survive the screen. This is the W11 "settings never persist" defect instantiated in the Effects area, compounded by the W8 "awaits a nonexistent backend" defect.
What correct looks like: at minimum persist the EQ state through the settings seam (the settings store other areas use — which itself never persists per W11, so the honest fix is the awaited `audio.set-eq` command + backend wiring); until then, the fader panel should carry the same disabled truth as its siblings instead of an interactive local toy.
Confidence: HIGH — Verification: read (state scope traced; command absence verified by rg on both the generated catalog and the commands crate; prior waves W7-A06/W8/W11 cross-cited from their logs).

### F-002 [S3]
Where: ui/src/screens/settings/areas/effects.tsx:172-233 (StagedDropdownRow), 376-383 (the one call site)
Code:
```tsx
          <Dropdown
            options={[{ value: optionKey, labelKey: optionKey }]}
            value={optionKey}
            onChange={() => {
              /* staged — disabled means never fired */
            }}
```
What is wrong: the Spatial-audio-mode row is a disabled dropdown whose option list is a single hard-coded frame value ("headphone-virtualization") — it cannot represent spatializer modes that a real backend would expose (binaural, room shapes, speaker layouts), and its `onChange` is a dead closure. The comment defends this as "no invented engine modes" — correct for honesty, but the row is a picture of a dropdown rather than a disabled control awaiting data: when the command lands, this row needs a rewrite, not an enable.
Why it matters: the pattern difference matters for maintenance: the staged TOGGLES await only a command (flip `disabled`), while this row awaits a command AND an option model; nothing in the row's structure records that second dependency, so the enable-flip will silently ship a one-mode spatializer.
What correct looks like: model it like the preset row — options sourced from a shared list (even a one-entry staged list in one place), with the second dependency named in the data-pending marker.
Confidence: MEDIUM — Verification: read (structure traced; future-rewrite claim is reasoned, not executed).

### F-003 [S4]
Where: ui/src/screens/settings/areas/effects.tsx:100-115 (StagedToggleRow props)
Code:
```tsx
function StagedToggleRow({
  labelKey,
  pendingCommand,
  info = false,
  note,
  kind,
}: {
```
What is wrong: `note` is a REQUIRED prop but is used only when `info` is true (the ⓘ reveal); a caller passing `info={false}` supplies a note that renders nowhere. Today every call site passes `info`, so no live bug — the type just permits a silently-dropped prop.
Why it matters: API hygiene; the dropped-value case is one forgotten `info` away.
What correct looks like: make `note` optional and require it when `info` is set (discriminated props), or always render the note as the row's sr-only description.
Confidence: HIGH — Verification: read (both render sites 131-139/152-156 gated on `info`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useEffectsMsg` (makeAreaMsg product) | 74-83 | REAL | — | Lane-key widening factory; shared j13 dedup. |
| `stagedNote` | 86-87 | REAL | — | One shared note string. Trivial. |
| `StagedToggleRow` | 100-159 | REAL (inert by design) | F-003 | Disabled Switch, checked={false} constant; staged markers; operable ⓘ; pitch via data-kind. |
| `StagedDropdownRow` | 172-233 | REAL (inert by design) | F-002 | Disabled single-option Dropdown; dead onChange closure (documented). |
| `PresetRow` | 243-296 | REAL | — | The one interactive control: real 8-preset dropdown driving shared state; staged-marked wrapper. |
| `StagedNote` | 303-309 | REAL | — | Single section-level staged note (g12-P3-8 pitch fix). |
| `EffectsArea` | 316-397 | SHALLOW | F-001 | Composition; area-local EQ state (discarded on unmount); five inert rows + panel wrapper + note. |

## 5. Test honesty (effects.test.tsx, 12 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| header/embed/preset-row placement/preset list/preset→fader reshaping (34-102) | YES | DOM assertions incl. shared-state behavior across two components. |
| six ⓘ affordances (103) | YES | Operable button + caption reveal per row. |
| staged rows inert WITH reason (164-228) | YES | Asserts disabled + data-pending-command naming the exact awaited command per row. |
| "staged controls never send anything (no live commands left in the area)" (229) | YES | No-dispatch assertion over the transport — the honesty contract is pinned. |
| NOT covered: state loss on unmount (F-001's discard) | — | No test asserts the EQ config survives navigation — because it doesn't. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the EQ enable awaits the `audio.set-eq` catalog command; engine `Eq3Band` exists, doc 05 §2" (module header 6-8) | this file | REAL (engine stage exists) / the awaited command does not | `rg` zero hits for `audio.set-eq` in ui/src/generated/commands.ts and crates/commands/src; W07-A06: Eq3Band constructed by nothing in any shipped build. |
| "STAGED rows … render DISABLED with the honest reason … visible, inert, never a dead control that looks alive" (20-28) | this file | REAL | Verified by read + effects.test.tsx:164-228 (baseline #23). |
| "The embedded GraphicEQ + the preset row keep their interactive LOCAL state (the w26.c1 contract)" (27-28) | this file | REAL | Local useState traced; test :90 pins the interaction — and F-001 documents the discard. |
| Matrix row for equalizer marked "partial" | docs/feature-matrix.toml (per W07-A06) | FALSE (over-credit, prior wave) | W07-A06 F-001: "Eq3Band constructed by nothing in any shipped build; UI EQ staged-inert …; matrix row 'partial' over-credits." |

## 7. Dead / duplicated / unreachable code
- No dead code; every row renders. The five `onChange={() => {}}` closures are deliberately inert (documented, test-pinned).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Imports: Switch (form), Dropdown (menu), SettingInfoDot (SettingField), equalizer lane (EQ_PRESETS/NEUTRAL_GAINS/GraphicEQ/useEqMsg), makeAreaMsg. No feature flags. effects.css imported (another agent's file list has it? — it is not in my list; note only).

## 9. Open questions for the next agent
- Whoever audits `player/tools/equalizer/*` (the GraphicEQ source): confirm the panel itself never dispatches (the c1 "frozen contract") — effects.test.tsx:229 asserts it for THIS area only.
- The `audio.set-spatializer` forward declaration (w26.c2) — the spatializer UI exists nowhere else in ui/src per my rg during PlayerScreen work; if true, this row awaits a command with no other consumer.
