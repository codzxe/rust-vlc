# AUDIT: ui/e2e/visual/baselines/linux/settings-global-1280x720-1x-dark-cyan.png
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary asset — pinned visual-regression baseline PNG |
| Size / sha256-prefix / dims | 71203 bytes / 5aa4c95e0546 / 1280x720 (IHDR verified; PNG magic OK) |
| Manifest case | route `/settings/global`, viewport 1280x720, DPI 1x, theme `dark` -> caseId `settings-global-1280x720-1x-dark-cyan` |
| Lane | nightly(accent-sample) |
| File verdict | REAL — manifest-generated consumer, compare-verified run on record |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Verdict
Real PNG baseline, byte-verified (magic + IHDR dims 1280x720). It is
consumed by name: ui/e2e/visual/manifest.ts `caseId()` reproduces this
exact filename from the case `/settings/global` x 1280x720 x 1x x `dark`, and
ui/e2e/visual/visual.spec.ts:102 asserts the rendered page against it:
```
      await expect(page).toHaveScreenshot(`${id}.png`, { clip: box });
```
(caseId, manifest.ts:142-145:
```
export function caseId(c: VisualCase): string {
  const accent = c.accent === DEFAULT_ACCENT ? "" : `-${c.accent}`;
  return `${slug(c.route)}-${c.width}x${c.height}-${c.dpi}x-${c.theme}${accent}`;
}
```
)
Run evidence (committed, 2026-11-13 5th re-pin, ui/e2e/visual/MATRIX-20261113.md:147-151):
```
Run record (5th re-pin, all under the flag, this sandbox):
- PR `--update`: **50 passed (1.9m)**; compare verify **50 passed (1.6m)**
- nightly `--update`: **216/216 (sharded 108+108; 3.9m + 4.7m)**
- nightly compare verify: **216/216 (sharded 108+108; 3.5m + 3.6m)**
```
PR-lane cases run on every push via .github/workflows/ui.yml `visual` job
(`node scripts/visual-run.mjs --mode=pr`); nightly-lane cases run via the
`visual-nightly` schedule. CI execution itself: UNVERIFIED in this sandbox
(no runner here; local record above is the committed evidence).

## 2. Notes
- Non-default-accent sampling case (`cyan`), part of the 16 accent-sample cases, nightly lane only.
