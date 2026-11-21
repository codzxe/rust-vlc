# AUDIT: ui/src/screens/player/player.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 606 / 34 rule blocks (+1 cross-sheet `:has()` rule) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as CSS: PlayerScreen.tsx:136 `import "./player.css"` → App.tsx:303-305 `/player`+`/player/music`+`/miniplayer` routes → GatedShell → PlayerRoute → PlayerScreen. The UI bundle builds (baseline INDEX #22 `npm run build` OK). NO as product: no GUI binary exists — the tauri host crate cannot compile at HEAD (W14-A01, E0425 `spawn_session_pump`), so this sheet only ever runs in the browser/dev/vitest lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -l "lumen-player__<class>" src --glob '*.tsx'` for all 36 class fragments (every class has ≥1 TSX consumer; dock/panel/now classes → PlayerScreen.tsx only; `--strip` → AppShell.tsx:847 strip host + AppShell.test.tsx:670); `rg -n "lumen-player__media|__canvas|__topbar" src` (MediaSurface.tsx:82,176; PlayerScreen.tsx:993,1010); Read appshell.css:268-277 (`.app-shell__sidebar[hidden] { display: none }` exists — the `:has()` rule at player.css:603 has a real counterpart); `rg -n '"--' src/tokens/generated/tokens.ts` (all 31 tokens used by this sheet exist in the generated semantic set); `sed -n '304,320p' appshell.css` vs `sed -n '278,297p' player.css` (verbatim duplication confirmed); `rg -n "data-sidebar|data-chrome=" PlayerScreen.tsx` (950, 947 — `:has()`/`[data-chrome]` selectors match real attributes); `rg -l "clip-path: inset\(50%\)"` (the visually-hidden pattern is hand-copied in 8 CSS files). NOTE: bash `cat`/`sed` output in this sandbox strips a literal `[h` from displayed lines (terminal escape artifact — first suspected selector corruption was FALSE); every quote below was re-verified with the Read/Grep tools. |

## 1. One-paragraph verdict
This is real, reachable, disciplined design-token CSS: every one of its 36 class selectors has a TSX consumer, every `var(--…)` it consumes exists in `src/tokens/generated/tokens.ts`, the two raw hexes carry inline stylelint-disables with documented rationale, and the one cross-sheet rule (`.app-shell__frame:has(> .app-shell__sidebar[hidden])…`) targets classes/attributes that verifiably exist in AppShell.tsx/PlayerScreen.tsx. The hostile findings are duplication and comment rot, not fakeness: the docked-search-field block is a property-for-property copy of the shell sidebar's (drift guaranteed), the visually-hidden pattern is hand-copied twice in this file and 8 times repo-wide, and two comments still claim the frame/decode half of the canvas is "engine-wired nowhere until 4.H1" — stale since w31.4 wired `session.set-frame-channel` into MediaSurface.tsx. The sheet's product-relevance caveat is the wave's standing one: with the desktop host crate unbuildable, this CSS runs only in browser/dev builds, where the canvas region it styles is an honest empty dark surface (the frame `<canvas>` mounts only inside the shell — MediaSurface.tsx:106-108).

## 2. What actually works (verified)
— verified by `rg -l "lumen-player__dock-search-input" src --glob '*.tsx'` → 1 file (PlayerScreen.tsx); all 36 classes → ≥1 consumer; no dead selectors in this sheet.
— verified by Grep `\--color-surface-deep|--radius-ring|--shadow-overlay|…` in `src/tokens/generated/tokens.ts:31` → all tokens this file consumes (surface/chrome/deep/panel/raised/hover, content primary/secondary/tertiary/bright, stroke-border/icon, accent/accent-icon, radius inset/card/card-lg/ring, shadow-overlay, font family/size/weight set, space grid/inset) exist in the generated semantic token list.
— verified by baseline INDEX #22 (`npm run build` → tsc+vite OK) and #24 (`npm run tokens:check` PASS) — the sheet type-checks' consuming TSX compiles and the token contract it relies on is enforced by a committed gate.
— verified by Read `appshell.css:271-274` → `.app-shell__sidebar[hidden] { display: none; }` exists, so player.css:603's `:has(> .app-shell__sidebar[hidden])` premise (shell sidebar collapses via `hidden`) holds; AppShell.tsx:701 `hidden={!sidebarOpen}` sets the attribute.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/player.css:278-297
Code:
```css
.lumen-player__dock-search-input {
  box-sizing: border-box;
  width: 100%;
  padding: var(--space-grid) var(--space-inset);
  font: inherit;
  font-size: var(--font-size-caption);
  color: var(--color-content-primary);
  background: var(--color-surface-panel);
  border: 1px solid var(--color-stroke-border);
  border-radius: var(--radius-inset);
}
```
What is wrong: This block, its `::placeholder`, and its `:focus-visible` follow-up are a property-for-property copy of `.app-shell__sidebar-search-input` (+pseudo rules) at `src/components/appshell/appshell.css:304-323` — 3 rule blocks duplicated verbatim across two sheets that the comments themselves call "the same panel family" (player.css:109-112 "the same panel family as the shell's playlists sidebar"). The dock section also mirrors the shell's modes cluster, grid, and card rules with only class-name substitutions.
Why it matters: Any fix to one copy (focus ring offset, contrast, hit size) will not propagate; the two "one family" panels drift silently. This is exactly how the a11y/visual-regression bugs this codebase's own gates exist to prevent get re-introduced.
What correct looks like: A shared `.lumen-sidebar-*` (or utility) rule set in a common sheet consumed by both the shell sidebar and the player dock, with per-surface overrides only where the frames genuinely differ (the ink steps the comments already document).
Confidence: HIGH — Verification: read both files side by side (sed ranges above); diff is class names only.

### F-002 [S3]
Where: ui/src/screens/player/player.css:417-427 and 572-582
Code:
```css
.lumen-player__sr-title {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  padding: 0;
  overflow: hidden;
  clip-path: inset(50%);
```
What is wrong: `.lumen-player__sr-title` and `.lumen-player__sr-note` are byte-identical visually-hidden blocks inside one file, and `rg -l "clip-path: inset\(50%\)"` shows the same hand-copied pattern in 8 CSS files (surface.css, form.css, playlist.css, player.css, onboarding.css, settings.css, setting-field.css, hotkeys.css). No shared `.sr-only` utility exists anywhere in `src/`.
Why it matters: The visually-hidden recipe is accessibility-critical (a typo in one copy — e.g. the classic missing `white-space: nowrap` or `overflow` — silently breaks screen-reader exposure on exactly one screen). 10+ copies is 10+ chances for that, and no single place to fix it.
What correct looks like: One `.lumen-visually-hidden` utility in the theme/base sheet, reused.
Confidence: HIGH — Verification: read both in-file blocks; rg file list pasted in Commands run.

### F-003 [S4]
Where: ui/src/screens/player/player.css:429-433
Code:
```css
/* ---- video canvas (Player.svg x=427..1920, y=40..1026) ------------------
 * Near-black in BOTH themes — the picture-frame rule (W25 spec §1: light
 * mode keeps the canvas #2b2b2b–#333). w26.i02 (h03 #2): the canvas hosts
 * the real media surface (./states overlays sit above it, z-index 1); the
 * decode/frames half stays engine-wired nowhere until 4.H1.
 */
```
What is wrong: The claim "the decode/frames half stays engine-wired nowhere until 4.H1" is stale: `src/components/player/MediaSurface.tsx` (w31.4) mounts a frame `<canvas>` inside the desktop shell, binds it via the `session.set-frame-channel` command (MediaSurface.tsx:147-152) and blits LUF1 records. Same rot in the file header, line 12-13 ("the honest engine-unwired placeholder until card 4.H1").
Why it matters: Comment rot on THE wave-critical question (does the GUI display video?) actively misleads: an auditor or developer reading this sheet concludes the video path is unwired by design, when the code (unreachable only because the host crate does not compile — W14-A01) is wired. Doc-vs-code drift is this audit's core fraud vector.
What correct looks like: Comments updated to: video element mounts with url sources; frame canvas mounts in-shell via w31.4; the whole surface is dead in every build that exists today because `lumen-tauri` does not compile.
Confidence: HIGH — Verification: read MediaSurface.tsx:106-184 (FrameCanvasSurface + set-frame-channel bind).

### F-004 [S4]
Where: ui/src/screens/player/player.css:458-461 and 507-510
Code:
```css
:root[data-theme="light"] .lumen-player__canvas {
  /* stylelint-disable-next-line color-no-hex -- frame: Player - Light.svg — the picture-frame rule (W25 spec §1): the video canvas stays dark (#2b2b2b–#333) in BOTH themes; every surface token flips in light mode, and no stays-dark semantic token exists yet (token-lane handoff, w26.a13 lane log). */
  background: #2b2b2b;
}
```
What is wrong: Raw hex values (`#2b2b2b`, and `#eaeaea` at 507-510) escape the sheet's own "Semantic tokens only (§3.2)" rule. Both are acknowledged, documented, lint-suppressed token-lane debt ("no stays-dark semantic token exists yet").
Why it matters: Acknowledged debt, but debt with a blast radius: every future stays-dark surface (dialogs over video, subtitle backdrop) must re-hardcode or the light theme will wash them out; the workaround is invisible to `tokens:check`.
What correct looks like: A `--color-surface-video` (stays-dark) semantic token in the generated set, consumed here; delete both hexes and both stylelint-disables.
Confidence: HIGH — Verification: read both blocks; token list (tokens.ts:31) confirmed to contain no stays-dark surface token.

### F-005 [S4]
Where: ui/src/screens/player/player.css:22-23 vs 94-95
Code:
```css
 * Proportions (Player.svg @1920 → 1440 render): title 40→30, rail 50→38,
 * sidebar 377→283 (W26 mission pins the docked sidebar at 200–220), controls
```
vs
```css
 * is the playlists BROWSER (Player.png's single sidebar), 377px @1920 → 283px
 * at our 1440 render — the h03 measurement pinned the old 216px dock at
 * 18.6% of the window vs the frame's 22.2% left-chrome share. Square-edged
```
What is wrong: The file header still cites the superseded "mission pins the docked sidebar at 200–220" while the dock section (line 94-95) says that pin is superseded by the 283px frame measurement ("the old W26 mission pin '~200–220px' is superseded"). Two different stated truths about the same width in one file.
Why it matters: Internal contradiction invites a future "fix" back toward 200-220px; a reader cannot tell which comment is authoritative without the lane logs.
What correct looks like: Header cites the 283px frame-measured pin and drops the stale mission range.
Confidence: HIGH — Verification: read both comment ranges; the implemented width is 283px (line 101).

## 4. Per-function table (rule blocks)
| Rule (selector) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-player` | 34-47 | REAL | — | Root; `height:100vh` neutralized in-shell by appshell.css:490-493 `.app-shell__main > .lumen-player { height: 100% }` (verified AppShell.tsx:837 renders children directly — direct-child selector matches). |
| `.lumen-player--strip` | 53-60 | REAL | — | Strip-host reuse; asserted by AppShell.test.tsx:670. Neutralizes position/height/overflow/background; deliberately keeps flex+font cascade so controls.css scope rules match the portaled bar. |
| `[data-chrome-part]` transition | 66-68 | REAL | — | Attribute set by PlayerScreen (`data-chrome=` at 947/950); reduced-motion override 80-84 pairs with useChromeAutoHide. |
| `[data-chrome="hidden"]` cursor + parts | 70-78 | REAL | — | Fade+pointer-none+visibility; matches useChromeAutoHide.ts contract. |
| `.lumen-player__sidebar` | 96-107 | REAL | — | 283px frame-measured width; deep surface; used by PlayerScreen. |
| `.lumen-player__dock-head` | 118-125 | REAL | — | 40px structural. |
| `.lumen-player__dock-title` | 127-139 | REAL | — | content-secondary ink per frame. |
| `.lumen-player__dock-meta` | 144-151 | REAL | — | 36px row + hairline. |
| `.lumen-player__dock-count` | 153-156 | REAL | — | caption/tertiary. |
| `.lumen-player__dock-create` (+hover/focus/label) | 161-193 | REAL | — | Full-bleed row button; focus-visible outline -2px inside. |
| `.lumen-player__dock-rows` | 199-207 | REAL | — | Scroll region; nested inside `__sidebar` overflow:auto — double scroll containers, benign (inner is the only one with content to scroll). |
| `.lumen-player__dock-list` | 209-213 | REAL | — | List reset. |
| `.lumen-player__dock-empty` | 215-220 | REAL | — | Empty state. |
| `.lumen-player__dock-row` (+states/label) | 222-257 | REAL | — | 36px pitch; hover/focus lift to bright. |
| `.lumen-player__dock-back/forward/more` | 265-269 | REAL | — | `flex:none` spacers; styles owned by button.css. |
| `.lumen-player__dock-search` | 273-276 | REAL | — | Wrapper. |
| `.lumen-player__dock-search-input` (+pseudo) | 278-297 | REAL | S3 | F-001 verbatim copy of appshell.css:304-323. |
| `.lumen-player__dock-modes` (+aria-pressed) | 303-312 | REAL | — | In-subtree token remap for pressed mode; mirrors shell modes cluster (part of F-001 family duplication). |
| `.lumen-player__dock-grid` | 316-323 | REAL | — | 2-up grid; mirrors `.app-shell__plgrid` (appshell.css:437). |
| `.lumen-player__dock-card` (+states) | 325-349 | REAL | — | Mirrors `.app-shell__plcard` family (F-001). |
| `.lumen-player__dock-card-thumb` | 351-360 | REAL | — | 32×32 structural. |
| `.lumen-player__dock-card-label` | 362-368 | REAL | — | Caption + ellipsis. |
| `.lumen-player__body` | 374-380 | REAL | — | Anchors floating panels. |
| `.lumen-player__main` | 384-390 | REAL | — | Column. |
| `.lumen-player__topbar` | 396-406 | REAL | — | Rendered only by layouts declaring a top region; default layout does not (PlayerScreen.test.tsx:142 asserts null) — rule is live for custom layouts. |
| `.lumen-player__title` | 408-413 | REAL | — | Label scale. |
| `.lumen-player__sr-title` | 417-427 | REAL | S3 | F-002 duplicate #1. |
| `.lumen-player__canvas` | 434-441 | REAL | — | Near-black base; comment rot F-003 in its header. |
| `.lumen-player__media` | 448-456 | REAL | — | Absolute-fill object-fit:contain; applies to BOTH the `<video>` (MediaSurface.tsx:82) and the frame `<canvas>` (:176) — correct for canvas as replaced element. |
| light override `__canvas` | 458-461 | REAL | S4 | F-004 hex #2b2b2b. |
| `.lumen-player__now` | 468-477 | REAL | — | Honest identity stand-in block. |
| `.lumen-player__now-glyph` | 479-490 | REAL | — | 72px ring chip. |
| `.lumen-player__now-title` (+light) | 492-510 | REAL | S4 | F-004 hex #eaeaea (507-510). |
| light `__now-glyph` icon ink | 512-514 | REAL | — | Token remap, documented contrast rationale. |
| `.lumen-player__controls` | 525-533 | REAL | — | 44px full-width band; ControlBar portals into it (strip host). |
| `.lumen-player__panel` (+--left/--right/data-emphasized) | 540-569 | REAL | — | Floating tool panels; `[data-emphasized]` accent edge (music lane hook, PlayerScreen.tsx:1511 cites it). |
| `.lumen-player__sr-note` | 572-582 | REAL | S3 | F-002 duplicate #2. |
| `.app-shell__frame:has(...)` rail rule | 603-606 | REAL | — | Cross-sheet reach, provenance-commented (lines 584-602); premise verified (appshell.css:272 `[hidden]` rule exists; AppShell.tsx:701 sets the attribute; PlayerScreen.tsx:950 sets data-sidebar). Cross-file edit is documented as lane-blocked handoff. |

## 5. Test honesty
No tests target this CSS directly. Indirect: AppShell.test.tsx:670 asserts the `--strip` class contract; PlayerScreen.test.tsx:142 asserts the default-layout topbar is NOT rendered (the `.lumen-player__topbar` rule stays live only for custom layouts — honest). Visual harness (visual-run.mjs) would catch geometry drift but is not part of the committed baseline evidence beyond the 4 ui checks (INDEX #24); CSS-rule-level assertions would fail if selectors were deleted only via the TSX-class greps an agent runs — no automated dead-CSS gate exists (see §7).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2); raw px appears solely for structural dimensions" | player.css:31-32 (self-claim) | REAL (with 2 documented exceptions) | Only raw colors are the two light-mode hexes (F-004), each inline-disabled + rationale; all other raw values are px geometry. |
| "the decode/frames half stays engine-wired nowhere until 4.H1" | player.css:433 (self-claim) | FALSE (stale) | MediaSurface.tsx:106-184 wires the frame canvas + set-frame-channel (w31.4). |
| "the honest engine-unwired placeholder until card 4.H1" | player.css:12-13 (self-claim) | FALSE (stale, same rot) | Same proof as above; the *effective* truth (no video in any existing build) holds for a different reason — host crate unbuildable (W14-A01). |
| Sidebar width 283px frame-measured; mission pin 200-220 superseded | player.css:94-101 vs 22 | Internally contradictory | F-005. |

## 7. Dead / duplicated / unreachable code
Dead: none (all 36 selectors consumed — see Commands run). Duplicated: F-001 (cross-sheet search field + family), F-002 (in-file ×2, repo ×8). Unreachable: nothing in-file; the whole sheet's product lane is unreachable (no GUI binary — W14-A01), but the sheet is exercised by the committed UI test/visual lane.

## 8. Dependencies & flags
Consumes only the generated semantic token set (`src/tokens/generated/tokens.css` via the global chain) — no external CSS deps. Depends at runtime on: appshell.css (`--strip` neutralization partner at AppShell.tsx:847, `.app-shell__main > .lumen-player` height patch, `.app-shell__sidebar[hidden]` premise for the `:has()` rule), button.css (IconButton styles for dock-back/forward/more), controls.css (portaled bar internals). No feature flags.

## 9. Open questions for the next agent
- Who owns the acknowledged "stays-dark token" handoff (w26.a13 lane log) and is it tracked anywhere executable, or only in comments? If the token lane is dead, F-004 is permanent.
- The dock family vs appshell sidebar family (F-001): the shell lane (k05/Lead) owns appshell.css per the contract note at 597-602 — a dedup needs a cross-lane owner.
- Does any committed visual baseline cover the light-mode canvas (the two hardcoded hexes)? If not, F-004's contrast rationale (1.3:1 claim at 501-506) is unverifiable.
