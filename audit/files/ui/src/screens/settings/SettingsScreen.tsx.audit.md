# AUDIT: ui/src/screens/settings/SettingsScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 496 / 14 (SIDEBAR_GROUPS, AREA_ICONS, iconOf, windowTitleKey, isAreaId, groupRows, FOCUSABLE, SettingsShell, focusActiveNav, clearMountFocus-effect, onWindowKeyDown, requestCloseEffect, requestArea, requestClose, discardAndContinue, SettingsScreen) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (App.tsx route /settings/:area → SettingsScreen; verified App route table). SHIPPED DESKTOP BINARY: NO — tauri host unbuildable at HEAD (baseline #2 exit 101; W14-A01 E0425 app.rs:301) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | sed persistence.ts:1-80 (localStorage seam real); sed config-settings.ts:1-260 (desktop bridge); rg "persist" audit/runs/W11-A*.log.md; sed W11-A09.log.md:40-55, W11-A02.log.md:27; rg tests SettingsScreen.test.tsx:30-380; rg SettingsProvider apply/save; rg tauri-transport send (no timeout) |

## 1. One-paragraph verdict
A real, carefully-built settings window: schema-driven navigation, cross-area
search through the same i18n seam that renders labels, dirty-state
confirm-on-navigate, focus trap, deep-link canonicalization — all executed by
substantive tests (SettingsScreen.test.tsx, passed in baseline
audit/runs/baseline/ui-vitest.log.txt). The screen's own code is honest about
which persistence seam it mounts (localStorage in the browser, config bridge
in the desktop shell). The fatal part is not in this file: the desktop
"REAL engine config" it round-trips to is, per W11, a dead in-memory BTreeMap
— nothing survives a restart, while this screen's Apply flow shows the user
an applied/restart notice. Close behavior also hardcodes /player regardless
of where the user came from.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): SettingsScreen.test.tsx
executes nav rendering, window shell, ✕/Escape close, Escape-with-dirty →
confirm dialog, focus trap, unknown-area redirect, slug canonicalization,
per-type primitives, feature-gate hiding, apply-persists bar, revert,
restart badges, cross-area search, query-clear-on-navigate (j12 P1-1),
single-field reset.
— verified by read: browser seam persistence.ts:52-77 writes/reads
localStorage under "lumen.settings.overrides.v1" with per-key type validation
on load — real for the dev shell.
— verified by read: config-settings.ts:120-215 dispatches real
config.set-setting/config.reset-setting/config.get-setting through the
transport with per-write outcome reporting (never swallowed) and
setting-changed subscription folding.

## 3. Findings
### F-001 [S2] "Apply round-trips the REAL engine config" — true at the wire, FALSE end-to-end: the engine store it writes is volatile
Where: ui/src/screens/settings/SettingsScreen.tsx:450-456
Code:
```
  // App-layer wiring (issue #3 — the 4.H1 handoff d): in the desktop
  // shell, Apply round-trips the REAL engine config over the client
  // bridge (config.set-setting / config.reset-setting; hydration via
  // config.get-setting) and `config.setting-changed` events keep the
  // store live. The browser dev shell keeps the localStorage seam —
```
What is wrong: This screen does dispatch the real commands — but the backend
they reach was audited in Wave 11: `config.set-setting` in the shipped CLI
host writes an in-memory BTreeMap with zero persistence (W11-A02 F-001:
"validated config.set-setting writes go to a dead in-memory BTreeMap — sole
reader is config.get-setting, zero persistence"); "no preferences.toml I/O
exists anywhere" (W11-A09 log). After an app restart every setting reverts to
product.toml defaults, while this screen told the user changes were applied
(line 407-411 renders `settings.restart-applied` as a role=status notice).
Why it matters: The user-facing settings feature is a fiction one restart
deep; the UI comment's "REAL engine config" framing hides that the round-trip
ends in volatile memory. The defect is backend-side, but this screen is the
surface that sells it.
What correct looks like: Either the engine persists (preferences.toml writer
— the missing half per W11-A02 F-002/A09), or the UI copy/notice honestly
labels desktop settings as session-only until the backend lands.
Confidence: HIGH — Verification: read + cross-cited W11-A02.log.md:27,
W11-A09.log.md:40-55 (static audits of host.rs:181,246,1403 — no I/O).
### F-002 [S3] ✕/Escape/discard always navigate to /player — the user's previous route is destroyed
Where: ui/src/screens/settings/SettingsScreen.tsx:264-277
Code:
```
  const requestClose = () => {
    if (settings.dirtyCount > 0) {
      setPending({ close: true });
      return;
    }
    navigate("/player");
  };
  const discardAndContinue = () => {
    const next = pending;
    setPending(null);
    settings.revert(); // discard: draft ← persisted
    if (next !== null && "area" in next) navigate(`/settings/${next.area}`);
    else navigate("/player");
```
What is wrong: Every close path hardcodes /player. A user who was on
/playlist, /convert or /discovery and opened settings (route or deep link)
is relocated to the player on close — the "floating window" metaphor
(Settings - *.svg frames: a window OVER the app) is implemented as a route
that eats history.
Why it matters: Wrong behavior in a realistic case (open settings from any
non-player context, close it, land somewhere you never were).
What correct looks like: `navigate(-1)` for the clean-close path (or route
state carrying the origin), with /player as the fallback when history is
empty.
Confidence: HIGH — Verification: read; test 84-95 pins the current behavior
("closes the window back to the player") — the tests encode the defect as a
contract.
### F-003 [S3] The hydration gate has no failure mode — a hung desktop transport leaves the settings screen on "loading" forever
Where: ui/src/screens/settings/SettingsScreen.tsx:465-487
Code:
```
  useEffect(() => {
    if (config === null) return;
    let cancelled = false;
    void config.hydrate().finally(() => {
      if (!cancelled) setHydrated(true);
    });
```
What is wrong: `hydrate()` resolves only when every `config.get-setting`
promise settles (config-settings.ts:238-243 `Promise.all(entries.map(...))`);
`readKey` catches rejections, but the tauri transport's `send`
(tauri-transport.ts:138-156) has NO timeout — an invoke that never answers
hangs the whole gate. The screen then renders `settings-hydrating` (line
480-486) with no retry, no error, no escape except closing the app.
Why it matters: One wedged IPC call (the desktop host is exactly the crate
Wave 14 proved unbuildable/untested) bricks the entire settings surface
silently.
What correct looks like: `Promise.race` with a timeout → surface a typed
"settings unavailable" state with a retry affordance.
Confidence: MEDIUM — Verification: read; hang path not reproducible in
sandbox (no desktop shell — baseline #2 exit 101).
### F-004 [S4] Search-label resolution is type-unsound by design
Where: ui/src/screens/settings/SettingsScreen.tsx:300-303
Code:
```
  const labelOf = (entry: { description: string }): string => {
    const text = t(entry.description as MessageKey);
    return typeof text === "string" ? text : "";
  };
```
What is wrong: `entry.description` is a schema string cast to MessageKey; if
a schema description key is absent from both the catalog and the lane
fallback, the entry silently vanishes from search results (empty label can
never match). Documented in-comment as intentional, but the cast defeats
compile-time key checking for schema→i18n drift.
Why it matters: A schema key rename silently drops that setting from search
with no error anywhere.
What correct looks like: A generated key-coverage check (l10n:check family)
asserting every SETTINGS_ENTRIES description resolves.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SIDEBAR_GROUPS / AREA_ICONS / iconOf | 75-108 | REAL | — | static config; orphan-ids caught by groupRows (line 127-131) so no registered area is unreachable |
| windowTitleKey | 113-116 | REAL | — | membership probe then registry fallback; type-safe cast |
| isAreaId / groupRows | 120-132 | REAL | — | orphan catch-all verified by read |
| FOCUSABLE | 140-141 | REAL | — | exported so the trap test can't drift (j12 P3-9) |
| SettingsShell | 143-446 | REAL-UNPROVEN | S2/S3 | executed by tests; F-001/F-002/F-004 live here |
| focusActiveNav (callback ref) | 188-194 | REAL | — | fires at mount incl. async nav arrival; data-mount-focus ring suppression cleared on first keydown (198-206) |
| onWindowKeyDown (Tab trap) | 208-227 | REAL | — | wraps first/last, skips prevented events; tested (110) |
| Escape effect | 239-250 | REAL | — | overlay-guard via DOM query; fresh closure via post-commit ref write (283-285) — correct pattern |
| requestArea / requestClose / discardAndContinue | 256-277 | REAL | S3 (F-002) | dirty-gated; hardcoded /player |
| query-clear effect | 163-165 | REAL | — | tested (247, 269) |
| SettingsScreen (route) | 449-496 | REAL-UNPROVEN | S2/S3 | desktop bridge selection; hydration gate F-003 |

## 5. Test honesty (SettingsScreen.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders schema-generated nav (35-59) | YES | asserts every registered area row present |
| ✕/Escape close, dirty → confirm (84-108) | YES | asserts navigation target + dialog |
| focus trap (110-136) | YES | asserts focus starts on active row and Tab wraps |
| apply persists and clears bar (195-206) | YES (mock) | browser/test persistence only — does NOT exercise the desktop bridge |
| revert discards without persisting (207-217) | YES | asserts draft ← persisted |
| search across areas + no-results + query-clear (230-283) | YES | real search.ts execution |
| single-field reset (320-341) | YES | per-field default reset |
Mutation check: delete `settings.revert()` inside discardAndContinue →
test 297 fails. Delete the query-clear effect → tests 247/269 fail. Strong
suite. Gap: no test mounts the desktop config-bridge path
(isDesktopShell() is never true in jsdom) — the F-001 surface is untested
end-to-end by design of the test env.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "in the desktop shell, Apply round-trips the REAL engine config" | lines 450-452 | SHALLOW | wire-level true; engine store volatile (W11-A02 F-001, W11-A09) |
| "the demo transport honestly answers feature-not-exposed for config.*" | lines 454-456 | REAL | config-settings readKey → null on typed error (verified read) |
| "restart badges + notices" contract (header 44-45) | file header | REAL | tested (218-229) |
| doc 07 §4.1 layer-4 "persisted settings" | via W11-A09 | FALSE end-to-end | W11-A09 log: "no preferences.toml I/O exists anywhere" |

## 7. Dead / duplicated / unreachable code
None. All exports consumed (FOCUSABLE → trap test; SettingsScreen → App route).

## 8. Dependencies & flags
react-router, components, i18n, generated schema, persistence,
config-settings, theme sync — in-tree. No feature flags; the desktop/browser
split is runtime probed (isDesktopShell).

## 9. Open questions for the next agent
- Does anything route users INTO /settings from non-player screens (menu
  wiring)? If every entry comes from the player's app menu, F-002's blast
  radius shrinks to deep links only.
- tauri-transport.ts has no send timeout (line 138) — should a global
  request deadline live in the transport rather than per-consumer gates?
