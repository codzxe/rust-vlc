# AUDIT: apps/web/src/render/backend.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 596 / 24 (negotiate + 2 enums w/ as_str x3 + 2 structs w/ methods + RenderLane x6 + Default + backend_index + 7 tests) |
| File verdict | REAL (pure state machine, fully tested, wired into WebHost) — but its subject is a renderer that does not exist (see F-001) |
| Reachable from shipped entry point? | NO as product surface (apps/web outside lumen-cli/desktop lane; product UI never loads wasm). YES inside the web lane: host.rs:847 `render_lane: RenderLane::new()`, host.rs:1019 `self.render_lane.boot(caps)`, host.rs:1052 `surface_lost`, host.rs:1146 `active().is_none()` gates every present — call chain verified. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | Read of backend.rs (full), host.rs excerpts (1110-1170, 1360-1380), canvas.rs:150-245, harness.js probe+present sections; rg for API consumers; baseline test citation. |

## 1. One-paragraph verdict
This is the one file in my set that is genuinely what it says: a pure, exhaustive, mutation-resistant backend-negotiation and loss state machine (WebGPU→WebGL2 ladder, consumed-backend bookkeeping, audio-only degrade) with 7 tests that actually pin every rule they name, wired into the host (boot, surface_lost, present gating, capability string). The verdict is dragged down not by the logic but by what surrounds it: the negotiated backend never renders anything — `CanvasSurface::present()` (canvas.rs:237-242) is a no-op `Ok(())`, the e2e harness only counts `frame.rgba.byteLength` (harness.js:417-423), and no code in the repo draws a pixel via WebGPU/WebGL2 in the web lane. The module also carries dead surface: `settings_label` (zero callers), an unreachable `NoWebGl2Context` variant, and a test-only `RendererCaps::NONE`.

## 2. What actually works (verified)
— 7/7 tests pass in baseline (cargo-test-workspace.txt lumen-web block; `render::backend::tests::*` listed there). Not re-run (cargo budget; cited per spec §2).
— Ladder totality: `the_ladder_over_every_capability_report` iterates all 8 boolean combos and asserts active+reason for each — a mutation in `negotiate`'s condition order fails it.
— Loss semantics: `loss_falls_one_step_and_consumes` covers WebGPU→WebGL2 fall, stale-report ignore, floor-loss audio-only, postmortem string; `webgpu_loss_without_the_floor_degrades_to_audio_only` pins the probe-facts consultation.
— Present gating is real: host.rs:1146-1149 returns early when `render_lane.active().is_none()` — an unprobed or fully-degraded lane ships no PresentFrame (verified by Read).

## 3. Findings
F-001 [S3]
Where: apps/web/src/render/backend.rs:5-9 vs apps/web/src/render/canvas.rs:237-242 and ui/e2e/web/site/harness.js:417-423
Code:
```
//! The **backend negotiation state machine** for the browser render
//! lane: WebGPU preferred, WebGL2 the universal floor, `unavailable`
//! the honest answer — and the ACTIVE backend is the only thing the
//! capability snapshot ever reports
```
What is wrong: the negotiated backend is a LABEL with no rendering behind it in this repository. `CanvasSurface::present()` is:
```
    pub fn present(&mut self) -> Result<(), CanvasError> {
        if self.lost {
            return Err(CanvasError::SurfaceLost);
        }
        Ok(())
    }
```
(canvas.rs:237-242 — "returns with the frame logically on the surface": no canvas, no GL call, no GPU) and the only browser consumer of the frames asserts byte length, never pixels: `assertEq(frame.rgba.byteLength, frame.width * frame.height * 4, ...)` (harness.js:420-423). The capability snapshot answers `renderer: "webgl2"` for a lane in which WebGL2 is never used to draw anything. Why it matters: the e2e `renderer-ladder-consistency` scenario (harness.js:328-344) treats that string as the render truth; a reader concludes the web lane renders via WebGL2 when it ships counted RGBA byte arrays and nothing else. What correct looks like: either the drawing path (page-side WebGL2 blit of the RGBA lane) or a snapshot string that says "rgba-delivery (no gpu surface)". Confidence: HIGH — Verification: read (canvas.rs, harness.js, present.rs grep: no draw/texImage2D anywhere in repo web lane).

F-002 [S3]
Where: apps/web/src/render/backend.rs:91-100
Code:
```
    /// The settings-page label (doc 09 §4.4: the page states e.g.
    /// "Rendering: WebGPU" / "WebGL2 (fallback)" — the fallback marker
    /// is part of the honest label, not decoration).
```
What is wrong: `settings_label` has ZERO callers outside its own test (rg over apps/web + ui: only backend.rs:491-492). The web settings page that would state it does not exist; the product UI (ui/src) never loads the wasm host at all. Dead public API with a doc that describes a live consumer. Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: apps/web/src/render/backend.rs:115-117, 191-201
Code:
```
            // Unreachable by the ladder's construction (an adapter
            // implies active); kept total so the reason type stays
            // exhaustive without a `None`-with-`Some(reason)` state.
            UnavailableReason::NoWebGl2Context
```
What is wrong: the variant's own doc sells it as "the typed `renderer-unavailable` fatal (doc 09 §4.4 step 4 — the UI shows the unsupported state)" — but `negotiate` can never produce it (api&&adapter ⇒ active; !api ⇒ NoWebGpuApi; api&&!adapter ⇒ NoWebGpuAdapter), and `snapshot_string`'s `unwrap_or` arm (162) is equally unreachable for negotiate-produced values. The UI-shows-unsupported state is actually carried by NoWebGpuApi/NoWebGpuAdapter. A doc-described state that cannot occur. Confidence: HIGH — Verification: read + 8-combo test confirms coverage never asserts it.

F-004 [S4]
Where: apps/web/src/render/backend.rs:304-320 vs 327-340
Code:
```
        let reason = if self.active.is_some() {
            None
        } else {
            // The probe-facts outcome's own reason (the lane's
            // runtime-loss vocabulary is separate — see
            // [`RenderLane::lane_string`]).
            negotiate(caps).reason
        };
```
What is wrong: after a total runtime loss, `snapshot()` reports the PROBE reason (e.g. "requestAdapter refused") while `lane_string()` reports the LOSS postmortem ("the WebGL2 context was lost — was presenting via webgl2") — two APIs of the same struct answer different stories for one state. Host code consumes `lane_string` (capability snapshot); `render_snapshot` is diagnostics-only (host.rs:964). Divergent duplicate truth, commented but untested (no test covers snapshot() post-loss). Confidence: MEDIUM — Verification: read; no test pins the post-loss snapshot shape.

F-005 [S4]
Where: apps/web/src/render/backend.rs:60-64
Code:
```
    pub const NONE: RendererCaps = RendererCaps {
        webgpu_api: false,
        webgpu_adapter: false,
        webgl2_context: false,
    };
```
What is wrong: public const consumed only by a host.rs test (host.rs:2366 `renderer: Some(crate::render::RendererCaps::NONE)`). A public API surface item whose only user is a test. Confidence: HIGH — Verification: rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RendererCaps (struct) | 44-55 | REAL | — | camelCase serde matches harness.js:150 (`webgpuApi` etc.) — boundary consistent |
| RendererCaps::NONE | 60-64 | REAL (test-only use) | S4 | F-005 |
| RenderBackend::as_str | 83-89 | REAL | — | kebab-case wire ids "webgpu"/"webgl2" |
| RenderBackend::settings_label | 94-100 | DEAD | S3 | F-002: zero callers |
| UnavailableReason::as_str | 122-133 | REAL | — | human sentences |
| RendererNegotiation::snapshot_string | 155-166 | REAL | S4 | unwrap_or arm unreachable (F-003) |
| negotiate | 182-210 | REAL | — | pure; all 8 combos pinned by test |
| LossReason::as_str | 227-233 | REAL | — | |
| LaneTransition::is_audio_only | 253-256 | REAL | — | consumed by host event lane |
| RenderLane::new / Default | 278-286, 390-394 | REAL | — | |
| RenderLane::boot | 292-299 | REAL | — | resets consumed + degraded; re-negotiates (tested) |
| RenderLane::snapshot | 304-320 | REAL | S4 | F-004 divergence post-loss |
| RenderLane::lane_string | 326-340 | REAL | — | postmortem string tested |
| RenderLane::active | 343-346 | REAL | — | gates host.rs:1146 present |
| RenderLane::surface_lost | 357-387 | REAL | — | stale-report ignore, one-step fall, consume, never-falls-up — all tested |
| backend_index | 397-402 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_ladder_over_every_capability_report | YES | exhaustive 8-combo truth table |
| adapter_without_api_is_ignored | YES | lying-probe rule pinned |
| snapshot_strings_are_the_honest_answer | YES | exact string asserts |
| loss_falls_one_step_and_consumes | YES | full transition + stale + floor + postmortem |
| webgpu_loss_without_the_floor_degrades_to_audio_only | YES | pins the caps consult |
| unprobed_lane_has_no_snapshot | YES | |
| reboot_renegotiates_from_fresh_facts | YES | consumed reset pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the active backend is reported … never a guess, never an aspiration" | backend.rs:8-9 | REAL | negotiate is pure probe-facts logic; host.rs:953 quotes lane_string |
| "the lane falls ONE step down … a backend that was lost is CONSUMED" | backend.rs:29-33 | REAL | surface_lost:365-381 + tests |
| "Losing the floor is the audio-only degrade: an explicit event" | backend.rs:33-35 | REAL | host.rs:1052-1056 surfaces RenderDegraded |
| Settings page states "Rendering: WebGPU"/"WebGL2 (fallback)" | backend.rs:91-93 | FALSE (no such page) | F-002: zero consumers |
| renderer negotiation = the browser render lane | module docs | SHALLOW | F-001: no drawing path exists behind the label |

## 7. Dead / duplicated / unreachable code
settings_label (dead), NoWebGl2Context (unreachable variant), RendererCaps::NONE (test-only). No duplication.

## 8. Dependencies & flags
serde only (derive); no feature gates. `#[serde(rename_all = "camelCase")]` on RendererCaps matches the harness's wire form — the one place the JS<->Rust casing contract is load-bearing and it is consistent (verified harness.js:150-172).

## 9. Open questions for the next agent
- present.rs (875 lines, not mine): confirm PresentMailbox's displaced-frame accounting and that `take_present` latest-wins cannot drop the FINAL frame of a session (busy() at host.rs:1366 keys off present_pending — a stuck mailbox would keep the pump alive forever).
- canvas.rs auditor: F-001's no-op present() — is any doc row citing canvas rendering as evidence?
