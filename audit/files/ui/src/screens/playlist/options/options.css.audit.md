# AUDIT: ui/src/screens/playlist/options/options.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 397 / 0 (stylesheet, 27 selectors) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported at options/PlaylistOptionsMenu.tsx:23; provider mounted at playlist/PlaylistScreen.tsx:376 (routed screen) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 selector-vs-code cross-check (27/27 classes referenced); `rg -n "resolveMeta" ui/src --glob '!**/*.test.*'`; `sed -n '370,384p' PlaylistScreen.tsx` |

## 1. One-paragraph verdict
Real, frame-faithful stylesheet for the playlist options layer (sorting/view
menus, thumbnail grid, media list rows). All 27 selectors are consumed by
production components (15 files reference `lumen-plo*`). No dead rules. The
hostile catch: this sheet styles an "unwired state" that is in fact the ONLY
state the shipped app can render — the engine metadata seam the hints refer to
is never wired outside test harnesses.

## 2. What actually works (verified)
— python3 cross-check: 27/27 `.lumen-plo*` classes referenced in ui/src
(non-test files among the 15). — `import "./options.css"` at
PlaylistOptionsMenu.tsx:23; PlaylistScreen.tsx:376 mounts the provider.
— prefers-reduced-motion guard (css:393-397) present.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/options.css:118-127 (with
PlaylistScreen.tsx:376-384 and PlaylistOptionsProvider.tsx:71)
Code:
```
/* w26.k15 (j15 P3-11) — quiet honest-degradation suffix after the label
 * ("Needs media metadata" on no-op sort rows): tertiary ink, label scale,
 * no layout weight of its own. Not frame-pinned — it renders only in the
 * unwired state (frame/harness mounts wire a metadata resolver). */
```
What is wrong: the comment claims the hint "renders only in the unwired
state (frame/harness mounts wire a metadata resolver)" — but the ONLY shipped
mount, PlaylistScreen.tsx:376, is `<PlaylistOptionsProvider entries={openedEntries}>`
with NO `resolveMeta` prop. rg over non-test ui/src: `resolveMeta` is threaded
through options/* internals and defaulted to undefined; nothing outside the
options folder (and nothing but test support) ever supplies it. In the shipped
app, metadata-dependent sort rows are permanently no-ops and watched/progress
rendering runs on client-local overrides only.
Why it matters: this is a dead UI surface styled and copy-written as if it were
a transient degradation; the user-facing promise ("sort by artist/album/etc.")
can never fire.
What correct looks like: wire `resolveMeta` to a real engine metadata command
at the PlaylistScreen mount, or delete the metadata sort rows.
Confidence: HIGH — Verification: read + rg + sed (mount site inspected).

## 4. Per-function table
N/A (stylesheet). Selector inventory: 27 `.lumen-plo*` selectors + 1
`prefers-reduced-motion` block + 1 raw-color dark override (css:309-312,
stylelint-sanctioned #303030 with token-lane handoff note). All consumed.

## 5. Test honesty
OptionsMenus.test.tsx / PlaylistOptionsMenu.test.tsx / ThumbnailGrid.test.tsx /
MediaList.test.tsx / harness.test.tsx exist in the same folder (other agents'
scope if assigned); harness.test.tsx:25 asserts this css file is part of the
visual harness — the sheet is exercised by the UI test suite (not run here:
no node_modules; baseline vitest 1705 passed cited).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "it renders only in the unwired state (frame/harness mounts wire a metadata resolver)" | css:120-121 | FALSE (production is always the unwired state) | PlaylistScreen.tsx:376 mount passes no resolveMeta (sed quote) |
| "the engine has no artwork seam yet, so the well renders honestly empty" | css:189-190 | REAL | no artwork source in ui/src client (rg artwork -> none in client/) — UNVERIFIED against engine crates (other domain) |

## 7. Dead / duplicated / unreachable code
None inside the file. Styled-but-permanently-dead surfaces: `.lumen-plo__hint`
(F-001); `.lumen-plo-card--inert` / `--inert` rows are presentation-only by
declared design (w26.f06), not a defect.

## 8. Dependencies & flags
Token-based except two sanctioned frame-pinned raw values (border-radius 10px
css:52; background #303030 css:311 with stylelint-disable + provenance
comment). No imports, no feature gates.

## 9. Open questions for the next agent
- Does ANY engine command expose per-entry metadata (watched state, duration,
  title) that the UI could wire into `resolveMeta`? (bridge-client / commands
  crate domain.) If none exists, the metadata sort rows are unimplementable
  today and should be cut.
