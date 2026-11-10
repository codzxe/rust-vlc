//! The autoplay-policy gate (card 7.H3, doc 09 §5.3).
//!
//! Browsers start an `AudioContext` suspended without a user
//! gesture (Chrome/WebKit/Firefox policies differ per browser and
//! version — `[VERIFY]` at test time per doc 09 §8). The binding
//! design doc 09 §5.3 pins:
//!
//! - `playback.play` is **accepted**; the engine enters its normal
//!   pre-play path — but a suspended-and-not-gesture-authorized
//!   context parks playback in a distinct, surfaced condition;
//! - the state presentation is *paused-pending-gesture* — an
//!   explicit substate, never a lie;
//! - **never fake playing**: no muted-autoplay trick forces progress
//!   while claiming playback. On this host the park is structural:
//!   the audio lane stops rendering (a suspended context pulls
//!   nothing — the same rule as pause), so the sample-counter clock
//!   freezes and media time does not advance.
//!
//! # The gate's mechanics
//!
//! The gate owns two bits — *parked* (the context is suspended) and
//! *surfaced* (the `GestureRequired` event has been emitted for this
//! parking cycle) — and answers [`GateChange`] instructions:
//!
//! - [`AutoplayGate::report`] applies the page's context-state
//!   report ([`crate::protocol::HostToWorker::AudioContext`]);
//! - [`AutoplayGate::observe`] is the host's pump/play-time check:
//!   parked + playback wanting audio + not yet surfaced ⇒
//!   [`GateChange::Parked`] — emit the typed
//!   [`crate::protocol::WorkerToHost::GestureRequired`] event
//!   exactly once per cycle (never spam).
//!
//! A report that arrives while nothing wants to flow parks silently
//! (the gate is armed; the first observation that finds playback
//! wanting audio surfaces the event then). The gate does not know
//! about the command layer's event catalog — the
//! `playback.gesture-required` catalog row is the Lead's additive
//! decision (doc 09 §12 OQ-3, `[PROPOSAL]`); the transport-level
//! event carries the same condition without editing the shared
//! catalog.

/// The autoplay gate: parked/surfaced state plus the report and
/// observation transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoplayGate {
    /// The context is suspended (audio cannot flow).
    parked: bool,
    /// The `GestureRequired` event was emitted for this parking
    /// cycle (the once-per-cycle latch).
    surfaced: bool,
}

/// What one gate transition instructs the host to surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateChange {
    /// Nothing to surface.
    None,
    /// Emit the typed `GestureRequired` event and park the audio
    /// lane (the *paused-pending-gesture* condition begins).
    Parked,
    /// The gate cleared: playback may truly flow (the page drove the
    /// gesture; audio flows, the state says so — the truth rule).
    Resumed,
}

impl AutoplayGate {
    /// An open gate (the default — the headless lane's permanent
    /// state: no `AudioContext` exists to suspend).
    #[must_use]
    pub fn new() -> AutoplayGate {
        AutoplayGate {
            parked: false,
            surfaced: false,
        }
    }

    /// Whether playback is parked pending a gesture (the audio lane
    /// must not render — the clock freezes, exactly like pause).
    #[must_use]
    pub fn is_parked(&self) -> bool {
        self.parked
    }

    /// Apply the page's context-state report. A fresh suspension
    /// arms the gate; a resume after a park clears it (and re-arms
    /// the once-per-cycle event latch for the next cycle).
    pub fn report(&mut self, running: bool) -> GateChange {
        if running {
            if self.parked {
                self.parked = false;
                self.surfaced = false;
                return GateChange::Resumed;
            }
            return GateChange::None;
        }
        if !self.parked {
            // A fresh suspension: arm, and re-arm the latch (an
            // already-parked gate keeps its surfaced bit — duplicate
            // reports must not re-fire the event).
            self.parked = true;
            self.surfaced = false;
        }
        GateChange::None
    }

    /// The pump/play-time observation: parked + playback wanting
    /// audio + not yet surfaced ⇒ surface the event once. This is
    /// the host's per-slice check, so the event fires whenever the
    /// two conditions FIRST hold together — report-then-play and
    /// play-then-report orderings both surface exactly one event.
    pub fn observe(&mut self, wants_audio: bool) -> GateChange {
        if self.parked && wants_audio && !self.surfaced {
            self.surfaced = true;
            GateChange::Parked
        } else {
            GateChange::None
        }
    }
}

impl Default for AutoplayGate {
    fn default() -> Self {
        AutoplayGate::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The report matrix: a fresh suspension arms, duplicates are
    /// idempotent, resume clears, re-suspension re-arms.
    #[test]
    fn suspension_cycles_arm_and_clear() {
        let mut gate = AutoplayGate::new();
        assert!(!gate.is_parked());
        assert_eq!(gate.report(false), GateChange::None, "arming is silent");
        assert!(gate.is_parked());
        // Duplicate suspended reports: still parked, no transition.
        assert_eq!(gate.report(false), GateChange::None);
        assert!(gate.is_parked());
        // Resume.
        assert_eq!(gate.report(true), GateChange::Resumed);
        assert!(!gate.is_parked());
        // Running reports while open: nothing.
        assert_eq!(gate.report(true), GateChange::None);
        // A new cycle re-arms.
        assert_eq!(gate.report(false), GateChange::None);
        assert!(gate.is_parked());
        assert_eq!(gate.report(true), GateChange::Resumed);
    }

    /// The observation matrix: parked + wanting ⇒ surface ONCE;
    /// duplicate observations never spam; the next cycle surfaces
    /// again; an open gate never surfaces.
    #[test]
    fn observation_surfaces_once_per_cycle() {
        let mut gate = AutoplayGate::new();
        assert_eq!(gate.observe(true), GateChange::None, "open gate");
        gate.report(false);
        // Parked + wanting: the event, exactly once.
        assert_eq!(gate.observe(true), GateChange::Parked);
        assert_eq!(gate.observe(true), GateChange::None, "once per cycle");
        // Parked but not wanting (e.g. paused): nothing either way.
        assert_eq!(gate.observe(false), GateChange::None);
        assert_eq!(gate.observe(true), GateChange::None);
        // Resume + a fresh cycle: surfaces again.
        gate.report(true);
        gate.report(false);
        assert_eq!(gate.observe(true), GateChange::Parked);
    }

    /// Both orderings surface exactly one event: report-then-play
    /// and play-then-report (the observation is the join point).
    #[test]
    fn both_arrival_orders_surface_once() {
        // Order 1: suspended report arrives, then playback starts.
        let mut gate = AutoplayGate::new();
        gate.report(false);
        assert_eq!(gate.observe(true), GateChange::Parked);
        assert_eq!(gate.observe(true), GateChange::None);
        // Order 2: playback wanting, then the suspended report.
        let mut gate = AutoplayGate::new();
        assert_eq!(gate.observe(true), GateChange::None, "open gate");
        gate.report(false);
        assert_eq!(gate.observe(true), GateChange::Parked);
        assert_eq!(gate.observe(true), GateChange::None);
    }

    /// The default gate is open (the headless lane's truth).
    #[test]
    fn the_default_gate_is_open() {
        let gate = AutoplayGate::default();
        assert!(!gate.is_parked());
        assert_eq!(gate, AutoplayGate::new());
    }
}
