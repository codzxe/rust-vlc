# NEXT-STEPS — card 4.A1/4.A2 successors

Session 1 (agent P4-A1) delivered: Vite + React + TS-strict scaffold, vitest +
RTL, ESLint flat config with two local rules, mock command transport +
ClientProvider hook, routing skeleton for all doc 08 §1.6 routes, Playwright
config. typecheck/test/lint green.

Session 2 (agent P4-A2) delivered the token pipeline (doc 08 §3):
`ui/scripts/gen-tokens.mjs` (zero deps) syncing the crates/theme-tokens
output (`themes/generated/tokens.css`) into `src/tokens/generated/` (CSS copy
+ typed accessors + sha256), `ThemeProvider`/`useTheme` (dark/light/auto × 9
accents via `data-theme`/`data-accent` on `<html>`), the
`local/semantic-tokens-only` lint rule, a `/dev/tokens` sandbox fixture, 20/20
unit tests + 4/4 e2e (browsers were already cached; added the missing
`baseURL` to playwright.config.ts — latent 4.A1 gap). Decision record:
`src/tokens/README.md`.

Closure session (P4-A1 relaunch 2, 2026-10-23): chromium installed
(156.0.8078.4), smoke spec GREEN 1/1 (needed `baseURL` added to
playwright.config.ts — the scaffold config had omitted it), HTML reporter
added for the CI artifact, `.github/workflows/ui.yml` PROPOSED (Lead wires it
at merge), README brought to closure state. All four gates green.

## Do next (roughly in order)

> **4.B1 CLOSED** (agent P4-B1, sessions 1-2, commits `7c6eb1d` + `a79e9f7`
> + `1778be8`): ALL components landed — button trio (`Button` default/primary
> variants, sizes 28/32/48; `IconButton` mandatory `aria-label`; `Toggle`
> `role="switch"`+`aria-checked` per doc 08 §4.5, deviation recorded in
> `src/components/button/states.md`, micro labels ON/OFF at
> `--font-size-micro` = accent/content-tertiary, decorative aria-hidden) AND
> form inputs (`TextField`, `NumberField` with clamping steppers, `Checkbox`,
> `RadioGroup` with roving tabindex + arrow keys, `Switch` = Toggle re-export).
> Full §4.4 matrix audited (pressed `:active` skins added to form controls;
> pending NOT required on form inputs — §5 scopes it to command-dispatching
> components, recorded in `form/states.md`); token-only CSS; 86/86 tests,
> typecheck + lint clean. Open token asks (Lead, themes/tokens.json):
> `--radius-knob` 3px, `--radius-input` 5px, `--color-danger` — see worklog
> 4.B1 Handoffs.

1. ~~**4.A2 completion pass**~~ — DONE (closure session 3, agent P4-A2
   relaunch 2): full 9-accent × light/dark visual matrix E2E (18 tests,
   computed-style assertions vs values from themes/tokens.json) green;
   `npm run tokens:check` green (sha256 832da9d0… unchanged — no Rust-side
   change, no re-pin needed); worklog entry filed; Rust-consumer survey
   written into `src/tokens/README.md` (next item).
2. ~~**Rust-constants side**: survey consumers~~ — DONE, survey only, see
   `ui/src/tokens/README.md` § "Rust-constants consumer survey": zero
   current consumers of `crates/theme-tokens/src/generated/tokens.rs`;
   would-be consumers = renderer clear/letterbox color (today hardcoded
   `color::BLACK`; `backend.rs` already takes `background: Rgba` as a
   param — the ARCH-3-correct seam) + future subtitle-safe margins (no
   margin constants exist yet). Lead gates any engine-side dep.
3. **Settings wiring**: ThemeProvider currently defaults to dark + orange
   with NO persistence — wire the "Accent color" + theme settings through
   the settings schema (doc 07) when those cards land (4.E2); `ui/src/theme/`
   is the seam.
4. **ESLint rules v2**: promote the three inline rules in `eslint.config.js`
   into a proper local plugin (e.g. `tools/eslint-plugin-lumen/`) with real
   schema + tests; template-literal coverage for
   `local/semantic-tokens-only` (currently regex-grade on quasis only);
   Stylelint for plain `.css` files (doc 08 §3.4) — TokenSandbox.css is
   var()-only by construction, not by enforcement.
5. **Command client growth**: extend `src/client/transport.ts` — requestId
   allocation, pending-request map, event dispatch, stale-generation
   handling (doc 05 §4); keep the `src/generated/` import seam in one file.
   Add a `useCommand` hook. Real fixture replay comes from doc 05 §7 testkit.
6. **Screen stubs**: split ScreenPlaceholder into real per-screen modules as
   4.B primitives land. The `/dev/tokens` route is a dev fixture, NOT a
   product screen — do not copy its pattern into product screens.
7. **CI UI job** (Lead-owned at merge): propose `.github/workflows/ui.yml` —
   `working-directory: ui`, steps: `npm ci`, `npm run typecheck`, `npm run
   lint`, `npm run tokens:check`, `npm run test`, then Playwright with
   cached browser install.
8. **Directory drift note**: doc 08 §8 wants `src/bindings/` + `src/app/`;
   current scaffold has `src/generated/` (1.D4 output path — don't move it
   without updating `cargo xtask codegen`) and root-level `App.tsx`/
   `main.tsx`. Reconcile when components land.

## Honest caveats from session 2 (4.A2)

- `react-refresh/only-export-components` now also warns on `useTheme` beside
  `ThemeProvider` — same accepted provider/hook-pairing warning as 4.A1.
- vitest stubs raw `.css` imports, so the generated CSS text is also carried
  inside `tokens.ts` (`TOKENS_CSS`) for the snapshot test — the browserside
  cascade still loads the real `tokens.css` via `main.tsx`.
- `local/semantic-tokens-only` v1 misses: CSS-in-`<style>` tags, non-literal
  (computed) color strings, px on non-font-size/radius keys. Documented in
  the rule comment; v2 promotion tracked above.
- `themes/generated/tokens.css` was NOT regenerated this session (Rust side
  untouched, read-only) — the pipeline consumes the committed 1.B4 output.

## Honest caveats from session 1 (4.A1)

- Node 24.21.0 / npm 11.19.0; registry reachable (PONG 201ms).
- Vite 8 / vitest 5 / TS 6 / ESLint 10 are the current majors — if any behave
  oddly, pinning down one major is acceptable, record why here.

## 4.B2 — slider primitives (Slider / Seekbar / VolumeSlider)

Done, committed on `wip/p4-b2` (`ui/src/components/slider/**` only):

- `Slider` — generic controlled slider, horizontal + vertical (EQ §1.4
  anatomy). Pointer scrub with `setPointerCapture`, keyboard: arrows ±step,
  Shift = coarse ×10, PageUp/PageDown coarse, Home/End bounds.
- `Seekbar` — 28px track / inset-2 accent-tinted fill / 8×24 rx4 accent knob
  per design; engine-reported `buffered` ranges rendered as honest
  `surface-muted` sub-regions (no ARIA on them — §4.5 assigns none);
  caller-owned `pending` prop renders doc 08 §5.2 truthfully: knob locks at
  the requested target (`data-pending`, accent-strong) while the fill stays
  at engine position until the caller confirms via `value`.
- `VolumeSlider` — vertical stroke-based variant (volume.svg §1.5), 0…1,
  percent `aria-valuetext`. Icon + flyout chrome = later VolumeControl
  composite (mute-pending etc.).
- Live-vs-committed contract: `onChangeLive` continuous during drag / on each
  key step; `onCommit` on release / keyboard set (live fires before commit —
  asserted in tests). Fully controlled: no optimistic jumps.
- 31 tests (pointer sequences, keyboard, call-order, pending/buffer
  rendering, ARIA, token-only CSS). typecheck + lint + vitest all green
  (113 total). Zero new deps.

Next for sliders:

- Wire `Seekbar.pending` to the command layer's `playback.seek`
  request/generation lifecycle (doc 05 §4 staleness) when composites land.
- i18n: components take `aria-label` from callers via `useMsg` — composites
  must pass MessageKeys (no literals in primitives, lint-enforced).
- Buffer-range token (`surface-muted`) is a [PROPOSAL] mapping — confirm
  against Figma when a buffer state export exists (states.md).
- Miniplayer dense variant (16px track / 8×12 knob) reuses base `.lumen-slider`
  styles as-is; add a variant class only if the composite needs it.

## 4.C1 — player screen shell (session 1, committed 77f4888 on wip/p4-c1)

Done (ui/src/screens/player/** only + /player route + player.* i18n keys):

- `PlayerScreen` — layout root per doc 08 §1.6 anatomy: top bar (banner,
  h1), video canvas (role="region" + aria-label, data-placeholder=
  "engine-wiring-4.H1"), controls-bar region stub, side panels as labelled
  complementary landmarks when open. First-run empty state (doc 08 §5.3)
  with a DISABLED "Open media" Button (data-wiring-stub="session.open —
  card 4.C2"). No <video>/<canvas> element anywhere — dishonest until the
  render engine is wired.
- `layout.ts` — full consumption side of doc 07 §7.2: TOML-subset parser
  ([[region]]/[[menu]]/[[menu.item]], inline tables, scalars), layout.v1
  validation (anchors, unique ids, condition tokens, label_key resolution
  via new i18n `isMessageKey` guard), visible_when evaluation against a
  has-media/has-video/fullscreen/has-chapters context. [PROPOSAL] recorded:
  config/layouts/ does not exist yet → `DEFAULT_LAYOUT_TOML` (same format,
  parsed by the same parser) is the stand-in; missing/empty source →
  default, invalid source → loud LayoutParseError (doc 07 §3.5 philosophy).
- `keymap.ts` — keymap-data-driven seam (doc 05 §8.1): parser + key
  normalization ("Esc"→Escape, modifier combos captured but unclaimed) +
  matcher for plain-key player-focused bindings + shell recipe dispatch
  (fullscreen-toggle / exit-fullscreen-close-dialog /
  show-controls-fullscreen per keymaps/default.toml lines 154-156 —
  verbatim excerpt constant; swap for the real loader when it lands, open
  question Q1/Q4 in ui-inventory.md §9). Everything else → `onBinding`
  seam for 4.C2's command dispatch.
- `useChromeAutoHide` — visible/hidden/locked machine. [PROPOSAL] 3000 ms
  idle delay (doc 08 §4.3 pins NO number — VERIFY against design when a
  timing lands); scoped to fullscreen per §4.3; any pointer/key activity
  reveals + restarts; open panels or empty state lock; stale `hidden`
  cleared on unlock/disable. Fade is CSS under prefers-reduced-motion:
  no-preference (§4.5).
- `useFullscreen` — request on container; state synced ONLY from
  fullscreenchange (jsdom-verified: request without event ≠ fullscreen);
  FullscreenUnsupportedError for bare hosts (4.C2 surfaces it per doc 05
  taxonomy). ESC exits natively + via keymap data.
- `controls/index.ts` + `states/index.ts` — explicit TODO stubs; 4.C2/4.C3
  own those subtrees.
- Tests: 49 new (landmarks, empty state, layout valid/minimal/missing/
  invalid, condition gating, chrome timing w/ fake timers, fullscreen
  event sync, keymap match/forward). typecheck + lint + vitest 239/239
  green; tokens:check OK; zero new deps.

Next for 4.C2 (control bar) — wiring points already in place:

- `onBinding` seam: dispatch forwarded bindings (e.g. exit-fullscreen's
  dialog-close half) to real commands via the client.
- Enable the Open-media button: file-dialog ui-action + `session.open`.
- Fill the controls-bar region (transport per doc 08 §5.1 state table,
  Seekbar pending wiring per §5.2, volume, track menus from `tracks.list`).
- Panel toggle ui-actions should drive `openPanels` (shell locks chrome
  while any is open — keep that invariant).

Open items (VERIFY/PROPOSAL ledger for this card):

- Chrome auto-hide delay 3000 ms + fullscreen-only scoping: [PROPOSAL],
  doc 08 §4.3 leaves timing open ("Player - hide UI.svg" has no number).
- Fade duration 160 ms and panel width 377 px in player.css have no
  semantic token (no motion/size tokens exist yet) — 160 ms matches the
  4.B4 sheet-animation precedent; 377 px is doc 08 §4.3. Promote to tokens
  if/when they land.
- Default layout content (regions set, media-info visible_when=has-media,
  app menu leaf) is [PROPOSAL] until config/layouts/main.toml + the
  schemas/layout.schema.json validator land; then `resolveLayout` swaps
  its source and DEFAULT_LAYOUT_TOML is deleted.
- Full keymap loading (whole keymaps/default.toml, `when` conditions,
  modifier combos, mode gating beyond player-focused) — 4.C2 command layer.
- Empty-state visual polish (centering tokens, art) deferred to 4.G1
  visual pass; DOM shape + ARIA asserted now.
- 4.C3 note: first-run empty state lives in the shell per card 4.C1;
  coordinate if states/** re-implements it richer (loading/buffering/
  unsupported/error own that dir).
## Card 4.E1 — Settings framework (schema-driven) — DONE

What landed (branch wip/p4-e1, commits d8114b1 + tests):

- **Generator** `ui/scripts/gen-settings-screens.mjs` (zero-dep node): parses
  `schemas/settings.toml` (READ-ONLY source of truth, card 1.B2 — decision:
  no JSON export exists; the crate's `schemas/generated/settings.d.ts` is
  types-only, so the generator embeds a TOML-subset parser) and resolves
  `default_ref` targets + `[features]` exposure from `product.toml`. Emits
  committed `ui/src/screens/settings/generated/{schema.ts, Screens.tsx}`.
  `npm run settings:generate` / `settings:check` (byte-compare drift guard,
  wired like `tokens:check`). Validates D1–D5 subset + ranges at generation.
- **Framework** `ui/src/screens/settings/`: `SettingsProvider` (draft vs
  persisted dirty-state, apply/revert, restart-pending tracking, per-field +
  per-screen reset-to-default), cross-area search with honest no-results,
  confirm-on-leave via 4.B4 Dialog, `SettingField` type→primitive map
  (bool→Switch, enum→RadioGroup, numeric→NumberField w/ schema min/max,
  string/list→TextField). Feature-gated entries hidden+inert (product.toml
  F1/F2 — playback.hw-decode-policy correctly absent until 7.B).
- **AC proof**: `generated.test.ts` mutates a COPY of the schema (default
  change / restart-flag flip / entry removal) and asserts `settings:check`
  exits 1; pristine check exits 0. 39 new tests total, 229 ui-wide green,
  tsc strict + eslint 0 errors, vite build OK.

Handoffs / next actions:

- **App layer (owner: app team)**: replace the localStorage MOCK persistence
  seam (`persistence.ts`) with the real write path — `config.set-setting`
  over the client bridge (doc 05 §2.11). Interface is ready
  (`SettingsPersistence`); overrides-only store shape matches layer-4 user
  config.
- **4.E2/E3/E4 (content groups)**: add `[[setting]]` entries to
  `schemas/settings.toml` (crate is read-only for us; schema edits belong to
  the schema owner card), then run `npm run settings:generate` — new areas
  appear in nav automatically; add label msgids to `ui/src/i18n/messages.ts`
  (the i18n-coverage test fails loudly until you do). Section folding for
  `advanced = true` fields is currently a badge only — real VLC-style folding
  is open.
- **Schema owner**: doc 07 §5.2 note says the ~45-entry fuller seed "arrives
  with the settings framework (card 4.E1)" — NOT done (schema is read-only
  for this card; we render all 21 ui entries that exist). Growing the schema
  needs the 1.B2 owner; `layouts/settings-areas.toml` (§5.4) does not exist
  yet — nav order currently follows `[areas].known` in the schema.
- **i18n pipeline (doc 07 §6)**: settings msgids live in the scaffold
  `messages.ts` for now — migrate to `locales/en-US.toml` when the generated
  locale packs land; the localization crate's reference-complete audit (rule
  4) will then own the join.

## Card 4.C2 — Control bar (transport/seek/volume/track menus) — DONE

Delivered (branch `wip/p4-c2`, commit 12ec606): `ui/src/screens/player/context.tsx`
(shared player state seam — **4.C3 consumes `usePlayerControls()`; do not duplicate**),
`controls/{state-mapping.ts,ControlBar.tsx,controls.css}` + 52 tests. 330/330 ui
tests green, tsc strict clean, eslint 0 errors.

- **State mapping**: doc 08 §5.1 table pinned in `state-mapping.ts` + parametrized
  tests. Doc-silent cells recorded as `[PROPOSAL]` in code: `buffering` → transport
  disabled + aria-busy hint (volume/track stay live per §5.3); `stopped` → restart
  affordance like `ended`, stop disabled.
- **Keymap wiring**: `controlCommandOf()` maps binding `command` data
  (playback.play/pause/toggle-play-pause/stop) → dispatch in PlayerScreen's
  forward branch. **The keymap SEED has no space→play/pause binding** — when the
  keymap loader (doc 05 §8.1, inventory Q4) lands, add the binding to
  `keymaps/default.toml`; no control-bar change needed. `[PROPOSAL]`.
- **session.open is NOT wired**: the empty-state "Open media" stub stays disabled —
  it needs the file-dialog ui-action host (doc 07 §7.2 registry) which no card has
  exposed. Owner: whoever builds ui-actions. The §5.1 opening→pending path IS
  implemented (event-driven).
- **Track menus** use B3 `Dropdown` with identity label resolution (runtime track
  names are data, not msgids). When a real menu primitive with selected-markers in
  the trigger lands, swap — mapping stays in `ControlBar.tsx`.
- **Disk**: npm install failed (nospc); `ui/node_modules` is a SYMLINK to
  `worktrees/p4-c1/ui/node_modules` (same lockfile, zero new deps). Replace with a
  real install when disk is freed.
- **4.C3 note**: `PlayerControlsProvider` requires a `ClientProvider` ancestor
  (like `main.tsx`); tests wrap with `MockTransport` — with it, all mount queries
  answer typed `feature-not-exposed` and the bar renders honestly disabled with
  the error surfaced (by design).
## Card 4.C3 — playback UX states (branch wip/p4-c3, commit d55ef69)

Shipped: `ui/src/screens/player/states/**` — `StateRouter` (engine
`PlaybackState` → exactly zero-or-one overlay on the C1 canvas region),
`LoadingOverlay`/`BufferingOverlay` (determinate only from engine
`session.opening-progress`/`session.buffering` events; else honest
indeterminate — never a fabricated %), `UnsupportedOverlay` (format/codec
from the typed error's `details` whitelist + "no playback attempted"),
`ErrorOverlay` (stage-tagged pill from the typed `Stage`, redacted Display
message verbatim, actionable hint per `ErrorCode`, retry only when wired),
`EmptyOverlay` (migrated from the C1 shell markup). 28 matrix tests.

Open items for later cards:

- **4.C2 reconciliation**: `states/context-bridge.ts` is the MINIMAL
  stand-in for C2's `player/context.ts` (absent when this card started —
  note in-file). When it lands, delete the bridge and feed
  `StateRouter`'s `PlayerEngineView` from the canonical context; the
  components need no changes. Also wire the real `onRetry` (re-issue
  `session.open` of the same source) and `onOpenMedia` (file-dialog
  ui-action) — both currently render as disabled honest stubs.
- **Shell engine feed**: `PlayerScreen` takes `engineState` as a prop;
  nothing calls `usePlayerEngineView(useClient())` in the app tree yet
  (engine wiring is 4.H1). No transport → honest first-run empty.
- **Error copy-details affordance** (doc 08 §5.4): blocked on an error-id
  field in the catalog envelope — `CommandError` has no id today; adding
  one is a `crates/commands` change, not UI.
- **Non-fatal error counters** (doc 05 C5, e.g. decode errors): need an
  engine-side counter query; not renderable from the current event set.
- **States state-table md** (doc 08 §5.5): `states/states.md` prose table
  for these overlays is a nice-to-have follow-up; the executable matrix in
  `StateRouter.test.tsx` is the source of truth for now.
- **Spacing/motion tokens**: overlay CSS uses structural raw px (gaps,
  28px spinner, 160px bar) + 200ms fill transition — no spacing/motion
  tokens exist yet (same precedent as the C1 shell's structural px).
## Card 4.E2 — Settings group 1 (Interface, Appearance, Language, Hotkeys) — DONE

Branch `wip/p4-e2`, commit `f3bc058`. 312/312 vitest, tsc strict, eslint 0
errors, settings:check + new hotkeys:check green.

What landed (`ui/src/screens/settings/areas/**` + `ui/src/theme/settingsThemeSync.ts`):

- **Extension mechanism (recorded choice)**: E1's generator emits only what is
  schema-static. `areas/index.tsx` merges `GENERATED_{AREA_IDS,TITLES,SCREENS}`
  with hand-written extensions and exports the merged registry
  (`SETTINGS_AREA_*`); `SettingsScreen` imports from `./areas` instead of
  `generated/Screens`. E3/E4: append your dynamic areas + per-key widgets
  there (schema-backed ones still come via the generator). `SettingField`
  consults `SPECIAL_FIELD_WIDGETS[entry.key]` before its type-based renderer
  (wrapper chrome — dirty/restart/reset — preserved).
- **Appearance wiring**: `useSettingsThemeSync` (theme-side seam; ThemeProvider
  untouched except `ThemeContext` export) pushes the `interface.theme` DRAFT
  into ThemeProvider → `data-theme` on `<html>` (live preview, revert-safe).
  Special widget: `ThemeModePicker` (schema enum dark/light).
- **Language**: `LocalePicker` for `global.language` — options = schema enum ∩
  real shipped catalogs (`locales/` dir = en-US only today, per
  crates/localization `load_locales_dir`).
- **Hotkeys editor**: `scripts/gen-hotkeys-data.mjs` (zero-dep; reuses
  gen-settings-screens' TOML parser — player/layout.ts's cannot parse dotted
  headers/nested inline tables) → `areas/hotkeys/generated/bindings.ts`
  (141 bindings + 9 macOS overrides from keymaps/default.toml), drift-guarded
  via `npm run hotkeys:check`. `HotkeysEditor`: per-binding combo edit,
  reset-to-default per row, draft→Apply→localStorage overrides seam
  (`lumen.hotkeys.overrides.v1` — real write path = app-layer handoff, same as
  settings persistence), conflict detection on normalized (mode, combo) groups
  using 4.C1's `normalizeBindingKey`, inline warning on EVERY conflicting row.
- **Tests (34 new)**: theme wiring incl. html-attribute flips + revert +
  never-guess; locale rendering/binding; editor render/edit/conflict-fire/
  conflict-clear/reset/apply/disabled; conflicts pure logic (7); AC test —
  every SPECIAL_FIELD_WIDGETS key ∈ schemas/settings.toml + every interface
  entry renders bound to its schema key + hotkeys rows from real data.

Handoffs / gaps (honest):

- **Schema owner (1.B2 follow-up)**: NO `interface.accent` key in
  schemas/settings.toml → accent picker NOT rendered (config-never-invents);
  ThemeProvider's 9 accents stay on the 4.A2 default until the key lands (the
  sync seam is ready to grow). Likewise no hotkey [[setting]] entries exist —
  intentional: hotkeys are dynamic keymap data, handled via the generated
  bindings module instead.
- **i18n pipeline**: 133 `hotkeys.*` label msgids added to messages.ts with
  copy derived mechanically from the keymap label-msgids — replace with real
  copy via locales/en-US.toml when the localization pipeline owns the join
  (doc 07 §6).
- **Hotkeys editor known limits**: key capture is a plain text input (typed
  combo must match keymap spelling grammar — `Ctrl+Shift+X`); search does not
  cover hotkey rows (schema-entry search only); the editor's draft state is
  NOT wired into the shell's confirm-on-leave dialog (self-contained bar);
  platform overrides are surfaced as data only. Startup theme still defaults
  dark until an app-level mount reads persisted settings before the settings
  screen opens (my seam only runs while SettingsProvider is mounted).
- **E3/E4 coordination**: `areas/index.tsx`, `SettingField.tsx` (3-line seam)
  and `messages.ts` (append-only) are the shared touchpoints — merge order
  may need a trivial resolve.
## Card 4.E3 — Settings group 2 (audio / video / subtitles) @ c2b6e66

**Done:** `areas/audio.tsx` (output-device control via `audio.list-devices` /
`audio.set-device`; absent capability → DISABLED WITH REASON, proven against
MockTransport's `feature-not-exposed`), `areas/video.tsx` + `areas/subtitles.tsx`
(honest empty states — zero controls), `areas/audio-fixtures.ts` (test seam),
16 tests in `areas/*.test.tsx` (294/294 green).

**Schema owner handoff (blocking most of this card's documented surface):**
`schemas/settings.toml` has ZERO entries for screens `video`, `audio`,
`subtitles` (doc 07 §5.4 seeds them: `video.deinterlace`/`default-aspect-ratio`/
`default-crop`/`frame-drop-policy`; `audio.output-device`/`volume-default`/
`volume-ceiling`/`normalize`/`visualizer`; `subtitle.font-size`/`color`/
`outline-strength`/`position`/`sync-step-ms`/`fallback-encoding`). Until
entries land, E1's generator emits no such screens and my area files render
nothing for those options (never dead controls). Specifically requested by the
card but schema-absent: **audio exclusivity mode**, **video display selection**
(also no display-enumeration command in the catalog — needs a 1.D4 schema
decision, not just a settings entry), and **OSD options** (doc 07 §5.4 puts
`controls.osd-duration-ms` on the *controls* screen — confirm ownership).

**Integration:** `AudioArea`/`VideoArea`/`SubtitlesArea` are exported but NOT
wired into `SettingsScreen` routing (`GENERATED_AREA_IDS` is generated from
the schema). When schema entries land, either let the generator own these
screens or register the hand-authored modules; decide before deleting them.
Feature-gate note: E1's `SettingField` hides feature-gated entries
(doc 05 §6.4) — if the schema owner wants gated-but-documented entries to
render disabled-with-reason instead (the 4.E3 pattern), that's a framework
change in `SettingField.tsx` + `SettingsSections.tsx` (E1 owner).

## Card 4.E4 — Settings group 3 (input/codecs, playlist/services, privacy/network, updates/about, advanced)

**Done:** five hand-authored areas registered through the `areas/index.tsx`
mechanism (no generator change): `areas/codecs.tsx` (hw-decode policy bound to
`playback.hw-decode-policy`, rendered DISABLED-WITH-REASON per ADR-001's
honest status — accepted policy, Phase 7 implementation, product.toml F2 keeps
`features.hw-decode = false`; cache controls bound to
`performance.cache-demux-mib` / `cache-network-mib`), `areas/playlist-services.tsx`
+ `areas/privacy-network.tsx` (honest empty states — zero schema entries for
service discovery or proxies; the privacy half renders via E1's generated
screen with telemetry defaults OFF, asserted in tests), `areas/updates-about.tsx`
(About panel with REAL version data injected at build time from
ui/package.json via vite `define` — `src/build-info.ts`; no hardcoded literal;
update-checks bound to `privacy.update-checks`), `areas/advanced.tsx` (every
`advanced = true` schema entry folded into one screen, collapsed by default,
engine ui=false entries excluded). 26 tests across
`areas/{codecs,playlist-services,privacy-network,updates-about,advanced}.test.tsx`
+ updated the areas-registry assertion (48 files / 435 tests green; tsc, eslint,
`settings:check`, `vite build` all clean).

**Schema owner handoff (blocking documented surface):** ZERO entries exist for
service discovery (discover-enable etc.), network proxies, and update actions
(no update command in the catalog either — no "check now" button by design).
Until entries land these areas render NOTHING (never dead controls).

**Placement duplication (D-rules follow-up):** `performance.cache-*` render on
both the generated performance screen and the codecs area;
`privacy.update-checks` renders on both the generated privacy screen and the
Updates & About area. The schema owner should consolidate placement (move
cache to a codecs/input section, update-checks to an updates section) — then
the hand-authored areas become pure presentation. E3's unrouted
`AudioArea`/`VideoArea`/`SubtitlesArea` remain unregistered (their schema
screens are still empty); same decision pending.

**Build-info seam:** `vite.config.ts` `define` reads ui/package.json →
`src/build-info.ts` (+ `build-info.globals.d.ts`, `resolveJsonModule` added).
If the engine later exposes `system.get-engine-info` version data in the UI
shell, the About panel should prefer it for engine/protocol versions; the
package identity stays build-injected.
## Card 4.F1 — onboarding handoff (app layer)

`ui/src/screens/onboarding/onboardingState.ts` persists to its own
localStorage seam (`lumen.onboarding.v1`) via an E1-shaped
`OnboardingPersistence` interface — separate key on purpose: E1's
`SettingsProvider.apply()` rewrites `lumen.settings.overrides.v1` wholesale
(schema overrides only), so an onboarding flag written there would be wiped
by the next settings Apply. App layer must, at startup or on wizard finish:

1. Translate the choices to real config writes over the client bridge
   (doc 05 §2.11 `config.set-setting`): `choices.telemetry` →
   `privacy.telemetry`, `choices.updateChecks` → `privacy.update-checks`
   (both schema keys, doc 07 §5.4), then delete the onboarding store row.
2. `choices.onlineSuggestions` has NO schema key yet ([PROPOSAL], card
   4.F1 wording) — needs a settings-schema entry before it can be written
   for real; until then it stays a UI-only flag, honestly labeled.
3. `onboarding.complete` is the first-run gate (`RequireOnboarding` on
   /player in App.tsx); the app layer may pre-seed it true for
   migrated/managed installs (admin lock list, doc 07 §8) to skip the
   wizard — that path must still default every privacy flag OFF.
4. Appearance step previews via ThemeProvider only (rendering seam,
   card 4.A2) — persisting theme/accent belongs to the settings
   integration (4.E2 sync seam), not the wizard.
## Card 4.D2 — media info screen handoff

`ui/src/screens/media-info/**`: `MediaInfoProvider` (seeds via `media.get-info`,
stays authoritative from `media.info-ready` events), shared `MediaInfoPanel`
(general / video / audio / subtitles / metadata sections, `data-field` hooks),
`MediaInfoScreen` (/media-info route body), token-only `media-info.css`,
fixture set shaped exactly like the `MediaInfo` wire type. Wired: `/media-info`
route in App.tsx + the player shell `media-info` region (PanelRegion in
PlayerScreen.tsx). **AC honored:** every absent field renders the i18n
`media-info.unknown` ("Unknown") — never an invented value; streams keep
showing missing fields (info-density), and empty/not-exposed states are honest.

**Catalog gaps (recorded, never faked):** the `MediaInfo` wire type carries no
`size` and no `bitrate` (general or per-stream), no per-stream bitrate/tag
details, and no disc/track numbers. Those General rows render "Unknown" today;
adding fields to the catalog (`crates/commands/src/catalog`) + codegen would
let them render for real. `duration: infinite` renders the labeled "∞ (live)"
(engine-reported), `unknown` renders "Unknown".

**App-layer handoffs:** (1) no navigation affordance reaches /media-info yet —
menu/keymap wiring (e.g. an "Info" toggle over the player region, mpv `i`
semantics) is app-shell work; (2) the player-shell region wiring line is
covered only indirectly (PlayerScreen test suite renders it via openPanels);
(3) doc 08 has no dedicated media-info frame [VERIFY-DESIGN] — styling follows
the settings language per the doc's own proposal.

## Card 4.F3 — customize panels handoff

`ui/src/screens/player/customize/**` implements the four design panels
(Crop/Aspect, Video Colors, Essential, HDR — doc 08 §1.6) as a fully
data-driven surface: `panels.ts` is the field catalog (field → command id →
args payload), `capabilities.ts` gates each field on the engine's
`system.list-commands` listing, and `CustomizePanels.tsx` renders only
capability-backed fields (unsupported = HIDDEN, per the card AC — contrast
settings' disabled-with-reason). Panels whose fields are ALL unsupported are
hidden entirely; against the scaffold MockTransport (empty listing) the
whole surface renders only an sr-only honest note.

Handoffs / decisions for later cards:

1. **Forward-declared commands:** video colors (`video.set-brightness` /
   `-contrast` / `-saturation` / `-hue` / `-gamma`) and HDR tone mapping
   (`video.set-tone-mapping`, `-strength`) are in the DESIGN but not the 1.D4
   wire catalog. `CustomizePanels.dispatch` bridges with an
   `as unknown as CommandRequest` cast; their arg keys (`value`, `mode`,
   `strength`) are [PROPOSAL]. When `cargo xtask codegen` regrows the
   catalog, replace the cast + reconcile arg keys; fields light up purely by
   listing exposure — no renderer change.
2. **Layout seam:** the `customize` region (right anchor, `visible_when =
   "has-media"`) + `open-panel:customize` menu ui-action are additive entries
   in `DEFAULT_LAYOUT_TOML`. The app layer still needs to route that
   ui-action to the `openPanels` prop (the shell already renders the region
   content when open) and add real `config/layouts/customize.toml` when the
   config system lands (doc 07 §7).
3. **Confirmed-state sources:** select selections / slider committed values
   update ONLY from command completions (`applied-aspect-ratio`, `applied-crop`,
   `applied-repeat`, `applied-rate`, `applied-zoom`, `applied-bool`) and
   events (`playback.rate-changed`, `playlist.changed` repeat-set/shuffle-set).
   Live slider drags are visual-only (doc 08 §5). An engine-side query for
   current color/crop values would let panels open PRE-populated — none
   exists in the catalog yet, so panels start at neutral display positions.
4. **Essential panel scope:** speed slider, ±10 s/±60 s seeks, stop, repeat
   (off/all/one), shuffle. The design's full button catalog also lists
   Record / Snapshot / PiP / A-B loop — Snapshot maps to `video.take-screenshot`
   (in catalog) but is a control-bar candidate (4.C2 ownership), and Record /
   PiP / A-B have no commands; they were left out rather than rendered dead.

## 4.G1 — screenshot-diff harness landed (2026-10-23)

`npm run visual` / `cargo xtask visual [--mode=pr|nightly] [--update]
[--seed]`: Playwright-driven screenshot-diff harness per doc 08 §6 —
manifest-driven matrix, per-OS baselines (`e2e/visual/baselines/<platform>/`),
PR-lite (8 cases, ~12 s, strict) + nightly (32 cases) modes, tolerance
threshold 0.1 / maxDiffPixelRatio 0.001, flakiness controls (reduced
motion, deterministic onboarding state, #root clip, retries=1). Seeded
2-px regression FAILS as required (7440 px diff, exit 1); clean tree
passes both lanes. Demo transcript: `e2e/visual/README.md`.

Handoff to 4.G2: re-pin the FROZEN baseline matrix after F4 merges
(`npm run visual -- --mode=nightly --update`), extend ROUTES to the full
doc 08 §1.6 inventory + accent ×9 sampling, wire the per-OS CI jobs
(PR lane per PR, full matrix nightly).

## 4.F4 — keyboard navigation + focus + a11y pass landed (2026-10-24)

`ui/e2e/a11y.spec.ts` (16 tests, all green): axe wcag2a+wcag2aa scan on
EVERY routable screen — player, player/music, miniplayer, playlist,
media-info, settings ×5 areas, onboarding (onboarded routes seed the 4.F1
gate) — plus a tab-order audit (focus order == DOM order, every stop
visible, every stop paints a ring) on player/playlist/settings/onboarding,
plus a computed-style assertion that the ring color resolves from
`--color-accent` (not a hardcoded paint). Exclusions live in
`JUSTIFIED_EXCLUSIONS` with reasons and print on every run — currently
EMPTY (zero justified findings).

Fixes (all additive, tokens only): global `:focus-visible` floor +
body-level surface paint floor (`src/a11y/focus.css`, `src/a11y/page.css`)
— playlist/media-info previously rendered on raw browser WHITE in dark
theme (headings ~3.2:1); player error chip raised→chrome surface
(4.08/4.46→5.3:1+); toggle off-label and playlist micro-text
tertiary→secondary (3.3–3.7:1→5.4:1+); settings badge muted→selected
surface (3.5→5.4:1). Playlist visual baselines re-pinned (the white-page
bug was in the old captures); player/settings/onboarding baselines
unchanged. `e2e/visual/visual.spec.ts` got a zero-dep `process` typing
(pre-existing `tsc --noEmit` break over e2e/). Toggle.test.tsx off-label
expectation updated tertiary→secondary — deliberate, documented deviation:
tertiary can never clear 4.5:1 on either theme's track (design follow-up
below). Dep added per AGENTS.md rule 2: @axe-core/playwright 4.14.0
(MPL-2.0, dev-only, no bundle/runtime impact, npm provenance) — recorded
in ui/README's dependency table; Lead approval to confirm.

Handoffs: (1) design — a compliant "dim" text tier is needed; tertiary
fails 1.4.3 on every dark surface, so all micro-text now uses secondary
(Figma state frames should follow). (2) shortcuts — only the player is
keymap-data-driven (C1/C2); settings-search focus shortcut and onboarding
key nav have NO keymap data in doc 05 yet — gap for a future card, not
invented here. (3) C3 overlays already announce (role=status/alert);
player/music + miniplayer routes are still placeholders (axe-green but
trivially so — rescan when 4.G2+ land real variants).

## 4.G2 — full baseline matrix captured + first run green (2026-10-24)

Manifest grown to the full routed surface (15 routes: player,
player/music, miniplayer, playlist, media-info, 5 settings areas,
onboarding, keyboard-shortcuts, open-network-stream, not-found,
/dev/tokens) × {1280×720,1920×1080} × DPI {1,2} × {dark,light} × default
accent, + doc 08 §6.1 accent sampling (8 non-default accents on
onboarding + settings-global @1280×720 1× dark) = 136 nightly / 30 PR
cases. Post-F4 re-pin captured (4.5 m), nightly verify 136/136 green
(3.2 m, zero retries), PR lane 30/30 (43 s). Zero remediations; no
tolerance changes. Record: ui/e2e/visual/MATRIX-20261024.md (scaling
decisions, Linux-only scope note, per-OS extension path).

NEXT: (1) CI wiring — per-OS jobs (Linux canonical), PR lane per PR,
nightly full matrix (doc 08 §6.3); macOS/Windows baseline sets pin with
one --update run each once runners exist (sandbox is Linux-only —
infra-blocked, gate precedent). (2) Grow ROUTES as §1.6's unrouted
compositions land (player customize/hide-ui, miniplayer states, context
menus, audio/video tools, playlist/subtitles flow) — each lands with its
own re-pin commit per doc 08 §6.5. (3) A cross-OS text nondeterminism
that survives retries still needs the per-case documented-exception path
(zero cases so far — mechanism documented in the matrix record).
