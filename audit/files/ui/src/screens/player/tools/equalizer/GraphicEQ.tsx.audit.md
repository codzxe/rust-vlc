# AUDIT: ui/src/screens/player/tools/equalizer/GraphicEQ.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 261 / 5 (formatDb, snapDb, GraphicEQ, EqFaderColumn, commit helpers) |
| File verdict | REAL-UNPROVEN (as a component) / SHALLOW (as a feature end-to-end) |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (settings Effects area embed + EqualizerTool in the player Tools dialog; rg consumers verified). ENGINE: NO — `audio.set-eq` does not exist in the catalog (rg over generated/commands.ts, dispatch.rs, host.rs: zero hits); the only engine EQ is Eq3Band (crates/audio-engine/src/effects/eq.rs). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg "audio.set-eq" (EqualizerTool.tsx:49 PENDING_COMMAND, effects.tsx:262-264 staged markers — NO dispatch anywhere); rg "Eq3Band" (crates/audio-engine/src/effects/eq.rs — 3-band); sed eq.rs:1-14 (preamp + low/mid/high); sed bands.ts:25-34 (10 bands), presets.ts:64-84 (defaults, ±20 dB); GraphicEQ.test.tsx survey:16-169 |

## 1. One-paragraph verdict
A cleanly engineered controlled/uncontrolled panel: the w26.k10 fix (internal
state written only when uncontrolled) is correct, the 0.1 dB lattice snap
kills float dust, the controlled seam is well documented and test-pinned.
But the entire wire seam terminates in staged markers: both hosts
(EqualizerTool, settings/effects) hold local state and display
`data-pending-command="audio.set-eq"` — a command that exists nowhere. The
W8 pin confirmed at this file's door. Worse, the panel ships 11 faders
(preamp + 10 bands, EQ.svg frame) while the only engine EQ core is a
THREE-band shelf (Eq3Band) — even when the command lands, this state shape
cannot drive that engine without an engine rewrite. Frame fidelity is
pursued further than the engine can ever follow.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): GraphicEQ.test.tsx
executes the morphing bar↔panel toggle (controlled + uncontrolled), the 11
fader columns with frame chips (preamp 12.0 dB, bands 0.0 dB), ±20 dB range
+ tabular aria-valuetext, 0 dB tick placement, monochrome columns, the
state seam (controlled drive, defaultState seeding, commit fires onChange
with full state).
— verified by read: snapDb (line 63) is the correct boundary for the float
dust problem (5.6+0.1 → 5.700000000000003); commit paths (134-146) are
uncontrolled-only writes with onChange always fired — the controlled-mount
shadow-write bug (j10 P3-3) is genuinely fixed.
— verified by rg: EqFaderColumn's onChangeLive/commit contract matches
EQSlider's (the chip shows live drag value, honest).

## 3. Findings
### F-001 [S3] The onChange wire seam has zero wired consumers — every fader drag is decorative (W8 pin)
Where: ui/src/screens/player/tools/equalizer/GraphicEQ.tsx:88-92
Code:
```
  /**
   * Fires on every committed change (fader commit) with the panel's full
   * state — the wire seam for the effects settings area / tools dialog.
   */
  onChange?: (state: EqualizerState) => void;
```
What is wrong: Both consumers of the seam (EqualizerTool's commitFader,
settings/areas/effects.tsx) store the state locally and render staged
markers; NEITHER dispatches anything. `audio.set-eq` has no catalog entry
(rg: generated/commands.ts, apps/cli dispatch.rs, host.rs — zero), so no
host CAN dispatch it. Dragging all 11 faders changes no audio anywhere —
the audio path in every shipped build is the null device anyway (baseline
INDEX: "Audio 'plays' only into the null device").
Why it matters: This is the equalizer a user sees when they open the player
tools dialog; it is a meticulously frame-accurate mock. The honesty markers
exist (EqualizerTool.tsx:47-49, effects.tsx:262-264) but the faders are
fully interactive.
What correct looks like: The engine core exists (Eq3Band — preamp + 3
shelves, bit-exact identity at 0 dB); a minimal `audio.set-eq` mapping the
engine's actual shape would make at least the preamp/3-band subset real.
Confidence: HIGH — Verification: read + rg (no command, no dispatch, no
wired consumer).
### F-002 [S3] 11-fader UI vs 3-band engine: the state shape can never drive the only existing EQ core
Where: ui/src/screens/player/tools/equalizer/GraphicEQ.tsx:190-201
Code:
```
          {EQ_BANDS.map((band, index) => (
            <EqFaderColumn
              key={band.id}
              label={eq(band.labelKey)}
              value={state.gains[index] ?? 0}
```
What is wrong: EQ_BANDS is 10 graphic bands (60 Hz…16 kHz, bands.ts:25-34)
+ preamp; the engine's Eq3Band is preamp → low shelf → mid peak → high
shelf (eq.rs:1-14). effects.tsx:7 openly notes "engine `Eq3Band` exists,
doc 05 §2" — the UI was built frame-first against a 10-band model the
engine does not have. When `audio.set-eq` is eventually designed, the
payload shape (preamp + 10 gains) mismatches the engine's (preamp + 3).
Why it matters: Architecture drift: the seam's forward-declared contract
guarantees a breaking redesign at wiring time; 7 of 10 faders have no
possible engine meaning today.
What correct looks like: Either the engine grows a 10-band biquad bank
(matching the frame), or the UI honestly exposes 3 bands until it does.
Confidence: HIGH — Verification: read both sides (bands.ts vs eq.rs).
### F-003 [S4] Frame-true default preamp +12 dB is a hearing hazard once wired
Where: ui/src/screens/player/tools/equalizer/presets.ts:66-72 (consumed at GraphicEQ.tsx:116-119)
Code:
```
/** Default preamp in dB (frame: Audio - Equalizer.svg reads "12.0 dB"). */
export const DEFAULT_PREAMP_DB = 12;
```
(quoted from presets.ts — the state this panel seeds at 116-119)
What is wrong: The default state ships preamp +12 dB (≈4× linear gain)
because the static frame depicts the slider there. Harmless today (staged),
but the moment the seam is wired, an untouched panel default demands +12 dB
— clipping risk and a loud surprise; VLC's default preamp is 0 dB.
Why it matters: A frame-fidelity decision carrying a latent safety defect
into the future wiring.
What correct looks like: Default 0 dB; the frame's 12.0 was a depiction,
not a spec (the file's own [VERIFY with the engine lane] tag at presets.ts:81
flags this family).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| formatDb | 50-52 | REAL | — | one decimal + i18n unit |
| snapDb | 63 | REAL | — | correct lattice snap, documented rationale |
| GraphicEQ | 104-206 | REAL-UNPROVEN | S3 (F-001/F-002) | component correct; feature staged end-to-end |
| setOpen | 129-132 | REAL | — | uncontrolled-only write + onOpenChange — the j10 P3-3 fix |
| commit / commitPreamp / commitBand | 134-146 | REAL | — | immutable state updates; onChange always fires |
| EqFaderColumn | 227-261 | REAL | — | live-preview chip via onChangeLive; commit clears live — truthful |

## 5. Test honesty (GraphicEQ.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bar↔panel toggle, controlled + defaultOpen (17-72) | YES | asserts aria-expanded/hidden and onOpenChange calls |
| 11 vertical faders + frame chips (75-105) | YES | counts columns, asserts chip text |
| ±20 range + aria-valuetext (107-117) | YES | asserts slider attributes |
| 0 dB tick placement + monochrome (119-144) | YES | CSS-class pins |
| state seam: controlled drive, seeding, commit payload (147-169+) | YES | asserts onChange receives full state, ArrowUp raises |
Mutation check: make commit write internal state even when controlled →
tests 147+ (controlled drive) fail — the suite pins the fix. Honest suite;
it never pretends audio flows (no audio assertions at all — correct, none
exist).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "all state is LOCAL by default … with an `onChange` wire seam" | header 26-31 | REAL | code + tests |
| (implied by the seam) hosts wire it to the engine | — | SHALLOW | F-001: both hosts stage |
| afeat-equalizer engine core exists (afeat row) | effects.tsx:7 comment | REAL | crates/audio-engine/src/effects/eq.rs Eq3Band |
| 10-band graphic EQ (EQ.svg frame) | header 1-24 | REAL as UI / SHALLOW as feature | frame-true panel; engine is 3-band (F-002) |

## 7. Dead / duplicated / unreachable code
None in-file. The deadness is systemic (the seam's consumers are staged),
counted once in F-001.

## 8. Dependencies & flags
components/form (ValueChip), icon, EQSlider, bands/presets/useEqMsg,
equalizer.css. No feature flags, no client import (correct for a leaf
panel).

## 9. Open questions for the next agent
- Who owns the eventual `audio.set-eq` payload shape — the 10-band frame or
  the 3-band engine? (F-002; a catalog decision that should be made BEFORE
  the command is designed, not after.)
- EqualizerTool.tsx + effects.tsx are other agents' files — confirm their
  staged markers never regress into silent dispatch attempts.
