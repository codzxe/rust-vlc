# AUDIT: ui/src/screens/player/tools/dynamics/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 53 / 2 (HarnessCell, DynamicsToolsHarness) |
| File verdict | REAL (UNREACHABLE from app routes — by design, disclosed) |
| Reachable from shipped entry point? | NO — deliberately: rg census shows the only importer is harness.test.tsx:11 (test bundle only; no app route/module imports it). The header's "never imported by app routes" claim is TRUE, so it ships in no production chunk. Its render path executes in the default vitest suite (harness.test.tsx:25-32, baseline row 23); its export path is env-gated (F-001). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg census DynamicsToolsHarness (1 importer); Read of harness.test.tsx (110 lines) + harness.css; ls docs/waves/w26/logs/w26.c2.md (EXISTS, 141 lines) + sed of its comparison-loop record; ls design/figma-renders/Audio_-_{Compressor,Spatializer,Stereo}*.png (all 3 EXIST). |

## 1. One-paragraph verdict
A 53-line dev-only vision harness whose every structural claim verifies: it is genuinely unreachable from app routes (single test importer — the "never imported by app routes" claim is a census fact, not prose), the frame-cell geometry arithmetic is exact (400px cell, 20px frame padding, border-box → cards land at exactly 360px; the lane log's iter-1 record even shows the 328px slider measurement being caught and fixed through this harness), the three cited comparison PNGs exist in design/figma-renders/, and the cited lane log (docs/waves/w26/logs/w26.c2.md) records a real 3-iteration comparison loop with concrete measurements and a MATCH verdict. The caption literals ("Audio - Compressor Settings") ride the i18n rule's designed custom-prop gap, disclosed inline and immaterial for a never-shipped dev fixture. One test-honesty defect: the HTML-export test early-returns when W26C2_HARNESS_OUT is unset, so in every default run (including the baseline's 1705-pass) it passes while executing nothing — the harness's actual deliverable (the standalone-HTML export that feeds the screenshot loop) has no always-on regression coverage, and the loop's execution evidence lives only in agent-authored lane-log prose.

## 2. What actually works (verified)
- Dev-only isolation: only harness.test.tsx imports DynamicsToolsHarness (rg census); the module and harness.css never enter a production chunk — the header and lane log ("b10 pattern") both state it and the census proves it.
- Geometry claims: .w26c2-frame width 400px + padding 20px + border-box = 360px content = "the cards land at 360px wide" (:20) exact; 20px gap = the frames' state gap; the canvas choice (--color-surface-deep, harness.css:14-21) is reasoned and disclosed (the iter-2 record explains why plain surface hid the collapsed pill).
- Comparison-loop artifacts: all three design/figma-renders PNGs exist; lane log w26.c2.md records 3 iterations with numeric gaps (row pitch 86→76, slider 344→328), a 1024×768 clip check, and a light-mode pixel-sample pass.
- Render test is real: harness.test.tsx:25-32 asserts 6 .dynamics-tool elements (collapsed + expanded per tool) — the "never ships collapsed-only" contract is pinned; ran green in baseline row 23.
- Lint-gap disclosure accurate: caption strings pass through a custom prop (`caption="…"`), which local/i18n-no-literals does not police (it checks JSX text + the 7 user-visible DOM attrs) — the inline comment states exactly that, and the strings are frame names, not copy.
- CSS discipline: harness.css uses tokens throughout (surface-deep canvas, font-size-caption, content-secondary); the hex values (#090909/#191919/#1B1B1B) appear only in explanatory comments.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/dynamics/harness.test.tsx:34-36 (the test covering harness.tsx's export path)
Code:
```
  it("exports a standalone HTML snapshot when W26C2_HARNESS_OUT is set", () => {
    const out = process.env.W26C2_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
```
What is wrong: In every default run — including the baseline vitest pass (row 23: 1705 passed / 1 skipped) — this test returns before its first assertion and counts as a passing test while verifying nothing. The CSS-inlining, file-writing export path (harness.test.tsx:38-55), which is the harness's entire purpose (feeding the contract §5 screenshot loop), has zero always-on coverage: a broken CSS_FILES list, a bad resolve, or a writeFileSync regression would pass every default suite.
Why it matters: The 1705-pass number includes vacuous tests; the export feature's only executed evidence is the lane log's agent-authored loop record (docs/waves/w26/logs/w26.c2.md:75-95) — prose, not a cited machine run. The skip is disclosed in the comment, but a test that always skips should be `it.skipIf`/`describe.env` so suites report it as skipped rather than passed.
What correct looks like: `it.skipIf(!process.env.W26C2_HARNESS_OUT)(…)` (vitest reports the skip honestly in the 1-skipped bucket), or an always-on test that sets the env var in a temp dir and asserts the file contents (CSS inlined, 6 tool mounts).
Confidence: HIGH — Verification: read of the early return; baseline row 23 shows only 1 skipped test suite-wide (not this one — it PASSES vacuously).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HarnessCell | 21-33 | REAL | — | Caption prop (lint-gap disclosed), children array into .w26c2-frame |
| DynamicsToolsHarness | 36-53 | REAL | — | 3 cells × (collapsed + defaultOpen expanded) = the 6-tool render contract; consumers: test only |
| (harness.css companions) | 1-40 | REAL | — | 400/20/360 arithmetic exact; token-only styling |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders all three tools in frame layout without crashing" (:25) | YES | 6 .dynamics-tool mounts asserted; a broken tool import/prop crashes it |
| "exports a standalone HTML snapshot when W26C2_HARNESS_OUT is set" (:34) | NO (in default runs) | F-001: early return = vacuous pass in every default suite; only env-gated runs exercise it |
| frame-contract pins describe block (:59-110) | YES | CSS-source regex pins of dynamics.css (h14 fixes: flush thumb travel, −7.5px chevron inset, tick paint, light remaps) — real regression pins, would fail on CSS drift |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | :1-2 | REAL | rg census: only harness.test.tsx imports it |
| "the comparison loop (contract §5) can screenshot them against design/figma-renders/Audio_-_{Compressor,Spatializer,Stereo}*.png" | :4-6 | REAL (artifacts) / REAL-UNPROVEN (the loop itself) | All 3 PNGs exist; the loop's execution is recorded in lane-log prose (w26.c2.md:70-95) but no committed machine run captures it |
| "Rendered by harness.test.tsx into static HTML (jsdom → document), then screenshotted headlessly — see the lane log" | :8-9 | REAL (render) / REAL-UNPROVEN (screenshot) | Render test green in baseline; screenshot step is env-gated + prose-recorded |
| "20px gutters — the cards land at 360px wide" | :19-20 | REAL | harness.css arithmetic: 400 − 2×20 border-box = 360 |
| "the figma frame name itself, not product copy (plain prop pass-through — the lint rule doesn't fire here)" | :27-28 | REAL | Custom-prop gap of local/i18n-no-literals verified in my rule audit (file 13) |
## 7. Dead / duplicated / unreachable code
- The whole module is UNREACHABLE from shipped entry points — intentionally, twice-disclosed, test-only. Not dead: the test exercises it every run.
## 8. Dependencies & flags
Imports the three real production tools (CompressorTool/SpatializerTool/StereoWidenerTool — those belong to other agents' audits; W8/W11 pins about backend wiring apply to the TOOLS, not this composition). harness.css scoped to the harness. Env flag: W26C2_HARNESS_OUT (export path). No feature flags.
## 9. Open questions for the next agent
- The three tool .tsx files (unassigned to me): the staged-inert seams the harness screenshots — whoever audits them should confirm the tools' data-pending-command discipline my video.css audit saw for the video tools.
- The "b10 pattern" citation (lane log) refers to a sibling harness pattern — the playlist/options/harness.tsx and subtitles/harness.tsx files (other agents) share this shape; F-001's vacuous-export class likely repeats there.
