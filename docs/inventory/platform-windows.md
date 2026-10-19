# Platform Record: Windows (WebView2 / Media Foundation / WASAPI / wgpu-DX12)

- Task ID: 0.C5-b (split retry of 0.C5-I; Windows+macOS half — Linux owned by the sibling task).
- Date of research: 2026-10-19 (all URLs accessed this date from the audit sandbox).
- Feeds: ADR-001 (hw decode: Media Foundation), ADR-003 (video-in-webview, Candidates A/B), ADR-005 (audio backend).
- Method note: sandbox has no WebSearch/WebFetch tools; all web facts below were fetched directly
  (curl on learn.microsoft.com / crates.io / store API; headless Chromium via agent-browser for the
  JS-rendered Microsoft Store). crates.io facts via `https://crates.io/api/v1/crates/<name>`.
  Crate facts verified against published crate tarballs (Cargo.toml + src) where noted.
- Classification reminder (policy §9): metadata-generated Rust bindings to OS ABIs (windows /
  windows-sys / webview2-com / windows-webview) are the standard **E2-class** access path — this
  record classifies, it does not approve. Verdicts are for the Lead's ADRs, not decisions.
- Clean-room: only public API *names* and documented behavior are recorded; no VLC source consulted.

## 1. Component versions (verified 2026-10-19 via crates.io API)

| Crate | Max stable | Newest | Last update | Role |
|---|---|---|---|---|
| wry | 0.57.0 | 0.57.0 | 2026-09-08 | webview abstraction used by Tauri (WebView2 backend) |
| tauri | 2.12.4 | 3.0.0-alpha.5 | 2026-10-14 | app framework (v2 stable line pinned here) |
| tauri-runtime-wry | 2.12.2 | — | — | tauri↔wry bridge; pins wry 0.57.0 + tao 0.37.0 (verified in its Cargo.toml) |
| tao | 0.37.1 | 0.37.1 | 2026-09-26 | windowing (Tauri); `rwh_06` feature |
| winit | 0.30.13 | 0.31.0-beta.3 | 2026-09-04 | alternative windowing (has `with_clip_children`) |
| raw-window-handle | 0.6.2 | 0.6.2 | 2024-05-17 | handle abstraction for surface creation |
| webview2-com | 0.39.1 | 0.39.1 | 2026-03-11 | Microsoft-lineage WebView2 COM wrappers (wry dep) |
| windows | 0.62.2 | 0.62.2 | 2025-10-06 | windows-rs facade (E2-class) |
| windows-sys | 0.61.2 | 0.61.2 | 2025-10-06 | raw-sys windows-rs variant (E2-class) |
| windows-core | 0.100.0 | 0.100.0 | 2026-09-03 | new common-version generation (see §1 note + §4.3) |
| windows-webview | 0.100.0 | 0.100.0 | 2026-09-03 | NEW Microsoft WebView2 wrapper crate (see §2.4) |
| wgpu | 30.0.1 | 30.0.1 | 2026-08-22 | GPU abstraction (dx12 in default features) |
| cpal | 0.18.2 | 0.18.2 | 2026-08-16 | audio; WASAPI host via windows 0.62 (see §5) |

Note (windows-rs generation shift): microsoft/windows-rs now tags releases 69..74 (tag 74 =
2026-09-03) and moved supported crates to a common `0.100.0` version + Rust 2024 edition (MSRV
Rust 1.95; windows-sys 1.88; windows-link 1.85) per the August 2026 newsletter (issue #4867,
https://github.com/microsoft/windows-rs/issues/4867). At tag 74 the workspace `windows` crate is
version 0.100.0 (verified from repo Cargo.toml) but crates.io still lists `windows` facade at
0.62.2 while windows-core/windows-link/windows-webview/windows-window/windows-composition/
windows-canvas 0.100.0 are already published. Treat "which generation do we depend on" as a
dependency-audit decision (ADR-002 / doc 11) — both 0.62.x and 0.100.x lines are E2-class.

## 2. Candidate A: native wgpu surface behind a transparent webview

### 2.1 WebView2 hosting modes (documented support)

Source: "Windowed vs. Visual hosting of WebView2",
https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/windowed-vs-visual-hosting
(accessed 2026-10-19). Three modes:

1. **Windowed hosting** (default): WebView2 content hosted in an HWND; OS delivers input.
   "Windowed hosting is a good starting point for most apps." This is what wry 0.57.0 implements
   (verified: `CreateCoreWebView2ControllerWithOptions(hwnd, ..)` / `CreateCoreWebView2Controller`
   in `src/webview2/mod.rs`; no composition code present).
2. **Window to Visual hosting** (newer): content output to an `IDCompositionVisual` hosted in an
   HWND; enabled by environment variable `COREWEBVIEW2_FORCED_HOSTING_MODE =
   COREWEBVIEW2_HOSTING_MODE_WINDOW_TO_VISUAL` before WebView2 init. Near-windowed dev experience
   while avoiding DPI/input-queue issues (different DPI-awareness / integrity levels of apps
   sharing a user-data folder; VSTO add-in scaling hangs).
3. **Visual hosting**: `ICoreWebView2CompositionController`; the app composes WebView2 into a
   DirectComposition / Windows.UI.Composition visual tree (`IDCompositionVisual`,
   `IDCompositionTarget`, `Windows.UI.Composition.Visual`). Requirements: app must handle content
   scaling (Rasterization Scale APIs) and must forward all spatial input (mouse/touch/pen) itself,
   transforming coordinates into WebView2 space.

### 2.2 Transparency: documented support and artifacts

Source: ICoreWebView2Controller2,
https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller2
(introduced WebView2 1.0.774.44; accessed 2026-10-19). Verbatim-critical facts:

- `put_DefaultBackgroundColor(COREWEBVIEW2_COLOR)` — RGBA; "The A represents an Alpha value,
  meaning DefaultBackgroundColor can be transparent. In the case of a transparent
  DefaultBackgroundColor WebView will render hosting app content as the background."
  → this is the documented foundation of Candidate A on Windows (windowed mode).
- **Alpha is binary**: "The only supported alpha values are 0 and 255, all other values will
  result in E_INVALIDARG." No semi-transparency. Not supported on Windows 7 (E_INVALIDARG).
- **Known artifact (Microsoft-documented)**: "There is a known issue with background color where
  setting the color by API can still leave the app with a white flicker before the
  DefaultBackgroundColor takes effect. Setting the color via environment variable solves this
  issue" — `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` (hex, alpha in the first 2 digits).
- wry 0.57.0 maps `WebViewBuilder::with_transparent(true)` to exactly this: sets background
  (0,0,0,0) via `ICoreWebView2Controller2::SetDefaultBackgroundColor`, guarded by `!is_windows_7()`
  (verified in `wry-0.57.0/src/webview2/mod.rs`, `set_background_color` + transparent branch).
  Note: wry's `ICoreWebView2ControllerOptions3::SetDefaultBackgroundColor` path force-coerces any
  alpha != 0 to 255, so transparency via controller *options* is NOT available — only via the
  controller-level property after creation (consistent with the doc).

### 2.3 Known artifacts from the field (wry/Tauri issues)

- wry issue #1540 (2025-04-09, closed) "Transparent webview also makes the content behind it
  transparent": embedding a transparent wry webview in another native app (Godot) on Windows
  showed the **desktop**, not the host app's own rendering — parent-surface/child-HWND compositing
  order problem; reporter's macOS test "renders perfectly fine". Resolution referenced
  bevyengine/bevy#16544: the fix is **`WS_CLIPCHILDREN` off** on the parent window
  (`winit_window_attributes.with_clip_children(false)`); with WS_CLIPCHILDREN the parent's
  swapchain area under the child webview is clipped away.
  https://github.com/tauri-apps/wry/issues/1540, https://github.com/bevyengine/bevy/issues/16544
- wry issue #1658 (2026-01-22, closed): Windows `with_decorations(false)` +
  `with_transparent(true)` causes a titlebar artifact (transparent titlebar not removed).
- wry issue #408 (2021, closed): `backdrop-filter` not applied in transparent windows on
  Windows 10 (Chromium + WS_EX_LAYERED/DWM interplay) — CSS blur effects over transparent areas
  are a risk for the Figma-fidelity UI (doc 08).
- Implication for ADR-003 Candidate A on Windows: workable with windowed hosting **iff** the
  wgpu host window is created without WS_CLIPCHILDREN (winit supports `with_clip_children(false)`;
  tao 0.37.1 does **not** expose a clip-children control — grep of tao source shows no
  WS_CLIPCHILDREN handling; tao's default styles don't set WS_CLIPCHILDREN, but the Bevy bug
  indicates winit's default does. [VERIFY] tao default style bits on a real Windows box in Phase 2
  prototype). Visual hosting (composition controller) is the fully-supported z-order-free path but
  costs input forwarding + rasterization-scale handling and is not implemented by wry today.

### 2.4 Raw-window-handle path: what wry/Tauri expose today (source-verified)

- raw-window-handle 0.6.2 (2024-05-17): `HasWindowHandle` / `HasDisplayHandle` traits.
- wry 0.57.0 (`src/lib.rs`, verified in the published tarball):
  - `WebViewBuilder::build<W: HasWindowHandle>(&self, window: &'a W)` — webview fills the window;
    `build_as_child` — webview as a **child window** with bounds.
  - re-exports `raw_window_handle`; docs state the webview requires "a window type that implements
    HasWindowHandle, or a gtk container widget" (X11/Wayland).
  - `trait WebViewExtWindows`: `controller() -> ICoreWebView2Controller`, `environment() ->
    ICoreWebView2Environment`, `webview() -> ICoreWebView2`, `reparent(hwnd: isize)`, and
    **`hwnd() -> windows::Win32::Foundation::HWND`** ("Returns the child HWND hosting this
    webview") — i.e. wry itself owns the WebView2 child HWND; the parent HWND is the tao/winit
    window we hand to wgpu.
- tauri 2.12.4 (`src/webview/webview_window.rs`, verified): `WebviewWindow<R>` implements
  `raw_window_handle::HasWindowHandle` **and** `HasDisplayHandle`; plus platform methods
  `hwnd() -> crate::Result<HWND>` (Windows) and `ns_window()/ns_view()` (macOS). `Window<R>`
  implements the same. Dependency chain verified in Cargo.tomls: tauri 2.12.4 →
  tauri-runtime-wry 2.12.2 → wry 0.57.0 + tao 0.37.0 (`rwh_06`).
- tao 0.37.1 extras (verified): `with_transparent(bool)` exists; on Windows `transparent` maps to
  `WindowFlags::TRANSPARENT` which sets **no** extended style (it is effectively a no-op on
  Windows); `WS_EX_NOREDIRECTIONBITMAP` is reachable only via platform-specific
  `no_redirection_bitmap` attribute (`WindowFlags::NO_BACK_BUFFER`).
- NEW Microsoft option: **windows-webview 0.100.0** (+ windows-window 0.100.0), published
  2026-09-03 in the windows-rs repo (release 74). Wraps WebView2 COM APIs ("Windows WebView wraps
  the WebView2 COM APIs to host Microsoft Edge (Chromium) in a window"); exposes
  `Environment::new()`, `environment.create_controller(&window)`, `Controller` with
  `default_background_color(Color)` (backed by SetDefaultBackgroundColor; verified in
  src/controller.rs + bindings.rs). Windowed hosting only — no ICoreWebView2CompositionController
  bindings in 0.100.0. A candidate E2-class replacement for webview2-com/wry on Windows if we ever
  self-host the webview; wry remains the battle-tested path (40.6M downloads).
- wgpu 30.0.1 consumes rwh 0.6 via `SurfaceTarget` (window or raw handle) — exact 30.x surface
  entry API [VERIFY against docs.rs when Phase 2 scaffolding is written].

## 3. Candidate B on WebView2: WebCodecs

- WebView2 Runtime is Chromium: "Evergreen" distribution — same platform as Microsoft Edge;
  "By the time a new WebView2 SDK package is released, a compatible version of the WebView2
  Runtime has already been distributed to clients."
  https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution (2026-10-19).
- WebCodecs API support: Chromium since 94 (2021) — caniuse `webcodecs` feature data (Chrome 94,
  Edge 94; fetched from https://raw.githubusercontent.com/Fyrd/caniuse/master/features-json/webcodecs.json,
  2026-10-19); MDN BCD `api.VideoDecoder` chrome 94. WebView2 tracks current Edge, so
  VideoDecoder/VideoEncoder/VideoFrame are available today in any updated runtime.
- "Differences between Microsoft Edge and WebView2"
  (https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/browser-features,
  2026-10-19) — the web-platform features listed as unavailable in WebView2 are only: Push
  Notifications, Web Payment API, Periodic Background Sync. WebCodecs is **not** restricted, and
  no codec differences between Edge and WebView2 are listed on that page.
- MDN WebCodecs overview: decode "efficiently (using hardware acceleration) ... per-frame basis"
  (https://developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API). Chromium's WebCodecs uses
  platform decoders (D3D11/DXVA on Windows) where available.
- Frame path for Candidate B: `VideoFrame` → `canvas` 2D `drawImage` or `createImageBitmap`; GPU
  path via WebGPU `importTexture` offscreen [VERIFY Chromium + WebView2 support for the WebGPU
  import path in the pinned runtime; WebGPU shipped Chromium 113].
- Codec availability inside WebView2 (B-relevant): H.264/VP8/VP9/AV1 software+hardware per
  Chromium/Edge capability; **HEVC in Chromium-class browsers requires platform HEVC support**
  (Edge enables HEVC when the OS can decode it) — exact WebView2 behavior on a stock Windows
  install [VERIFY on hardware]; this overlaps the §4 store-extension situation.
- Latency/design note: Chromium schedules decoders on internal threads; frame streaming to canvas
  adds a copy + compositing hop per frame vs Candidate A's direct scanout. Numbers to be measured
  in the Phase 2 prototype (doc 12 budget TBD there).

## 4. HW decode via Media Foundation (ADR-001 Windows path)

### 4.1 In-box vs install-required codec MFTs

- In-box software/hardware-assisted MFTs, per "Supported Media Formats in Media Foundation"
  (https://learn.microsoft.com/en-us/windows/win32/medfound/supported-media-formats-in-media-foundation,
  2026-10-19): video — H.264 decoder+encoder (Windows 7+), MPEG-4 Part 2, H.263 (Win8), MJPEG,
  DV, WMV family; audio — AAC, MP3, WMA, ADPCM, μ-law, GSM. **HEVC, VP9, AV1 are not in this
  table** — they arrive via Store "extensions" that install MFTs:
- Store situation, verified 2026-10-19 via apps.microsoft.com product search API
  (`/api/products/search`, publisher = Microsoft Corporation):
  - **HEVC Video Extensions — USD 0.99** — current product id `9nmzlz57r3t7`.
    SURPRISE/DISCREPANCY: the long-standing id `9N4WGH0Z6VHM` now returns HTTP 410 /
    "Product not found" (verified both via displaycatalog.mp.microsoft.com empty Products[] and
    the rendered store page) — **old deep links and older documentation referencing that id are
    dead**; anything documenting HEVC acquisition must cite the new id or a Store search.
    The "HEVC Video Extensions from Device Manufacturer" (free, OEM-preinstall, historically
    unlisted) does not appear in search results — assumed still unlisted/OEM-only [VERIFY].
  - **AV1 Video Extension — free** — `9mvzqvxjbq9v` (store page live, title verified).
  - **VP9 Video Extensions — free** — `9n4d0msmp0pt` (live).
  - **Web Media Extensions (VP9/Opus/Vorbis) — free** — `9n5tdp8vcmhs`.
  - **MPEG-2 Video Extension — free** — `9n95q1zzpmh4`.
  - **AVC Encoder Video Extension — free** — `9pb0trcnrhfx` (encoder-side; noted for completeness).
- Distribution reality: Win10 1709+ preinstall VP9/Web Media Extensions and many OEM boxes ship
  HEVC via Device Manufacturer variant, but **nothing can be assumed**: the engine must
  feature-detect at runtime (MFTEnumEx) and surface an install prompt (app-execution alias/msix
  protocol to launch the Store listing) for HEVC/AV1. [VERIFY] preinstall matrix on the Phase 0
  hardware matrix doc; decision on shipping a bundled HEVC fallback belongs to ADR-001 (note:
  bundling the MFT is not permitted; only Store acquisition or software fallback).

### 4.2 API surface (names only — public Win32/COM names, clean-room safe)

- Startup: `MFStartup`, `MFShutdown` (mfapi). Verified doc URLs (HTTP 200, 2026-10-19).
- Decoder discovery/creation: `MFTEnumEx` (MFT_CATEGORY_VIDEO_DECODER with
  MFT_ENUM_FLAG_HARDWARE / MFT_ENUM_FLAG_SYNCMFT etc.) → `IMFActivate` →
  `IMFActivate::ActivateObject(IID_IMFTransform)`; or direct `CoCreateInstance` on known in-box
  decoder CLSIDs. NOTE: **`MFCreateVideoDecoder` does not exist** (learn.microsoft.com 404,
  checked 2026-10-19) — the task brief's assumed name is corrected to the MFTEnumEx/IMFActivate
  pattern.
- Decode loop: `IMFTransform::SetInputType/SetOutputType/ProcessInput/ProcessOutput`,
  `IMFSample`, `IMFMediaType` (NV12/P010 output), `MFCreateSample`/`MFCreateMediaBuffer`.
- Hardware/zero-copy: `MFCreateDXGIDeviceManager` → `IMFDXGIDeviceManager`, decoder message
  `MFT_MESSAGE_SET_D3D_MANAGER`, output `ID3D11Texture2D` samples; `MFCreateDXGISurfaceBuffer`.
  All referenced doc pages verified live (nf-mfapi-mfcreatedxgidevicemanager, nn-mftransform-imftransform,
  medfound/h-264-video-decoder).
- In-box H.264 decoder page: https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-decoder
  (DXVA 1.0/2.0 accelerated; profiles up to High).

### 4.3 Rust access (E2-class)

- Primary: `windows` 0.62.2 — feature `Win32_Media_MediaFoundation = ["Win32_Media"]` verified
  present in its Cargo.toml (as are `Win32_Graphics_Direct3D11`, `Win32_Graphics_Dxgi`,
  `Win32_System_Com` needed for the DXVA interop). cpal 0.18.2 already builds against windows
  0.62 + windows-core 0.62 (verified), so one generation of windows-rs can serve MF + WASAPI.
- `windows-sys` 0.61.2 available where raw-C style is preferred (smaller compile, no COM
  ergonomics). The 0.100.0 generation (windows-core 0.100.0 etc.) is published for the new app
  crates; pick one generation per ADR-002 audit.
- No unsafe-free guarantee: platform adapter will contain `unsafe` blocks behind the
  platform-adapter ADR + invariants (AGENTS §3.3) — windows-rs COM calls are `unsafe` by nature.

## 5. Audio: WASAPI via windows-rs (pure-Rust path) — ADR-005 input

- cpal 0.18.2 (2026-08-16) implements its WASAPI host directly on `windows` 0.62 /
  `windows-core` 0.62 (verified from its Cargo.toml: features Win32_Media_Audio,
  Win32_Media_KernelStreaming, ...). No C library involved → E2-class.
- Verified implementation details in `cpal-0.18.2/src/host/wasapi/device.rs`:
  - shared mode only: `AUDCLNT_SHAREMODE_SHARED` at stream build;
  - **event-driven**: `AUDCLNT_STREAMFLAGS_EVENTCALLBACK` is the default flag, paired with
    `IAudioClient::SetEventHandle` — the callback wakes on buffer events, i.e. the event-driven
    design the audio-output subsystem wants exists as a proven Rust pattern to copy (behavior,
    not code — cpal is MIT/Apache-2.0 [VERIFY license choice with doc 11]).
  - Exclusive/bit-exact mode is NOT provided by cpal's WASAPI host (only via the optional
    `asio-sys` 0.4 backend) → our own exclusive-mode path (IAudioClient2/IAudioClient3 with
    `AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOLOOPBACK`-style low-latency
    initialization, `IAudioRenderClient`, `IAudioClock` for drift) would be written directly on
    windows-rs if ADR-005 requires it [VERIFY] whether exclusive mode is in scope.
- API names involved: `IMMDeviceEnumerator`, `IMMDevice`, `IAudioClient`/`IAudioClient2`/
  `IAudioClient3`, `IAudioRenderClient`, `IAudioClock`, `IAudioSessionControl`,
  `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`, `AUDCLNT_SHAREMODE_*`. Pro audio thread priority: cpal
  uses optional `audio_thread_priority` 0.35 (Microsoft-rustaudio crate) [VERIFY classification
  if adopted].
- Latency expectation: shared-mode event-driven typically ~10-20 ms buffers; IAudioClient3
  low-latency can reach ~<10 ms with small buffers on supported drivers [VERIFY on hardware].

## 6. wgpu / DX12 on Windows

- wgpu 30.0.1 (2026-08-22): default features include `dx12` (plus metal/gles/vulkan, wgsl,
  webgpu) — verified in wgpu-30.0.1/Cargo.toml. DX12 backend via wgpu-hal (native, no C deps).
- Surface creation from a tao/winit/Tauri window through raw-window-handle 0.6 → HWND →
  DX12 swapchain; with transparent windowed Candidate A the swapchain composites under the
  WebView2 child HWND (subject to the WS_CLIPCHILDREN caveat, §2.3).
- Interop risk for ADR-001: Media Foundation hardware decoders emit **D3D11** textures
  (ID3D11Texture2D) while wgpu 30 on Windows renders via **D3D12**. Zero-copy requires
  D3D11↔D3D12 shared-resource paths (shared NT handles / keyed mutex) or a D3D11-based blit/copy
  hop. This is the single biggest integration unknown on Windows; options:
  (a) copy D3D11 → CPU → wgpu staging (simple, costs a copy);
  (b) shared handle import into wgpu-hal DX12 texture [VERIFY wgpu 30 hal support for importing
  D3D11-shared textures — wgpu exposes `hal` texture creation from raw handles only in limited
  forms; exact API to be pinned in ADR-001 spike];
  (c) run the presentation path on D3D11 directly for the video surface (abandoning wgpu for the
  video quad) [PROPOSAL — out of scope unless ADR-003 decides].
- Alternative: `windows` 0.62 features `Win32_Graphics_Direct3D11` + `Win32_Graphics_Dxgi` are
  all present for whichever interop path is chosen.

## 7. Verdict block (evidence summary — NOT an ADR decision)

ADR-003 (video-in-webview) Windows feasibility:
- Candidate A (wgpu surface behind transparent webview): **feasible with conditions**.
  Documented transparent-background support exists (put_DefaultBackgroundColor, alpha 0/255 only);
  the "host app content shows through" behavior is explicitly documented by Microsoft. Conditions:
  parent window without WS_CLIPCHILDREN (winit has the switch; tao does not expose it —
  [VERIFY] tao defaults on hardware, or switch the host windowing to winit, or reparent trick);
  accept the white-flicker startup artifact (mitigate via WEBVIEW2_DEFAULT_BACKGROUND_COLOR env
  var); [VERIFY] backdrop-filter/CSS effects over transparent regions (wry #408 lineage).
  Visual/composition hosting is the robust long-term path (true z-order, no child-HWND issues)
  but is unimplemented in wry and Microsoft's new windows-webview, and costs input forwarding +
  rasterization-scale handling — treat as a Phase 2+ escalation if windowed mode shows artifacts.
- Candidate B (WebCodecs frame streaming): **available today** in evergreen WebView2 (Chromium
  ≥94; not in WebView2's disabled-features list). Caveats: HEVC availability inherits the §4
  store-extension problem; per-frame copy/composite cost vs Candidate A unknown until measured;
  [VERIFY] WebGPU importTexture fast path.
- Comparative note: field evidence (wry #1540) shows Windows is the platform where transparent
  overlay is *hardest* (child-HWND compositing) while macOS handled the same scenario cleanly —
  the opposite of the naive expectation.

ADR-001 (hw decode) Windows path readiness:
- Media Foundation is fully reachable from pure Rust via windows-rs (E2-class; feature flags
  verified). In-box: H.264 solid (decoder MFT + DXVA). HEVC/AV1/VP9 = Store-extension-dependent
  → runtime feature detection (MFTEnumEx) + user-facing install flow MUST be designed; HEVC is
  paid ($0.99, new product id 9nmzlz57r3t7) on clean installs [VERIFY preinstall rate on target
  hardware matrix]. Decoder MFT output is D3D11 — the D3D11↔D3D12/wgpu interop is the top open
  technical risk ([VERIFY] wgpu 30 import path in an ADR-001 spike).
- API-name correction for the ADR text: use MFTEnumEx/IMFActivate/IMFTransform, not the
  non-existent "MFCreateVideoDecoder".

Open items [VERIFY] (Windows):
1. tao 0.37.1 default window styles re WS_CLIPCHILDREN on real Windows 10/11 (Phase 2 prototype).
2. WebView2 WebCodecs: HEVC decode behavior on clean Win11 (with/without HEVC extension) and
   WebGPU VideoFrame import in the pinned runtime version.
3. wgpu 30.0.1 surface API shape (SurfaceTarget) + D3D11-shared-texture import capability.
4. windows-rs generation choice (0.62.x vs 0.100.x) — dependency-audit/doc-11 decision.
5. HEVC extension preinstall rate on the Phase 0 hardware matrix; "Device Manufacturer" variant
   current availability (unlisted) and licensing implications of directing users to the $0.99
   listing (doc 11 §9 territory).
6. Old-vs-new HEVC store id documentation drift (9N4WGH0Z6VHM → 9nmzlz57r3t7) — propagate to
   ADR-001 draft and user-facing docs.
7. WASAPI exclusive-mode scope for ADR-005 (bit-exact passthrough for audio?).
8. Startup white-flicker mitigation actually working with WEBVIEW2_DEFAULT_BACKGROUND_COLOR in a
   Tauri-packaged app (env var must be set before WebView2 environment creation — process-wide).
