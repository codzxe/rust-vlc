# AUDIT: ui/src/screens/player/tools/dynamics/StereoWidenerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 68 / 2 (STEREO_WIDENER_ROWS data, StereoWidenerTool) |
| File verdict | SHALLOW (honestly-staged inert tool — data module + 8-line composition) |
| Reachable from shipped entry point? | YES (web build): ToolsDialog.tsx:37 imports, :129 mounts `<StereoWidenerTool />` (player screen → Tools dialog); index.ts:7 re-exports. Baseline #22 build OK, #23 vitest aggregate 136 files/1705 green. DEV-ONLY: harness.tsx (never imported by app routes). No GUI binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -ln "StereoWidenerTool" src` (ToolsDialog, effects lane does NOT mount it — it stages its own StagedToggleRow, effects.tsx:384-389); repo-wide Grep `set-stereo-widener|stereo_widener` (17 hits: ui lane files, docs/waves/w26/*, audit/ — ZERO in crates/ apps/ ui/src/generated/commands.ts); `rg -o '"audio\.[a-z-]+"' src/generated/commands.ts | sort -u` (catalog audio.* = get-volume/list-devices/set-delay/set-device/set-mute/set-volume/device-lost/device-changed only); read DynamicsCard.tsx (staging mechanism), types.ts, ToolSliderRow.tsx, shared/SliderRow.tsx:118 (tick math `((index+1)/(ticks+1))*100`); `rg -n "widener" src/i18n/keys/w26.c2.ts src/i18n/messages.ts` (all 5 keys exist); read StereoWidenerTool.test.tsx (118 lines, 8 tests); cross-refs: W15-A10 DynamicsCard.tsx.audit.md (engine has Gain/Mute/Balance/Eq3Band only), my effects.tsx F-001, ToolsDialog F-001. |

## 1. One-paragraph verdict
A pure data module doing exactly what it admits: four frame-calibrated slider rows (delay 0–75 ms; three ratio-scaled gains 0–0.9/0–0.8/0–1.0) wrapped in the shared DynamicsCard with the honest-staging apparatus — and every value the user drags dies in card-local React state, because `audio.set-stereo-widener` exists nowhere: not in the generated command catalog (verified: the only audio.* commands are volume/device/mute/delay), not in any Rust crate (repo-wide grep zero outside ui/docs/audit), and the audio engine implements no widener stage (W15-A10: effects.rs has Gain/Mute/Balance/Eq3Band only). The file is honest about this (role="note" staged line, `data-pending-command`, types.ts' "STAGED DISPLAY RANGES" contract), and its frame arithmetic — the tick-count claims in the header — verifies exactly against the shared renderer's i/(n+1) math. The one real defect beyond the inertness is cosmetic-but-real: gains are ratio-scaled yet formatted with a "%" unit, so the widest setting reads "0.9 %".

## 2. What actually works (verified)
— Verified by arithmetic against shared/SliderRow.tsx:118 (`left = ((index + 1) / (ticks + 1)) * 100`): every tick claim in the header comment is exact — Feedback (n=8, marks at 1/9…8/9): 0.9·2/9 = 0.2 = default ✓; Crossfeed (n=7, marks at 1/8…7/8): 0.8·3/8 = 0.3 ✓; Dry Mix (n=9, marks at 1/10…9/10): 1.0·8/10 = 0.8 ✓; Delay Time has no `ticks` property → no tick overlay (SliderRow.tsx:104 `ticks > 0` guard). The test's 24-tick total (8+7+9) and the zero-tick delay row pin the same facts.
— Verified by `rg` over src/generated/commands.ts + repo-wide: this file does not lie about wiring — the staging markers it sets (`data-staged-tool`, `data-pending-command`, DynamicsCard.tsx:91-92,138-140) name a command that genuinely does not exist, and the test (:109-117) pins those markers, so the honesty cannot silently regress.
— Verified by read + test: all four labelKeys exist (w26.c2.ts:37-40) and are registered in messages.ts (:960-963); row defaults render frame-format values ("20.0 ms", "0.2 %", "0.3 %", "0.8 %") — pinned by FRAME_ROWS at test:11-16.
— Verified by baseline #23 aggregate (EXIT=0): StereoWidenerTool.test.tsx's 8 tests are in the 136-file green run.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/tools/dynamics/StereoWidenerTool.tsx:63
Code:
```tsx
      pendingCommand="audio.set-stereo-widener"
```
What is wrong: the tool's entire reason to exist — widening stereo audio — is unreachable. The awaited command exists in no layer: generated catalog (audio.* = device-changed/device-lost/get-volume/list-devices/set-delay/set-device/set-mute/set-volume — verified by exhaustive `rg -o` extract), no Rust handler (repo-wide `set-stereo-widener|stereo_widener` matches only ui/ docs/waves/w26/ audit/), and the audio engine has no widener stage (W15-A10: `crates/audio-engine/src/effects.rs` implements Gain/Mute/Balance/Eq3Band only). Every slider drag mutates DynamicsCard's local `useState` and is discarded on unmount.
Why it matters: a fully interactive, tactilely satisfying 4-row DSP card in the shipped Tools dialog that can never affect one sample of audio. The staging is honest (markers + note), so this is not fraud — but it is the third parallel surface for the same missing command (ToolsDialog card, effects.tsx:386 StagedToggleRow, this file), and a user who expands "Stereo Widener" and drags Crossfeed gets zero feedback that nothing happened beyond one small "staged" note.
What correct looks like: the DSP command layer this marker forward-declares, or the card disabled with an explanation — the same bar set in my effects.tsx F-001.
Confidence: HIGH — Verification: catalog extract (rg), repo-wide grep, engine-stage evidence cited from W15-A10's audited rg over effects.rs.

### F-002 [S4]
Where: ui/src/screens/player/tools/dynamics/StereoWidenerTool.tsx:33 (also :43, :53)
Code:
```tsx
    initial: 0.2,
    ticks: 8,
    format: (value) => `${value.toFixed(1)} %`,
```
What is wrong: the gain rows are ratio-scaled (max 0.9 / 0.8 / 1.0) but formatted with a percent unit — the Feedback default renders "0.2 %" and the maximum possible reading is "0.9 %". A user reading "%" expects a 0–100 scale (20%, 90%); these are raw ratios with a % sign appended. The header comment (:3) calls this "PLAIN values (percent units on the gain rows)" and the frame is the design authority (types.ts:27-30), so it is a faithful port of the frame's value column — but the unit is wrong in the frame too.
Why it matters: unit confusion on a control that purports to be calibrated; when the engine range lands (types.ts:30 "Engine ranges replace these"), a "0.9 %"→"90 %" rescale silently changes every frame-derived default unless the format is revisited with it.
What correct looks like: either drop the "%" (VLC-style 0.0–1.0 plain ratios) or scale ×100 in the format (`${(value*100).toFixed(0)} %` → "20 %") when engine semantics land.
Confidence: HIGH — Verification: read of all three format lambdas + tests pinning "0.2 %"/"0.3 %"/"0.8 %" (:84, :103).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `STEREO_WIDENER_ROWS` (module const) | 15-55 | REAL (data) | F-001, F-002 | 4 row defs; tick counts 8/7/9/none — every default-on-tick claim arithmetically verified against SliderRow's i/(n+1) paint. No engine counterpart for any range (staged display ranges by contract, types.ts:27-30). |
| `StereoWidenerTool(props)` | 58-68 | REAL (composition) | F-001 | 8-line pass-through to DynamicsCard; spreads DynamicsToolProps (defaultOpen/id) — no ignored params. Reachable: ToolsDialog.tsx:129. |

## 5. Test honesty (StereoWidenerTool.test.tsx, green in baseline #23 aggregate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| card shell (:24-55) | YES | Collapsed-by-default, aria-controls resolves, hidden body — break the toggle wiring and aria-expanded/hidden assertions fail. |
| frame rows (:57-86) | YES | Mutation: change `initial: 20`→`15` → FRAME_ROWS' "20.0 ms" missing; change ticks 8→7 → 24-tick count fails; change max 0.8→1.0 → aria-valuemax fails. |
| local slider state (:88-107) | YES | ArrowRight 20→20.5 pins step 0.5; shift+arrow 0.2→0.3 pins the coarse step — real interaction through the shared Slider. |
| honest staging (:109-117) | YES | data-staged-tool + data-pending-command + role=note pinned — the honesty markers cannot silently disappear. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "each default lands on its tick (0.2 of 0.9 at 2/9, 0.3 of 0.8 at 3/8, 0.8 of 1.0 at 8/10)" (:6-9) | this file | REAL | Arithmetic verified against SliderRow.tsx:118 ((i+1)/(n+1) paint): all three exact. |
| "PLAIN values (percent units on the gain rows)" (:3) | this file | SHALLOW | The values are plain; the "percent" unit is F-002 (ratios with a % suffix). |
| "The Stereo Widener tool card — mountable as-is (tools dialog, effects area)" (:57) | this file | REAL / FALSE-half | Tools dialog: YES (ToolsDialog.tsx:129). Effects area: NO — effects.tsx stages its own StagedToggleRow (:384-389), never mounts this component; the "effects area" half of the comment is stale. |
| types.ts:79-81 "No such command exists yet" | types.ts | REAL | Catalog extract + repo-wide grep (F-001 evidence). |

## 7. Dead / duplicated / unreachable code
- Not dead: mounted by ToolsDialog; harness usage is dev-only by declared design.
- Duplication: the nonexistent `audio.set-stereo-widener` command name now lives in THREE production surfaces — this file (:63), ToolsDialog's sibling cards (via DynamicsCard instances), and effects.tsx:386's StagedToggleRow — plus docs/waves/w26/{logs,findings,reviews}. When the command lands, three call sites must be reconciled; cross-ref effects.tsx F-001 and DynamicsCard.tsx F-001 (W15-A10).
- Engine-unreachable by construction (F-001); binary-unreachable via W14.

## 8. Dependencies & flags
- DynamicsCard + types (same lane), ToolSliderRow→shared/SliderRow, i18n keys w26.c2 (pre-registered lane keys merged into messages.ts — verified present). No flags. No generated-command import (correctly none — the command doesn't exist).

## 9. Open questions for the next agent
- None for this file. The lane-level question (who owns the audio.set-{eq,spatializer,stereo-widener,compressor} gap) belongs to the effects/DynamicsCard owners already cross-referenced; suggest a single consolidated cross-wave finding instead of a fourth copy.
