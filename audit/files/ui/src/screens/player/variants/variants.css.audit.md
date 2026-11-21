# AUDIT: ui/src/screens/player/variants/variants.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 722 / 48 rule groups (≈75 selectors) |
| File verdict | UNREACHABLE (real, token-clean, test-pinned stylesheet; no shipped binary reaches it) |
| Reachable from shipped entry point? | NO — only shipped host is `lumen-tauri`, which cannot compile at HEAD (W14-A01: E0425 `spawn_session_pump` called at apps/desktop-tauri/src/app.rs:301, defined nowhere; baseline run #2 exit 101). Dev/vite + vitest DO import it: MiniPlayer.tsx:64 `import "./variants.css"`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "variants.css" ui/src` → imported by MiniPlayer.tsx:64, read as text by variants.test.tsx:33; `rg -n "lumen-mini__|lumen-music__|data-variant="` → every selector class maps to MiniPlayer.tsx / MusicMode.tsx / ControlBar.tsx / AppShell.tsx renders; token loop `rg -c -- "--<name>:" src/tokens/generated/tokens.css` → all 31 referenced custom properties defined; `rg -n "data-control=" controls/` → audio/subtitles/settings/queue all exist (ControlBar.tsx:725,736,747,762); `rg -n "MediaSurface" src/` → mounted only in PlayerScreen.tsx:1062,1098 (full variant); sed verification of every quote below. |

## 1. One-paragraph verdict
This is a genuinely built, frame-pinned stylesheet: every one of its ~75 selectors matches a class some component actually renders (verified selector-by-selector), all 31 custom properties it references are defined in the generated token sheet, and its frame-measured values are pinned by tests that read the sheet (variants.test.tsx). But it is unreachable from any shipped binary — the desktop shell it styles cannot compile (W14-A01) — and it styles two "media" surfaces that ship with no media element inside them: the miniplayer's 16:9 "video area" and the music-mode "artwork" are both honest placeholders (disclosed in the components), so the two variants this sheet exists for cannot display the media they play. The header's "every color below is a semantic token" claim is contradicted by five stylelint-disabled raw-hex rules in the same sheet, three of which hard-code copies of the dark theme's token values that will drift if the theme retunes.

## 2. What actually works (verified)
— selector→render census: every `.lumen-mini__*`/`.lumen-music__*` class is emitted by MiniPlayer.tsx/MusicMode.tsx (e.g. `.lumen-mini__video` MiniPlayer.tsx:189, `.lumen-music__artwork` MusicMode.tsx:137), `.lumen-player__controls[data-variant="music"]` is written by PlayerScreen.tsx:1187, `> .app-shell__sidebar` matches AppShell.tsx:647/698. No orphan selector found.
— token integrity: `rg -c -- "--<name>:" ui/src/tokens/generated/tokens.css` for all 31 names (space-track-inset, radius-card, shadow-card, color-accent-*, font-size-sidebar, …) → every one defined (≥1). No undefined var() reference.
— executed tests over this sheet: baseline run #23 (ui-vitest.log.txt: 1705 passed / 1 skipped, 136 files) includes variants.test.tsx which readFileSync's this file and pins 20+ values; baseline #22 (build) compiles the sheet.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/player/variants/variants.css:59-75 (+ consumers MiniPlayer.tsx:185-195, MusicMode.tsx:137-147)
Code:
```
/* Video area — the mini canvas seam, and the overlay transport's stage.
 * Dark in BOTH themes (the picture-frame rule, W25 spec §1 — same override
 * pattern as the full canvas in player.css). */
.lumen-mini__video {
  position: relative;
  flex: none;
  aspect-ratio: 16 / 9; /* frame: thumbnail 373×209 */
```
What is wrong: The sheet's two hero surfaces are dead video surfaces. `.lumen-mini__video` is rendered with `data-placeholder="engine-wiring-4.H1"` and contains only a screen-reader note (`MiniPlayer.tsx:185-195`: "no media element until card 4.H1"); MiniPlayer never mounts MediaSurface in any branch. `.lumen-music__artwork` is an `aria-hidden` glyph placeholder ("until the engine reports artwork" — the engine has no artwork lane). So even in a hypothetical compiled GUI, a user playing a file in the miniplayer or music variant watches an empty 16:9 box / glyph square for the entire session; the only variant that mounts a `<video>` (full) mounts it src-less for file sources (PlayerScreen.tsx:1078-1093 comment: "file sources mount with no src").
Why it matters: This is the Wave-15 hostile question answered for the variants lane: the compact and music "player variants" cannot display any video at all, by construction. The styling is meticulous for a surface that renders nothing.
What correct looks like: either the 4.H1 render seam ships (canvas/LUF1 consumer in the mini variant) or the frame copy and this sheet's video-area rules are marked `[PROPOSAL]` like other unfinished seams.
Confidence: HIGH — Verification: read MiniPlayer.tsx (full), MusicMode.tsx renders, PlayerScreen.tsx:999-1110; MediaSurface grep shows mounts only at PlayerScreen.tsx:1062/1098.

### F-002 [S3]
Where: ui/src/screens/player/variants/variants.css:6-8 (header) vs 72-75, 450-453, 505-508, 540-543, 545-548
Code:
```
 * gradient treatment is GONE — every color below is a semantic token; raw
 * px appears solely for frame-measured structural dimensions (marked
 * `frame:`), the C1 player.css precedent.
```
and (lines 540-543):
```
:root[data-theme="light"] .lumen-music__title {
  /* stylelint-disable-next-line color-no-hex -- stays-dark stage ink: the dark theme's --color-content-bright #eaeaea (player.css now-title precedent, w26.d2). */
  color: #eaeaea;
}
```
What is wrong: The header asserts "every color below is a semantic token"; five rules in the same sheet set raw hex colors under `stylelint-disable-next-line color-no-hex`. Worse, 540-548 hard-code the dark theme's *current* token values (#eaeaea = --color-content-bright, #8f8f8f = --color-content-secondary) — copies, not references. If the dark theme retunes those tokens, the light-theme music ink silently drifts.
Why it matters: A policy claim in the file header that the file body violates is the exact "comment says X, code does Y" pattern this audit exists to catch; the duplicated token values create cross-file drift the token system was built to prevent.
What correct looks like: a `--color-*-stays-dark` pair of real tokens (the comments themselves say "token-lane handoff") and a header that says "semantic tokens, plus 5 documented stays-dark handoffs".
Confidence: HIGH — Verification: read; sed-verified both quotes; token values cross-checked in src/tokens/generated/tokens.css.

### F-003 [S4]
Where: ui/src/screens/player/variants/variants.css:717-722
Code:
```
.lumen-player[data-variant="music"][data-sidebar="open"] .lumen-music__stage {
  position: relative;

  /* frame: (rail 52 + dock 300) / 2 — Music-1 art center 960/1920; clamp: art left edge ≥ dock right edge */
  left: max(-176px, calc(452.75px - 50vw));
}
```
What is wrong: Magic constants 176/452.75 encode rail 52 (owned by appshell.css) + dock 300 (owned by line 655 of this sheet) with no custom-property seam; the comment admits "re-pin if the rail retunes".
Why it matters: Three files must stay in numeric lockstep by hand; a rail retune silently mis-centers the music artwork.
What correct looks like: `--rail-w`/`--dock-w` custom properties consumed here.
Confidence: HIGH — Verification: read; constants traced to appshell rail + line 655 dock.

### F-004 [S4]
Where: ui/src/screens/player/variants/variants.css:423 and 463-467
Code:
```
.app-shell__frame:has(.lumen-player[data-variant="miniplayer"]) > .app-shell__sidebar {
  display: none;
}
```
What is wrong: `:has()`, `color-mix()` (line 465) and `backdrop-filter: blur(50px)` (line 644) need a recent WebKitGTK; the only shipped engine for this UI (tauri webview) is unbuildable at HEAD, so whether these render or silently drop in the product is UNVERIFIED. `backdrop-filter` on WebKitGTK has a long history of no-op bugs.
Why it matters: The music band's "95% + 50px blur" chrome and the miniplayer's sidebar suppression are visual pillars of both variants; a silent drop would leave the miniplayer with a wrong shell layout.
What correct looks like: verify on the target webview once the host compiles, or provide the no-blur fallback the sheet lacks.
Confidence: MEDIUM — Verification: read; UNVERIFIED at runtime (no browser/tauri in sandbox; baseline #2 exit 101).

## 4. Per-function table (rule groups)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-music .lumen-icon-button, .lumen-mini .lumen-icon-button` radius | 40-43 | REAL | — | rx4 override; scoped fix over button.css, disclosed |
| `.lumen-mini` card | 45-57 | REAL | — | token-only colors; frame-pinned 391px; `margin:auto` host-owned note |
| `.lumen-mini__video` (+light override) | 62-75 | REAL (styles a placeholder) | S2 | F-001: no media element ever inside; raw hex #2b2b2b |
| `.lumen-mini__controls` | 81-86 | REAL | — | flex column wrapper (MiniPlayer footer) |
| `.lumen-mini__controls .lumen-mini__seek` + track/fill/thumb/buffer/[data-empty] | 95-151 | REAL | — | re-styles shared slider classes; width calc(+2px)/−1px margins documented; honest empty state |
| `.lumen-mini__transport` (+focus-within/hover, play, skip--prev/next) | 162-206 | REAL | — | opacity never removes a11y tree (verified note); ±75px frame pins |
| `.lumen-mini__play` hover/active/disabled | 213-239 | REAL | — | honest disabled state mirrors controls.css rule |
| `.lumen-mini__title` (+[data-empty]) | 245-262 | REAL | — | ellipsis + honest empty tone |
| `.lumen-mini__meta`, `-chip`, `-text`, `-toggle`(+[data-active]) | 265-303 | REAL | — | all rendered conditionally on real data |
| `.lumen-mini__queue` family (queue/head/title/meta/summary/rows/handle/tools/add-popup/list/focus/empty) | 310-408 | REAL | — | handle rule matches Row.tsx:79 (draggable→handle); add-popup fixed-position escape of overflow:hidden is correct; focus-visible ring = W25 §2 |
| `.app-shell__frame:has(...) > .app-shell__sidebar` | 423-425 | REAL | S4 | F-004 `:has()` support; matches AppShell.tsx:647/698 DOM |
| `.lumen-music__art` (+light, ::before glow) | 431-469 | REAL | S4 | color-mix UNVERIFIED on target webview |
| `.lumen-music__stage` / `__artwork` (+light, glyph) | 471-513 | REAL (styles a placeholder) | S2 | F-001: artwork is aria-hidden glyph placeholder |
| `.lumen-music__title` / `__caption` (+light ink overrides) | 517-548 | REAL | S3 | F-002: raw hex copies of dark tokens |
| music seek tick field (track/fill/thumb) | 570-630 | REAL | — | dual-anchor (footer + strip portal) matches PlayerScreen.tsx:1187 + AppShell.tsx:847; textures documented as NOT audio data (honesty) |
| music chrome band | 640-645 | REAL | S4 | backdrop-filter UNVERIFIED |
| `.lumen-player[data-variant="music"] .lumen-player__sidebar` + empty | 654-661 | REAL | — | matches player.css dock seam |
| `.lumen-music__playlist-toggle` | 666-671 | REAL | — | rendered MusicMode.tsx:124 |
| music timecode tone | 676-678 | REAL | — | matches controls.css .lumen-controls__time |
| music `[data-control]` reduction | 694-699 | REAL | — | all four data-control values exist (ControlBar.tsx:725/736/747/762) |
| stage shift clamp | 717-722 | REAL | S4 | F-003 magic constants |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| variants.test.tsx frame-pin describes (482-594, reads this file as text) | YES for value drift, NO for rendering | String pins like `margin: 20.4px 0 0` (test line 548) fail on any retune — mutation-sensitive — but no test renders the sheet, so a broken selector (typo'd class) still passes; F-001's placeholder surfaces pass all of them |
| AppShell.test.tsx:295-347 (sidebar suppression) | YES | jsdom render + stylesheet string check; would catch a dropped `:has()` rule as text, not engine behavior |
| baseline #23 vitest run | — | 1705 passed/1 skipped includes these files (audit/runs/baseline/ui-vitest.log.txt) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every color below is a semantic token" | variants.css:6-8 (file header) | FALSE | 5 raw-hex rules (F-002) |
| "the + wired to the real enqueue seam — w26.i04" | variants.css:19-20 | REAL | MiniPlayer.tsx:536-547 MenuPopup → enqueueEntries → playlist.playLater (392) |
| "prev/next are REAL playlist commands" | variants.css:158-160 | REAL | MiniPlayer.tsx:219/241 `playlist.prev()/next()` |
| "no stage waveform … engine audio levels do not exist yet, so nothing here claims to be audio data" | variants.css:27-28, 566-568 | REAL | tick field is pure repeating-gradient texture (574-589) |
| design matrix rows for miniplayer/music variants | docs/feature-matrix.toml (W16 lane) | not re-audited here | out of this file's scope |

## 7. Dead / duplicated / unreachable code
No orphan selectors (full census above). The whole sheet is unreachable from shipped binaries (host crate E0425, W14-A01) — reachable only via vite dev/vitest. Duplication: light-theme stays-dark overrides repeat the same #2b2b2b block three times (72-75, 450-453, 505-508) — candidate for one shared class or token (see F-002).

## 8. Dependencies & flags
Consumes only generated design tokens (src/tokens/generated/tokens.css — all 31 verified present) + shared component classes (slider, lrow, icon-button, controls, app-shell). No new tokens declared. Stylelint-config-standard present but `lint:css` script is the custom scripts/lint-css.mjs (see that file's audit); baseline run #24 did NOT include lint:css — the hex disables are therefore UNVERIFIED against a real stylelint run.

## 9. Open questions for the next agent
- PlayerScreen.tsx#part2 auditor: confirm `stripContext` portal actually places `.lumen-player__controls` as a DIRECT child of `.app-shell__strip` (line 641's second anchor) — I verified the class names but not the portal ordering.
- Whoever audits MediaSurface.tsx (not in my list): does the `<video>` element ever receive file-source bytes in the tauri host, or is src-less the terminal state (F-001's blast radius)?
