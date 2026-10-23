# Visual screenshot-diff harness (card 4.G1, doc 08 §6)

Layout:

- `manifest.ts` — the screen matrix (route × viewport × DPI × mode × accent)
- `visual.spec.ts` — the Playwright spec that walks it
- `baselines/<platform>/<case>.png` — pinned baselines (Linux canonical)
- driver: `ui/scripts/visual-run.mjs` ← `npm run visual*` ← `cargo xtask visual`

Tolerance, modes, flakiness controls and the baseline-update policy are
documented in `ui/README.md` § "Visual screenshot-diff harness".

## Seeded-regression demonstration (the 4.G1 acceptance proof)

Recorded 2026-10-23 on this tree (chromium 156.0.8078.4, Linux). A 2-px
`translateX` is injected on the app shell (`#root > *`) — a test-only style
override, no product code touched:

```console
$ npm run visual:seed          # = visual-run.mjs --mode=pr --seed
[visual] mode=pr update=false seed=true
  ✘  1 e2e/visual/visual.spec.ts:40:5 › visual player-1280x720-1x-dark ›
       matches pinned baseline (pr)
    Error: expect(page).toHaveScreenshot(expected) failed
      7440 pixels (ratio 0.01 of all image pixels) are different.
      … (all 8 PR-lite cases fail; each reports expected/received/diff
      attachments under test-results/)
  8 failed
$ echo $?
1
```

The 2-px shift produces 2 337–7 440 differing pixels per case (0.9–2.3×
the whole clip area in diff terms — ratio 0.01–0.02), far above the pinned
`maxDiffPixelRatio: 0.001` tolerance, and fails on the retry too (a real
regression, not a transient). With the seed removed:

```console
$ npm run visual:pr
  8 passed (14.2s)            # exit 0

$ npm run visual:nightly
  32 passed (54.2s)           # exit 0 — full matrix
```

`cargo xtask visual --mode=pr` runs the same lane through the Rust task
runner and forwards the exit code verbatim (verified: 0).

## What 4.G2 delivered (2026-10-24)

- Full-route manifest: every route in `ui/src/App.tsx` (15) × the
  viewport/DPI/mode grid + the doc 08 §6.1 accent sampling (8 non-default
  accents on onboarding + settings-global) → **136 nightly cases**, 30 in
  the PR lane. Scaling decisions + run record:
  [`MATRIX-20261024.md`](MATRIX-20261024.md).
- FROZEN baseline matrix captured post-F4 merge
  (`npm run visual -- --mode=nightly --update`) and verified green without
  `--update` — Linux canonical set (per-OS sets pending CI runners; the
  `<platform>` baseline dir is the extension seam, doc 08 §6.3).

## What remains

- Wire CI: per-OS jobs (Linux canonical), PR lane on every PR, full matrix
  nightly (doc 08 §6.3). macOS/Windows baseline sets get pinned by one
  `--update` run per OS once runners exist.
- Grow `ROUTES` as doc 08 §1.6's unrouted compositions land (player
  customize/hide-ui, miniplayer states, context menus, audio/video tools,
  playlist/subtitles flow).
