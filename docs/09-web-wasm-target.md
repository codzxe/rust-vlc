# 09 — Web/WASM Target Specification

| Field | Value |
|---|---|
| Task | 1.B9 (Wave 1, planning document — specification, no implementation) |
| Status | implemented (awaiting Lead review) |
| Authoritative source | `docs/MASTER-PROMPT.md` (MP) — primarily §10 (Web target), §2 (fixed decisions), §3.5/§3.9 (command layer), §11 (performance), §12 (security), §14 (known limits), §16 Phase 7 (web build parity) |
| Consumes | `docs/01-architecture.md` (crate registry, ADR-009), `docs/02-phase0-audit-plan.md` §4 (feature-matrix schema, status vocabulary), `docs/03-engine-plan.md` §4.9/§4.11/§9 (AudioSink, RenderSurface, threading), `docs/04-codec-format-roadmap.md` (per-codec web columns, WebCodecs ADR), `docs/05-command-layer.md` §5.2 (wasm binding), §7 (conformance) |
| Owning code (future) | `apps/web` (host page + worker bootstrap), `ui/` (shared, via `@lumen/wasm-client`), wasm-capable cfg paths in `crates/*` |
| Parallel work | `docs/08-ui-figma-fidelity.md` was **absent** when this document was written (parallel Wave B agent); UI-sharing statements here are derived from MP §2/§7 and doc 01 and must be reconciled with doc 08 §8 when it lands. |
| Verification date | 2026-10-19. Browser-support facts verified against `@mdn/browser-compat-data` (unpkg latest snapshot, fetched and queried programmatically on that date); Rust target facts against the rustc book (doc.rust-lang.org/nightly/rustc/platform-support). Facts that could not be checked live are marked `[VERIFY]`. |

**Terminology.** *Web build* — the Lumen product delivered as a static web page: the Rust engine compiled to WebAssembly, driven by the shared React+TS UI. *Engine worker* — the dedicated Web Worker hosting the wasm engine. *Isolated page* — a page for which `crossOriginIsolated === true` (COOP/COEP served), prerequisite for `SharedArrayBuffer`.

---

## 1. Target definition (Trace: MP §2 "Web build", §10 bullet 1)

**What the web build is.** One product, same engine: the crates under `crates/` — media-types through playlists, plus commands, product-config and settings-schema (the full registry is doc 01 §3) — compiled to `wasm32` and orchestrated by `apps/web`. The engine owns exactly what it owns on desktop: probing, demuxing, decoding, filtering, clocking, queueing, subtitle parsing, playlists, and every media decision (MP §2 bullet 1). The browser provides only *substrates*: byte sources, a rendering surface, an audio device, and a scheduler. No media decision ever moves into JavaScript.

**What the web build is not.** It is not a `<video>`-tag player. The `<video>`/MediaSource path is explicitly rejected as the primary pipeline because it forfeits engine-owned demux/decode decisions (and is rejected for the same clean-room honesty reasons as any other hidden native engine). It may appear only as a documented comparison baseline in tests, never in the product. [PROPOSAL — rejection rationale; the Lead may revisit if engine throughput on low-end mobile proves inadequate, but that would be an ADR-level change.]

**The four pillars** (all fixed by MP §2/§10; this document specifies each):

1. **Same engine, wasm-compiled** — behind a typed JS/TS binding (`@lumen/wasm-client`, doc 05 §5.2) exposing the *same* command catalog, envelope, events and truth rules as the Tauri host (MP §3.5: "one command layer").
2. **Shared UI** — the `ui/` React+TS workspace is consumed unmodified by `apps/web`; only the client package differs (`@lumen/wasm-client` vs `@lumen/client`). §7 and doc 08 §8 (pending) cover the seam.
3. **Feature-matrix web column** — every feature carries `status.web` with the vocabulary of doc 02 §4.3 (`not_started`, `in_progress`, `implemented`, `verified`, `partial`, `unsupported`, `blocked`); `unsupported`/`blocked` require `blocked_reason`, `partial` requires `notes` (MP §10: "Mark such features blocked or unsupported for web with the reason").
4. **Honest capability reporting** — the web host reports what it can actually do *right now, on this page, in this browser*: renderer backend, threaded-or-not, seekability of the current source. This mirrors the hardware-decode truthfulness rule (MP §2; doc 05 `system.get-engine-info` `decoders[].hardware/backend/active`).

**Success criterion (Phase 7 gate, MP §16).** "The web and desktop builds both play real media through the same command layer" — concretely: the doc 05 §7 conformance suite passes on a real browser host, and at least the Phase 2 vertical slice (AVI+MJPEG+PCM per doc 03 §7 [awaiting Lead confirmation]) plays to canvas + WebAudio via the typed client.

---

## 2. Build & toolchain (Trace: MP §10, §13 local gate "Tauri and WASM builds")

### 2.1 Target selection: `wasm32-unknown-unknown` — recommendation

The engine compiles to **`wasm32-unknown-unknown`**. The alternatives were evaluated against the rustc book (verified 2026-10-19, https://doc.rust-lang.org/nightly/rustc/platform-support.html):

| Candidate | std threads | std time/fs | Browser fit | Verdict |
|---|---|---|---|---|
| `wasm32-unknown-unknown` | `std::thread::spawn` **panics**; no syscalls ("std::fs always return errors") | none — host shims required | The target every browser-facing Rust library actually targets: `wgpu`'s web backend, `web-sys`, `wasm-bindgen`, `wasm-bindgen-rayon` | **Recommended** |
| `wasm32-wasip1` | `spawn` "always return[s] an error" unless the separate `wasm32-wasip1-threads` target is used | full-ish (clock, fd, preopens) | WASI syscalls don't exist in browsers; requires a JS syscall shim (wasi.js-style) heavier than direct wasm-bindgen shims; `wasip1-threads` needs a wasi-threads host implementation no browser ships | Rejected for v1 |
| `wasm32-wasip1-threads` | yes, via wasi-threads proposal | as wasip1 | Tier 3/nightly, `target_feature = "atomics"` nightly-only, "the Rust project doesn't run any tests for this target"; on the web you still hand-roll Worker spawning with SAB | Rejected; revisit if it stabilizes |
| `wasm32-wasip2` | no threads today (WASI 0.2 has none; the page documents no threading) | component-model std | Designed for server composition; browser use requires `jco`-generated bindings that duplicate our typed command layer with WIT machinery; WASI 0.3 (native async + threads) not yet the shipped baseline [VERIFY current status before any switch] | Rejected for v1; re-evaluate via ADR when WASI 0.3 + browser component runtimes mature |

**Justification.** The engine's platform surface is already trait-isolated (doc 03: `AudioSink`, `RenderSurface`, the input access trait); on the web those traits are implemented by thin wasm-bindgen adapters inside `apps/web`, so the engine core never needs OS syscalls. What `unknown-unknown`'s missing std surface costs: (a) `std::time::Instant` — replaced by an injectable time source backed by `performance.now()` (doc 03's media-clock already takes its truth from the audio sample counter; wall-clock is fallback only); (b) `std::thread` — §3; (c) `std::fs`/`std::net` — never touched on any platform (I/O lives behind the input trait, §6). This target also keeps the wgpu-on-web path — the only strategy that reuses `renderer-wgpu` unforked (doc 03 §4.11).

**Engine-side discipline required (binding):** engine crates must not call `std::thread::spawn`, file, socket or clock APIs outside the adapter traits; the architecture check (doc 01 xtask `check-architecture`) gains a wasm-lint rule scanning for those calls outside `apps/web` and approved adapters. [PROPOSAL — rule detail; consistent with doc 01's boundary scan.]

### 2.2 wasm-bindgen vs raw imports [PROPOSAL]

**Recommendation: `wasm-bindgen` (+ `web-sys`/`js-sys`), not hand-written `extern "C"` imports.**

- *For:* typed JS interop with `#[wasm_bindgen]` exports/imports; first-class support by wgpu's web backend and `wasm-bindgen-test` (already assumed by doc 05 §7.2 `WasmNodeHost`); `wasm-bindgen-rayon` provides the Worker-pool threading path (§3.3) with `SharedArrayBuffer`; the project's TS types are schema-generated (doc 05 §5.1) so wasm-bindgen only marshals the *envelope*, which is a narrow, tested surface.
- *Against raw imports:* we would hand-maintain the JS side of every import (memory growth, string marshaling, GC rooting of host objects like `AudioWorklet` handles) — exactly the kind of unverifiable glue the dependency policy distrusts. The size overhead of wasm-bindgen's support JS is a few KB post-gzip, negligible against the engine [VERIFY measured number at Phase 2].
- **Recorded trade-off:** wasm-bindgen enters the Phase 0 dependency audit like any dependency (build-time tool + JS shim; license and maintenance recorded). If it ever fails the audit, the fallback is raw imports behind the *same* `apps/web` adapter boundary — all interop lives in `apps/web` and adapter crates, never in engine crates. [PROPOSAL — this containment rule.]

**Future option, not v1:** JavaScript Promise Integration (JSPI; V8 `WebAssembly.Suspending`/`promising`) would let wasm await host promises directly, removing the callback-trampoline layer around `fetch`. BCD shows JSPI shipped Chrome/Edge 137, Firefox 153, Safari 27 [verified 2026-10-19 — recent and mobile-incomplete; re-verify at build time]. Track as a Phase 7+ experiment behind the same adapter boundary. [PROPOSAL]

### 2.3 Bundling & serving (`apps/web`)

`apps/web` is a **static site**: no server-side code, no proxy. [PROPOSAL — details]

- Engine: `cargo build --target wasm32-unknown-unknown --features web` → `wasm-bindgen --target web` (ESM) → `wasm-opt -O` (binaryen). Panic strategy `panic=abort`, `opt-level = "z"` (profile `wasm-release`), LTO on. Panic/abort messages surface as a typed fatal `playback.error { stage: host }` event — never a silent dead worker.
- UI: the shared `ui/` workspace built by the same bundler as desktop (Vite is the working assumption across both hosts; the choice is doc 08/Lead's to confirm). The worker script is an ES module worker; `apps/web` registers it and transfers a `MessagePort` pair (§7).
- Serving requirements (documented in `apps/web/README.md` + a deploy-check script): correct `application/wasm` MIME (streaming instantiation), and the optional COOP/COEP pair for threaded mode (§3.3) — served conditionally because of the embedding trade-offs in §9.5. A capability-probe page prints the negotiated snapshot (§3.3/§4.4) for support tickets and QA — the web analog of "paste your engine info".

### 2.4 Size budget philosophy (Trace: MP §11 "bounded memory… report honestly")

Baseline + per-codec gates, enforced in CI, never aspirational:

- **Baseline budget** (engine + UI + host, gzipped): Phase 2 vertical ≤ **3 MB**; full P0 feature set ≤ **6 MB** [PROPOSAL — numbers to be pinned by the first real build; the *mechanism* is the fixed part].
- **Per-codec feature gates:** every decoder/demuxer plugin is a Cargo feature (doc 01 "built-in plugins registered via Cargo features"); the web product ships the P0 set; a `--no-default-features` build proves decoders tree-shake out of the final module (dead-code elimination + `wasm-opt`). CI asserts: building with codec *X* disabled removes X's decode symbols from the wasm (symbol-scan check; also proves no hidden dependency pulled it back in — an anti-bloat and language-policy check in one). [PROPOSAL]
- **Regression gate:** CI records size per merge; growth > 5% on the baseline file fails the job with a "justify or optimize" message. Size is printed in release notes next to parity claims (MP §17 honesty).
- **Memory budget:** wasm `Memory` maximum is set from `product.toml` `resource_limits` (MP §5); OOM surfaces as `resource-limit` errors, not a crashed tab. The engine's bounded queues (doc 03 §4.4) bound resident media bytes identically to desktop.

### 2.5 CI additions (Trace: MP §13; doc 01 "wasm target must keep compiling in CI from Phase 1")

A `web` job in the workspace CI (Linux runner), from Phase 1 onward:

1. `cargo check --target wasm32-unknown-unknown --workspace --features web` (and `--no-default-features` variant for tree-shake proof).
2. `cargo clippy --target wasm32-unknown-unknown ... -D warnings` — wasm-specific lint set (e.g., `unused_variables` etc. already workspace-wide; plus the §2.1 no-thread/fs/net scan).
3. `cargo test --target wasm32-unknown-unknown` via `wasm-bindgen-test` in Node (this *is* doc 05 §7.2 `WasmNodeHost`) — engine logic, slicing, capability errors.
4. Build + `wasm-opt` + size-budget check (§2.4).
5. Headless-browser integration job (§11) — Playwright Chromium on the runner; Firefox/Safari coverage per §8.2.
6. COOP/COEP matrix: the integration job runs the page twice — with and without the headers — asserting both capability snapshots are self-consistent and truthfully reported (§3.3).

---

## 3. Threading model on the web (Trace: MP §10 "Threaded WASM needs COOP/COEP headers"; doc 03 §9)

### 3.1 The constraint, stated plainly

`wasm32-unknown-unknown` cannot `std::thread::spawn` (verified, §2.1). Doc 03's engine is thread-role-based: session thread, input reader, demux thread, decode worker pool, render thread, real-time audio callback. On the web, **v1 runs all engine roles on one thread** — a dedicated Worker — with an explicitly cooperatively-scheduled mapping; **v2 (Phase 7)** adds true wasm threads when the page is cross-origin isolated. Both modes are the *same engine code* behind a scheduler facade; capability is discovered at runtime, never assumed.

### 3.2 v1: engine-in-a-Worker, cooperative scheduling

Topology (matches doc 05 §5.2's Worker plan):

```
main thread                        engine Worker (wasm32)
┌────────────────────┐   request/  ┌─────────────────────────────┐
│ ui/ (shared React) │◄──response──► wasm engine                  │
│ @lumen/wasm-client │   MessagePort│  cooperatively scheduled:   │
│ canvas present     │◄──events────┤  open/probe/demux/decode/   │
│ (WebGPU/WebGL2)    │   frames ───►│  filters/clock — one run   │
│ WebAudio graph     │  (ImageBitmap│  loop, sliced               │
└────────────────────┘   transfer) └─────────────────────────────┘
```

- **Why a Worker at all if single-threaded?** The main thread belongs to the UI: React rendering, input and the canvas present loop must never starve behind a decode slice, and a wasm OOM/abort in the worker must not blank the page. The Worker is also the natural seed of the threaded future (§3.3): the pool grows *around* the engine worker, which becomes worker #0.
- **Cooperative scheduling [PROPOSAL — mechanism]:** doc 03's channel topology is preserved, but on web the channels are same-thread queues driven by a run loop: `pump(deadline)` visits roles in priority order (audio fill → demux → decode batch → events flush), each with a bounded slice (default 4 ms audio-critical / 8 ms total per pump [PROPOSAL — numbers]); when the budget is exhausted or a role has no ready work, control returns to the Worker event loop. Blocking `recv` is forbidden in web builds — `try_recv` only; the §2.1 lint enforces this. The run loop is re-armed by host timers (`setTimeout(0)`/message ticks) and by audio-buffer-low signals (§5.2). This is doc 03's ownership model with thread *roles* reinterpreted as tasks; the single-writer invariants (S1–S4) are structurally unchanged because there is exactly one writer anyway.
- **Frames leave the worker as transferables:** decoded video frames are wrapped (zero-copy where the layout allows) into `ImageBitmap`s or `ArrayBuffer`-backed plane sets and transferred to the main thread (doc 05 §5.2 "transferable frame buffers"); the present loop and GPU work live on the main thread (§4.2). Audio flows the same direction (§5).
- **Truth rule:** in v1 the engine's capability snapshot says `threading: single`, decode parallelism is 1, and the UI's performance surfaces (dropped-frame counters, `system.get-engine-info`) report that honestly. No UI copy may claim "multi-core accelerated" on a single-threaded page.

### 3.3 v2: threaded WASM via SharedArrayBuffer + COOP/COEP

When the page is cross-origin isolated, the engine may spawn a Worker pool where every worker instantiates *the same* wasm module with a shared `SharedArrayBuffer`-backed memory (the `wasm-bindgen-rayon` pattern). Threads then behave like real threads: the decode pool (doc 03 §4.5), input and demux roles can run on workers; atomics-based SPSC rings replace message-passing on the hot paths.

**Required headers (documented deploy requirement, served by `apps/web` and any embedder):**

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Every subresource (wasm, JS, fonts, media fetched by the *page* — not engine-fetched media, which is CORS anyway) must then carry `Cross-Origin-Resource-Policy: cross-origin` or be CORS-fetched. `COEP: credentialless` is a lower-friction alternative [VERIFY current cross-browser support, esp. Safari/iOS, before offering it; it has known gaps with credentialed subresources].

**Browser support** (BCD, verified 2026-10-19): `SharedArrayBuffer` and wasm threads/atomics are exposed in Chrome/Edge 74+/79+, Firefox 79+, Safari 15.2+, iOS Safari 15.2+ — **but** all engines gate SAB on cross-origin isolation in practice (Chrome since 92; Firefox on desktop; WebKit has progressively enforced it — treat isolation as mandatory everywhere and re-verify per release: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer). `Atomics.waitAsync` (needed for park/unpark of wasm threads from JS): Chrome/Edge 90+, Firefox 145+, Safari 16.4+ [verified]. **iOS is the constraint platform:** without the headers there is no SAB on iOS Safari; with them, embedding scenarios that pull non-CORP third-party resources break (§9.5).

**Capability negotiation and graceful degradation (binding design):**

1. On boot the host reads `crossOriginIsolated` (BCD: Chrome/Edge 87+, Firefox 72+, Safari 15.2+ [verified]).
2. If true → pool size = `min(product.toml web.max_worker_threads, hardwareConcurrency)` [PROPOSAL formula]; snapshot says `threading: pooled(n)`.
3. If false → single-threaded mode (§3.2); snapshot says `threading: single`; the **capability is reported, not papered over**: the settings UI shows "Multi-threaded decoding requires site isolation headers" (localized string key, not a lie), and perf-sensitive features (e.g. 4K software decode) degrade by *policy*: lowered decode resolution is **not** silently applied — it is an explicit user setting with honest labeling, defaulting off [PROPOSAL].
4. The doc 05 catalog needs a machine-readable home for this: extend `EngineInfo` with `host_capabilities { threading, renderer_backend, ... }` **or** add `system.get-host-capabilities`. Additive under doc 05 §6 rules; decision requested (§12, OQ-2). [PROPOSAL]
5. Feature-matrix rows for threaded-wasm perf carry `web = "partial"` with notes naming the COOP/COEP condition — never `verified` on the strength of a single-browser lab run.

---

## 4. Rendering: WebGPU preferred, WebGL2 fallback (Trace: MP §10 bullet 2)

### 4.1 Mapping `renderer-wgpu` to the web

Doc 03 §4.11 isolates rendering behind `RenderSurface` (`resize`, `present(PresentRequest)`), with color conversion in shaders, one latest-wins present mailbox, and `RenderError::SurfaceLost` semantics. On the web:

- **WebGPU path:** the surface is a canvas whose `getContext("webgpu")` is configured by the host (format from `getPreferredCanvasFormat`, `alphaMode`, `colorSpace` [VERIFY HDR/`colorSpace` canvas options per browser — not uniformly implemented]). wgpu's web backend drives `navigator.gpu` on `wasm32-unknown-unknown`; the same WGSL color-conversion/compositing pipeline as native runs unmodified — naga translates once, the engine never sees a browser type (MP §3.6). [VERIFY wgpu's current web backend feature names and WebGL2 fallback status at implementation; wgpu's webgl path has had churn — pin the wgpu version and re-verify in Phase 2.]
- **WebGL2 fallback:** same `RenderSurface` trait over wgpu's GLES/WebGL2-on-web path. WebGL2 is the universal floor (BCD, verified 2026-10-19: Chrome 56+, Edge 79+, Firefox 51+, Safari 15+, iOS 15+). The fallback must be *feature-equivalent for presentation* (YUV→RGB, scaling, aspect, subtitle alpha-compositing) even if slower; anything the fallback genuinely cannot do (e.g. specific texture formats) is reported through the same typed error path, never silently dropped.
- **Where rendering runs:** v1 presents on the **main thread** (frames arrive from the engine worker as transferables, §3.2) — this honors doc 05 §5.2's "transferable frame buffers to the main-thread canvas". Phase 7 option: move the canvas into the engine worker via `OffscreenCanvas` (BCD: Chrome 69+, Firefox 105+, Safari 16.4+ [verified]) so GPU present competes with UI even less, and/or a third dedicated render worker. [PROPOSAL — decide by measurement, not preference.]

### 4.2 Frame upload paths

| Source of pixels | Path to GPU | Copies | Notes |
|---|---|---|---|
| Software-decoded frame in wasm linear memory | `writeTexture`/mapped-staging from the wasm-side plane buffers; planes uploaded as textures, conversion in shader | 1 (wasm mem → GPU) | The single allowed CPU→GPU copy of doc 03 §4.11(e). |
| `ImageBitmap` transferred from the engine worker | `copyExternalImageToTexture` | 0 additional on main thread | Zero-copy handoff across the worker boundary; bitmap is pre-tiled by the browser. |
| WebCodecs `VideoFrame` (hardware decode, §10) | `importExternalTexture` → `GPUExternalTexture` in the draw pass, or `copyExternalImageToTexture` into an owned texture | 0 (external) / 1 (owned) | `GPUExternalTexture` is single-frame-lifetime — import per present. [VERIFY: `GPUDevice.importExternalTexture` accepting WebCodecs `VideoFrame` — BCD lists `GPUExternalTexture` at parity with WebGPU support (Chrome/Edge, Firefox 141, Safari 26 [verified 2026-10-19]); test per browser in Phase 7.] |

`requestVideoFrameCallback` (Chrome 83+, Firefox 132+, Safari 15.4+ [verified]) paces the present loop against the browser compositor on the main thread; `requestAnimationFrame` is the fallback.

### 4.3 DPR, scaling, resize

`canvas.width = round(cssWidth × devicePixelRatio)` recomputed on `ResizeObserver` + `devicePixelRatio` change events; aspect/crop math is the same pure, unit-tested code as native (doc 03 §4.11(f)). Presenting at DPR 1 on low-power mode is a user setting (default off, honestly labeled) [PROPOSAL].

### 4.4 Capability negotiation (truthful, mirroring the hw-decode rule)

Boot sequence: (1) probe `navigator.gpu`; (2) `requestAdapter({ powerPreference: "high-performance" })`; (3) on failure or adapter/device loss → WebGL2 context probe; (4) on failure → typed fatal error `renderer-unavailable` and the UI shows the unsupported state (MP §7 "clear … unsupported and error states"). The **active backend is reported** in the capability snapshot and `system.get-engine-info` (`decoders[].backend` analog for the renderer), and the settings page states e.g. "Rendering: WebGPU" / "WebGL2 (fallback)". A device lost mid-session maps to `RenderError::SurfaceLost` → audio-only degrade with an explicit event (doc 03 §4.11(d)) — never a frozen last frame pretending to play.

Current WebGPU availability (BCD, verified 2026-10-19): Chrome/Edge 113+ (Windows/macOS/ChromeOS; Linux from 144 with Intel Gen12+ per current BCD notes), Chrome Android 121+, **Firefox 141+ (Windows only)**, **Safari 26+ incl. iOS 26**. Firefox Android: none. Consequence: **WebGL2 remains a first-class, tested path**, not an afterthought — on Firefox-Android-class browsers it *is* the renderer. Feature-matrix `vout-webgpu` row tracks per-browser verification evidence.

---

## 5. Audio output: WebAudio (Trace: MP §10 bullet 2 "WebAudio-compatible output")

### 5.1 Mapping `audio-output` to AudioWorklet

Doc 03 §4.9's traits map as: `AudioSink` → a host-owned `AudioWorkletNode` + `AudioContext`; `RenderCallback::render(out)` → the worklet processor's `process(inputs, outputs)` on the browser's real-time audio thread; `sample_counter()` → the processor's played-sample counter. The worklet module is a **tiny, fixed, hand-audited JS shim** (ring read, fill, silence-on-underrun, counter bump — a direct transcription of doc 03 §4.9(c)'s allowed operations); no engine code runs inside it. [PROPOSAL — shim is generated from a template with the ring constants, kept under review like platform adapters.]

- **Transport to the worklet, threaded mode:** the SPSC ring lives in a `SharedArrayBuffer`; engine worker writes (Atomics-backed), worklet reads. Same wait-free rules as native; doc 03's RT prohibitions (no locks/alloc/syscalls in the callback) carry over verbatim — the browser enforces some of this itself (worklets that misbehave get killed by the audio thread watchdog).
- **Transport, single-threaded mode (no SAB):** message-passing — the engine worker posts `Float32Array` chunks (transferred, not copied) to a main-thread `MessagePort` routed into the worklet node's port; the shim keeps a bounded prebuffer (target ~2× the worklet quantum [PROPOSAL — number]) and reports underruns identically. `Atomics.waitAsync` is **not** usable without SAB, so backpressure signaling in this mode is port-message based.
- **Clock:** the media-clock's anchor comes from the worklet's played-sample counter (posted back via port or written to SAB) — audio-master on web, exactly the native truth source (doc 03 §4.7). Wall-clock (`AudioContext.currentTime`, `performance.now()`) is only the fallback when no audio track exists.

### 5.2 Rate, drift, latency

The `AudioContext` runs at the device rate (48 kHz typical), the media at its own rate; `audio-engine`'s conversion stage resamples once (doc 03 §4.8) — there is no second hidden resampler. Drift between the worklet counter and the video present clock is handled by the same anchor-refresh policy as native (doc 03 §4.7); `AudioSink::latency()` is mapped to `AudioContext.baseLatency + outputLatency` where exposed [VERIFY per-browser `outputLatency` support]. The pump arming rule of §3.2 (audio-buffer-low wakes the run loop) replaces the native audio thread's pull.

### 5.3 Autoplay policy (truthful, never fake playing)

Browsers start `AudioContext` suspended without a user gesture (Chrome https://developer.chrome.com/blog/autoplay/, WebKit https://webkit.org/blog/7734/auto-play-policy-changes-for-macos/, Firefox autoplay-blocking — policies differ per browser and version [VERIFY at test time per §8]). Binding design:

- `playback.play` is **accepted**; the engine enters its normal pre-play path but if the context is suspended-and-not-gesture-authorized it parks in a distinct, surfaced condition — proposed event `playback.gesture-required` (additive per doc 05 §6) and state presentation as *paused-pending-gesture*, an explicit substate rather than a lie. [PROPOSAL — event shape; see §12 OQ-3.]
- The UI shows an unambiguous "tap to start audio" affordance; the gesture calls `resume()` (via the host shim) and playback truly starts, with `session.state-changed → playing` emitted only when audio actually flows (truth rule C-series, doc 05 §1).
- **Never fake playing:** no muted-autoplay trick is used to force progress while claiming playback; a video-only media may present frames, but if audio is gated the state says so.

---

## 6. I/O & access layer on the web (Trace: MP §10 bullet 1, §12)

### 6.1 Source kinds and the access abstraction

The input crate's byte-source abstraction (doc 03 §4.1) gains two web adapters, selected by `SourceUri` variant; no engine change:

- **`WebUrl` (http/https):** `fetch` with a `ReadableStream` body reader feeding the input ring. **Seek = cancel + re-`fetch` with a `Range` header**; seekability is *detected, not assumed*: a probe (`HEAD` or ranged `GET`) checks for `206 Partial Content` / `Accept-Ranges: bytes`. If absent, the source is reported unseekable — `media.get-info` says so, `playback.seek` returns `not-seekable` (doc 05 §2), and live-style forward-only playback rules apply. Redirects follow browser policy; credential policy: `omit` by default, `include` only via explicit user setting [PROPOSAL — mirrors doc 05's redirect/credential policy rule]. Note: `Range` is not a CORS-safelisted header — every ranged seek on cross-origin media triggers a preflight; servers must answer `OPTIONS` with `Access-Control-Allow-Headers: Range` (§9.1).
- **`Blob`/`File` (local files without filesystem access):** drag-and-drop and `<input type=file>`/picker hand the engine a `File`; the adapter serves bytes via `slice()` + `arrayBuffer()` — fully seekable, and the file never leaves the page (no upload, ever — this is a privacy property we state in the UI). `session.open` and `subtitle.add-file` accept the blob variants of doc 05 §5.2; large blobs are referenced, not cloned [PROPOSAL — param variant detail: the TS client transfers the `File` handle through the worker port by structured clone; the engine sees an opaque `BlobId` + size + seek/read calls].

Matrix rows: `access-http` (web column tracks Range-seek semantics), `access-file-blob` (Lumen-only row, `vlc_name` empty). The File System Access API (Chrome/Edge only — `showOpenFilePicker` is `NO` on Firefox/Safari, verified 2026-10-19) may later add a real directory-source on Chromium; `OPFS` may serve as engine cache storage [PROPOSAL — both Phase 7+].

### 6.2 Streaming protocols

HLS/DASH (Phase 6) run *engine-side* over the fetch adapter — the demuxer-level logic is shared with desktop; only the byte transport differs. The playlist crate consumes m3u8/XSPF/etc. from the same sources. WebTransport is *not* planned for v1 [PROPOSAL — revisit for low-latency use cases; would need its own access adapter and ADR].

### 6.3 Blocked capabilities — typed, not silent

Per MP §10 and doc 05 §5.2 "capability gaps are typed, not silent": raw UDP, RTSP/RTP, optical discs (DVD/Blu-ray/CD-DA), DVB tuners and direct capture hardware have **no browser substrate**. Every corresponding command (`session.open` with those `SourceUri` variants, `device.*` enumeration beyond the single WebAudio device) returns `unsupported-for-host` naming the capability; the UI rejects such URLs client-side *with the same localized message*, and menus grey out from data (doc 05 §6.4). Capture (webcam/mic/screen) has a browser substitute — `getUserMedia`/`getDisplayMedia` (universally available; verified) — feeding a `MediaStream` source adapter in a later phase (§10), since `MediaStream`→engine plumbing (pull-based frames via `VideoFrame`/`AudioData` from a `MediaStreamTrackProcessor`) is WebCodecs-adjacent [VERIFY `MediaStreamTrackProcessor` availability outside Chrome — not in the BCD set fetched; treat as Chromium-first and gate the feature accordingly].

---

## 7. Command layer binding on web (Trace: MP §3.5; doc 05 §5.2, §7)

**`@lumen/wasm-client`** is the web twin of `@lumen/client`: same schema-generated types (doc 05 §5.1 codegen — no hand-written TS), same promise/event API, same staleness filter and truth rules, implemented over the Worker channel instead of Tauri IPC.

- **Transport.** The client spawns the engine Worker, then wires one `MessageChannel` pair: *downstream* — request envelopes (`{requestId, cmd, args}`) via `postMessage`; *upstream* — responses and the full event stream (doc 05 §4.4: in-process hosts deliver all events; filtering is client-side `on(eventId, filter)`). Frame/audio buffers never share this port — they travel on dedicated transfer-only ports so a large frame cannot reorder behind a burst of commands [PROPOSAL — port separation detail].
- **Lifecycle.** Boot: `new Worker(..., {type:"module"})` → engine `init` → handshake reply carrying `EngineInfo` + capability snapshot (§3.3/§4.4) → client transitions `connecting → ready`. Teardown: `system.shutdown` (graceful, closes sessions, flushes events) with a hard `worker.terminate()` deadline (2 s default [PROPOSAL]). Worker crash (OOM/abort): typed fatal event `system.host-error { stage: host, kind: worker-crashed }` [PROPOSAL — event name], UI offers reload; no silent respawn that would lose session state.
- **Catalog parity.** Identical IDs, envelopes, ordering guarantees, `gen` staleness semantics (doc 05 §4). Web-specific behaviors live in the *host*, not the contract: `unsupported-for-host` errors (§6.3), blob param variants (§6.1), `video.set-fullscreen` via the Fullscreen API with its gesture constraint (doc 05 already records this), `audio.set-device` limited to the single WebAudio device.
- **Conformance on web.** Two hosts, per doc 05 §7.2 and §11 of this doc: `WasmNodeHost` (Node, `wasm-bindgen-test`) covers engine logic, slicing, capability errors; a **`WasmBrowserHost`** (headless real browser) covers the full stack — Worker transport, event ordering across the port, rendering, autoplay. The suite's scenarios and truth-rule assertions (doc 05 §7.1/§7.3) run unmodified: the web binding must pass *the same* scripts, because the UI is shared and must not be able to detect host-specific contract drift.

---

## 8. Browser support matrix (Trace: MP §10 "Test across Chrome, Firefox and Safari and record results")

### 8.1 Support matrix (BCD-verified 2026-10-19; re-verified per release)

| Capability | Chrome/Edge | Firefox | Safari (macOS) | Safari iOS | Notes |
|---|---|---|---|---|---|
| WASM MVP + SIMD128 | yes | yes | yes | yes | fixed-width SIMD universal |
| Wasm GC / EH-final | 119 / 137 | 120 / 131 | 18.2 / 18.4 | 18.2 / 18.4 | affects Rust codegen defaults |
| Wasm threads + `SharedArrayBuffer` | 74+/68+ (isolation-gated since 92) | 79+ | 15.2+ | 15.2+ | **COOP/COEP mandatory in practice** — WebKit enforcement history varies by release [VERIFY] |
| `Atomics.waitAsync` | 90+ | 145+ | 16.4+ | 16.4+ | needed by wasm-bindgen-rayon |
| WebGPU (`navigator.gpu`) | 113+ (Win/mac/ChromeOS; Linux 144, Intel Gen12+ per BCD notes) | 141+ **Windows only** | 26+ | 26+ | Firefox Android: none → WebGL2 is the renderer there |
| WebGL2 | 56+ | 51+ | 15+ | 15+ | universal floor |
| AudioWorklet | 66+ | 76+ | 14.1+ | 14.5+ | universal |
| WebCodecs `VideoDecoder`/`VideoFrame` | 94+ | 130+ (desktop; Android NO) | 16.4+ | 16.4+ | §10 WebCodecs proposal |
| `GPUExternalTexture` (`importExternalTexture`) | with WebGPU | 141+ | 26+ | 26+ | [VERIFY VideoFrame-accepting overloads per browser] |
| OffscreenCanvas | 69+ | 105+ | 16.4+ | 16.4+ | Phase 7 render-in-worker option |
| `requestVideoFrameCallback` | 83+ | 132+ | 15.4+ | 15.4+ | present pacing |
| File System Access (picker) | 86+ | NO | NO | NO | future opt-in source |
| MediaSession / WakeLock | 73+/84+ | 82+/126+ | 15+/16.4+ | 15+/18.4+ | OS integration, battery honesty |

Source: `@mdn/browser-compat-data` snapshot queried 2026-10-19 (https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API, …/SharedArrayBuffer, …/AudioWorklet, …/WebCodecs_API, …/GPUDevice/importExternalTexture). **iOS constraints:** no SAB without COOP/COEP on iOS; WebGPU only from iOS 26; lower memory ceilings (§2.4 wasm budget must respect them); fullscreen and autoplay gesture-gated.

### 8.2 Test matrix

- **Automated in CI (every merge):** Playwright **Chromium** on Linux (engine worker; both COOP/COEP modes; WebGL2 forced by flag-blocking WebGPU; autoplay-gesture simulated and withheld). [PROPOSAL — add Firefox to the Playwright job if headless support with required APIs is stable on the runner; verify.]
- **Automated, scheduled/weekly, non-merge-blocking [PROPOSAL]:** Firefox desktop (WebGPU being Windows-only means Linux CI runners exercise the WebGL2 path there — label honestly) and Playwright's WebKit build as a smoke tier; deltas vs real Safari recorded.
- **Manual protocol (per release, per MP §10 recording):** Chrome, Edge, Firefox on Windows/macOS/Linux; Safari on macOS; **Safari on a physical iOS device** (SAB/isolation, WebGPU iOS 26+, memory pressure, background-tab throttling). Checklist in `docs/web/` [PROPOSAL — path]: fresh profile → capability probe → fixture corpus → dropped-frame/underrun recording → truth-surface verification (renderer backend, threading, seekability) → results written as feature-matrix `evidence`. Real-browser manual evidence is labeled separately from headless CI evidence (MP §13).
- Optional later: commercial real-device browser grid [PROPOSAL — cost/benefit at Phase 7].

### 8.3 Baseline promise

The web build **claims** (feature matrix `web` column, `product.toml` feature exposure) only what §8.2 has evidence for. Minimum support floor for "works on the web": Chromium, Firefox, Safari desktop on the current two major versions, plus iOS Safari; anything beyond is bonus, not claim. [PROPOSAL — floor wording for product.toml.]

---

## 9. Web-specific security & privacy (Trace: MP §12)

### 9.1 CORS is the trust boundary

All remote media is fetched same-origin or CORS-authorized (`fetch` enforces it); we add no proxy and no `no-cors` path. Servers must send `Access-Control-Allow-Origin` and — because `Range` is not a CORS-safelisted header — answer the seek preflight with `Access-Control-Allow-Headers: Range`. Media bytes stay untrusted input (MP §12); the doc 04 fuzz-target registry additionally runs its parsers on the wasm host in the web CI job. [PROPOSAL — wasm fuzz tier; cheap, catches wasm-only codegen issues.]

### 9.2 No local filesystem

Only user-selected `File` objects, held in memory/OPFS within the origin; no path handling exists on this host — a class of MP §12 path attacks is structurally absent.

### 9.3 Origin isolation

The engine worker is same-origin, module-served, no `eval`. Host page CSP: `script-src 'self' 'wasm-unsafe-eval'` (wasm instantiation requires the wasm directive; JS `unsafe-eval` stays forbidden), `worker-src 'self'`, `connect-src` limited to self + user-opened origins [VERIFY final CSP against bundler requirements]. A CSP-hardening CI check asserts the shipped headers. [PROPOSAL]

### 9.4 WASM sandbox is inherent — but re-audit `plugin-host-wasm` on web

Extension plugins are wasm *inside* the wasm engine: the browser sandbox bounds them process-wise, manifests and capability gates still apply, but engine-internal resource limits must be re-proven on this host: memory caps via `Memory` limits remain enforceable; CPU budgets (fuel/metering per ADR-002 runtime) must be re-benchmarked in single-thread mode, because a greedy extension now steals slices from *the decode loop itself*. Mitigation: extensions are scheduled only from a bounded slice of each pump (§3.2); repeat offenders are isolated/disabled with existing `plugins.crashed` semantics (doc 05 §3). [PROPOSAL — policy; depends on ADR-002 runtime.]

### 9.5 COOP/COEP and embeddability

Cross-origin isolation restricts what the embedding page may include (non-CORP third-party resources break). Lumen ships with the headers **off by default** (single-threaded, §3.3) and documents the trade-off; embedders who enable isolation get threading. This is a site-owner decision stated in deploy docs, not ours to force.

### 9.6 No telemetry by default

Zero non-media network calls (MP §12): no analytics, update pings or crash reporting unless enabled in `product.toml` *and* surfaced in the privacy UI. Settings persist to `localStorage` (origin-scoped) only, with a "forget everything" control. Privacy claims are a release-checklist item. [PROPOSAL — checklist item.]

---

## 10. Known-limit table (Trace: MP §10, §14; vocabulary per doc 02 §4.3)

| Capability | web status | Reason (matrix vocabulary) | User-facing behavior |
|---|---|---|---|
| Raw UDP, RTSP/RTP access | `unsupported` | No raw sockets in browsers (MP §10); `blocked_reason` cites §6.3 | Open-URL rejects `rtsp://`/`udp://` with localized "not available on web"; HTTP(S)/HLS/DASH offered |
| Optical discs (DVD/BD/CD) | `unsupported` | No disc APIs in browsers; also ADR-gated on desktop (MP §14) | Open dialog filters discs out; no disc menu items |
| DVB / direct capture hardware | `blocked` | No hardware access; investigation done (this doc); next step: browser-API substitutes (MP §17 blocked-item rule) | Devices menu shows only the WebAudio output; capture entry hidden |
| Capture via `getUserMedia`/`getDisplayMedia` | `not_started` (Phase 7+) | Browser-API substitute exists (§6.3); `MediaStreamTrackProcessor` Chromium-first [VERIFY] | Not exposed until implemented and tested |
| Hardware decode | `partial` **[PROPOSAL — pending WebCodecs ADR]** | **WebCodecs** is the web analog of ADR-001 OS decoders: a *browser API* (`VideoDecoder`/`AudioDecoder`), not a bundled native codec — policy-clean in principle pending the Phase 0 audit (doc 04 flags the same ADR). Support: Chrome/Edge 94+, Firefox 130+ desktop, Safari 16.4+ (verified §8.1) | When active: reported truthfully via `system.get-engine-info → decoders[].hardware = true, backend = "WebCodecs"`; when absent (e.g. Firefox Android): software decode, or `blocked` per codec where no pure-Rust decoder exists (H.264/HEVC — doc 04 gap register) |
| Threaded decode perf | `partial` | Conditional on COOP/COEP (§3); `notes` must name the condition | Capability snapshot says `threading: single|pooled(n)`; settings explains the header requirement |
| Local filesystem browsing | `unsupported` | By design (§9.2); File System Access API may lift on Chromium later | File open works via picker/drag-and-drop; "open folder" absent |
| Renderer | `partial` | WebGPU where available (§8.1); WebGL2 fallback everywhere | Backend displayed truthfully; Firefox Android permanently on WebGL2 |
| `<video>`-tag fallback path | `unsupported` (by design) | Rejected as a hidden native engine (§1) [PROPOSAL] | — |

Every row is (or becomes, on seeding) a `docs/feature-matrix.toml` row with `status.web` set and required `blocked_reason`/`notes` — this table is a *view* of the matrix, never a second source of truth.

---

## 11. Test plan (Trace: MP §13; doc 05 §7)

1. **Unit / wasm-logic (Node, `wasm-bindgen-test`):** cooperative-scheduler slice budgets (pump respects deadline under synthetic load); capability-query logic against mocked `navigator`/`crossOriginIsolated`; fetch-source Range/seekability detection against a scripted mock; blob-source slicing; ring-buffer math in both transports; worklet shim counter arithmetic (JS-side tests).
2. **Headless-browser integration (Playwright Chromium, merge-blocking):** real Worker + real engine; fixture corpus from `tests/fixtures` served as blob and over HTTP (test server with and without Range support); open→play→pause→seek→stop; stale-generation races (doc 05 §7.1 scenarios re-run on this host); truth-rule assertions (no early success — doc 05 §7.3 verbatim); forced-WebGL2 path; COOP/COEP on/off snapshots self-consistent; autoplay with/without gesture (§5.3); worker-crash event; CSP hardening check (§9.3).
3. **Conformance suite (doc 05 §7) on the web host:** `WasmNodeHost` merge-blocking; `WasmBrowserHost` as in §7 — scripts are host-parameterized, never forked.
4. **Cross-browser manual protocol:** §8.2's per-release checklist with evidence into the feature matrix; iOS Safari on physical hardware is mandatory for any `web = "verified"` claim touching SAB, WebGPU or autoplay.
5. **CI wiring:** all of the above in the `web` job (§2.5), with artifacts — capability snapshots, size report, dropped-frame/underrun counters — persisted and diffed, so web performance regressions are caught by the same evidence-first rule as parity (MP §11).

---

## 12. Open questions for the Lead (decisions requested, per AGENTS §3.6)

1. **wasm-bindgen adoption** — approve as the binding tool (incl. `wasm-bindgen-rayon` for §3.3) through the Phase 0 dependency audit, or require a raw-import evaluation first? (§2.2)
2. **Capability reporting home** — extend `EngineInfo` with `host_capabilities` vs a new `system.get-host-capabilities` command (additive per doc 05 §6). (§3.3)
3. **Autoplay substate** — `playback.gesture-required` event + paused-pending-gesture presentation vs a state-machine extension; doc 05 catalog decision. (§5.3)
4. **Worker crash semantics** — `system.host-error` event name/shape and reload-vs-respawn policy. (§7)
5. **Size budget numbers** — 3 MB / 6 MB baselines (§2.4) are proposals; pin after the first real build.
6. **WebCodecs ADR** — open the "WebCodecs as the web's OS-decoder" ADR (doc 04 flags it; §10) — until it passes, H.264/HEVC stay `blocked` on web.
7. **OffscreenCanvas render worker** — v1 main-thread present vs worker present; decide by Phase 7 measurement. (§4.1)
8. **Embedding policy** — confirm shipping with COOP/COEP off by default (§9.5).
9. **Firefox-Android-class browsers** — accept WebGL2-only as a `verified` renderer tier, or `partial`? Matrix semantics decision.

## Unfinished

None. `docs/08-ui-figma-fidelity.md` §8 (UI sharing) was absent at writing time; §1/§7 statements await reconciliation with it (noted in the document header).
