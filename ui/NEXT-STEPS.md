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
