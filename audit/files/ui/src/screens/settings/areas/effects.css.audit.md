# AUDIT: ui/src/screens/settings/areas/effects.css (compact, 44 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 44 (2 rule groups: the EQ preset two-layer trigger; the staged full-strength override) |
| File verdict | REAL (styles implement a documented decision — that decision is the finding) |
| Reachable from shipped entry point? | YES — the settings Audio screen's Effects section (effects.tsx) ships it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg "set-eq\|set-spatializer\|set-stereo-widener" crates/commands/src/catalog/ids.rs ui/src/generated/commands.ts` -> NO matches (staged commands absent); `rg Eq3Band crates/audio-engine/src` -> effects/eq.rs EXISTS; read of effects.tsx header |

## 1. One-paragraph verdict
Two honest rule groups and one policy choice that deserves hostile light.
(1) The EQ-preset trigger styling is frame-cited token work. (2) The
`:40-43` override forces DISABLED controls in the Effects area to full
opacity — the comment says "the audio-1 frame shows live-looking controls
... the honesty lives in the disabled state + the row markers + the one
section note, never in dimming". The result: the Effects section ships
controls at full visual strength that are inert (audio.set-eq /
set-spatializer / set-stereo-widener do not exist in the catalog —
verified), plus an EQ preset dropdown and fader panel that ARE interactive
but drive LOCAL STATE ONLY (effects.tsx:12-17: "interactive LOCAL state
... staged-marked"). The engine's Eq3Band exists (effects/eq.rs) with no
command to reach it. A user dragging the EQ faders changes nothing but a
React state variable — with honesty outsourced to a ⓘ reveal and a
data-staged-row attribute.

## 2. What actually works (verified)
— Token usage correct (all vars exist per tokens.css audit); frame
dimensions cited inline.
— The staging claims are truthful: the awaited commands genuinely do not
exist (rg), and the rows genuinely render disabled with markers
(effects.tsx rules, quoted above).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/effects.css:40-43
Code:
[data-settings-area="effects"] .lumen-switch:disabled,
[data-settings-area="effects"] .lumen-dropdown__trigger:disabled {
  opacity: 1;
}
What is wrong: this override deliberately erases the ONLY conventional
visual signal of inertness (the global disabled grammar is
--opacity-disabled) so staged rows look live at full ink; combined with
effects.tsx's interactive-but-local EQ preset + faders, the whole Effects
section is either inert-but-live-looking or interactive-but-effect-free.
Honesty is carried by a ⓘ reveal caption and data attributes invisible to
users. Why it matters: this is the maximal-theater end of the repo's
"honest staging" spectrum — the section reads as a working equalizer in
any screenshot while no build can move a single engine gain. What correct
looks like: dimming (the thing this rule removes) or a persistent visible
"not wired" chip on the section header. Confidence: HIGH — Verification:
read + rg (catalog absence verified).
(Cross-file: the staging decision lives in effects.tsx — flagged here
because this file is the instrument that implements look-alive.)

## 4. Per-function table (compact)
| Rule | Lines | Verdict | Notes |
|---|---|---|---|
| .effects-eq-preset trigger | 21-33 | REAL | token-clean, frame-cited |
| staged full-strength override | 40-43 | REAL (policy) | S3 F-001 |

## 5. Test honesty
areas.test.tsx / audio tests assert the staged markers and disabled
states; no test can catch "looks alive but isn't" — that is a visual-
policy defect, not a DOM defect.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "staged rows render FULL-STRENGTH while inert ... the honesty lives in the disabled state + the row markers" | :35-39 | REAL (as description) | the code does exactly this; the policy is F-001 |
| "the EQ enable awaits the audio.set-eq catalog command; engine Eq3Band exists" | effects.tsx:6-7 | REAL | command absent (rg); Eq3Band present (effects/eq.rs) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Shared dropdown/switch class grammar; no flags.

## 9. Open questions for the next agent
- staged.tsx auditor: what do the data-pending-command markers actually
  gate? If nothing reads them outside tests, the "row markers" half of the
  honesty contract is test-only fiction.
