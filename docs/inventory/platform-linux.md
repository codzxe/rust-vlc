# Platform Inventory — Linux (Task 0.C5-a)

- Scope: ONE platform record for Linux (desktop) feeding ADR-001 (hw decode), ADR-003 (video-in-webview), ADR-005 (audio backend).
- Author/agent: 0.C5-a (split retry of timed-out 0.C5-I). Date of all evidence gathering: **2026-10-19** (all URLs accessed this date unless noted).
- Method honesty: this sandbox (Debian 13 trixie, x86_64, headless) has **no WebSearch/WebFetch tools**; all web evidence gathered via `curl` (recorded in `tmp/linuxaudit/`, git-ignored). GitHub REST API core quota (60/h, shared IP) was exhausted mid-run; remaining GitHub facts pulled from public HTML pages. GNOME/freedesktop GitLab instances are behind anti-bot (Anubis) and could **not** be queried — WebKit-upstream issue citations are therefore Tauri/wry-side only; flagged as [VERIFY] follow-ups where relevant.
- No VLC source used; no ADR decisions made here (evidence only). `[VERIFY]` = unconfirmed; `[PROPOSAL]` = beyond brief.
- **Executive summary (TL;DR)**: On Linux the webview is WebKitGTK and only WebKitGTK (Tauri v2 → wry 0.57 → webkit2gtk **4.1/GTK3**; current engine series 2.54, distros ship 2.50–2.54). Candidate A (wgpu surface behind a transparent webview) has **no supported embedding path on Wayland** in wry today (child webviews are X11-only) plus current open bugs on NVIDIA and on transparent Wayland surfaces; Candidate B (frame streaming to canvas) is structurally safer, with WebCodecs **proven compiled-in** to Debian's 2.52.6 binary (GStreamer-backed, prefer-hardware honored since 2.48) — only its default-enabled state is unverified. VA-API is the only real hw-decode API (libva 2.22; Intel/AMD/NVIDIA-shim cover H.264/HEVC/VP9/AV1 decode); every Rust binding (cros-libva lineage) wraps the same C ABI, leaving the OS-service-vs-C-library classification as the open ADR-001 policy question. Audio: PipeWire is the distro reality; only the `pulseaudio` crate is pure Rust; cpal 0.18 has native ALSA/JACK/PipeWire/PulseAudio backends. wgpu 30 is pure Rust over Vulkan (lavapipe as software fallback).

## 1. WebKitGTK reality check

### 1.0 Framing: "the Linux webview" is exactly one engine

- MASTER-PROMPT §7 requires testing WebView2, WKWebView and WebKitGTK separately. On Linux the field collapses: **WebView2 is Windows-only; WKWebView is Apple-only; the only production webview is WebKitGTK** (GTK port of WebKit, maintained by an Igalia-led team). Tauri does not bundle or fork it — it links the system library. Therefore the Linux platform record = the system-WebKitGTK reality below, plus whatever wry/Tauri do on top.
- Related but distinct: WPE WebKit (embedded port) and cog exist but are not Tauri targets [VERIFY not planned in doc 08].

### 1.1 Version landscape and API series

| Item | Fact | Source |
|---|---|---|
| Current stable series | **2.54.x** (2.54.0 released 2026-09-16; 2.54.1 2026-10-02) | webkitgtk.org homepage + news index |
| Previous stable series | 2.52.x (2.52.0 2026-03-18 … 2.52.6 2026-08-19); 2.50.x before that (2.50.6 2026-03-12) | webkitgtk.org/news.html index |
| Cadence | ~2 stable series/year, 5–7 micro-releases each | news index |
| API series **4.0** | GTK3 + libsoup2. **Dead**: 2.52.0 is "the first stable release to ship without support for libsoup 2" (announced 2025-10-07; "applications that use the webkit2gtk-4.0 API can easily migrate to the 4.1 one") | https://webkitgtk.org/2025/10/07/webkitgtk-soup2-deprecation.html |
| API series **4.1** | GTK3 + libsoup3 (`libwebkit2gtk-4.1-0`); what Tauri/wry target today | Debian trixie apt (`2.52.6-1~deb13u1` main / `2.54.0-1~deb13u2` security) + wry Cargo.toml (below) |
| API series **6.0** | GTK4 + libsoup3 (`libwebkitgtk-6.0-4`, also `webkit2gtk-driver`); co-packaged at same versions | Debian trixie apt-cache (local) |
| Maintenance | Engine developed by Igalia-led WebKit team; security advisories ongoing (WSA-2026-0005/0006) | webkitgtk.org/security.html |

Rendering-stack timeline (each from the release posts at webkitgtk.org):
- **2.42.0** (2023-09-15): "New renderer based on **DMA-BUF** shared buffers" introduced.
- 2.42.3 (2023-12-05): "Fix flickering while playing videos with DMA-BUF sink" (the flicker era).
- **2.44.0** (2024-03-16): "**Removed the X11 and WPE renderers in favor of DMA-BUF**" — DMABUF became the *only* accelerated compositing path; escape hatches are env vars only.
- **2.46.0** (2024-09-17): **Skia replaces Cairo** as 2D backend; MotionMark "up to four times better (4x) on powerful desktops with discrete GPUs", 2x on iGPU laptops.
- **2.48** (2025-03-14 + notes 2025-04-08): GPU worker threads with Skia (default); GStreamer-GL sink handles DMA-BUF buffers; WebCodecs `prefer-hardware` honored (§2).
- 2.50 (2025-09-17): threaded rendering switched to Skia API; Managed Media Source enabled; XDG portal capture.
- 2.52 (2026-03-18): libsoup2 removed; damage propagation in non-composited mode; Audio Output Devices API (off by default).
- **2.54** (2026-09-16): **Skia-based compositor replaces TextureMapper**; Cairo option removed entirely; damage-aware compositing by default; "Skia can target both OpenGL and Vulkan, so the compositor no longer stands in the way of **Vulkan-based rendering in the future**". WPE 2.54 (shares compositor): MotionMark +36% (+45% composition variant) on Raspberry Pi 4.

### 1.1.1 Distro availability matrix (what users actually run)

System-library reality — the app cannot pin WebKitGTK; it inherits the distro's security updates:

| Distro | webkit2gtk (4.1) shipped | Source (accessed 2026-10-19) |
|---|---|---|
| Debian 13 trixie | **2.52.6** (main) / **2.54.0** (security) | local `apt-cache policy` |
| Ubuntu 26.04 LTS "resolute" | 2.52.6 | Launchpad API getPublishedSources |
| Ubuntu 25.10 "questing" | 2.52.3 | Launchpad API |
| Ubuntu 24.04 LTS "noble" | 2.52.6 (security-updated) | Launchpad API |
| Ubuntu 25.04 "plucky" | 2.50.4 | Launchpad API |
| Ubuntu 22.04 "jammy" | 2.50.4 (ESM-era update) | Launchpad API |
| Fedora / Arch | [VERIFY via Bodhi / Arch package API — not queried this run] | — |

Engineering consequence: our floor across supported distros is realistically **WebKitGTK ≥ 2.44–2.46** (DMABUF-only renderer guaranteed; Skia 2D guaranteed on everything except Ubuntu 20.04 "focal" = 2.38.6, which is EOL for standard support). Anything we ship must tolerate 2.52-era and 2.54-era compositors simultaneously.

### 1.2 Tauri's Linux webview = wry → WebKitGTK 4.1

- wry **0.57.0** (crates.io, updated 2026-09-08; repo github.com/tauri-apps/wry). Linux deps from `Cargo.toml` @ tag `wry-v0.57.0`: `webkit2gtk = "=2.0.2"` (feature `v2_38`; `v2_40` behind `linux-body` feature), `webkit2gtk-sys =2.0.2`, `javascriptcore-rs =1.1.2`, `gtk 0.18`, `soup3 0.5`, `x11-dl`, `gdkx11` → **GTK3 / WebKitGTK 4.1 API, not 6.0**.
- Tauri v2 (crates.io 2.12.4, 2026-10-14) prerequisites page: Debian `sudo apt install libwebkit2gtk-4.1-dev …`; Arch `webkit2gtk-4.1 …` (https://tauri.app/start/prerequisites/). tauri 3.0.0-alpha.5 exists (2026-10-14); wry PR #1767 "Migrating to gtk4/webkit6" (community) is **still open** → 4.1/GTK3 remains the shipping Linux stack. [VERIFY: tauri 3.0 webview choice before we pin.]
- Transparency API: `WebViewBuilder::with_transparent` exists cross-platform; on Linux implemented as `webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.))` plus `background_color` support (wry `src/webkitgtk/mod.rs` lines 292–305, 859–867 @ wry-v0.57.0). Also `set_background_color` runtime API.
- **Child webviews (`build_as_child`) are Linux-X11-only** in wry 0.57: README states "supported on macOS, Windows and Linux (X11 Only)"; source `new_as_child` → `new_x11` with `#[cfg(not(feature = "x11"))] → Err(UnsupportedWindowHandle)`. On Wayland the only embedding path is `WebViewBuilderExtUnix::build_gtk` into a GTK window/widget. Architecturally decisive for Candidate A (§5).
- wry itself tests against `wgpu 23` + `winit 0.30` (dev-dependencies) — GPU interop is on their radar but no overlay-behind-webview API exists.
- Transparency caveats are NOT applied automatically: grep of wry 0.57 `src/webkitgtk/mod.rs` and tauri-runtime-wry 2.12.4 `lib.rs` finds **no `WEBKIT_DISABLE_*` env-var handling** — workarounds are user-applied, not library-applied. [VERIFY tao/wry full tree + Tauri docs recommendations.]

### 1.3 Transparency + compositing artifacts (issue evidence)

All GitHub, accessed 2026-10-19. Numbers verified via GitHub search API / issue pages:

| ID | State | Finding |
|---|---|---|
| tauri#14924 | open (2026-02-10) | Linux/NVIDIA: `"transparent": true` triggers crash ("Failed to create GBM buffer", Error 71) or artifacts (black corners, ghosting) on 40xx/50xx + driver 590+; "conflict between WebKitGTK's DMABUF renderer and Nvidia's driver when handling transparent surfaces". https://github.com/tauri-apps/tauri/issues/14924 |
| wry#1731 | open | Transparent **Wayland** layer-shell windows: `WebKitWebProcess` accumulates `WebKitSharedMemory` (wl_shm) buffers unboundedly (~100 KB/s at 1–3 repaints; 2034 bufs / ~153 MB after 15 min) because `wl_buffer.release` never arrives (Hyprland; not Sway); `WEBKIT_DISABLE_DMABUF_RENDERER=1` **no effect**; non-transparent unaffected. https://github.com/tauri-apps/wry/issues/1731 (+ related #1524 ghost pixels) |
| tauri#9304 / tauri#10702 | open | NVIDIA Linux window-creation failures; Error 71 Wayland protocol error. |
| tauri#15936 | open | Blank window under Xvfb until `WEBKIT_DISABLE_COMPOSITING_MODE=1`, no diagnostics. |
| tauri#16226 | open PR | "warn about webkit2gtk **2.44 Wayland DMA-BUF launch failure** at startup" (2.44-specific Wayland regression class). |
| wry#1366 / wry#1727 | open | wry cannot create windows on Arch+NVIDIA; text blur after resize on Wayland. |
| tauri#13270 | open | `transparent: true` *improves* resize performance / removes black-white bar artifacts (compositing-mode behavior differences). |
| wry#1540 | closed | Transparent webview also punches through content behind it (Windows case) — semantics of "behind" differ per platform. |
| tauri#9394 | open | "[docs] Documenting Nvidia problems in Tauri" — collects WebKitGTK+NVIDIA failures/workarounds. |

- Workaround prevalence: GitHub issue search `repo:tauri-apps/tauri WEBKIT_DISABLE_DMABUF_RENDERER` → **44 issues** (2026-10-19). The env-var escape hatches (`WEBKIT_DISABLE_DMABUF_RENDERER`, `WEBKIT_DISABLE_COMPOSITING_MODE`) are the standard mitigations, both of which **disable the accelerated path** (software rendering) — unacceptable as a permanent solution for a video player, acceptable as user-facing fallback toggle.
- [VERIFY follow-up: GNOME WebKit GitLab upstream issues for transparency/DMABUF (blocked by anti-bot in this sandbox); specifically whether transparent GDK windows under DMABUF rendering are composited GPU-side on X11.]

### 1.4 Accelerated compositing vs a wgpu surface BEHIND the webview (Candidate A)

- Since 2.44 the accelerated compositor is DMABUF-only; since 2.54 it is Skia-based and can target GL or Vulkan. The webview's final frame is shared to the display server as DMA-BUF buffers.
- For A to work, the webview must be alpha-composited over our wgpu surface in the same window region. Achievable layering mechanisms:
  1. **GTK-internal stacking** (webview GdkWindow above a GDK-owned render surface in one GtkWindow) — the only mechanism available on **both** X11 and Wayland given wry's `build_gtk`; wgpu would need a surface bound to a GDK-owned wl_surface/xid (Vulkan WSI via raw-window-handle of the GTK widget) or the GL backend inside `GtkGLArea`. No wry/wgpu example of this exists; untested combination. [VERIFY by spike.]
  2. **Native child windows** — supported by wry only on X11 (`build_as_child`); on Wayland there is no cross-toolkit child-window embedding standard in wry today.
  3. **Two stacked toplevels** — X11 override-redirect/sibling tricks are WM-fragile; Wayland deliberately forbids client-controlled global stacking (except layer-shell on wlroots compositors, non-portable).
- NVIDIA is the documented weak point for transparent WebKitGTK surfaces (§1.3 tauri#14924); Wayland has an open unbounded-memory bug class on transparent webview surfaces (wry#1731). Both are **current, open** as of 2026-10.

### 1.5 Performance reputation

- Post-2.46 Skia era: large, measured wins (2.46 notes: up to 4x MotionMark desktops/2x iGPU; 2.54 notes: +36–45% MotionMark on RPi4-class hardware, less GPU busy, damage-aware compositing). WebKitGTK's CPU-side reputation has historically lagged Chromium (community consensus, not measured here) [VERIFY with own MotionMark runs on target HW].
- 2.48+ moves tile painting off the main thread (GPU worker threads); 2.52+ run-loop scheduled layer flush; 2.54 damage-restricted draws.
- For a video UI (mostly static chrome + one changing region), damage-aware compositing (2.54) is directly relevant: a canvas repainting at 24–60 fps will be treated as persistent damage → full repaint cost per frame. [VERIFY: canvas/video frame painting path cost on 2.52 vs 2.54.]

## 2. Candidate B on Linux — frame streaming into the webview

### 2.0 Transport surface available for B (wry, verified in source @ wry-v0.57.0)

- `WebViewBuilder::with_custom_protocol(name, handler)` (+ asynchronous variant; `mod custom_protocol_workaround` exists in the tree) — binary frame transport from engine to JS without touching disk/network.
- `with_ipc_handler` — JS→Rust command channel (JSON string messages, no binary).
- `with_initialization_script(_for_main_only)` — install the player bootstrap/bridge script.
- `evaluate_script` / `evaluate_script_with_callback` — Rust→JS calls (imperative; not for per-frame data).
- Consequence: B's hot path should be custom-protocol pulls (fetch/video src) driven by JS, with IPC only for control-plane (play/pause/seek), keeping the GTK main loop out of the frame path where possible. [VERIFY actual threading of custom-protocol handlers on the GTK backend — see §2.3 latency notes.]

### 2.1 WebCodecs support status in WebKitGTK

- **Compiled into the Debian trixie build — verified in-binary**: `apt-get download libwebkit2gtk-4.1-0=2.52.6-1~deb13u1` (24.2 MB) → `dpkg-deb -x` → `strings` over `libwebkit2gtk-4.1.so.0.21.10` (96 MB) contains: runtime feature flags `WebCodecsVideoEnabled`, `WebCodecsAudioEnabled`, `WebCodecsAV1Enabled`, `WebCodecsHEVCEnabled` ("Enable WebCodecs …" descriptions); JS interface names `VideoDecoder`(14), `AudioDecoder`(13), `VideoEncoder`(16), `AudioEncoder`(15), `EncodedVideoChunk`(9), `EncodedAudioChunk`(9), `VideoFrame`(98), `ImageDecoder`(3); 79 "WebCodecs" strings total. (Raw evidence: tmp/linuxaudit/wk-strings.txt.)
- Upstream behavior confirmation — WebKitGTK 2.48 release notes (2025-04-08): "Video handling using the **WebCodecs API** no longer ignores the **prefer-hardware** option. It is used as a hint to attempt using **hardware-accelerated GStreamer components**. If that fails, software based codecs will be used as fallback." → the GTK port's WebCodecs is GStreamer-backed and can hit VA-API via GStreamer plugins. The binary also contains `WEBKIT_GST_ENABLE_LEGACY_VAAPI` (selects legacy `vaapi` vs newer `va` GStreamer element). Debian trixie ships `gstreamer1.0-vaapi 1.26.2-1` (local apt) [VERIFY: whether the newer `va` plugin is in trixie's gst-plugins-bad].
- **Default-enabled state not proven**: presence of runtime feature-flag names suggests per-part toggles exist. [VERIFY on hardware: `WebKitFeatureList`/`webkit_settings_set experimental features` probe, or a JS `typeof VideoDecoder` probe under WebKitGTK 2.52/2.54.] Safari history: WebCodecs shipped in WebKit for Cocoa ports first (Safari 16.4+); GTK/WPE enablement is GStreamer-dependent (2.48 notes imply usable state).

### 2.2 MSE / other channels

- Classical **MediaSource**: long-standing in the GTK port (GStreamer pipeline); 2.50 (2025-09-17 notes) "**Managed Media Source support has been enabled**", which "allows the user agent to react to changes in memory and CPU availability". Binary strings: `MediaSource` (120), `ManagedMediaSource` (11).
- `ImageBitmap` / `createImageBitmap` + canvas 2D `putImageData`/`drawImage`: always available (no feature flag risk); requires full-frame RGBA copies through JS per frame — the most CPU-expensive B variant but the most portable.
- Realistic B-path ranking for Linux (evidence-based, decision belongs to ADR-003): (1) WebCodecs `VideoDecoder` fed over custom protocol + `VideoFrame`→canvas, if runtime-enabled [VERIFY]; (2) MSE with fMP4 segments pushed via `SourceBuffer.appendBuffer` (engine must remux demuxed packets to fMP4 in Rust — permitted, engine-side); (3) raw RGBA frames to canvas via ImageBitmap (no enablement risk, highest CPU/GC cost). All three keep the video *inside* the webview → no transparency layering needed at all (this is B's structural advantage over A on Linux).

### 2.3 The realistic B-path on Linux (engineering detail)

Data-flow options, best-first (decision is ADR-003's):

1. **WebCodecs** (if runtime-enabled [VERIFY]): Rust engine demuxes/decodes-header-parses → encoded chunks shipped over a Tauri custom protocol (or IPC channel) → JS `VideoDecoder.configure({codecs:'avc1…', hardwareAcceleration:'prefer-hardware'})` → `VideoFrame` drawn to a canvas. Hardware path rides WebKitGTK's GStreamer pipeline into gstreamer1.0-vaapi → libva → driver (§2.1). No transparency/window tricks needed; alpha and CSS compositing stay inside the web.
2. **MSE**: engine remuxes elementary streams to fMP4 segments in Rust (permitted — engine-side, no C deps) and feeds `MediaSource`/`SourceBuffer.appendBuffer` via JS. Older but ubiquitous; codec set limited by what the GStreamer pipeline accepts (H.264/HEVC/VP9/AV1 all plausible) [VERIFY per-codec MSE support in 2.52].
3. **ImageBitmap/canvas 2D**: engine decodes (hw or sw) to RGBA, transfers frames over custom protocol (binary body), JS `createImageBitmap(ImageData)` → `drawImage`. Zero feature risk, highest CPU cost (full-frame copy + JS GC churn per frame at 24–60 fps). Fine for MVP-gate verticals (MJPEG-class), painful for 4K HEVC.

Latency/ordering notes (all [VERIFY] by benchmark on target HW):
- Custom-protocol round-trip adds a per-frame hop (wry custom protocols answer via the GTK main loop on Linux).
- WebCodecs keeps frames in-GPU (VideoFrame is texture-backed) → cheapest display path when available.
- MSE hands buffering/buffer-policy to the web layer — least control for A/V sync, which doc 12 §3 requires us to own.

## 3. Hardware decode — VA-API

### 3.1 What it is

- "VA-API is an open-source library and API specification, which provides access to graphics hardware acceleration capabilities for video processing. It consists of a **main library and driver-specific acceleration backends** for each supported hardware vendor." (intel/libva README, https://github.com/intel/libva — accessed 2026-10-19.) I.e. `libva.so.2` (userspace, C ABI) + per-vendor driver DSOs (`<name>_drv_video.so`) it discovers/loads [VERIFY exact loader mechanics: driver registry + `LIBVA_DRIVER_NAME`], over DRM.
- Linux graphics-stack position: VA-API is the standard freedesktop video-acceleration interface; drivers ship with the graphics stack itself (Mesa, intel-media-driver) or the vendor. Debian trixie: `libva2 2.22.0-3`, `libva-drm2`, `libva-x11-2`, `libva-wayland2` (all 2.22.0-3; libva2 + drm/x11 variants **installed in this sandbox** — pulled in by Mesa). `libva-utils`/`vainfo` for probing.
- **Policy note (recorded, not decided)**: libva is a *userspace C library*, not a kernel API — whether it counts as an "OS-service binding (system graphics stack)" under §9 E2, or fails as a forbidden third-party C library, is the core ADR-001 question. Both framings have evidence (§6.2).

### 3.2 Codec coverage per driver class (upstream docs, accessed 2026-10-19)

| Driver class | Package (trixie) | Codecs | Source |
|---|---|---|---|
| Intel `iHD` (media-driver, Gen9+; classic `i965` for Broadwell–Skylake) | `intel-media-va-driver 25.2.3+dfsg1-1`; `i965-va-driver 2.4.1` | Decode: **H.264, HEVC (8/10/12-bit incl. 422/444), VP9, AV1** (from DG1/TGL-class onward per matrix), VVC 8/10 (newest gens), MPEG-2, VC-1, JPEG, VP8. Encode also available for H.264/HEVC/VP9/AV1 (gen-dependent; HuC firmware needed for enc rate control) | https://github.com/intel/media-driver README codec matrix |
| AMD via Mesa Gallium VA state-tracker (`radeonsi` on UVD/VCN blocks) | `mesa-va-drivers 25.0.7-2+deb13u1` | H.264/HEVC decode broadly; VP9, AV1 on VCN-capable ASICs (RDNA2+ for AV1) [VERIFY per-ASIC matrix upstream — Mesa docs restructured, could not cite exact page this run] | docs.mesa3d.org (envvars confirm VA state tracker); [VERIFY] |
| NVIDIA — `nvidia-vaapi-driver` (VA-API **shim over NVDEC**) | distro `libnvidia-vaapi1` [VERIFY trixie packaging] | **Decode-only** (encode unsupported): AV1, H.264, HEVC, VP8, VP9, MPEG-2, VC-1; **not** MPEG-4 (bitstream insufficiency) or JPEG. "Specifically designed to be used by Firefox… may not operate correctly in other applications." Requires NVIDIA driver 470 or 500+. Experimental "direct backend" hits NVIDIA's unstable kernel API | https://github.com/elFarto/nvidia-vaapi-driver README |

Practical readout for the four target codecs (H.264/HEVC/VP9/AV1): all three vendor classes cover all four for **decode** (Intel native; AMD via Mesa; NVIDIA via shim with caveats) — matching what a Linux user gets in Firefox/mpv today.

### 3.3 Rust access options (crates.io, accessed 2026-10-19)

| Crate | Ver / updated | Downloads | Repo / maintainer | Purity / linkage | Notes |
|---|---|---|---|---|---|
| **cros-libva** | 0.0.13 / 2024-12-06 | **1,277,658** | github.com/chromeos/cros-libva (The ChromiumOS Authors, BSD-3-Clause) | **Not pure Rust**: build.rs runs **bindgen** against installed `va.h`/`va_drm.h` + pkg-config; links native libva (≥1.20) at link time | "Safe bindings… developed for use in ChromeOS, but has no ChromeOS specifics"; most battle-tested (ChromeOS CI) |
| libva | 0.1.5 / 2026-10-17 | 1,040 | github.com/mlm-games/cros-libva (fork) | same approach | actively-published fork of cros-libva |
| cros-libva-extended | no stable / 2026-08-06 | 4,833 | chojs23 fork | same | fork w/ extensions |
| vacc-libva | 0.1.0 / 2026-10-12 | 19 | andreytkachenko fork | same | "VAAPI-decoder fixes (zero-in…)" |
| libva-sys | 0.1.2 / 2021-08-25 | 13,857 | vgarleanu | sys FFI to libva **and libva-utils** | stale (5 yrs) |
| fev | 0.2.3 / 2024-05-29 | 10,186 | — | high-level bindings | low recent activity [VERIFY] |
| Higher-level codec crates | moq-vaapi 0.1.0 (desc says "(AI GENERATED)" — treat with caution), cros-codecs-extended / cros-codecs-generic-vaapi, nuxodecs 0.1.9, oxideav-vaapi 0.0.3, vacc (Vulkan/NVDEC/VAAPI unified), rawshift-hwdec | | | | H.264/HEVC/VP9/AV1 decode implemented on top of the above bindings |

**No pure-Rust VA-API exists or is possible** (the API *is* a C ABI); the choice is which binding wrapper (or our own audited FFI module). The dominant lineage is cros-libva (ChromeOS upstream, forked by mlm-games/libva, chojs23, andreytkachenko).

### 3.4 Raw dlopen/FFI option — policy question (recorded, not decided)

- Alternative to a link-time sys crate: `libloading`-based **dlopen of system `libva.so.2`** with hand-declared FFI. Arguments heard in the repo policy so far, restated neutrally:
  - *For OS-service classification*: VA-API is the freedesktop system video-acceleration service (analogous to the Vulkan loader, XCB/Wayland client libs, ALSA lib) — drivers are part of the OS graphics stack (Mesa/intel-media-driver/NVIDIA), the API is a stable spec with backward-compat guarantees ("No code changes may be introduced that would regress support for existing API" — libva README), and VLC/mpv/Firefox all consume it as a system service.
  - *Against*: it is a userspace library, not a kernel interface; dlopen vs link-time does not change what is being called; a hand-rolled FFI surface is unaudited surface area vs an audited binding crate.
  - Either way, an E2-class ADR listing exact symbols/entrypoints (vaInitialize/vaCreateConfig/vaCreateContext/vaBeginPicture/vaRenderPicture/vaEndPicture/vaSyncSurface/vaCreateSurfaces + va_drm) would be required, plus the DMA-BUF/prime export path for zero-copy into the renderer [VERIFY symbol list against va.h 2.22].
- **VDPAU legacy note**: `libvdpau1 1.5-3+b1` present in trixie (installed in sandbox); the VA↔VDPAU bridge `vdpau-va-driver` is **gone from trixie** (apt Candidate: none). VDPAU remains only as NVIDIA's native path + Mesa state tracker; the ecosystem (Firefox, Chromium, GStreamer, VLC 4 direction) has consolidated on VA-API. Treat VDPAU as out-of-scope for the new engine (VLC 3.0.23 still ships vdpau modules — see 0.A2 baseline dumps — but that is baseline observation, not a requirement).

### 3.5 Programming-model summary (what an E2 adapter would touch)

- Init: open DRM render node (`/dev/dri/renderD128`) → `vaGetDisplayDRM` → `vaInitialize`; query `vaQueryConfigEntrypoints` for `VAEntrypointVLD` (slice-level decode) per `VAProfileH264*`/`HEVC*`/`VP9*`/`AV1*`; `vaCreateConfig`/`vaCreateContext`; per-frame `vaBeginPicture` → `vaRenderPicture` (slices) → `vaEndPicture` → `vaSyncSurface`. Surfaces: `VA_SURFACE_ATTRIB_MEM_TYPE_DRM_PRIME_2` export → DMABUF fd for zero-copy hand-off to a Vulkan external-memory import in wgpu. [VERIFY exact 2.22 API spelling against va.h + cros-libva surface bindings; the symbol list for the ADR-001 exception ledger must be exact.]
- Runtime discovery: `vainfo` (libva-utils) enumerates profiles/entrypoints — our capability-probe design can mirror it. Driver selection via `LIBVA_DRIVER_NAME` fallback (iHD vs i965 vs Mesa vs nvidia shim).
- **Oracle corroboration (our own baseline)**: VLC 3.0.23 on Debian trixie ships exactly this architecture — `05-vlc-list.txt` (0.A2 dump) lists modules `vaapi`, `vaapi_drm`, `vaapi_filters`, `glconv_vaapi_x11`, `glconv_vaapi_wl`, `glconv_vaapi_drm` (VA-API → GL per display server) plus the VDPAU family (`vdpau_avcodec`, `vdpau_display`, `glconv_vdpau`, …). The reference player's Linux hw path is VA-API-first with display-specific zero-copy converters — a useful independent confirmation that the candidate architecture is the ecosystem norm.

## 4. Audio — pure-Rust options for ADR-005 (Linux inputs)

crates.io facts (accessed 2026-10-19; dependency graphs via `/api/v1/crates/<name>/<ver>/dependencies`):

| Path | Crate (ver, updated) | Classification | Notes |
|---|---|---|---|
| ALSA direct | **alsa** 0.12.1 (2026-07-31, diwic/alsa-rs) | Safe wrapper over `alsa-sys 0.6.1` (**FFI bindings to libasound, C**) | Kernel device access *via* C library; no maintained pure-Rust ALSA client exists. trixie: `libasound2-dev 1.2.14`. No session mixing/routing |
| PipeWire | **pipewire** 0.10.1 (2026-08-19) = official `pipewire-rs` (gitlab.freedesktop.org/pipewire/pipewire-rs) | C-ABI wrappers (`pipewire-sys`, `libspa-sys` → libpipewire-0.3.so.0) — **upstream project's own Rust bindings** | trixie: pipewire 1.4.2, wireplumber 0.5.8, `libpipewire-0.3-0t64`. PipeWire is the modern Linux session audio graph (sinks to ALSA; speaks Pulse protocol too) |
| PulseAudio client (C) | **libpulse-binding** 2.30.1 (2025-04-19, jnqnfe) | Rust binding over `libpulse-sys 1.22` → **libpulse C** | mature; links C client lib |
| PulseAudio protocol (pure Rust) | **pulseaudio** 0.3.2 (2026-10-16, colinmarc/pulseaudio-rs) | **Pure Rust** — "native rust implementation of the PulseAudio protocol" | Works against PulseAudio **and pipewire-pulse**; actively updated [VERIFY completeness: playback stream + latency + volume for our needs] |
| Cross-platform facade | **cpal** 0.18.2 (2026-08-16, RustAudio) | Linux default host = **ALSA** (via `alsa ^0.11`, i.e. C lib); optional `jack` (libjack C), `pipewire` (dep:pipewire — same upstream crate, min PipeWire 0.3.53), `pulseaudio` (dep:pulseaudio — **the pure-Rust protocol crate**) | README: "ALSA is needed even when using JACK, PipeWire, or PulseAudio" (build-time dep). MSRV table per backend in README |
| JACK | jack 0.13.5 (RustAudio/rust-jack) | libjack C bindings | pro-audio path; out of MVP scope |

ADR-005 input summary (not a decision): on Linux every path except `pulseaudio` (protocol) goes through a C library that is arguably an OS service (sound server / kernel audio). cpal offers the least code across platforms but pins the Linux default to ALSA and is itself a thin layer; direct `pipewire` gives the modern routing graph; pure-Rust `pulseaudio` is the only zero-C option but rides the compatibility protocol. [VERIFY: sample-codecs/latency behavior of pipewire-rs for a player use-case.]

### 4.1 Decision-input matrix (facts only; choice is ADR-005's)

| Option | C surface | Routing/mixing (desktop users) | Latency character | Purity risk | Maturity |
|---|---|---|---|---|---|
| `alsa` direct | libasound (C) | none (single stream per device; no session policy) | lowest, direct | E2 sys-binding | 0.12.1, long-lived (diwic) |
| `pipewire` (upstream) | libpipewire-0.3 (C) | full graph (per-app volume, moves, filters) | good (quantum-tunable) | E2 sys-binding | 0.10.1, upstream project itself |
| `pulseaudio` crate | **none (native PA protocol)** | full via pipewire-pulse/PulseAudio | protocol round-trip | none (pure Rust) | 0.3.2, young but active (2026-10-16) [VERIFY completeness] |
| `libpulse-binding` | libpulse (C) | full | good | E2 sys-binding | 2.30.1, very mature |
| `cpal` facade | depends on enabled backend (ALSA default = C) | backend-dependent | backend-dependent | audit per backend | 0.18.2, RustAudio flagship |

Note for the matrix: "no session mixing" on ALSA matters for a VLC-class app (users expect per-app volume/routing); PipeWire is what modern distros run as the session server (trixie: pipewire 1.4.2 + wireplumber 0.5.8).

## 5. wgpu / GPU on Linux

- **wgpu 30.0.1** (crates.io, 2026-08-22): "cross-platform, safe, **pure-Rust** graphics API. It runs natively on Vulkan, Metal, D3D12, and OpenGL; and on top of WebGL2 and WebGPU on wasm" (README @ v30.0.1). Platform table: Linux → **Vulkan ✅ first-class**, OpenGL (GLES 3.0+) 🆗 best-effort. Pure Rust internally (ash/glow); the only native contact is the **Vulkan loader/ICDs + X11/Wayland client libs** — same E2 class as windowing.
- **lavapipe** = Mesa's software Vulkan driver (llvmpipe-based CPU rasterizer), packaged in `mesa-vulkan-drivers 25.0.7-2+deb13u1` (trixie; available, not installed in this headless sandbox — loader `libvulkan.so.1.4.309` present, no ICDs). Use: CI/headless wgpu tests without GPU. Cite: docs.mesa3d.org/llvmpipe.html ("LLVMpipe … the fastest software rasterizer for Mesa") [VERIFY: lavapipe doc page — Mesa restructured docs; current exact URL not found this run].
- **Transparent windows (candidate A substrate)**:
  - winit 0.30.13 `with_transparent`: "Sets whether the background of the window should be transparent… On some platforms this is more of a hint" (docs.rs/winit). winit exposes X11 visual override (`X11_VISUAL_ID`) in examples.
  - **X11**: translucency only visible with a running compositor; 32-bit visual + premultiplied ARGB. Stacking a separate transparent webview window above the wgpu window is possible but WM-dependent; wry child-webview (X11-only) is the supported embedding.
  - **Wayland**: every surface is compositor-managed; alpha = argb + premultiplied (wl_surface); no global stacking control → the only portable overlay is GTK-internal (§1.4). Open risk: transparent WebKitGTK surfaces take wl_shm paths on some compositors with an unbounded buffer-accumulation bug (wry#1731).
  - wgpu surface alpha: `CompositeAlphaMode::{Auto, Opaque, PreMultiplied, PostMultiplied, Inherit}` (docs.rs/wgpu/latest/wgpu/enum.CompositeAlphaMode.html) — PreMultiplied is the Wayland requirement; X11 compositors vary; the wgpu layer behind a transparent webview must negotiate a supported mode or render premultiplied.
- GPU driver coverage on Linux for wgpu/Vulkan: Mesa RADV (AMD) + ANV (Intel) + NVK (NVIDIA, newer; docs.mesa3d.org drivers index) + lavapipe (software) + proprietary NVIDIA Vulkan [VERIFY NVK vs proprietary maturity for our surfaces; not MVP-blocking].

### 5.1 Candidate-A substrate comparison: X11 vs Wayland

| Concern | X11 | Wayland |
|---|---|---|
| Window transparency | needs running compositor + 32-bit ARGB visual; historically flaky without one | always composited; argb wl_surface + premultiplied alpha is native |
| Embed webview over wgpu surface | wry `build_as_child` (X11-only) OR GTK-internal stacking | **GTK-internal stacking only** (wry `build_gtk`; no cross-window embedding) |
| Stacking control of separate windows | WM-dependent (override-redirect hacks) | forbidden by design (except wlroots layer-shell, non-portable) |
| Known WebKitGTK transparent-window bugs | NVIDIA black corners/crash (tauri#14924) | wl_shm buffer accumulation on transparent surfaces (wry#1731, Hyprland); text blur after resize (wry#1727) |
| wgpu alpha mode | PostMultiplied/PreMultiplied vary by compositor | PreMultiplied required |
| Escape hatch env vars | `WEBKIT_DISABLE_COMPOSITING_MODE`, `WEBKIT_DISABLE_DMABUF_RENDERER` (both force slower paths) | same, plus `GDK_BACKEND=x11` to hop display servers (heavy hammer) |

## 6. Verdict block (feeds synthesis — NOT decisions)

### 6.1 ADR-003 Linux feasibility (video in webview)

- **Candidate B is the structurally safer path on Linux.** Evidence: all rendering stays inside WebKitGTK (no transparency layering at all); WebCodecs is compiled into the Debian-stable build (in-binary proof, §2.1) with hardware decode reachable via GStreamer/VA-API; MSE (incl. ManagedMSE in 2.50) is the fallback; ImageBitmap is the no-feature-risk fallback. Confidence: **medium-high** (one open question: WebCodecs default-enabled state — [VERIFY] probe on HW).
- **Candidate A is plausible on X11, weak on Wayland today**: wry's child-webview is X11-only; Wayland requires GTK-internal stacking which no wry+wgpu combination demonstrates; transparent-surface artifacts are documented and current (NVIDIA black-corners/crash tauri#14924; Wayland wl_shm accumulation wry#1731); DMABUF-only accelerated compositing since 2.44 means transparency workarounds disable acceleration. Confidence A-on-Linux-overall: **low-medium**; would need a hardware spike (GTK overlay + wgpu surface + alpha mode + compositor matrix) before commitment.
- Contingency (if A fails everywhere but Windows/macOS): B is already the Linux answer; MVP gate = WebCodecs-or-MSE capability probe at runtime with ImageBitmap as last resort.

### 6.2 ADR-001 Linux path (VA-API)

- **Technical readiness: good.** libva 2.22 in Debian stable; all three GPU vendor classes decode H.264/HEVC/VP9/AV1 (§3.2, NVIDIA via shim w/ Firefox-grade caveats); Rust binding lineage exists and is field-proven (cros-libva, 1.28M downloads, ChromeOS upstream, BSD-3).
- **Policy question for the Lead (evidence only, no recommendation recorded here)**: is VA-API an *OS-service (system graphics stack)* E2 exception (freedesktop standard, drivers ship with the OS, stable-API guarantee, consumed as a system service by Firefox/mpv/VLC — analogous to Vulkan loader/ALSA lib), or does it *fail policy* as a userspace C library accessed via C ABI (not a kernel interface; dlopen vs link-time irrelevant)? If E2: choose audited binding (cros-libva family) vs own FFI module, exact symbol list, DMA-BUF export invariants, and driver-matrix CI. Record also: VDPAU is legacy/declining (bridge package gone from trixie).
- Zero-copy path VA-API→wgpu is via DMABUF/prime export [VERIFY: exact VA surface → Vulkan external-memory interop on each driver class; known to work in mpv/GStreamer but unproven by us].

### 6.3 ADR-005 Linux inputs

- PipeWire (1.4.2 in trixie) is the session audio reality; `pipewire` crate is upstream-official but C-ABI; `pulseaudio` crate is the only pure-Rust client path (protocol-level, works via pipewire-pulse); `alsa` crate is C-ABI kernel-adjacent; cpal 0.18.2 now has native ALSA/JACK/PipeWire/PulseAudio backends (ALSA always required at build). All options classified in §4; trade-offs (C-surface size vs routing features vs purity) are the Lead's call.

### 6.4 Open items / [VERIFY] follow-ups (with concrete probes)

1. WebCodecs default-enabled state in WebKitGTK 2.52/2.54 → hardware runbook: open a wry/Epiphany window, run `typeof VideoDecoder` in console; enumerate experimental features via `WebKitFeatureList` (or the runtime feature API). Expected artifact: text dump under docs/baselines/raw/linux/.
2. GNOME WebKit GitLab upstream search for transparency + DMABUF issues (blocked by anti-bot in this sandbox) — complement Tauri-side evidence.
3. Whether Tauri v2.12 sets any `WEBKIT_*` env vars itself — grep found none in `tauri-runtime-wry` 2.12.4; [VERIFY tao + wry full-tree + tauri CLI].
4. tauri 3.0 (alpha now) webview backend choice; wry#1767 (gtk4/webkit6 migration) landing timeline.
5. AMD per-ASIC VA-API codec matrix citation (Mesa docs restructure blocked exact page).
6. lavapipe canonical doc URL (Mesa docs tree moved).
7. VA-API↔Vulkan external-memory zero-copy proof per driver class (Phase-1 spike: VA surface → DMABUF export → wgpu import → present).
8. `pulseaudio` (pure-Rust) crate completeness for playback+volume+latency.
9. GStreamer `va` (non-legacy) plugin availability in trixie gst-plugins-bad for the WebKitGTK hw path.
10. NVK vs proprietary NVIDIA Vulkan driver maturity for wgpu surfaces.
11. Fedora/Arch current webkit2gtk versions (Bodhi / Arch package API) to finish the distro floor (§1.1.1).
12. Custom-protocol handler threading on wry's GTK backend (§2.0) — measure per-frame round-trip jitter.

### 6.5 Risk observations (inputs for doc 14, not register edits)

| # | Observation (evidence) | Suggested severity frame |
|---|---|---|
| L1 | Candidate A on Wayland has no supported embedding path in wry today (child-webview X11-only; §1.2) | blocks A on Wayland until spike/wry changes |
| L2 | NVIDIA + transparent WebKitGTK windows: crash or artifacts, open, driver-590-era (tauri#14924) | blocks A on a whole GPU class |
| L3 | Transparent webview surfaces on Wayland can leak wl_shm buffers unboundedly on some compositors (wry#1731) | reliability risk for any always-on transparent UI, incl. A |
| L4 | All transparency workarounds (`WEBKIT_DISABLE_*`) disable accelerated compositing → unusable for video UI | workaround-only, not a strategy |
| L5 | WebCodecs enablement default is unverified (§2.1) → B's best path may need runtime probing + MSE fallback | medium, testable early |
| L6 | Every candidate audio path except `pulseaudio` crate links a C library (§4) | policy exposure per ADR-005 choice |
| L7 | VA-API on NVIDIA is a Firefox-targeted shim, decode-only, "may not operate correctly in other applications" (§3.2) | per-vendor caveats for ADR-001 claims |
| L8 | wry/tauri Linux stack is GTK3/4.1; gtk4/webkit6 migration is an open community PR (wry#1767); tauri 3.0 in alpha | mid-term churn; pin + re-audit at tauri upgrades |

## 7. Source register (all accessed 2026-10-19)

- https://webkitgtk.org/ (homepage; stable 2.54.1, unstable 2.53.92) · /news.html (full release index) · /releases/ (tarballs) · /2025/10/07/webkitgtk-soup2-deprecation.html · /2023/09/15/webkitgtk2.42.0-released.html · /2023/12/05/webkitgtk2.42.3-released.html · /2024/03/16/webkitgtk2.44.0-released.html · /2024/10/04/webkitgtk-2.46.html · /2025/04/08/webkitgtk-2.48.html · /2025/11/26/webkitgtk-2.50.html · /2026/03/18/webkitgtk-2.52-highlights.html · /2026/09/16/webkitgtk-2.54-highlights.html
- https://github.com/tauri-apps/wry (tag wry-v0.57.0: Cargo.toml, src/lib.rs, src/webkitgtk/mod.rs, README.md) · https://github.com/tauri-apps/tauri (tag tauri-v2.12.4: crates/tauri-runtime-wry/src/lib.rs) · https://tauri.app/start/prerequisites/ · https://tauri.app/reference/config/
- Issues: github.com/tauri-apps/tauri/issues/{14924,13270,9304,10702,15936,9394}, /pull/16226 · github.com/tauri-apps/wry/issues/{1731,1540,1366,1727}, /pull/1767 · GitHub search counts via api.github.com/search/issues (2026-10-19)
- crates.io API (2026-10-19): wry, tauri, wgpu, winit, cpal (+deps), alsa (+deps), alsa-sys, pipewire (+deps), libpulse-binding (+deps), pulseaudio, jack, cros-libva, libva, cros-libva-extended, vacc-libva, libva-sys, fev, moq-vaapi, nuxodecs, oxideav-vaapi, vacc, rawshift-hwdec; searches `q=vaapi`, `q=libva`
- https://github.com/intel/libva (README) · https://github.com/intel/media-driver (README codec matrix) · https://github.com/elFarto/nvidia-vaapi-driver (README) · https://github.com/chromeos/cros-libva (README, lib/Cargo.toml, lib/build.rs) · https://github.com/mlm-games/cros-libva (README)
- https://docs.rs/winit/latest/winit/window/struct.WindowAttributes.html · https://docs.rs/wgpu/latest/wgpu/enum.CompositeAlphaMode.html · https://github.com/gfx-rs/wgpu (README @ v30.0.1) · https://github.com/RustAudio/cpal (README + Cargo.toml @ master) · https://docs.mesa3d.org/llvmpipe.html · https://docs.mesa3d.org/envvars.html · https://docs.mesa3d.org/index.html (drivers index)
- Local sandbox (Debian 13 trixie, 2026-10-19): `apt-cache policy` for libwebkit2gtk-4.1-0 / libwebkitgtk-6.0-4 / webkit2gtk-driver / libva* / mesa-va-drivers / i965-va-driver / intel-media-va-driver / gstreamer1.0-vaapi / pipewire / wireplumber / libasound2-dev / libvdpau-dev / mesa-vulkan-drivers / vdpau-va-driver; binary probe of libwebkit2gtk-4.1 2.52.6 (download → dpkg-deb -x → strings) recorded in tmp/linuxaudit/wk-strings.txt.

## 8. Appendix — raw evidence inventory (this run)

All under git-ignored `tmp/linuxaudit/` (not committed; per AGENTS §3.8 and clean-room rules):

- `crate-*.json`, `search-vaapi.json`, `search-libva.json`, `deps-*.json` — crates.io API responses (2026-10-19).
- `webkitgtk-*.html`, `news-index.html`, `2023..2026_webkitgtk*.html`, `wk244.html` — webkitgtk.org pages/posts.
- `wry-cargo-wry-v0.57.0.toml`, `wry-lib.rs`, `wry-webkitgtk.rs`, `wry-README.md`, `tauri-runtime-wry.rs` — upstream sources at tags.
- `tauri-prereq.html`, `tauri-config-ref.html`, `winit-attrs.html`, `wgpu-alpha.html`, `wgpu-README-v30.0.1.md`, `cpal-README.md`, `cpal-Cargo.toml` — docs pages.
- `ghsearch*.json`, `tauri-apps_*.html` — GitHub issue searches + issue pages (API quota exhausted mid-run; HTML fallback).
- `libva-README.md`, `media-driver-README.md`, `nvvaapi-README.md`, `croslibva-*` — VA-API upstream docs.
- `mesa-*.html`, `mesa-tree.json` — Mesa docs probes (several 404/anti-bot results recorded honestly).
- `wk-strings.txt` — 909,815 strings from Debian's libwebkit2gtk-4.1 2.52.6 binary (WebCodecs/MSE proof).
- `lp-webkit.json` — Launchpad API webkit2gtk versions per Ubuntu series.

*Clean-room note: no VLC source/assets consulted; all engine facts from public specifications, release notes, package metadata, and independent binary inspection.*
