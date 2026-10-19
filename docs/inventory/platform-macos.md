# Platform Record: macOS (WKWebView / VideoToolbox / CoreAudio / wgpu-Metal)

- Task ID: 0.C5-b (split retry of 0.C5-I; Windows+macOS half — Linux owned by the sibling task).
- Date of research: 2026-10-19 (all URLs accessed this date from the audit sandbox).
- Feeds: ADR-001 (hw decode: VideoToolbox), ADR-003 (video-in-webview, Candidates A/B), ADR-005
  (audio backend). Notarization notes feed doc 11 §9 (Phase 8) — brief by design.
- Method note: no WebSearch/WebFetch tools in the sandbox; facts fetched directly with curl
  (developer.apple.com documentation JSON endpoints, webkit.org, crates.io API) and with the
  wry/tao/tauri/cpal published crate tarballs (Cargo.toml + src inspection). Apple doc pages are
  JS-rendered SPAs; content was pulled from
  `https://developer.apple.com/tutorials/data/documentation/<path>.json`.
- Classification reminder (policy §9): objc2-family, core-foundation, core-video, coreaudio-rs
  are Rust bindings to OS ABIs = **E2-class** standard access path; this record classifies, does
  not approve. No ADR decisions made here. Clean-room: public API names and documented behavior
  only.

## 1. Component versions (verified 2026-10-19 via crates.io API)

| Crate | Max stable | Last update | Role / classification |
|---|---|---|---|
| wry | 0.57.0 | 2026-09-08 | webview abstraction; WKWebView backend (uses objc2) |
| tauri | 2.12.4 (3.0.0-alpha.5 newest) | 2026-10-14 | app framework |
| tauri-runtime-wry | 2.12.2 | — | pins wry 0.57.0 + tao 0.37.0 (rwh_06) — verified in Cargo.toml |
| tao | 0.37.1 | 2026-09-26 | windowing; NSWindow/NSView via objc2-app-kit |
| raw-window-handle | 0.6.2 | 2024-05-17 | handle traits for wgpu surface creation |
| objc2 | 0.6.5 | 2026-10-05 | objc runtime bindings (madsmtm/objc2) — E2 |
| objc2-video-toolbox | 0.3.2 | 2025-10-04 | generated VideoToolbox bindings — E2 |
| objc2-core-video | 0.3.2 | 2025-10-04 | CVPixelBuffer / CVMetalTextureCache — E2 |
| objc2-core-graphics | 0.3.2 | 2025-10-04 | — E2 |
| objc2-metal | 0.3.2 | 2025-10-04 | MTLTexture etc. — E2 |
| objc2-app-kit | 0.3.2 | 2025-10-04 | NSView/NSWindow — E2 |
| objc2-web-kit | 0.3.2 | 2025-10-04 | WKWebView/WKConfiguration — E2 (used by wry, verified) |
| objc2-av-foundation | 0.3.2 | 2025-10-04 | — E2 (metadata-generated) |
| core-foundation | 0.10.1 | 2025-05-26 | servo core-foundation-rs (CF types) — E2 |
| core-foundation-sys | 0.8.7 | 2024-08-11 | — E2 |
| core-video | 0.6.1 | 2026-06-09 | RustAudio core-video-rs (CVBuffer wrappers) — E2 |
| coreaudio-rs | 0.14.2 | 2026-04-29 | RustAudio CoreAudio wrapper — E2 (see §5) |
| coreaudio | 0.5.0 | 2026-10-11 | DIFFERENT crate (Keegan-Wilgermein) — see §5 caution |
| cpal | 0.18.2 | 2026-08-16 | audio; macOS host = coreaudio-rs 0.14.2 + objc2-* (verified) |
| wgpu | 30.0.1 | 2026-08-22 | `metal` in default features (verified Cargo.toml) — E2 |

The whole objc2 framework-crate set is version-locked at 0.3.2 (2025-10-04) against objc2 0.6.x;
wry 0.57.0's macOS backend itself uses `objc2` + `objc2-app-kit` + `objc2-web-kit`-style
generated bindings (verified in wry source: `Retained<WryWebView>`, `objc2_app_kit::NSColor`,
`msg_send![super(webview), initWithFrame: configuration:]`).

## 2. Candidate A: WKWebView transparency + native surface overlay

### 2.1 Documented reality of WKWebView transparency

- **There is no public WKWebView API for a transparent background.** The de-facto standard is
  KVC on a private key: `setValue:forKey:@"drawsBackground"` (NSNumber NO) on the
  **WKWebViewConfiguration** before creating the webview, and/or on the webview instance for
  runtime toggling. wry 0.57.0 implements exactly this and annotates it honestly — source
  comments (verified in `wry-0.57.0/src/wkwebview/mod.rs`):
  - config level: `config.setValue_forKey(Some(&no), ns_string!("drawsBackground"))` when
    `attributes.transparent || attributes.background_color.is_some()`;
  - runtime level: `// NOTE: Private API — drawsBackground is a private KVC key on WKWebView
    instance.` (used by `set_background_color` at runtime).
  - Public-API complement: `setUnderPageBackgroundColor` (macOS 12+) is used for the overscroll
    region color ("Available: macOS 12+, iOS 15+" per wry's Apple-doc citation).
- Tauri packaging: the `macos-private-api` cargo feature historically gated this; in
  tauri 2.12.4 `src/lib.rs` documents: "Enables features only available in macOS's private APIs,
  currently the `transparent` window functionality ... **No-op since Tauri 2.12.1 because the
  APIs are always enabled now.**" → transparent windows + drawsBackground are always compiled in
  for current Tauri. Consequence: App Store review risk (private API usage) is accepted by the
  Tauri ecosystem but NOT zero — direct distribution (our Phase 8 default, notarized Developer
  ID) does not run App Review. [VERIFY] current Gatekeeper/notarization treatment of the
  drawsBackground KVC (no evidence of notarization rejection; see §6).
- wry issue trail (github.com/tauri-apps/wry):
  - #1780 (2026-07-22, closed) "refactor(macos/ios): remove fullscreen and transparent feature
    flags" — transparency un-gated in wry itself.
  - #1867 (2026-09-28, closed) "macOS 15.7: transparent window base stays black despite config
    KVC, instance _setDrawsBackground:, NSWindow clear...": on macOS **15.7** the transparent
    base rendered opaque black even with every documented + private path applied; reporter
    confirmed on 2026-09-30 that **15.8.1 fixed it with no app-side workarounds** ("Opaque red
    window placed behind ... shows through completely (2583/2583 sampled pixels)").
    → OS-point-release fragility is real; the Phase 2 prototype must pin and test macOS point
    versions, and the UI must degrade gracefully (opaque fallback).
  - #1266 (2024-05-15, closed) "Transparent WebView not working on MacOS" — older breakage.
  - #408-lineage note: macOS does not have the Windows backdrop-filter/WS_EX_LAYERED problems;
    transparency over sibling views is the *normal* macOS compositing case (contrast
    wry #1540 where the same scenario failed on Windows).

### 2.2 View hierarchy / NSView overlay mechanics (source-verified)

- wry on macOS (verified in `src/wkwebview/mod.rs`):
  - `WebViewBuilder::build(&window)` — obtains the NSWindow's content view and calls
    `ns_view.addSubview(&webview)` — the WKWebView becomes a **subview of the content view**,
    replacing nothing.
  - `build_as_child` — creates a parent NSView, `ns_window.setContentView(Some(&parent_view))`,
    then `parent_view.addSubview(&webview)`.
- Therefore Candidate A has a clean, documented-by-construction path: our own layer-backed
  (or layer-hosting) NSView — e.g. an `MTKView`/CAMetalLayer view created by wgpu — can be added
  as a **sibling subview behind the WKWebView** (`addSubview` order defines z-order; or
  `addSubview:positioned:relativeTo:`), with the webview transparent (drawsBackground=false) so
  the Metal surface shows through. This is the "native wgpu surface behind a transparent
  webview" arrangement, and it needs no private window styles beyond the drawsBackground KVC
  already required for transparency.
- Handles exposed today (all verified in wry 0.57.0 / tauri 2.12.4 sources):
  - `wry::WebViewExtMacOS::webview() -> Retained<WryWebView>` (the WKWebView itself),
    `ns_window() -> Retained<NSWindow>`, `reparent(*mut NSWindow)`.
  - `tauri::WebviewWindow`: implements `HasWindowHandle` + `HasDisplayHandle` (rwh 0.6), plus
    `ns_window() -> Result<*mut c_void>` and `ns_view() -> Result<*mut c_void>` (verified).
  - So the app can: get NSWindow → contentView → insert its own NSView subview behind
    `webview()`'s superview ordering. [VERIFY] exact insertion (contentView vs wry's parent view)
    in the Phase 2 prototype; `build_as_child`'s parent-view indirection is the cleaner hook.
- wgpu 30.0.1 on macOS creates its surface via CAMetalLayer (Metal backend; `metal` is a default
  feature). Tauri/tao windows are layer-backed; creating a *separate* layer-hosting subview for
  the wgpu surface is standard practice for NSView overlays [VERIFY wgpu 30 surface-from-raw-
  NSView behavior in prototype; wgpu historically creates a CAMetalLayer on the NSView].

### 2.3 Known artifacts / risks (macOS Candidate A)

1. drawsBackground is private-KVC → App Store distribution would be blocked-in-principle; direct
   notarized distribution unaffected in practice [VERIFY §6].
2. macOS point-release regressions (15.7 black-base bug, wry #1867) — pin OS versions in the
   test matrix; provide opaque fallback.
3. WKWebView composites its own layer; any effect requiring the webview to *sample* what is
   behind it (CSS backdrop-filter over the video) has historically had WebKit quirks [VERIFY]
   — same caution as Windows but for different reasons; Figma-fidelity UI (doc 08) should avoid
   backdrop-filter over the video region on all platforms until proven.
4. Traffic-light (window buttons) insets: wry exposes `set_traffic_light_inset` for
   hidden-titlebar decorated windows; with a fullscreen video surface the UI plan must place
   controls away from the native buttons (doc 08 concern, recorded not decided).

## 3. Candidate B on WKWebView: WebCodecs status

- WebKit shipped the **video portion** of WebCodecs in Safari 16.4: "Safari 16.4 adds support
  for the video portion of Web Codecs API" — WebKit blog, "WebKit Features in Safari 16.4",
  https://webkit.org/blog/13966/webkit-features-in-safari-16-4/ (2026-10-19).
- Safari 26.0 completed it: "Safari 26.0 expands support for WebCodecs API by adding
  AudioEncoder and AudioDecoder" — https://webkit.org/blog/16935/webkit-features-in-safari-26-0/
  (2026-10-19).
- Cross-check, caniuse `webcodecs` data (fetched 2026-10-19): Safari 16.4–18.2 = partial
  ("Video-only support"), Safari 26.0 = full; MDN BCD `api.VideoDecoder`: Safari 16.4+, iOS
  mirror. macOS version mapping: Safari 16.4 ≈ macOS Ventura 13.3; Safari 26 ≈ macOS Tahoe 26
  (2025).
- WKWebView uses the system WebKit, so `VideoDecoder`/`VideoFrame` are expected in any
  WKWebView on macOS ≥ 13.3, full WebCodecs (incl. audio) on macOS ≥ 26 [VERIFY on hardware:
  WKWebView parity with Safari for these APIs has no Apple-published exception list, unlike
  WebView2's documented differences page].
- Frame path: `VideoFrame` → `canvas` 2D / `createImageBitmap` (video-only parts are the ones we
  need — audio would be engine-side anyway per ADR-003/005 split). WebGPU also ships in Safari 26
  (WebKit blog "Unlock GPU computing with WebGPU at WWDC25", same 26.0 post) — a GPU fast path
  for frame upload may exist on macOS 26+ only [VERIFY importTexture support].
- Bottom line for ADR-003: Candidate B is viable on modern macOS for *video decode + canvas*, but
  the WebCodecs surface predating 26.0 is video-only (fine — audio is not B's job), and the
  oldest supported macOS target gates everything (support floor decision belongs to doc 01/11).

## 4. HW decode via VideoToolbox (ADR-001 macOS path)

### 4.1 Framework facts (Apple documentation, 2026-10-19)

- VideoToolbox: "a low-level framework that provides direct access to hardware encoders and
  decoders... services for video compression and decompression, and for conversion between
  raster image formats stored in CoreVideo pixel buffers" —
  https://developer.apple.com/documentation/videotoolbox (docs JSON).
- `VTDecompressionSession` — "A decompression session supports the decompression of a sequence
  of video frames. The session is a reference-counted Core Foundation (CF) object." —
  https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession
- Runtime hw probe: the framework documents a function described as "Returns a Boolean value
  that indicates whether the current system supports hardware decode for the specified codec"
  (`VTIsHardwareDecodeSupported`) — seen in the VideoToolbox symbol index; use it for
  feature detection instead of hardcoding a per-model matrix [VERIFY exact signature spelling in
  the objc2-video-toolbox crate docs].
- Codec availability picture (each row's per-OS/per-SoC truth must be probed at runtime; rows
  marked [VERIFY] lack a single canonical Apple matrix page):
  - **H.264**: ubiquitous, hardware decode on all supported Macs (Intel + Apple Silicon).
  - **HEVC**: hardware decode on all Apple Silicon Macs and HEVC-capable Intel Macs; software
    decode available on newer OSes [VERIFY software-decode floor].
  - **ProRes**: decode support built into macOS (ProRes is Apple's own codec; used by camera/
    pro-app ecosystem) [VERIFY exact floor version].
  - **VP9**: decode available in VideoToolbox on recent macOS [VERIFY hardware vs software
    split per SoC — Safari has long played VP9 (e.g., YouTube 4K); hardware VP9 decode on
    Apple Silicon is a per-generation question].
  - **AV1**: "Safari 17.0 adds support for AV1 video on devices with hardware decoding support,
    like iPhone 15 Pro and iPhone 15 Pro Max" (WebKit blog 14445) → hardware AV1 decode exists
    on M3/A17 Pro-generation and later SoCs; a VideoToolbox software AV1 decoder on macOS 14+ /
    iOS 17 is commonly reported [VERIFY against Apple docs].
  - No store/installation friction exists on macOS (contrast Windows §4): codecs ship with the
    OS — the only variance is hardware presence, which VTIsHardwareDecodeSupported resolves.

### 4.2 API surface (names only — public API names, clean-room safe)

- Session: `VTDecompressionSessionCreate`, `VTDecompressionSessionDecodeFrame`,
  `VTDecompressionSessionInvalidate`, `VTDecompressionSessionCanBeCreatedForWidth`-family
  capability checks, output callback type (per-frame decode-completion callback/block, as the
  docs JSON summarizes).
- Probing/properties: `VTIsHardwareDecodeSupported`; `CMVideoFormatDescription` creation
  (CoreMedia: `CMVideoFormatDescriptionCreateFromH264ParameterSets` /
  `...FromHEVCParameterSets` for AnnexB→sample-entry conversion [VERIFY exact current names]).
- Pixel flow: decoded frames arrive as `CVPixelBuffer` (CoreVideo) in `CVImageBuffer` callbacks;
  zero-copy to Metal via `CVMetalTextureCache` (`CVMetalTextureCacheCreate`,
  `CVMetalTextureCacheCreateTextureFromImage`) → `MTLTexture` (CoreVideo/Metal interop,
  objc2-core-video + objc2-metal) — the standard macOS zero-copy video→GPU path [VERIFY wgpu 30
  import of an existing MTLTexture (wgpu-hal metal has raw-texture-from-layer paths; exact API
  to be pinned in the ADR-001 spike)].

### 4.3 Rust access (E2-class, versions verified 2026-10-19)

- `objc2` 0.6.5 (2026-10-05) + `objc2-video-toolbox` 0.3.2 — symbols `VTDecompressionSessionCreate`
  and `VTIsHardwareDecodeSupported` confirmed present on docs.rs (fetched 2026-10-19).
- `objc2-core-video` 0.3.2 (CVPixelBuffer/CVMetalTextureCache), `objc2-metal` 0.3.2 (MTLTexture),
  `objc2-core-graphics` 0.3.2, `objc2-app-kit` 0.3.2 — all 0.3.2, 2025-10-04, metadata-generated
  from Apple SDK headers by the madsmtm/objc2 project.
- Legacy-path crates still maintained: `core-foundation` 0.10.1 (servo; CF types),
  `core-video` 0.6.1 (RustAudio core-video-rs, 2026-06-09). wry 0.57 itself uses the objc2
  family; **recommendation for consistency [PROPOSAL]**: standardize on the objc2 family for new
  engine code to avoid two binding stacks in the tree (final call is ADR-002/doc 11 audit).
- unsafe policy: all of the above require `unsafe` (FFI) — confined to the platform adapter
  behind an ADR with documented invariants (AGENTS §3.3).

## 5. Audio: CoreAudio (ADR-005 macOS path)

- Two live Rust options (both E2-class):
  1. **coreaudio-rs 0.14.2** (RustAudio, 2026-04-29) — "A friendly rust interface for Apple's
     Core Audio API... expose and wrap the functionality of the original C API in a zero-cost,
     safe, Rust-esque manner. If you just want direct access to the Core Audio APIs, use the
     appropriate crates of the objc2 project." (README, fetched 2026-10-19 from repo master).
     This is what cpal 0.18.2 uses on Apple targets (verified: `coreaudio-rs = 0.14.2`,
     features `core_audio` + `audio_toolbox`, plus objc2 0.6 / objc2-audio-toolbox 0.3 /
     objc2-core-audio 0.3 / objc2-core-audio-types 0.3 / objc2-core-foundation 0.3.1).
  2. Direct `objc2-core-audio` / `objc2-audio-toolbox` bindings (the README's own suggestion).
- CAUTION: a **different** crate named `coreaudio` (0.5.0, publisher Keegan-Wilgermein, updated
  2026-10-11, github.com/Keegan-Wilgermein/coreaudio) exists on crates.io — do not confuse it
  with RustAudio's coreaudio-rs; provenance/maintenance audit required before any use (doc 11).
- AudioUnit path: the classic output unit (RemoteIO/AURemoteIO subtype; modern replacement
  `AVAudioEngine` also exists via objc2-av-foundation 0.3.2). cpal's macOS host drives an
  AudioUnit render callback (coreaudio-rs `audio_unit` module) [VERIFY] whether engine output
  wants default-output-device AudioUnit vs AVAudioEngine for device-routing/SPAT features.
- Callback constraints (design-critical for the audio-output crate): the AudioUnit render
  callback runs on a **real-time audio thread owned by CoreAudio**:
  - must not allocate, lock, file-IO, or send Objective-C messages;
  - fixed signature, returns frames into a caller-provided buffer list;
  - underruns (kAudioUnitRenderErr_... / failure to fill) are audible glitches.
  → the engine's audio ring-buffer/SPSC design must hand pre-filled buffers to this callback;
  event-driven wakeup is *inverted* vs WASAPI (CoreAudio pulls; WASAPI events push). Recorded as
  a cross-platform design constraint for ADR-005.
- Pro-level extras: sample-rate switching / exclusive-ish "hog mode", channel layouts, and
  device change notifications are all CoreAudio surface (AudioObjectID property APIs) reachable
  through the same E2-class crates [VERIFY scope with ADR-005].

## 6. Notarization / signing (brief — detail owned by doc 11 §9, Phase 8)

- Apple notary service: "Notarize your macOS software to give users more confidence that the
  Developer ID-signed software you distribute has been checked by Apple for malicious
  components. Notarization of macOS software is not App Review." —
  https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
  (docs JSON, 2026-10-19).
- Hard requirements listed by Apple: code-sign all executables with a **Developer ID
  Application** certificate (not ad hoc/local/dev), enable the **hardened runtime**, and staple
  or online-resolve the notarization ticket for Gatekeeper. "Beginning in macOS 10.15, all
  software built after June 1, 2019, and distributed with Developer ID must be notarized."
- Since macOS 10.14.5/10.15 the practical rule: unsigned/un-notarized downloads get
  Gatekeeper-blocked UI. Universal (arm64+x86_64) builds require signing each slice.
- Notarization does not reject private-API *use* per se (it is not App Review) — relevant to the
  drawsBackground KVC in §2.1 [VERIFY with a Phase 8 notarization dry run; Apple's automated
  checks target malicious behavior, but the final word on private API risk for any future
  App Store channel stands].
- Rust-specific: `codesign` runs on the final .app bundle incl. all embedded dylibs; our
  100%-Rust engine means no third-party dylibs to herd (an audit advantage for doc 11).

## 7. Verdict block (evidence summary — NOT an ADR decision)

ADR-003 (video-in-webview) macOS feasibility:
- Candidate A (Metal/wgpu surface behind transparent WKWebView): **structurally the cleaner
  platform**. Transparency via the (private-KVC) drawsBackground path is what wry/Tauri already
  ship enabled-by-default (Tauri 2.12.1+ no-op feature gate); WKWebView is an ordinary subview,
  so a sibling layer-hosting NSView with a CAMetalLayer (wgpu) sits behind it with no child-HWND
  z-order problem (the exact failure mode that bit Windows in wry #1540, where macOS worked).
  Risks: private-API provenance (App Store only), OS point-release regressions (wry #1867,
  macOS 15.7 black base — fixed in 15.8.1), [VERIFY] wgpu-30 surface-from-NSView mechanics.
- Candidate B (WebCodecs frame streaming): video-decode WebCodecs exists from macOS 13.3+
  (Safari/WebKit 16.4, video-only), full API from macOS 26 — WKWebView parity presumed but
  [VERIFY] on hardware. Older floors exclude B entirely; audio is engine-side either way.
- Feasibility ranking evidence (both platforms): A is architecturally better supported on macOS
  than on Windows; B is uniformly available on Windows (evergreen Chromium) but version-gated on
  macOS. ADR-003 may rationally end up A-on-macOS + A-or-B-on-Windows; that choice is the Lead's.

ADR-001 (hw decode) macOS path readiness:
- VideoToolbox is fully reachable in pure Rust: objc2 0.6.5 + objc2-video-toolbox 0.3.2
  (VTDecompressionSessionCreate / VTIsHardwareDecodeSupported confirmed on docs.rs) + CoreVideo/
  Metal interop via CVMetalTextureCache (objc2-core-video 0.3.2 + objc2-metal 0.3.2).
- No install friction (codecs ship with the OS — contrast Windows HEVC $0.99 extension);
  hardware presence is the only variable and is runtime-probeable (VTIsHardwareDecodeSupported).
- Codec coverage expectation: H.264/HEVC/ProRes solid; AV1 hardware only on newest SoCs
  (Safari 17-era hardware decode, e.g. iPhone 15 Pro class); VP9 details [VERIFY].
- Top technical unknown: zero-copy CVPixelBuffer → wgpu (MTLTexture import into wgpu 30)
  [VERIFY] in the ADR-001 spike; CPU-copy fallback is straightforward.

Open items [VERIFY] (macOS):
1. WKWebView WebCodecs availability/parity on the minimum supported macOS (13.3? 14? 15? —
   support-floor decision pending in doc 01) — needs a hardware probe matrix.
2. VP9 hardware/software decode split per Apple Silicon generation (VideoToolbox).
3. Software AV1 decode in VideoToolbox (macOS 14+) — confirm exact floor + performance class.
4. wgpu 30 surface creation from a custom layer-hosting NSView + MTLTexture import path.
5. drawsBackground KVC: notarization dry-run (Phase 8) and any residual private-API exposure in
   the WKWebViewConfiguration vs instance variants.
6. macOS 15.7-class point-release regressions: add OS-version pinning to the QA matrix (evidence:
   wry #1867); confirm our minimum macOS avoids known-bad point releases.
7. coreaudio vs coreaudio-rs crate disambiguation + audit (doc 11), AudioUnit-vs-AVAudioEngine
   scope decision for ADR-005 (device routing, hog mode, channel maps).
8. Real-time callback contract (no alloc/lock/Obj-C) to be encoded as a tested invariant in the
   audio-output crate design (ADR-005).
