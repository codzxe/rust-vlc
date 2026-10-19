# MASTER PROMPT v1 — Config-Driven, Plugin-Based Rust Reimplementation of VLC

This is the single authoritative prompt for the project. If it conflicts with any
earlier prompt, this one wins. The lead agent keeps it as the root brief;
sub-agents receive scoped assignments (section 15) and read AGENTS.md and the
relevant docs/ pages. Detail lives in docs/, not here.

## 1. Mission

Build a clean-room, from-scratch reimplementation of VLC media player: Rust media
engine, a new Figma-exact desktop UI, a WebAssembly web build, a fully
config-driven product, and a plugin architecture. Targets are Windows, Linux,
macOS and the browser.

"Exact" means measured. Parity is defined against a pinned baseline, VLC 3.0.23
(desktop, released 2026-01-08). VLC 4.0 is a later delta, tracked separately.
Parity is the share of inventory features in docs/feature-matrix.toml that are
verified by automated evidence. Never claim "exactly like VLC" or "all formats
supported" beyond that evidence.

Priorities, in order: (1) enforce the engine language policy, (2) correct
architecture and auditable dependencies, (3) one real end-to-end playback path,
(4) config-driven product and Figma-exact UI, (5) systematic VLC feature
coverage, (6) verified behavior on every claimed platform, (7) speed, security
and release quality throughout.

## 2. Fixed decisions (change only through an ADR)

• Engine: 100% Rust. It owns playback state, probing, demuxing, decoding,
  filters, audio/video pipelines, synchronization, streaming, devices,
  playlists, subtitle parsing and every media decision.
• UI: React + TypeScript. It owns presentation, component state, animation,
  layout, themes and interactions, and nothing else.
• Desktop shell: Tauri. The OS webview is a documented exception to the
  Rust-only rule. The project must never be described as all-Rust.
• Web build: the same engine compiled to wasm32 and called through a typed
  binding. The UI code is shared with desktop.
• Contract: the UI talks to the engine only through documented, typed commands
  and playback events (one command layer, section 3).
• Truthful UI: the UI never shows a playback operation as successful until the
  engine reports completion. Distinguish "accepted" from "completed".
• Config-driven: product.toml plus schemas are the source of truth for product
  decisions (section 5).
• Plugins in v1: built-in compile-time plugins and sandboxed WASM extension
  plugins (section 6).
• Monorepo: one Cargo workspace plus one UI workspace.
• Clean-room: no VLC source, comments, assets, logos, shaders or branding.
  Public specifications, observed behavior and independent tests only. VLC is
  GPL; running it as an external reference oracle in tests is allowed,
  vendoring or copying it is not.
• Hardware decoding (default, confirm by ADR in Phase 0): software codecs are
  written in Rust. OS decoders (VideoToolbox, Media Foundation, VA-API and
  similar) may be used only after audit, as a documented OS-ABI exception, with
  truthful reporting of when they are active.

## 3. Architecture and boundaries

1. Media types depend on nothing UI, GPU or platform specific.
2. Input and demux depend only on media types and shared interfaces.
3. The playback engine orchestrates sources, packets, decoders, clocks, queues,
   sinks and cancellation. It owns no UI state.
4. The renderer is isolated from decoding and demuxing.
5. One typed command layer serves the desktop UI (Tauri IPC), the web UI (wasm
   binding), the CLI, tests, and an optional local control server (loopback,
   versioned, authenticated for sensitive operations).
6. Platform adapters implement explicit traits and do not leak OS types into
   the core.
7. Test utilities may depend on production crates, never the reverse.
8. No cyclic crate dependencies, no hidden global mutable state. An automated
   boundary check runs in CI.
9. Commands are validated at the boundary and return typed results correlated
   to requests.

Pipeline: input, buffering, probe and demux, timestamped packets, bounded audio
and video queues, decoders (audio to mix and output, video to presentation
scheduler and renderer), all driven by one playback clock (audio master by
default). Subtitles, chapters, metadata and track selection are timed events on
the same model.

## 4. Monorepo layout (start small, split when justified)

```
AGENTS.md  Cargo.toml  rust-toolchain.toml  product.toml  product.schema.json
config/    themes/ keymaps/ layouts/ profiles/
locales/   en-US.toml ...
schemas/   settings schema (single source for UI, CLI, docs, validation)
apps/      desktop (Tauri)  web (wasm host)  cli  control-server
ui/        React + TypeScript workspace (components, tokens, screens)
crates/    product-config  localization  theme-tokens  commands  settings-schema
           media-types  media-probe  input  demux  packet-queue  codec-api  codecs
           playback-engine  media-clock  audio-engine  audio-output
           video-engine  renderer-wgpu  subtitles  playlists  streaming
           transcode  device-discovery  platform-{windows,linux,macos}
           plugin-api  plugin-host-wasm  control-protocol  testkit
plugins/   builtin/ (compile-time)   extensions/ (WASM examples)
tools/     xtask (rebrand, tokens, schema-gen) config-validator feature-audit
           dependency-audit license-audit parity-report bench-vs-vlc
tests/     fixtures integration playback compatibility security regression visual
fuzz/  benches/  docs/ (architecture, decisions, feature-matrix.toml, ...)  .github/workflows/
```

Every crate needs a real responsibility, a public interface, a dependency
direction and tests. No empty scaffolds.

## 5. Config-driven rules (nothing hardcoded)

Configure product decisions, not algorithms. Codec bitstream parsing, protocol
syntax and OS API calls are technical constants and stay in code.

• product.toml (validated by product.schema.json): name, short name, publisher,
  version display, app and bundle identifiers, description, window defaults and
  title template, brand assets, accent color, default theme and locale, feature
  exposure, default playback and track-selection rules, cache and resource
  limits, telemetry and privacy defaults (opt-in only), help/support/privacy/
  update URLs, schema version and migrations.
• Layered config, lowest to highest precedence: product defaults, profile,
  admin override, user preferences, command-line flags, session. Document it
  and test it.
• One settings schema (type, range, description, restart-required, UI
  placement) generates the settings screens, --help and CLI flags (VLC has
  thousands of options), documentation and validation. No option is defined
  twice.
• Localization: every user-visible string uses a stable key with parameterized
  messages, explicit fallback, tests for missing or duplicate keys and
  placeholder validation. No literal UI text in Rust or TSX. Technical
  identifiers are not product copy.
• Declarative UI structure: which panels, menus, toolbars and buttons exist,
  and their order, come from layout config. Pixel detail comes from design
  tokens (section 7).
• Commands and keymaps are data; menu items map to command IDs.
• Feature exposure lists only implementations that exist. Config can never
  enable a missing codec, protocol or hardware capability.
• Rebrand: cargo xtask rebrand rewrites crate names, paths and packaging for
  build-time changes. Changing the name, logo, accent color, menu labels or
  defaults in config must propagate to the window title, installer, About
  page, CLI identity, logs and docs. An acceptance test proves this.
• Invalid critical config fails with an actionable diagnostic. It is never
  silently replaced.
• CI fails on hardcoded user-visible strings, duplicated identity values,
  missing translation keys, schema violations and broken crate boundaries.

## 6. Plugin architecture

Anything with two or more implementations, or a real third-party need, is a
plugin: demuxers, decoders, encoders, filters, audio and video outputs,
subtitle formats, playlist formats, access and network protocols, metadata
providers, UI panels, services discovery.

• Each plugin ships a manifest (id, kind, version, capabilities such as
  formats, priority, permissions, config_schema, API version, dependencies).
  The engine builds its registry from manifests and selects by capability and
  priority. Config can enable, disable and reorder plugins, never invent them.
• Built-in plugins: crates behind stable traits, registered via manifests and
  Cargo features. All core codecs and demuxers start here for speed and safety.
• WASM extension plugins (v1): sandboxed, capability-based, with declared
  permissions, resource limits (memory, CPU time, I/O), API versioning, install
  and remove flows, provenance and trust checks, and error isolation so a
  plugin crash cannot take down playback. Suited to UI panels, playlist and
  metadata handlers, and light filters. The WASM runtime must pass the
  dependency audit with no hidden native code.
• Forbidden: dynamic loading of native .so/.dll plugins, any C/C++ plugin, and
  automatic execution of downloaded plugins.
• A plugin conformance kit is mandatory. Every plugin passes it, and every
  parser plugin has a registered fuzz target.

## 7. UI and Figma fidelity

• The Figma file is the visual source of truth. Obtain the file link or
  exported frames plus variables (colors, type scale, spacing, radii, shadows)
  and all component states (hover, pressed, focus, disabled, fullscreen,
  loading, error, empty).
• Token pipeline: Figma variables to a tokens file to generated CSS variables
  and Rust constants. Components consume semantic tokens only.
• Acceptance test: automated screenshot diffs against the Figma exports at
  defined viewport sizes and DPI. "Pixel-exact" means layout, spacing, color,
  radii, effects and typography match within a documented tolerance. OS text
  rasterization differs between platforms, so byte-identical output is not
  claimed.
• Required UX: playback canvas with unobtrusive controls, seek and volume,
  fullscreen, track menus, playlist and queue, media information, settings by
  area, light and dark themes, drag-and-drop, high-DPI, keyboard navigation,
  focus states and accessible labels, and clear loading, buffering,
  unsupported and error states.
• All controls invoke real commands, reflect real engine state and surface
  real errors.
• Prototype early (Phase 2): video presentation in the webview. Candidates
  are a native wgpu surface behind a transparent webview, or frame streaming
  to a canvas. Linux webviews are the highest risk. Test WebView2, WKWebView
  and WebKitGTK separately.

## 8. Media engine requirements

• Explicit types for sources, streams, packets, frames, timestamps, time bases
  and tracks. Missing and invalid values are explicit variants, never sentinel
  numbers. Time-base conversion is tested for overflow, discontinuity and
  missing timestamps.
• Bounded queues with backpressure. Seek, stop, source replacement and track
  switch cancel old work via generation IDs or cancellation tokens, so stale
  packets and frames cannot reappear.
• Threading by measured need. No async for CPU-heavy decode. No locks across
  blocking work. No blocking in real-time audio callbacks.
• Typed errors that name the failing stage and avoid leaking secrets:
  unsupported, corrupt, inaccessible, network, decoder, device, resource limit.
  Bounded, observable recovery. A failed open or seek is never reported as
  success.
• Unsupported media returns a clear error with known format info. Never
  fabricate frames, silence, durations or metadata.
• Per-format records for containers, audio and video codecs and subtitle
  formats: parse, decode or encode, bit depths, tracks, seeking, timestamps,
  corrupt-input behavior, limits, platforms, corpus, results.

## 9. Rust-only policy and dependencies

• Forbidden: C, C++, Objective-C sources; build scripts that invoke C/C++
  compilers; third-party C/C++ media engines behind Rust wrappers; vendored or
  downloaded native code; hidden transitive native dependencies; bundling a
  native codec while calling the result Rust.
• Documented exceptions only: the OS webview (UI shell) and audited OS-service
  bindings (windowing, audio devices, GPU, hardware decoders). Each needs an
  ADR, a list of exact APIs and a safety review.
• Before accepting any crate, record its language and source, full transitive
  graph, build scripts, native links, targets, actual feature coverage,
  license, maintenance and security record, and provenance. Keep an allowlist.
  A Rust API does not prove a crate is pure Rust.
• If no compliant implementation exists, document the limitation and continue.
  Do not quietly relax the rule.
• Unsafe code is denied in application and core crates. It is permitted only in
  approved platform adapters, each with documented invariants.

## 10. Web (WASM) target

• The web build has its own column in the feature matrix. It cannot do raw UDP
  or RTSP, optical discs or direct capture hardware. Capture goes through
  browser APIs. Threaded WASM needs COOP/COEP headers. Mark such features
  blocked or unsupported for web with the reason.
• Use WebGPU or WebGL fallback for rendering and WebAudio-compatible output.
  Test across Chrome, Firefox and Safari and record results.

## 11. Performance ("fast" is measured)

Baseline first, then targets. Measure against VLC 3.0.23 on the same files and
hardware: startup, probe latency, time to first frame and sound, seek latency,
CPU and GPU use, peak memory, dropped frames, audio underruns, rebuffer
frequency. Design for zero-copy frame paths, GPU color conversion, SIMD in hot
loops, hardware decode for high resolutions and bounded memory. Pure-Rust
software decoders may start slower than VLC's tuned decoders. Report that
honestly and improve it with evidence. Never trade correctness for a benchmark.

## 12. Security

Treat all media, playlists, subtitles, metadata, network responses and control
messages as untrusted. Bound memory, probing depth and queues. Use safe integer
conversions, deadlines and cancellation on I/O, safe path handling, TLS
verification, explicit redirect and credential policies, and secret redaction
in logs. No default telemetry. Fuzz every parser. A local control server binds
to loopback, is versioned and validates every request.

## 13. Testing and CI (merge-blocking on Windows, Linux, macOS)

• Local gate: cargo fmt --all -- --check, cargo check --workspace --all-targets
  --all-features, cargo clippy --workspace --all-targets --all-features --
  -D warnings, cargo test --workspace --all-features, cargo tree --workspace,
  plus UI lint, typecheck and tests, and Tauri and WASM builds. Report exact
  commands and actual results. Never claim a command ran if it did not.
• Lints: deny unsafe_code, unused_must_use, unused_imports, unused_variables;
  clippy deny unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro,
  await_holding_lock. Every crate inherits workspace lints. Never suppress
  warnings to hide defects.
• CI gates: schema validation, config propagation test, native-code and license
  audit (cargo-deny, cargo-vet, SBOM), vulnerability checks, architecture
  boundaries, feature-matrix validation, Figma screenshot diffs, plugin
  conformance, packaging and launch tests, fuzzing and property tests.
• Corpus: small, legally distributable fixtures (public domain, licensed or
  generated) with recorded expectations, plus generated malformed, truncated,
  multi-track and discontinuous-timestamp cases.
• Integration: open, play, pause, seek, stop, repeated seeks, A/V sync, track
  switching, corrupt and missing files, interrupted network, device loss, long
  and short media, reopen cycles, exit during playback. Run soak and stress
  tests. Label mock evidence separately from real hardware evidence.
• Differential tests: compare outputs against VLC 3.0.23 run externally as an
  oracle.
• Every defect gets a regression test. Never weaken or delete a test to pass.

## 14. Known limits (record in the matrix, do not hide)

• VLC Lua extensions and playlist scripts need a C Lua runtime. They are
  replaced by the WASM plugin model and existing Lua scripts will not run.
• DVD and Blu-ray copy protection and menus involve C libraries and legal
  questions. Decide each by ADR.
• DVB and capture hardware cannot be verified without the hardware.
• Codec patent and redistribution risk is assessed separately from source
  licenses.
• Parity with H.264, HEVC and other complex codecs depends on the Phase 0
  codec-coverage audit and the hardware-decoding ADR.

## 15. Multi-agent protocol

The lead agent manages, assigns and reviews. It does not write everything
itself. It gives each agent detailed, constructive feedback and rejects work
that is incomplete, unsafe or out of scope.

• Lead: architecture, shared types and interfaces, command and plugin
  contracts, dependency approval, integration order, CI policy, final
  verification.
• Investigation agents: VLC inventory, Rust codec and demuxer coverage,
  platform and hardware options, UI and video-in-webview, dependency and
  license audit, security. Findings are written to the repo as source-linked
  reports.
• Implementation agents: one per stable subsystem, on separate crates, branches
  or worktrees. Agents never edit the same shared interface or root config at
  the same time.
• Every assignment states: exact scope, out-of-scope, owned files or crates,
  interfaces, acceptance criteria, tests, commands to run, dependency-policy
  rules, integration dependencies, and a final report (files changed, tests run
  with results, limitations, open issues).
• Agents never mark their own work verified. The parity tool decides, from
  automated evidence.
• Continue until the phase gate passes. Stop and ask only for a decision that
  changes an interface, policy or fixed decision (section 2). When a
  requirement is ambiguous, request a specification decision instead of
  inventing behavior.

## 16. Roadmap and gates

• Phase 0 — Audit: pin VLC 3.0.23, build the inventory from VLC's plugin
  categories, options, preferences, menus and hotkeys, write the feasibility
  report (Rust-only parity vs reality), decide hardware decoding and the WASM
  runtime by ADR.
• Phase 1 — Foundation: workspace, AGENTS.md, product.toml, schemas, lints, CI,
  native-code audit, command layer, plugin API, settings-schema generator,
  token pipeline.
• Phase 2 — First vertical: one real file from open to decode to audio and
  video on screen, through the command layer, from CLI and from the UI.
  Include the video-in-webview prototype and the Figma token proof.
• Phase 3 — Reliable engine: clocks, queues, generations, seeking, source
  replacement, A/V sync, recovery.
• Phase 4 — Desktop product: Figma-exact UI, layouts from config, themes,
  localization, playlists, settings, plugins, rebrand test.
• Phase 5 — Breadth: containers, audio and video codecs, subtitles, filters,
  one verified increment at a time.
• Phase 6 — Network and streaming: HTTP(S), HLS, DASH, RTSP where possible,
  buffering, reconnect, authentication, fuzzing.
• Phase 7 — Platform and advanced: devices, hardware decode, recording,
  conversion, stream output, WASM extensions, web build parity.
• Phase 8 — Hardening and release: compatibility, security, soak, performance
  versus VLC, packaging, honest parity report.

A phase is done only when its gate passes with recorded evidence on each
claimed platform.

## 17. Reporting and definition of done

After each cycle, report: Implemented, Changed, Verified (commands and actual
results), Compatibility (formats and platforms tested), Dependency compliance,
Known gaps, Risks, Next action. Failing checks are fixed before the work counts
as done.

Statuses are not_started, in_progress, implemented, verified, blocked. A feature
is verified only when it is specified, complete, tested including error paths,
tested on each claimed platform, documented with its limits, and
non-regressing. A blocked item needs a reason, the investigation done,
alternatives considered and a next step.

The project is done when: the engine passes the language-policy audit,
product.toml and schemas drive the product with a passing rebrand test, the UI
matches the Figma exports within tolerance, plugins load and sandbox correctly,
the web and desktop builds both play real media through the same command layer,
CI is merge-blocking on all platforms, and the parity report states exactly
what is verified, partial, unsupported or blocked.
