# AUDIT: ui/src/screens/player/tools/equalizer/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 43 / 2 harness components (+ harness.css stage, harness.test.tsx driver) |
| File verdict | REAL(DEV-ONLY) — mechanically sound and honestly fenced, but its evidence chain is stale: the only loop record that ever used this harness's HTML export verified the RETIRED A1 anatomy; the A2 content it composes today has no recorded screenshot run through this harness |
| Reachable from shipped entry point? | NO — by design and verified: `rg` shows harness.tsx imported only by its own harness.test.tsx; index.ts exports no harness; j10 review line 29 confirms "grep zero hits outside their own harness.test.tsx". The EqualizerTool it composes IS reachable (ToolsDialog/effects — see audit/files/.../EqualizerTool.tsx.audit.md). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "harness" index.ts` (no export); `rg -n "from \"./harness\"" src/` (8 lanes, all test-only imports); read harness.test.tsx, harness.css, EqualizerTool.tsx:1-120; read docs/waves/w26/logs/w26.c1.md (loop record, A1-era), w26.k10.md (§3 loop: 429×3 → measurement via `lumen:open-tools`, NOT this harness), findings/w26.e10.md (F1: A2 unimplemented at e10 time); `rg "W26C1_HARNESS_OUT" docs/` (only c1 log); baseline #23 (vitest 1705 pass incl. harness.test 2 + k10 pins) |

## 1. One-paragraph verdict
A minimal, honest dev-only harness in the b10 family: two stage components over the real `EqualizerTool` (collapsed + `defaultOpen`, presets-open), a `.w26c1-stage` whose 824px/20px-inset/20px-gap geometry matches EQ.svg's canvas, and a test driver that exports standalone HTML on `W26C1_HARNESS_OUT` — plus the k10 CSS regression pins living in harness.test.tsx. The defect is provenance: the header points at "the lane log for the loop record", but that record (w26.c1.md, 2026-11-12) describes the w26.f05-retired A1 horizontal-rows card — it compared `Audio_-_Equalizer.png` and recorded `EQ.png` as GAPS/unimplemented — while the harness now composes the A2 vertical-fader panel. The post-rework verification (w26.k10, measurement after vision 429×3) reached the surfaces through the live tools dialog (`lumen:open-tools`), never through this harness's export. No recorded screenshot run has exercised this file's export for the content it renders at HEAD.

## 2. What actually works (verified)
— verified by `rg` → "never imported by app routes" (header :1-2) is TRUE: sole importer is harness.test.tsx:12.
— verified by read harness.css:12-21 + analysis/tools-settings.md §A2 → stage geometry (824px width, 20px inset, 20px gap = EQ.svg title-bar bottom y56 → panel top y76) frame-correct for the A2 canvas it claims to mimic.
— verified by baseline #23 (vitest 1705 pass) → both harness tests green: crash-render of all states, and the export test's happy path; k10's pins (rail-pan width/max-width/overflow/scrollbar-width, data-preset glyph keying, `:nth-child` ban) are live in harness.test.tsx:94-114 and match equalizer.css at HEAD.
— verified by read harness.test.tsx:18-27 → the CSS inlined for the export covers every sheet the tool pulls in (tokens, lumen, form, icon, menu, slider, equalizer, harness) — the exported HTML is self-contained.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/tools/equalizer/harness.tsx:4-9
Code:
```
// element) so the comparison loop (contract §5) can
// screenshot them next to design/figma-renders/{EQ,EQ_presets}.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly at the A2 frame scale (824px canvas — the
// renders' 900px width at ~1.09 device scale) — see the lane log for the
// loop record.
```
What is wrong: the loop record the header cites does not describe this file's content. The only recorded use of this harness's export is w26.c1.md's comparison loop (2026-11-12), which ran against the A1 stacked-rows card (comparator `Audio_-_Equalizer.png`); its `EQ.png` comparison was recorded GAPS with "EQ.svg vertical-fader frame is unmounted… confirmed" (e10 F1). The w26.f05 A2 ruling later replaced the tool's anatomy with the vertical-fader GraphicEQ this harness now composes — and the post-ruling verification (w26.k10 §3, 2026-11-14) reached the presets/panel states via the live tools dialog (`lumen:open-tools` + Playwright click-through), not via this harness's HTML export. `rg "W26C1_HARNESS_OUT" docs/` finds only the c1 log. So at HEAD the harness's declared purpose — screenshots of its own stages for the §5 loop — has zero recorded executions for the A2 content.
Why it matters: the harness is the lane's frame-fidelity evidence instrument; a reader following the header's pointer concludes the A2 card was screenshot-verified through it, when the only export-based record predates the anatomy it renders. Same defect class as the subtitles harness (my file-13 audit): header evidence claims false at HEAD.
What correct looks like: either re-run the export→screenshot loop against EQ.png/EQ_presets.png for the A2 anatomy and record it, or rewrite the header to name k10's dialog-path measurement as the current evidence and mark the export path unused since the f05 ruling.
Confidence: HIGH — Verification: read of w26.c1.md §Comparison loop, w26.k10.md §3, w26.e10.md F1; rg for the export env var across docs/.

### F-002 [S4]
Where: ui/src/screens/player/tools/equalizer/harness.tsx:33-35
Code:
```
 * State 3 — the expanded tool with the preset list open (EQ presets.svg
 * state=open). The trigger is click-gated, so the harness test opens the
 * list through the public Dropdown API before serializing.
```
What is wrong: "the public Dropdown API" overstates it — harness.test.tsx:64 opens the list with `fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }))`, a DOM interaction through the accessible name, not a programmatic Dropdown API (the primitive exposes no open() imperative handle). Also the header's "A2 frame scale" label sits opposite the c1 log's "A1 render scale 400px × 2.25 = 900px" phrasing for the same ~1.09 factor (900/824 = 1.0922) — the scale math is right, the A1/A2 naming drifted across the two documents.
Why it matters: comment precision in an evidence-instrument file; a fixer aiming to "use the public Dropdown API" will find none to call.
What correct looks like: "opens the list by clicking the trigger's accessible button".
Confidence: HIGH — Verification: read harness.test.tsx:59-66; no imperative API on the Dropdown (menu/Dropdown.tsx exposes props only).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `EqualizerCardStates` | 23-30 | REAL | — | two EqualizerTool mounts (collapsed + defaultOpen); stage gap 20px = EQ.svg y56→y76; composes the real tool, no fixture state |
| `EqualizerPresetsHarness` | 37-43 | REAL | S3 | single defaultOpen tool; the presets-open interaction lives in the test (F-002 wording) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders all frame states without crashing" (harness.test:42-48) | WEAK | asserts only innerHTML.length > 0 — a smoke test; acceptable for a harness, but it cannot catch composition regressions (the k10 pins below carry that weight) |
| "exports standalone HTML when W26C1_HARNESS_OUT set" (:50-81) | YES | writes + asserts both files exist; skips cleanly when unset; opens the presets list before serializing |
| k10 CSS pins (:94-114) | YES | string pins on the real equalizer.css: rail pan (width/max-width 748/overflow-x/scrollbar-width), data-preset glyph keying, `:nth-child` ban — these fail on real regressions |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "never imported by app routes" | harness.tsx:1 | REAL | rg: sole importer harness.test.tsx:12; index.ts exports none |
| "screenshot them next to design/figma-renders/{EQ,EQ_presets}.png" | harness.tsx:3-4 | REAL-UNPROVEN for current content | only c1-era export loop (retired A1 comparator); k10 verified A2 via the dialog path; no post-f05 use of this harness's export |
| "see the lane log for the loop record" | harness.tsx:8-9 | FALSE at HEAD | the cited record (w26.c1.md) documents the pre-f05 A1 harness content; see F-001 |
| stage "824px wide — panel x20–804 with 20px insets" | harness.css:3,17-18 | REAL | matches analysis/tools-settings.md §A2 geometry |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped entry BY DESIGN (verified) — this is the file's purpose. The export path (W26C1_HARNESS_OUT) is de-facto dead instrumentationally since f05 (F-001), though mechanically functional.

## 8. Dependencies & flags
EqualizerTool (real component), harness.css (dev-only, body{margin:0} scoped to the standalone export), no feature flags; test driver depends on node:fs/path under an env gate — never runs in normal CI passes (baseline #23 runs the skip branch).

## 9. Open questions for the next agent
- Where is the w26.f05 A2-ruling loop record? No w26.f05.md log exists; the EqualizerTool header cites the ruling but the only post-ruling loop evidence is k10 §3 (dialog path). If the f05 lane's own loop was never recorded, the A2 panel's screenshot-fidelity evidence rests entirely on k10's measurement — worth a verify-wave row.
