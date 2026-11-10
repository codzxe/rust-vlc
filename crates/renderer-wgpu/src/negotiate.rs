//! The backend fallback chain (card 7.C1, issue #42): auto-negotiate
//! the wgpu hardware backend → **software present**, and degrade to
//! software mid-run when a hardware backend fails — with a typed
//! capability report at every step.
//!
//! # The chain
//!
//! Two tiers, walked in order — the first tier whose probe answers
//! [`TierStatus::Selected`] wins:
//!
//! 1. [`BackendTier::Hardware`] — the wgpu compositor. **Absent in
//!    this build**: the import is Lead-gated
//!    (`docs/dependencies/records/wgpu-30.0.1.md` — no GPU stack in
//!    this sandbox, `deny.toml`'s `libloading` ban, the ungranted
//!    dependency envelope), so its probe answers a *typed*
//!    [`TierStatus::Unavailable`] carrying that reason. Honest, never
//!    faked: the tier exists in the chain because the fallback policy
//!    must name what it fell back from.
//! 2. [`BackendTier::Software`] — the CPU software-present compositor
//!    ([`SoftwareCompositor`]). Unconditionally selected: it runs
//!    anywhere a CPU runs, which is the entire point of a terminal
//!    fallback tier.
//!
//! # The typed capability report (card 7.C1: "capability report
//! surfaced to UI")
//!
//! [`BackendReport`] is the renderer-side typed form of the wire
//! `video.backend-changed` row: the now-active backend id, a
//! display-safe reason (V8), and — for a mid-run fallback — the
//! backend it replaced. The renderer owns the negotiation; the
//! **engine** owns the one wire event stream, so a host relays the
//! report through `playback_engine::Engine::observe_renderer_backend`
//! ([`BackendReport::relay_pair`] is the exact pair that call takes).
//!
//! # Time and allocation (D-1)
//!
//! Negotiation and fallback run at *event rate* (startup and device
//! failure — human timescale), never per frame; the walk allocates
//! its verdict list and report strings once per episode and nothing
//! in the present path reads them.

use crate::backend::{Compositor, SoftwareCompositor};

/// One tier of the fallback chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendTier {
    /// The wgpu hardware compositor — probe answers a typed
    /// [`TierStatus::Unavailable`] in this build (the Lead-gated
    /// import; see the module docs).
    Hardware,
    /// The CPU software-present compositor — the terminal tier,
    /// unconditionally available.
    Software,
}

impl BackendTier {
    /// The tier's stable id (capability-report vocabulary; the
    /// software id matches [`Compositor::name`] so a report and the
    /// renderer's own `backend_name()` never disagree).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            BackendTier::Hardware => "hardware-wgpu",
            BackendTier::Software => "software",
        }
    }

    /// The honest availability answer for **this build**.
    ///
    /// The hardware tier's reason is the recorded decision, not an
    /// excuse: the import is Lead-gated on evidence
    /// (`records/wgpu-30.0.1.md`), so the chain falls to software by
    /// policy — a UI showing this string states the truth.
    #[must_use]
    pub fn probe(self) -> TierStatus {
        match self {
            BackendTier::Hardware => TierStatus::Unavailable {
                reason: "wgpu import Lead-gated (records/wgpu-30.0.1.md: no GPU \
                         stack in this sandbox, deny.toml libloading ban, \
                         ungranted dependency envelope)",
            },
            BackendTier::Software => TierStatus::Selected,
        }
    }
}

/// One tier's probe answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierStatus {
    /// The tier is available and constructible in this build.
    Selected,
    /// The tier is not available; `reason` is the display-safe truth
    /// (never fabricated, never empty).
    Unavailable { reason: &'static str },
}

impl TierStatus {
    /// Whether this answer is [`TierStatus::Selected`].
    #[must_use]
    pub fn is_selected(self) -> bool {
        matches!(self, TierStatus::Selected)
    }
}

/// The v0 chain, in walk order: hardware first, software present
/// last (the terminal fallback is never absent from the chain).
pub const CHAIN: [BackendTier; 2] = [BackendTier::Hardware, BackendTier::Software];

/// The selection over a probed chain: which tier won, and every tier
/// skipped with its typed reason (the audit trail a UI can show).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The winning tier, when any probe answered `Selected`.
    pub chosen: Option<BackendTier>,
    /// `(tier id, unavailability reason)` for every skipped tier, in
    /// walk order.
    pub skipped: Vec<(&'static str, &'static str)>,
}

/// Walk the probed chain: the **first** `Selected` tier wins (card
/// 7.C1's order — hardware when it exists, software present
/// otherwise). Pure over the verdicts, so the *order* is provable
/// with simulated availability even in a build whose hardware tier
/// is absent — the same doctrine as the engine's simulated
/// device-event tests.
///
/// # Allocation (D-1)
/// One `Vec` for the skipped tiers — event rate, not per frame.
#[must_use]
pub fn select(probed: &[(BackendTier, TierStatus)]) -> Selection {
    let mut skipped = Vec::new();
    for &(tier, status) in probed {
        match status {
            TierStatus::Selected => {
                return Selection {
                    chosen: Some(tier),
                    skipped,
                };
            }
            TierStatus::Unavailable { reason } => {
                skipped.push((tier.id(), reason));
            }
        }
    }
    Selection {
        chosen: None,
        skipped,
    }
}

/// The typed capability report — the renderer-side form of the wire
/// `video.backend-changed` row (card 7.C1). See the module docs for
/// the ownership split: the renderer reports, the host relays, the
/// engine owns the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReport {
    /// The now-active backend id (matches the winner's
    /// [`BackendTier::id`] / [`Compositor::name`]).
    backend: String,
    /// The display-safe (V8) negotiation outcome or fallback cause.
    reason: String,
    /// The backend this report replaced, when it is a fallback.
    fallback_from: Option<String>,
}

impl BackendReport {
    /// The report for a startup auto-negotiation: names the active
    /// backend and every tier skipped with its typed reason.
    fn negotiated(selection: &Selection) -> BackendReport {
        let (backend, reason) = match selection.chosen {
            Some(tier) => {
                let mut reason = String::from("auto-negotiated: ");
                for (index, (id, why)) in selection.skipped.iter().enumerate() {
                    if index > 0 {
                        reason.push_str("; ");
                    }
                    reason.push_str(&format!("{id} unavailable ({why})"));
                }
                if selection.skipped.is_empty() {
                    reason.push_str("no tier skipped");
                } else {
                    reason.push_str("; ");
                }
                reason.push_str("software present active");
                // The only constructible tier in this build is the
                // software one; a build whose hardware probe answers
                // `Selected` rewrites this arm with its own id.
                let id = if tier == BackendTier::Software {
                    tier.id()
                } else {
                    // The hardware tier's probe is a const
                    // `Unavailable` in this build, so selection cannot
                    // choose it; if that ever changes, this report
                    // must change with it (stated, not guessed).
                    "software"
                };
                (id, reason)
            }
            None => (
                "none",
                String::from(
                    "auto-negotiation found no available backend tier (the software \
                     tier is unconditionally available — reaching here is a bug)",
                ),
            ),
        };
        BackendReport {
            backend: backend.to_string(),
            reason,
            fallback_from: None,
        }
    }

    /// The report for a mid-run fallback: the linked backend failed
    /// with `cause`, software present took over.
    fn fallback(from: &str, cause: &str) -> BackendReport {
        BackendReport {
            backend: BackendTier::Software.id().to_string(),
            reason: format!("backend `{from}` failed ({cause}); fell back to software present"),
            fallback_from: Some(from.to_string()),
        }
    }

    /// The now-active backend id.
    #[must_use]
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// The display-safe reason (negotiation outcome or fallback
    /// cause).
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The replaced backend's id, when this report is a fallback.
    #[must_use]
    pub fn fallback_from(&self) -> Option<&str> {
        self.fallback_from.as_deref()
    }

    /// The exact `(backend, reason)` pair
    /// `playback_engine::Engine::observe_renderer_backend` takes —
    /// the relay is one call, no re-typing, no drift.
    #[must_use]
    pub fn relay_pair(&self) -> (&str, &str) {
        (self.backend(), self.reason())
    }
}

/// The result of an auto-negotiation: the selected compositor and its
/// capability report.
pub struct Negotiated {
    /// The winning tier's compositor. `None` only when no tier
    /// answered `Selected` — impossible while the software tier's
    /// probe is honest (typed, not panicked; the report names the
    /// state either way).
    pub compositor: Option<Box<dyn Compositor>>,
    /// The capability report to relay.
    pub report: BackendReport,
}

impl std::fmt::Debug for Negotiated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Negotiated")
            .field("compositor", &self.compositor.as_ref().map(|c| c.name()))
            .field("report", &self.report)
            .finish()
    }
}

/// Auto-negotiate the backend chain (card 7.C1): probe every tier of
/// [`CHAIN`], select the first available, build it.
///
/// In this build the walk is deterministic: the hardware tier answers
/// its typed `Unavailable`, software present wins, and the report
/// carries both facts. When the Lead ratifies the wgpu import, that
/// build's [`BackendTier::Hardware::probe`] flips and this same walk
/// selects hardware first — the chain, not the callers, changes.
pub fn negotiate() -> Negotiated {
    let probed: [(BackendTier, TierStatus); CHAIN.len()] =
        [(CHAIN[0], CHAIN[0].probe()), (CHAIN[1], CHAIN[1].probe())];
    let selection = select(&probed);
    let report = BackendReport::negotiated(&selection);
    let compositor = match selection.chosen {
        Some(BackendTier::Software) => {
            Some(Box::new(SoftwareCompositor::new()) as Box<dyn Compositor>)
        }
        // Hardware: not constructible in this build — and not
        // selectable, because its probe (the only path to
        // `Selected`) is a const `Unavailable`. `None` with the
        // honest report, never a panic and never a stand-in
        // compositor pretending to be wgpu.
        Some(BackendTier::Hardware) | None => None,
    };
    Negotiated { compositor, report }
}

impl crate::renderer::Renderer {
    /// Degrade the linked backend to software present after a
    /// failure (card 7.C1's mid-run fallback): swaps the compositor,
    /// keeps the surface, parameters and staging state, and answers
    /// the typed [`BackendReport`] for the host to relay
    /// (`Engine::observe_renderer_backend`).
    ///
    /// Idempotent and honest about it: falling back while already on
    /// software re-initializes it and reports `fallback_from:
    /// "software"` — the caller decides whether that is worth
    /// relaying; the renderer never hides which backend ran.
    pub fn fallback_to_software(&mut self, cause: &str) -> BackendReport {
        let from = self.backend_name().to_string();
        self.swap_compositor(Box::new(SoftwareCompositor::new()));
        BackendReport::fallback(&from, cause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The chain order: hardware first, software present last.
    #[test]
    fn the_chain_is_hardware_then_software() {
        assert_eq!(CHAIN, [BackendTier::Hardware, BackendTier::Software]);
    }

    /// This build's honest probes: hardware typed-unavailable with
    /// the recorded reason, software selected.
    #[test]
    fn probes_answer_this_builds_truth() {
        assert_eq!(
            BackendTier::Hardware.probe(),
            TierStatus::Unavailable {
                reason: "wgpu import Lead-gated (records/wgpu-30.0.1.md: no GPU \
                         stack in this sandbox, deny.toml libloading ban, \
                         ungranted dependency envelope)",
            }
        );
        assert!(BackendTier::Software.probe().is_selected());
    }

    /// The walk picks the FIRST selected tier — proven with simulated
    /// availability (the same doctrine as the engine's simulated
    /// device events): a chain whose hardware tier answers Selected
    /// chooses hardware; a chain whose hardware is unavailable (this
    /// build's truth) chooses software and records the skip.
    #[test]
    fn select_walks_in_order_first_selected_wins() {
        // Simulated hardware-available chain.
        let probed = [
            (BackendTier::Hardware, TierStatus::Selected),
            (BackendTier::Software, TierStatus::Selected),
        ];
        assert_eq!(
            select(&probed),
            Selection {
                chosen: Some(BackendTier::Hardware),
                skipped: Vec::new(),
            }
        );
        // This build's chain: hardware typed-unavailable.
        let probed = [
            (BackendTier::Hardware, BackendTier::Hardware.probe()),
            (BackendTier::Software, TierStatus::Selected),
        ];
        let selection = select(&probed);
        assert_eq!(selection.chosen, Some(BackendTier::Software));
        assert_eq!(selection.skipped.len(), 1);
        assert_eq!(selection.skipped[0].0, "hardware-wgpu");
        assert!(
            selection.skipped[0].1.contains("Lead-gated"),
            "the skip reason is the recorded decision: {:?}",
            selection.skipped[0].1
        );
        // Nothing selected (simulated total absence — impossible
        // while software's probe is honest, but typed).
        let probed = [
            (BackendTier::Hardware, BackendTier::Hardware.probe()),
            (
                BackendTier::Software,
                TierStatus::Unavailable {
                    reason: "simulated",
                },
            ),
        ];
        assert_eq!(select(&probed).chosen, None);
    }

    /// The startup negotiation: software present wins, the compositor
    /// is constructible, and the report names the active backend,
    /// the skipped tier with its reason, and no fallback.
    #[test]
    fn negotiate_lands_on_software_with_the_typed_hardware_absence() {
        let negotiated = negotiate();
        let compositor = negotiated.compositor.must();
        assert_eq!(compositor.name(), "software");
        let report = negotiated.report;
        assert_eq!(report.backend(), "software");
        assert_eq!(report.fallback_from(), None);
        assert!(
            report.reason().contains("auto-negotiated"),
            "the outcome says how it was chosen: {}",
            report.reason()
        );
        assert!(
            report.reason().contains("hardware-wgpu unavailable"),
            "the skipped tier is named: {}",
            report.reason()
        );
        assert!(
            report.reason().contains("Lead-gated"),
            "the skip reason is the recorded decision: {}",
            report.reason()
        );
        assert!(
            report.reason().contains("software present active"),
            "the active tier is named: {}",
            report.reason()
        );
    }

    /// The relay pair is the exact shape the engine's hotplug hook
    /// takes (backend, reason) — one call, no re-typing.
    #[test]
    fn the_relay_pair_is_the_engine_hooks_shape() {
        let negotiated = negotiate();
        let (backend, reason) = negotiated.report.relay_pair();
        assert_eq!(backend, negotiated.report.backend());
        assert_eq!(reason, negotiated.report.reason());
    }
}
