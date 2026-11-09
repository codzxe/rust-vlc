# 06 — Plugin Architecture

Status: planning (Wave 1, Task 1.B6). Owner: engine/plugin track; contract
owner for the plugin API is the **Lead** (doc 01 §7: `plugin-api` is a
Lead-owned shared interface).

Scope: the complete plugin system specification — taxonomy, manifests,
registry and selection, built-in registration, the sandboxed WASM extension
host, forbidden-list enforcement, the mandatory conformance kit, the v1 scope
fence, and the test plan. Primary source: MASTER-PROMPT (MP) **§6**; also §5
(config-driven, enable/disable/reorder, never invent), §9 (Rust-only policy,
no hidden native code), §12 (security), §13 (conformance + fuzzing), §14
(known limits), §16 (phases). Downstream consumers: docs/07 (config-driven
system, Wave B sibling), docs/10 (testing/CI), and the Phase 1 implementation
of `crates/plugin-api` and `crates/plugin-host-wasm` (doc 01 §2).

Conventions: inventions are marked `[PROPOSAL]`; facts not verified in this
wave are marked `[VERIFY]` or `[VERIFY-P0]` (Phase 0 must close them). Trait
names follow doc 01 §2; where this document introduces a trait or splits a
doc-01 kind, the delta is flagged for Lead reconciliation (§10 Q1–Q3).

---

## 1. Plugin taxonomy and kinds

### 1.1 What is a plugin

MP §6: *"Anything with two or more implementations, or a real third-party
need, is a plugin."* A plugin is therefore a **unit of replaceable media
capability** described by a manifest, discovered through a registry, and
selected by capability and priority. In v1 a plugin exists in exactly two
origins (MP §2):

- **builtin** — Rust crates compiled into the engine, registered at startup
  behind stable traits, gated by Cargo features (§4);
- **extension** — a sandboxed WASM module installed on disk, hosted by
  `crates/plugin-host-wasm` (§5).

There is no third origin: dynamic loading of native `.so`/`.dll` plugins,
C/C++ plugins, and automatic execution of downloaded plugins are **forbidden**
(MP §6, enforced per §6 of this doc).

### 1.2 The sixteen kinds

Doc 01 §2 defines a 14-variant `PluginKind` enum. This specification splits
two of them — `SubtitleFormat` into *subtitle-parser* + *subtitle-renderer*,
and `PlaylistFormat` into *playlist-parser* + *playlist-exporter* — because
parsing and rendering/exporting have different traits, lifecycles, and
sandbox postures. Result: **16 kinds**. The merge/split is a `[PROPOSAL]`
requiring Lead reconciliation of `plugin_api::PluginKind` (§10 Q1).

| # | Kind | Stable trait (owner crate, doc 01 layer) | Typical origin in v1 | Pipeline position |
|---|------|------------------------------------------|----------------------|-------------------|
| 1 | `access` | `plugin_api::Access` → `SourceStream` (`plugin-api`, L1; impls in `input`, L2) | builtin | Byte source: file, disc image (ADR-008). First stage; feeds probe + demux |
| 2 | `network-protocol` | `plugin_api::Access` (`plugin-api`, L1; impls in `streaming`, L2) | builtin | Stream source: HTTP(S), HLS, DASH, RTSP. First stage, with buffering/reconnect policy |
| 3 | `demuxer` | `plugin_api::Demuxer` (`plugin-api`, L1; impls in `demux`, L2) | builtin | Container parse → timestamped `Packet`s, seek tables |
| 4 | `decoder` | `codec_api::Decoder` (`codec-api`, L1; impls in `codecs`, L3; hw via `codec_api::HwAccelerator` in `platform-*`, L5) | builtin (+ OS hw per ADR-001) | Packet → `Frame`; one instance per track |
| 5 | `encoder` | `codec_api::Encoder` (`codec-api`, L1; impls in `codecs`, L3) | builtin (Phase 7) | Frame → packet; transcode jobs only |
| 6 | `audio-filter` | `audio_engine::AudioFilter` (`audio-engine`, L3) | builtin + WASM ("light filters", MP §6) | Post-decode audio chain → mixer → sink |
| 7 | `video-filter` | `video_engine::VideoFilter` (`video-engine`, L3) | builtin + WASM ("light") | Post-decode video chain → presentation scheduler |
| 8 | `audio-output` | `audio_output::AudioBackend` : `audio_engine::AudioSink` (`audio-output`, L4; OS impls `platform-*`, L5) | builtin only in v1 (§5.9) | Final audio sink; supplies the master clock position |
| 9 | `video-output` | `video_engine::Presenter` (`video-engine`, L3; impl `renderer-wgpu::Renderer`, L4) | builtin only in v1 | Presentation: `PresentAction` → surface |
| 10 | `subtitle-parser` | `plugin_api::SubtitleParser` (`plugin-api`, L1; impls in `subtitles`, L2) | builtin | Cue/text extraction → timed `SubtitleCue`s |
| 11 | `subtitle-renderer` | `SubtitleRenderer` **[PROPOSAL: placement]** (default impl in `renderer-wgpu`, L4) | builtin | Cue + style → overlay draw list, composited at presentation |
| 12 | `playlist-parser` | `playlists::PlaylistParser` (`playlists`, L2) | builtin + WASM (MP §6) | URI/drop → `Playlist` entries |
| 13 | `playlist-exporter` | `playlists::PlaylistExporter` **[PROPOSAL]** (`playlists`, L2) | builtin + WASM | `Playlist` → file (`playlist.save`, doc 05 §2.7) |
| 14 | `metadata-provider` | `plugin_api::MetadataProvider` (`plugin-api`, L1; builtin home `media-probe`, L2) | builtin + WASM (MP §6) | Tag/cover art extraction at probe; on-demand lookups |
| 15 | `ui-panel` | `plugin_api::UiPanel` host contract **[PROPOSAL]**; guest-side protocol in `plugin-host-wasm`, L6 | **extension only** | UI surface; talks only to the command layer (doc 01 §2.8) |
| 16 | `service-discovery` | `device_discovery::ServiceDiscovery` (`device-discovery`, L2) | builtin + WASM | Background enumeration → `Device` lists for UI browsing |

Notes on trait ownership: doc 01 §2.3 already flags cross-cutting trait
placement (`SourceStream`, `Access`, `Demuxer`, `SubtitleParser`,
`MetadataProvider` in `plugin-api`) as its §8 Q3 `[PROPOSAL]`; this document
adopts that placement. Domain-owned traits (`AudioFilter`, `VideoFilter`,
`AudioBackend`, `Presenter`, `PlaylistParser`, `ServiceDiscovery`) stay in
their domain crates — `plugin-api` names them via the manifest's `kind`, not
by importing them (keeps L1 free of L2/L3/L4 deps, doc 01 ARCH rules).

**Implemented placement as built (recorded 2026-10-22, card 3.DOC-1):** card
1.D2 staged the host traits for kinds 6, 7, 12, 13 and 16 — `AudioFilter`,
`VideoFilter` (+ `FilterOutcome`/`FrameSource`), `PlaylistParser`,
`PlaylistExporter` (+ `Playlist`/`PlaylistEntry`), `ServiceDiscovery` (+
`Device`/`DiscoveryChange`) — **in `crates/plugin-api`** as recorded
`[PROPOSAL]` deviations from the table above (worklog 1.D2): the domain
crates did not exist when the contract landed, and L1 cannot depend on
L2/L3. As of Phase 2 the domain crates define their own pipeline traits
(`audio_engine::AudioSink`, `video_engine::Presenter`) and do not yet
re-export the plugin-api filter/playlist/discovery traits — the filter-chain
card (doc 03 §4, later wave) owns the reconciliation. The 16-variant
`PluginKind` (`crates/plugin-api/src/kind.rs`) mirrors this section's table
exactly; doc 01 §2.3 was synced to it by 3.DOC-1 (D-004's promised edit,
done).

### 1.3 Lifecycle

All kinds share one state vocabulary (reported via the doc 05 event trio
`plugins.loaded` / `plugins.failed` / `plugins.crashed`):

```
Registered ──select──▶ Instantiated ──▶ Active ──▶ (Failed | Crashed)
   (manifest known)      (constructed)    (in pipeline)   │
                         ▲                               │ backoff (extensions)
                         └────────── restart ◀───────────┘  → Disabled
```

- **Registered**: manifest parsed, validated, present in the registry. No
  code has run.
- **Instantiated**: the factory ran; a trait object exists. Not yet selected.
- **Active**: selected into a live pipeline and processing.
- **Failed**: construction/startup returned a typed error; engine continues
  without the plugin (MP §6 error isolation).
- **Crashed** (extensions only): the sandbox trapped; instance is destroyed,
  playback survives (§5.6).

Instance scope per kind (who owns construction/teardown):

| Scope | Kinds |
|---|---|
| per open source | `access`, `network-protocol` |
| per playback session | `demuxer`, `decoder` (per track), `subtitle-parser`, `subtitle-renderer`, `audio-filter`, `video-filter` |
| per job / per call | `encoder`, `playlist-parser`, `playlist-exporter` |
| app singleton | `audio-output`, `video-output`, `metadata-provider`, `service-discovery`, `ui-panel` |

Teardown always rides the pipeline **generation** mechanism (doc 03 §5): a
seek/source-replace/track-switch bumps the generation; plugin instances
belonging to the old generation are dropped and their work cancelled — stale
packets/frames can never reappear (MP §8).

### 1.4 Which kinds may be WASM extensions in v1

MP §6 names the suited set: *"UI panels, playlist and metadata handlers, and
light filters."* This document makes it a hard registry rule `[PROPOSAL]`:

- **Allowed as extensions in v1**: `ui-panel`, `playlist-parser`,
  `playlist-exporter`, `metadata-provider`, `service-discovery`,
  `audio-filter`, `video-filter` (the two filter kinds additionally pass a
  performance gate — §7 — because they sit nearer the realtime path).
- **Refused by the v1 host, with a clear typed error**: `access`,
  `network-protocol`, `demuxer`, `decoder`, `encoder`, `audio-output`,
  `video-output`. Rationale: byte-source and codec kinds dominate throughput
  (sandbox crossing per packet/frame is a measured cost MP §11 forbids us to
  ignore), outputs need OS/realtime access the sandbox cannot grant, and
  safety-critical paths stay in audited Rust (MP §6 "all core codecs and
  demuxers start here"). Revisit after Phase 7 with evidence `[PROPOSAL]`.

---

## 2. Manifest format

### 2.1 Principles

- One TOML manifest per plugin, named `<id>.toml`, schema-validated by
  `schemas/plugin.schema.json` `[PROPOSAL: file naming]`, generated from and
  tested against the `plugin-api` Rust types (one schema, all hosts — same
  discipline as doc 05 §5.1). Unknown fields are **rejected**, not ignored
  (forward-compat is handled by `api_version`, not by silent fields).
- Builtin manifests live in `plugins/builtin/` and are embedded into the
  binary with `include_str!` (§4.1). Extension manifests live inside the
  extension package (§5.7).
- A manifest describes **capability, not implementation**: it never carries
  code paths, feature flags of the engine, or config values.

### 2.2 Field-by-field specification

**`id` (required, string).** Globally unique, stable, never reused. Grammar:
`^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*){1,2}$`, max 64 bytes, at least two
dot-separated segments. Namespacing rules `[PROPOSAL]`:

- `lumen.<kind-area>.<name>` — first-party builtin (e.g.
  `lumen.demux.matroska`, `lumen.codec.flac`). The `lumen.` prefix is
  reserved; the registry rejects it for extensions.
- `ext.<publisher-slug>.<name>` — third-party extensions (e.g.
  `ext.acme.lyrics-panel`). `publisher-slug` must match the `provenance`
  publisher once a registry entry is created; the slug is claimed on first
  install for that user `[PROPOSAL]`.
- No other prefixes in v1.

**`kind` (required, enum).** One of the 16 kinds of §1.2.

**`version` (required, semver 2.0 string).** The plugin's own version.
Extensions: must equal the version encoded in the package filename and in
`provenance.hash`'s subject; mismatch = install rejection (§5.7).

**`api_version` (required, positive integer).** The engine **plugin API
major** the plugin was built against (§5.5). Load rule: extension loads iff
`api_version ≤ ENGINE_PLUGIN_API_MAJOR` and the engine still serves that
major (compat matrix §7.6). Builtin manifests carry the compiled-in value and
are checked by CI to match — a mismatch is a build break, never a runtime
surprise.

**`priority` (optional integer, default `100`, range 0–1000).** Selection
weight; higher wins among equal probe scores (§3.3). First-party defaults:
`100`; extensions default `50` so builtin wins contentional ties
`[PROPOSAL]`. Users may override per-id in settings (§3.4) within the same
0–1000 range — overrides are clamped, never rejected silently.

**`origin` (required, enum `builtin` | `extension`).** Set by the build for
embedded manifests; must be `extension` for anything loaded from disk. A disk
manifest claiming `builtin` is rejected as a provenance forgery attempt
(§9.4).

**`[capabilities]` (required table).** Kind-specific capability claims. Every
claim is verified by the conformance kit (§7.2) — a claim the plugin cannot
demonstrate on the corpus is a **manifest lie** and blocks shipping.

| Kind | Capability keys |
|---|---|
| `access` | `schemes = ["file", "file-disc"]`, `extensions` (hint), `supports_seek` |
| `network-protocol` | `schemes = ["https", "hls+https"]`, `features = ["redirects", "ranges"]` |
| `demuxer` | `formats = ["matroska", "webm"]`, `extensions = [".mkv", …]`, `mime_types = […]` |
| `decoder` / `encoder` | `codecs = ["flac"]`, `profiles`, `max_channels`, `sample_rates`, `bit_depths`, `hardware = false` (truthful, MP §2) |
| `audio-filter` / `video-filter` | `effects = ["gain", "eq-10band"]`, `formats_in`, `formats_out` |
| `audio-output` | `devices = ["default", "null"]`, `latency_class` |
| `video-output` | `surfaces = ["wgpu-window", "canvas"]` |
| `subtitle-parser` | `formats = ["ass", "ssa"]`, `features = ["styling-subset"]` |
| `subtitle-renderer` | `renders = ["text", "styled-subset", "bitmaps"]` |
| `playlist-parser` / `playlist-exporter` | `formats = ["m3u8", "xspf", "json"]` |
| `metadata-provider` | `provides = ["tags", "cover-art", "lyrics"]`, `sources = ["musicbrainz"]` |
| `ui-panel` | `surfaces = ["side-panel"]` |
| `service-discovery` | `discovers = ["upnp", "mdns"]` |

Lists are non-empty, deduplicated, lowercase; each entry ≤ 32 bytes; total
capabilities per manifest ≤ 64 entries (oversized claims are rejected — §9.4
tests this as a DoS guard).

**`[permissions]` (extensions only; must be absent for builtin).** Deny by
default. Recognized capabilities in v1 `[PROPOSAL — the set is additive
across `api_version` minors]`:

| Permission | Grants | Host enforcement point |
|---|---|---|
| `fs-sandboxed-read` | Read-only access to files under user-granted sandbox roots (the media directory of the open item, plus user-picked folders) | `lumen:v1.fs` import; canonicalized path check, symlink-escape rejection (MP §12 safe paths) |
| `http-fetch` | Outbound HTTPS GET via the host fetcher | `lumen:v1.http` import absent ⇒ no network at all |
| `network-host-allowlist` | (Companion list to `http-fetch`) Exact hostnames the plugin may contact | Host rejects any other host before connecting; TLS verification always on (MP §12) |
| `ui-surface` | Register panels/menus via the layout system | `lumen:v1.ui` import; panels render through the command layer only |
| `device-query` | Read device/output inventory (names, kinds, capabilities) | `lumen:v1.devices` import; read-only, no control |

Escalation attempts (calling an import whose permission was not declared)
trap the instance and are logged as security events (§5.3).

**`[config_schema]` (optional table).** Reference to the plugin's settings
fragment — a settings-schema document (type, range, default, description
key, `restart_required`, UI placement) per MP §5 and doc 07. Shape
`[PROPOSAL]`: `settings = "settings/<area>/<id>.toml"` (builtin, checked into
the repo) or an inline fragment (extension, embedded in the package). Rules:
keys are namespaced under the plugin id (`plugins.<id>.*`); duplicates across
plugins fail registry build with an actionable diagnostic (MP §5 "no option
is defined twice"); fragments feed the settings screens, `--help`, docs and
validation exactly like builtin settings.

**`[dependencies]` (optional array of plugin ids).** Plugin-to-plugin
dependencies (not crate dependencies). All listed ids must be present and
enabled, else the depending plugin is not loaded and `plugins.failed`
carries the missing id (actionable, never silent — MP §5). Cycles are
rejected at registry build. Extensions may depend on builtin ids and on
other extensions; builtin manifests may not depend on extensions (layering,
§4.3).

**`[provenance]` (required for extensions).**

| Field | Meaning |
|---|---|
| `publisher` | Human-readable publisher name (shown in install UI) |
| `source` | Canonical source URL of the package |
| `hash` | `sha256:<hex>` of the package's `plugin.wasm` — required, verified at install and at every load |
| `signature` | Detached signature over the package `[PROPOSAL: ed25519; scheme + key distribution decided by ADR, §10 Q5]` — optional in v1, required if `product.toml` demands signed extensions |
| `license` | SPDX identifier |

Builtin manifests carry `origin = "builtin"` and no provenance table: their
provenance is the signed git commit + the reproducible build, not a hash.

### 2.3 Example manifests

**(a) Builtin Matroska demuxer** (`plugins/builtin/lumen.demux.matroska.toml`):

```toml
id = "lumen.demux.matroska"
kind = "demuxer"
version = "0.1.0"
api_version = 1
priority = 100
origin = "builtin"

[capabilities]
formats = ["matroska", "webm"]
extensions = [".mkv", ".mka", ".mks", ".webm"]
mime_types = ["video/x-matroska", "audio/x-matroska", "video/webm", "audio/webm"]

[config_schema]
settings = "settings/demux/matroska.toml"
```

**(b) Builtin FLAC decoder** (`plugins/builtin/lumen.codec.flac.toml`):

```toml
id = "lumen.codec.flac"
kind = "decoder"
version = "0.1.0"
api_version = 1
priority = 100
origin = "builtin"

[capabilities]
codecs = ["flac"]
profiles = ["baseline"]
max_channels = 8
sample_rates = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100,
                48000, 88200, 96000, 176400, 192000]
bit_depths = [16, 24, 32]
hardware = false
```

**(c) WASM metadata-provider extension** (package
`ext.acme.musicbrainz-tags-1.0.0.lumenext`):

```toml
id = "ext.acme.musicbrainz-tags"
kind = "metadata-provider"
version = "1.0.0"
api_version = 1
priority = 50
origin = "extension"

[capabilities]
provides = ["tags", "cover-art"]
sources = ["musicbrainz", "coverartarchive"]

[permissions]
http-fetch = true
network-host-allowlist = ["musicbrainz.org", "coverartarchive.org"]

[config_schema]
settings = """
[plugins.ext.acme.musicbrainz-tags]
enabled = { type = "bool", default = true }
preferred-release = { type = "enum", values = ["album", "single"], default = "album" }
"""

[dependencies]

[provenance]
publisher = "Acme Tools"
source = "https://example.com/lumen/acme-musicbrainz-tags-1.0.0.lumenext"
hash = "sha256:9f2c…(64 hex)"
license = "MIT"
```

**(d) WASM ui-panel extension** (package
`ext.acme.lyrics-panel-0.4.2.lumenext`):

```toml
id = "ext.acme.lyrics-panel"
kind = "ui-panel"
version = "0.4.2"
api_version = 1
priority = 50
origin = "extension"

[capabilities]
surfaces = ["side-panel"]

[permissions]
ui-surface = true
fs-sandboxed-read = true      # reads .lrc files next to the open media

[dependencies]                # works best with tags, but degrades without it

[provenance]
publisher = "Acme Tools"
source = "https://example.com/lumen/acme-lyrics-panel-0.4.2.lumenext"
hash = "sha256:1ab4…(64 hex)"
license = "MIT"
```

---

## 3. Registry and selection

> **[LEAD-ATTENTION] — §3 and §4 are unowned spec (recorded 2026-10-22, card
> 3.DOC-1; audit source: 3.HARD-AUDIT AUDIT-2; re-dated 2026-11-09, issue
> #21).** Nothing in this section or
> §4 is implemented, and **no card in doc 13's wave program owns either**:
> plugin-api (1.D2) delivered §1–2 only — kinds, manifests, host traits —
> and the Phase-2..6 cards consume the engine without a registry
> (re-verified 2026-11-09: no `PluginRegistry` type exists in
> `crates/plugin-api` — only the doc comment at `crates/input/src/lib.rs:53`
> awaiting it; `plugins/builtin/` does not exist). The original proposal
> windows (wave-3 tail, Phase-5 head) have passed unexecuted. Updated
> proposal: add one S/M card to doc 13 (a Phase-6-tail or Phase-7-head slot,
> Lead's scheduling call): *"plugin registry + builtin registration v0"* —
> `PluginRegistry` built from manifests, capability+priority selection
> (§3.2), per-domain `register()` composition (§4.1), config
> enable/disable/reorder (§3.4) — sized against its first real consumers
> (playback-engine sink wiring, apps composition roots, UI plugin
> browsing). Existing doc 13 cards are NOT renumbered; this blockquote is
> the proposal, awaiting Lead dispatch.

### 3.1 Registry construction at startup

The registry (`plugin_api::PluginRegistry`, doc 01 §2.3) is built by the
**composition root** (apps/desktop, apps/cli, apps/web — doc 01 L6), in five
deterministic steps:

1. **Compile-time selection.** `cargo build` includes exactly the builtin
   plugin crates/features the product asks for (workspace features per doc 04
   tables, e.g. `demux-matroska`, `codec-flac`). What is not compiled in
   cannot be registered — the registry can never contain an un-compiled
   builtin.
2. **Builtin registration.** Each domain crate exposes
   `pub fn register(&mut PluginRegistry)`; the composition root calls them in
   layer order (L2 `input`, `streaming`, `demux`, `media-probe`, `subtitles`,
   `playlists`, `device-discovery` → L3 `codecs`, `audio-engine`,
   `video-engine` → L4 `audio-output`, `renderer-wgpu`). Each registration
   embeds its manifest via `include_str!` and a typed factory (§4.1).
3. **Extension scan.** If `product.toml` enables extensions
   (`[extensions] enabled = true` `[PROPOSAL — key shape, doc 07 owns]`),
   the host enumerates `<config-dir>/extensions/*/manifest.toml` (§5.7),
   validates each against the schema, verifies `provenance.hash`, resolves
   `[dependencies]`, and inserts passing entries. Failures emit
   `plugins.failed` and are otherwise ignored — one bad extension never
   blocks startup (MP §6 error isolation).
4. **Config application.** Layered config (product → profile → admin → user
   → CLI flags → session, MP §5 precedence) may **enable, disable and
   reorder** registered plugins — never add, never invent capabilities
   (§3.4).
5. **Freeze.** The registry is snapshotted; the snapshot is immutable for the
   process lifetime except that extension install/remove triggers an explicit
   rebuild (new snapshot; active sessions unaffected — §3.5) `[PROPOSAL]`.
   `plugins.loaded` events fire for every entry that made it in.

Registry internals `[PROPOSAL]`: a `slotmap` keyed by `PluginId` plus
per-kind capability indexes (`HashMap<CapabilityKey, Vec<PluginSlot>>`) so
selection is a lookup + short sort, not a scan. Factories are type-erased
(`Box<dyn Any>`); consumers downcast to the domain trait — a mismatch is a
manifest lie and fails conformance (doc 01 §2.3 rule).

### 3.2 Capability negotiation (probe → candidates → selection)

The general flow, concretized for the demuxer case (the pattern every
"claimed capability" kind follows):

1. **Open a source** via an `access`/`network-protocol` plugin selected by
   URI scheme (+ config order, §3.3).
2. **Bounded probe** (doc 01 `media-probe`): read a bounded header window
   (`ProbeLimits { max_bytes, deadline }` — defaults `[PROPOSAL]`: 64 KiB or
   250 ms) and offer it to every *enabled* demuxer/decoder capability that
   the URI hints (extension, MIME, scheme) could match.
3. Each candidate's plugin computes a **probe score** ∈ [0, 100] (the
   `Demuxer::score(header)` contract, doc 01). Scores below
   `CLAIM_THRESHOLD = 50` `[PROPOSAL]` are non-claims (doc 01: "scores below
   threshold are failures").
4. **Rank** the claiming candidates (§3.3) and select the winner.
5. If nothing claims ≥ threshold: typed `Unsupported` carrying known format
   info from the probe window (MP §8) — never a fallback guess.

Kind-specific negotiation:

- **decoder**: capability key is the codec id (+ profile) discovered by the
  demuxer; hardware/software preference follows ADR-001 (default: hardware
  first when available, *truthfully reported* via
  `system.get-engine-info`, doc 05 §2.9; user may force software).
- **filters**: chains are **explicitly ordered** in settings — negotiation
  only checks each configured filter exists and accepts the format; a missing
  id is an actionable config error, not a silent drop (§3.4).
- **outputs**: platform availability filters first (L5 adapters), then rank.
- **metadata-provider**: fan-out — *all* enabled providers whose `provides`
  matches the request run concurrently under a deadline; results merge by
  source precedence `[PROPOSAL]`; slow providers cannot block probe beyond
  the deadline.

### 3.3 Selection scoring and tie-breaks `[PROPOSAL]`

MP §6: "selects by capability and priority." Concrete total order — a
candidate's rank key, compared lexicographically, first component most
significant:

1. **probe score** — descending (capability confidence dominates; a
   demuxer that recognizes the actual bytes beats one matched only by file
   extension);
2. **effective priority** — descending (manifest `priority`, overridden by
   user setting, clamped 0–1000);
3. **user order index** — ascending (position in the user's reorder list;
   only distinguishes plugins the user explicitly ordered);
4. **class rank** — builtin before extension (MP §6: core codecs and
   demuxers start builtin for speed and safety; ties go to the audited
   binary);
5. **id** — ascending (final determinism guarantee; no two plugins share an
   id, so the order is total).

The order must be **deterministic and reproducible**: the same registry
contents and the same probe window always yield the same selection
(registry-build and selection determinism are tested — §9.1/§9.2). Selection
is pure: it reads the frozen snapshot and the probe window, nothing else.

### 3.4 Config interplay — enable, disable, reorder, never invent

- Sources of plugin config: `product.toml` (which builtin features are
  compiled in is a *build* decision; product.toml also states defaults and
  the extensions toggle), profiles, admin override, user preferences, CLI
  flags, session — full MP §5 precedence, owned by doc 07.
- Allowed operations, exhaustively: `enabled = false|true` (per id),
  `priority` override (clamped), chain/order lists (per kind where order is
  meaningful: filters, metadata precedence, output preference).
- **The never-invent rule** (MP §5/§6) is enforced mechanically:
  1. Config keys referencing a plugin id **not in the registry** fail
     validation with an actionable diagnostic naming the unknown id (MP §5:
     invalid critical config is never silently replaced). One exception
     `[PROPOSAL]`: config entries for builtin plugins not compiled into this
     build produce a *warning* (they exist in the schema but not in this
     binary) — because product.toml is shared across builds.
  2. Config **cannot create capabilities**: there is no config syntax that
     adds a format, scheme, codec or permission. Feature exposure
     (`system.get-engine-info` `features`/`plugins`/`decoders`, doc 05 §2.9)
     is generated **from the registry**, never from config.
  3. The settings-schema generator (MP §5) treats registry output as the
     only source of "what exists".
- Runtime surface (doc 05 §2.9): `system.list-plugins`,
  `system.plugin-enable`, `system.plugin-disable` — synchronous config
  writes returning `{ applied, takes_effect, restart_required }`.
  `takes_effect` semantics per kind `[PROPOSAL]`: source kinds take effect
  on next open; decoder on next session (doc 05 notes disabling an in-use
  decoder affects the next session); filters on next chain rebuild;
  outputs/metadata/discovery may restart the subsystem in place.
- Events (doc 05 §3): `plugins.loaded`, `plugins.failed`,
  `plugins.crashed { action_taken: Disabled|Isolated }`.

### 3.5 Hot-reload policy `[PROPOSAL]`

- **Builtin plugins: no hot reload.** They are compiled in; changing the set
  means rebuilding (Cargo features) or, for enable/disable, a registry
  rebuild that never swaps components of *running* pipelines — generations
  (doc 03 §5) carry the transition.
- **Extensions: install/remove flow only** (§5.7). Installing or removing an
  extension triggers a registry rebuild; new selections use the new snapshot;
  active sessions keep their old instances until they end. No in-place
  upgrade of a running instance, no module swapping under a live pipeline.
- Rationale: hot-swapping pipeline components interacts badly with
  generation-based cancellation and A/V stability, expands the sandbox's
  state machine, and buys little for a media player. Revisit only with a
  demonstrated need and a conformance suite extension.

---

## 4. Built-in plugins

### 4.1 The Cargo-feature registration pattern

One module per format/codec inside the domain crate, one workspace Cargo
feature per plugin, one manifest per plugin in `plugins/builtin/`, embedded
at compile time:

```rust
// crates/demux/src/lib.rs
pub fn register(registry: &mut PluginRegistry) {
    #[cfg(feature = "demux-matroska")]
    registry.insert(BuiltinRegistration {
        // validated by testkit in CI — see §7; never panics in a shipped build
        manifest: PluginManifest::from_toml(
            include_str!("../../plugins/builtin/lumen.demux.matroska.toml")),
        factory: BuiltinFactory::Demuxer(||
            Box::new(matroska::MatroskaDemuxer::new())),
    });
    #[cfg(feature = "demux-mp4")]
    registry.insert(/* … mp4 … */);
    // … one block per feature, straight from the doc 04 tables …
}
```

`BuiltinRegistration` holds the parsed manifest and a **typed** factory
variant (an enum, not a raw `dyn Any` — the type erasure happens inside the
registry, which stores `Box<dyn Any>` and downcasts to the domain trait at
selection; a downcast mismatch is a manifest lie and a conformance failure).
This gives: compile-time knowledge of what exists, zero runtime discovery
cost, monomorphized direct trait calls on the hot paths (no sandbox
crossing), and manifests as reviewable data files.

A CI check (`tools/feature-audit`) asserts **bijection**: every
`plugins/builtin/*.toml` matches exactly one `#[cfg(feature = …)]`
registration, and vice versa — no orphan manifests, no unregistered features
`[PROPOSAL: tool scope]`.

> **Status note (2026-11-09, issue #21):** `tools/feature-audit` is **not
> built** — the `[PROPOSAL]` above stands, and the bijection check has no
> runner today (nothing to check yet: `plugins/builtin/` does not exist; the
> pattern lands with the §3 registry card). The registry-vs-implementation
> drift check that *does* run today is the fuzz-target analog
> `scripts/fuzz_registry_check.sh` (card 5.Q1, gate #6 of
> `scripts/full_gate.sh`), plus `cargo xtask check-architecture` for the
> crate graph.

### 4.2 Why all core codecs and demuxers start builtin

MP §6 states it; the engineering reasons this document records:

- **Speed**: builtin plugins run as native Rust behind direct trait objects
  — no linear-memory crossing, no serialization of packets/frames (MP §11
  zero-copy frame paths).
- **Safety**: they pass the full §9 dependency audit at build time, carry no
  runtime trust question, and fail via typed errors, not traps.
- **Determinism**: registry contents are a compile-time fact; offline and
  first-run behavior never depends on disk state.
- **Auditability**: provenance is the git commit + reproducible build, which
  is stronger than any per-file hash scheme.

### 4.3 The no-cycles boundary

Doc 01's layering is the rule; this is how plugin registration respects it:

- Builtin plugin code lives in domain crates (L2/L3) that depend only
  downward (`media-types`, `plugin-api`, `codec-api`, …) — never on
  `playback-engine`, never on each other except via allowed layer edges
  (e.g. `codecs` → `codec-api`).
- `register()` is *called by* the composition root (L6 apps), which may
  depend on everything; apps are leaves, so no cycle can form.
- `playback-engine` (L4) hosts the registry **type** and consumes traits; it
  does not link implementation crates (doc 01 §2.6) — the registry arrives
  pre-built through `EngineSession::new(cfg, registry, sinks)`.
- Builtin manifests may not declare `[dependencies]` on extension ids
  (§2.2) — a compiled-in plugin cannot depend on user disk state.
- `cargo xtask check-architecture` (doc 01 §6) covers crate-graph cycles;
  the plugin-level rules above are checked by the same tool's manifest pass
  `[PROPOSAL]`.

---

## 5. WASM extension host (`crates/plugin-host-wasm`)

### 5.1 Runtime candidate requirements

MP §6: *"The WASM runtime must pass the dependency audit with no hidden
native code."* The runtime choice is **ADR-002** (doc 01 §7). Candidates
this document puts forward (all claims `[VERIFY-P0]` — Phase 0 must audit
before ADR-002 signs):

| Candidate | Posture | Why it is a candidate | Audit focus |
|---|---|---|---|
| `wasmtime` | JIT (Cranelift) + interpreter | Bytecode Alliance flagship; mature sandbox primitives: store-level memory limits, epoch interruption, fuel metering, import dispatch control | Transitive graph purity (no `cc`-built artifacts, no C runtime bits), size/licensing of the Cranelift graph, deterministic-build story |
| `wasmi` | Pure-Rust interpreter | Small, embeddable, long used in production (Polkadot ecosystem); no JIT ⇒ smaller attack surface | Whether current fuel-metering semantics suffice for our CPU budgets; throughput on metadata/playlist workloads |
| wasm3, Fizzy, and any C/C++ runtime | **Rejected outright** | C/C++ sources violate MP §9 | — |

The Phase 0 audit **must verify, per candidate**: (a) full transitive graph
via `cargo tree` against the allowlist; (b) zero C/C++/asm sources and no
`build.rs` invoking a non-Rust toolchain (tarball + repo scan, doc 04 §1.2
method); (c) presence of the primitives this spec needs — linear-memory cap,
epoch-or-fuel CPU limiting, per-instance import resolution; (d) license
compatibility; (e) maintenance/security record; (f) target coverage
(x86-64/aarch64, Windows/Linux/macOS); (g) behavior under our limit-breach
tests (§9.4). A dual-runtime posture — `wasmi` for debug builds, `wasmtime`
for release — is a fallback `[PROPOSAL]` if neither candidate passes all
gates alone; shipping **no** compliant runtime means extensions do not ship
(documented limitation, MP §9 "do not quietly relax the rule").

### 5.2 Execution model

- Each extension instance is a WASM module instance with its own **linear
  memory**. Host and guest never share memory; there are **no host pointers
  in the sandbox** — every import takes offsets+lengths inside guest memory
  (or plain integers/handles) and the host copies what it needs after
  bounds-checking. The guest cannot name host addresses at all.
- Imports are grouped into versioned modules (`lumen:v1.log`,
  `lumen:v1.fs`, `lumen:v1.http`, `lumen:v1.ui`, `lumen:v1.devices`,
  `lumen:v1.media`, `lumen:v1.host`) `[PROPOSAL: module set]`; the host
  resolves them per-instance according to the manifest's declared
  permissions.
- All entry points are request/response calls with deadlines; there is no
  guest-initiated host thread, no callbacks into arbitrary host state, and
  no ambient authority of any kind.
- Extensions run **off the realtime paths** (§5.8) and off the UI thread.

### 5.3 Capability-based permissions

Permissions are declared in the manifest (§2.2) and enforced **once, at
import dispatch**: the host builds the instance's import table containing
only the modules the declared permissions grant. An undeclared import is
either an instantiation error (module missing) or, if the module exists but
a specific call exceeds the grant (e.g. `http-fetch` without the host in
`network-host-allowlist`), a typed `PermissionDenied` error the guest can
observe. A permission escalation attempt is logged as a security event
(id, import, arguments' shapes — never raw buffers) and counted; repeated
attempts disable the extension `[PROPOSAL: threshold 3 per session]`.
Permissions are monotone within an `api_version` major: a new engine minor
may add permissions, never silently widen old ones.

### 5.4 Resource limits `[PROPOSAL — all numbers]`

Limits live in `product.toml` (schema-validated, MP §5) with these defaults:

| Limit | Default | Enforcement |
|---|---|---|
| Linear memory | 32 MiB (metadata/playlist/discovery), 64 MiB (ui-panel), 64 MiB (filters) | store-level memory cap; growth beyond cap traps `MemoryOom` |
| CPU per dispatched call | 100 ms wall-clock (epoch deadline); long-running ops get explicit budgets: 1 s per metadata batch, 5 s per playlist parse | epoch interruption preferred; fuel metering as fallback if ADR-002 picks `wasmi`-only |
| Instance startup | 500 ms | instantiation deadline |
| Single I/O transfer | 16 MiB | import argument check |
| Sustained I/O rate | 1 MiB/s average, 16 MiB burst per instance | host-side token bucket |
| HTTP requests | 32/min per instance, 8 concurrent | host fetcher |
| Open handles | 8 per instance | handle-table cap |
| Concurrent instances | 4 active per host, 16 installed-and-loaded per app | host scheduler |
| Total extension heap | 256 MiB across instances | host accounting |

Every breach produces a typed `ResourceLimit` error or a trap that the host
converts into the crash/isolation path (§5.6) — never a hang, never an OOM
of the engine process (MP §12 "bound memory"; doc 01 `ResourceLimits`).

### 5.5 API surface versioning

- The stable import set is versioned by module namespace: `lumen:v1.*`. The
  engine's `ENGINE_PLUGIN_API_MAJOR` (manifest `api_version`) is the
  compatibility contract.
- Within a major: **additive only** — new imports, new optional parameters,
  new result fields; existing signatures never change (same discipline as
  doc 05 §6).
- Deprecation: an import marked deprecated stays functional for **two
  engine minors** with a log-on-first-use notice, then is removed at the
  next major `[PROPOSAL: window]`.
- Major bump: `lumen:v2.*` modules are added alongside `v1` for **one
  transition cycle** so extensions can recompile; after the cycle, `v1`
  refuses to load with a clear "rebuild against api_version 2" error.
- The compat matrix (which engine minors support which majors) is itself a
  conformance artifact (§7.6).

### 5.6 Error isolation, restart policy, crash reporting

- A trap (unreachable, memory cap, epoch expiry, failed import) **destroys
  only that instance**. The engine's playback pipeline never had the
  extension on its critical path, so playback continues (MP §6: a plugin
  crash cannot take down playback). The host emits `plugins.crashed { id,
  error, action_taken: Isolated }` (doc 05 §3).
- Restart policy `[PROPOSAL]`: exponential backoff — immediate retry once,
  then 5 s, 30 s, 5 min; a fourth crash within the window moves the
  extension to `Disabled` (`action_taken: Disabled`) with an actionable
  message in the plugin UI. A user can re-enable (which resets the backoff).
- Crash reporting: a bounded local log (id, kind, trap reason, backtrace of
  the *host* call chain — guest backtraces are best-effort) with secret
  redaction per MP §12. No network reporting; if telemetry is ever added it
  is opt-in only (MP §5).
- Repeated-crash protection is per-version: upgrading an extension resets
  the crash counter (the user explicitly re-trusted it).

### 5.7 Install and remove flow

- **On disk**: per-user only — `<config-dir>/extensions/<id>/<version>/`
  containing `manifest.toml`, `plugin.wasm`, optional `assets/`, optional
  `signature`. Never a system directory, never on `PATH`, no executable bit
  required `[PROPOSAL: layout]`. Multiple versions coexist; the highest
  valid one loads.
- **Package format** `[PROPOSAL]`: `.lumenext` — a ZIP archive with
  `manifest.toml` at the root, `plugin.wasm` (magic bytes `\0asm` — anything
  else is rejected: this is the native-code fence for extensions), optional
  assets and detached signature. Size cap 64 MiB.
- **Install steps** (all user-initiated — MP §6 forbids automatic execution
  of downloaded plugins):
  1. User invokes install (UI file-open + confirmation dialog, or CLI
     `lumen extensions install <pkg>`); merely *downloading* or
     *double-opening* a `.lumenext` never installs or executes anything
     (tested, §9.4).
  2. Structural validation: archive shape, schema-valid manifest,
     `origin = "extension"`, wasm magic, capability sanity (§2.2 limits).
  3. Provenance: `hash` verified against `plugin.wasm`; signature verified
     if present or if policy requires it; publisher surfaced to the user
     with requested permissions in plain language before the final confirm
     (install-time consent is the permission grant moment).
  4. Copy into place; registry rebuild (§3.1 step 3–5); `plugins.loaded`.
     First execution happens only when something selects the plugin.
- **Remove**: user-initiated; active instances are terminated with a
  deadline (500 ms) then reaped; directory deleted; registry rebuilt. Refuse
  removal of a builtin (there is none on disk to remove).

### 5.8 Threading model

- The host owns a dedicated extension thread pool sized
  `min(4, available_parallelism)` `[PROPOSAL]`. Every guest call executes on
  a pool thread under a deadline; **no guest code ever runs on the audio
  realtime callback, the video presentation thread, or the UI thread** (MP
  §8; doc 01 threading model).
- `ui-panel` extensions do not touch the UI implementation at all: they
  exchange declarative panel descriptors and commands through the **command
  layer only** (doc 01 §2.8 `[PROPOSAL]`, doc 05 §1) — the panel renders in
  the React UI from data, exactly like builtin panels defined by layout
  config (MP §5). Events flow back over the same typed event stream
  (`config.setting-changed` already carries a `plugin` origin, doc 05 §3).

### 5.9 Kinds refused in v1

Per §1.4: `access`, `network-protocol`, `demuxer`, `decoder`, `encoder`,
`audio-output`, `video-output` extensions are refused at install with a
clear error naming the v1 policy. The package format, sandbox and limits are
nonetheless specified generically so later phases can widen the set by
evidence rather than redesign `[PROPOSAL]`.

---

## 6. Forbidden-list enforcement

MP §6 forbids three things outright; each gets a mechanical CI check that
**proves absence**, not a convention:

| Forbidden | Enforcement mechanism | Proof command (CI, merge-blocking per MP §13) |
|---|---|---|
| Dynamic loading of native `.so`/`.dll` plugins; `libloading` of native libs | Dependency-graph ban + source-token scan; no `cdylib` plugin crates exist in the workspace | `cargo tree --workspace -e normal,build` (must contain neither `libloading` nor any loader crate — `tools/dependency-audit` fails on match); token scan for `dlopen(`, `LoadLibrary`, `libloading` across `crates/ apps/ tools/` must return zero hits (allowlist: empty `[PROPOSAL]`) |
| Any C/C++ plugin (and any C/C++ anywhere outside documented OS-ABI exceptions — MP §9) | Native-code audit of the full transitive graph: `*.c *.cpp *.cc *.h *.hpp *.S *.asm` sources and `build.rs` scripts invoking non-Rust toolchains; extension packages accept only wasm payloads | `cargo deny check bans sources licenses` + `tools/dependency-audit --deny-native` (exit 0 = clean); install-path test: package with ELF/Mach-O/PE payload rejected by the `\0asm` magic check (§9.4) |
| Automatic execution of downloaded plugins | Install is a user-invoked command with explicit confirmation; no code path from download/open to execute; downloads land outside the extensions dir `[PROPOSAL: quarantine dir]` | Integration test `no_auto_exec`: open a `.lumenext` from a browser-download location — assert zero registry changes, zero process spawns (§9.4); code-review rule recorded here |

Additional standing gates that protect the plugin system:

- `cargo xtask check-architecture` — crate boundaries, cycles, the plugin-level
  manifest rules of §4.3 `[PROPOSAL: manifest pass]`.
- `tools/feature-audit` — builtin manifest ↔ Cargo-feature bijection (§4.1)
  and the fuzz-target registry completeness rule (§7.7).
- Schema validation: every manifest (builtin at build time, extension at
  install time) validated against `schemas/plugin.schema.json`; CI validates
  all `plugins/builtin/*.toml` and all `plugins/extensions/**` examples.
- Unsafe-code lint posture (MP §13): `plugin-api` and `plugin-host-wasm` are
  core crates — `#![forbid(unsafe_code)]`; if the chosen runtime requires an
  unsafe boundary, it lives inside the audited runtime crate, not ours
  (ADR-002 records this).

Every check reports exact commands and actual results (MP §13); none may be
weakened to pass (AGENTS §3.5).

---

## 7. Plugin conformance kit

MP §6: *"A plugin conformance kit is mandatory. Every plugin passes it, and
every parser plugin has a registered fuzz target."* The kit is implemented in
`crates/testkit` (`ConformanceRunner`, doc 01 §2.8) and runs for **builtin
plugins in CI** and for **extensions at install time and in CI** (example
extensions in `plugins/extensions/`). A plugin that has not passed the kit is
not `implemented`, cannot ship, and (extension) cannot be enabled.

### 7.1 Functional conformance (corpus in/out)

Per-kind corpora from `tests/fixtures` (MP §13: legally distributable +
generated malformed/truncated/multi-track/discontinuous cases; doc 04 §8.5):
every fixture has a recorded expectation file (packets/frames/cues/entries
hashes, track maps, timestamps). The runner executes the plugin against the
corpus and diffs against expectations. Demuxers/decoders additionally run the
differential harness against VLC 3.0.23 as an external oracle (MP §13) where
the format's record permits.

### 7.2 Capability honesty

Every manifest claim is exercised: a demuxer claiming
`formats = ["matroska", "webm"]` must probe-score ≥ threshold on every
claimed-format corpus file and produce conforming output; an `access` plugin
must actually open every claimed scheme; a filter must accept its claimed
`formats_in` and emit its claimed `formats_out`. A claim that cannot be
demonstrated = **manifest lie** = kit failure (the plugin must narrow its
claims). This is the MP §5 "feature exposure lists only implementations that
exist" rule applied at the plugin level.

### 7.3 Error-path tests

Corrupt, truncated, oversized, and adversarial inputs (MP §12) must produce
typed stage-named errors — never panics, never hangs, never fabricated
success (MP §8). The kit injects the `testkit::malformed` generators and
asserts: bounded recovery, bounded time, and honest `Unsupported`/`Corrupt`
reporting.

### 7.4 Limits compliance (extensions)

For each resource limit of §5.4, a dedicated breach test: a guest that
allocates past the memory cap traps `MemoryOom`; a tight loop is killed by
the epoch deadline within 2× budget; I/O abusers are throttled then refused
— and in every case the host surfaces a typed `ResourceLimit` outcome and
the engine keeps running.

### 7.5 Crash injection (extensions)

The kit ships malicious/trap-happy extension fixtures (unreachable on entry,
trap mid-call, trap in an import callback re-entry, memory-exhaustion loop).
Required result: instance destroyed, `plugins.crashed` emitted, host and
playback session survive, backoff sequence correct (§5.6).

### 7.6 API-version compatibility matrix

The kit builds each example extension against every supported
`api_version` major (and the oldest supported minor) and asserts the load
outcome matches the published matrix (loads / refuses with the exact
actionable error). The matrix itself is generated into
`docs/` + `system.get-engine-info` output `[PROPOSAL: artifact location]`.

### 7.7 Fuzz-target registry

Every **parser** plugin — demuxers, decoders/encoders (bitstream parsers),
subtitle-parsers, playlist-parsers/exporters, network-protocol parsers,
metadata-provider tag parsers (MP §12 "fuzz every parser") — registers a
cargo-fuzz target named:

```
fuzz/<kind-short>_<capability-id>
```

with the kind-short table: `demux` (demuxer), `codec` (decoder), `encode`
(encoder), `sub` (subtitle-parser), `playlist` (playlist-parser/exporter),
`proto` (network-protocol), `access` (access), `meta` (metadata-provider).
This reproduces doc 04's established names (`fuzz_demux_mkv` ↔ our
`fuzz_demux_matroska` — ids normalize to the doc 04 form where they differ
`[PROPOSAL: reconciliation rule — one canonical id per capability]`).
Registry file `fuzz/registry.toml` `[PROPOSAL]` maps plugin id → target
name; `tools/feature-audit` fails CI if any parser-kind plugin lacks an
entry. Doc 04 §8.5 rules apply: 10 minutes clean in CI (release), committed
seed corpus, coverage gate.

### 7.8 Conformance report format

The runner emits one TOML report per plugin, `conformance-<id>.toml`
`[PROPOSAL — schema versioned like all artifacts]`, consumed by the
parity-report tool and CI:

```toml
report_version = 1
plugin_id = "lumen.demux.matroska"
plugin_version = "0.1.0"
kind = "demuxer"
origin = "builtin"
api_version = 1
kit_version = "0.2.0"
environment = "linux-x86-64 / rustc 1.8x / CI-run-<id>"
verdict = "pass"            # pass | fail

[functional]
status = "pass"
corpus_files = 214
expectations_matched = 214
differential = "pass (oracle: VLC 3.0.23, 41 files)"

[capability_honesty]
claims = 2                   # matroska, webm
verified = 2

[error_paths]
cases = 96
panics = 0
hangs = 0
fabricated_success = 0

[limits]                     # extensions only; omitted for builtin
memory_breach = "trap-isolated"
cpu_breach = "epoch-killed"
io_breach = "throttled-then-refused"

[crash_injection]            # extensions only
traps = 12
engine_survived = true
backoff_sequence_ok = true

[api_compat]
matrix_entries = 3
mismatches = 0

[fuzz]
target = "fuzz_demux_matroska"
registry_ok = true
```

Rules: `verdict = "fail"` if any section fails; failing sections list the
failing fixtures with error text; reports are CI artifacts and are never
hand-edited (a hand-edited report is a fabrication violation, AGENTS §3.4).

---

## 8. v1 scope fence

Explicitly **not** in v1 — recorded so scope creep has a documented fence
(MP §6, §14):

| Item | Status | Rationale / reference |
|---|---|---|
| Native dynamically-loaded plugins | **Forbidden permanently**, not merely deferred | MP §6 forbidden list; §6 of this doc proves absence |
| C/C++ plugins of any kind | Forbidden permanently | MP §9; no exception path exists |
| Plugin marketplace / directory service | Not in v1 | Nothing in MP requires it; provenance model (§5.7) is manual-install by design |
| Automatic plugin updates (signed or not) | Not in v1 | Update = manual reinstall; avoids auto-exec-of-downloads entirely (MP §6); unsigned auto-updates remain forbidden beyond v1 too |
| VLC Lua extensions and playlist scripts | **Will not run** — known limit, recorded in the feature matrix | MP §14: they need a C Lua runtime; the WASM extension model replaces them; no compatibility shim is planned |
| Extension loading on the web build | Not in v1 `[PROPOSAL]` | Nested-wasm execution cost/COOP-COEP questions unresolved `[VERIFY-P0]`; desktop-first, revisit Phase 7+ |
| Extensions of source/codec/output kinds | Refused at install (§5.9) | MP §6 "suited to" set; revisit with evidence |
| Hot reload | No (§3.5) | Generations/complexity cost exceeds value |
| Mandatory extension signatures | Not in v1 (hash required, signature optional) `[PROPOSAL]` | Key distribution needs ADR (§10 Q5); policy can require signatures per-product via `product.toml` |

---

## 9. Test plan

All tests land in the owning crates plus `tests/` integration dirs; CI runs
them merge-blocking on all three desktop platforms (MP §13). Naming
`[PROPOSAL]`: suites prefixed `plugin_*`.

**9.1 Unit — manifest & registry.**
`plugin_manifest_validation` (every §2.2 rule as a negative case: bad id
grammar, reserved prefix misuse, unknown kind, bad semver, oversized
capability lists, `origin` forgery, unknown fields, builtin with
permissions, extension without provenance); `plugin_registry_build`
(deterministic order, duplicate-id rejection, dependency cycle rejection,
missing-dependency → skip + `plugins.failed`); `plugin_selection_scoring`
(the §3.3 total order: probe-score dominance, priority, user order, builtin
first, id tiebreak).

**9.2 Property tests.** Selection is a total order and pure (proptest:
random registries + probe vectors → sorted ranking is permutation-stable
under insertion order shuffles; clamping invariants; threshold behavior at
boundary scores 49/50/100). Manifest TOML fuzzing (arbitrary byte strings →
validator never panics, always typed error).

**9.3 Integration — config interplay.**
`plugin_config_layers` (MP §5 precedence for enable/disable/reorder;
unknown-id diagnostic; the compiled-out-builtin warning exception);
`plugin_commands` (doc 05 `system.list-plugins` /
`plugin-enable` / `plugin-disable` round-trips incl.
`takes_effect`/`restart_required` per kind); `plugin_never_invent`
(config attempts to reference/enable non-existent capability are refused;
feature-exposure output generated from registry only).

**9.4 Integration — sandbox & security.**
`plugin_sandbox_escape` (out-of-bounds offsets in import args → typed error,
no UB; symlink escape under `fs-sandboxed-read` → `PermissionDenied`;
canonicalization tests); `plugin_permission_escalation` (undeclared imports
absent; allowlist-violating fetch refused; escalation counter → disable);
`plugin_trap_loop` (infinite loop killed by epoch deadline ≤ 2× budget);
`plugin_memory_exhaustion` (grow-loop traps at cap; host heap unaffected);
`plugin_io_abuse` (rate/burst/concurrency caps); `plugin_malicious_manifest`
(forged `origin = "builtin"`, hash mismatch, signature mismatch, oversized
claims, dependency cycles across packages); `plugin_no_auto_exec`
(downloaded package opened → no install, no execution, no registry change);
`plugin_host_pointer_smuggling` (guest attempts to pass host-looking
handles; every import validates ownership).

**9.5 Integration — crash isolation & lifecycle.**
`plugin_crash_isolation` (trap-happy extension + real playback session:
playback survives, `plugins.crashed` observed, UI state truthful per doc 05
§1); `plugin_restart_backoff` (backoff sequence and eventual `Disabled`);
`plugin_install_remove` (valid/invalid packages; version coexistence;
remove-during-active-instance reaping within deadline; refusal to remove
builtin); `plugin_conformance_runner` (self-tests + run over all
`plugins/extensions/` examples, producing §7.8 reports).

**9.6 CI wiring.** Builtin conformance + fuzz registry on every PR;
extension-host security suite on every PR; full corpus runs nightly;
api-version matrix build on minor bumps. Soak suite (MP §13) includes an
extensions-enabled configuration with installed crash-yielding extensions to
prove long-run stability of the isolation path.

---

## 10. Open questions for the Lead

- **Q1**: Adopt the 16-kind split (subtitle-parser/renderer,
  playlist-parser/exporter) and update `plugin_api::PluginKind` (doc 01 has
  14 variants)? This doc's §1.2 is written against the split.
- **Q2**: `SubtitleRenderer` trait placement — `video-engine` (this doc's
  lean), `renderer-wgpu`, or `subtitles`; reconcile with doc 03's subtitle
  compositing proposal and doc 04's ASS styling-subset decision.
- **Q3**: `PlaylistExporter` in `playlists` (needed by `playlist.save`,
  doc 05 §2.7) — confirm ownership.
- **Q4**: ADR-002 runtime: `wasmtime` vs `wasmi` vs dual-runtime debug/release
  fallback (§5.1); audit results pending `[VERIFY-P0]`.
- **Q5**: Extension signature scheme (ed25519 detached proposed) and key
  distribution/pinning model; per-product signature policy knob.
- **Q6**: Approve (or tune) the §5.4 default limit numbers and §3.3 scoring
  constants (`CLAIM_THRESHOLD = 50`, defaults 100/50).
- **Q7**: Web-build extension loading — defer to Phase 7+ or descope
  explicitly in the feature matrix?
- **Q8**: `ui-panel` protocol: declarative descriptor set vs richer retained
  mode — affects the `ui-surface` permission's meaning and doc 08.
- **Q9**: Fuzz-target id canonicalization vs doc 04 names
  (`fuzz_demux_mkv` vs `fuzz_demux_matroska`) — one registry, one truth.

---

### Marker ledger (summary)

- `[PROPOSAL]` (main): 16-kind split & `PluginKind` reconciliation; manifest
  schema file/id-namespacing/priority defaults; permission set + shapes;
  selection scoring & tie-breaks; registry snapshot immutability &
  internals; hot-reload=none; builtin/feature bijection check; runtime
  candidates table posture (dual-runtime fallback); import module set; all
  §5.4 limit numbers; deprecation window; restart backoff; crash-counter
  threshold; `.lumenext` package format & disk layout; escalation-disable;
  extension kind allowlist; fuzz registry file; conformance report format;
  v1 fence rows marked inline.
- `[VERIFY-P0]`: every runtime-candidate claim (§5.1: purity, primitives,
  coverage of `wasmtime`/`wasmi`); nested-wasm on web feasibility (§8);
  (nothing else in this doc asserts unverified external facts).

*End of doc 06. When reality disagrees with this spec, the spec loses — via
an ADR or a Lead decision recorded in the worklog.*


