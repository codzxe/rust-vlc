# 07 — Config-Driven System: The Product Configuration Specification

| Field | Value |
|---|---|
| Task | 1.B7 (Wave 1, planning document — TOML/schema sketches, no implementation) |
| Status | implemented (awaiting Lead review) |
| Authoritative source | `docs/MASTER-PROMPT.md` (MP). Primary: MP §5 (config-driven rules). Also §2, §4 (layout), §7 (tokens), §13 (CI gates), §16 (phases). |
| Consumes | `docs/01-architecture.md` §2.3 (L1 config crates), §5 (boundary enforcement); `docs/05-command-layer.md` §2.11 (config commands), §5.3 (CLI), §6.4 (feature-gating), §8 (keymaps/menus as data); `design/README.md` (screen inventory); `docs/02-phase0-audit-plan.md` §4 (`feature-matrix.toml` coupling). |
| Parallel work | `docs/06-plugin-architecture.md` **absent at time of writing** (agent 1.B1 parallel). References to plugin manifests / `config_schema` are kept minimal and flagged for reconciliation when doc 06 lands. |
| Owning crates | `crates/product-config`, `crates/settings-schema`, `crates/localization`, `crates/theme-tokens`; tools: `tools/xtask` (rebrand, schema-gen, lint-literals, check-identity), `tools/config-validator`. |
| Inventions | Marked `[PROPOSAL]`; collected in §11. Unverified facts marked `[VERIFY]`. |

**Goal.** Everything in this document exists to make one sentence true and testable:
*the product is assembled from data — `product.toml`, schemas, catalogs, layouts, keymaps, profiles, themes — and CI fails when code tries to smuggle a product decision back into itself* (MP §5).

---

## 1. Philosophy and boundary — configure product decisions, not algorithms

MP §5 opens with the rule that governs everything below: **"Configure product decisions, not algorithms. Codec bitstream parsing, protocol syntax and OS API calls are technical constants and stay in code."**

### 1.1 What "product decision" means

A product decision is a choice among implemented behaviors that a distributor, admin, translator or user is entitled to make **without reading engine source**. Four litmus tests, all of which must pass for something to belong in config:

- **L1 — Rebrand test.** Would a downstream distributor want to change it while shipping the same engine? → config.
- **L2 — Translation test.** Would a localizer need to change it to serve a language? → config (specifically, a locale catalog).
- **L3 — Policy/mechanism test.** Is it a *policy* (which behavior, which default, which preference) rather than the *mechanism* (how the behavior is implemented)? → policy goes to config; mechanism stays code.
- **L4 — Safety-bound test.** Is it a *limit* (how much) rather than the *enforcement* (what happens when the limit is hit)? → the limit value goes to config, constrained to a reviewed safe range declared in the schema; the enforcement (backpressure, rejection, error) stays code. MP §8/§12 own the enforcement.

### 1.2 What belongs in config

| Category | Examples | Where |
|---|---|---|
| Names & identity | product name, publisher, identifiers, description | `product.toml` (§2) |
| Defaults | default theme/locale/profile, volume, rate, window geometry | `product.toml` + `schemas/settings.toml` |
| Exposure | which implemented features the product shows | `product.toml` `[features]` (§2.7) |
| Ordering | menu order, panel order, settings-screen order, track-language preference order | `layouts/`, settings-schema |
| Limits within safe ranges | cache sizes, queue depths, probe depth, playlist caps | `product.toml` `[limits]`, ranges in schema (§3.4) |
| Wiring | which command a button invokes, which key triggers it | layouts + keymaps (§7) |
| Copy | every user-visible string | `locales/` (§6) |

### 1.3 What stays in code

- **Bitstream syntax**: EBML ID lengths, H.264 NAL unit types, ADTS frame layout. These are facts of external specifications (clean-room sources, MP §1), identical for every distributor; putting them in config would imply they are product choices. They are *technical constants* (MP §5).
- **Protocol grammar**: HTTP header parsing, HLS playlist grammar, RTSP state machine.
- **OS API calls**: which system frameworks platform adapters invoke and how (MP §3.6); the *choice* of e.g. audio device is config/data, the *call* is code.
- **Algorithms**: A/V sync drift correction, seek pipeline, backpressure, color conversion, rebuffer policy.
- **Technical identifiers**: command IDs, key names (`"MediaNext"`), format names (`"H.264"`), error codes. These appear in UIs as data but are not product copy — they are not translated and not configurable (MP §5: "Technical identifiers are not product copy").

### 1.4 Borderline cases, with verdicts

| # | Case | Verdict | Reasoning |
|---|---|---|---|
| 1 | Preferred audio language `"en"` | **Config** (§2.8) | Pure policy (L3); ordered preference list. |
| 2 | EBML max element-ID length | **Code** | Bitstream syntax (MP §5 verbatim). |
| 3 | Video queue depth 512 packets | **Config**, range 64..=4096 in schema | Limit within a safe range (L4); the *range* is a reviewed schema bound, the *backpressure* is code (doc 03 §4). |
| 4 | A/V sync drift correction | **Code**; the *default policy* ("resample" vs "skip-repeat") is **config** | Mechanism vs policy (L3). Config may only choose among implemented policies (§2.7 rule F2). |
| 5 | `Right` = seek +10 s | **Keymap config** (§7.3) | Wiring (L1: a distributor ships different defaults). The seek pipeline is code. |
| 6 | Volume ceiling 125 % | **Config** (doc 05 §9 Q7) | Policy within the safe range the mixer was audited for (L4); the loudness curve is code. |
| 7 | Subtitle color "yellow" | **Config** — but the value must be a *theme token* reference, not raw hex (MP §7) | The palette is design data; the compositing is code. |
| 8 | "Cannot open file" wording | **Locale catalog** (§6) | Translation test (L2). The error *code* and taxonomy are code (doc 05 §2.13). |
| 9 | `"H.264"` shown in the track menu | **Code constant rendered as data** | Technical identifier, exempt from the no-literal-text lint (§6.5). |
| 10 | Hardware-decode policy | **Config enum** — currently a single legal value `software-only` pending ADR-001 (MP §2, doc 04) | Policy (L3), but each enum value requires an audited implementation before it may appear in the schema (rule F2). |
| 11 | Menu/panel order | **Config** (`layouts/`) | MP §5 verbatim: ordering is layout config. |
| 12 | Probe depth limit | **Config** (16 MiB, range 1–64 MiB); the sniffing logic is **code** | L4; also an MP §12 untrusted-input bound. |
| 13 | Which demuxer parses Matroska | **Code + plugin registry** | Capability selection by manifest (MP §6); config enables/reorders plugins, never invents them. |

### 1.5 The anti-patterns this document is built against

1. **The hidden fork**: a `#define`-style constant that is really a product decision (default language compiled into the engine). CI check: `lint-literals` + `check-identity` (§9).
2. **The silent fallback**: invalid config quietly replaced by a default (MP §5: "never silently replaced"). CI check: validator golden tests (§10.1).
3. **The phantom feature**: config exposing something unimplemented (MP §5: "Feature exposure lists only implementations that exist"). CI check: feature-registry cross-validation (§3.4).
4. **The double definition**: the same option defined in two files that drift apart. CI check: uniqueness rules (§5.5).

---

## 2. `product.toml` — complete initial draft

This is the **real file content for the Phase 1 commit** (MP §16 Phase 1 lists `product.toml` explicitly), every key documented inline. It is validated by `product.schema.json` (§3) via `cargo xtask config-validator`. Structural note: user-changeable settings live in `schemas/settings.toml` (§5), **not** here — `product.toml` carries the *product's* defaults and decisions; §5.6 defines how the two files reference each other without duplicating a literal (rule D1).

```toml
# ============================================================================
# product.toml — the product definition of Lumen (working name).
#
# Single source of truth for product decisions (MP §5). Validated by
# product.schema.json via `cargo xtask config-validator`; a violation is a
# build-breaking error with an actionable diagnostic (MP §5), never a silent
# fallback. Do not hand-edit files generated from this one (§8, §9).
# ============================================================================

schema_version = 1

# --- 2.1 Identity -----------------------------------------------------------
# Rebrandable without touching code: `cargo xtask rebrand` consumes these
# values (§8). Every surface that shows a name reads from here at runtime;
# packaging metadata is regenerated from here at build time.

[identity]
# Full display name: window idle title, About, installer. 1–40 chars.
name = "Lumen"
# Compact name for space-constrained surfaces (taskbar, tray dialogs,
# file-manager "opens with" lists). 1–12 chars, no spaces.
short_name = "Lumen"
# Legal entity shown in About copyright and installer metadata.
publisher = "Lumen Contributors"          # [PROPOSAL] placeholder entity
# Human-facing version string. NOT the engine/catalog/protocol versions —
# those are reported by system.get-engine-info (doc 05 §2.12).
display_version = "0.1.0-dev"
# Monotonic build number for installers/updaters that compare integers.
version_code = 1
# Product description: a localization key, never literal text (MP §5).
# The prose lives in locales/en-US.toml under this key.
description_key = "product.description"

[identity.ids]
# Reverse-DNS root from which per-OS identifiers derive — one literal,
# many surfaces (§8.2). Regex: ^[a-z0-9]+(\.[a-z0-9-]+)+$  (all lowercase).
reverse_dns = "dev.lumen"

[identity.ids.windows]
# AppUserModelId (taskbar grouping, notifications) and the MSI UpgradeCode
# (must stay stable across versions so upgrades replace, not co-install).
app_user_model_id = "dev.lumen.Lumen"
msi_upgrade_code = "7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701"   # [PROPOSAL] generate once, then frozen

[identity.ids.macos]
bundle_id = "dev.lumen.Lumen"

[identity.ids.linux]
# Desktop-entry / AppStream ID; flatpak/snap IDs derive from it.
appstream_id = "dev.lumen.Lumen"

[identity.ids.web]
# PWA identity (origin-independent), used in the web manifest.
app_id = "dev.lumen.Lumen"

# --- 2.2 Window defaults ----------------------------------------------------
# Reference viewport: the design frames export at 1920x1080 (design/figma/
# Player.svg). The default window is 1280x720 — same 16:9 shape, one step
# down, so the layout scales 2:3 without reflow; the design remains the
# fidelity target at its native size (doc 08). [VERIFY] final size pinned by
# doc 08 screen analysis.

[window]
width = 1280                  # schema 480..=3840
height = 720                  # schema 320..=2160
min_width = 480               # schema: min_width < width
min_height = 320              # schema: min_height < height
start_maximized = false
# Remember window geometry across runs (position + size, per monitor).
remember_geometry = true
# Title template (MP §5). Allowed variables, validated by schema:
#   {product_name}  -> identity.name
#   {media_title}   -> current media title, else the idle title below
# Unknown variables are a validation error. The template composes; wording
# (the idle string) is localized (window.title.idle, §6). The em-dash is a
# template separator, not translatable prose. RTL-aware templating is a
# documented future extension [PROPOSAL].
title_template = "{media_title} — {product_name}"
# Idle title (no media open): localization key rendered with {product_name}.
title_idle_key = "window.title.idle"

# --- 2.3 Brand assets -------------------------------------------------------
# Paths are repo-relative; existence is checked by config-validator (§3.4).
# Rebrand swaps these files + the accent (§8); nothing else may hardcode
# brand colors or logos (§9, duplicated-identity check).

[brand]
# Accent color. MUST equal the theme token `color.accent.primary` once the
# token extraction lands (doc 08); until then this literal is provisional.
# [VERIFY] against design package token extraction.
accent_color = "#4f8cff"
# Per-platform icons (validated formats: ico / icns / png-512).
icon_ico = "assets/brand/icon.ico"
icon_icns = "assets/brand/icon.icns"
icon_png_512 = "assets/brand/icon-512.png"
# Vector logo for About, onboarding and docs.
logo_svg = "assets/brand/logo.svg"

# --- 2.4 Product defaults ---------------------------------------------------

[defaults]
# Theme id: must match themes/<id>.toml. The design package is dark-first
# (design/README.md), so the product default is dark; light ships alongside.
theme = "dark"
# Default locale: must match locales/<tag>.toml. en-US is the reference
# catalog (§6.3) and therefore always available.
locale = "en-US"
# Default playback profile id: must match profiles/<id>.toml (§7.4).
profile = "default"

# --- 2.5 Schema version & migrations ----------------------------------------
# See [migrations] below; schema_version is validated == the max `to` of
# all migration entries (or 1 when empty).

# --- 2.6 Telemetry & privacy (opt-in only, default OFF) ---------------------
# MP §5 "telemetry and privacy defaults (opt-in only)" + MP §12 "No default
# telemetry". Shipped defaults are off; users may opt in. An admin lock
# (§4.3) can additionally force these off.

[telemetry]
enabled = false               # schema: bool, shipped default false — hard rule
# Crash-report behavior: "off" | "ask" | "send". Never silently above "off".
crash_reports = "off"
usage_stats = false
update_checks = false

# --- 2.7 Feature exposure (initially minimal) -------------------------------
# MP §5: "Feature exposure lists only implementations that exist. Config can
# never enable a missing codec, protocol or hardware capability."
#
# Semantics (doc 05 §6.4): effective availability =
#     engine build features  ∩  this table  ∩  host capability (e.g. web).
# A feature id that is not in the engine's static feature registry is a
# VALIDATION ERROR here (rule F1) — you cannot even name an unimplemented
# feature, let alone enable it. UIs grey out non-exposed commands from data
# (system.list-commands + get-engine-info), never from hardcoded lists.
#
# Initial set = exactly what the Phase 2 vertical slice implements (MP §16):
# one real file, open to decode to audio and video on screen, through the
# command layer, from CLI and UI. Everything else is explicitly false with
# its target phase — false entries document intent, they enable nothing.

[features]
core-playback = true            # session open/play/pause/seek/stop of local files
local-file = true               # file:// source access
audio-output = true             # default audio device output
video-render = true             # video presentation (video-in-webview prototype, MP §7)
track-selection = true          # tracks.set-*-track over available streams
config-surface = true           # config.get/set/reset-setting + settings UI
# --- explicitly off until the implementing phase ships (MP §16) ---
playlists = false               # Phase 4
library = false                 # Phase 4
streaming-http = false          # Phase 6
streaming-hls = false           # Phase 6
streaming-dash = false          # Phase 6
streaming-rtsp = false          # Phase 6
optical-disc = false            # Phase 7, ADR-gated (MP §14)
device-capture = false          # Phase 7
audio-device-enumeration = false # Phase 3/4, host-dependent
recording = false               # Phase 7
transcode = false               # Phase 7
wasm-extensions = false         # Phase 7
control-server = false          # Phase 5+ [PROPOSAL]
hw-decode = false               # pending ADR-001 — mirrors [playback] policy below
# Rule F2: enabling a feature requires its implementation to exist AND pass
# its gate (feature-matrix.toml row + tests); the validator cross-checks
# exposure against the registry (§3.4), config alone enables nothing.

# --- 2.8 Default playback & track-selection rules ---------------------------

[playback]
# Ordered BCP-47 preference lists (MP §5 "track selection preferences").
# Matching: exact tag first, then primary subtag (en matches en-GB), then
# any track. Empty list = no preference (first track wins).
preferred_audio_languages = ["en"]
preferred_subtitle_languages = ["en"]
# Subtitle mode at open: "auto" (select a preferred-language or forced
# track), "forced-only", "on" (select best), "off".
subtitle_mode = "auto"
# Hardware-decode policy — PENDING ADR-001 (MP §2; doc 04 gap register).
# "software-only" is the ONLY legal value until the ADR lands; the schema
# enum grows ("prefer-hardware", "hardware-only") only after an audited
# implementation exists (rule F2). Reporting of active hardware decode is
# always truthful via system.get-engine-info (doc 05 §2.12).
hw_decode_policy = "software-only"
start_paused = false            # new sessions begin paused (VLC parity item)
default_rate = 1.0              # schema 0.25..=4.0 in v1

# --- 2.9 Cache & resource limits (safety-bounded) ---------------------------
# Values are config; RANGES live in product.schema.json and are reviewed
# safety bounds (L4). Out-of-range values are rejected with a diagnostic —
# never clamped (MP §5). Enforcement (backpressure, rejection) is code.

[limits]
cache_demux_mib = 32            # demux read-ahead. schema 4..=512
cache_network_mib = 64          # network buffer. schema 4..=1024
queue_packets_audio = 1024      # bounded audio packet queue (doc 03 §4). schema 64..=8192
queue_packets_video = 512       # bounded video packet queue. schema 64..=4096
probe_max_bytes = 16777216      # probing bound (MP §12). schema 1 MiB..=64 MiB
max_playlist_items = 10000      # schema 100..=100000
max_open_sessions = 4           # schema 1..=16
max_wasm_extension_memory_mib = 64  # Phase 7 sandbox bound. schema 8..=256

# --- 2.10 Product URLs ------------------------------------------------------

[urls]
# Rendered in the Help menu and About (doc 05 §8.3); schema enforces https://.
# [PROPOSAL] placeholder domains until a distribution identity exists.
help = "https://lumen-player.example.org/docs"
support = "https://lumen-player.example.org/support"
privacy = "https://lumen-player.example.org/privacy"
update_feed = "https://updates.lumen-player.example.org/feed.json"

# --- 2.11 Migrations --------------------------------------------------------
# Placeholder (MP §5 "schema version and migrations"). Born at schema_version 1,
# so the list is empty. Contract for future entries:
#   [[migrations.entries]]
#   from = 1
#   to = 2
#   summary = "renamed limits.cache_mib -> limits.cache_demux_mib"
#   migrator = "config-migrate-1-to-2"   # xtask subcommand; deterministic,
#                                        # idempotent, unit-tested (§10.2)

[migrations]
# entries = []
```

**Alignment with the design package.** The product's UI ships the screens inventoried in `design/README.md`: the 14-file Settings group (global, interface, privacy, playback, video, audio, subtitles, controls, library, network, performance, recording, extensions — see the count note in §5.4), the Player family (dark default, Light theme, Music mode, hide-UI, customize), Miniplayer, onboarding flows, dialogs (network stream, keyboard shortcuts), and the audio/video tool panels. `defaults.theme = "dark"` reflects the dark-first package; the Settings screens are generated from the settings-schema placement data (§5.4), and the tool panels are declared in `layouts/` (§7.2).

---

## 3. `product.schema.json` — validation design

### 3.1 What it is

`product.schema.json` is a JSON Schema (draft 2020-12 `[PROPOSAL]` — version pinned by the validator dependency, subject to the Phase 0 dependency audit) that defines every legal `product.toml` shape. It is itself data: it lives in `schemas/` next to the file it validates, is version-controlled, and changes only through review — schema edits are policy changes (§1.4 case 3) and require the same care as code.

### 3.2 Structure sketch

```jsonc
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://lumen-player.example.org/schemas/product.v1.json",
  "title": "Lumen product definition",
  "type": "object",
  "required": ["schema_version", "identity", "window", "brand", "defaults",
               "features", "playback", "limits", "telemetry", "urls", "migrations"],
  "additionalProperties": false,          // unknown keys are errors: typo detection
  "properties": {
    "schema_version": { "const": 1 },    // must equal max(migrations.to) or 1
    "identity": {
      "type": "object", "required": ["name", "short_name", "publisher",
        "display_version", "version_code", "description_key"],
      "properties": {
        "name":           { "type": "string", "minLength": 1, "maxLength": 40,
                            "not": { "enum": ["VLC", "VideoLAN", "vlc"] } },
        "short_name":     { "type": "string", "pattern": "^[A-Za-z0-9-]{1,12}$" },
        "display_version":{ "type": "string", "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+(-[A-Za-z0-9.]+)?$" },
        "version_code":   { "type": "integer", "minimum": 1 },
        "description_key":{ "$ref": "#/$defs/msgid" }   // must exist in the reference catalog
      }
    },
    "window": {
      "type": "object", "required": ["width", "height", "min_width", "min_height",
                                     "title_template", "title_idle_key"],
      "properties": {
        "width":  { "type": "integer", "minimum": 480, "maximum": 3840 },
        "height": { "type": "integer", "minimum": 320, "maximum": 2160 },
        "min_width":  { "type": "integer", "minimum": 320 },
        "min_height": { "type": "integer", "minimum": 200 },
        "title_template": {
          "type": "string",
          "pattern": "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$",
          "description": "only {product_name} and {media_title} are defined"
        },
        "title_idle_key": { "$ref": "#/$defs/msgid" }
      }
    },
    "brand": {
      "type": "object",
      "required": ["accent_color", "icon_ico", "icon_icns", "icon_png_512", "logo_svg"],
      "properties": {
        "accent_color": { "type": "string", "pattern": "^#[0-9a-f]{6}$" }
        // icon_* and logo_svg: type string, repo-relative, POSIX-safe; existence
        // and format are checked in the validator's semantic pass (§3.4)
      }
    },
    "defaults": {
      "type": "object", "required": ["theme", "locale", "profile"],
      "properties": {
        "theme":   { "type": "string", "enum": [] },   // enum injected at validation
        "locale":  { "type": "string", "enum": [] },   // time from locales/ listing
        "profile": { "type": "string", "enum": [] }    // (cross-file rules, §3.4)
      }
    },
    "features": {
      "type": "object", "additionalProperties": { "type": "boolean" }
      // unknown feature ids rejected in the semantic pass against the
      // engine feature registry (rule F1) — a static enum here would drift
    },
    "playback": {
      "type": "object",
      "properties": {
        "preferred_audio_languages":    { "type": "array", "minItems": 0, "maxItems": 8,
                                          "items": { "$ref": "#/$defs/bcp47" } },
        "preferred_subtitle_languages": { "type": "array", "minItems": 0, "maxItems": 8,
                                          "items": { "$ref": "#/$defs/bcp47" } },
        "subtitle_mode":     { "enum": ["auto", "forced-only", "on", "off"] },
        "hw_decode_policy":  { "enum": ["software-only"] },   // grows only post-ADR-001 (rule F2)
        "start_paused":      { "type": "boolean" },
        "default_rate":      { "type": "number", "minimum": 0.25, "maximum": 4.0 }
      }
    },
    "limits": { /* cache_demux_mib 4..=512, cache_network_mib 4..=1024,
                   queue_packets_audio 64..=8192, queue_packets_video 64..=4096,
                   probe_max_bytes 1MiB..=64MiB, max_playlist_items 100..=100000,
                   max_open_sessions 1..=16, max_wasm_extension_memory_mib 8..=256 */ },
    "telemetry": {
      "type": "object",
      "properties": {
        "enabled":       { "type": "boolean", "default": false },
        "crash_reports": { "enum": ["off", "ask", "send"], "default": "off" },
        "usage_stats":   { "type": "boolean", "default": false },
        "update_checks": { "type": "boolean", "default": false }
      }
    },
    "urls": { /* all type string, format uri, pattern ^https:// */ },
    "migrations": { /* entries[]: from < to, ascending, no gaps, no dupes */ }
  },
  "$defs": {
    "msgid":  { "type": "string", "pattern": "^[a-z0-9-]+(\\.[a-z0-9-]+)+$" },
    "bcp47":  { "type": "string", "pattern": "^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$" }
  }
}
```

### 3.3 Rule inventory

| Class | Rules |
|---|---|
| **Required fields** | Top-level keys per §3.2 `required`; identity core (name, short_name, publisher, display_version, version_code, description_key); window geometry + both title keys; all brand assets; the three defaults; all feature groups. |
| **Enums** | `subtitle_mode`, `hw_decode_policy`, `crash_reports`; **injected enums** for `defaults.theme/locale/profile` (from the `themes/`, `locales/`, `profiles/` directory listings at validation time — cross-file, see §3.4). |
| **Ranges** | Geometry, all `[limits]` values, `default_rate`. Ranges encode *reviewed safety bounds* (L4) — widening one is a reviewed schema change, never a config tweak. |
| **Unique-identity constraints** | (a) `name`/`short_name` must differ from the reserved list (`VLC`, `VideoLAN` — clean-room, MP §1); (b) `reverse_dns` and derived identifiers must be mutually consistent (`app_user_model_id == reverse_dns + "." + PascalCase(name)` `[PROPOSAL]`); (c) `msi_upgrade_code` must be a stable GUID — validator warns if it changes between commits (diff-aware check) `[PROPOSAL]`. |
| **Path existence** | `brand.icon_*`, `brand.logo_svg` must exist in-repo and have the right magic bytes (ICO/ICNS/PNG/SVG). JSON Schema cannot touch the filesystem, so this is the validator's semantic pass. |
| **Cross-field rules** | `min_width < width`, `min_height < height`; `schema_version == max(migrations.entries.to)`; `defaults.theme ∈ themes/`, `defaults.locale ∈ locales/`, `defaults.profile ∈ profiles/`; `description_key` and `title_idle_key` exist in the reference catalog; `features` keys ⊆ engine feature registry (F1); any `features.* = true` must reference a feature whose implementing crates are compiled in (F2, engine-build intersection per doc 05 §6.4); `telemetry.enabled = true` requires `urls.privacy` to be set (privacy policy must exist before data flows). |

### 3.4 `tools/config-validator` — how the schema is invoked

The validator is a small binary (dependency-audited; a pure-Rust JSON Schema implementation is a Phase 0 `[VERIFY]` item) run as `cargo xtask config-validator`. Pipeline, in order — each stage can fail independently:

1. **Parse**: `product.toml` → TOML → `serde_json::Value` (via typed `product-config` structs; unknown keys fail here thanks to `deny_unknown_fields`).
2. **Structural validation**: run JSON Schema over the value; report every violation, not just the first.
3. **Semantic passes**: path existence + magic bytes; injected enums from directory listings; cross-field rules of §3.3; feature-registry intersection (with `docs/feature-matrix.toml` per doc 02 §4 — exposure must also agree with matrix rows marked implemented).
4. **Sibling configs**: same treatment (with their own schemas) for every shipped `profiles/*.toml`, `keymaps/*.toml`, `layouts/*.toml`, `themes/*.toml`, `locales/*.toml`, and plugin manifests (schema shared with doc 06 when it lands).
5. **Exit**: `0` clean; non-zero with the aggregate diagnostic list. CI gate (§9).

### 3.5 Actionable diagnostics — the error-message contract

MP §5: "Invalid critical config fails with an actionable diagnostic. It is never silently replaced." A diagnostic is *actionable* only if it names the file, the key path, the violated rule, and a hint. Mandatory format `[PROPOSAL]`:

```
error[config]: product.toml:limits.probe_max_bytes
  value: 1048576 (1.0 MiB)
  rule:  must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)
  hint:  probing below 16 MiB rejects valid multi-track files; if you need a
         lower bound for an embedded target, change the schema range via review
         (docs/07-config-driven-system.md §3.3), not this value.
```

Rules: (1) the **key path** is always fully qualified from the file root (`file:key.path`); (2) the **rule** is stated as the constraint, with units expanded for humans; (3) the **hint** points to a fix, or to the document that governs the rule; (4) where a near-miss key exists (`cache_demux_mib` vs a typo'd `cache_demux_mb`), the hint says "did you mean `…`" (Levenshtein over the known key set `[PROPOSAL]`); (5) the runtime behavior on any critical violation is **abort into the config-error surface** (desktop: full-screen config-error state with the diagnostic rendered and a "reset to defaults" action that moves the offending file aside — it does not delete it; CLI: non-zero exit) — never a fallback default, never a silently-different product.

---

## 4. Layered precedence model

### 4.1 The six layers (MP §5, lowest → highest)

| # | Layer | Nature | Written by |
|---|---|---|---|
| 1 | **Product defaults** | `product.toml` + `schemas/settings.toml` defaults (§5) | distribution (build-time, read-only at runtime) |
| 2 | **Profile** | `profiles/<id>.toml` — a named bundle of setting overrides (§7.5) | distribution ships seeds; user can select/clone |
| 3 | **Admin override** | managed config + lock list | IT administrator (per-OS mechanism, §4.3) |
| 4 | **User preferences** | persisted settings written by `config.set-setting` | the user |
| 5 | **Command-line flags** | process args, generated from the settings schema | the invoker |
| 6 | **Session** | in-memory, per-session transient values | engine/session runtime |

Layer 6 semantics: a setting declared `scope = "session"` (§5.2) may be overridden per open media or per app run without persistence; persisted settings cannot be *written* by the session layer. This extends doc 05 §2.11 — `config.set-setting` routes by the key's declared scope: user-scope keys persist (layer 4), session-scope keys stay in memory (layer 6) `[PROPOSAL]`. v1 ships with very few session-scoped keys (candidate: `playback.rate` resets on close) — the full set is a Lead decision (§11).

### 4.2 Per-layer file locations per platform

| Layer | Windows | macOS | Linux |
|---|---|---|---|
| 1 Product | `<install>\config\` (read-only; Program Files) | `Lumen.app/Contents/Resources/config/` | `/usr/share/lumen/` (or `/app/share/lumen/` in Flatpak `[PROPOSAL]`) |
| 2 Profile (shipped) | same as product, `profiles\` | `<bundle>/Resources/profiles/` | `<datadir>/lumen/profiles/` |
| 2 Profile (user clones) | `%APPDATA%\Lumen\profiles\` | `~/Library/Application Support/Lumen/profiles/` | `$XDG_CONFIG_HOME/lumen/profiles/` |
| 3 Admin | registry `HKLM\SOFTWARE\Policies\dev.lumen\Lumen\` (GPO); fallback file `%ProgramData%\Lumen\managed.toml` `[PROPOSAL]` | MDM configuration profile (`.mobileconfig` keys under `dev.lumen.Lumen`) or `/Library/Application Support/Lumen/managed.toml` `[PROPOSAL]` | `/etc/lumen/managed.toml` (first match across `$XDG_CONFIG_DIRS` `[PROPOSAL]`) |
| 4 User | `%APPDATA%\Lumen\preferences.toml` | `~/Library/Application Support/Lumen/preferences.toml` | `$XDG_CONFIG_HOME/lumen/preferences.toml` (~/.config/lumen) |
| 5 Flags | process argv | process argv | process argv |
| 6 Session | memory only | memory only | memory only |

Platform dir resolution is a `platform-*` adapter responsibility (MP §3.6 — no OS types leak into `product-config`; the adapter supplies paths). Web host: layers 1–2 only (compiled into the wasm bundle); layers 3–5 do not exist (browser URLs cannot carry argv); layer 6 plus browser storage-backed user preferences `[PROPOSAL]` — see doc 09 for the web column.

### 4.3 Admin override mechanism `[PROPOSAL]`

The admin layer is a normal settings file plus a **lock list**: `locked = ["playback.hw-decode-policy", "telemetry.enabled", …]`. Semantics:

- An admin-layer *value* overrides product/profile values (normal precedence).
- A *locked* key cannot be overridden by layers 4–6. `config.set-setting` returns `permission-denied` (doc 05 §2.11); a CLI flag targeting a locked key is a **hard startup error** (loud, per MP §5); a stale locked key found in the user file is a logged warning and the user value is ignored — boot must not fail because an admin tightened policy after the user saved preferences.
- Windows registry values mirror the TOML key paths (e.g. `HKLM\…\playback\hw-decode-policy`, REG_SZ/REG_DWORD); the registry reader is part of the Windows platform adapter and is an audited OS-API exception (MP §9). macOS MDM keys use the same paths in the `dev.lumen.Lumen` preference domain.
- Admin files pass the same schema validation; an invalid admin file is a **startup error** with the §3.5 diagnostic (critical config).

### 4.4 The merge algorithm

Implemented in `crates/product-config` (doc 01 §2.3 sketch):

1. **Deep merge for tables**: recursively; a key present in a higher layer replaces the lower value (after type validation).
2. **Scalars**: replace.
3. **Arrays**: default policy is **replace** (the whole list is one value — e.g. `preferred_audio_languages`). A setting may declare `array_policy = "append-unique"` in the schema (§5.2) where additive semantics are intended `[PROPOSAL]` — v1 uses replace everywhere except plugin enable-lists (doc 06 reconciliation).
4. **Typed overrides**: every layer's value for a known key is validated against the same schema entry (type, range, enum) *before* merging. Out-of-range in *any* layer = §3.5 diagnostic. **Unknown keys in any layer are errors** (typo detection), with did-you-mean hints.
5. **No null/unset**: TOML has no null; "reset" is a command (`config.reset-setting`) that removes the key from the user layer, exposing the next-lower value.
6. **Locks**: applied after merge — locked keys resolve to the admin value regardless of higher layers (§4.3).
7. **Determinism**: the merge is a pure function of the ordered layer list; the resolved snapshot is itself serializable (for `system.get-state-snapshot` hydration and tests).

### 4.5 Precedence test matrix (the MP §5 "document it and test it" demand)

`crates/product-config` tests build synthetic layer stacks and assert resolved values. The matrix is exhaustive over layer *interactions*; each row is a named test:

| # | Scenario | Expected |
|---|---|---|
| T1 | only product layer | product default resolves |
| T2 | profile overrides product | profile value |
| T3 | admin overrides profile+product | admin value |
| T4 | user overrides 1–3 | user value |
| T5 | flag overrides 1–4 | flag value |
| T6 | session value on session-scoped key overrides 1–5 | session value; gone after session close |
| T7 | session write on user-scoped key | rejected (`invalid-argument`) |
| T8 | admin lock + user value | admin value; `config.set-setting` → `permission-denied`; user file logs warning |
| T9 | admin lock + CLI flag | hard startup error with diagnostic |
| T10 | unknown key in user layer | startup error, did-you-mean hint |
| T11 | out-of-range value in profile layer | startup error (profile is shipped/critical) |
| T12 | out-of-range value in user layer | config-error surface (§3.5), offer reset |
| T13 | array with policy `replace` | higher layer list fully replaces |
| T14 | missing product layer file | impossible in packaged builds (embedded); dev-mode diagnostic |
| T15 | two layers agree | value resolves once, no double-apply artifacts |
| T16 | `config.reset-setting` after override | next-lower value returns |

---

## 5. `settings-schema` — the single source of truth

### 5.1 Principle

MP §5: "One settings schema (type, range, description, restart-required, UI placement) generates the settings screens, --help and CLI flags (VLC has thousands of options), documentation and validation. **No option is defined twice.**" The authored artifact is `schemas/settings.toml` (TOML: it is human-written). Everything else — TS types, CLI flag tables, docs, runtime validation — is *generated from it* and committed for reviewability; CI proves the generated files match regeneration (§9).

### 5.2 Entry format

```toml
[[setting]]
key = "playback.seek-step-short"        # dotted area.key — the canonical ID
type = "float"                          # bool|int|float|string|enum|path|url|
                                        # list<T>|duration-ms|percent
range = { min = 1.0, max = 60.0 }       # or enumeration = ["auto","on","off"]
unit = "seconds"                        # display + CLI parsing hint [PROPOSAL]
default = 10.0                          # OR default_ref (below) — exactly one
description = "settings.playback.seek-step-short"   # msgid (§6); never literal text
restart_required = false                # true => UI shows restart hint; CLI exits with
                                        # "applied, restart required" (doc 05 §2.11)
placement = { screen = "playback", section = "navigation", order = 30 }
                                        # screen: one of layouts/settings-areas.toml ids
                                        # (§5.4); ui = false for engine-internal entries
cli = { flag = "--playback-seek-step-short", short = "" }   # short flags from a fixed
                                        # reserved table; collisions fail the validator
feature_gate = "core-playback"          # absent = ungated; entry hidden+inert when the
                                        # feature is not exposed (doc 05 §6.4)
since = "0.1.0"                         # first product version shipping this entry
# --- optional, all [PROPOSAL] extensions ---
scope = "user"                          # "user" (persisted, layer 4) | "session" (layer 6)
array_policy = "replace"                # §4.4 rule 3
advanced = false                        # VLC-style "show advanced" folding
deprecated_in = ""                      # set when retiring; docs mark it; removal only at
                                        # a schema major bump (doc 05 §6.3 analogy)
```

Mandated fields: `key`, `type`, `range`/`enumeration`, `default`/`default_ref`, `description` (msgid), `restart_required`, `placement`, `cli.flag`, `feature_gate`, `since` — every entry carries all of them (empty string / `false` where not applicable), so generators never special-case. **Naming conventions**: setting keys are kebab-case dotted ids (`playback.hw-decode-policy`), matching command-ID style (doc 05 §2.1); `product.toml` table keys are TOML-conventional snake_case (`hw_decode_policy`). `default_ref` paths use the `product.toml` spelling. Both spellings denote the same decision; the validator's D1 check joins them, and a `default_ref` pointing at a nonexistent snake_case path is an error. **Single-definition rules (D-rules):**

- **D1**: `default` (literal) xor `default_ref` (a `product.toml` key path, e.g. `defaults.theme`) — product-level defaults are stated *once*, in `product.toml`, and referenced by the schema. The validator fails on a literal default that duplicates a `product.toml` value it should reference `[PROPOSAL heuristic: same file+area]`, and on `default_ref` targets that don't exist.
- **D2**: `key` values globally unique.
- **D3**: `cli.flag` values globally unique, and disjoint from the fixed short-flag table and the top-30 subcommands (doc 05 §5.3).
- **D4**: `description` msgids are unique 1:1 with entries (one entry per msgid, no shared prose keys across settings).
- **D5**: `placement` orders are unique within a `(screen, section)`; `screen` must exist in `layouts/settings-areas.toml`; entries with `ui = false` must set `advanced` or a documented reason comment.
- **D6**: no two entries may write the same engine runtime slot (validator cross-checks the `owner` mapping in the generated table) `[PROPOSAL]`.

### 5.3 One schema, four generators (`tools/schema-gen`)

`cargo xtask schema-gen` reads `schemas/settings.toml` and emits (all committed; `--check` mode diffs for CI):

| # | Target | Artifact | How it is consumed |
|---|---|---|---|
| G1 | **Settings screens** | `ui/src/generated/settings-schema.ts` — typed entries grouped by screen/section/order, with msgids and control type hints (toggle/slider/select/input/list) | React screens render navigation from `layouts/settings-areas.toml` and controls from G1 — **no hand-written settings component carries its own option list**; the settings UI is a schema renderer. Control selection: bool→toggle, enum→segmented/dropdown, range numeric→slider+input, list<T>→tag editor, path→path picker, duration-ms→duration input. |
| G2 | **CLI flags + `--help`** | static tables compiled into `apps/cli` (clap-adjacent `[VERIFY]` crate choice) — long flags `--<area>-<key>`, help text from the schema description strings (English, dev-facing; localized help is a v2 question), plus `--generate-manifest` dumping the full option table as TOML/JSON for scripts | `lumen --help` lists areas with counts, `lumen --help playback` lists that area's flags — VLC's `--longhelp` pattern, generated. Short flags come from the fixed reserved table (doc 05 §5.3). |
| G3 | **Documentation** | `docs/settings-reference.md` (per-area tables: key, type, default, range, since, restart) and `docs/cli-reference.md` | generated at build; PR diff shows doc drift for free |
| G4 | **Runtime validation** | the schema itself, embedded via `include_str!` into `crates/settings-schema`; `config.set-setting` validates every write (type/range/enum) before applying — `invalid-argument` on violation, never silent coercion (doc 05 §2.11) | also backs `config.get-setting`'s `schema_meta` result field |

Idempotence: `schema-gen` output is a pure function of its input; running twice is byte-identical (§10.3).

### 5.4 Mapping entries to the design-package screens

`layouts/settings-areas.toml` declares the screen navigation (id, `label_key`, icon token, order) — the design package's Settings group is the source of the *list* (design/README.md): `global, interface, privacy, playback, video, audio, subtitles, controls, library, network, performance, recording, extensions`.

> **Count note.** design/README.md says "14 settings areas" but its inventory names **13** distinct screens (global, interface, privacy, playback, video, audio, subtitles, controls, library, network, performance, recording, extensions). Either the count or the list needs correction from the Figma file. **[VERIFY] — doc 08 (1.B8) owns the screen-by-screen analysis; until then this document uses the 13 named screens.** Because screens are config (the areas file), adding a 14th is a config change plus schema placements, not code.

Initial seed of entries per screen (Phase 1–2 scope; every entry follows §5.2; abbreviated here to key / type / default / gate):

| Screen | Entries |
|---|---|
| global | `global.language` (enum: locales listing, default_ref `defaults.locale`) · `global.start-action` (enum welcome/open-file/none, none) · `global.update-checks` (bool, default_ref `telemetry.update_checks`) |
| interface | `interface.theme` (enum: themes listing, default_ref `defaults.theme`) · `interface.show-tooltips` (bool, true) · `interface.ui-scale` (enum 100/110/125 %, 100) |
| privacy | `privacy.telemetry` (bool, default_ref `telemetry.enabled`) · `privacy.crash-reports` (enum off/ask/send, off) · `privacy.usage-stats` (bool, false) |
| playback | `playback.preferred-audio-languages` (list, default_ref) · `playback.preferred-subtitle-languages` (list, default_ref) · `playback.subtitle-mode` (enum, auto) · `playback.hw-decode-policy` (enum, software-only; gate `hw-decode`, pending ADR-001) · `playback.seek-step-fine` (float, 3 s) · `playback.seek-step-short` (float, 10 s) · `playback.seek-step-coarse` (float, 60 s) · `playback.autoplay-next` (bool, true) · `playback.pause-on-minimize` (bool, false) |
| video | `video.deinterlace` (enum auto/on/off; gate) · `video.default-aspect-ratio` (enum) · `video.default-crop` (enum) · `video.frame-drop-policy` (enum auto/never; advanced) |
| audio | `audio.output-device` (enum dynamic, "default") · `audio.volume-default` (percent, 100) · `audio.volume-ceiling` (percent 100–125, 125) · `audio.normalize` (bool, false; gate) · `audio.visualizer` (enum none/bars/wave; gate) |
| subtitles | `subtitle.font-size` (percent 50–200, 100) · `subtitle.color` (enum: theme token palette) · `subtitle.outline-strength` (enum) · `subtitle.position` (enum bottom/top) · `subtitle.sync-step-ms` (int, 100) · `subtitle.fallback-encoding` (enum, UTF-8) |
| controls | `controls.keymap` (enum: keymaps listing, default) · `controls.mouse-wheel-action` (enum seek/volume/none, seek) · `controls.osd-duration-ms` (duration-ms, 1500) |
| library | `library.watch-folders` (list path; gate `library`) · `library.scan-on-start` (bool; gate `library`) |
| network | `network.timeout-ms` (duration-ms, 15000; gate `streaming-http`) · `network.user-agent` (string; gate) · `network.proxy-mode` (enum none/system/manual; gate) · `network.reconnect-attempts` (int 0–10, 3; gate) |
| performance | `performance.decoder-threads` (enum auto/1..8, auto) · `performance.cache-override-mib` (int, 0 = use product default; default_ref to `limits.cache_demux_mib` when nonzero) |
| recording | `recording.dir` (path; gate `recording`) · `recording.format` (enum mkv/mp4; gate) · `recording.include-subtitles` (bool; gate) |
| extensions | `extensions.auto-update` (bool, false; gate `wasm-extensions`) · `extensions.default-permissions` (list enum; gate) |

Engine-internal entries (`ui = false`, CLI + docs + validation only, never rendered): `engine.log-level` (enum), `engine.tick-interval-ms` (doc 05 §3.2 default 250), `engine.max-event-rate` — the escape valve that keeps VLC-style hidden power options honest: they exist in the *same* schema, so they cannot duplicate anything (D-rules) and still appear in `--help` and docs. The **seed total is ≈ 45 entries**; screens whose features are not yet exposed (library, network, recording, extensions) carry gated entries that render as documented-but-absent until their phase ships — consistent with MP §5: exposure lists only implementations, and the gate field keeps the promise mechanical.

### 5.5 Honest option-count budget (mirroring VLC's thousands)

VLC 3.0 exposes on the order of **2 000+** preference/CLI options `[VERIFY — exact count is a Phase 0 inventory output, doc 02 O2]`, because every module (each codec, filter, access) carries tuning knobs. Our model differs structurally: **an option exists only when its feature exists** (rule F2), and per-codec tuning stays in per-format records/engine code unless a product decision needs it (§1). Budget `[PROPOSAL]`: Phase 1 ≈ 45 (the seed above), Phase 4 ≈ 120, v1.0 (post-Phase 7) ≈ 250–350, of which ~15 % engine-internal. We do not chase VLC's count; we mirror its *capability* (every implemented behavior tunable) with a fraction of the surface, and the parity report (doc 02 §4) says so.

---

## 6. Localization system

### 6.1 Key grammar

Every user-visible string is addressed by a **stable dotted key**: segments of `[a-z0-9-]+`, two to five segments, dotted — `player.controls.play`. Keys are *API*: renaming one is a breaking change for every catalog, so keys name the concept (`player.controls.play`), never the wording (`player.controls.play-button-green`). Reserved top-level namespaces:

| Namespace | Contents |
|---|---|
| `product.*` | identity-driven strings rendered from config (description, title templates) |
| `window.*` | window titles (idle title) |
| `player.*` | playback surface: controls, tooltips, states |
| `settings.*` | area titles, section titles, setting labels + descriptions (1:1 with schema msgids, rule D4) |
| `menu.*` | menu labels (doc 05 §8.3) |
| `dialog.*` | dialogs: open-URL, keyboard shortcuts, onboarding |
| `error.*` | user-facing error rendering (per error code, doc 05 §2.13) |
| `common.*` | shared vocabulary (OK/Cancel/Reset…) — the only namespace where sharing is allowed |

### 6.2 Message format

Parameterized messages use an ICU-MessageFormat subset `[PROPOSAL — exact feature set and the pure-Rust implementation (e.g. the Fluent family) is a Phase 0 dependency-audit item, MP §9]`:

```toml
# locales/en-US.toml (flat dotted keys; TOML prohibits duplicate keys by
# definition, so duplicate detection is parse-level for free)
"window.title.idle"            = "{product_name}"
"player.controls.play"         = "Play"
"player.controls.pause"        = "Pause"
"player.controls.next"         = "Next"
"player.controls.prev"         = "Previous"
"player.controls.volume"       = "Volume"
"player.controls.mute"         = "Mute"
"player.controls.fullscreen"   = "Fullscreen"
"player.status.buffering"      = "Buffering… {percent, number, percent}"
"player.status.opening"        = "Opening {source}"
"player.status.ended"          = "Ended"
"player.error.unsupported"     = "This media can't be played: {format, select, known {no {format_name} decoder is available} unknown {the format isn't recognized}}"
"settings.area.playback.title" = "Playback"
"settings.area.global.title"   = "Global"
"settings.area.interface.title"= "Interface"
"settings.area.privacy.title"  = "Privacy"
"settings.playback.subtitle-mode.label" = "Subtitle mode"
"common.reset"                 = "Reset"
"menu.media.open-file"         = "Open File…"
"product.description"          = "A clean-room, Rust-native media player."
```

v1 subset: `{name}` simple placeholders; `{name, number, percent}`; `{name, select, …}`; `{name, plural, …}` for counts (e.g. playlist items). Placeholder names are part of each key's contract.

### 6.3 Catalog layout and fallback chain

- `locales/en-US.toml` is the **reference catalog**: shipped first, complete, and the source of truth for the key set. It is a CI error for the reference catalog itself to miss a key referenced anywhere (schema descriptions D4, layout `label_key`s, menu entries, Rust/TS lookups).
- Other catalogs are added per language tag (`de.toml` → `de`, `pt-BR.toml` → `pt-BR`; file name = BCP-47 tag).
- **Fallback chain** (MP §5 "explicit fallback"): requested locale → its language-primary parent (`pt-BR` → `pt`, when such a catalog exists) → `defaults.locale` → `en-US` → (debug builds only) the key itself, logged as `missing:<key>`. The chain is total — a missing translation can never crash rendering — but CI (§6.4) makes the final two links unreachable in shipped builds.
- Locale selection: `global.language` setting → OS locale → `defaults.locale`.

### 6.4 Compile-time tests (CI)

Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`, doc 01 §2.3):

1. **Missing-key detection**: every catalog ⊇ reference key set (missing keys are hard errors in CI; a coverage percentage is also printed for maintainers `[PROPOSAL]`).
2. **Duplicate-key detection**: TOML parse error on duplicates inside a file; cross-file duplicates of *values* are not errors (translations may coincide) — but a catalog key absent from the reference is an error (orphan key).
3. **Placeholder validation**: for each key, the placeholder set (names + types) must equal the reference catalog's exactly — compile-time, before any translation renders. `{percent, number, percent}` in en-US and `{percent}` in de is a CI failure.
4. **Reference-completeness**: every `label_key` / `description` msgid referenced from `product.toml`, `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`, `themes/*` and doc 05's menu/UI-action tables resolves to a reference-catalog key (cross-file join; also run by config-validator §3.4).
5. Plural-rule conformance per locale is checked by the message-format engine at load `[VERIFY — depends on §6.2 crate choice]`.

### 6.5 "No literal UI text in Rust or TSX" — the lint strategy

MP §5: "No literal UI text in Rust or TSX. Technical identifiers are not product copy." Enforcement is mechanical, in two linters plus an allowlist:

**Rust — `cargo xtask lint-literals --rust`.** Scans string literals in `crates/` and `apps/`:
- *Deny by construction*: the localization API accepts only `MessageKey` values (newtype over validated keys); there is no `translate(&str)` overload, so a raw literal cannot reach a user.
- *Heuristic sweep*: any string literal containing a space and an uppercase letter, or ≥ 70 % similar to a reference-catalog value (normalized edit distance `[PROPOSAL]`), is flagged — this catches copy smuggled into logs returned to UIs, panic-ish messages, or `format!` strings that end up rendered.
- *Exemptions by category* (allowlist file `tools/xtask/data/literal-allowlist.toml`, each entry carries a reason and an owner): `tracing::` log calls (developer-facing, English-only by policy — user-facing copy never comes from logs), test fixtures, path constants, error-code names, technical identifiers (`"H.264"`, `"MediaNext"`, command IDs).
- Inline escape hatch: `// l10n:allow technical-identifier` with a mandatory reason token; audited quarterly `[PROPOSAL]`.

**TypeScript/TSX — ESLint.** A `no-literal-strings`-style rule (`[VERIFY]` — pick an existing community rule or write a small custom one in Phase 1) bans:
- JSX text nodes (`<button>Play</button>` → fail);
- string props on presentation components except an allowlisted set (`className`, `data-testid`, icon names, component-internal technical props);
- `aria-label`/`alt` with literals (they are user-visible: must use the `useMsg()` hook from `@lumen/i18n`).
- Exemptions: `tests/`, `*.stories.*`, generated files under `ui/src/generated/` (machine-written; G1 emits msgids, not prose), and a comment escape `// l10n:allow` with reason, same audit as Rust.

**Positive/negative fixtures.** Both linters ship fixture suites (`tests/lint-fixtures/literals/{positive,negative}/…`) — positive = must be flagged (a literal "Play" in TSX, a smuggled sentence in Rust), negative = must pass (technical identifiers, allowlisted entries, log lines). The linters' own tests assert exact expected violation sets; CI runs the linters over the real tree with zero tolerance (§10.5). This makes the lint *itself* tested — a lint that can be silently broken is not a gate (AGENTS §3.5 ethos).

---

## 7. Declarative UI structure — `layouts/`, `keymaps/`, `profiles/`, `themes/`

MP §5: "Declarative UI structure: which panels, menus, toolbars and buttons exist, and their order, come from layout config. Pixel detail comes from design tokens (section 7)." MP §7: "Commands and keymaps are data; menu items map to command IDs."

### 7.1 What layouts decide — and what they never decide

`layouts/` decides **existence and order** of UI structure: which panels/menus/toolbars/buttons exist, their labels (msgids), and which command or UI action each invokes. `layouts/` never decides pixels: no coordinates, sizes, colors or typography — those are design tokens consumed by components (doc 08 owns that pipeline), and themes reference tokens only (§7.5). A layout change re-ships the product's UI shape without touching React code; a token change re-skins it without touching layouts.

### 7.2 `layouts/` files

| File | Declares |
|---|---|
| `layouts/main.toml` | the player window's regions: playback surface, controls bar, title bar, playlist sidebar, miniplayer; each region's control composition and order |
| `layouts/menus.toml` | menu trees per doc 05 §8.3 (leaf = `{label_key, command \| ui_action, args?, accelerator_binding?}`; dynamic submenus `{dynamic: tracks.audio}` populated from engine data) |
| `layouts/ui-actions.toml` | the UI-action registry per doc 05 §8.4 (dialogs, panel toggles, focus management) |
| `layouts/settings-areas.toml` | settings screen navigation (id, `label_key`, icon token, order) — §5.4 |
| `layouts/dialogs.toml` | dialog inventory: open-network-stream, keyboard-shortcuts, onboarding steps `[PROPOSAL]` |

Sketch (`schema = "layout.v1"`, validated by `schemas/layout.schema.json`):

```toml
[[region]]
id = "controls-bar"
visible_when = "has-media"            # condition tokens as in keymaps (§7.3)

[[region.control]]
id = "play-pause"
kind = "button"                        # rendered by a token-driven component
label_key = "player.controls.play"     # msgid; states may swap keys (pause)
command = "playback.toggle-play-pause" # from doc 05 §2 — validated to exist
icon = "lumen:play"                    # icon-set token [PROPOSAL]
order = 10

[[region.control]]
id = "volume"
kind = "slider"
command = "audio.set-volume"
order = 40
args = { percent = true }
```

Validator rules: every `command` must exist in the command catalog (cross-check against the generated catalog tables, doc 05 §2.1); every `label_key` must resolve (§6.4 rule 4); `order` unique per region; controls whose `feature_gate` is unexposed render disabled *from data* (doc 05 §8.3).

### 7.3 `keymaps/` — binding-id indirection

Format per doc 05 §8.1 (authoritative there; summarized for completeness): `keymaps/default.toml` seeds bindings `{id, key, command|recipe, args, mode, when}`; **menus reference binding ids, never raw keys**, so one rebinding updates every surface. The keymap participates in the §4 layering (user override layer `keymaps/user.toml` in the user config dir `[PROPOSAL]`). **Cross-doc note:** doc 05 §8.1 places the seed at `config/keymaps/default.toml`; MP §4 lists `keymaps/` as a top-level directory. This document assumes **`keymaps/default.toml`** (MP layout wins); flagging the mismatch for the Lead to reconcile in doc 05 (§11 Q4). Validator (doc 05 §8.1 + §3.4): unknown command IDs, args failing the command's param schema, duplicate `key`+`mode`, contradictory conditions — all hard errors.

### 7.4 `profiles/` — preset bundles

A profile is a named, layer-2 bundle of setting overrides (VLC's preset-bundle analog — the design package ships `Profiles.svg` and `Setting Preset.svg` screens):

```toml
# profiles/default.toml
schema = "profile.v1"
name_key = "profile.default.name"
[settings]
"video.deinterlace" = "auto"
"audio.visualizer" = "none"
```

Rules: profile files may only reference keys that exist in the settings schema (D-rules) and only values that pass it; the active profile is `defaults.profile` (product) overridable by the user setting `global.profile` `[PROPOSAL]`; switching profiles re-resolves layers 1–3 and emits the usual `config.setting-changed` events. Seed profiles: `default` (mirrors product defaults) and, when their features ship, `music` and `cinematic` `[PROPOSAL]`.

### 7.5 `themes/` — token references only

```toml
# themes/dark.toml
schema = "theme.v1"
name_key = "theme.dark.name"
[palette]                    # values are token references — never raw hex
surface = "color.surface.default"
accent  = "color.accent.primary"   # must equal product.toml brand.accent_color (§3.3 cross-check)
[typography]
body = "type.body"
```

Themes bind *semantic* slots to *token* names resolved by `crates/theme-tokens` (doc 01 §2.3); the token definitions themselves come from the Figma-variables pipeline (doc 08). Seed: `dark` (design default) and `light` (`Player - Light.svg`). The validator fails a theme that contains a raw color literal `[PROPOSAL]` — the same no-hardcoding principle, applied to pixels.

---

## 8. Rebrand system

### 8.1 Two rebrand planes

| Plane | Mechanism | Examples |
|---|---|---|
| **Build-time** — `cargo xtask rebrand` | deterministic rewrites of the source/packaging tree | crate names (`lumen-*` → `nova-*`), binary name, `apps/` package identifiers, Tauri `productName`/`identifier`, installer metadata (MSI, DMG, AppStream), desktop entry `Name=`, macOS `CFBundleName`/bundle id, icon file wiring, generated docs |
| **Runtime** — config-driven | every surface reads `product-config` at run time | window title (template + `identity.name`), About page, CLI `--version`/banner, log line prefix, control-server `engineInfo`, PWA manifest name, onboarding copy via `product.*` keys |

The split is the §1 boundary again: *packaging identity is a build fact* (an installer is an artifact), while *everything a running process shows is data*. `xtask rebrand` consumes a target `product.toml` (or a delta file `[PROPOSAL]`), performs the rewrites, and regenerates all generated artifacts (§5.3, §8.2) so the two planes stay consistent.

### 8.2 Propagation matrix (config field → every surface)

| `product.toml` field | Window title | Installer/packaging | About page | CLI identity | Logs | Generated docs | Control server |
|---|---|---|---|---|---|---|---|
| `identity.name` | idle title, `{product_name}` in template | product name, desktop `Name=`, plist | heading | `--version` banner, usage header | line prefix | headers, titles | `engineInfo.name` |
| `identity.short_name` | — | installer dir name, "opens with" | — | compact usage lines | — | — | — |
| `identity.publisher` | — | copyright, maintainer fields | copyright line | — | — | credits | — |
| `identity.ids.*` | — | AUMID, UpgradeCode, bundle id, AppStream id | — | — | startup log (ids) | packaging docs | — |
| `identity.display_version` / `version_code` | — | package version | version line | `--version` | startup log | docs | `engineInfo.engine_version` |
| `brand.accent_color` | UI accent (via theme token cross-check) | installer accent art | accents | — | — | styled docs | — |
| `brand.logo_svg` / `icon_*` | taskbar/window icon | installer icons | logo | — | — | embedded logo | — |
| `window.title_template` | live | — | — | — | — | — | — |
| `urls.*` | — | — | links | `--help` footer | — | links | — |
| `defaults.theme/locale` | initial skin/language | — | — | initial locale for localized output | — | — | — |

### 8.3 Acceptance test — the rebrand proof (MP §5, CI gate per MP §13)

`tests/regression/rebrand/` ships a fixture product definition ("Nova Player", accent `#ff5500`, distinct logo files). The test `[PROPOSAL — shape; full-build tier is a release/nightly job, see tiering below]`:

1. Copy the tree (or use a git worktree); run `cargo xtask rebrand --config tests/regression/rebrand/nova.toml`.
2. Assert generated packaging: Tauri config, desktop entry, macOS plist, MSI metadata all carry Nova ids and *no* Lumen identity outside the allowlist (`tools/xtask/data/rebrand-allowlist.toml` — e.g. clean-room scanner wordlists, upstream URLs).
3. Build (tier 2) and boot the CLI headlessly; assert `--version` prints `Nova Player …`; assert a log line prefix carries `nova`.
4. Unit-assert runtime surfaces with the fixture config loaded: window title resolves to `"<title> — Nova Player"` with a fake `{media_title}`; the About model renders Nova + logo path; `engineInfo.name` (mock) is Nova.
5. Assert generated docs (`docs/settings-reference.md`, `docs/cli-reference.md`) contain Nova headers and zero "Lumen" identity hits outside the allowlist.
6. **Reverse assertion**: in the *un-rebranded* tree, `cargo xtask check-identity` proves no identity literal is hardcoded outside `product.toml` + generated files + allowlist (§9) — the reason rebrand *can* work.

Tiering `[PROPOSAL]`: steps 1–2, 4–6 run every PR (fast, no full build); step 3 (full rebrand build of the rewritten tree) runs in the nightly/release job — MP §13's "config propagation test" gate covers the fast tier on every merge.

---

## 9. CI enforcement — the concrete failing conditions (MP §5)

MP §5's last bullet names five CI-failing conditions; each maps to a check, a tool, and a fixture:

| # | Failing condition | Check | What exactly fails CI |
|---|---|---|---|
| C1 | **Hardcoded user-facing strings** | `cargo xtask lint-literals` (Rust §6.5) + ESLint `no-literal-strings` (TSX §6.5) | any flagged literal in `crates/`, `apps/`, `ui/src` outside exemptions; linter fixture suites must themselves pass their expected-violation tests |
| C2 | **Duplicated identity values** | `cargo xtask check-identity` | identity literals (`name`, `short_name`, ids, accent hex) appearing outside `product.toml`, generated artifacts and the audited allowlist; generated artifacts differing from regeneration (`schema-gen --check`, packaging regen check) |
| C3 | **Missing translation keys** | `cargo xtask l10n-audit` (§6.4) | any catalog missing a reference key; orphan keys; placeholder-set mismatch; unresolved `label_key`/msgid references from any config file |
| C4 | **Schema violations** | `cargo xtask config-validator` (§3.4) | any shipped config failing its schema or semantic pass: `product.toml`, settings schema D-rules, profiles, keymaps, layouts, themes, locales, plugin manifests (doc 06), feature-exposure/registry intersection (F1/F2), feature-matrix agreement |
| C5 | **Broken crate boundaries** | `cargo xtask check-architecture` — owned by **doc 01 §5** (boundaries manifest, cycle/global-state/unsafe scans, single-command registration) | per doc 01; listed here because MP §5 names it as a config-system sibling gate — config crates (`product-config`, `settings-schema`, `localization`, `theme-tokens`) must stay L1-isolated |

**Job sequence** (each step gates the next; the whole chain is merge-blocking on Windows/Linux/macOS per MP §13):

```
1  cargo fmt --all -- --check
2  cargo clippy --workspace --all-targets --all-features -- -D warnings
3  cargo xtask check-architecture          (doc 01 §5)
4  cargo xtask config-validator            (C4 — schema + semantics + D-rules)
5  cargo xtask schema-gen --check          (generated files match source)
6  cargo xtask l10n-audit                  (C3)
7  cargo xtask lint-literals               (C1, Rust + TSX; includes UI lint/typecheck)
8  cargo xtask check-identity              (C2, fast rebrand tier §8.3)
9  cargo test --workspace --all-features   (unit + precedence matrix T1–T16)
10 contract suite (doc 05 §7, four hosts)  + rebrand acceptance (fast tier)
11 packaging + launch smoke (release job; full rebrand build here)
```

Steps 4–8 are the config-driven system's own teeth; they run before any compile-heavy work so config violations fail fast.

---

## 10. Test plan — per component

| Component | Tests |
|---|---|
| **10.1 `config-validator`** | One unit test per schema rule (§3.3): each range bound (below/above), each enum violation, reserved names, identifier consistency, template-variable check. Semantic passes: path existence (present/missing/wrong magic bytes), injected enums (theme/locale/profile present and absent), cross-field rules, F1 (unknown feature), F2 (exposed-but-unimplemented), telemetry-requires-privacy-URL. **Golden diagnostics**: every rule's failure message snapshot-tested against the §3.5 format (key path + rule + hint present). Property tests: no valid config ever produces output; every diagnostic parses back to a structured error. |
| **10.2 `product-config` merge** | The full precedence matrix T1–T16 (§4.5) as named tests; array policies (replace, append-unique); lock semantics incl. T8/T9; determinism (same layers → byte-identical resolved snapshot, 100 iterations `[PROPOSAL]`); unknown-key rejection with did-you-mean (fixture typos); config-error surface behavior for user-layer violations; migration dry-runs (schema v1→v2 fixture) once migrations exist. |
| **10.3 `schema-gen` codegen** | **Idempotence**: generate twice → byte-identical (all four targets G1–G4). **Golden files** per target, updated only by intentional schema changes. Drift detection: `--check` on a mutated generated file fails. Coverage: every type in §5.2 renders in TS and CLI tables; enum/range/list/path/duration each have a golden. TS output typechecks (`tsc --noEmit`) in the UI job. D-rule violation fixtures: duplicate key, duplicate flag, duplicate msgid, clashing placement order, `default`+`default_ref` both set — all rejected. |
| **10.4 Rebrand propagation** | §8.3 acceptance script: fixture rebrand → surface assertions (packaging, `--version`, title resolution, About, logs, docs) + reverse no-hardcode assertion; allowlist integrity (every allowlist entry has reason + owner). Full-build tier in nightly/release. Regression hook: any future identity-consuming surface must add a row to §8.2 *and* an assertion in the script (doc-review checklist `[PROPOSAL]`). |
| **10.5 Lint effectiveness** | Positive fixtures (must flag): TSX text node, literal `aria-label`, literal in Rust rendered path, catalog-value near-duplicate in Rust. Negative fixtures (must pass): technical identifiers, log lines, test fixtures, allowlisted entries with reasons, generated files. Each linter's test asserts the exact violation set over both fixture suites — a lint that over- or under-fires is a test failure (AGENTS §3.5). |
| **10.6 `localization`** | Fallback chain (each hop, incl. absent parent locale); missing-key runtime behavior (debug marker, no panic); placeholder rendering (counts, percents, select); audit outputs (missing/duplicate/orphan/placeholder-mismatch fixtures); locale-tag validation; plural samples per supported CLDR rule `[VERIFY]`. |
| **10.7 `settings` runtime path** | `config.get/set/reset-setting` against the embedded schema: type/range/enum acceptance and rejection (`invalid-argument`, never coercion); `restart_required` reporting; gated entries hidden + inert when feature off; `config.list-areas` matches `layouts/settings-areas.toml`; session-scope routing (T6/T7). Runs in the doc 05 §7 conformance suite on all four hosts. |

**Evidence discipline** (MP §13): every test above is a named, automated artifact; nothing in this document counts as verified until these exist and pass — statuses per MP §17.

---

## 11. Open questions for the Lead (per AGENTS §3.6)

1. **Q1 — Admin mechanism depth (§4.3)**: registry/GPO + MDM + `/etc` file as proposed, or ship v1 with *file-based admin override only* on all three OSes (simpler, still testable) and add native policy plumbing later?
2. **Q2 — Session-scope key set (§4.1)**: which settings are session-scoped in v1? Candidate: only `playback.rate`. This changes `config.set-setting` semantics (doc 05 §2.11) — needs a catalog-level decision.
3. **Q3 — Settings-screen count (§5.4)**: design/README.md says 14 areas but names 13 — resolve via doc 08 before freezing `layouts/settings-areas.toml`.
4. **Q4 — keymap path reconciliation (§7.3)**: `keymaps/default.toml` (this doc, per MP §4) vs `config/keymaps/default.toml` (doc 05 §8.1) — pick one and patch the other document.
5. **Q5 — Message-format engine (§6.2)**: approve the ICU-subset scope and shortlist pure-Rust implementations (Fluent family or equivalent) for the Phase 0 dependency audit; confirms placeholder/validate tooling.
6. **Q6 — Option-count budget (§5.5)**: accept ≈ 250–350 at v1.0 as the honest target (vs VLC's ~2 000 `[VERIFY]`), recorded in the parity report?
7. **Q7 — Rebrand tiers (§8.3)**: fast tier per-PR + full-build nightly, as proposed?
8. **Q8 — `config/` directory role (MP §4)**: this document assigns `config/` = shipped runtime-config templates + admin example; `keymaps/ layouts/ profiles/ themes/ locales/ schemas/` are top-level. Confirm, or re-scope.
9. **Q9 — doc 06 reconciliation**: plugin manifest `config_schema` and enable/reorder semantics are MP §6/doc 06 territory; §4.4's plugin-list array policy and the manifest schema slot in §3.4 stage 4 must be reconciled when doc 06 lands.

## Unfinished

None — all ten assigned areas (§1–§10) are covered. Cross-doc follow-ups: doc 08 (1.B8) owns token extraction (feeds §2.3 accent [VERIFY] and §7.5); doc 06 (1.B1) owns plugin manifest schema (§3.4 stage 4, §11 Q9); doc 10 owns the CI workflow files themselves; Phase 0 owns ADR-001 (hw-decode enum, §2.8) and the VLC option-count verification (§5.5).


