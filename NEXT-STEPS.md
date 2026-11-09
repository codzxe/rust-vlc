# NEXT-STEPS — canonical open-items ledger

Consolidated on 2026-11-09 (issue #25) from four files: this file's former
7-g / 4.B3 / 4.B4 content (including the committed, unresolved merge
conflict — open items from BOTH sides are preserved below),
`NEXT-STEPS-w7h.md`, `ui/NEXT-STEPS.md`, and `tools/sweep/NEXT-STEPS.md`.
Those three are now pointer stubs; pre-consolidation content lives in git
history (`git log --follow -- <path>`).

Planning context: [`docs/ROADMAP.md`](docs/ROADMAP.md) (phase gates 0–8,
wave program, decision register). Per-issue tracking: the GitHub issue
tracker. This file is the single tactical queue of open work items;
completed/stale items were dropped in the consolidation (audit in issue
#25). Provenance tags name the card/session that recorded the item:
`[4.B*]`/`[4.C*]`/… = ui cards, `[7-g]`/`[7-h]` = soak/bench tasks,
`[ui]` = the old ui do-next list, `[sweep]` = tools/sweep notes.

## Now — actionable without waiting on another owner

- [ ] [4.C3] Reconcile the states bridge: delete
  `ui/src/screens/player/states/context-bridge.ts` once `StateRouter` feeds
  from the canonical `player/context.ts` (PlayerScreen currently consumes
  the bridge — verified still in place 2026-11-09).
- [ ] [4.C3] Wire the real `onRetry` on the error overlay (re-issue
  `session.open` of the same source) — still an unwired optional prop.
- [ ] [4.B3] Popup rgba token: menu popups need a translucent surface token
  for doc 08 §115 ("#191919 @ 0.95") — currently solid `--color-surface`
  (known-gap note in `ui/src/components/menu/menu.css` + states.md).
- [ ] [4.B4] Icon glyphs are provisional geometry (TODO [VERIFY-DESIGN] in
  `ui/src/components/icon/icons.tsx`): mine real paths from
  `design/figma/Icon.svg` / `Controls.svg` (318 stroke-width-1.5 elements)
  or get named exports from design.
- [ ] [4.B4] Icon emphasis variants (stroke 2/3) are unimplemented.
- [ ] [4.E1] App-layer settings persistence: replace the localStorage MOCK
  seam (`ui/src/screens/settings/persistence.ts`) with the real write path
  — `config.set-setting` over the client bridge (doc 05 §2.11);
  `SettingsPersistence` interface is ready, overrides-only store shape
  matches layer-4 user config.
- [ ] [4.F1] Onboarding persistence: translate wizard choices to real
  config writes over the client bridge (`choices.telemetry` →
  `privacy.telemetry`, `choices.updateChecks` → `privacy.update-checks`),
  then delete the onboarding store row.
- [ ] [4.F4] a11y rescan: player/music + miniplayer were placeholder-trivial
  at the 4.F4 pass; Solstice v2 landed real variants — re-run the axe +
  tab-order suite over them.
- [ ] [7-h] Handoff to the 3.E3 owner: re-bless the W5-14 expectation
  update in `tools/bench-vs-vlc`
  (`filtered_video_stream_does_not_touch_the_audio_digest` now expects 1
  Recovered / 0 skip bytes — silent-skip semantics).
- [ ] [7-h] Profile `media/0339-avi-video.avi` (25.5 s decode, ~10× the
  next-slowest, near the 30 s sweep deadline; smoke max was 0.63 s) before
  running larger corpora.

## Next — queued, waiting on a card / owner / config landing

- [ ] [4.C1] Keymap loader: load the whole `keymaps/default.toml` (`when`
  conditions, modifier combos, mode gating beyond player-focused) —
  `ui/src/screens/player/keymap.ts` still carries a verbatim excerpt
  constant for the shell recipes; swap for the real loader (doc 05 §8.1,
  inventory Q1/Q4).
- [ ] [4.C2] Add the missing space→play/pause binding to
  `keymaps/default.toml` when the keymap loader lands (no control-bar
  change needed).
- [ ] [4.C1] Land `config/layouts/main.toml` + `schemas/layout.schema.json`
  validator, then swap `resolveLayout`'s source and delete
  `DEFAULT_LAYOUT_TOML` (ui/src/screens/player/layout.ts).
- [ ] [4.B3] Compose the app MenuBar from `layouts/menus.toml` + keymap
  binding-id accelerators when those config files land (doc 05 §8.1/§8.3).
- [ ] [4.B3] ContextMenu coordinates are unclamped against the viewport
  (no layout in jsdom) — clamp when composed into real screens.
- [ ] [4.B3/4.G2] Menu visual snapshots (card `+SB` AC): states.md carries
  the state tables + design provenance (context menu rx10, items 48px,
  hover `#1F1F1F` rx6, dropdown trigger rx6/inner rx4); add menu cases to
  the 4.G1/G2 screenshot matrix.
- [ ] [4.E2] Accent wiring: no `interface.accent` key in
  `schemas/settings.toml` → accent picker not rendered
  (config-never-invents); ThemeProvider's 9 accents stay on the 4.A2
  default until the key lands (the sync seam is ready to grow).
- [ ] [4.E2] Startup theme still defaults dark until an app-level mount
  reads persisted settings before the settings screen opens
  (`settingsThemeSync` only runs while SettingsProvider is mounted).
- [ ] [4.E3] Settings schema seeds (schema owner 1.B2): zero entries exist
  for the video/audio/subtitles screens — doc 07 §5.4 seeds are
  `video.deinterlace`/`default-aspect-ratio`/`default-crop`/
  `frame-drop-policy`; `audio.output-device`/`volume-default`/
  `volume-ceiling`/`normalize`/`visualizer`; `subtitle.font-size`/`color`/
  `outline-strength`/`position`/`sync-step-ms`/`fallback-encoding`.
- [ ] [4.E4] Settings schema seeds (cont.): zero entries for service
  discovery (discover-enable etc.), network proxies, and update actions
  (no update command in the catalog — no "check now" button by design);
  areas render NOTHING until they land (never dead controls).
- [ ] [4.E1] The doc 07 §5.2 ~45-entry fuller settings seed "arrives with
  the settings framework" — NOT done (schema is read-only for ui cards;
  growing it needs the 1.B2 owner).
- [ ] [4.E3] When schema entries land: decide whether the generator owns
  the video/audio/subtitles screens or the hand-authored
  AudioArea/VideoArea/SubtitlesArea get registered — before deleting them.
- [ ] [4.E4] Placement consolidation (D-rules follow-up):
  `performance.cache-*` renders on both the generated performance screen
  and the codecs area; `privacy.update-checks` on both the privacy screen
  and Updates & About — schema owner should consolidate, then the
  hand-authored areas become pure presentation.
- [ ] [ui] ESLint rules v2: promote the three inline rules in
  `ui/eslint.config.js` into a proper local plugin (e.g.
  `tools/eslint-plugin-lumen/`) with real schema + tests;
  template-literal coverage for `local/semantic-tokens-only` (currently
  regex-grade on quasis only).
- [ ] [ui] Stylelint for plain `.css` files (doc 08 §3.4) —
  TokenSandbox.css is var()-only by construction, not by enforcement.
- [ ] [ui] Command client growth: add a `useCommand` hook (requestId
  allocation, pending map, event dispatch, stale-generation handling
  already landed in `ui/src/client/transport.ts`).
- [ ] [ui] Real fixture replay from the doc 05 §7 testkit for the command
  client; keep the `src/generated/` import seam in one file.
- [ ] [4.E1/4.E2] i18n migration (doc 07 §6): move settings + the 133
  mechanically-derived `hotkeys.*` label msgids out of
  `ui/src/i18n/messages.ts` into `locales/en-US.toml` with real copy when
  the generated locale packs land (the localization crate's
  reference-complete audit then owns the join).
- [ ] [4.H3] Real locale packs (de, pt-BR, …) + the crates/localization
  runtime pipeline per doc 07 §6.
- [ ] [4.D2] MediaInfo catalog gaps: the wire type carries no `size`, no
  general/per-stream `bitrate`, no per-stream tag details, no disc/track
  numbers — those rows render "Unknown"; adding fields to
  `crates/commands/src/catalog` + codegen would let them render for real.
- [ ] [4.D2] No navigation affordance reaches /media-info yet — menu/
  keymap wiring (e.g. an "Info" toggle over the player, mpv `i` semantics)
  is app-shell work.
- [ ] [4.F3] Forward-declared commands: video colors
  (`video.set-brightness`/`-contrast`/`-saturation`/`-hue`/`-gamma`) and
  HDR tone mapping (`video.set-tone-mapping`, `-strength`) are in the
  design but not the 1.D4 wire catalog; replace the
  `as unknown as CommandRequest` bridge cast + reconcile the [PROPOSAL]
  arg keys when `cargo xtask codegen` regrows the catalog.
- [ ] [4.F3] Route the `open-panel:customize` ui-action to the shell's
  `openPanels` prop (region already renders when open); add real
  `config/layouts/customize.toml` when the config system lands (doc 07
  §7).
- [ ] [4.F4] Design follow-up: a compliant "dim" text tier is needed —
  tertiary fails WCAG 1.4.3 on every dark surface, so all micro-text uses
  secondary; Figma state frames should follow.
- [ ] [4.G2] Grow visual ROUTES as doc 08 §1.6's unrouted compositions
  land (player customize/hide-ui, miniplayer states, context menus,
  audio/video tools, playlist/subtitles flow) — each with its own re-pin
  commit per doc 08 §6.5.
- [ ] [4.B1] Token asks for `themes/tokens.json` (Lead): `--radius-knob`
  3px, `--radius-input` 5px, `--color-danger` (see worklog 4.B1
  handoffs).

## Parked — conditional / nice-to-have / awaiting a trigger

- [ ] [4.B4] Dialog portal: renders in place with a fixed-position scrim;
  fine for current hosts — revisit if stacking contexts clip it
  (DialogHost overlay is a doc 08 §4.1 [PROPOSAL]).
- [ ] [4.B4] `scrollToIndex` is prop-driven (target object), not
  ref-imperative; wrap in `useImperativeHandle` if screens need
  programmatic control mid-gesture.
- [ ] [4.B2] Buffer-range token (`surface-muted`) is a [PROPOSAL] mapping
  — confirm against Figma when a buffer state export exists (states.md).
- [ ] [4.B2] Miniplayer dense slider variant (16px track / 8×12 knob):
  reuses base `.lumen-slider` styles; add a variant class only if the
  composite needs it.
- [ ] [4.C1] Chrome auto-hide delay 3000 ms + fullscreen-only scoping is
  [PROPOSAL] — doc 08 §4.3 pins no number ("Player - hide UI.svg" has
  none); verify against design when a timing lands.
- [ ] [4.C1] Fade duration 160 ms and panel width 377 px in player.css
  have no semantic token — promote to tokens if/when motion/size tokens
  land.
- [ ] [4.C3] Overlay CSS structural raw px (gaps, 28px spinner, 160px
  bar) + 200 ms fill transition — same spacing/motion-token gap.
- [ ] [4.C1] Empty-state visual polish (centering tokens, art) — deferred
  to the visual pass; DOM shape + ARIA asserted now.
- [ ] [4.C2] Track menus use the B3 Dropdown with identity label
  resolution; swap for a menu primitive with selected-markers in the
  trigger when one lands (mapping stays in ControlBar.tsx).
- [ ] [4.C3] States state-table md (doc 08 §5.5) — nice-to-have; the
  executable StateRouter matrix is the source of truth for now.
- [ ] [4.E1] Section folding for `advanced = true` fields is a badge only
  — real VLC-style folding is open.
- [ ] [4.E1] `layouts/settings-areas.toml` (doc 07 §5.4) does not exist;
  nav order currently follows `[areas].known` in the schema.
- [ ] [4.E2] Hotkeys editor: key capture is a plain text input (typed
  combo must match keymap spelling grammar — `Ctrl+Shift+X`).
- [ ] [4.E2] Hotkeys editor: search does not cover hotkey rows (schema
  entries only).
- [ ] [4.E2] Hotkeys editor: draft state is NOT wired into the shell's
  confirm-on-leave dialog (self-contained bar).
- [ ] [4.E2] Hotkeys editor: platform overrides are surfaced as data
  only.
- [ ] [4.E3] Feature-gate rendering: if the schema owner wants
  gated-but-documented entries to render disabled-with-reason (the 4.E3
  pattern) instead of hidden (4.E1 default), that is a framework change
  in `SettingField.tsx` + `SettingsSections.tsx`.
- [ ] [4.E4] About panel should prefer `system.get-engine-info` version
  data if the engine exposes it in the UI shell; package identity stays
  build-injected.
- [ ] [4.F1] `choices.onlineSuggestions` has no schema key yet
  ([PROPOSAL], card 4.F1 wording) — needs a settings-schema entry before
  it can be written for real; stays a UI-only flag, honestly labeled.
- [ ] [4.F1] App layer may pre-seed `onboarding.complete` true for
  migrated/managed installs (admin lock list, doc 07 §8) to skip the
  wizard — that path must still default every privacy flag OFF.
- [ ] [4.F3] Panels open at neutral display positions — an engine-side
  query for current color/crop values would let them open pre-populated;
  none exists in the catalog.
- [ ] [4.F3] Essential-panel extras: Snapshot maps to
  `video.take-screenshot` (in catalog; control-bar candidate, 4.C2
  ownership); Record / PiP / A-B loop have no commands — left out rather
  than rendered dead.
- [ ] [4.G2] A cross-OS text nondeterminism that survives retries needs
  the per-case documented-exception path (zero cases so far — mechanism
  documented in the matrix record).
- [ ] [4.F4] Keymap data for the settings-search focus shortcut and
  onboarding key nav — no data in doc 05 yet; gap for a future card, not
  invented here.
- [ ] [4.F4] Lead approval to confirm: @axe-core/playwright 4.14.0 dev-dep
  (MPL-2.0, dev-only; recorded in ui/README's dependency table).
- [ ] [4.H3] Lead to confirm the ui.yml step-3 l10n:check insertion
  (numbering 3→5 updated; Lead-owned file).
- [ ] [4.H3] Formal pixel-matrix pseudo screenshots: append
  `&locale=pseudo` to the G1/G2 matrix URLs when the matrix is free.
- [ ] [sweep] Add a `--manifest` flag so class derivation reads the
  manifest's `class` field directly (1:1 with the generator) instead of
  filename parsing (`outcome::class_of`).
- [ ] [ui] Directory drift: doc 08 §8 wants `src/bindings/` + `src/app/`;
  current tree has `src/generated/` (1.D4 output path — don't move it
  without updating `cargo xtask codegen`) and root-level App.tsx /
  main.tsx. Reconcile when components land.

## Blocked — needs an external unblock

- [ ] [7-g] 4 h soak baseline: the CI nightly host now EXISTS — issue #18
  wired `.github/workflows/soak-nightly.yml` (cron `0 3 * * *` UTC +
  dispatch; `SOAK_MINUTES=240 SOAK_LEAK_MB_PER_H=5 SOAK_REPORT=
  soak-report.md cargo test -p playback-engine --test soak --release --
  --nocapture`; design + the real 2-min smoke profile:
  `docs/perf/soak-nightly-20261109.md`). This sandbox still cannot run
  it (reaps background procs — the w7-h record — and 10-min tool
  timeout < 240 min; chunked restarts invalid: one slope, one window).
  Remaining: first scheduled run's artifact (`evidence-soak-nightly-…`)
  → copy report → `docs/perf/soak-4h-YYYYMMDD.md`, raw log →
  `docs/perf/raw/`, commit, worklog entry with the D-10 block, and let
  that measurement bless or renegotiate the 5 MB/h `[PROPOSAL]`
  ceiling (generous default stays 50 MB/h, `SOAK_LEAK_MB_PER_H`).
- [ ] [4.C3] Error copy-details affordance (doc 08 §5.4): blocked on an
  error-id field in the catalog envelope — `CommandError` has no id today;
  adding one is a `crates/commands` change, not UI.
- [ ] [4.C3] Non-fatal error counters (doc 05 C5, e.g. decode errors):
  need an engine-side counter query; not renderable from the current
  event set.
- [ ] [4.E3] Audio exclusivity mode + video display selection:
  schema-absent, and display selection also needs a display-enumeration
  command in the catalog (a 1.D4 schema decision, not just a settings
  entry).
- [ ] [4.E3] OSD options: doc 07 §5.4 puts `controls.osd-duration-ms` on
  the *controls* screen — ownership needs confirming.
- [ ] [4.G2] Per-OS visual CI jobs (doc 08 §6.3): Linux canonical; PR
  lane per PR, nightly full matrix; macOS/Windows baseline sets pin with
  one `--update` run each once runners exist (sandbox is Linux-only —
  infra-blocked, gate precedent).

## Standing notes

- **Sandbox process reaping** [7-g, 7-h, sweep]: `setsid nohup` background
  jobs die silently at tool-call end (heartbeat-verified); run long
  builds/tests in the foreground. `tee | head` SIGPIPE-truncates files
  (durations.tsv needed a rerun). A shared-target cargo lock waits, never
  fails.
- **Sweep semantics** [sweep]: non-AVI containers honestly land
  `unsupported` (probe/demuxer) — the census reports this as coverage,
  not failure. Worker protocol: 15 tab-separated fields after `SWEEP`;
  supervisor lines are `SWEEPLINE` + 15 fields. Corpus regeneration of a
  committed manifest goes through `--replay`; don't walk the corpus root
  (hls/ dirs carry extra unmanifested files). speed_bench needs manifest
  entries WITH a `bytes` field.
- **Virtualization scope** [4.B4]: fixed-height only per doc — variable
  heights / grid virtualization are NOT needed by any current screen; do
  not add speculatively.
- **i18n practice** [4.B2]: primitives take `aria-label` from callers via
  `useMsg`; composites pass MessageKeys (no literals in primitives,
  lint-enforced).
- **Test composition** [4.C2]: `PlayerControlsProvider` requires a
  `ClientProvider` ancestor (like main.tsx); tests wrap with
  MockTransport — with it, all mount queries answer typed
  `feature-not-exposed` and the bar renders honestly disabled.
- **Dev fixtures** [ui]: `/dev/tokens` is a dev fixture, NOT a product
  screen — do not copy its pattern into product screens.
- **Local env** [4.H3]: a sandbox node_modules lacking
  @axe-core/playwright (it IS in package-lock) shows 5 pre-existing
  e2e/a11y.spec.ts tsc errors locally that CI's `npm ci` will not.
- **Onboarding store key** [4.F1]: onboarding persists to its own
  `lumen.onboarding.v1` key on purpose — E1's `SettingsProvider.apply()`
  rewrites `lumen.settings.overrides.v1` wholesale, so an onboarding flag
  written there would be wiped by the next settings Apply.
