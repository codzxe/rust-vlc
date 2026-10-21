//! The desktop host seam: the command-layer correlation half of the
//! composition root (card 2.D3; doc 01 §2.7 "serves the `ui/` build
//! through Tauri IPC mapped 1:1 onto `commands`"; doc 05 §4).
//!
//! What CAN be built headlessly is exactly this half: request-id
//! allocation, the §4.2 staleness filter, the §4.3 delivery-log
//! validation, and the generation-bump signal that the wiring turns
//! into a frame-mailbox flush (`mailbox.rs`). The other half — the
//! window, the webview, the IPC channel itself — is the typed
//! [`HostError::WebviewPending`] state behind ADR-004 (the 2.C3
//! `UnsupportedBackend` precedent: never fake a window).
//!
//! Also here: ADR-003's **truthful capability reporting** as data —
//! [`strategy_for`] is the accepted per-OS landing (macOS A;
//! Windows A-or-B pending the prototype; Linux B) with the honest
//! status line for each, so the UI and the feature matrix can quote
//! the decision without re-implementing it.

use commands::envelope::{CommandResponse, Event};
use commands::{
    DeliveryLog, Generation, RequestId, RequestIdAllocator, SessionId, StalenessFilter,
};

use crate::error::HostError;

/// The ADR-003 per-OS presentation landing, as queryable data
/// (decision 4: the active path is an engine fact, never marketing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationVariant {
    /// Candidate A: native wgpu surface behind a transparent
    /// webview.
    NativeSurface,
    /// Candidate B: frame streaming to a canvas (this crate's wire
    /// protocol + the reference JS).
    CanvasStream,
    /// ADR-003 leaves Windows as "A-or-B (prototype decides)" — the
    /// 2.D3 per-OS matrix is the decider, and its cells are pending
    /// a desktop environment (`docs/prototypes/video-in-webview.md`
    /// §5).
    PrototypePending,
}

impl PresentationVariant {
    /// The one-line ADR-003 label.
    #[must_use = "pure reporter"]
    pub fn label(self) -> &'static str {
        match self {
            PresentationVariant::NativeSurface => "A (native wgpu surface)",
            PresentationVariant::CanvasStream => "B (canvas frame streaming)",
            PresentationVariant::PrototypePending => "A-or-B (prototype decides)",
        }
    }
}

/// The desktop operating systems ADR-003 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOs {
    /// Windows / WebView2.
    Windows,
    /// macOS / WKWebView.
    Macos,
    /// Linux / WebKitGTK.
    Linux,
}

impl TargetOs {
    /// The webview engine ADR-003's evidence covers.
    #[must_use = "pure reporter"]
    pub fn webview(self) -> &'static str {
        match self {
            TargetOs::Windows => "WebView2",
            TargetOs::Macos => "WKWebView",
            TargetOs::Linux => "WebKitGTK 4.1",
        }
    }
}

/// One row of the ADR-003 landing table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategyReport {
    /// The OS.
    pub os: TargetOs,
    /// The variant ADR-003 accepts for it.
    pub variant: PresentationVariant,
    /// The honest status of that landing *in this build*.
    pub status: &'static str,
}

/// The ADR-003 per-OS landing (accepted 2026-10-19; per-OS final
/// confirmation gated on the Phase 2 prototypes — this card's matrix
/// is that gate's evidence surface).
#[must_use = "pure data: the ADR as shipped"]
pub fn strategy_for(os: TargetOs) -> StrategyReport {
    let (variant, status) = match os {
        TargetOs::Macos => (
            PresentationVariant::NativeSurface,
            "accepted per ADR-003 (sibling NSView; Tauri >= 2.12.1 drawsBackground fix); unmeasured in this sandbox",
        ),
        TargetOs::Windows => (
            PresentationVariant::PrototypePending,
            "ADR-003 leaves A-or-B to the prototype; matrix cells pending a desktop run",
        ),
        TargetOs::Linux => (
            PresentationVariant::CanvasStream,
            "accepted per ADR-003 (B: no Wayland child-webview path for A; WebCodecs compiled into Debian's 2.52.6); unmeasured in this sandbox",
        ),
    };
    StrategyReport {
        os,
        variant,
        status,
    }
}

/// What observing a response told the host (the wiring's side of the
/// §4.2 contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseObservation {
    /// Whether the §4.3 log accepted the response (echo validity).
    pub log_ok: bool,
    /// A generation raise on a pipeline-scoped session — the signal
    /// the wiring turns into a frame-mailbox flush (`flush_stale`):
    /// frames in flight under the old generation are stale by
    /// construction.
    pub generation_bump: Option<(SessionId, Generation)>,
}

/// What delivering an event decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDelivery {
    /// Keep: current (or not pipeline-scoped).
    Kept,
    /// Drop: stale under the §4.2 watermark (rule C6 — stale
    /// evidence is not evidence).
    DroppedStale,
}

/// The desktop host's correlation state (doc 05 §4, host-side half).
#[derive(Debug, Default)]
pub struct DesktopHost {
    ids: RequestIdAllocator,
    staleness: StalenessFilter,
    log: DeliveryLog,
}

impl DesktopHost {
    /// A fresh host (empty correlation state).
    #[must_use = "pure constructor"]
    pub fn new() -> DesktopHost {
        DesktopHost {
            ids: RequestIdAllocator::new(),
            staleness: StalenessFilter::new(),
            log: DeliveryLog::new(),
        }
    }

    /// Allocate the next request id (monotonic from 1; the §4.3 log
    /// learns it, so responses to it are legal).
    pub fn next_request_id(&mut self) -> RequestId {
        let id = self.ids.allocate();
        self.log.note_allocated(id);
        id
    }

    /// Observe a response in delivery order: echo validity (§4.3) +
    /// watermark raise (§4.2) + the generation-bump signal.
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) -> ResponseObservation {
        let before = response
            .session
            .and_then(|session| self.staleness.watermark(session));
        self.staleness.observe_response(response);
        let after = response
            .session
            .and_then(|session| self.staleness.watermark(session));
        let generation_bump = match (response.session, before, after) {
            (Some(session), Some(before), Some(after)) if after > before => Some((session, after)),
            (Some(session), None, Some(after)) => Some((session, after)),
            _ => None,
        };
        let log_ok = self.log.observe_response(response);
        ResponseObservation {
            log_ok,
            generation_bump,
        }
    }

    /// Deliver an event: the §4.2 staleness rule, then the §4.3 seq
    /// check. A stale event is dropped before the log sees it (its
    /// `seq` consumed a slot either way — the engine's seq is
    /// global, not per-host).
    pub fn deliver_event<E>(&mut self, event: &Event<E>) -> EventDelivery {
        if !self.staleness.filter_event(event) {
            return EventDelivery::DroppedStale;
        }
        // The seq check records violations but does not gate delivery
        // (§4.3 is a validator: the host surfaces the violation, the
        // conformance suite fails on it — the UI still sees the
        // event).
        let _ = self.log.observe_event(event);
        EventDelivery::Kept
    }

    /// The recorded §4.3 violations (diagnostics).
    #[must_use = "observable evidence"]
    pub fn violations(&self) -> &[String] {
        self.log.violations()
    }

    /// The honest one-line capability report for this build (ADR-003
    /// decision 4: what is *actually* in use).
    #[must_use = "pure reporter"]
    pub fn active_presentation_path() -> &'static str {
        "skeleton: canvas-stream protocol core only — no webview host compiled (ADR-004 pending; docs/prototypes/video-in-webview.md)"
    }

    /// The typed pending state for the webview half (the honest
    /// answer to "open a window here").
    ///
    /// # Errors
    /// [`HostError::WebviewPending`] — always, in this build; the
    /// 2.C3 precedent: enumerate nothing, never fake success.
    pub fn open_webview(&self) -> Result<(), HostError> {
        Err(HostError::WebviewPending { adr: "ADR-004" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::envelope::{CommandOutcome, CommandRequest, CommandResponse, Event};
    use testkit::Must;

    // Minimal payload types (the envelope is generic over the
    // catalog; the host logic under test is payload-independent).
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeCommand(u32);
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeResult(u32);
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeEvent(u32);

    fn accepted_response(
        id: RequestId,
        session: SessionId,
        generation: Generation,
    ) -> CommandResponse<FakeResult> {
        CommandResponse {
            request_id: id,
            session: Some(session),
            generation,
            result: Ok(CommandOutcome::Accepted {
                session: Some(session),
                generation,
            }),
        }
    }

    fn pipeline_event(seq: u64, session: SessionId, generation: Generation) -> Event<FakeEvent> {
        Event {
            seq,
            session: Some(session),
            generation: Some(generation),
            origin: None,
            kind: FakeEvent(u32::try_from(seq).unwrap_or(u32::MAX)),
        }
    }

    #[test]
    fn ids_are_monotonic_and_responses_echo_legally() {
        let mut host = DesktopHost::new();
        let a = host.next_request_id();
        let b = host.next_request_id();
        assert_eq!(a.0 + 1, b.0);
        let obs = host.observe_response(&accepted_response(b, SessionId(7), Generation(1)));
        assert!(obs.log_ok);
        assert!(host.violations().is_empty());
        // A response for an id this host never allocated is a §4.3
        // violation (echo validity).
        let ghost = CommandResponse::<FakeResult> {
            request_id: RequestId(99),
            session: Some(SessionId(7)),
            generation: Generation(1),
            result: Ok(CommandOutcome::Completed(FakeResult(0))),
        };
        let obs = host.observe_response(&ghost);
        assert!(!obs.log_ok);
        assert_eq!(host.violations().len(), 1);
        assert!(host.violations()[0].contains("99"));
    }

    #[test]
    fn generation_bumps_signal_the_mailbox_flush() {
        let mut host = DesktopHost::new();
        let session = SessionId(3);
        let id = host.next_request_id();
        // First observation: the watermark appears (a bump from
        // nothing — the wiring flushes on it too: the session's
        // in-flight frames are from before the first response).
        let obs = host.observe_response(&accepted_response(id, session, Generation(2)));
        assert_eq!(obs.generation_bump, Some((session, Generation(2))));
        // Same generation again: no bump.
        let id2 = host.next_request_id();
        let obs = host.observe_response(&accepted_response(id2, session, Generation(2)));
        assert_eq!(obs.generation_bump, None);
        // A raise (seek/stop/track-switch class-A command): the
        // flush signal.
        let id3 = host.next_request_id();
        let obs = host.observe_response(&accepted_response(id3, session, Generation(5)));
        assert_eq!(obs.generation_bump, Some((session, Generation(5))));
    }

    #[test]
    fn stale_events_are_dropped_current_ones_kept() {
        let mut host = DesktopHost::new();
        let session = SessionId(1);
        let id = host.next_request_id();
        host.observe_response(&accepted_response(id, session, Generation(4)));
        // Stale (C6): dropped.
        assert_eq!(
            host.deliver_event(&pipeline_event(1, session, Generation(3))),
            EventDelivery::DroppedStale
        );
        // Current: kept.
        assert_eq!(
            host.deliver_event(&pipeline_event(2, session, Generation(4))),
            EventDelivery::Kept
        );
        // Newer: kept (and raises the watermark).
        assert_eq!(
            host.deliver_event(&pipeline_event(3, session, Generation(6))),
            EventDelivery::Kept
        );
        // Below the watermark now: stale (C6 is strictly-lower, so
        // equal generations stay current — asserted above at gen 4).
        assert_eq!(
            host.deliver_event(&pipeline_event(4, session, Generation(5))),
            EventDelivery::DroppedStale
        );
        // Non-monotonic seq is recorded, delivery continues.
        assert_eq!(
            host.deliver_event(&pipeline_event(3, session, Generation(7))),
            EventDelivery::Kept
        );
        assert!(
            host.violations()
                .iter()
                .any(|v| v.contains("non-monotonic seq"))
        );
    }

    #[test]
    fn strategy_table_is_the_adr_landing() {
        assert_eq!(
            strategy_for(TargetOs::Macos).variant,
            PresentationVariant::NativeSurface
        );
        assert_eq!(
            strategy_for(TargetOs::Windows).variant,
            PresentationVariant::PrototypePending
        );
        assert_eq!(
            strategy_for(TargetOs::Linux).variant,
            PresentationVariant::CanvasStream
        );
        assert_eq!(TargetOs::Windows.webview(), "WebView2");
        assert_eq!(TargetOs::Macos.webview(), "WKWebView");
        assert_eq!(TargetOs::Linux.webview(), "WebKitGTK 4.1");
        assert_eq!(
            PresentationVariant::CanvasStream.label(),
            "B (canvas frame streaming)"
        );
        // Every status line is honest about the sandbox.
        for os in [TargetOs::Windows, TargetOs::Macos, TargetOs::Linux] {
            let report = strategy_for(os);
            assert!(!report.status.is_empty());
        }
    }

    #[test]
    fn webview_open_is_a_typed_pending_never_a_fake() {
        let mut host = DesktopHost::new();
        let err = host.open_webview().err().must();
        assert!(matches!(err, HostError::WebviewPending { adr: "ADR-004" }));
        assert!(err.to_string().contains("ADR-004"));
        assert!(DesktopHost::active_presentation_path().contains("ADR-004"));
        // And the envelope round-trips through the generic host
        // without knowing the catalog (the 1:1 mapping seam).
        let id = host.next_request_id();
        let _request = CommandRequest {
            request_id: id,
            command: FakeCommand(1),
        };
        let response = accepted_response(id, SessionId(9), Generation(1));
        assert!(host.observe_response(&response).log_ok);
    }
}
