# AUDIT: ui/src/components/icon/icons.tsx#part1 (lines 1-451)
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 451 (of 905; chunk boundary: part1 = 1-451, ends after `"profile": personGlyph,` — part2 = 452-905: registry tail `artist`→`about`, `iconNames`, `getIconGlyph`, `twoToneGlyphs`, `getTwoToneGlyph`, `hasTwoToneVariant` belong to A09) / 0 functions, 5 shared consts, 26 registry entries, 76-name type union |
| File verdict | REAL (glyph data is real, typed, exhaustively consumed by `<Icon>`; vitest-proven) |
| Reachable from shipped entry point? | NO (same as all ui/src — no GUI binary at HEAD). SPA-internal: `Icon.tsx:59` `getIconGlyph(name)` renders these entries; every screen consumes `<Icon>`. Executed under vitest (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read 1-451 complete; python3 extents over `design/figma/Icon.svg` volume groups (speaker x per level: 288.547/332.547/376.547/420.547 — identical cell-relative anchor −7.45 at 44px cell pitch); sheet `lists` group paths extracted and matched byte-for-byte to both registry entries; `rg -o 'name=…'` sheet inventory (close value=none/low/medium/default + volume value=high — the misnamed-sheet story verified); Icon.test.tsx inventory (27 tests) + read of the volume-family test :161-189. |

## 1. One-paragraph verdict
A genuine, disciplined glyph registry: 76 typed names, exhaustively mapped (`Record<IconName, ReactElement>` — TypeScript forbids gaps), paths verbatim-mined from the committed design sheet with per-entry provenance, aliases factored into shared consts, and a prune pass (w26.k04) that actually deleted unconsumed names. The one real defect is measurable arithmetic: the four volume glyphs are each centered on their own bounding box, but the design sheet anchors the speaker at an identical cell-relative position in every level — so the port makes the speaker jump up to 3px horizontally when the volume level changes, and the regression test was written to compare path STRUCTURE with numbers abstracted, exactly hiding this. Second defect: `menu` and `lists` are byte-identical six-path glyphs registered twice, violating the file's own no-duplicated-alias-data convention.

## 2. What actually works (verified)
— Registry integrity: `Record<IconName, ReactElement>` (icons.tsx:239) + `iconNames = Object.keys(glyphs)` (:824, part2) — the 76-name union and the registry cannot drift apart under tsc. Baseline run #21 (`npm run build` = tsc --noEmit) passed.
— Sheet cross-check verified: Icon.svg carries `name=close, value=none|low|medium` + `name=volume, value=high` — the header's "misnamed close component" story is true (rg over the sheet).
— `lists` port verified exact: sheet paths (M464 32H465 / M468 32H480 / … y=27/32/37) translate to the registry's (M4 7H5 / M8 7H20 / … y=7/12/17) by the documented bbox-centering rule (arithmetic checked).
— Transport glyphs correctly carry `fill="currentColor"` for design-filled bodies (play :243, stop :255, next :262-263, previous :272-273).
— 27 Icon tests passed in baseline run #23, including per-glyph geometry regression pins (:198, :222).

## 3. Findings

### F-001 [S3]
Where: ui/src/components/icon/icons.tsx:277-310 (volume-mute/low/medium/high)
Code:
```
  "volume-low": (
    <>
      <path d="M6.547 8.294L13.547 4.738C14.212 4.4 15 4.883 15 5.63V12V18.37C15 19.117 14.212 19.6 13.547 19.262L6.547 15.706C6.211 15.536 6 15.191 6 14.815V12V9.185C6 8.809 6.211 8.464 6.547 8.294Z" />
      <path d="M18 11L18 13" />
    </>
  ),
```
What is wrong: The four volume speakers sit at four different x positions: mute 4.547, low 6.547, medium 5.047, high 3.547 (all 8.453 wide — measured from the four path strings). The design sheet does NOT do this: the speakers start at 288.547/332.547/376.547/420.547 on a 44px cell grid — i.e. at exactly the same cell-relative offset (−7.45 from each cell center) in every level; the sheet anchors the speaker and grows waves rightward. The port's convention "each glyph's bounding-box centre lands on (12,12)" (:60-62) re-centers each level's DIFFERENT-width bbox, dragging the speaker with it.
Why it matters: ControlBar's VolumeCluster swaps iconName with volume (`volume-mute`/`volume-low`/`volume-high`, ControlBar.tsx:456-460) — crossing a level threshold makes the speaker (the icon's dominant mass) jump horizontally by up to 3px. Visible layout twitch in a core control on every mute toggle and 0.5 crossing.
What correct looks like: Anchor the speaker at one x across all four levels (center on the speaker, not the union bbox), waves appended at the sheet's fixed offsets (+11.45/+14.45/+17.45).
Confidence: HIGH — Verification: read + python3 extents over design/figma/Icon.svg (cell-relative speaker anchor identical at −7.45 in all four groups; ported x offsets differ by up to 3.0).

### F-002 [S4]
Where: ui/src/components/icon/icons.tsx:316-327 and 371-381
Code:
```
  "menu": (
    <>
      <path d="M4 7H5" />
      <path d="M8 7H20" />
```
```
  "lists": (
    <>
      <path d="M4 7H5" />
      <path d="M8 7H20" />
```
What is wrong: `menu` and `lists` are byte-identical six-path glyphs (nub M4 xH5 + line M8 xH20 at y 7/12/17 in both). The header's own convention says "Shared consts serve registry aliases (same design glyph, two vocabulary names) without duplicating path data" (:188-189) — `folder`/`profile` follow it (folderGlyph/personGlyph), `menu`/`lists` don't.
Why it matters: Duplicated geometry that can drift independently on the next re-mine; violates the file's stated rule; misleading double provenance (one cites Controls.svg "Icon Wrapper_4", the other sheet Frame 1_34 — same output, so at least one citation is decorative).
What correct looks like: `const listRowsGlyph = (…); "menu": listRowsGlyph, "lists": listRowsGlyph` — or delete one name.
Confidence: HIGH — Verification: read (both entries quoted side by side); sheet `lists` group matched to the ported coordinates exactly.

### F-003 [S4]
Where: ui/src/components/icon/icons.tsx:338-346
Code:
```
  "more": (
    <>
      <path d="M6 12L6 12" strokeWidth={3} />
      <path d="M12 12L12 12" strokeWidth={3} />
      <path d="M18 12L18 12" strokeWidth={3} />
    </>
  ),
```
What is wrong: Dots are encoded as zero-length subpaths (`M6 12L6 12`) relying on round linecaps to paint them. SVG 2 makes zero-length subpath rendering with round caps explicit, but SVG 1.1 left it implementation-defined; renderers without round-cap zero-length support (some print/export pipelines, older librsvg) paint nothing.
Why it matters: Latent portability debt for a chrome glyph (Player.svg title-bar ⋯); all major browsers handle it today.
What correct looks like: `<circle cx="6" cy="12" r="1.5" fill="currentColor" stroke="none" />` — explicit, renderer-independent.
Confidence: MEDIUM — Verification: read; no renderer matrix run (out of scope) — the browser path is fine.

## 4. Per-function table (part1 has no functions; rows = consts + registry entries)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `IconName` union | 94-184 | REAL | — | 76 names, grouped with per-group provenance comments; exhaustiveness enforced by Record type. |
| `folderGlyph` | 191-194 | REAL | — | Shared alias const, cited (Vector 23_2). |
| `infoGlyph` | 199-205 | REAL | — | Detail-marked stem/dot, sw 2 verbatim; mono-inert (icon.css verified: detail styled only under --two-tone). |
| `wandGlyph` | 210-215 | REAL | — | Consumed by `customize-player` (part2 range). |
| `personGlyph` | 220-225 | REAL | — | Shared by `profile` (part1) + `artist`? (artist inlines its own paths :455-461 — near-duplicate of personGlyph head+shoulders with an extra note; part2's call). |
| `equalizerGlyph` | 228-237 | REAL | — | 3 sliders; consumed by `equalizer` (part2 range). |
| `glyphs` entries: play/pause/stop/next/previous | 240-275 | REAL | — | Filled transport glyphs; two-tone full-accent per right-click menu rows 1-4. |
| volume-mute/low/medium/high | 277-310 | REAL | F-001 | Speaker drift defect (measured). |
| fullscreen | 312-314 | REAL | — | Four corner marks, single path, M-chained. |
| menu | 316-327 | REAL | F-002 | Duplicate of lists. |
| close | 329-336 | REAL | — | From misnamed sheet group (verified). |
| more | 338-346 | REAL | F-003 | Zero-length dot encoding. |
| search | 348-350 | REAL | — | Single path, ellipse+handle. |
| sort | 352-359 | REAL | — | Two arrows. |
| list | 361-369 | REAL | — | Chips + thin lines (sw 1.28571 verbatim). |
| lists | 371-381 | REAL | F-002 | Duplicate of menu; sheet-verified translation. |
| grid | 383-391 | REAL | — | 2×2 rounded squares. |
| thumbnail | 393-401 | REAL | — | 2×2 r1.25 tiles (View Options.svg). |
| add | 403-410 | REAL | — | Plus. |
| check | 412-414 | REAL | — | Mono check; two-tone box variant lives in part2 twoToneGlyphs. |
| chevron-up/down/left/right | 416-421 | REAL | — | Four directions. |
| folder | 423-424 | REAL | — | Alias const. |
| playlist | 426-437 | REAL | — | Lines + accent play triangle. |
| queue | 439-448 | REAL | — | Accent pill + two base lines. |
| profile | 450-451 | REAL | — | Alias const. (Chunk boundary: part2 starts at `artist`, :452.) |

## 5. Test honesty (Icon.test.tsx, 27 tests — sibling file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "keeps every glyph inside the 24×24 grid" (:135) | YES | Numeric bounds assertion. |
| "volume family shares the mined speaker and adds one wave per level" (:161) | NO for F-001 | The test abstracts ALL numbers (`d.replace(/-?\d+/g,"#")`) and its comment asserts the drift is "exactly like the design sheet" — refuted by my sheet measurement (sheet anchors speaker at identical cell-relative x; the port does not). The test was written to tolerate the defect. |
| "pins mined sheet geometry — regression guard" (:198) / "pins w26.a07" (:222) | YES | Raw path-data pins. |
| "registry holds exactly the doc-named icons (no invented icons)" (:47) | YES | Name-set equality vs doc 08 §1.5/§1.6 catalogs. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "paths are translated from sheet coordinates so each glyph's bounding-box centre lands on (12, 12) — the same centring the sheet itself uses" | icons.tsx:60-63 | FALSE for the volume family | Sheet centers glyphs on fixed 44px-cell centers with an identical speaker anchor; bbox-centering moves the speaker per level (F-001). True for single-shape glyphs. |
| "Shared consts serve registry aliases … without duplicating path data" | icons.tsx:186-189 | FALSE | menu/lists duplicate (F-002). |
| "Registry: 76 names (32 wave-1 frozen + 44 a07)" | icons.tsx:57 | REAL | Union count = 76 (counted: :94-184). |
| "the volume family lives in the sheet under a misnamed 'close' component" | icons.tsx:25-27 | REAL | Sheet groups verified: name=close value=none/low/medium + name=volume value=high. |

## 7. Dead / duplicated / unreachable code
- `menu`/`lists` duplication (F-002). No dead names in part1 range: all part1 names are in the 76-name consumed registry (Icon renders any of them; consumers vary).

## 8. Dependencies & flags
Pure data module (ReactElement literals, no runtime logic). Consumed by Icon.tsx (mono) and part2's two-tone getters. No feature flags.

## 9. Open questions for the next agent
- Part2 owner: does `artist` (:452+) inline a near-copy of `personGlyph`'s head+shoulders instead of composing it? (Looks like it from the :455-461 paths — same circles/arcs with an eighth note added.)
- Does any renderer in the e2e/visual pipeline (playwright webkit?) fail the zero-length `more` dots? (F-003.)
- (Resolved while writing: `volume-medium` IS consumed — PlayerScreen.tsx:1502 NowPlayingMark + CustomizePanels.tsx:817. Not dead.)
