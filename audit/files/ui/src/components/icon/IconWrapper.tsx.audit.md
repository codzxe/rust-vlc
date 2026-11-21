# AUDIT: ui/src/components/icon/IconWrapper.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 58 / 1 (IconWrapper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (AppShell.tsx:669 `<IconWrapper size="lg" active={isActive}>` and :683 — the navigation rail on every screen) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read full file + icon.css:109-160 (the wrapper rules) · `rg -n "IconWrapper" ui/src` (production consumers: AppShell ×2, both `size="lg"`; tests + barrel otherwise) · Icon.test.tsx:400-437 read (the size/active pins) |

## 1. One-paragraph verdict
A correct, tiny presentational span that does exactly what its header promises and no more: three hit-area presets with their CSS counterparts present (24×24 / 30×32 / 44×44 in icon.css:122-135), a token-driven hover/focus backdrop matching the frame's "lift the background, not the ink" rule (:137-145), and an active accent state chained to the same token as selected menu rows (:150-153). The header's own w26.k04 disclosure is accurate and matches the tree: production mounts ONLY `lg` (AppShell's rail, both call sites explicit), while `sm`/`md` remain documented frozen-compatible presets pinned by tests. The one defect is that the DEFAULT size is `"md"` — a preset no production consumer uses — so any future caller that omits `size` silently gets the unused 30×32 frame instead of the production-proven 44px target.

## 2. What actually works (verified)
— Reachable and load-bearing: every AppShell rail stop wraps its glyph in `IconWrapper size="lg"` (AppShell.tsx:669, :683), and appshell.css:214 sizes the stops to the 44px contract this component provides.
— CSS counterparts all exist and match the header's claims: rx4 + transparent rest background (icon.css:109-118), the three size classes (:122-135), hover/focus backdrop via `--color-surface-hover` (:139-145), active accent via `--color-accent-icon` (:150-153), reduced-motion transition kill (:156-159).
— Test-pinned: Icon.test.tsx:400-437 pins the default class, all three size classes, and the active class (structure assertions).
— Presentational discipline holds: no interactivity in the wrapper itself; the header correctly instructs hosts to carry the accessible name.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/icon/IconWrapper.tsx:30-31, 36
Code:
```
export function IconWrapper({
  size = "md",
```
```
  /** Hit-area size. @default "md" (the 30×32 Icon Wrapper.svg frame) */
  size?: IconWrapperSize;
```
What is wrong: the default size is `md` — one of the two presets production never mounts (both AppShell call sites pass `size="lg"` explicitly). The header itself documents that "production currently mounts only lg (AppShell rail)".
Why it matters: a future caller omitting `size` gets the 30×32 preset that has zero production validation (no host, no visual test) instead of the proven 44px target; defaults should point at the exercised path.
What correct looks like: either default to `"lg"` (the production-proven hit area) or make `size` required so every caller chooses consciously.
Confidence: HIGH — Verification: read + consumer grep (both call sites explicit lg).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| IconWrapper | 30-58 | REAL | S4 | class composition correct; F-001 unused-preset default; sm/md documented frozen-compatible (w26.k04) + test-pinned |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Icon.test.tsx:400 default md class | YES | class assertion (also pins the F-001 default) |
| :417 all three size classes | YES | per-size class assertions |
| :432 active class | YES | class assertion |
Structure-only tests are adequate for a presentational span; the visual grammar is pinned from the consumer side (AppShell.test.tsx:247 "every stop is a ≥44px hit frame"). All green: baseline ui-vitest.log.txt.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "production currently mounts only lg (AppShell rail); sm/md stay as frozen-compatible additive API (pinned by tests)" | :19-21 | REAL | AppShell.tsx:669/:683 are the only consumers, both lg; Icon.test.tsx:417 pins all sizes |
| "on hover a subtle rounded backdrop (30×32 r4, white 6% in the frame)" | :6-8 | REAL | icon.css:109-145 (rx4, --color-surface-hover backdrop) |
| "sm 24×24 · md 30×32 · lg 44×44" | :12-14 | REAL | icon.css:122-135 |
| "wrap it in the interactive element … and give that element the accessible name" | :23-24 | REAL (discipline) | AppShell's rail buttons carry the labels |

## 7. Dead / duplicated / unreachable code
- `sm`/`md` presets: production-dead, deliberately frozen (documented + test-pinned) — the F-001 default is the actionable part.

## 8. Dependencies & flags
- icon.css only. No engine dependency, no feature flags.

## 9. Open questions for the next agent
- IconButton is the interactive sibling (header :24) — its auditor should confirm it composes this wrapper or duplicates the hit-area CSS.
