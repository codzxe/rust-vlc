# AUDIT: ui/e2e/visual/baselines/linux/keyboard-shortcuts-1920x1080-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Type / dims / size / sha256 | PNG image data (file + IHDR parsed), 1920x1080, 69721 B, b6479a300665c28d… (full digest row in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (pinned visual baseline; verified PNG structure: magic + IHDR + chunk walk to IEND) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: one pinned screenshot baseline of the 216-case Linux visual matrix
(MATRIX-20261113.md, the current pin; MATRIX-20261109.md is the superseded
historical record). Route/context: keyboard-shortcuts (15-route era); 1920 nightly 1x dark.
Consumer: ui/e2e/visual/visual.spec.ts `toHaveScreenshot(`${id}.png`)` with
the id built by ui/e2e/visual/manifest.ts:139-145 caseId (`<route>-<viewport>-
<dpi>x-<theme>[-<accent>]`) — the file IS the comparison target; any pixel
drift over threshold 0.1 / maxDiffPixelRatio 0.001 fails the lane.
Matrix-evidence: this PNG is the evidence artifact for its matrix cell (the
feature-matrix visual claims rest on the lane, not the file).
Execution: baseline shows `npm run visual` lanes in ui-test.txt/ui-build.txt
context — visual lane run itself not in audit/runs/baseline (UNVERIFIED
current-lane pass; last recorded runs are inside the MATRIX docs' own run
records).
