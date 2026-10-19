# Platform Synthesis — Consolidated Inputs for ADR-001 / ADR-003 / ADR-005 (Task 0.C5-s)

- Sources (read in full; **consolidation only — no new research performed**):
  - **[L]** `docs/inventory/platform-linux.md` (0.C5-a) · **[W]** `docs/inventory/platform-windows.md`
    (0.C5-b) · **[M]** `docs/inventory/platform-macos.md` (0.C5-b).
  - Supporting, where cited: `docs/inventory/modules-summary.md` (0.B2), `docs/13-agent-wave-program.md` §5,
    `docs/ROADMAP.md` D-008, `docs/MASTER-PROMPT.md` (MP) §2/§9.
- Citation convention: `[L §x]` / `[W §x]` / `[M §x]` = section of the named platform record.
- Feeds ADR-001 (hw decode), ADR-003 (video-in-webview), ADR-005 (audio backend). **Decision inputs,
  not decisions** (AGENTS §3.4/§3.6). All underlying evidence dated 2026-10-19.
- Common substrate (all three records): wry 0.57.0 / tauri 2.12.4 / tao 0.37.1 / raw-window-handle
  0.6.2 / wgpu 30.0.1 / cpal 0.18.2 [W §1, M §1, L §1.2/§4/§5].

## 1. Comparison tables

### 1.1 Candidate A — wgpu surface behind a transparent webview (ADR-003)

| OS | Verdict (record's own) | Key conditions | Confidence | Citations |
|---|---|---|---|---|
| Windows | Feasible **with conditions** | Documented transparency: `put_DefaultBackgroundColor`, alpha **binary 0/255 only**; host app content shows through (Microsoft-documented). Parent window must be created **without WS_CLIPCHILDREN** (winit has the switch; tao 0.37.1 does not expose one). Startup white-flicker artifact (mitigate via `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` env var, process-wide). `backdrop-filter` over transparent regions unproven (wry #408 lineage). Visual/composition hosting = robust long-term path but unimplemented in wry and windows-webview, costs input forwarding + rasterization-scale handling. | medium (derived from verdict language; conditions enumerable and documented) | [W §2.1–2.4, §7] |
| macOS | **Structurally the cleaner platform** | Transparency via private-KVC `drawsBackground` (wry implements it; Tauri ≥2.12.1 compiles it in unconditionally). WKWebView is an ordinary subview → sibling layer-hosting NSView (wgpu/CAMetalLayer) behind it via `addSubview` ordering; no child-HWND z-order problem. Risks: private-API provenance (App Store channel only; notarized direct distribution unaffected in practice), OS point-release regressions (macOS 15.7 black base, fixed in 15.8.1 — wry #1867), wgpu-30 surface-from-NSView mechanics unproven. | medium-high (derived from [M §7] "structurally the cleaner platform") | [M §2.1–2.3, §7] |
| Linux (X11) | Plausible | wry child webview (`build_as_child`) is **X11-only**; or GTK-internal stacking (no wry+wgpu combination demonstrates it). NVIDIA transparent-surface crash/artifact bugs open (tauri#14924, driver-590 era). DMABUF-only accelerated compositing since WebKitGTK 2.44 → transparency workarounds (`WEBKIT_DISABLE_*`) disable acceleration. | low-medium (record's own, [L §6.1]) | [L §1.2–1.4, §5.1, §6.1] |
| Linux (Wayland) | **No supported embedding path today** | Only embedding = GTK-internal stacking via `build_gtk`; no cross-toolkit child windows; client-controlled global stacking forbidden by design. Transparent surfaces can leak wl_shm buffers unboundedly on some compositors (wry#1731, Hyprland). | low (blocking issues current + open) | [L §1.2, §1.3, §1.4, §5.1, §6.5 L1/L3] |

### 1.2 Candidate B — frame streaming to canvas, incl. WebCodecs floors (ADR-003)

| OS | Viability | WebCodecs floor | Fallbacks / caveats | Citations |
|---|---|---|---|---|
| Windows | **Available today** | Chromium ≥94 in evergreen WebView2 runtime; WebCodecs not on WebView2's documented disabled-features list | HEVC inherits the Store-extension problem; per-frame copy/composite cost vs A unmeasured (Phase-2 prototype); WebGPU `importTexture` fast path unverified | [W §3, §4.1] |
| macOS | Viable on modern macOS | Video-only WebCodecs from **macOS ≥13.3** (Safari/WebKit 16.4); full API from **macOS ≥26** (Safari 26). WKWebView uses system WebKit; parity presumed, not verified | Older floors exclude B entirely; audio is engine-side regardless; minimum-supported-macOS decision pending (doc 01/11) | [M §3] |
| Linux | **Structurally safer path** | WebCodecs **proven compiled-in** to Debian-stable WebKitGTK 2.52.6 (in-binary strings: VideoDecoder/VideoFrame/etc.); GStreamer-backed; `prefer-hardware` honored since 2.48. **Default-enabled state unverified** | Ranked paths: (1) WebCodecs → canvas; (2) MSE / ManagedMSE (2.50+); (3) ImageBitmap/canvas-2D (no feature risk, highest CPU). Transport: wry custom protocol (binary) + IPC control plane; custom-protocol handler threading on GTK unverified | [L §2.0–2.3, §6.1] |

### 1.3 HW-decode API readiness × codec coverage × Rust access (ADR-001)

| OS / API | Decode coverage | Install friction | Rust access path | Policy classification | Zero-copy to renderer | Citations |
|---|---|---|---|---|---|---|
| Windows / Media Foundation | In-box: H.264 (DXVA, up to High) + MPEG-4 Part 2, H.263, MJPEG, DV, WMV. HEVC/VP9/AV1 arrive via Store extensions: HEVC **$0.99** (new id `9nmzlz57r3t7`; legacy `9N4WGH0Z6VHM` returns HTTP 410), AV1/VP9/Web Media free | Yes — clean installs need extensions; runtime `MFTEnumEx` feature-detect + Store install prompt **must** be designed; bundling the MFT not permitted | `windows` 0.62.2 (MF/D3D11/DXGI/COM features verified) or `windows-sys` 0.61.2; 0.100.0 generation also published — pick one line (doc 11) | **E2** (metadata-generated OS-ABI bindings) | HW MFTs emit **D3D11** textures vs wgpu's **D3D12** — D3D11↔D3D12 interop is the single biggest Windows integration unknown; CPU-copy fallback exists | [W §4.1–4.3, §6, §7] |
| macOS / VideoToolbox | H.264 ubiquitous (all supported Macs); HEVC hw on all Apple Silicon + HEVC-capable Intel; ProRes built in; VP9 hw/sw split per SoC unverified; AV1 hw only on newest SoCs (Safari-17-era), software floor unverified | **None** — codecs ship with the OS; only variable is hardware presence | `objc2` 0.6.5 + `objc2-video-toolbox` 0.3.2 (`VTDecompressionSessionCreate` / `VTIsHardwareDecodeSupported` confirmed on docs.rs) + `objc2-core-video` / `objc2-metal`; [PROPOSAL] standardize on the objc2 family | **E2** (metadata-generated OS-ABI bindings) | `CVPixelBuffer` → `CVMetalTextureCache` → `MTLTexture` → wgpu import; wgpu-30 import path unverified; CPU-copy fallback straightforward | [M §4.1–4.3, §7] |
| Linux / VA-API | Intel `iHD`: H.264/HEVC/VP9/AV1 (+VVC on newest gens); AMD via Mesa (H.264/HEVC broadly; VP9/AV1 VCN-dependent — per-ASIC citation missing); NVIDIA via **shim**: decode-only, all four, Firefox-targeted caveats. VDPAU = legacy, out of scope (bridge gone from trixie) | None — drivers ship with the graphics stack (libva 2.22 in Debian stable) | `cros-libva` 0.0.13 (ChromeOS upstream, BSD-3, bindgen + links C libva; 1.28M downloads) or its forks; **no pure-Rust VA-API is possible** (the API is a C ABI) | **Open classification question**: OS-service (freedesktop system video-accel service, stable-API guarantee, consumed by Firefox/mpv/VLC) vs forbidden userspace C library — **Lead/user call**; either way an E2 ADR with exact symbol ledger is required | DMABUF/prime export (`VA_SURFACE_ATTRIB_MEM_TYPE_DRM_PRIME_2`) → Vulkan external-memory import into wgpu; proven in mpv/GStreamer, unproven by us | [L §3.1–3.5, §6.2] |

Cross-cutting: API-name correction for ADR-001 text — use `MFTEnumEx`/`IMFActivate`/`IMFTransform`; `MFCreateVideoDecoder` **does not exist** [W §4.2]. Runtime probes for truthful reporting: `MFTEnumEx` [W §4.1–4.2], `VTIsHardwareDecodeSupported` [M §4.1], `vaQueryConfigEntrypoints`/vainfo-mirror [L §3.5].

### 1.4 Audio backend options per OS, with policy classification (ADR-005)

| OS | Option | Crate (ver) | C surface | Classification | Notes | Citations |
|---|---|---|---|---|---|---|
| Windows | WASAPI via cpal | cpal 0.18.2 | none beyond OS ABI (windows-rs 0.62) | E2 | Shared-mode **event-driven** (`AUDCLNT_STREAMFLAGS_EVENTCALLBACK`) = proven Rust pattern; no exclusive mode (only optional asio-sys); ~10–20 ms shared, <10 ms via IAudioClient3 (unverified on HW) | [W §5] |
| Windows | Direct WASAPI on windows-rs | windows 0.62.2 | none beyond OS ABI | E2 | `IAudioClient2/3` + `IAudioRenderClient` + `IAudioClock` for exclusive/bit-exact + drift; scope question open | [W §5] |
| macOS | CoreAudio via cpal | cpal 0.18.2 → coreaudio-rs 0.14.2 | none beyond OS ABI | E2 | AudioUnit render callback (pull model on CoreAudio-owned RT thread) | [M §5, §1] |
| macOS | Direct coreaudio-rs / objc2-core-audio | coreaudio-rs 0.14.2 / objc2-* 0.3.2 | none beyond OS ABI | E2 | Direct-access route per coreaudio-rs README; `AVAudioEngine` alternative via objc2-av-foundation. **Caution**: a *different* crate `coreaudio` 0.5.0 (other publisher) exists — audit before any use | [M §5] |
| Linux | ALSA direct | alsa 0.12.1 | libasound (C) | OS-service sys-binding (conditional E2) | Lowest latency; no session mixing/routing | [L §4] |
| Linux | PipeWire | pipewire 0.10.1 (upstream pipewire-rs) | libpipewire-0.3 (C) | OS-service sys-binding (conditional E2) | Full routing graph; distro reality (trixie: pipewire 1.4.2 + wireplumber 0.5.8) | [L §4] |
| Linux | PulseAudio **protocol** | pulseaudio 0.3.2 | **none — pure Rust** | Pure Rust | Only zero-C path; works against PulseAudio and pipewire-pulse; young, completeness unverified | [L §4] |
| Linux | libpulse client | libpulse-binding 2.30.1 | libpulse (C) | OS-service sys-binding (conditional E2) | Very mature | [L §4] |
| Linux | cpal facade | cpal 0.18.2 | backend-dependent (ALSA default = C) | **Conditional E2** (audit per backend) | Least cross-platform code; Linux default host = ALSA (no session mixing); optional jack/pipewire/pulseaudio backends; ALSA build-dep always | [L §4] |
| any | JACK | jack 0.13.5 | libjack (C) | sys-binding | Pro-audio path; out of MVP scope | [L §4] |

Policy frame: MP §9 names **audio devices** among the documented OS-service-binding exceptions, so per-OS audio bindings are the sanctioned class; open work is per-crate audit (doc 11) and, on Linux, which C library is accepted as an OS service. RT-model asymmetry is a design constraint: CoreAudio **pulls** (RT callback: no alloc/lock/Obj-C messages), WASAPI events **push** — inverted wakeup models for one cross-platform contract [M §5, W §5].

### 1.5 wgpu / GPU status per OS

| OS | Backend | Status | Risks / notes | Citations |
|---|---|---|---|---|
| Windows | D3D12 (default feature of wgpu 30.0.1) | Pure-Rust wgpu; surface via rwh 0.6 → HWND | D3D11 (MF decoder output) ↔ D3D12 (wgpu) interop = top risk; shared-handle import capability unverified | [W §6, §2.4] |
| macOS | Metal (default feature) | Pure-Rust wgpu; CAMetalLayer surfaces | Separate layer-hosting subview for overlay = standard practice; wgpu-30 from-raw-NSView mechanics unverified | [M §1, §2.2] |
| Linux | Vulkan first-class; GLES best-effort; lavapipe software fallback (CI/headless) | Pure-Rust wgpu (ash); only native contact = Vulkan loader/ICDs + X11/Wayland client libs (E2 class) | `CompositeAlphaMode::PreMultiplied` required on Wayland; X11 translucency needs a compositor; NVK vs proprietary NVIDIA maturity open | [L §5, §5.1] |

## 2. Honest bottom lines

1. **Candidate A**: Windows feasible-with-conditions (documented API, enumerable artifacts); macOS structurally cleaner (sibling-view compositing is the *normal* macOS case — the exact scenario that failed on Windows worked on macOS in wry #1540); Linux has **no Wayland path in wry today** → **B is the safer Linux answer**; A-on-Linux overall is low-medium confidence, X11-only plausible. [W §2.3/§7, M §7, L §6.1]
2. **Candidate B**: uniformly available on Windows today; version-gated on macOS (≥13.3 video-only, ≥26 full); compiled-in but default-unverified on Linux — one cheap hardware probe resolves the Linux question. [W §3, M §3, L §2.1]
3. The records suggest ADR-003 may rationally land as **A-on-macOS + A-or-B-on-Windows + B-on-Linux**; that split is the Lead's call, gated on the Phase-2 prototype. [M §7]
4. **ADR-001**: hw decode is reachable from pure Rust on all three OSes — windows-rs and objc2-video-toolbox are clean E2 paths; **VA-API classification is a Lead/user call** (both framings evidenced, no recommendation recorded in [L]). Top technical unknowns are the three zero-copy interops (D3D11↔D3D12; MTLTexture import; DMABUF→Vulkan), each with a CPU-copy fallback. [W §4.3/§6, M §4.2–4.3, L §3.3–3.5/§6.2]
5. **Windows HEVC is the user-facing edge case**: paid Store extension, dead legacy product id, runtime install flow required — nothing can be assumed; feature-detect everything. [W §4.1]
6. **macOS**: zero install friction, but point-release fragility is real (15.7 transparency regression fixed in 15.8.1) → pin OS versions in the QA matrix; private-API risk applies only to a hypothetical App Store channel, not notarized direct distribution. [M §2.1, §6]
7. **ADR-005**: audio-device bindings are the sanctioned OS-service exception (MP §9); **cpal conditional-E2 is acceptable** (E2 on Windows/macOS; per-backend audit on Linux); on Linux only the `pulseaudio` protocol crate is zero-C, and it is young; Windows exclusive/bit-exact scope is unresolved. [W §5, M §5, L §4]
8. **Oracle corroboration**: VLC 3.0.23's own module scoring prefers hw decode (ffmpeghw score 10001, dav1d 10000) and its Linux hw path is VA-API-first with display-specific zero-copy converters — independent confirmation that hw-first is the ecosystem norm. [modules-summary.md (0.B2); L §3.5]

## 3. ADR open-question matrix

Status legend: **answered** = answered-with-evidence (link) · **[VERIFY]** = concrete follow-up (what + who). No question is left unanswered in principle.

### 3.1 ADR-001 — hw decode (Phase-0 decision per ROADMAP D-008)

| # | Question | Status | Evidence / follow-up |
|---|---|---|---|
| 1 | Use OS hw decoders at all? | answered | MP §2 fixed decision (hw default + truthful reporting); oracle corroboration in modules-summary.md (0.B2) |
| 2 | Which API per OS? | answered | MF [W §4]; VT [M §4]; VA-API with VDPAU ruled out [L §3.4] |
| 3 | Rust access path per OS? | answered (Win/mac) · [VERIFY] (Linux) | windows-rs E2 [W §4.3] (+ generation choice → doc 11, SV-29 [W §1]); objc2-video-toolbox E2 [M §4.3] (+ objc2 standardization is [PROPOSAL] [M §4.3]); VA-API OS-service-vs-C-library ruling — **Lead/user** (SV-28) [L §3.4, §6.2] |
| 4 | Codec coverage matrix? | answered + per-cell [VERIFY] | Coverage tables [W §4.1, M §4.1, L §3.2]; [VERIFY] HEVC preinstall rate (hardware matrix), Apple VP9/AV1 splits, AMD per-ASIC citation — SV-10..13 |
| 5 | Zero-copy into renderer? | [VERIFY] per-OS spike | Paths documented [W §6, M §4.2, L §3.5]; proofs = ADR-001 spike — SV-6..9 |
| 6 | Runtime detection + truthful active-decoder reporting? | answered | MFTEnumEx [W §4.1–4.2]; VTIsHardwareDecodeSupported [M §4.1]; vainfo-mirror probe [L §3.5] |
| 7 | Software fallback when hw absent? | answered | Rust software codecs (MP §2); CPU-copy fallbacks [W §6, M §7, L §3.5] |
| 8 | Windows HEVC acquisition UX? | answered + [VERIFY] | Prices/ids/no-bundling rule [W §4.1]; [VERIFY] preinstall matrix + propagate new store id (doc 11 §9 + user docs) — SV-10, SV-38 |

### 3.2 ADR-003 — video-in-webview (Phase 0/2; finalized at gate 2.E2-V)

| # | Question | Status | Evidence / follow-up |
|---|---|---|---|
| 1 | Candidate A vs B per OS? | answered (evidence-complete); decision after prototype | Per-OS verdicts [W §7, M §7, L §6.1]; prototype card 2.D3 + gate 2.E2-V (doc 13 §5) |
| 2 | A's conditions per OS? | answered | [W §2.2–2.3, M §2.1–2.3, L §1.4/§5.1] |
| 3 | WebCodecs floors for B? | answered (Win/mac) · [VERIFY] (Linux default-enabled) | [W §3; M §3; L §2.1 → SV-1] |
| 4 | B frame transport? | answered | Custom protocol (binary) + IPC control plane [L §2.0]; same wry API cross-platform |
| 5 | B vs A cost/latency? | [VERIFY] — measured | 2.D3 AC: measured latency + compatibility matrix (doc 13 §5); doc 12 budget TBD [W §3] |
| 6 | CSS effects over the video region? | answered (guidance) | Avoid backdrop-filter over video until proven on all platforms [W §2.3, M §2.3]; [VERIFY] in prototype — SV-17 |
| 7 | Support floors gating B? | [VERIFY] — Lead | macOS minimum (13.3? 14? 15?) [M §3]; Linux distro floor ≥2.44–2.46 [L §1.1.1]; decision in doc 01/11 |
| 8 | Degraded-mode fallback chain? | answered | WebCodecs → MSE → ImageBitmap [L §2.2–2.3]; Windows Store install-prompt flow [W §4.1] |

### 3.3 ADR-005 — audio backend (Phase 1 wave 1.C per ROADMAP D-008)

| # | Question | Status | Evidence / follow-up |
|---|---|---|---|
| 1 | Backend per OS? | answered (options + rankings); decision Lead's | [W §5, M §5, L §4/§4.1]; ranking in §4.3 below |
| 2 | cpal facade vs direct per-OS? | answered (trade-off evidence) | Coverage + limits per OS [W §5, M §5, L §4] |
| 3 | Real-time callback contract? | answered | CoreAudio pull / no-alloc/no-lock/no-ObjC [M §5]; WASAPI event-driven push [W §5]; encode as tested invariant — SV-27 |
| 4 | Exclusive/bit-exact mode (Windows) scope? | [VERIFY] — Lead | [W §5] — SV-22 |
| 5 | Linux C-surface policy? | answered (evidence) + per-backend audit | All-but-pulseaudio paths link a C library [L §4]; audio devices = MP §9 exception class; audits in doc 11 |
| 6 | Crate audits before adoption? | [VERIFY] — doc 11 | coreaudio disambiguation [M §5]; audio_thread_priority [W §5]; pipewire-rs behavior + pulseaudio completeness [L §4] — SV-24, SV-25, SV-30, SV-31 |
| 7 | Pro features scope (hog mode, channel maps, device routing)? | [VERIFY] — Lead (ADR-005 scope) | Availability evidenced [M §5] |

## 4. Recommended decision inputs (NOT decisions)

### 4.1 ADR-003 — per-OS prototype plan

Prototype planning already exists in doc 13 §5 — referenced, not re-planned:
- **2.D3**: variant **B first** (frame streaming to canvas via a minimal `apps/desktop` Tauri skeleton); variant A only if B fails gates; test WebView2, WKWebView, WebKitGTK separately; AC = measured latency + compatibility matrix + honest recommendation → `docs/prototypes/video-in-webview.md`.
- **2.E2-V**: Phase-2 gate — webview prototype verdict recorded → ADR-003 finalized.

Evidence-weighted emphasis to carry into 2.D3 scoping (input only):

| OS leg | Must-cover items (from records) |
|---|---|
| Windows | WS_CLIPCHILDREN off (or winit-hosted window); white-flicker env-var mitigation in a packaged app; backdrop-filter check; WebCodecs HEVC on a clean install; WebGPU importTexture [W §2.2–2.3, §3] |
| macOS | drawsBackground KVC; OS point-version matrix (15.7-class); NSView insertion (build_as_child parent-view hook is the cleaner hook); wgpu surface from NSView; MTLTexture import [M §2.1–2.2, §4.2] |
| Linux | **WebCodecs default-enabled probe first** — cheapest, earliest de-risk, unblocks B's best path [L §6.4-1]; custom-protocol jitter measurement [L §2.0/§2.3]; if A is pursued: GTK-internal stacking spike + CompositeAlphaMode + compositor/NVIDIA matrix [L §1.4, §5.1] |

Sequencing note for the Lead: the records' evidence ordering (B safest on Linux, A cleanest on macOS) is consistent with 2.D3's B-first design; since macOS is where A's evidence is strongest, an early A-on-macOS data point would inform the final per-OS split even under a B-first plan [M §7].

### 4.2 ADR-001 — hw-first with truthful reporting (framing the evidence supports)

- Default-on hw decode with **per-stream truthful reporting of hw vs sw active** (MP §2) — directly supported by per-OS runtime probes: MFTEnumEx [W §4.2], VTIsHardwareDecodeSupported [M §4.1], vaQueryConfigEntrypoints / vainfo-mirror [L §3.5].
- Graceful degradation, never fabrication: Rust software codecs as fallback (MP §2); documented CPU-copy fallbacks where zero-copy interop fails [W §6, M §7, L §3.5].
- Encode per-OS caveats into user-visible truth: Windows Store-extension HEVC/AV1 (install prompt; bundling not permitted) [W §4.1]; NVIDIA VA-API shim is decode-only and Firefox-targeted [L §3.2]; Apple VP9/AV1 per-SoC variance [M §4.1].
- Treat the three zero-copy interops as **spike-gated acceptance criteria**, not assumptions [W §6, M §4.2, L §3.5].
- API-text corrections: MFTEnumEx/IMFActivate/IMFTransform (no `MFCreateVideoDecoder`) [W §4.2]; propagate the new HEVC store id `9nmzlz57r3t7` [W §4.1].
- The one **policy gate**: VA-API OS-service vs C-library classification (Lead/user); then binding choice — audited cros-libva lineage vs own FFI module with an exact symbol ledger [L §3.3–3.4, §6.2].

### 4.3 ADR-005 — backend options ranked (evidence-based; the choice is the Lead's)

- **Windows**: (1) cpal/WASAPI shared event-driven — E2, proven pattern, least code; (2) direct windows-rs WASAPI (IAudioClient2/3) if exclusive/bit-exact is in scope. Gate: SV-22. [W §5]
- **macOS**: (1) cpal (coreaudio-rs, E2); (2) direct coreaudio-rs; (3) direct objc2-core-audio / AVAudioEngine if device-routing features are required. Exclude pending audit: `coreaudio` 0.5.0 (different publisher). [M §5]
- **Linux** (ranking depends on the Lead's weighting): code-uniformity → **cpal** (pin the backend explicitly; ALSA default lacks session mixing); routing fidelity → **pipewire** (upstream, full graph); purity → **pulseaudio** (only zero-C; young, [VERIFY]); latency purism → **alsa** direct (no routing); maturity → **libpulse-binding**. [L §4, §4.1]
- **Cross-OS**: one RT callback contract regardless of facade (§3.3-Q3); a cpal + direct-escape-hatch hybrid is compatible with the platform-adapter ADR pattern [W §5, M §5].

## 5. Consolidated [VERIFY] ledger (deduped from [L §6.4], [W] "Open items", [M] "Open items" + inline marks)

| ID | Item (deduped) | Sources | Vehicle / owner |
|---|---|---|---|
| SV-1 | Linux WebCodecs default-enabled state (typeof VideoDecoder + WebKitFeatureList probe on 2.52/2.54) | [L §2.1, §6.4-1] | Hardware runbook / 2.D3 |
| SV-2 | WKWebView WebCodecs parity on the minimum supported macOS (+ WebGPU importTexture support, macOS 26+) | [M §3, item 1] | Hardware matrix |
| SV-3 | WebView2: HEVC decode on clean Win11 (± extension) + WebGPU VideoFrame import in pinned runtime | [W §3, item 2] | Hardware matrix / 2.D3 |
| SV-4 | wry custom-protocol handler threading/jitter on GTK backend | [L §2.0, §2.3, item 12] | 2.D3 measurement |
| SV-5 | GStreamer `va` (non-legacy) plugin availability in trixie gst-plugins-bad | [L §2.1, item 9] | Hardware runbook |
| SV-6 | wgpu 30 surface API shape (SurfaceTarget) | [W §2.4, item 3] | Phase-2 scaffolding / ADR-001 spike |
| SV-7 | D3D11-shared-texture import into wgpu-hal DX12 | [W §6, item 3] | ADR-001 spike |
| SV-8 | wgpu 30 surface from custom layer-hosting NSView + MTLTexture import | [M §2.2, §4.2, item 4] | ADR-001 spike / 2.D3 |
| SV-9 | VA surface → DMABUF → Vulkan external-memory import, per driver class | [L §3.5, §6.2, item 7] | ADR-001 spike (Phase 1) |
| SV-10 | Windows HEVC extension preinstall rate + OEM "Device Manufacturer" variant availability | [W §4.1, item 5] | Phase-0 hardware matrix |
| SV-11 | macOS VP9 hw/sw decode split per Apple Silicon generation | [M §4.1, item 2] | Hardware matrix |
| SV-12 | macOS AV1 software decode in VideoToolbox (floor + performance class) | [M §4.1, item 3] | Hardware matrix |
| SV-13 | AMD per-ASIC VA-API codec matrix citation (Mesa docs restructured) | [L §3.2, item 5] | Follow-up citation hunt |
| SV-14 | Exact API/symbol spellings: VTIsHardwareDecodeSupported + CMVideoFormatDescription* names; VA symbol list vs va.h 2.22; libva loader mechanics | [M §4.1–4.2; L §3.1, §3.5] | ADR-001 spike (exception ledger) |
| SV-15 | tao 0.37.1 default window styles re WS_CLIPCHILDREN on real Windows | [W §2.3, item 1] | 2.D3 prototype |
| SV-16 | White-flicker mitigation via WEBVIEW2_DEFAULT_BACKGROUND_COLOR in a packaged Tauri app | [W §2.2, item 8] | 2.D3 prototype |
| SV-17 | backdrop-filter over transparent regions / video (Windows + macOS) | [W §2.3, M §2.3] | 2.D3 prototype |
| SV-18 | macOS NSView insertion point (contentView vs wry parent view) | [M §2.2] | 2.D3 prototype |
| SV-19 | macOS point-release regression matrix (15.7-class); pin OS versions in QA | [M §2.1, item 6] | QA matrix (doc 10) |
| SV-20 | GNOME/freedesktop WebKit GitLab upstream transparency/DMABUF issues (anti-bot blocked) | [L §1.3, §6.4-2] | Follow-up (any agent with access) |
| SV-21 | Fedora/Arch webkit2gtk versions to finish the distro floor | [L §1.1.1, item 11] | Follow-up (package APIs) |
| SV-22 | WASAPI exclusive/bit-exact scope for ADR-005 | [W §5, item 7] | **Lead** |
| SV-23 | IAudioClient3 sub-10-ms latency on real drivers | [W §5] | Hardware matrix |
| SV-24 | pipewire-rs sample/latency behavior for a player use-case | [L §4] | Phase-1 audio spike |
| SV-25 | pulseaudio (pure-Rust) crate completeness: playback + volume + latency | [L §4, item 8] | Phase-1 audio spike |
| SV-26 | AudioUnit vs AVAudioEngine scope (routing, hog mode, channel maps) | [M §5, item 7] | **Lead** (ADR-005 scope) |
| SV-27 | RT callback contract (no alloc/lock/Obj-C) encoded as a tested invariant | [M §5, item 8] | audio-output crate design |
| SV-28 | VA-API OS-service vs C-library classification | [L §3.1, §3.4, §6.2] | **Lead + user** (policy) |
| SV-29 | windows-rs generation choice (0.62.x vs 0.100.x) | [W §1, §4.3, item 4] | doc 11 / ADR-002 audit |
| SV-30 | coreaudio vs coreaudio-rs crate disambiguation + provenance audit | [M §5, item 7] | doc 11 |
| SV-31 | audio_thread_priority 0.35 classification if adopted | [W §5] | doc 11 |
| SV-32 | cpal license-choice flag vs decided MIT (ADR-006) — reconcile | [W §5] | doc 11 |
| SV-33 | objc2-family standardization for new engine code ([PROPOSAL]) | [M §4.3] | ADR-002 / doc 11 |
| SV-34 | tauri 3.0 webview backend choice + wry#1767 (gtk4/webkit6) timeline | [L §1.2, item 4] | Re-audit at tauri upgrades |
| SV-35 | tauri/wry/tao full-tree WEBKIT_* env-var handling | [L §1.2, item 3] | Follow-up grep + Tauri docs |
| SV-36 | NVK vs proprietary NVIDIA Vulkan driver maturity for wgpu surfaces | [L §5, item 10] | Hardware matrix |
| SV-37 | Notarization dry run incl. drawsBackground KVC exposure | [M §6, item 5] | Phase 8 |
| SV-38 | HEVC store-id drift (9N4WGH0Z6VHM → 9nmzlz57r3t7) propagated to ADR-001 draft + user docs | [W §4.1, item 6] | ADR-001 author |
| SV-39 | WPE/cog confirmed not planned (doc 08) | [L §1.0] | doc 08 owner |
| SV-40 | lavapipe canonical doc URL (Mesa docs tree moved) | [L §5, §6.4-6] | Follow-up citation hunt |
| SV-41 | Canvas/video frame painting cost on WebKitGTK 2.52 vs 2.54 + own MotionMark runs on target HW | [L §1.5] | 2.D3 / doc 12 benchmarking |
| SV-42 | `fev` crate maintenance-activity audit (VA-API binding alternative) | [L §3.3] | doc 11 (only if considered) |

## 6. Gaps observed during consolidation (listed, not filled)

- Fedora/Arch WebKitGTK versions unqueried this run (Bodhi / Arch package API) — distro floor incomplete [L §1.1.1].
- GNOME/freedesktop GitLab (WebKit upstream issue base) unreachable (anti-bot) — Linux transparency evidence is Tauri/wry-side only [L §6.4-2].
- No canonical Apple per-SoC codec-coverage matrix page exists; per-row hardware probes are the only route [M §4.1].
- Mesa docs restructure blocked the exact AMD per-ASIC VA-API citation [L §3.2].
- No Apple-published WKWebView-vs-Safari feature exception list (unlike WebView2's documented differences page) [M §3].
- The Windows/macOS records are documentation- and crate-source-derived only (no Windows/macOS hardware in this Phase-0 sandbox); all their hardware-dependent [VERIFY]s route to the hardware runbook and Phase-2 prototypes [W, M headers].

*Clean-room note: consolidation of the three platform records only; no VLC source consulted; no new external research performed; no ADR decisions made here.*
