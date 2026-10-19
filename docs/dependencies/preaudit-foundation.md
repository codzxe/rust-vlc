# Foundation dependency pre-audit (Phase 1–2 candidates)

Task ID: 0.C4-R (completed by Lead after agent timeout — salvage per protocol).
Method: `docs/dependencies/audit-method.md` (quick pass: crates.io API metadata
+ dependency listing; full R1–R10 records happen at Phase 1 when the workspace
exists and versions are pinned). All data fetched 2026-10-19 via
`https://crates.io/api/v1/crates/<name>` and `.../<version>` (User-Agent
`lumen-preaudit/0.1`); raw JSON preserved in `tmp/preaudit2/` (gitignored).

**Verdict vocabulary** (per `allowlist-format.md`): `pass` (pure Rust,
permissive license) · `conditional` (pure Rust core but E2-class OS-ABI
bindings per target — needs ADR-tracked exception list) · `fail` (forbidden) ·
`exception-ADR-XXXX` (consumed only under a decided exception).

**E2 class reminder** (doc 11 §1): Rust bindings to **OS-service** ABIs
(windowing, audio devices, GPU, hw decoders) are permitted as audited
exceptions; bindings to **third-party C libraries** are forbidden. Metadata-
generated bindings (windows-rs family, objc2 family) are the canonical E2 path.

## 1. Summary table

| Crate | Version (latest stable, 2026-10-19) | License | Updated | Purity class | Verdict |
|-------|------|---------|---------|--------------|---------|
| serde | 1.0.229 | MIT OR Apache-2.0 | 2026-07-18 | Pure Rust (proc-macro + runtime) | **pass** |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 2026-10-11 | Pure Rust | **pass** |
| toml | 1.1.8+spec-1.1.0 | [VERIFY-P1: version endpoint returned null; crate docs state MIT OR Apache-2.0 — confirm at pin time] | 2026-10-09 | Pure Rust | **pass** (license re-check at pin) |
| schemars | 1.2.3 | MIT | 2026-10-12 | Pure Rust | **pass** |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | 2026-09-23 | Pure Rust (proc-macro) | **pass** |
| tracing | 0.1.44 | MIT | 2025-12-18 | Pure Rust | **pass** |
| tracing-subscriber | 0.3.23 | MIT | 2026-03-13 | Pure Rust core (watch out: `tracing-log`/`fmt` feature pulls only pure-Rust deps) | **pass** |
| cpal | 0.18.2 | Apache-2.0 | 2026-08-16 | Rust core; per-target E2 OS-audio bindings | **conditional** (E2: ALSA via `alsa`+`libc`, CoreAudio via `coreaudio-rs`/`objc2-*`, Android via `ndk`/`jni`, wasm via `wasm-bindgen`/`js-sys`) |
| wgpu | 30.0.1 | MIT OR Apache-2.0 | 2026-08-22 | Pure Rust (GPU API; native backends talk to OS graphics ABIs via E2-class crates: `d3d12`, `naga`… verify at pin) | **conditional** (E2 graphics-stack; self-describes "pure-rust graphics API") |
| winit | 0.30.13 | Apache-2.0 | 2026-09-04 | Pure Rust core; E2 windowing per target (Win32, X11 via `x11-dl` [VERIFY: dlopen of system libs], Wayland via `wayland-client` — wayland crates are Rust protocol implementations over unix sockets; macOS via `objc2`) | **conditional** (E2 windowing) |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | 2024-05-17 | Pure Rust (types only) | **pass** |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | 2026-09-25 | Pure Rust + JS shim | **pass** (web target toolchain) |
| windows | 0.62.2 | MIT OR Apache-2.0 | 2025-10-06 | Metadata-generated Rust bindings to Win32/COM ABIs | **conditional** (E2, canonical) |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | 2025-10-06 | Same, raw-FFI flavor | **conditional** (E2) |
| objc2 | 0.6.5 | MIT | 2026-10-05 | Metadata-generated Rust bindings to Objective-C runtime | **conditional** (E2, canonical for VideoToolbox/CoreMedia per 0.C5-b) |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | 2025-05-26 | Rust bindings to Core Foundation ABI | **conditional** (E2) |
| tauri | 2.12.4 | Apache-2.0 OR MIT | 2026-10-14 | Rust core hosting OS webviews | **exception-ADR-004** (OS webview exception; also pulls `wry`, `tao` — same exception envelope) |
| wry | 0.57.0 | Apache-2.0 OR MIT | 2026-09-08 | WebKitGTK/WebView2/WKWebView host (per 0.C5-a: WebKitGTK 4.1/GTK3 on Linux) | **exception-ADR-004** |

## 2. Notes per crate (pre-audit level, not R1–R10)

- **serde / serde_json / thiserror / schemars / tracing / tracing-subscriber**:
  industry-standard, pure Rust, permissive. No action beyond normal pinning.
  `schemars` 1.2.3 is the settings-schema codegen input (doc 07 §5).
- **toml**: toml 1.x is the modern fork line of the original crate
  [VERIFY-P1: exact lineage + license field null at version endpoint —
  confirm from tarball at pin time].
- **cpal**: 0.18.2 dependency listing (fetched 2026-10-19): `alsa`, `libc`,
  `coreaudio-rs`, `mach2`, `objc2-audio-toolbox`, `objc2-core-audio`,
  `objc2-core-audio-types`, `dasp_sample`, `jni`, `ndk`, `ndk-context`,
  `js-sys`, `wasm-bindgen` — i.e. **every native dependency is an OS audio
  service binding (E2), no third-party media C library**. This matches and
  slightly improves on 0.C5-a's finding (no more `-sys` Pulse/ALSA shims in
  the current dep set). ADR-005 (audio backend) will decide cpal vs
  per-platform direct (WASAPI via windows-rs, PipeWire protocol crate per
  0.C5-a's finding). cpal's callback model aligns with doc 03 §4.9
  audio-output rules.
- **wgpu / winit**: both are the "GPU + windowing" E2 envelope the brief
  already anticipates (MASTER-PROMPT §9 exception list names windowing, audio
  devices, GPU, hardware decoders). Full tarball scan at pin time must verify:
  wgpu's `d3d12`/`naga` native-backend graph; winit's `x11-dl` (dlopen-based
  X11 access — system libraries, E2-acceptable, but record it) and
  `wayland-client` purity.
- **windows / windows-sys / objc2 / core-foundation**: metadata-generated OS
  ABI bindings — the canonical clean way to reach Media Foundation (windows)
  and VideoToolbox (objc2), per 0.C5-b evidence. Belong on the E2 allowlist
  with per-target feature lists in their R1–R10 records (e.g. only
  `Win32_Media_Audio`, `Win32_Media_MediaFoundation` features — minimize
  surface).
- **tauri / wry**: consumed ONLY under ADR-004 (OS webview exception). They
  are the UI shell, not engine code; the engine never links them. wry 0.57 =
  WebKitGTK 4.1 on Linux per 0.C5-a — feeds ADR-003 risk assessment.
- **Not pre-audited here** (Phase 5+ media crates): symphonia, mp4parse, ogg,
  lewton, claxon, rav1d, mpeg2ts-reader, mpeg-ps, truehdd — those carry
  doc 04's verified findings and get full R1–R10 records at their phase.

## 3. Policy consequences

1. The engine's *core* crates (doc 01 L0–L4) should be able to depend only on
   `pass` crates (serde family, thiserror, tracing family, raw-window-handle)
   — anything more native belongs behind a platform adapter (L5) or renderer.
2. Every `conditional` crate gets an R1–R10 record at Phase 1 pin time with
   its exact E2 API list, satisfying MASTER-PROMPT §9 ("each needs an ADR, a
   list of exact APIs and a safety review") — one umbrella ADR-004/E2
   register + per-crate records, rather than one ADR per crate
   [PROPOSAL — Lead to ratify at Phase 1].
3. cpal vs direct-API decision = ADR-005 (Phase 1 wave 1.C per D-008).

## 4. Sources

All fetched 2026-10-19 (crates.io API, UA `lumen-preaudit/0.1`): crate roots
`/api/v1/crates/{serde,serde_json,toml,schemars,thiserror,tracing,tracing-subscriber,cpal,wgpu,winit,raw-window-handle,wasm-bindgen,windows,windows-sys,objc2,core-foundation,tauri,wry}`,
version endpoints for licenses, and
`/api/v1/crates/cpal/0.18.2/dependencies`. Raw JSON in `tmp/preaudit2/`.
Cross-references: `docs/inventory/platform-{linux,windows,macos}.md` (0.C5-a/b
findings), `docs/adr/ADR-002-evidence.md` (wasmi/wasmtime/wasmer/fizzy),
`docs/dependencies/records/wasmi.md`.
