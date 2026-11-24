# AUDIT: ui/src/screens/player/tools/dynamics/CompressorTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 119 / 8 |
| File verdict | REAL for what it pins; the pinned tool itself is a staged (not-yet-wired) card — disclosed to the user, so the test is honest about a deferral |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:129 `✓ …CompressorTool.test.tsx (9 tests) 298ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline; rg `set-compressor` repo-wide; rg audio.* vocabulary in generated/commands.ts; rg staged note copy |

## 1. One-paragraph verdict
Solid component tests: collapsed-by-default with hidden-body semantics, the
seven frame rows pinned label+value+slider in frame order, ARIA ranges and
valuetext, keyboard commit (25.0→25.5 ms) and End-clamp, and the staged-tool
markers. The important context I verified: the compressor card is
deliberately NOT wired to the engine — `audio.set-compressor` appears
NOWHERE in the protocol vocabulary (generated/commands.ts audio.* = get/set
volume, mute, delay, device only) and CompressorTool.tsx/types.ts contain no
send call at all; the card carries a visible note whose copy is
"Staged — controls activate with the audio effects command card"
(messages.ts:2404). So the test pins an honestly-disclosed deferral, and the
values are local UI state that activates nothing today.

## 2. What actually works (verified)
— Baseline ui-test.txt:129: 9 tests green in 298 ms.
— Vocabulary sweep (rg): `set-compressor` exists only in
ui/src/screens/player/tools/dynamics/{CompressorTool.tsx,types.ts} (L88
`pendingCommand="audio.set-compressor"`); `rg '"audio\.[a-z-]+"' ui/src/
generated/commands.ts` → 8 commands, none compressor. No send/cmd: lines in
the tool (rg L"UI-USES" sweep: 0 hits).
— Mutation reasoning: collapse regression fails L30-35; a missing row or
wrong default fails the FRAME_ROWS loop (L66-70); the keyboard-commit value
fails L97-98; staged-marker removal fails L114-115.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/dynamics/CompressorTool.test.tsx:110-118
Code:
  it("marks the card staged with the pending command marker", () => {
    render(<CompressorTool defaultOpen />);
    const card = screen.getByLabelText("Compressor");
    expect(card).toHaveAttribute("data-staged-tool", "compressor");
    expect(card).toHaveAttribute("data-pending-command", "audio.set-compressor");
What is wrong: the marker names a command that does not exist in the
generated protocol vocabulary (verified — see §2); the card sends nothing.
This is an honest, disclosed deferral (the note copy promises activation "with
the audio effects command card", messages.ts:2404), so the test is truthful
about what exists — but the attribute value is a forward-declared command
name, and this file pins it as if it were a live contract. Blast radius: one
staged tool card; severity floor S4.
Confidence: HIGH — Verification: read + rg sweeps.
F-002 [S4]
Where: :117 `expect(note.textContent).toMatch(/staged/i)` — a case-insensitive substring on the user copy; a copy change to "queued for a future card" passes the spirit but fails nothing (or vice versa the regex can never fail as long as the word "staged" survives any rewrite). Weak assertion pinned beside otherwise-exact ones.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| card shell | 26-60 | REAL | hidden-body + aria-expanded semantics |
| frame rows | 62-89 | REAL | 7 rows exact; ranges; no-ticks pin |
| local slider state | 91-108 | REAL | keyboard commit; clamp |
| honest staging | 110-119 | REAL | pins the disclosed deferral (F-001) |

## 5. Test honesty
All 8 fail on their named regression (§2). No fake assertions; the staging
test is honest about the deferral BEING a deferral.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "honest staged markers" | file L4 | REAL | note copy verified at messages.ts:2404 |
| "controls activate with the audio effects command card" (future) | messages.ts:2404 | PAPER-ONLY (no such command card in the tree) | rg vocabulary sweep |

## 7. Dead / duplicated / unreachable code
None in the test.

## 8. Dependencies & flags
RTL + vitest; the real CompressorTool; no transport needed (card is local
state — consistent with the deferral).

## 9. Open questions for the next agent
- When the audio effects command card lands, does `audio.set-compressor`
  enter the generated vocabulary and this test get a transport-seam suite?
  (config-settings.test.ts is the pattern to copy.)
