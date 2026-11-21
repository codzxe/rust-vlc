# AUDIT: crates/renderer-wgpu/src/renderer.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 438 / 19 units (14 methods, 1 trait impl, 1 free fn, Debug impl, struct) + 9 in-file tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:81` `use renderer_wgpu::renderer::Renderer` → `:1314` `Renderer::new_headless(frame extent, Bgra8Srgb, Fifo)` → `Presenter::present` per presented frame; the AVI first-vertical lane `avi-firstvertical-open-session.txt` + `v_frames=60 v_decoded=60` bench presents through this path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of full file; `sed -n` quote verification (38-42, 153-161, 193-198, 218-228); `rg "wgpu" crates/renderer-wgpu/` (mentions are comments/docs only — Cargo.toml deps: media-types + video-engine, no wgpu crate); `rg` CLI usage (pipeline_host.rs:79-83, 885, 1300-1330; apps/cli/Cargo.toml:60); `rg` `uploads_format` callers (apps/desktop/src/wire.rs:999 — live consumer) and `swap_compositor`/`fallback_to_software` (negotiate.rs:311-313, tests/backend_fallback.rs only); read `media-types/src/frame.rs:91-103` (PixelFormat = exactly 5 variants); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:4766-4800+` (renderer_wgpu lib: 60 tests ok, incl. all 9 renderer::tests), `:tests/presenter_seam.rs` 5 ok incl. `ac_scheduler_queue_renderer_end_to_end`. |

## 1. One-paragraph verdict
This is the shipped video presenter — small, typed, and genuinely executed: the `lumen` binary constructs this `Renderer` headless per open (pipeline_host.rs:1314) and runs `Presenter::present` for every presented frame of the AVI first-vertical lane, the in-file tests all ran green in the no-fail-fast baseline, and a five-test integration suite (`presenter_seam.rs`, also green) wires this renderer behind the real scheduler and queue end-to-end. Every refusal path is typed (no-surface, format-unsupported, opaque-handle, size-mismatch, zero-extent, present-mode), and the test suite is honest — it asserts bytes (byte-exact 1:1 passthrough), pixels (letterbox rows), counters (staging copies == 1), and error codes verbatim; the `uploads_format` classification is pinned exhaustively against all five `PixelFormat` variants. The two findings are debt, not fraud: the crate's name promises wgpu while the file (and crate) contains none — a thoroughly documented Lead-gated deferral, but the name still lies on every `Cargo.toml` line that depends on `renderer-wgpu` — and the `swap_compositor` backend-handover seam has no shipped caller (its only consumer is the negotiate chain, which only tests exercise today).

## 2. What actually works (verified)
— Executed in the shipped lane: `apps/cli/src/pipeline_host.rs:1314` builds `Renderer::new_headless(...)`; baseline AVI smoke (`avi-firstvertical-open-session.txt`: `Buffering → Playing → Stopped → Idle`, 60/60 frames in the bench) presents through `Presenter::present` — this file's core path.
— All 9 in-file tests executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:4766+` — `renderer::tests::{presenter_round_trip_byte_exact_and_reports_the_slot, format_routing_matches_the_upload_path, opaque_handles_are_refused_with_the_named_path, truncated_frames_are_decode_corruption_not_guesses, detached_renderer_degrades_not_crashes, resize_flows_to_the_surface, params_swap_takes_effect_next_present, headless_constructor_validates_like_the_surface}` all `ok` (60-test lib suite, 0 failed).
— End-to-end with the real scheduler: `tests/presenter_seam.rs::ac_scheduler_queue_renderer_end_to_end` + 4 more, all ok (baseline, presenter_seam binary).
— Refusals are typed and exhaustive: `PixelFormat` has exactly 5 variants (frame.rs:91-103) and `format_routing_matches_the_upload_path` covers all 5 against both `uploads_format` and an actual `present` refusal — the probe and the behavior cannot drift unnoticed.

## 3. Findings

### F-001 [S4]
Where: crates/renderer-wgpu/src/renderer.rs:38-42 (with crates/renderer-wgpu/Cargo.toml:1-14, deps at :31-32)
Code:
```
/// Construct headless for tests and the software fallback
/// ([`Renderer::new_headless`]), or inject a surface + compositor pair
/// for the hardware path ([`Renderer::with_backend`]) — the wgpu
/// surface and compositor join behind the exact same constructor when
/// the Lead-gated import lands (crate docs).
```
What is wrong: the crate is named `renderer-wgpu` and this file is `renderer.rs` inside it, yet neither the file nor the crate contains one line of wgpu code — the manifest's only dependencies are `media-types` and `video-engine`; every "wgpu" occurrence in the crate is a comment naming a deferred import. The hardware path this doc promises ("the wgpu surface and compositor join … when the Lead-gated import lands") does not exist.
Why it matters: naming debt with a paper trail is still naming debt: every dependent manifest (`apps/cli/Cargo.toml:60 renderer-wgpu = …`) reads as a GPU dependency while linking a CPU blit; capability claims and SBOM readers inherit the wrong name. The deferral is honestly recorded (Cargo.toml header cites worklog 2.D1, records/wgpu-30.0.1.md, deny.toml's libloading ban; matrix row `platform-vout-wgpu` = not_started on all four platforms — consistent), which is why this is S4 and not FALSE.
What correct looks like: rename the crate to `renderer` (or `video-present`) now and let `renderer-wgpu` be created by the card that actually adds the import — or record the rename debt in the crate docs' first line instead of the fourth paragraph.
Confidence: HIGH — Verification: read + rg ("wgpu" hits are comments only; deps list read).

### F-002 [S4]
Where: crates/renderer-wgpu/src/renderer.rs:153-161
Code:
```
    /// Swap the linked compositor — the backend handover seam (card
    /// 7.C1): the surface, parameters and staging state survive the
    /// swap untouched; the next present runs on the new backend.
    /// [`crate::negotiate::Renderer::fallback_to_software`] is the
    /// fallback chain's policy wrapper over this (it also answers
    /// the typed capability report).
    pub fn swap_compositor(&mut self, compositor: Box<dyn Compositor>) {
        self.compositor = compositor;
    }
```
What is wrong: `swap_compositor`'s only caller is `negotiate::Renderer::fallback_to_software` (negotiate.rs:311-313), and the negotiate chain's only consumers are the crate's own tests (`tests/backend_fallback.rs`) — no app or engine code reaches the backend-handover seam in any shipped binary. The engine-side hook that would relay `RendererBackendReport` exists (`playback-engine/src/hotplug.rs:298` references "auto-negotiated wgpu backend") but is not wired to this seam.
Why it matters: the 7.C1 fallback story (hardware loss → typed report → software present) is implemented and test-proven but dormant; dead-in-production seams rot silently — the next agent verifying "backend fallback works in the product" would find no path that exercises it.
What correct looks like: either the hotplug card wires `negotiate` into the CLI/desktop pipeline (making the seam live) or the docs state plainly that the fallback chain is test-only until that card lands.
Confidence: HIGH — Verification: read + rg caller trace (negotiate.rs:311-313; tests/backend_fallback.rs:180,221,250; no app hits).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-23 | REAL | — | Threading model (mailbox depth 1, `[PROPOSAL]`) and timestamp honesty (`[VERIFY]` on compositor clock) — honest annotations of unshipped aspects. |
| `struct Renderer` | 43-48 | REAL | — | surface: Option, compositor: Box, staging, params — all injectable. |
| `Debug for Renderer` | 50-61 | REAL | — | Redacts the surface to (extent, format); `finish_non_exhaustive`. |
| `with_backend` | 67-78 | REAL | — | Pure constructor; the hardware injection point (no hardware impl exists — F-001). |
| `new_headless` | 87-94 | REAL | — | Propagates HeadlessSurface typed errors; executed by the CLI (pipeline_host.rs:1314). |
| `detach_surface` | 99-101 | REAL | — | Returns the surface — caller owns reattachment. Tested (detached_renderer test). |
| `attach_surface` | 104-106 | REAL | — | Plain setter; tested with reattach-resumes assertion. |
| `has_surface` | 110-112 | REAL | — | Trivial, correct. |
| `params` / `set_params` | 116-124 | REAL | — | Cheap swap; next-present pickup tested pixel-exactly (params_swap test). |
| `resize` | 132-135 | REAL | — | NoSurface/ZeroExtent typed; flows to surface — tested. |
| `extent` | 141-144 | REAL | — | Typed NoSurface. |
| `backend_name` | 149-151 | REAL | — | Truthful "software" today; asserted by round-trip test (`"software"`). |
| `swap_compositor` | 159-161 | REAL | S4 | **No shipped caller (F-002)**. |
| `staging_stats` | 165-167 | REAL | — | Observable evidence; asserted (bytes 4*4*4, copies 1). |
| `readback_rgba` | 175-178 | REAL | — | The screenshot/AC path; used by tests to assert bytes and by `crate::screenshot`. |
| `render` | 188-215 | REAL | — | validate → stage → place → composite → present; OpaqueHandle refused typed (195-197); every error stage-tagged. Executed in the AVI lane. |
| `Presenter::present` | 219-228 | REAL | — | `presented_at = request.when` — the scheduled-slot echo, documented as synchronous-Fifo truth; tested (round-trip asserts the echo). |
| `uploads_format` | 239-241 | REAL | — | Pure classification; exhaustively pinned (all 5 PixelFormats) and live-consumed (apps/desktop/src/wire.rs:999). |

## 5. Test honesty (9 in-file tests, all executed ok in baseline :4766+)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `presenter_round_trip_byte_exact_and_reports_the_slot` | YES | Asserts readback bytes == input bytes (byte-exact 1:1), `presented_at == request.when`, staging stats exact — a dropped present, a swizzle bug, or a slot lie fails it. |
| `format_routing_matches_the_upload_path` | YES | All 5 PixelFormat variants pinned against both `uploads_format` and an actual refusal with the exact context code `render.upload.format-unsupported`. |
| `opaque_handles_are_refused_with_the_named_path` | YES | `OpaqueHandle(7)` → `render.upload.opaque-handle` verbatim. |
| `truncated_frames_are_decode_corruption_not_guesses` | YES | 8 bytes for a 4×4 RGBA → `render.upload.size-mismatch` + Display contains "truncated" — pins no-silent-guess. |
| `detached_renderer_degrades_not_crashes` | YES | Detach → typed `render.surface.missing` → reattach → present succeeds (params/stats survive). |
| `resize_flows_to_the_surface` | YES | Extent follows resize; letterboxed present ok; zero resize → `ZeroExtent`. |
| `params_swap_takes_effect_next_present` | YES | Pixel-level letterbox asserts (rows 0/3 background white, row 1 frame black) — placement math verified through actual output bytes. |
| `headless_constructor_validates_like_the_surface` | YES | Zero extent → `ZeroExtent`; Mailbox → `PresentModeUnsupported` with supported=[Fifo] verbatim. |
| Mutation checks (reasoned): present() dropped from `render` | CAUGHT | Round-trip readback would return the previous/stale buffer, not the input bytes (headless surface commits on present). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "wgpu + winit surfaces … Unified video output" | matrix row `platform-vout-wgpu` (status not_started ×4) | REAL (honest not_started) | No wgpu code exists (deps: media-types+video-engine); the matrix row says not_started — doc and code agree, the CRATE NAME is the outlier (F-001). |
| "`PresentInfo::presented_at` is the request's scheduled slot … vsync-quantized reality … is the hardware backend's to fill `[VERIFY]`" | module docs :16-23 | REAL | `present` echoes `request.when` (225-227); headless Fifo is synchronous so the claim holds for the only shipped path; the hardware caveat is flagged, not hidden. |
| "v0 is the synchronous core … the playback engine (Phase 3) hosts the thread and the mailbox" | module docs :11-14 | REAL | No thread/mailbox in this file (the CLI drives it synchronously per tick — presenter_seam tests confirm the synchronous shape). The Phase-3 wiring remains unshipped — consistent with the doc's framing. |
| "The YUV family converts on the GPU (doc 03 §4.11 e) — a later card; until then the renderer refuses those frames with a typed error" | `uploads_format` docs :231-237 | REAL | Nv12/I420/Yuyv422 → `render.upload.format-unsupported` (tested); no GPU conversion exists. |

## 7. Dead / duplicated / unreachable code
- `swap_compositor` — dormant seam, F-002.
- `with_backend` — has no hardware caller (the headless constructor is the only production entry), but it is the documented injection point and is exercised by negotiate/backend tests; not dead, unwired.
- No duplication.

## 8. Dependencies & flags
- Std-only crate (C5): deps `media-types` + `video-engine`; dev-deps `testkit` + `media-clock` (test-only, boundaries.toml-recorded). No feature gates in this file.
- The wgpu import is Lead-gated (records/wgpu-30.0.1.md; deny.toml bans libloading) — the deferral is documented in the manifest header.

## 9. Open questions for the next agent
- `crates/renderer-wgpu/src/negotiate.rs` (the fallback chain + BackendReport) is unaudited by me — its `probes_answer_this_builds_truth` test ran ok in baseline; whoever audits it should start from F-002's dormancy question.
- `staging.rs` (the measured uploader that produces `render.upload.*` errors) and `surface.rs` (HeadlessSurface) are the error-code sources asserted by this file's tests — unaudited here.
- Does any desktop/web lane construct a NON-headless surface? apps/desktop/src/wire.rs probes `uploads_format` but the surface construction there is outside my read — a next agent should confirm whether the desktop app ever presents to a real target or is also headless-only.
