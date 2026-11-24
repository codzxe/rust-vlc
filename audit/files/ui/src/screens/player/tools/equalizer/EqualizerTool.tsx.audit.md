# AUDIT: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 165 / 1 component (+3 inner handlers) |
| File verdict | REAL component, STAGED by design — an 11-fader graphic EQ whose every control writes LOCAL state only; no `audio.set-eq` command exists in the catalog |
| Reachable from shipped entry point? | YES (ToolsDialog.tsx:36,122 mounts it in the Audio tab; effects.tsx:262-264 stages the settings-side rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg `EqualizerTool` src (exit 0 — mount ToolsDialog.tsx:122, no onChange passed); rg `audio.set-eq` src (exit 0 — zero command-surface hits; only staged markers); rg presets (exit 0 — 10 preset ids); sed ToolsDialog.tsx:100-130 |

## 1. One-paragraph verdict
Frame-faithful VLC-style graphic EQ (preset dropdown — 10 presets incl. classical/club/dance/full-bass — plus preamp + 10 band faders via GraphicEQ) that is honest theater: all state is local (`useState` L92-96), the mount passes no `onChange` (ToolsDialog.tsx:122 `<EqualizerTool />`), and the awaited command `audio.set-eq` (L49) does not exist in `ui/src/generated/commands.ts`'s CommandRequest union (verified by reading the union — no audio.set-* beyond set-volume/mute/device/delay). The tool self-labels: `data-staged-tool`/`data-pending-command` markers (L130-131) and the visible note "Staged — controls activate with the audio effects command card" (messages.ts:2406). Note the engine reportedly HAS a 3-band Eq (effects.tsx:7 "engine `Eq3Band` exists, doc 05 §2") — the UI ships a 10-band stage for a 3-band engine capability that itself has no command. Disclosure is genuine, but the product ships its flagship audio feature as an inert replica.

## 2. What actually works (verified)
— verified by read: preset application (L98-107) and fader commits (L109-112) genuinely update local state and would fire `onChange` if any host wired it — the seam is real, currently consumerless.
— verified by rg: staged markers + note asserted by tests in the lane (EqualizerTool.test/ToolsDialog.test exist); markers present at L130-131.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:45-49, 124-132 (vs ToolsDialog.tsx:122)
Code:
```
/**
 * Forward-declared command the tool's controls await (the honest-staging
 * marker, the sibling tools' pattern — no `audio.set-eq` command exists yet).
 */
const PENDING_COMMAND = "audio.set-eq";
```
What is wrong: the entire equalizer surface — the most recognizable VLC audio feature — has zero backend effect in every shipped build. The mount passes no onChange; no audio.set-eq exists in the catalog; even if it did, the SERVED host list would refuse it (apps/cli/src/dispatch.rs:41-56 has no audio effect commands). Disclosed via markers/note, hence S3 not S2 — but it is a complete flagship-feature replica shipped inert. Confidence: HIGH — Verification: read + rg (union + mount site).
F-002 [S3]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:20-23 (documented redundancy, still shipped)
Code:
```
// The tools dialog's transitional wrapper
// (ToolsDialog.tsx + dialogs.css `:has(.eq-tool--open)` note) is now
// redundant — equalizer.css suppresses THIS note inside that wrapper until
// its owner deletes it (see the w26.k10 lane log).
```
What is wrong: the tools dialog renders its own staged note around the tool (ToolsDialog.tsx:117-125: wrapper div + `tools-dialog__staged` note) AND the tool renders an identical internal note — equalizer.css hides the inner one via `:has()`. Duplicated disclosure machinery kept alive by CSS suppression instead of deletion; the CSS dependency means the note logic cannot be understood from either file alone. Confidence: HIGH — Verification: read (both files) + rg note text (single message key, two render sites).
F-003 [S4]
Where: ui/src/screens/player/tools/equalizer/EqualizerTool.tsx:52-58 vs presets.ts
Code:
```
  /** Preamp in dB (frame default 12.0). */
  readonly preamp: number;
  /** 10 band gains in dB, band order = EQ_BANDS. */
  readonly gains: readonly number[];
```
What is wrong: the UI models 10 bands + preamp while the engine's actual capability is Eq3Band (effects.tsx:7 comment) — a shape mismatch that guarantees the eventual command wiring cannot be a straight pass-through. Cosmetic today (state is local anyway). Confidence: MEDIUM — engine-side band count not independently verified by me (UNVERIFIED — engine crate outside my domain; comment is the source).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PENDING_COMMAND | 49 | REAL marker | S3 | names a nonexistent command (F-001) |
| EqualizerState (iface) | 52-59 | REAL | S4 | 10-band shape vs 3-band engine (F-003) |
| EqualizerToolProps | 61-76 | REAL | — | onChange seam exists, consumerless |
| EqualizerTool | 85-164 | REAL / staged | S3 | markers + note; local state only |
| applyPreset | 98-107 | REAL | — | unknown preset id falls back to NEUTRAL_GAINS |
| commitFader | 109-112 | REAL | — | |
| presetOptions | 114-122 | REAL | — | structural data-preset marker |

## 5. Test honesty
Lane tests assert the staged markers/notes (pattern verified in sibling dynamics tests). Would they fail if someone silently WIRED the tool to a fake success? No — nothing asserts absence of dispatches (there are none to assert). The tests pin the disclosure, not the capability. Baseline vitest covers. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no `audio.set-eq` command exists yet" (L47) | comment | REAL | generated/commands.ts CommandRequest union has no such id (read) |
| "Staged — controls activate with the audio effects command card" (messages.ts:2406) | user-visible note | REAL as disclosure | the command card has not landed |

## 7. Dead / duplicated / unreachable code
Duplicated staged-note machinery (F-002). The onChange seam is currently dead in production (no caller passes it — rg exit 0).

## 8. Dependencies & flags
components/menu Dropdown, ./presets (10 presets), GraphicEQ, useEqMsg lane seam, equalizer.css. No feature flags; staging is data-attribute based.

## 9. Open questions for the next agent
- Who owns the "audio effects command card"? If it lands with Eq3Band only, the 10-band UI needs a remap layer — plan it now (F-003).
- ToolsDialog owner: delete the transitional wrapper + CSS suppression (F-002) — the lane log already mandates it.
