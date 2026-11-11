//! Backend negotiation (card 7.H2, issue #48; doc 09 §4.1/§4.4).
//!
//! # What this module is
//!
//! The **backend negotiation state machine** for the browser render
//! lane: WebGPU preferred, WebGL2 the universal floor, `unavailable`
//! the honest answer — and the ACTIVE backend is the only thing the
//! capability snapshot ever reports (doc 09 §4.4: "the active backend
//! is reported … never a guess, never an aspiration").
//!
//! # The split (why a `RendererCaps` arrives from outside)
//!
//! The wasm module has no DOM and no `navigator`: the PAGE performs
//! doc 09 §4.4's probe sequence —
//!
//! 1. `navigator.gpu` present?
//! 2. `requestAdapter({ powerPreference: "high-performance" })`
//!    resolved non-null?
//! 3. `canvas.getContext("webgl2")` non-null? (the universal floor)
//!
//! — and reports the raw facts with the [`HostToWorker::Boot`]
//! handshake. This module walks the ladder from those facts; the
//! facts are DATA, so every branch of the ladder is testable
//! headless-native against simulated capability reports (the tests
//! below), while the real probes remain browser-side `[VERIFY]`.
//!
//! # Runtime loss (the second half of the state machine)
//!
//! A device lost mid-session maps to the typed surface-lost
//! vocabulary (doc 09 §4.4 → doc 03 §4.11 d): the lane falls ONE step
//! down the ladder — WebGPU → WebGL2 — and a backend that was lost is
//! CONSUMED (never retried within the lane: a flapping backend must
//! not ping-pong the canvas). Losing the floor is the audio-only
//! degrade: an explicit event, never a frozen last frame pretending
//! to play. Re-negotiation after total loss is a NEW lane (the page
//! re-probes and re-boots the render surface) — recorded as the
//! `[PROPOSAL]` recovery path for card 7.I1's matrix to exercise.

use serde::{Deserialize, Serialize};

/// The page's raw renderer probe facts (doc 09 §4.4's boot sequence,
/// reported with the handshake). Every field is the ACTUAL probe
/// outcome — the page never guesses on the engine's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererCaps {
    /// `navigator.gpu` is present (the WebGPU API exists at all).
    pub webgpu_api: bool,
    /// `requestAdapter({ powerPreference: "high-performance" })`
    /// resolved with a non-null adapter.
    pub webgpu_adapter: bool,
    /// `canvas.getContext("webgl2")` answered a non-null context (the
    /// universal floor — BCD-verified 2026-10-19 per doc 09 §4.1).
    pub webgl2_context: bool,
}

impl RendererCaps {
    /// The all-absent report (the honest floor of the ladder: nothing
    /// probes true ⇒ the typed `unavailable` answer).
    pub const NONE: RendererCaps = RendererCaps {
        webgpu_api: false,
        webgpu_adapter: false,
        webgl2_context: false,
    };
}

/// An active present backend (the negotiation's non-fatal outcomes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderBackend {
    /// `navigator.gpu` + a live adapter — the preferred path (doc 09
    /// §4.1's WebGPU surface).
    WebGpu,
    /// The WebGL2 context — the universal floor and the deliberate
    /// first-class path (Firefox-Android-class browsers live here
    /// permanently; doc 09 §4.4).
    WebGl2,
}

impl RenderBackend {
    /// The wire id (the capability snapshot's `renderer` field and
    /// `system.get-engine-info`'s backend analog).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RenderBackend::WebGpu => "webgpu",
            RenderBackend::WebGl2 => "webgl2",
        }
    }

    /// The settings-page label (doc 09 §4.4: the page states e.g.
    /// "Rendering: WebGPU" / "WebGL2 (fallback)" — the fallback marker
    /// is part of the honest label, not decoration).
    #[must_use]
    pub fn settings_label(self) -> &'static str {
        match self {
            RenderBackend::WebGpu => "WebGPU",
            RenderBackend::WebGl2 => "WebGL2 (fallback)",
        }
    }
}

/// Why the ladder ended at `unavailable` — the STAGED reasons, each
/// naming the probe step that refused (doc 09 §4.4's truthful
/// negotiation: the UI's unsupported state says what was missing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailableReason {
    /// `navigator.gpu` is absent — the whole WebGPU half of the
    /// ladder is unreachable on this browser.
    NoWebGpuApi,
    /// The API exists but `requestAdapter` refused (blocklisted
    /// driver, out of resources) — and no WebGL2 context either.
    NoWebGpuAdapter,
    /// Neither half answered: the typed `renderer-unavailable` fatal
    /// (doc 09 §4.4 step 4 — the UI shows the unsupported state).
    NoWebGl2Context,
}

impl UnavailableReason {
    /// The human sentence (V7 — text, never markup).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            UnavailableReason::NoWebGpuApi => "navigator.gpu is absent (no WebGPU on this browser)",
            UnavailableReason::NoWebGpuAdapter => {
                "requestAdapter refused and no WebGL2 context answered"
            }
            UnavailableReason::NoWebGl2Context => {
                "neither WebGPU nor WebGL2 produced a usable context"
            }
        }
    }
}

/// The negotiation outcome for one probe report: the active backend
/// or the staged reason it is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererNegotiation {
    /// The active backend, when the ladder found one.
    pub active: Option<RenderBackend>,
    /// The staged refusal reason, set exactly when `active` is
    /// `None`.
    pub reason: Option<UnavailableReason>,
    /// The probe facts this outcome was computed from (the snapshot
    /// quotes them — evidence, not a claim).
    pub caps: RendererCaps,
}

impl RendererNegotiation {
    /// The capability snapshot's `renderer` string: the ACTIVE
    /// backend, or the honest `unavailable (…)` sentence naming the
    /// refusing probe step. Never a guess, never an aspiration.
    #[must_use]
    pub fn snapshot_string(&self) -> String {
        match self.active {
            Some(backend) => backend.as_str().to_string(),
            None => format!(
                "unavailable ({})",
                self.reason
                    .unwrap_or(UnavailableReason::NoWebGl2Context)
                    .as_str()
            ),
        }
    }
}

/// Walk doc 09 §4.4's ladder from one probe report (the pure core —
/// the same function the boot handshake and the re-negotiation
/// recovery both call).
///
/// The ladder's rules, each pinned by a test:
///
/// - WebGPU needs BOTH the API and a live adapter (`webgpu_api &&
///   webgpu_adapter`) — an adapter claim without the API is ignored
///   (a lying probe report never manufactures a backend).
/// - WebGL2 is the floor: `webgl2_context` alone activates it.
/// - Otherwise `unavailable` with the staged reason: API absent →
///   adapter refused → no context (each names the first refusing
///   step in the probe order).
#[must_use]
pub fn negotiate(caps: RendererCaps) -> RendererNegotiation {
    let active = if caps.webgpu_api && caps.webgpu_adapter {
        Some(RenderBackend::WebGpu)
    } else if caps.webgl2_context {
        Some(RenderBackend::WebGl2)
    } else {
        None
    };
    let reason = if active.is_none() {
        Some(if !caps.webgpu_api {
            UnavailableReason::NoWebGpuApi
        } else if !caps.webgpu_adapter {
            UnavailableReason::NoWebGpuAdapter
        } else {
            // Unreachable by the ladder's construction (an adapter
            // implies active); kept total so the reason type stays
            // exhaustive without a `None`-with-`Some(reason)` state.
            UnavailableReason::NoWebGl2Context
        })
    } else {
        None
    };
    RendererNegotiation {
        active,
        reason,
        caps,
    }
}

/// Why a live surface went away (the typed loss vocabulary's cause
/// half; the effect half is [`LaneTransition`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LossReason {
    /// `webglcontextlost` — the WebGL2 context died (the classic
    /// driver reset path).
    ContextLost,
    /// The WebGPU device was lost (`device.lost` resolved) — doc 09
    /// §4.4's "adapter/device loss".
    DeviceLost,
}

impl LossReason {
    /// The human sentence.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LossReason::ContextLost => "the WebGL2 context was lost",
            LossReason::DeviceLost => "the WebGPU device was lost",
        }
    }
}

/// One runtime loss transition's observable outcome (what the host
/// surfaces to the UI as the explicit degrade event).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneTransition {
    /// The backend that was presenting when the loss arrived.
    pub from: RenderBackend,
    /// The backend the lane fell back to, if any.
    pub to: Option<RenderBackend>,
    /// Why (the loss vocabulary's cause).
    pub reason: LossReason,
}

impl LaneTransition {
    /// Whether this transition is the audio-only degrade (no backend
    /// remains — doc 09 §4.4: "never a frozen last frame pretending
    /// to play").
    #[must_use]
    pub fn is_audio_only(&self) -> bool {
        self.to.is_none()
    }
}

/// The live render lane: the negotiation plus the runtime loss state
/// (which backends are consumed). One per booted host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderLane {
    /// The active backend or `None` after a total loss / before
    /// boot.
    active: Option<RenderBackend>,
    /// The probe facts (kept: a loss consults them for the fallback
    /// step — e.g. WebGPU loss falls to WebGL2 only if the page's
    /// probe found a context).
    caps: Option<RendererCaps>,
    /// Backends consumed by a loss (never retried within this lane).
    consumed: [bool; 2],
    /// The last loss's reason, when the lane is degraded to none.
    degraded_reason: Option<(RenderBackend, LossReason)>,
}

impl RenderLane {
    /// An unprobed lane (before the handshake).
    #[must_use]
    pub fn new() -> RenderLane {
        RenderLane {
            active: None,
            caps: None,
            consumed: [false, false],
            degraded_reason: None,
        }
    }

    /// Apply the handshake's probe report (the page probed, then
    /// booted — doc 09 §4.4's order). Idempotent per boot: a second
    /// call re-negotiates from the fresh facts (the page's own
    /// recovery path).
    pub fn boot(&mut self, caps: RendererCaps) -> RendererNegotiation {
        self.caps = Some(caps);
        self.consumed = [false, false];
        self.degraded_reason = None;
        let negotiation = negotiate(caps);
        self.active = negotiation.active;
        negotiation
    }

    /// The lane's current negotiation-shaped snapshot (unprobed
    /// before the handshake — the honest "the page did not report"
    /// state, distinct from a probe that refused).
    #[must_use]
    pub fn snapshot(&self) -> Option<RendererNegotiation> {
        let caps = self.caps?;
        let reason = if self.active.is_some() {
            None
        } else {
            // The probe-facts outcome's own reason (the lane's
            // runtime-loss vocabulary is separate — see
            // [`RenderLane::lane_string`]).
            negotiate(caps).reason
        };
        Some(RendererNegotiation {
            active: self.active,
            reason,
            caps,
        })
    }

    /// The lane's presentable string: the ACTIVE backend, the honest
    /// `unavailable (probe: …)` refusal, or — after a runtime loss —
    /// the `unavailable (… was presenting via …)` postmortem. This is
    /// what the capability snapshot's `renderer` field quotes.
    #[must_use]
    pub fn lane_string(&self) -> Option<String> {
        let caps = self.caps?;
        Some(match self.active {
            Some(backend) => backend.as_str().to_string(),
            None => match self.degraded_reason {
                Some((backend, reason)) => format!(
                    "unavailable ({} — was presenting via {})",
                    reason.as_str(),
                    backend.as_str()
                ),
                None => negotiate(caps).snapshot_string(),
            },
        })
    }

    /// The active backend, if the lane is presenting.
    #[must_use]
    pub fn active(&self) -> Option<RenderBackend> {
        self.active
    }

    /// Report a surface loss for `backend` (the JS glue's
    /// `webglcontextlost` / `device.lost` report). Falls ONE step
    /// down the ladder through backends the probe facts actually
    /// support, skipping consumed ones; the lost backend is consumed
    /// (never retried here). Answers the transition to surface.
    ///
    /// A loss for a backend the lane is not presenting is ignored
    /// (answers `None`): a stale or duplicated report must not
    /// degrade a healthy lane.
    pub fn surface_lost(
        &mut self,
        backend: RenderBackend,
        reason: LossReason,
    ) -> Option<LaneTransition> {
        if self.active != Some(backend) {
            return None;
        }
        let index = backend_index(backend);
        self.consumed[index] = true;
        // The fallback: the floor, when the probe found it and it is
        // not consumed. The ladder never falls UP (WebGL2 loss does
        // not re-probe WebGPU — see the module docs).
        let fallback = if !self.consumed[backend_index(RenderBackend::WebGl2)]
            && self.caps.is_some_and(|caps| caps.webgl2_context)
            && backend == RenderBackend::WebGpu
        {
            Some(RenderBackend::WebGl2)
        } else {
            None
        };
        self.active = fallback;
        if fallback.is_none() {
            self.degraded_reason = Some((backend, reason));
        }
        Some(LaneTransition {
            from: backend,
            to: fallback,
            reason,
        })
    }
}

impl Default for RenderLane {
    fn default() -> Self {
        RenderLane::new()
    }
}

/// The ladder slot of one backend (`consumed` bookkeeping).
fn backend_index(backend: RenderBackend) -> usize {
    match backend {
        RenderBackend::WebGpu => 0,
        RenderBackend::WebGl2 => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// Every combination of the three probe booleans walks the
    /// ladder to the documented outcome (the simulated capability
    /// reports the card names — native-cfg, real arithmetic, no
    /// browser).
    #[test]
    fn the_ladder_over_every_capability_report() {
        for api in [false, true] {
            for adapter in [false, true] {
                for webgl2 in [false, true] {
                    let caps = RendererCaps {
                        webgpu_api: api,
                        webgpu_adapter: adapter,
                        webgl2_context: webgl2,
                    };
                    let outcome = negotiate(caps);
                    if api && adapter {
                        assert_eq!(outcome.active, Some(RenderBackend::WebGpu), "{caps:?}");
                        assert_eq!(outcome.reason, None, "{caps:?}");
                    } else if webgl2 {
                        assert_eq!(outcome.active, Some(RenderBackend::WebGl2), "{caps:?}");
                        assert_eq!(outcome.reason, None, "{caps:?}");
                    } else if !api {
                        assert_eq!(outcome.active, None, "{caps:?}");
                        assert_eq!(
                            outcome.reason,
                            Some(UnavailableReason::NoWebGpuApi),
                            "{caps:?}"
                        );
                    } else {
                        assert_eq!(outcome.active, None, "{caps:?}");
                        assert_eq!(
                            outcome.reason,
                            Some(UnavailableReason::NoWebGpuAdapter),
                            "{caps:?}"
                        );
                    }
                }
            }
        }
    }

    /// An adapter claim without the API never manufactures a
    /// backend (the lying-probe rule).
    #[test]
    fn adapter_without_api_is_ignored() {
        let outcome = negotiate(RendererCaps {
            webgpu_api: false,
            webgpu_adapter: true,
            webgl2_context: false,
        });
        assert_eq!(outcome.active, None);
        assert_eq!(outcome.reason, Some(UnavailableReason::NoWebGpuApi));
        assert!(
            outcome
                .snapshot_string()
                .contains("navigator.gpu is absent"),
            "{}",
            outcome.snapshot_string()
        );
    }

    /// The snapshot string reports the ACTIVE backend verbatim — or
    /// the honest `unavailable (…)` sentence (never a guess).
    #[test]
    fn snapshot_strings_are_the_honest_answer() {
        let webgpu = negotiate(RendererCaps {
            webgpu_api: true,
            webgpu_adapter: true,
            webgl2_context: true,
        });
        assert_eq!(webgpu.snapshot_string(), "webgpu");
        let floor = negotiate(RendererCaps {
            webgpu_api: false,
            webgpu_adapter: false,
            webgl2_context: true,
        });
        assert_eq!(floor.snapshot_string(), "webgl2");
        let none = negotiate(RendererCaps::NONE);
        assert_eq!(
            none.snapshot_string(),
            "unavailable (navigator.gpu is absent (no WebGPU on this browser))"
        );
        assert_eq!(RenderBackend::WebGl2.settings_label(), "WebGL2 (fallback)");
        assert_eq!(RenderBackend::WebGpu.settings_label(), "WebGPU");
    }

    /// The runtime loss ladder: WebGPU falls to WebGL2 when the
    /// probe found a context; the lost backend is consumed; WebGL2
    /// loss never falls UP; total loss is the audio-only degrade.
    #[test]
    fn loss_falls_one_step_and_consumes() {
        let mut lane = RenderLane::new();
        lane.boot(RendererCaps {
            webgpu_api: true,
            webgpu_adapter: true,
            webgl2_context: true,
        });
        assert_eq!(lane.active(), Some(RenderBackend::WebGpu));
        // WebGPU device loss → the WebGL2 floor.
        let step = lane
            .surface_lost(RenderBackend::WebGpu, LossReason::DeviceLost)
            .must();
        assert_eq!(
            step,
            LaneTransition {
                from: RenderBackend::WebGpu,
                to: Some(RenderBackend::WebGl2),
                reason: LossReason::DeviceLost,
            }
        );
        assert!(!step.is_audio_only());
        // A duplicated/stale WebGPU loss report is ignored.
        assert!(
            lane.surface_lost(RenderBackend::WebGpu, LossReason::DeviceLost)
                .is_none()
        );
        assert_eq!(lane.active(), Some(RenderBackend::WebGl2));
        // The floor's loss: no fallback (never falls up, the probe
        // offered nothing below) — the audio-only degrade.
        let final_step = lane
            .surface_lost(RenderBackend::WebGl2, LossReason::ContextLost)
            .must();
        assert_eq!(final_step.from, RenderBackend::WebGl2);
        assert_eq!(final_step.to, None);
        assert!(final_step.is_audio_only());
        assert_eq!(lane.active(), None);
        // The degraded snapshot names the loss, not a silent blank.
        let lane_string = lane.lane_string().must();
        assert!(lane_string.contains("context was lost"), "{lane_string}");
        assert!(lane_string.contains("webgl2"));
    }

    /// WebGPU loss with NO WebGL2 context probed is the audio-only
    /// degrade immediately (the fallback consults the probe facts,
    /// never assumes).
    #[test]
    fn webgpu_loss_without_the_floor_degrades_to_audio_only() {
        let mut lane = RenderLane::new();
        lane.boot(RendererCaps {
            webgpu_api: true,
            webgpu_adapter: true,
            webgl2_context: false,
        });
        let step = lane
            .surface_lost(RenderBackend::WebGpu, LossReason::DeviceLost)
            .must();
        assert!(step.is_audio_only());
        assert_eq!(lane.active(), None);
    }

    /// Before the handshake the lane reports nothing — the unprobed
    /// state is distinct from a refusal (the page did not report,
    /// and the snapshot must not invent an outcome either way).
    #[test]
    fn unprobed_lane_has_no_snapshot() {
        let lane = RenderLane::new();
        assert!(lane.snapshot().is_none());
        assert!(lane.lane_string().is_none());
        assert_eq!(lane.active(), None);
    }

    /// A fresh boot re-negotiates cleanly (the page's recovery
    /// path: re-probe → re-boot the render surface).
    #[test]
    fn reboot_renegotiates_from_fresh_facts() {
        let mut lane = RenderLane::new();
        lane.boot(RendererCaps {
            webgpu_api: true,
            webgpu_adapter: true,
            webgl2_context: true,
        });
        let _ = lane
            .surface_lost(RenderBackend::WebGpu, LossReason::DeviceLost)
            .must();
        let _ = lane
            .surface_lost(RenderBackend::WebGl2, LossReason::ContextLost)
            .must();
        assert_eq!(lane.active(), None);
        // The recovery: the page re-probed and found the floor back.
        let outcome = lane.boot(RendererCaps {
            webgpu_api: true,
            webgpu_adapter: false,
            webgl2_context: true,
        });
        assert_eq!(outcome.active, Some(RenderBackend::WebGl2));
        assert_eq!(lane.active(), Some(RenderBackend::WebGl2));
    }
}
