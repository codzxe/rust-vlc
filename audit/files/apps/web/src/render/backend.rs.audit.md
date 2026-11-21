# AUDIT: apps/web/src/render/backend.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Lines / functions | 596 / 18 (12 fns + 5 impl-block as_str/label fns + `negotiate` + `backend_index`) + 7 tests |
| File verdict | REAL for the negotiation half (executed in a real Chromium once, committed evidence); UNREACHABLE-in-browser for the runtime-loss half (no JS producer exists) |
| Reachable from shipped entry point? | PARTIAL. YES for `negotiate`/`RenderLane::boot`/`lane_string`/`active`: wasm lane `boot` message carries `renderer` caps → host.rs:1019 `render_lane.boot(caps)` → `negotiate` (rg "render_lane" apps/web/src); the page-side producer is `ui/e2e/web/site/harness.js:148-174` (`probeRenderer()` + `lane.post({type:"boot", renderer})`), evidenced by committed Playwright run `docs/web/browser-matrix-20261110.md:65-66` (`renderer-ladder-consistency ✅ probed {webgpuApi:true, webgpuAdapter:false, webgl2Context:true} → activeBackend "webgl2"`). NO shipped product page exists (only the e2e harness); NO for `surface_lost`/`LossReason`/`LaneTransition`/consumed bookkeeping: the only trigger is the wire message `surfaceLost` (protocol.rs:89,121) and **zero JS in the repo sends it** — `rg "surfaceLost" -t js,ts,tsx,html` repo → only worker.js:26-27 doc comment. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg "RenderLane|surface_lost|negotiate|RendererCaps|lane_string|snapshot_string" apps/web/src` (wiring: host.rs:806,847,953,964,1019,1052; protocol.rs:50,65,89-93,181) · `rg "surfaceLost"` repo-wide incl. `-t web` js/ts/html (ONLY worker.js:26-27 comment — no sender) · `rg -i "lost|webglcontext|device\.lost" ui/e2e/web/site/harness.js apps/web/worker.js` (no listener either) · `rg "renderDegraded"` (no JS consumer of the output side) · `rg "settings_label"` repo (zero callers outside backend.rs) · `rg "navigator\.gpu|getContext\(.webgl2|requestAdapter" apps/web/*.js` (zero — the probe lives ONLY in the e2e harness page) · `rg "new Worker|lumen_web"` (harness + web-matrix-run.mjs are the only instantiators) · baseline: `orchrun-test-workspace-nofailfast.log.txt:3087-3180` (lumen_web 111/111 pass incl. all 7 `render::backend::tests`); NOTE `test-workspace.log.txt` (INDEX #3) ABORTED at apps/desktop (`error: test failed` fail-fast) and never reached lumen_web — the 1681-passed headline undercounts; the no-fail-fast orchrun rerun is the complete evidence · `sed -n`/`od -c` quote verification (raw bytes of `#[must_use]` confirmed) |

## 1. One-paragraph verdict
The pure state machine is the best-audited kind of code in this wave: `negotiate` walks a 3-boolean ladder with total, exhaustively-tested branches; `RenderLane` adds consumed-backend and degraded-reason bookkeeping with stale-report rejection; all 7 tests are mutation-hard (the 8-combination ladder test alone pins every branch) and all pass in the no-fail-fast baseline rerun. Real-browser evidence exists for the negotiation half (committed Playwright Chromium run, `renderer-ladder-consistency` pass). The rot is around the machine, not in it: (1) the entire runtime-loss half is triggered by a `surfaceLost` wire message that no JavaScript in the repository ever sends — the module doc's "the JS glue's `webglcontextlost` / `device.lost` report" describes glue that does not exist, so in a real session a driver-reset context loss will never degrade the lane, producing exactly the "frozen last frame pretending to play" the module promises never happens; (2) `RenderLane::snapshot` can emit a `RendererNegotiation` that violates its own documented invariant (`reason` set exactly when `active` is `None`) after a runtime loss. And nothing ships this crate: the only page that drives it is an e2e harness, and the wasm module is decoder-less by design (parity web=0/252, INDEX #9).

## 2. What actually works (verified)
— `negotiate` ladder: all 8 boolean combinations traced by hand against 184-210; adapter-without-API ignored (lying-probe rule); exhaustive-test `the_ladder_over_every_capability_report` (414-448) covers all 8 — green in `orchrun-test-workspace-nofailfast.log.txt:3178`.
— `RenderLane::surface_lost` bookkeeping: traced WebGpu-loss→WebGl2→loss→None against 357-387; stale/duplicate loss ignored (362-364); never falls up (372); all pinned by `loss_falls_one_step_and_consumes` (499-539) + `webgpu_loss_without_the_floor_degrades_to_audio_only` (545-557) — green (3175, 3180).
— Real-browser execution of `negotiate`: `docs/web/browser-matrix-20261110.md:66` — `renderer-ladder-consistency ✅ {"probed":{"webgpuApi":true,"webgpuAdapter":false,"webgl2Context":true},"activeBackend":"webgl2"}` (Playwright Chromium 157, captured 2026-11-11 at commit 3755dc3, branch w24/a — predates baseline e83e5bd, so REAL at that SHA, REAL-UNPROVEN at baseline SHA).

## 3. Findings

### F-001 [S2] The runtime-loss lane has no producer: no JS anywhere sends `surfaceLost` — a real device/context loss will never reach the state machine
Where: apps/web/src/render/backend.rs:348-349 (doc claim) — with apps/web/src/protocol.rs:89,121 and apps/web/worker.js:26-27
Code:
```rust
    /// Report a surface loss for `backend` (the JS glue's
    /// `webglcontextlost` / `device.lost` report). Falls ONE step
    /// down the ladder through backends the probe facts actually
    /// support, skipping consumed ones; the lost backend is consumed
```
What is wrong: The doc says the loss report comes from "the JS glue". The JS glue does no such thing: the only occurrence of `surfaceLost` in any js/ts/html file is worker.js:26-27, a comment ("`surfaceLost` reports a dead canvas context / WebGPU device"). No `addEventListener("webglcontextlost")`, no `device.lost` await, no `postMessage({type:"surfaceLost"})` exists anywhere (`rg -i "lost|webglcontext|device\.lost" ui/e2e/web/site/harness.js apps/web/worker.js` → comments only). The consumer side is equally absent: `renderDegraded` (the transition the host emits at host.rs:1052-1056) has no JS reader either.
Why it matters: The runtime-loss half (LossReason, LaneTransition, consumed/degraded bookkeeping, `RenderDegraded` message, ~130 lines of state machine + 2 dedicated tests) is dead in every non-test context. In a real browser session — the exact scenario the module documents (backend.rs:27-37: "A device lost mid-session … never a frozen last frame pretending to play") — a WebGL2 context loss or WebGPU device loss will never be reported to the engine: the lane keeps its `active` backend, keeps pumping `PresentFrame`s to a dead canvas, and the audio-only degrade never fires. The documented anti-goal becomes the actual behavior.
What correct looks like: worker.js (or the harness page) listens for `webglcontextlost` on the present canvas and `device.lost` on the WebGPU device, and posts `{type:"surfaceLost", backend, reason}` per protocol.rs:89-93; until then the module docs must state the loss lane is engine-side-only with the browser trigger unimplemented.
Confidence: HIGH — Verification: read + repo-wide rg (both directions: no sender, no consumer); Rust side traced; tests green only prove the Rust half in isolation.

### F-002 [S3] `RenderLane::snapshot` can violate `RendererNegotiation`'s documented invariant (`active=None` with `reason=None`) and its string then lies about why
Where: apps/web/src/render/backend.rs:143-145 + 305-320
Code:
```rust
    /// The staged refusal reason, set exactly when `active` is
    /// `None`.
    pub reason: Option<UnavailableReason>,
```
and
```rust
        let reason = if self.active.is_some() {
            None
        } else {
            // The probe-facts outcome's own reason (the lane's
            // runtime-loss vocabulary is separate — see
            // [`RenderLane::lane_string`]).
            negotiate(caps).reason
        };
```
What is wrong: After a runtime loss to `None` (e.g. boot `{api:true, adapter:true, webgl2:false}` → WebGpu active → `surface_lost(WebGpu, _)`), `snapshot()` recomputes the reason from the PROBE facts: `negotiate(caps)` returns `active=Some(WebGpu), reason=None`, so `snapshot()` yields `RendererNegotiation{active: None, reason: None}` — breaking the struct's own contract ("set exactly when `active` is `None`"). Feeding that value to `snapshot_string()` (156-166) prints `"unavailable (neither WebGPU nor WebGL2 produced a usable context)"` — false: both probes succeeded; the device died mid-session.
Why it matters: `render_snapshot()` is the diagnostics accessor (host.rs:961-966, "tests + diagnostics") — a diagnostic that misattributes a runtime loss to probe failure sends an engineer hunting the wrong layer. Blast radius limited: `booted_capabilities` (the shipped snapshot path, host.rs:953) uses `lane_string()`, which handles the degraded state correctly (331-338); no non-test caller of `render_snapshot` found.
What correct looks like: `snapshot()` should surface the degraded state — either map `degraded_reason` into the reason slot with a loss-specific vocabulary, or return `None`-shaped diagnostics that name the loss, mirroring `lane_string`'s postmortem branch.
Confidence: HIGH — Verification: read + control-flow trace of both branches; no execution (no cargo per rules); the existing tests never call `snapshot()` after a loss (`unprobed_lane_has_no_snapshot` covers only the pre-boot state), so the gap is untested.

### F-003 [S4] `settings_label` is dead public API
Where: apps/web/src/render/backend.rs:91-100
Code:
```rust
    /// The settings-page label (doc 09 §4.4: the page states e.g.
    /// "Rendering: WebGPU" / "WebGL2 (fallback)" — the fallback marker
    /// is part of the honest label, not decoration).
    #[must_use]
    pub fn settings_label(self) -> &'static str {
```
What is wrong: Zero callers outside backend.rs's own tests (`rg "settings_label"` repo → only this file). No web settings page exists in the repo; the React `ui/` settings screens do not consume wasm-side labels.
Why it matters: Public API whose doc claims a UI consumer that does not exist — a small doc-vs-reality lie of the same species as F-001.
What correct looks like: Either delete it until the page exists, or mark it `#[cfg(test)]`/document "no consumer yet".
Confidence: HIGH — Verification: repo-wide rg.

### F-004 [S4] The e2e browser evidence cited by this module is one Chromium run at a foreign commit
Where: apps/web/src/render/backend.rs:23-25 (module doc: "the real probes remain browser-side `[VERIFY]`") — evidence file docs/web/browser-matrix-20261110.md
Code:
```rust
//! facts are DATA, so every branch of the ladder is testable
//! headless-native against simulated capability reports (the tests
//! below), while the real probes remain browser-side `[VERIFY]`.
```
What is wrong: Nothing in the code; the honesty label is correct. The record: the only real-browser execution of this module (Playwright Chromium, 2026-11-11) was captured at commit `3755dc3` on branch `w24/a` — not the baseline SHA `e83e5bd` (browser-matrix-20261110.md "Honesty note": Firefox/WebKit/Safari never executed). The `[VERIFY]` debt is still open at baseline: no browser exists in the audit sandbox (INDEX "Tooling gaps").
Why it matters: The negotiation half is REAL-UNPROVEN at the audited commit: real at 3755dc3, green natively at e83e5bd, but no baseline-browser run pins the two together.
What correct looks like: A browser-matrix rerun at the audited SHA (REQUEST-ORCH-RUN below).
Confidence: HIGH — Verification: read of the committed matrix report + INDEX.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RendererCaps::NONE` const | 57-65 | REAL | — | Used in host.rs tests; trivial. |
| `RenderBackend::as_str` | 80-89 | REAL | — | Wire ids "webgpu"/"webgl2"; matched by harness assertions + matrix record. |
| `RenderBackend::settings_label` | 91-100 | REAL | S4 | F-003 dead API. |
| `UnavailableReason::as_str` | 120-134 | REAL | — | Sentences used verbatim in snapshot strings (test-pinned 486-490). |
| `RendererNegotiation::snapshot_string` | 151-166 | REAL | S3 | `unwrap_or(NoWebGl2Context)` fallback only fires for the invariant-breaking state of F-002. |
| `negotiate` | 169-210 | REAL | — | Pure ladder; 3rd reason branch honestly documented as unreachable (197-200). All 8 combos test-pinned. |
| `LossReason::as_str` | 225-234 | REAL | S2 | Fine itself; the vocabulary it feeds (F-001) has no producer. |
| `LaneTransition::is_audio_only` | 249-257 | REAL | — | Trivial; test-pinned. |
| `RenderLane::new`/`Default` | 276-286, 390-394 | REAL | — | Trivial. |
| `RenderLane::boot` | 288-299 | REAL | — | Resets consumed + degraded (fresh lane semantics); test `reboot_renegotiates_from_fresh_facts`. |
| `RenderLane::snapshot` | 301-320 | REAL | S3 | F-002 invariant violation post-loss. |
| `RenderLane::lane_string` | 322-340 | REAL | — | Correct postmortem branch; this is the SHIPPED snapshot path (host.rs:953). |
| `RenderLane::active` | 342-346 | REAL | — | Trivial accessor. |
| `RenderLane::surface_lost` | 348-387 | REAL (Rust) / UNREACHABLE (browser) | S2 | Logic traced correct (stale-ignore, one-step fall, consume, no fall-up); trigger wiring absent — F-001. |
| `backend_index` (private) | 396-402 | REAL | — | Trivial. |
| Structs/enums `RendererCaps`/`RenderBackend`/`UnavailableReason`/`RendererNegotiation`/`LossReason`/`LaneTransition`/`RenderLane` | 44-55, 68-78, 106-118, 138-149, 214-223, 238-247, 261-274 | REAL | — | serde camelCase/kebab-case wire shapes consumed by protocol.rs + harness JSON (matrix record shows the shapes round-tripped in a real browser). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_ladder_over_every_capability_report` (413-448) | YES | Mutations: drop the `webgpu_adapter` conjunct → api=true,adapter=false cases fail; drop webgl2 floor → webgl2-only cases fail; swap reason order → reason asserts fail. All 8 combos × active+reason asserted with `{caps:?}` context. |
| `adapter_without_api_is_ignored` (452-468) | YES | Pins the lying-probe rule + the exact sentence. |
| `snapshot_strings_are_the_honest_answer` (472-493) | YES | Byte-exact string asserts incl. the double-parenthesized unavailable sentence; also pins settings_label strings. |
| `loss_falls_one_step_and_consumes` (498-539) | YES | Full transition struct equality; stale-loss ignored; floor loss → audio-only; degraded `lane_string` contains loss vocabulary. Would catch a consumed-flag or degraded_reason regression. |
| `webgpu_loss_without_the_floor_degrades_to_audio_only` (544-557) | YES | Fallback consults probe facts, never assumes. |
| `unprobed_lane_has_no_snapshot` (562-568) | YES | Pins the None-before-boot shape. |
| `reboot_renegotiates_from_fresh_facts` (572-595) | YES | Consumed state cleared on boot; fresh facts win. |
| GAP | — | No test exercises `snapshot()` AFTER a loss (F-002's broken state is untested); no test covers `surface_lost` arriving via the actual wire (impossible today — no sender, F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the ACTIVE backend is the only thing the capability snapshot ever reports … never a guess" | backend.rs:7-9; doc 09 §4.4 | REAL | host.rs:953 `booted_capabilities` → `lane_string`; matrix record 65 `"renderer":"webgl2"`. |
| "the page performs doc 09 §4.4's probe sequence" | backend.rs:13-19 | REAL (e2e harness only) | harness.js:148-165 `probeRenderer()`; no product page exists. |
| "the JS glue's `webglcontextlost` / `device.lost` report" | backend.rs:348-349 | FALSE | F-001: no JS listens or sends. |
| "never a frozen last frame pretending to play" | backend.rs:33-34 | FALSE in-browser / REAL in-Rust | The degrade only fires on a message nothing sends (F-001); in Rust tests it fires correctly. |
| "Re-negotiation after total loss is a NEW lane … [PROPOSAL] recovery path" | backend.rs:35-37 | REAL | `boot` reset semantics + `reboot_renegotiates_from_fresh_facts`. |
| Ladder = WebGPU preferred, WebGL2 floor, unavailable honest | doc 09 §4.4 / backend.rs:5-8 | REAL | All-8-combos test + real Chromium matrix row. |

## 7. Dead / duplicated / unreachable code
- The runtime-loss lane (LossReason, LaneTransition, `surface_lost`, consumed/degraded fields, `RenderDegraded` emission at host.rs:1052-1056): unreachable from any browser surface; Rust-test-only (F-001).
- `settings_label` (F-003).
- No duplication found in-crate.

## 8. Dependencies & flags
- Only serde (derive) — no feature flags; compiled for both native (workspace member, baseline build #1 compiles lumen-web rlib) and wasm32 (cdylib, NOT built in baseline: no wasm32 target run in INDEX; the shipped-artifact lane is scripts/web-matrix-run.mjs + xtask web-budget #16, which checks committed size records, not a fresh wasm build).
- The crate's own README/Cargo.toml admit the wasm module is decoder-less (no builtin-codecs feature by default) — so even where this state machine runs, nothing decodes (parity web=0/252, INDEX #9).

## 9. Open questions for the next agent
- A03-A06 (host.rs parts): does any host.rs path OTHER than the Boot handler construct `RendererCaps`? (host.rs:1019 is the only boot site I found; harness is the only sender.)
- A01 (worker.js + io/mock_fetch.rs): worker.js line 23 says boot "may carry renderer" — confirm the worker forwards an absent `renderer` field as `None` (unprobed) rather than `RendererCaps::NONE` (probed-and-refused); the distinction is this module's unprobed-vs-unavailable contract.
- Wave-19 (docs/web): browser-matrix-20261110.md is the sole real-browser evidence for the web lane — audit its provenance chain (json record → md renderer) for drift vs the current harness.
- REQUEST-ORCH-RUN (for the wave gate): `node ui/scripts/web-matrix-run.mjs --restamp` (or a fresh run with WEB_MATRIX_CHROMIUM set) at the baseline SHA, to re-pin renderer-ladder evidence at e83e5bd.
