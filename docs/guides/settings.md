# Settings

Every setting Lumen knows is defined in **one schema**:
`schemas/settings.toml` (doc 07 §5). The UI settings screens, the CLI
`--help` text and the CLI flags are all **generated from it** — nothing
here is a second hand-written list. Defaults resolve through
`product.toml` (`default_ref`), so a rebrand/re-default never touches
the schema.

## The 14 areas

`schemas/settings.toml` `[areas] known` (UI-exposed screens carry
`ui = true`; engine-internal areas are CLI/docs-only):

| Area | UI screen | What it holds |
|---|---|---|
| `global` | yes (`/settings/global`) | interface language |
| `interface` | yes (`/settings/interface`) | theme, window geometry defaults |
| `privacy` | yes (`/settings/privacy`) | telemetry, crash reports, usage stats, update checks — all OFF by default |
| `playback` | yes (`/settings/playback`) | track languages, subtitle mode, hw-decode policy, start-paused, default rate |
| `performance` | yes (`/settings/performance`) | demux/network cache sizes, max open sessions |
| `video` / `audio` / `subtitles` / `controls` / `library` / `network` | schema-defined, screens land with their phases | — |
| `recording` / `extensions` | no (`ui = false`) | recording + extension-host internals |
| `engine` | no (`ui = false`) | log level, tick interval, event rate, probe bound, queue depths |

## Reading and changing settings (CLI)

```sh
$ lumen setting get playback.subtitle-mode
$ lumen setting set playback.subtitle-mode forced-only
```

One-shot flags mirror every key (`lumen --help <area>` lists them;
generated from the same schema):

```sh
$ lumen --help playback
  --playback-start-paused <on|off>
  --playback-default-rate <0.25..4>
  --playback-preferred-audio-languages <list>
  --playback-preferred-subtitle-languages <list>
  --playback-subtitle-mode <auto|forced-only|on|off>
  ...
```

## The keys and their defaults (21 UI entries + engine/extension rows)

| Key | Type | Default (source) |
|---|---|---|
| `global.language` | enum `en-US` | `product.toml defaults.locale` — restart required |
| `interface.theme` | enum `dark`/`light` | `product.toml defaults.theme` |
| `interface.start-maximized` | bool | `product.toml window.start_maximized` |
| `interface.remember-window-geometry` | bool | `product.toml window.remember_geometry` |
| `interface.window-width` / `-height` | int | `product.toml window.width` (1280) / `height` (720) |
| `interface.window-min-width` / `-height` | int | `product.toml window.min_width` (480) / `min_height` (320) |
| `privacy.telemetry` | bool | `product.toml telemetry.enabled` = **false** (opt-in only, MP §12) |
| `privacy.crash-reports` | enum `off`/`ask`/`send` | `telemetry.crash_reports` = **off** |
| `privacy.usage-stats` | bool | **false** |
| `privacy.update-checks` | bool | `telemetry.update_checks` = **false** |
| `playback.preferred-audio-languages` | BCP-47 list | `["en"]` (`product.toml`) |
| `playback.preferred-subtitle-languages` | BCP-47 list | `["en"]` |
| `playback.subtitle-mode` | enum `auto`/`forced-only`/`on`/`off` | `auto` |
| `playback.hw-decode-policy` | enum `software-only`/`prefer-hardware`/`hardware-only` | `software-only` — **feature-gated** (`features.hw-decode = false` until the Phase-7 integration passes its gate; ADR-001) |
| `playback.start-paused` | bool | `false` |
| `playback.default-rate` | float 0.25–4.0 | `1.0` |
| `performance.cache-demux-mib` | int 4–512 | `limits.cache_demux_mib` = 32 |
| `performance.cache-network-mib` | int 4–1024 | `limits.cache_network_mib` = 64 |
| `performance.max-open-sessions` | int 1–16 | `limits.max_open_sessions` = 4 |

Engine-internal (`ui = false`): `engine.log-level`,
`engine.tick-interval-ms`, `engine.max-event-rate`,
`engine.probe-max-bytes`, `engine.queue-packets-audio`,
`engine.queue-packets-video`, plus the extension-host crash-backoff
trio — see `schemas/settings.toml` for ranges and defaults.

Ranges are **reviewed safety bounds** (`product.schema.json`): an
out-of-range value is rejected with a diagnostic, never silently
clamped (MP §5).

## Feature gating (why some settings do nothing yet)

Effective availability = engine build features ∩ the `[features]`
table in `product.toml` ∩ host capability (doc 05 §6.4). Config can
never enable a missing capability: `playback.hw-decode-policy` is
inert while `features.hw-decode = false`, and the UI greys out
non-exposed commands from `system.list-commands` data — never from
hardcoded lists.
