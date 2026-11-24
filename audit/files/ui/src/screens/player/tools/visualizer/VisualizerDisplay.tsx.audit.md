# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerDisplay.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 (rev 2 — see §3 F-002 retraction) |
| Lines / functions | 189 / 5 (clamp01, modeLabelKey, VisualizerDisplay, BarSlot, SpectrometerDial) |
| File verdict | REAL (presentational) with an UNREACHABLE live path |
| Reachable from shipped entry point? | YES — VisualizerTool.tsx:93 renders it -> tools/visualizer/index.ts -> ToolsDialog.tsx:130 (`<VisualizerTool />`) -> player screen -> App -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "VisualizerTool" ToolsDialog.tsx` (mount at :130); Event-catalog scan for levels/peak events (none); `sed -n '44,46p' VisualizerTool.tsx` + `od -c` (byte-level); VisualizerTool.test.tsx case scan; `rg -c "\[m"` on all my audited files (0 everywhere) |

## 1. One-paragraph verdict
The display component itself is real, honest, well-built: it renders exactly
what it is given (clamped bars or the static frame-accurate resting depiction),
never simulating data, with correct clamping and clean svg math. Its LIVE mode
is permanently unreachable in the shipped app: the wire contract has no
audio-levels/FFT event of any kind (Event union, commands.ts:46 — verified by
scan) and the only production host mounts it with no `levels`
(ToolsDialog.tsx:130). The shipped visualizer is a static picture, honestly
labeled staged. Rev-2 note: an earlier draft of this audit reported a
compile-breaking "syntax error" in the host file — RETRACTED as a display
artifact (§3).

## 2. What actually works (verified)
— Code read: clamp01 handles non-finite (L35-38); resting/live switch (L69-70);
  SpectrometerDial radius lerp r∈[6,20] with (i+level)/n spread (L160-161);
  arc dash math 2πr/8 (L156, L184). — Tests: VisualizerTool.test.tsx:209-214
  asserts live bars + data-state="live" WITH supplied levels (host-seam test),
  and resting behavior otherwise — honest about both paths. Suite not run here
  (ui/node_modules absent, npm budget); baseline records vitest 1705 passed /
  1 skipped. — Host file integrity byte-verified via od (see §3).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/VisualizerDisplay.tsx:49-50 (with
ToolsDialog.tsx:130 and ui/src/generated/commands.ts:46)
Code:
```
  /** Live levels 0..1, one per bar; absent/empty → the resting state. */
  levels?: readonly number[];
```
What is wrong: no engine data source can ever supply `levels`. The Event
catalog (commands.ts:46) contains no audio-levels/peak/FFT event; the only
production host mounts `<VisualizerTool />` with neither levels nor onChange
(ToolsDialog.tsx:130). `data-state="live"` is unreachable in the shipped app —
the visualizer area is decorative geometry forever.
Why it matters: a "visualizer" that never visualizes; the honest staging
(role="note", documented seam) keeps this from being fraud, but it is a dead
surface with production chrome.
Confidence: HIGH — Verification: read + rg (event catalog + mount site).

F-002 [RETRACTED — was S0]
An earlier revision recorded: "VisualizerTool.tsx:45 `const ode, setMode] =
useState<VisualizerMode>(defaultMode);` is a syntax error breaking the whole UI
build." That was FALSE — a display-layer artifact: my tool-output channel
strips the literal two-character sequence "[m" (ANSI-reset-like), so
`const [mode, setMode]` DISPLAYED as `const ode, setMode]` in sed/cat -A/rg
output. Byte-level `od -c` of that line shows the intact text:
```
0000000           c   o   n   s   t       [   m   o   d   e   ,       s
0000020   e   t   M   o   d   e   ]       =       u   s   e   S   t   a
```
i.e. `const [mode, setMode] = useState<VisualizerMode>(defaultMode);` — valid.
Same artifact verified and cleared for parseTimeInput.ts:40 (`const [major,
minor, lowest = 0]`) and HdrTool.tsx:52 (`const [mode, setMode]`).
Consequence: the baseline claim "UI: typecheck/test/build all pass" is NOT
falsified; no repo defect exists here. Retraction logged in
audit/runs/W15-A02.log.md. All quotes in my other audit files were re-checked:
`rg -c "\[m"` over the 11 files audited before this correction -> 0
occurrences everywhere, so no other audit was affected.
Confidence: HIGH — Verification: ran od -c (bytes pasted above).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| clamp01 | 35-38 | REAL | — | non-finite → 0 |
| modeLabelKey | 41-43 | REAL | — | template key cast |
| VisualizerDisplay | 62-102 | REAL | S3 (F-001) | live path unreachable in shipped app |
| BarSlot | 105-144 | REAL | — | spectrum peak dot; eq token cycling via data-accent |
| SpectrometerDial | 152-189 | REAL | — | ring lerp + arc grammar correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "live levels drive one bar per entry, clamped to the band" (VisualizerTool.test.tsx:209) | YES | supplies levels, asserts data-state="live" + bar count |
| resting tests | YES | assert static depiction + data-state="resting" |
| dropdown/keyboard cases (L36-161) | YES | role/focus assertions |
Not run in this sandbox (no node_modules); baseline suite cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no FFT / audio-analysis source exists yet" | VisualizerDisplay.tsx:3 | REAL | no levels event in the Event catalog (commands.ts:46) |
| "never animated and never simulated" | VisualizerDisplay.tsx:7-8 | REAL | code renders static resting geometry (L89) |

## 7. Dead / duplicated / unreachable code
The live rendering path (levels !== undefined branches) is production-unreachable
(F-001) — reachable only from tests/harness (harness.tsx references levels 2x).

## 8. Dependencies & flags
./types (restingLevels, VisualizerMode, VisualizerLabelKey). No transport, no
feature gates. Pure presentational.

## 9. Open questions for the next agent
- Does any planned engine event (audio-engine domain) expose per-band levels?
  If not, the entire visualizer seam is speculative.
- NOTE for all agents reading via this toolchain: any displayed code line that
  looks like it is missing a "[" before an identifier beginning with "m" must
  be re-verified with `od -c` — the display layer eats "[m" (see F-002).
