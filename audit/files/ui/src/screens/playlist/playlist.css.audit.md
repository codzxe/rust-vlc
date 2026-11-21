# AUDIT: ui/src/screens/playlist/playlist.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 640 / 43 rule blocks (+1 @media) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: App.tsx:319 `/playlist` route → screens/playlist/PlaylistScreen.tsx:58 `import "./playlist.css"`; PlayerScreen.tsx:1543 → PlaylistPanel.tsx:47; AppShell.tsx:852 → NewPlaylistDialog.tsx:28. NO shipped binary mounts it: W14 proved apps/desktop-tauri cannot compile at HEAD (app.rs:301 `spawn_session_pump` E0425), so no `lumen-tauri` binary exists (wave-14/SUMMARY.md). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | rg class-name census in ui/src (all 36 hooks used, see §2); sed -n quote verification; token existence check (27/27 `var(--…)` resolve in tokens/generated/tokens.css); rg over design/figma/*.svg + design/figma-fidelity-spec.md (hex pins verified); rg importers of playlist.css. |

## 1. One-paragraph verdict
Styling-only sheet for the three playlist surfaces (queue panel, /playlist library page, create-playlist dialog). Every class hook defined here is consumed by reachable TSX (36/36 census, §2), every `var()` resolves to a defined token (27/27), and the six raw-hex "frame pin" exceptions each carry an inline stylelint-disable and were cross-verified against the actual Figma sources (`#1B1B1B`, `#2D2D2D` + `stroke-dasharray` in design/figma/new playlist.svg; `#6B6B6B`, `#303030` in design/figma/media.svg; hover `#1c1c1c` in design/figma-fidelity-spec.md §2). The three findings are documentation/debt-level: the header's blanket "every value is a semantic token" claim is contradicted by its own six documented hex pins, one comment mis-states its own applied size (13px vs the 15px token), and one @media rule is a dead duplicate of a shared rule in list.css. No functional defect found; hostile questions (dead buttons, backend wiring) live in the TSX layers this sheet styles, not here — the styled components are honest about seams (e.g. PlaylistScreen.tsx:20-21: "Play in the context menu is disabled until a catalog command exists").

## 2. What actually works (verified)
- Class-hook census: `rg -l "<class>" --glob '!*.css'` for all 36 `lumen-playlist*`/`lumen-plpage*`/`lumen-plnew*`/`lumen-plrow--current` hooks → every one referenced by PlaylistScreen.tsx / PlaylistPanel.tsx / NewPlaylistDialog.tsx / options/*.tsx (e.g. `lumen-plrow--current` at PlaylistPanel.tsx:340, `lumen-playlist__sr` at :372, `aria-pressed` styled at :191). No dead selectors.
- Token integrity: extracted 27 distinct `var(--…)` from this sheet → all defined in ui/src/tokens/generated/tokens.css (negative control with a bogus name flagged correctly).
- Frame-pin honesty: `#2d2d2d`/`#1b1b1b`/dasharray present in design/figma/"new playlist.svg"; `#6b6b6b` in media.svg; `#303030` in media.svg; hover `#1c1c1c` in design/figma-fidelity-spec.md.
- Baseline: ui vite build OK (audit/runs/baseline/INDEX.md row 22) compiles this sheet; vitest 1705 passed (row 23) exercises the components whose DOM this sheet styles (PlaylistPanel.test.tsx, options/MediaList.test.tsx query `.lumen-plo-*`/`.lumen-plrow*` hooks). Visual pixel fidelity: UNVERIFIED — no visual-regression run in baseline.
- Cross-sheet contracts verified: list.css:208 `.lumen-lrow{position:relative}` makes the absolute actions overlay (this sheet :121-132) anchor correctly; list.css:484-490 owns the shared reduced-motion rule (see F-003).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/playlist.css:14-16 (contradicted at :432, :483, :527, :543, :567, :624)
Code:
```
 * The 2026-11-09 v2 aesthetic (sunset veil, sun disc, chips, tape) is retired;
 * every value is a semantic token. Raw px is structural only (frame-measured
 * where marked: dialog column, tile sizes, drop-zone pitch).
```
What is wrong: "every value is a semantic token" is false. The file itself pins six raw hex colors (e.g. `border: 2px dashed #2d2d2d;` at :432, `background: #1b1b1b;` at :483, `background: #1c1c1c;` at :527/:624, `color: #6b6b6b;` at :543, `background: #303030;` at :567), each individually disclosed via `stylelint-disable-next-line color-no-hex`, but the header blanket claim is wrong.
Why it matters: The header is the sheet's contract statement; a reader auditing token discipline ("semantic-tokens-only" is an enforced repo rule for TSX) would stop at line 16 believing there are no hex exceptions. Six exceptions exist.
What correct looks like: Header should read "every value is a semantic token except six documented dark-only frame pins (see the stylelint-disabled rules)".
Confidence: HIGH — Verification: read (sed -n '430,433p' etc.) + rg census of `#[0-9a-fA-F]\{6\}` in file.

### F-002 [S4]
Where: ui/src/screens/playlist/playlist.css:43-46
Code:
```
/* Player.svg sidebar title grammar: 13px medium label. */
.lumen-playlist__title {
  margin: 0;
  font-size: var(--font-size-time); /* 15px — the sidebar/menu medium class */
```
What is wrong: The rule's lead comment says "13px medium label" while the applied token `--font-size-time` is 15px (tokens/generated/tokens.css:74) — and the inline comment on the same property says "15px". The two comments on consecutive lines disagree.
Why it matters: Frame-fidelity comments are the only traceability from this sheet to the Figma sources; a wrong size in the comment poisons every future frame-diff against it.
What correct looks like: Either the comment cites the frame's actual measured size with the scale conversion ("frame 13px at 0.785 scale ≈ …") or it says 15px like the inline one.
Confidence: HIGH — Verification: read + sed -n '74p' tokens.css.

### F-003 [S4]
Where: ui/src/screens/playlist/playlist.css:505-509
Code:
```
@media (prefers-reduced-motion: reduce) {
  .lumen-playlist .lumen-lrow__actions {
    transition: none;
  }
}
```
What is wrong: Dead duplicate. components/list/list.css:484-490 already sets `.lumen-lrow__actions { transition: none; }` inside the identical `@media (prefers-reduced-motion: reduce)` block, globally (the queue panel's rows are `.lumen-lrow__actions` too). No rule in playlist.css re-introduces a `transition` on those actions, so the higher-specificity re-statement changes nothing.
Why it matters: Duplication that looks load-bearing invites a false "this sheet owns reduced-motion for the dock" belief; when the shared grammar changes, this stale copy silently diverges.
What correct looks like: Delete the block, or keep it only if playlist.css ever adds its own transition to `.lumen-playlist .lumen-lrow__actions` (it currently does not).
Confidence: HIGH — Verification: read both files; rg 'transition' playlist.css → no other hit.

## 4. Per-function table (rule blocks; CSS has no functions — grouped by section)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-playlist` (+header/title/count/controls/note/footer/empty/list/sr) | 26-115 | REAL | — | All hooks used (PlaylistPanel.tsx:179,243,372); sr-only clip pattern correct |
| `.lumen-playlist__list .lumen-vrow:hover` → transparent | 103-106 | REAL | — | Overrides list.css:153 (0,2,0) at 0,3,0 — order-independent, comment discloses |
| `.lumen-playlist .lumen-lrow__actions` overlay | 121-148 | REAL | — | Anchored by list.css:208 `position:relative`; pointer-events none/auto choreography correct (invisible actions unclickable) |
| `.lumen-playlist .lumen-plrow--current .lumen-lrow__title` | 152-154 | REAL | — | Hook at PlaylistPanel.tsx:340 |
| `.lumen-plpage` (+head/title/count/actions) | 161-202 | REAL | — | Hook at PlaylistScreen.tsx:325,470 |
| `.lumen-plpage__backrow`/`__back` | 208-220 | REAL | — | Negative margin tie-in documented |
| `.lumen-plpage__panel.lumen-card` | 226-233 | REAL | — | Specificity-beats-primitive pattern, disclosed |
| `.lumen-plpage__list` (+title override 246-248) | 235-253 | REAL | — | stylelint-disable no-descending-specificity disclosed; scoped override |
| `.lumen-plpage__row-wrap` / `.lumen-playlist__list .lumen-context-menu` | 259-270 | REAL | — | Width+height pass-through fix, cross-file class verified (options css/tsx) |
| `.lumen-dialog.lumen-plnew` (+head/title/section/label/helper/row/input) | 277-346 | REAL | — | NewPlaylistDialog.tsx:28 importer; width+max-width duplication defensive, not a bug |
| `.lumen-plnew .lumen-field__control` skin | 335-346 | REAL | — | 27.5px/5px frame-scaled values; fractional px fine in browsers |
| `.lumen-plnew__thumb` (+tile/rule/drop/file-input/drop-text/drop-tile/files-count/actions/cancel) | 349-503 | REAL | — | focus-visible outlines present; disabled cursor honest (371-373); dark/light theme split explicit |
| `@media (prefers-reduced-motion)` | 505-509 | DEAD DUPLICATE | S4 | F-003 |
| w26.i10 pins (1) hover #1c1c1c + selected mirrors | 524-534 | REAL | — | Frame value verified in design/figma-fidelity-spec.md; mirrors keep order-independence |
| (2) watched title #6b6b6b + hover mirrors | 541-549 | REAL | — | #6B6B6B verified in media.svg; options.css:274-284 is the base rule |
| (3) grid artwork well #303030 | 561-568 | REAL | — | media.svg carries #303030 ×4; light face uses token — disclosed as sanctioned |
| (4) file-menu width + check-in-box | 580-601 | REAL | — | 265px pin + 15×15 ::after box; cross-file hooks verified in options/ |
| (4b) kebab margin 18px | 608-610 | REAL | — | Hook `.lumen-plo-mrow__menu` at options.css:337 |
| (5) arrow-key grid active row | 618-629 | REAL | — | `data-active` + aria-activedescendant painted only under container focus (APG convention) |
| (6) roving wrappers flex | 634-640 | REAL | — | `__listwrap`/`__view` hooks used in PlaylistScreen |

## 5. Test honesty
No tests target CSS directly. Indirect: PlaylistPanel.test.tsx / PlaylistScreen.test.tsx / options/MediaList.test.tsx assert on class hooks this sheet defines (e.g. `.lumen-plo-mrow--watched` at MediaList.test.tsx:35) — they verify DOM structure, not computed styles. Would a broken CSS break them? No — vitest/jsdom does not apply styles. Style application is only covered by the UNRUN visual harness (playwright.visual.config.ts) and UNRUN `npm run lint:css` (not in baseline rows 21-24).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every value is a semantic token" | playlist.css:15 header comment | FALSE (6 hex pins) | F-001; sed-verified |
| "~66px r10 rows" library grammar | playlist.css:10 + PlaylistScreen.tsx:5 | REAL | list.css:238 `.lumen-lrow--comfortable{height:66px}`, `border-radius:10px` |
| Contracts list (ROW_HEIGHTS, `.lumen-plrow--current`, sr-only "Now playing", `playlist-empty-open-media` CTA, aria-labels) | playlist.css:18-22 | REAL | PlaylistPanel.tsx:256 (data-testid), :340, :372; engine-truth aria-pressed at :186-191 |
| w26.i10 frame pins (#1c1c1c/#6b6b6b/#303030/#1b1b1b/#2d2d2d/265px) | comments :432-:624 | REAL | rg in design/figma/*.svg + figma-fidelity-spec.md (§2) |

## 7. Dead / duplicated / unreachable code
- F-003's @media block (dead duplicate of list.css:484-490). Nothing else: every selector matched a TSX/other-css consumer.

## 8. Dependencies & flags
Depends on tokens/generated/tokens.css (27 vars, all defined), components/list/list.css (vrow/lrow grammar), components/form/form.css (field control), components/menu (context-menu popup), screens/playlist/options.css (plo-* pins rely on `:root:not([data-theme="light"])` prefix for order-independence — verified pattern). No feature flags. stylelint config: ui/stylelint.config.js (color-no-hex exceptions inline).

## 9. Open questions for the next agent
- PlaylistScreen.tsx / PlaylistPanel.tsx / NewPlaylistDialog.tsx are assigned elsewhere — verify the "persisted" claim (playlistsStore.ts `lumen.playlists.v1` localStorage, spot-checked REAL by me: localStoragePlaylistsPersistence at :78-87) and the queue panel's engine wiring (PlaylistProvider, not audited here).
- The axe "3.2:1 dark" contrast figure at :53-54 could not be reproduced by hand (#828282 on #191919 ≈ 4.6:1) — likely computed against the raised panel; visual/a11y agent should re-run axe on the built UI (REQUEST-ORCH-RUN territory, not essential).
