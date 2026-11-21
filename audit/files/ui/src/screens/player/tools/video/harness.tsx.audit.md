# AUDIT: ui/src/screens/player/tools/video/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 6 (CardFrameStates, ColorsHarness, CropHarness, EssentialHarness, HdrHarness, HDR_HARNESS_CAPTION) |
| File verdict | UNREACHABLE (deliberate dev-only vision harness; consumed only by its vitest file) |
| Reachable from shipped entry point? | NO — by design and verified: only importer is harness.test.tsx; tools/video/index.ts (the app surface) does not re-export it; rg over ui/src shows no app-route import. The four tools it composes ARE app-reachable (ToolsDialog.tsx:39 imports ColorsTool/CropTool/EssentialTool/HdrTool; mounted at ToolsDialog.tsx:134-137). No shipped binary exists at all (W14 pin). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -n 'from "\./harness"' ui/src` (only *.test.tsx consumers across 6 harness modules); cat tools/video/index.ts (no harness export); `file design/figma-renders/*.png` (dims 900×1998/1847/1847/935); `rg width= height= design/figma/*.svg` (400×888, 400×821, 400×821, 406×422 — comment dims EXACT); docs/waves/w26/logs/w26.c4.md + w26.i08.md read in full (loop record located); harness.css read (canvas 400px/20px insets, hdr 406px/22px — matches comments); sed 1-11/66-83 quote verification |

## 1. One-paragraph verdict
Honest, working dev-only fixture: every structural claim checks out (dev-only exclusion proven by import graph; the four composed tools are real, app-mounted components; the frame dimensions quoted in each doc-comment match the SVGs exactly; the figma-renders PNGs exist). The vision-comparison loop the harness exists to feed DID run — but not when the header comment promises ("see the lane log"): the w26.c4 lane log records the agent died pre-proof with the vision pass OWED; the actual loop record (4/4 MATCH after fixes) lives in w26.i08.md, weeks of lanes later. Two further hygiene nits: the blanket "2.25 device scale" claim is false for the HDR frame (its render is ~2.2167×), and the harness's own smoke test asserts only that HTML is non-empty. No product code touches this file.

## 2. What actually works (verified)
- Dev-only claim: TRUE — the only import of `./harness` (video) is harness.test.tsx:14-19; index.ts (the lane's public surface) exports the four tools but not the harness; no App-route file names it.
- Frame-dimension comments vs the real SVGs: Video - Colors.svg 400×888, Video - Crop.svg 400×821, Video - Essential.svg 400×821, HDR Options.svg 406×422 — harness.tsx:36,46,56,66 quote exactly these.
- Scale math for the three card tools: 400×2.25 = 900 and 888×2.25 = 1998, 821×2.25 ≈ 1847 — PNGs measure 900×1998 and 900×1847 (×2). `file` output verbatim.
- The comparison loop ran and is recorded: docs/waves/w26/logs/w26.i08.md:131-141 — "Scale-matched harness compares (2.25×/2.2167× via the w26.c4 HTML export): HDR GAPS→MATCH, Colors GAPS→MATCH, Crop GAPS→MATCH, Essential MATCH (iter 1)"; two real defects found and fixed (trigger label alignment, "Bottom" truncation).
- defaultOpen prop contract real: VideoCard.tsx:48-52, ColorsTool.tsx:197,225, types.ts:34-38 (doc: "exists for hosts (harness, effects area)").
- Baseline vitest 1705/1705 pass (INDEX #23) includes harness.test.tsx's render smoke.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/video/harness.tsx:4-6
Code:
```tsx
// loop (contract §5) can screenshot them 1:1 (2.25 device scale = the 900px
// renders) against design/figma-renders/Video_-_Colors.png, Video_-_Crop.png,
```
What is wrong: The blanket "2.25 device scale = the 900px renders" holds for the three card tools (400×2.25=900) but NOT for the HDR frame: HDR_Options.png is 900×935, and 935/422 ≈ 2.2167 ≠ 2.25 (900/406 ≈ 2.2167 as well). The HDR render was exported at a different scale — corroborated by w26.i08.md:137, which had to compare at "2.25×/2.2167×" respectively.
Why it matters: The harness's whole purpose is 1:1 screenshots; a consumer trusting the header would screenshot the HDR state at 2.25 and get a ~1.5% linear scale mismatch — exactly the "scale/density noise" class w26.i08 had to decompose before finding real defects.
What correct looks like: "2.25 device scale for the three card tools; the HDR render is ~2.2167× (900×935 for 406×422) — screenshot per-frame at the render's own scale."
Confidence: HIGH — Verification: read + `file` on the PNGs + SVG dims + w26.i08.md:137.

### F-002 [S4]
Where: ui/src/screens/player/tools/video/harness.tsx:8-9
Code:
```tsx
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
```
What is wrong: "The lane log" (w26.c4) contains NO loop record — it records the opposite: "Agent died post-code (infra)… Vision proof owed (agent died pre-proof) — flagged for wave-e" (docs/waves/w26/logs/w26.c4.md:16,18). The loop record the comment promises lives in a different lane's log (w26.i08.md:131-141), written later.
Why it matters: A citation that points at a log saying "proof owed" is how paper trails get trusted without being read; an auditor following the comment finds a debt, not a record.
What correct looks like: Cite the actual record: "loop record: docs/waves/w26/logs/w26.i08.md (4/4 MATCH); w26.c4 log: proof owed at lane close."
Confidence: HIGH — Verification: read both logs in full.

### F-003 [S4]
Where: ui/src/screens/player/tools/video/harness.tsx (its test, harness.test.tsx:60-67)
Code:
```tsx
  it("renders all four tools in frame layout without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
```
What is wrong: The harness's own in-repo test is crash-only: `innerHTML.length > 0` passes for any non-empty render and asserts nothing about frame layout (canvas width, 20px gap, two stacked states). The layout contract survives only in the out-of-repo screenshot loop and the video.css regex pins lower in the same file. Related swallow in the export branch: `existsSync(path) ? readFileSync(...) : \`/* missing: ${file} */\`` — a drifted CSS path silently degrades every exported screenshot instead of failing.
Why it matters: In baseline runs (no W26C4_HARNESS_OUT), the export branch never executes, so the repo's own green suite proves only "does not crash + some HTML" — the 1:1 promise is entirely external.
What correct looks like: Pin the DOM contract in jsdom: stage width 400/406, exactly two tool roots per stage, caption text = frame name; make a missing CSS file a hard failure in the export branch.
Confidence: HIGH — Verification: read harness.test.tsx:60-92 in full.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CardFrameStates` | 22-34 | REAL | — | pure presentational; children typed ReactElement[] (each harness passes exactly 2); caption = frame name, not product copy (documented) |
| `ColorsHarness` | 37-44 | REAL | — | collapsed + defaultOpen states; matches Video - Colors.svg 400×888 comment |
| `CropHarness` | 47-54 | REAL | — | same grammar; 400×821 verified |
| `EssentialHarness` | 57-64 | REAL | — | same grammar; 400×821 verified |
| `HDR_HARNESS_CAPTION` | 69 | REAL | — | const "HDR Options"; split from the JSX only for the lint rule noted in CardFrameStates |
| `HdrHarness` | 71-83 | REAL | S4 | hand-rolls the CardFrameStates markup (stage/caption/frame trio duplicated with --hdr modifiers) instead of parameterizing it — divergence risk if the shared grammar changes; second HdrTool wrapped in `.w26c4-hdr-open` (min-height 420px in harness.css) for the opened-list shot, driven by the test's Dropdown API click (harness.test.tsx:70-74) — the w26.c4 comment's "opens through the public Dropdown API" claim verified in the test |

## 5. Test honesty (if tests exist in/for this file)
harness.test.tsx (same directory) is the only consumer.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders all four tools in frame layout without crashing" (:60-67) | ONLY on crash/throw | non-empty-HTML assertion; a harness that lost the stage/gap/two-state layout entirely still passes (F-003) |
| "exports standalone HTML snapshots when W26C4_HARNESS_OUT is set" (:69-92) | NOT in baseline | early-returns without the env var; the fs-write branch executed only in the manual w26.i08 loop (record cited) — REAL-UNPROVEN in-repo |
| "frame-contract pins (video.css)" (:94-140) | YES | regex pins on load-bearing CSS (value-span display:none, field-sizing content, dropdown cap var, no scoped text-align copy) — these are strong, and they guard the product sheet, not the harness |
| Mutation check (reasoned): drop the second `<ColorsTool defaultOpen />` from ColorsHarness | NOT caught in-repo | smoke only counts HTML > 0; the two-state contract is enforced solely by the external screenshot loop |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | harness.tsx:1 | REAL | import graph: only harness.test.tsx; index.ts omits it |
| "2.25 device scale = the 900px renders" | harness.tsx:4-5 | FALSE for HDR / REAL for the 3 card tools | PNG dims: 900×1998, 900×1847 ×2 (2.25 exact); HDR_Options.png 900×935 = 406×422 at ≈2.2167 (F-001) |
| "see the lane log for the loop record" | harness.tsx:9 | MISLEADING | w26.c4 log: "Vision proof owed"; record is in w26.i08.md:131-141 (F-002) |
| Frame dims per harness (400×888 / 400×821 / 400×821 / 406×422) | harness.tsx:36,46,56,66 | REAL | `rg width= height=` on the four SVGs — exact match |
| "cards land at 360px wide", "20px state gap" | harness.tsx:20-21 | REAL | harness.css: 400px stage, 20px padding, `.w26c4-frame { gap: 20px }` |

## 7. Dead / duplicated / unreachable code
- The module is unreachable from app code BY DESIGN (verified) — this is intentional tooling, not accidental dead code; the four tools it composes are live (ToolsDialog.tsx:134-137).
- Duplication: HdrHarness re-implements the CardFrameStates trio with modifiers (per-function table).

## 8. Dependencies & flags
Imports: the four tool components (app-real, app-mounted), harness.css (dev-only sheet, audited separately). No engine/bridge usage — the harness wires nothing to any backend; the tools themselves are local-state-only per index.ts ("All state is local to each tool"). No feature flags. No i18n (captions are frame names, deliberately outside the catalog).

## 9. Open questions for the next agent
- The other five harness modules (dynamics, visualizer, equalizer, dialogs, playlist options/subtitles) share this pattern — an agent auditing those should reuse the F-001..F-003 checklist (scale claims, lane-log citations, crash-only smoke tests).
- The HDR render's odd export scale (2.2167×) suggests the figma-renders set was exported inconsistently — worth one line in a design-assets audit.
