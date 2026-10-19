# ADR-003: Video-in-Webview Presentation — per-OS strategy with prototype gates

- **Status:** accepted (2026-10-19, Phase 0 gate; per-OS final confirmation gated on Phase 2 prototypes)
- **Deciders:** Lead (per MASTER-PROMPT §16 Phase 0 mandate + §7 "prototype early")
- **Evidence:** `docs/inventory/platform-synthesis.md` (0.C5-s) consolidating
  `platform-{windows,macos,linux}.md` (0.C5-a/b); doc 08 §7 candidate
  definitions; doc 13 §5 cards 2.D3 / 2.E2-V
- **Supersedes:** open question in doc 01 §7 / doc 02 §6

## Context

Candidate A = native wgpu surface behind a transparent webview; candidate B =
frame streaming to a canvas (WebCodecs where available, ImageBitmap/MSE
fallbacks). Evidence summary:

- **Windows (WebView2)**: A feasible-with-conditions (alpha-only background
  0/255; parent must drop `WS_CLIPCHILDREN` — tao lacks the switch, winit has
  it; wry #1540). B ready today (WebCodecs since Chromium 94; not in
  WebView2's disabled-features list).
- **macOS (WKWebView)**: A structurally cleaner (sibling NSView behind
  transparent webview; `drawsBackground` handling fixed in Tauri ≥ 2.12.1).
  B gated by floors (video-only WebCodecs 16.4+/macOS 13.3; full at 26).
- **Linux (WebKitGTK 4.1 via wry 0.57)**: A has **no Wayland path** (child
  webviews are X11-only) + open NVIDIA transparent-window crashes (tauri
  #14924) + wl_shm buffer accumulation (wry #1731). B safer; WebCodecs
  proven compiled-in to Debian's 2.52.6 binary (strings probe), default-state
  [VERIFY SV-1].

## Decision

1. **Adopt the per-OS landing**: A on macOS; A-or-B on Windows (prototype
   decides); **B on Linux**. No single-strategy monoculture.
2. **The engine is strategy-agnostic**: renderer-wgpu abstracts the surface
   (doc 03 §4.12); frame-streaming B is a transport of the same presented
   frames — the presentation scheduler owns timing either way. No decode or
   sync logic lives in the strategy.
3. **Phase 2 prototype gates** (doc 13 cards already exist): 2.D3 runs the
   A/B prototypes per OS with the evaluation matrix (latency, CPU, compositing
   artifacts, fullscreen, multi-monitor); 2.E2-V records verdicts. A platform
   whose prototype fails flips to its documented fallback — that is the
   design, not an exception.
4. **Truthful capability reporting**: the active presentation path is a
   queryable engine fact (like ADR-001's active-decoder report); the UI and
   matrix show what is actually in use.
5. **Linux contingency** (if B underperforms on WebKitGTK): escalate via the
   risk register (R-04) — options recorded, not pre-decided.

## Consequences

- R-04 becomes bounded-and-planned (worst case: Linux ships B with documented
  performance characteristics; the product never loses playback).
- The winit-vs-tao windowing detail (WS_CLIPCHILDREN) becomes a Phase 2
  engineering item, pre-assigned to the 2.D3 card.
- SV-1 (Linux WebCodecs default-enabled) is the cheapest open [VERIFY] and is
  assigned to the first Phase 2 wave.
