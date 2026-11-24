# AUDIT: ui/src/screens/player/tools/equalizer/GraphicEQ.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 261 / 4 (GraphicEQ, EqFaderColumn, formatDb, snapDb) |
| File verdict | SHALLOW (real UI component; the "wire seam" dead-ends in React state — no engine command exists) |
| Reachable from shipped entry point? | YES — settings /settings/audio effects area (effects.tsx:356) + tools dialog (ToolsDialog.tsx → EqualizerTool → GraphicEQ) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg EQ command in generated/commands.ts (none); trace of onChange consumers (EqualizerTool.tsx, effects.tsx); read of presets.ts, index.ts, equalizer.css:352-359 |

## 1. One-paragraph verdict
A beautifully built 11-fader equalizer whose values go nowhere: every commit lands in React state (`setEqState`, effects.tsx:356) and the "wire seam for the effects settings area / tools dialog" (L89-92) dead-ends — the 65-command wire union contains NO equalizer command (rg over generated/commands.ts: zero matches for equalizer/eq-), and presets.ts:8 admits "[VERIFY against the engine lane once an equalizer command card exists]". The user drags faders, sees "12.0 dB" chips, applies presets — and the audio is untouched. (The panel's collapse mechanism — `hidden={!isOpen}` at L180 + `.eq-panel__body[hidden]` at equalizer.css:359 — is intact per fixed-string count verification.)

## 2. What actually works (verified)
— verified by read: controlled/uncontrolled split (L129-137) is correct (no shadow-writes); snapDb kills 0.1-float dust; live chip + commit seam are internally consistent; aria-expanded/controls wiring complete.
— verified by rg: NO engine consumer exists — `audio.*` wire commands are set-volume/get-volume/set-mute/list-devices/set-device/set-delay only (generated/commands.ts L24 union).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/tools/equalizer/GraphicEQ.tsx:88-92; effects.tsx:356; presets.ts:8; index.ts:2-3
Code:
```
  /**
   * Fires on every committed change (fader commit) with the panel's full
   * state — the wire seam for the effects settings area / tools dialog.
   */
  onChange?: (state: EqualizerState) => void;
```
What is wrong: the "wire seam" has no wire. The only production consumer of onChange is `setEqState` in effects.tsx:356 (React state) and EqualizerTool's internal commit — nothing sends anything; the command catalog has no equalizer/effect command to send. index.ts:2-3 advertises "the default state payload for the engine wire-up" — there is no wire-up. Why it matters: a full professional-looking EQ surface (11 faders, presets Flat/Rock/etc.) that cannot affect playback — the single largest "dead control pretending" surface in the tools family. What correct looks like: an engine command card (e.g. audio.set-equalizer) + a transport sender at the seam, or the panel marked staged/inert like the video tools' `pendingCommand` markers. Confidence: HIGH — Verification: read + rg (no EQ command in union; no send at consumers).

F-002 [S3] — RETRACTED (display artifact)
An earlier version of this finding claimed the `.eq-panel__body[hidden]` rule was textually corrupted ("`.eq-panel__bodyidden]`"). That was a display artifact of the Bash tool's output sanitizer (strips `[h`/`[m` sequences from displayed text). `rg -c --fixed-strings '.eq-panel__body[hidden]'` → 1: the rule is INTACT and the collapse works. No finding stands here.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| formatDb/snapDb | 50-63 | REAL | - | float-dust guard |
| GraphicEQ | 104-206 | SHALLOW (backend) | S2 | real component, dead seam (F-001); collapse intact |
| EqFaderColumn | 227-261 | REAL | - | live/commit chip truthfulness |

## 5. Test honesty
Lane tests (not in my list) assert UI behavior; a mutation that made onChange a no-op would still pass them at the fader level — the seam's deadness is untestable from inside the UI suite. UNVERIFIED test content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wire seam for the effects settings area / tools dialog" | L89-90 | FALSE | no wire exists; consumer is setState (effects.tsx:356); no EQ command in the union |
| "the default state payload for the engine wire-up" | index.ts:2-3 | FALSE | no engine wire-up exists in the tree |
| "[VERIFY against the engine lane once an equalizer command card exists]" | presets.ts:8 | REAL (honest admission) | no command card exists (rg) |

## 7. Dead / duplicated / unreachable code
None beyond the dead seam itself.

## 8. Dependencies & flags
EQSlider, ValueChip, Icon primitives; bands/presets data. No env vars.

## 9. Open questions for the next agent
- Does the Rust engine have an equalizer module at all (crates/audio)? If yes, the missing piece is the command card; if no, the whole EQ family is theater.
