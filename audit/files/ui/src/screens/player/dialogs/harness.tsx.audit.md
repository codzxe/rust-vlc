# AUDIT: ui/src/screens/player/dialogs/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 17 / 1 (ToolsDialogHarness) |
| File verdict | REAL (dev-only, UNREACHABLE by design) |
| Reachable from shipped entry point? | NO — by design: sole importer is harness.test.tsx:11 (census over ui/src: no route/app import). Render test executes every default vitest run (baseline row 23); the env-gated export is `it.skipIf`-reported — and this file's test is THE "1 skipped" in the baseline summary (census: the only skipIf/skip in ui/src). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg census ToolsDialogHarness/tools-dialog-harness (test-only; ZERO css rules for the marker class — verified with a *.css glob); Read of harness.test.tsx (110 lines); ls docs/waves/w26/logs/w26.c5.md (EXISTS); rg skipIf/skip/todo over ui/src (1 hit = this file); baseline ui-vitest.log.txt:10 "1705 passed \| 1 skipped" correlation. |

## 1. One-paragraph verdict
A 17-line dev-only harness in the w26.c2 pattern, and the place where that pattern's one defect was already fixed: the env-gated HTML-export test uses `it.skipIf` (harness.test.tsx:46-48) with the comment naming the fix ("skipIf makes the env-gated export EXPLICIT in the runner output instead of an always-passing no-op — w26.k09") — this is the honest-skip shape my dynamics-harness audit (F-001) demanded, and it is the single "1 skipped" test in the baseline vitest run (census: the only skipIf in ui/src). The harness itself is census-verified dev-only (no route/app importer), mounts the real ToolsDialog on both tabs (open, no-op close — fine for a screenshot fixture), and the render test asserts the two-tab chrome. The cited lane log exists (docs/waves/w26/logs/w26.c5.md). One S4 lives in the export path: the hand-maintained 14-file CSS manifest degrades silently — `existsSync` misses become an inline `/* missing: file */` comment instead of an error, so a renamed component sheet ships a style-less comparison artifact to the vision loop without failing anything.

## 2. What actually works (verified)
- Dev-only isolation: sole importer is the test (census); "never imported by routes or app code" (:2-3) is TRUE.
- Honest skip: it.skipIf at harness.test.tsx:46 — the env-gated export is counted as skipped (the baseline's 1 skipped), never a vacuous pass. The w26.k09 comment cites the fix's provenance.
- Render test is real: both tabs mount, 2 role=tab asserted (harness.test.tsx:33-41) — ran green in baseline.
- Export artifact: writes tools-dialog-{audio,video}.html with 14 CSS sheets inlined and a surface-deep body — the screenshot-loop feed the header describes (:4-5).
- Marker discipline: data-harness-tab attribute + tools-dialog-harness class are pure markers (zero CSS rules anywhere — verified); no geometry claims made, so nothing to contradict (contrast: the c2 harness claimed 400px cells AND shipped harness.css).
- The subject is real: ToolsDialog is the production dialog (my dialogs.css audit, file 2, verified its wiring to the tool seams).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/dialogs/harness.test.tsx:50-53 (the export path this harness exists to feed)
Code:
```
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
```
What is wrong: A missing/renamed CSS path in the hand-maintained 14-file manifest degrades silently — the exported HTML carries a `/* missing */` comment where the sheet should be, and the headless screenshot loop then compares the dialog against design renders with style-less components. No error, no non-zero exit.
Why it matters: The harness's entire purpose is pixel-faithful comparison artifacts; a silent style drop poisons the comparison in the least visible way (the loop would report "gaps" that are harness rot, not product drift — exactly the class of noise the c2 lane log's iter-2 record complains about). The manifest is hand-maintained per harness (dynamics/harness.test.tsx:15-22 carries its own 6-file list), so every component-sheet rename must be mirrored in each list by hand.
What correct looks like: `if (!existsSync(path)) throw new Error(`harness CSS manifest stale: ${file}`)` — fail the env-gated export loudly (it only runs on demand, so the blast radius of a hard error is one dev command).
Confidence: HIGH — Verification: read of the fallback; both manifests compared.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ToolsDialogHarness | 11-17 | REAL | — | Thin: marker div + real ToolsDialog (open, no-op close, defaultTab); dev-only by census |
| (companion test export path) | harness.test.tsx:44-68 | REAL | S4 | F-001 silent missing-CSS fallback; skipIf honest otherwise |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders the tools dialog on both tabs without crashing" :33 | YES | Both tabs + 2 role=tab asserted |
| "exports standalone HTML snapshots when W26C5_HARNESS_OUT is set" :46 | YES when run (env-gated); honestly SKIPPED otherwise — reported in the skipped bucket, never a vacuous pass (baseline row 23's 1 skipped) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "dev-only vision harness: the mounted ToolsDialog (Audio tab and Video tab variants) for the contract §5 comparison loop" | :1-2 | REAL | Census + render test + c5 lane log exists |
| "Never imported by routes or app code (the w26.c2 harness pattern)" | :2-3 | REAL | Sole importer = harness.test.tsx |
| "harness.test.tsx exports standalone HTML (CSS inlined) when W26C5_HARNESS_OUT is set, for headless screenshotting" | :3-5 | REAL | Export path read; both tab HTML files written; caveat F-001 on silent misses |
## 7. Dead / duplicated / unreachable code
- Unreachable by design (twice disclosed, census-proven). No dead code in the 17 lines.
- Cross-harness duplication: the CSS_FILES manifest pattern is duplicated per harness (6-file dynamics list vs 14-file dialogs list) — noted in F-001.
## 8. Dependencies & flags
ToolsDialog + ToolsDialogTab (the production dialog). Env flag: W26C5_HARNESS_OUT. No feature flags. No CSS import in this harness (marker class only; the export inlines styles from the manifest).
## 9. Open questions for the next agent
- The c2 dynamics harness predates the k09 skipIf fix (my F-001 there) — if a wave owns a cleanup pass, porting it.skipIf + a throwing manifest check to both harnesses is a two-file fix.
- ToolsDialog.tsx itself (not assigned to me): the dialog's own audit should carry the tools' backend-seam verdicts (W8 audio.set-eq, staged-inert tools).
