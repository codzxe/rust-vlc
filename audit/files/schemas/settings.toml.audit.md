# AUDIT: schemas/settings.toml
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines / entries | 1627 / 121 `[[setting]]` entries (19 ui=false, 102 ui=true; 28 default_ref, 93 literal; 32 advanced; 21 `[VERIFY]` markers) |
| File verdict | SHALLOW — a real, well-validated *catalog* whose declared consumption surface (CLI flags, engine knobs, "decisions that ACTUALLY exist") is mostly wired to nothing |
| Reachable from shipped entry point? | YES — embedded via `include_str!` into settings-schema (crates/settings-schema/src/runtime.rs:18), reached from lumen-cli `setting get/set` (apps/cli/src/host.rs:1371,1402) and `--help` (apps/cli/src/help.rs). Executed. |
| Findings | S0: 0 S1: 0 S2: 3 S3: 4 S4: 3 |
| Commands run | `cargo test -p settings-schema --offline` (51 tests, all ok); `./target/debug/lumen setting get/set ...` (exit 0/64 verified); `./target/debug/lumen --global-language en-US` (exit 64); `./target/debug/lumen --help global|engine|recording|subtitles`; key-census rg loops over ui/src, apps, crates, plugins |

## 1. One-paragraph verdict
The catalog itself is genuine: 121 entries parse, self-validate D1–D6, the three committed generated artifacts are drift-free, and regeneration is idempotent (all proven by `cargo test -p settings-schema`, 51/51 ok, plus executed CLI runs). But the file's core claims — "single source of truth for every setting (… CLI flag …)", "entries mirror the decisions that ACTUALLY exist today" — are false at the consumption level. All 121 `cli.flag` values are advertised in `--help` output as "Settings surface as generated long flags" yet the argv parser rejects every one of them (executed: exit 64). `config.set-setting` validates and stores into a `BTreeMap` whose only reader is `config.get-setting` — an echo chamber. Of 121 keys, exactly 2 influence behavior anywhere (`interface.theme`, `global.accent-color` → UI theme sync), 5 more are consumed by plugin-host-wasm's supervisor which no shipped binary depends on, and 28 keys have zero references outside the generated schema and tests. The engine knobs (`engine.tick-interval-ms`, queue sizes, tick policy) are hardcoded in the CLI host, not read from this schema.

## 2. What actually works (verified)
— `cargo test -p settings-schema --offline` → 51 passed, 0 failed (43 unit + 8 integration incl. `committed_outputs_have_no_drift`, regeneration-idempotent, D1–D6 mutation tests). This file's parse/validate/generate pipeline is real.
— Executed `./target/debug/lumen setting set interface.theme dark` → `ok: set dark` (exit 0); `setting set interface.theme bogus` → exit 64 `rule V-ENUM: expected one of dark, light, auto`; `setting get no.such.key` → exit 64 `the schema defines 121 setting keys`. The validation seam works for all 121 keys.
— Executed `./target/debug/lumen --help global` → flags + defaults + `[gate: …]`/`[restart required]` markers render from this file.
— UI theme: `interface.theme`/`global.accent-color` are read by ui/src/theme/settingsThemeSync.ts:49-50 and actually drive ThemeProvider — the only behavioral consumers found.
— Key census: `comm` diff of the 121 keys vs ui/src/screens/settings/generated/schema.ts → empty (no drift between catalog and UI table).

## 3. Findings

F-001 [S2]
Where: schemas/settings.toml:69 (and 120 sibling `cli = { flag = … }` lines); evidence apps/cli/src/parse.rs:156-159, schemas/generated/settings-help.txt:7
Code (settings.toml:69, verbatim):
```
cli = { flag = "--global-language", short = "" }
```
Code (apps/cli/src/parse.rs:156-159, verbatim):
```
            "--json" => invocation.json = true,
            "--control-stdio" => invocation.control_stdio = true,
            "--version" | "-v" => invocation.version = true,
            "-h" | "--help" => {
```
What is wrong: every one of the 121 entries declares a CLI flag, and the generated help the user sees states "Settings surface as generated long flags (--<area>-<key> <value>)" (settings-help.txt:7) — but the argv parser accepts exactly 8 flags (--json, --control-stdio, --version, -h/--help, --start-time, --start-paused, --no-video, --no-audio); none of the 121 settings flags exist. Executed: `./target/debug/lumen --global-language en-US` → `error: unknown command '--global-language'`, exit 64. The D3 rule ("CLI flags … disjoint from reserved CLI names") polices a namespace that cannot be used. Why it matters: the help text actively instructs users into a guaranteed exit-64; the "CLI flag" field of the "single source of truth" is decoration. What correct looks like: either parse the flags (settings → store/apply) or stop advertising them as "command-line options" in the help golden. Confidence: HIGH — Verification: ran (exit 64 pasted above).

F-002 [S2]
Where: schemas/settings.toml:4 ("The single source of truth for every setting"), :24-25 ("entries mirror the decisions that ACTUALLY exist today"); evidence apps/cli/src/host.rs:1371-1373, 1402-1405
Code (host.rs:1402-1405, verbatim):
```
        settings_schema::runtime::validate_value(&full, &toml_value)
            .map_err(|err| invalid(err.to_string()))?;
        self.settings.insert(full, value.clone());
        Ok(CommandResult::SettingApplied {
```
Code (host.rs:1371-1373, verbatim):
```
            settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
        let value = match self.settings.get(&full) {
            Some(stored) => stored.clone(),
```
What is wrong: the entire settings store is `settings: BTreeMap<String, SettingValue>` (host.rs:181) whose only writer is `setting_set` and only reader is `setting_get`. rg over the repo: `self.settings` appears at exactly those 4 lines. No engine, window, audio, or playback code reads any key. `setting set playback.default-rate 4.0` changes nothing about playback. The 28 `default_ref` targets are product.toml slots the shipped CLI never even loads (apps/cli has no product-config dependency; host.rs:969 "values ride the product.toml mirror constants"). Why it matters: the catalog claims to be the product's decision surface; in the shipped binary it is a validated echo. What correct looks like: at least the default_ref-backed keys should flow into the open path (queue plan, tick policy, rate). Confidence: HIGH — Verification: read + executed seam + census. (Borderline S0 by the "doc says it works" test; held at S2 because the validation/echo seam itself is real and reachable, and the header hedges with "PHASE-1 SCOPE".)

F-003 [S2]
Where: schemas/settings.toml:370-380; evidence ui/src/screens/player/useChromeAutoHide.ts:47-49
Code (settings.toml:371-380, verbatim):
```
key = "interface.hide-controls-after"
type = "duration-ms"
range = { min = 0, max = 10000 }
default = 500             # frame C2 Playback Controls: "Hide after" slider, scale 0 s / 5 s / 10 s, small fill
description = "settings.interface.hide-controls-after"
restart_required = false
placement = { screen = "interface", section = "playback-controls", order = 30, ui = true }
cli = { flag = "--interface-hide-controls-after", short = "" }
feature_gate = ""
since = "0.1.0"
```
Code (useChromeAutoHide.ts:47-49, verbatim):
```
/** [PROPOSAL] idle delay before chrome hides (doc 08 §4.3 leaves timing open). */
export const CHROME_AUTO_HIDE_DELAY_MS = 3000;
```
What is wrong: a ui=true, user-editable, persisted setting (default 500 ms) that the player's auto-hide hook never reads — it hardcodes 3000 ms and takes no `useSettings` import. The same hook hardcodes the "fullscreen-only" policy that `interface.auto-hide-controls` (L358-368) claims to configure (`enabled: isFullscreen` in PlayerScreen.tsx:755). The settings screen shows a slider that lies. Why it matters: textbook "UI settings wired to nothing" (checklist 6.5). What correct looks like: `delayMs` defaulting from the persisted `interface.hide-controls-after` value. Confidence: HIGH — Verification: read (hook + census: hide-controls-after referenced only by settings-screen display files). Wave 15 should confirm on the UI side.

F-004 [S3]
Where: schemas/settings.toml (28 entries); representative: controls.* L1478-1583
What is wrong: 28 of 121 keys have ZERO references anywhere outside the generated schema and tests (verified by per-key `rg -F '"<key>"'` over ui/src, apps, crates, plugins, excluding tests/snapshots/generated): all 9 `controls.*`, `interface.remember-window-geometry`, `interface.start-maximized`, `interface.window-{width,height,min-width,min-height}`, `privacy.crash-reports`, `privacy.usage-stats`, `performance.{cache-demux-mib,cache-network-mib,max-open-sessions,drop-frames,realtime-priority}`, `engine.{max-event-rate,probe-max-bytes,queue-packets-audio,queue-packets-video}`, `recording.format`, `extensions.load-on-startup`. Declared-but-never-read. Confidence: HIGH — Verification: ran census (script in log). The engine.* limit entries are doubly dead: the CLI hardcodes the mirrors (`PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024` / `…VIDEO: u64 = 512`, demux_host.rs:71-73) and `TimeTickPolicy::default_policy()` (host.rs:977), never the schema.

F-005 [S3]
Where: schemas/settings.toml:1326-1401 (engine.*), 1408-1471 (extensions.crash-*); evidence host get path
Code (settings.toml:1339-1349, verbatim):
```
key = "engine.tick-interval-ms"
type = "duration-ms"
range = { min = 50, max = 2000 }
default = 250             # doc 05 §3.2 default 250 ms, floor 50 ms
description = "settings.engine.tick-interval-ms"
restart_required = true
placement = { screen = "engine", section = "internal", order = 20, ui = false }
cli = { flag = "--engine-tick-interval-ms", short = "" }
feature_gate = ""
since = "0.1.0"
advanced = true
```
What is wrong: `restart_required = true` on 18 entries is theater — nothing consumes the value, so no restart can ever be required. `engine.tick-interval-ms` claims to be the engine's tick; the engine gets `TimeTickPolicy::default_policy()` (hardcoded 250) and `TimeTickPolicy::from_config_ms` has no production caller. Additionally `setting get` on any of the 28 default_ref keys returns the literal placeholder string, not a value (host.rs:1567-1570: `DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}"))` — executed: `setting get privacy.telemetry` → `ok: -> product.toml telemetry.enabled`). Confidence: HIGH — Verification: read + ran.

F-006 [S3]
Where: schemas/settings.toml:1405-1406
Code (verbatim):
```
# The host consumes these through the embedded schema
# (plugin-host-wasm supervisor) — the values are never re-stated in code.
```
What is wrong: the claim is half-true. crates/plugin-host-wasm/src/supervisor.rs:170-171 does validate and map the 5 `extensions.crash-*` keys onto `BackoffPolicy` — real consumption — but NO app (apps/cli, apps/desktop, apps/desktop-tauri, apps/web, apps/control-server) depends on plugin-host-wasm (rg over all Cargo.tomls: zero references). The only genuine value-level consumer in the repo is unreachable from every shipped entry point. Confidence: HIGH — Verification: rg dependency scan.

F-007 [S4]
Where: schemas/settings.toml:42-48
Code (verbatim):
```
[areas]
known = [
    "global", "interface", "privacy", "controls",
    "playback", "video", "audio", "subtitles", "library",
    "network", "performance", "recording", "extensions",
    "engine",
]
```
What is wrong: 14 known screens; only 11 have entries. `subtitles`, `library`, `network` own zero `[[setting]]` rows — executed `./target/debug/lumen --help subtitles` → `error: unknown settings area 'subtitles' — known areas: audio, controls, engine, extensions, global, interface, performance, playback, privacy, recording, video`. The UI papers over the gap with hand-written area components (areas/index.tsx). Documented front-loading, cosmetic, but the "known" list lies about what the schema knows. Confidence: HIGH — Verification: ran.

F-008 [S4]
Where: schemas/settings.toml:1409-1471 (11 entries), evidence i18n catalog
What is wrong: the 11 ui=false entries' `description` msgids (`settings.engine.*`, `settings.extensions.crash-*`) exist nowhere in ui/src/i18n (per-msgid rg: all MISSING). D4 enforces uniqueness, never existence; the generated settings-docs.md therefore renders 11 label ids that resolve to nothing in any catalog. Harmless today (ui=false), but the docs page prints raw msgids. Confidence: HIGH — Verification: ran msgid census.

F-009 [S4]
Where: schemas/settings.toml — 21 `[VERIFY]` comment markers (L64, L176, L215, L283, L339, L690, L701, L738, L795, L950, L1005, L1017, L1079, L1178, L1190, L1202, L1248, L1260, L1273, L1286, L1355)
Code (L64, verbatim):
```
enumeration = ["en-US"]        # [VERIFY] dynamic locales/ listing (doc 07 §5.4)
```
What is wrong: 21 enumerations/defaults are self-flagged as Figma-frame-derived guesses ("vocabulary beyond shown value"). They are plausible but unproven against any engine capability (e.g. `video.deinterlace-method`'s yadif vocabulary vs the engine's actual deinterlacers — the engine has none in this build). Honest markers, unresolved debt; the vocabularies validate user input against invented domains. Confidence: HIGH — Verification: read (markers), engine registry per baseline (mjpeg+pcm only).

F-010 [S3] (added after fixture cross-check)
Where: schemas/settings.toml:434-445 vs product.toml:66-71
Code (settings.toml:435-438, verbatim):
```
key = "interface.window-min-width"
type = "int"
range = { min = 480, max = 3840 }
default_ref = "window.min_width"
```
Code (product.toml:66-71, verbatim):
```
# w28 fix 5: the UI floor (tauri.conf.json first-paint minWidth/minHeight
# and the W26 responsive layout's measured floor) — 940x600. The old
# 480x320 rows let users crush the desktop shell below its layout floor
# (a broken, "unsticky" UI); min sizes now agree with the design floor.
min_width = 940               # schema: min_width < width; = UI floor
min_height = 600              # schema: min_height < height; = UI floor
```
What is wrong: the w28 fix moved the product's window-min floor to 940x600 because "the old 480x320 rows let users crush the desktop shell below its layout floor (a broken UI)" — but the settings catalog still declares `interface.window-min-width` range `min = 480` (and min-height `min = 320`, L447-458) and validates 480 as legal. A user setting 500 passes schema validation yet produces exactly the broken UI the w28 fix outlawed. The D1 join checks the default_ref's existence, not the range's consistency with the referenced slot's own constraints. Same staleness applies to interface.window-min-height (min = 320 vs floor 600). Confidence: HIGH — Verification: read both files. (Moot today only because nothing consumes the value — see F-002 — but the catalog is the "source of truth" for the future consumer.)

## 4. Per-entry-class inventory (config file — per-area table)
| Area (entries) | ui=true/false | default_ref | Verdict | Notes |
|---|---|---|---|---|
| global (13) | 13/0 | 2 (defaults.locale, telemetry.update_checks) | SHALLOW | theme/accent consumed (UI); 11 display/echo only |
| interface (18) | 13/5 | 5 (window.*) | SHALLOW | hide-controls-after ignored by player (F-003); 5 window metrics referenced nowhere (F-004) |
| privacy (11) | 8/3 | 3 (telemetry.*) | SHALLOW | lastfm rows are pure decoration (no last.fm code in repo — rg: zero) |
| playback (17) | 17/0 | 6 | SHALLOW | onboarding writes 5 of them; engine reads none |
| video (13) | 13/0 | 0 | SHALLOW | brightness/hue/etc. → adjust filter not in this build; display wiring only |
| audio (11) | 11/0 | 0 | SHALLOW | startup-volume/maximum-volume → widget config only, Volume slider independent |
| performance (15) | 15/0 | 3 (limits.*) | SHALLOW | 5 keys referenced nowhere; limits hardcoded in CLI |
| engine (6) | 0/6 | 4 (limits.*) | STUB-as-declared | tick/queues/rate/probe never read (F-005) |
| extensions (6) | 1/5 | 0 | SHALLOW | 5 crash keys consumed only by unreachable supervisor (F-006) |
| controls (9) | 9/0 | 0 | DECLARED-ONLY | zero references anywhere (F-004) |
| recording (2) | 2/0 | 0 | GATED-INERT | feature recording=false; honest `[gate: recording]` in help (verified) |

## 5. Test honesty
| Test (settings-schema) | Would it fail if code were broken? | Reason |
|---|---|---|
| runtime::embedded_schema_parses_and_self_validates | YES | parses + D2–D6 on the real committed file |
| validate::tests d1..d6, e_gate_* | YES | mutation fixtures per rule (I ran them: all ok) |
| integration::committed_outputs_have_no_drift | YES | byte-compares all 3 generated artifacts vs regeneration |
| integration::regeneration idempotent + drift-exit-1 | YES | real generator binary, temp-dir fixtures |
| What NO test covers | — | whether any setting key is consumed by behavior (F-002..F-006); whether CLI flags parse (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "single source of truth for every setting (… CLI flag …)" | L4-5 | FALSE (flag half) | exit 64 run (F-001) |
| "entries mirror the decisions that ACTUALLY exist today" | L24-25 | FALSE | 2/121 keys affect behavior (F-002) |
| "Regenerate with `cargo xtask schema-gen`; a second run is byte-identical" | L11 | REAL | integration test passed (ran) |
| "D1–D6 … enforced by the validator" | L16-22 | REAL | 43 unit + 8 integration tests ok (ran) |
| "The host consumes these through the embedded schema (plugin-host-wasm supervisor)" | L1405-1406 | REAL-UNPROVEN/UNREACHABLE | supervisor code real; no app depends on the crate |
| "[VERIFY] dynamic locales/ listing" | L64 | honestly UNVERIFIED by authors; en-US-only is currently accurate | locales/ = en-US.toml only |

## 7. Dead / duplicated / unreachable code (catalog rows)
- 28 declared-never-read keys (F-004 list, census in log).
- 11 msgids resolving to nothing (F-008).
- `percent`/`list<path>` types in generated settings.d.ts union used by zero entries (grep: no `type = "percent"`).
- The d.ts header's promised value-bearing table path "ui/src/generated/settings-schema.ts" does not exist; the real table is ui/src/screens/settings/generated/schema.ts (stale path in generator output).

## 8. Dependencies & flags
- Consumed by: settings-schema (include_str! at compile time — file content IS the runtime schema), xtask schema_gen, plugin-host-wasm (supervisor defaults), CLI help.rs (G2 over embedded schema). No build-time feature gating of the file itself; 3 entries gated on product.toml features (recording, wasm-extensions, hw-decode — all false, verified in product.toml:164-168).

## 9. Open questions for the next agent
- Wave 15 (UI wiring): confirm `interface.hide-controls-after`/`auto-hide-controls`/`show-controls-in-fullscreen` reach no player code (I found none); confirm onboarding writes are display-only beyond theme/accent.
- Whoever owns apps/cli: is parsing the 121 flags planned, or should G2 stop calling them "command-line options"?
- Engine owner: `TimeTickPolicy::from_config_ms` (playback-engine/src/events.rs:173) is production-dead — settings `engine.tick-interval-ms` should be its caller.
- UNVERIFIED: `cargo xtask schema-gen` itself not run (cargo-build-budget: xtask not prebuilt); drift-freeness instead proven by the integration test that runs the schema-gen binary (passed).
