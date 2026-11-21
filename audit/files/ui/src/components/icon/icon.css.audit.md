# AUDIT: ui/src/components/icon/icon.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 160 / 16 rule blocks |
| File verdict | REAL (best-tested CSS in the wave's sample: Icon.test.tsx parses THIS sheet and asserts its token values; geometry test bounds every glyph) |
| Reachable from shipped entry point? | YES as web-UI source: `Icon.tsx` applies `lumen-icon`/`--two-tone`/`--strong`/`--disabled`; `IconButton.tsx:29` carries `lumen-icon-host`; `IconWrapper.tsx` carries `lumen-icon-wrapper` — all reachable from every screen. Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `grep -rn "lumen-icon-hit" src/` (ZERO setters — F-001); `grep -rn "lumen-icon-host" src --include=*.tsx` (IconButton.tsx + ControlBar.tsx:840/:1073 — comment cites 814/1040, F-002); `grep -rn 'variant="strong"\|variant="max"' src --include=*.tsx` (strong: MusicMode.tsx:139, SubtitlePickRow.tsx:69 — live; max: tests only, as documented); `sed -n '95,135p' Icon.test.tsx` (cssRule() reads the real file); tokens.css light-block cross-check (no `--color-accent-strong` in light — the sheet's documented token-lane debt is real); baseline #22/#23 |

## 1. One-paragraph verdict
The cleanest sheet in my assignment: 16 rules, every class produced, state narrative documented with stylelint justifications, dead host variants already pruned in-tree with grep-provenance (w26.k04), and — uniquely in this wave — a test file that reads the actual CSS and asserts the token values (`cssRule(".lumen-icon-host--open .lumen-icon")` must contain `var(--color-accent-icon)`), plus a glyph-geometry test bounding every icon to the 24×24 grid. The two-tone light-mode override is an honest, in-file-documented token-lane debt that I verified is real (tokens.css's light block genuinely lacks `--color-accent-strong`). Three S4 warts: a dead hit-area seam nobody opted into, a stale line-number citation, and a frozen-but-unused `--max` variant.

## 2. What actually works (verified)
— verified by `Icon.test.tsx:117-131` (baseline 1705-pass) → cssRule() loads this sheet and pins 4 token-bearing rules; the pruned host variants' absence is ALSO test-pinned — the CSS contract is executable here.
— verified by producers grep → `lumen-icon` (Icon.tsx:76), `--two-tone` (Icon.tsx, MenuItem-driven), `--strong` (MusicMode.tsx:139, SubtitlePickRow.tsx:69), `--disabled` (Icon.tsx:76), `lumen-icon-host` (IconButton.tsx:29), `lumen-icon-host--open` (ControlBar.tsx:840/:1073), wrapper presets (IconWrapper).
— verified by tokens.css cross-check → light block omits `--color-accent-strong` (my menu.css/tokens audits confirm) — the :57-64 override exists for a true gap, with the deletion condition written down.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/icon/icon.css:13-15
Code:
```
  /* Icon Wrapper.svg: ~48px hit area. Padding reaches it without inflating
   * layout — interactive hosts opt in via --lumen-icon-hit. */
  padding: var(--lumen-icon-hit, 0);
```
What is wrong: `--lumen-icon-hit` has zero setters in the entire repo — no host opts in; the documented mechanism is dead. The goal (48px hit) was later solved by a second mechanism (`.lumen-icon-wrapper--lg` 44×44, :132-135) — two designs for one problem, one abandoned in place.
Why it matters: dead seam in the base rule of the most-included sheet; a future host setting the var would get asymmetric padding without layout intent review.
What correct looks like: delete the custom property and comment, or one host adopts it to prove the seam.
Confidence: HIGH — Verification: repo-wide grep, zero hits outside this line.

### F-002 [S4]
Where: ui/src/components/icon/icon.css:32-33
Code:
```
 * w26.k04 (j04-6): the selected and disabled HOST variants were pruned —
 * zero producers anywhere (only --open is ever applied, ControlBar.tsx:814/1040);
```
What is wrong: the cited line numbers are stale — `lumen-icon-host--open` is produced at ControlBar.tsx:840 and :1073 at HEAD (verified by sed). The claim ("only --open is ever applied") remains true; the citation rots.
Why it matters: this file's culture is provenance-by-citation; wrong pointers erode the convention's value.
What correct looks like: cite by symbol (`ControlBar's two popup triggers`) not line numbers, or update.
Confidence: HIGH — Verification: grep + sed of both sites.

### F-003 [S4]
Where: ui/src/components/icon/icon.css:86-88
Code:
```
.lumen-icon--max {
  stroke-width: var(--stroke-icon-max);
}
```
What is wrong: zero production users (only Icon.test.tsx pins it) — "catalog-reserved, frozen-compatible additive API" per the comment. A deliberate keep, but it is shipped dead weight whose only consumer is its own test.
Why it matters: another glyph-strength level in the public surface that no frame uses; the test that pins it would pass meaninglessly if the token changed to any value.
What correct looks like: acceptable as documented; if the catalog reservation lapses, delete rule + test together.
Confidence: HIGH — Verification: production grep (variant="max" — none) + test grep.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-icon` | 7-19 | REAL | S4 | base; pointer-events:none (hosts own hits); F-001 dead hit var |
| `-host:hover/:active` | 23-29 | REAL | — | bright ink feedback |
| `-host--open` | 35-37 | REAL | — | accent; live producer (ControlBar ×2) |
| `-host:focus-visible` | 40-42 | REAL | — | focus feedback |
| `--two-tone .lumen-icon__detail` | 53-55 | REAL | — | #AB6314 details; verified vs frames narrative |
| light-mode detail override | 57-64 | REAL | — | documented token-lane debt; gap verified real |
| `__detail--bright` | 68-70 | REAL | — | rare #D37918 marks |
| `--strong` / `--max` | 82-88 | REAL / FROZEN-DEAD | S4 | strong live (2 sites); max reserved (F-003) |
| `--disabled` | 92-95 | REAL | — | leaf rule; Icon.tsx:76 applies |
| `@media reduced-motion` ×2 | 98-102, 156-160 | REAL | — | kills color/bg transitions |
| `.lumen-icon-wrapper` (+sm/md/lg, hover/focus, --active) | 109-153 | REAL | — | the w26.a07 second hit mechanism; sizes 24/30×32/44 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Icon.test.tsx cssRule assertions (:125-130) | YES | reads THIS file's text and asserts token values — a real CSS contract test, the only one in my 27 files |
| Icon.test.tsx glyph geometry (issue #35) | YES | bounds all mined glyphs to the 24×24 grid |
| Icon.test.tsx class assertions (:121,:270-287) | YES | variant/disabled/two-tone class application pinned |
| IconButton.test.tsx:38 | YES | host class composition pinned |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "interactive hosts opt in via --lumen-icon-hit" | icon.css:13-14 | FALSE | zero setters (F-001) |
| "only --open is ever applied, ControlBar.tsx:814/1040" | icon.css:33 | REAL (claim) / stale (citation) | applied at :840/:1073 (F-002) |
| "exists ONLY because the light theme block in tokens.css does not override --color-accent-strong" | icon.css:60-63 | REAL | tokens.css light block verified — no accent-strong |
| "production uses strong only; kept as frozen-compatible additive API" | icon.css:80-81 | REAL | grep: strong ×2 production, max ×0 |

## 7. Dead / duplicated / unreachable code
Dead: `--lumen-icon-hit` seam (F-001), `--max` rule (F-003, deliberate). Duplicated conceptually: hit-area via padding var vs wrapper presets (F-001). All other rules live.

## 8. Dependencies & flags
Tokens: 8 distinct, all defined (incl. --stroke-icon-strong/max, --color-accent-strong dark-only). No flags.

## 9. Open questions for the next agent
- The cssRule() test pattern should be cloned to the other primitive sheets (menu.css/list.css audits this wave showed class-name-only tests) — cheapest available upgrade to CSS verification without Playwright.
- If the tokens lane lands light `--color-accent-strong`, delete the :57-64 override — the file already says so; next tokens wave should check the promise is executable.
