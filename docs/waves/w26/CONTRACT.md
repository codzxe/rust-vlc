# Wave 26 — The Common Contract (binding for every w26 agent)

Status: ACTIVE (Lead, 2026-11-11). You received a Task ID like `w26.a07`.
This contract + `design/figma-fidelity-spec.md` (the W25 spec) + your assigned
reference frames are your ONLY design authorities. When they conflict with
anything else you find in the repo, these two documents win.

## 1. Mission

Implement the W25 Figma redesign (`design/figma/*` = source of truth) across
every screen of the Lumen UI, as ONE coherent product flow:

1. **Onboarding** (first-run gate) →
2. **Player** (home; empty-state hero; canvas + title bar + icon rail +
   playlist sidebar + control bar) →
3. **Playlists / Discovery / media-info / convert** (working screens) →
4. **Audio & video tools** (EQ, compressor, spatializer, widener, visualizer,
   colors, crop, essential, HDR) →
5. **Settings** (13 areas, App/Media/System sidebar groups) →
6. **Dialogs** (open network stream, keyboard shortcuts) and system surfaces
   (menus, context menus, dropdowns, profiles/presets, miniplayer, music mode,
   light mode, hide-UI, customize panel).

Same routes, same behavior contracts, same tests — new skin composed to the
Figma frames (W25 spec §0). The Solstice sunset/tape/keycap aesthetic is DEAD:
remove it wherever your owned files still carry it.

## 2. The design language in one paragraph (details: W25 spec §1–§2)

Near-black `#090909` surfaces, `#191919` panels, `#404040` hairline borders,
`#c6c6c6`/`#8f8f8f` quiet text, ONE sparing orange accent `#d37918`
(pills/active/fill; pressed `#ab6314`, hover `#e8851a`, glow tint `#371f06`).
Compact desktop density: 10–15px type, sentence case, tabular timecodes.
Rounding lives on CONTROLS (r4 inputs, r6 buttons/menu items, r10–12 menus,
r12–14 cards), panels are square-edged — this is NOT a bubbly UI. Video canvas
stays dark in BOTH themes. Font stack: `Inter, -apple-system, "Segoe UI",
Roboto, sans-serif` (Nunito is forbidden). The player empty state is an
onboarding-style hero, never a blank void.

## 3. Reference frames — YOUR map (PNG renders at `design/figma-renders/`,
## SVG sources at `design/figma/`; PNG name = SVG name with spaces→`_`)

| Surface | Route | Reference frames (read ALL of yours, PNG + SVG) |
|---|---|---|
| Onboarding | `/onboarding` | `Onboarding.png`, `Onboarding-1..10.png` (11-frame step flow) |
| Player core | `/player` | `Player.png`, `Player-1..6.png` |
| Player light | `/player` (light mode) | `Player_-_Light.png` |
| Music mode | `/player/music` | `Player_-_Music.png`, `Player_-_Music-1.png` |
| Hide-UI | `/player` (idle) | `Player_-_hide_UI.png` |
| Customize | `/player` (panel) | `Player_-_customize.png`, `-1..8.png` (9 states) |
| Miniplayer | `/miniplayer` | `Miniplayer.png` |
| Settings | `/settings/:area` | `Settings_-_global.png(-1)`, `-interface(-1)`, `-privacy`, `-playback(-1)`, `-video(-1)`, `-audio(-1)`, `-subtitles(-1)`, `-controls`, `-library`, `-network`, `-performance`, `-recording`, `-extensions` (13 areas) |
| Audio tools | player panel | `Audio_-_Equalizer.png`, `EQ.png`, `EQ_presets.png`, `Audio_-_Compressor_Settings.png`, `Audio_-_Spatializer_Settings.png`, `Audio_-_Stereo_Widener.png`, `Audio_Visualizer.png`, `audio_value.png` |
| Video tools | player panel | `Video_-_Colors.png`, `Video_-_Crop.png`, `Video_-_Essential.png`, `HDR_Options.png` |
| Playlists | `/playlist` | `playlist.png`, `new_playlist.png`, `Sorting_Options.png`, `View_Options.png`, `thumbnail.png`, `media.png` |
| Subtitle flows | `/playlist` dialogs | `subtitle_one.png`, `subtitle_multiple.png`, `sub_filename.png`, `Add_Subtitles_to_Playlist/` renders (4 folder states) |
| Menus | components | `menu.png`, `context_menu.png`, `file/folder/playlist_context_menu.png`, `right-click_menu.png`, `option.png`, `setting_category.png`, `dropdown.png`, `Profiles.png`, `Setting_Preset.png` |
| Control parts | components | `Controls.png`, `Button.png`, `Toggle.png`, `Values.png(-1)`, `Current_Time.png`, `Duration.png`, `volume.png`, `play.png`, `origin.png`, `target.png`, `sidebar.png`, `Icon.png`, `Icon_Wrapper.png` |
| Dialogs | `/open-network-stream`, `/keyboard-shortcuts` | `Open_network_stream.png`, `Keyboard_Shortcuts.png` |
| System sheets | reference only | `Components.png`, `Designs.png` |
| No direct frame | `/discovery`, `/convert`, `/media-info`, 404 | Use the design language (§2) + nearest grammar (panel r12–14, rows, section headers) |

## 4. Hard rules for parallel work (violations break 20-agent waves)

1. **Work in `/home/z/my-project/rust-vlc`** (shared checkout). **NEVER run
   any `git` command** — the Lead commits/pushes after each wave. **NEVER run
   `npm install`/`npm update`** or touch lockfiles.
2. **Edit ONLY your owned files** (listed in your task). Found a problem in a
   file you don't own? Write it in your lane log — do not fix it yourself.
   Shared files (`ui/src/App.tsx`, `ui/src/i18n/messages.ts`,
   `ui/src/components/index.ts`, `themes/tokens.json`, `ui/src/tokens/generated/*`)
   have exactly ONE owner per wave (named in the wave plan).
3. **Component public APIs are FROZEN** (wave 1): keep exported
   props/interfaces signature-compatible; restyle internals + CSS. New exports
   are additive only.
4. **Semantic tokens only** in components/CSS: `var(--color-accent)` etc.
   (eslint `semantic-tokens-only`). Raw hex ONLY in `themes/tokens.json` (the
   token lane's job) and in per-frame pixel-perfect SVG ports where a value is
   frame-measured — mark those with a `/* frame: <Frame.svg> */` comment.
5. **i18n**: user-visible strings flow through `useMsg()` + `MessageKey`.
   Reuse existing keys where the copy already exists. Genuinely new keys go in
   a NEW file `ui/src/i18n/keys/<task-id>.ts` exporting
   `export const keys = { "screen.eq.preamp": "Preamp" } as const;` — the Lead
   registers them post-wave. NEVER edit `ui/src/i18n/messages.ts` yourself.
6. **CSS**: screens own their `<screen>.css`; the base sheet is
   `ui/src/theme/lumen.css` (theme lane owns it). No inline `style=""` except
   computed/dynamic values. Stylelint standard config must stay green for your
   files (`npm run lint:css` may be run scoped).
7. **Allowed commands**: `npx tsc --noEmit` (shared), `npm run test -- <your
   test files>` (vitest scoped), `npx eslint <your files>`, `node
   /home/z/my-project/scripts/dev-shots.mjs <name>:<route> --out
   /tmp/shots-<task-id> --port 5173` (dev server already running on :5173),
   and `z-ai vision -p "<prompt>" -i <shot> -i <reference>` for comparisons.
   Do NOT kill/restart the dev server; do NOT run `npm run dev`/`build`
   yourself (Lead builds between waves).
8. **Tests**: keep your owned tests passing; DOM snapshots may be regenerated
   for intentional redesign changes (`-u`, commit the result). Never weaken
   assertions. New visual structure → add/extend tests where cheap.
9. **No fabricated claims**: report only what you actually ran, with real
   results. Unknown facts → `[VERIFY]`.
10. **No Rust changes** (UI wave: TS/CSS/React only), no `schemas/`,
   `product.toml`, `Cargo.*`, `.github/` edits — exceptions are explicitly
   granted per-lane in the wave plan (e.g. the settings-seed lane).

## 5. The comparison loop (how you prove "similar")

For every screen you own: screenshot it
(`dev-shots.mjs`, 1440×900), then run
`z-ai vision -p "Compare image 1 (current UI) to image 2 (target Figma design). List concrete visual gaps: layout regions, colors, radii, typography, spacing, component anatomy. End with VERDICT: MATCH or GAPS." -i /tmp/shots-<task-id>/<name>.png -i design/figma-renders/<frame>.png`
Iterate fix→screenshot→compare until VERDICT: MATCH (or only font-identity
deltas remain — documented, not blocking). Also spot-check 1024×768 (nothing
may clip). Record iteration count + last verdict in your lane log.

## 6. Deliverables per agent

1. Code/CSS/tests in your owned files (compiling, lint-clean, tests green).
2. Lane log: `docs/waves/w26/logs/<task-id>.md` — what you built, decisions,
   the comparison-loop record (iterations + verdicts), files touched, open
   issues, fixes needed by other lanes.
3. Final report to the Lead: ≤120 words, terse, structured (DONE / GAPS LEFT /
   FILES / NEEDS-FROM-LEAD).

## 7. Quality bar (what the harsh reviewers will grade in later waves)

Pixel-credible layout proportions; correct tokens everywhere; hover/focus/
active/disabled states; real iconography (no emoji); tabular timecodes; dark +
light both coherent; keyboard reachable; aria labels; no clipped text at
1440×900 or 1024×768; no Solstice remnants (sunset, tape, keycaps, Nunito,
`data-backdrop` tiers); i18n complete; zero console errors on your routes.

## 8. Self-push protocol (Amendment 1 — 2026-11-13, Lead; binding for waves h–m)

User directive: every lane pushes its own work to GitHub the moment it is
completed — the GitHub copy is the durability record, not the sandbox. §4.1's
"never run git" is REFINED, not revoked: agents still never run local `git`
commands in the shared checkout (no branch/commit/merge — parallel-lane
safety). Instead, every lane pushes its deliverable through the GitHub REST
API onto its OWN branch. The Lead merges lane branches into `main` after each
wave.

Per lane, after the deliverable file(s) are final in the shared checkout:

```bash
TOK="$GH_TOKEN"                                # supplied in your task prompt
REPO=codzxe/rust-vlc; BR="w26/<your-lane-id>"  # e.g. w26/h03
SHA=$(curl -s -H "Authorization: token $TOK" \
  https://api.github.com/repos/$REPO/git/ref/heads/main \
  | python3 -c 'import json,sys;print(json.load(sys.stdin)["object"]["sha"])')
curl -s -X POST -H "Authorization: token $TOK" \
  https://api.github.com/repos/$REPO/git/refs \
  -d "{\"ref\":\"refs/heads/$BR\",\"sha\":\"$SHA\"}"   # 422 = exists, fine
B64=$(base64 -w0 <your-deliverable-file>)
curl -s -X PUT -H "Authorization: token $TOK" \
  https://api.github.com/repos/$REPO/contents/<deliverable-path> \
  -d "{\"message\":\"<lane-id>: <one-line>\",\"content\":\"$B64\",\"branch\":\"$BR\"}"
# for a file you MODIFIED (not new): first GET it on $BR to obtain its blob
# sha, and include "sha" in the PUT body. Then verify:
curl -s -H "Authorization: token $TOK" \
  "https://api.github.com/repos/$REPO/contents/<path>?ref=$BR" -o /dev/null \
  -w '%{http_code}\n'    # must print 200
```

Token discipline (absolute): the token exists ONLY in your shell variable and
the `Authorization` header. It must NEVER appear in any file under the repo,
any lane log, any rating/review output, your branch names, commit messages, or
your report back to the Lead. Never `echo` it.

## 9. Tooling notes (Amendment 1 — 2026-11-13)

- Dev server: Lead-owned at `http://127.0.0.1:5173` (vite binds 127.0.0.1
  explicitly; `localhost`/`::1` are NOT reachable in this sandbox). Still
  never restart/kill it; if it answers anything other than 200, say so in your
  lane log — the Lead restarts it.
- `z-ai vision` may 429 under wave load: retry twice with 60s backoff; if the
  third attempt fails, fall back permanently to measurement — extract geometry
  from the SVG sources in `design/figma/` (path bboxes + attributes), probe the
  DOM (Playwright geometry dumps), and pixel-sample your screenshots. Note the
  fallback in your deliverable. g-lane verifiers proved this path works.
- In-flow states (wizard steps, open menus, tools dialog, customize panel,
  hide-UI idle) are reached with small Playwright scripts: navigate, click
  through, or `page.evaluate(() => window.dispatchEvent(new CustomEvent('lumen:open-tools')))` /
  `'lumen:open-customize'`, then screenshot. Working examples: the seam tests
  in `ui/src/screens/player/PlayerScreen.test.tsx` and e2e specs in `ui/e2e/`.
