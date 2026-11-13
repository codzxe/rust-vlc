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

## What issue #37 delivered (2026-11-09)

- Full **Solstice v2 + wave-3 re-pin** of the Linux canonical set: all 136
  baselines re-captured (`--mode=nightly --update`, 136 passed 7.7 m) and
  verified green without `--update` (nightly 136/136, PR lane 30/30).
  Environment provenance (playwright 1.64.1 / chromium 156.0.8078.4 build
  v1248 / node 24.22.2) and the deliberate-change justification:
  [`MATRIX-20261109.md`](MATRIX-20261109.md).
- Closed the 4.H1 snapshotDir trap in `playwright.config.ts`
  (`testIgnore: "**/visual/**"`): plain `npm run test:e2e` no longer runs
  visual cases against Playwright's default snapshot dir.

## What wave 26 delivered (2026-11-13)

- **W25 redesign re-pin — the missing provenance record, closed (j20
  P2-4)**: the 2026-11-12 full re-pin of the then-144-case set post-W25
  redesign shipped with no MATRIX record. Provenance — environment,
  justification, the grid-anchor-is-the-product-default resolution, and the
  wave-K 25-route completion — is now recorded in
  [`docs/waves/w26/baselines/MATRIX-20261112.md`](../../../docs/waves/w26/baselines/MATRIX-20261112.md).
- **25-route manifest truth (j20 P2-9)**: `ROUTES` now enumerates the full
  routable inventory — 12 non-settings renderable routes + all 13 settings
  areas + the `/dev/tokens` dev fixture — the same inventory
  `e2e/a11y.spec.ts` walks since w26.k19. Disk holds **216 baselines**
  (25 routes × 2 viewports × 2 DPIs × 2 themes = 200 grid cases + 16 accent
  samples); the nightly lane is **216 cases**, the PR lane **50**
  (25 routes × {dark, light}).
- The stale "Solstice gold" honesty note on `DEFAULT_ACCENT` is gone:
  `orange` is both the grid anchor and the product default since the W25
  re-anchor (see the rewritten note in `manifest.ts`).
- **Pending (Lead, wave L)**: waves i–k landed further UI changes after
  both capture dates, so the full-matrix diff-green re-verification +
  re-seed is deferred to the Lead's between-wave L pass (recorded in the
  wave-i integration commit); MATRIX-20261112 is the provenance anchor for
  that pass.

## What remains

- ~~Wire CI~~ (done, issue #5): Linux PR lane per push/PR, nightly full
  matrix + non-blocking macOS/Windows BASELINE SEED jobs in
  `.github/workflows/ui.yml`.
- macOS/Windows baseline sets: still CI-blocked on a first real seed run
  (the seed job captures + uploads them for human commit; Linux stays
  canonical).
- In-panel states (player customize/hide-ui, miniplayer states, context
  menus, audio/video tools, the playlist/subtitles flow) are NOT routes —
  they belong to the in-flow state harnesses (PlayerScreen seam tests, e2e
  dialog specs), not this route × viewport grid (manifest header, w26.k20).
  The route inventory itself is complete.
