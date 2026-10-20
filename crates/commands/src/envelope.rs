//! The §1.6 envelope: ids, request, response, outcome (doc 05).

use crate::error::CommandError;

/// Monotonic per host connection (doc 05 §4.1): assigned by the sender,
/// starting at 1, strictly increasing; reuse is forbidden. The engine
/// echoes it and never interprets it (the control server's anti-replay
/// check is that host's business).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(pub u64);

/// Engine-assigned session handle (doc 05 §4.2): `SessionId` ↔ the
/// engine's session epoch (doc 03 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);

/// Per-session pipeline generation (doc 05 §4.2): bumped atomically at
/// accept time of generation-bumping commands and engine-initiated
/// invalidations. `Generation` ↔ the engine's pipeline generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Generation(pub u64);

/// One host → engine request (doc 05 §1.6). Generic over the command
/// payload `C` — card 1.D4 instantiates `CommandRequest<Command>` with
/// the catalog enum; the envelope semantics here are payload-blind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest<C> {
    pub request_id: RequestId,
    pub command: C,
}

/// The truthful-UI outcome split (doc 05 §1.4 — hard contract):
/// `Accepted` is "validated and queued/started", `Completed` is "the
/// engine confirms it happened (in this response)".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome<R> {
    /// Class A: queued/started. `generation` is the generation under which the
    /// completion event will arrive — the NEW generation for generation-bumping
    /// commands, the current generation otherwise (§4.2).
    Accepted {
        session: Option<SessionId>,
        generation: Generation,
    },
    /// Class S: the engine confirms completion in this response; no
    /// event is required (rule C2).
    Completed(R),
}

impl<R> CommandOutcome<R> {
    /// The generation an `Accepted` outcome speaks for (§4.2: hosts
    /// track the highest observed generation per session from both outcomes
    /// and events). `None` for `Completed` — the RESPONSE envelope's
    /// `generation` field is authoritative there.
    pub fn accepted_generation(&self) -> Option<Generation> {
        match self {
            CommandOutcome::Accepted { generation, .. } => Some(*generation),
            CommandOutcome::Completed(_) => None,
        }
    }

    /// Whether the host must wait for a completion event (class A).
    pub fn requires_completion_event(&self) -> bool {
        matches!(self, CommandOutcome::Accepted { .. })
    }
}

/// One engine → host response (doc 05 §1.6). `request_id` is echoed,
/// always; `session`/`generation` are the context at handling time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResponse<R> {
    pub request_id: RequestId,
    pub session: Option<SessionId>,
    pub generation: Generation,
    pub result: Result<CommandOutcome<R>, CommandError>,
}

/// One engine → host event (doc 05 §1.6/§3.1). Generic over the event
/// payload `E` until the catalog lands. `seq` is engine-global and
/// strictly increasing; `generation` is present only for pipeline-scoped
/// events (the staleness rule applies exactly to those); `origin` names
/// the request that caused the event when attributable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<E> {
    pub seq: u64,
    pub session: Option<SessionId>,
    pub generation: Option<Generation>,
    pub origin: Option<RequestId>,
    pub kind: E,
}

impl<E> Event<E> {
    /// Whether the event is pipeline-scoped (subject to the §4.2
    /// staleness rule): it must carry BOTH a session and a generation.
    pub fn is_pipeline_scoped(&self) -> bool {
        self.session.is_some() && self.generation.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal payloads for envelope-shape tests.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeResult(u32);
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeEvent(&'static str);

    #[test]
    fn ids_are_plain_newtypes() {
        assert_eq!(RequestId(1), RequestId(1));
        assert!(RequestId(2) > RequestId(1));
        assert_eq!(SessionId(7).0, 7);
        assert_eq!(Generation(3).0, 3);
    }

    #[test]
    fn accepted_exposes_its_generation() {
        let accepted: CommandOutcome<FakeResult> = CommandOutcome::Accepted {
            session: Some(SessionId(1)),
            generation: Generation(4),
        };
        assert_eq!(accepted.accepted_generation(), Some(Generation(4)));
        assert!(accepted.requires_completion_event(), "class A waits");
        let completed: CommandOutcome<FakeResult> = CommandOutcome::Completed(FakeResult(9));
        assert_eq!(completed.accepted_generation(), None);
        assert!(
            !completed.requires_completion_event(),
            "class S is its own proof"
        );
    }

    #[test]
    fn pipeline_scoping_requires_session_and_gen() {
        let scoped = Event {
            seq: 1,
            session: Some(SessionId(1)),
            generation: Some(Generation(2)),
            origin: None,
            kind: FakeEvent("state-changed"),
        };
        assert!(scoped.is_pipeline_scoped());
        let sessionless = Event {
            seq: 2,
            session: None,
            generation: Some(Generation(2)),
            origin: None,
            kind: FakeEvent("global"),
        };
        assert!(!sessionless.is_pipeline_scoped());
        let genless = Event {
            seq: 3,
            session: Some(SessionId(1)),
            generation: None,
            origin: None,
            kind: FakeEvent("session-lifetime"),
        };
        assert!(!genless.is_pipeline_scoped());
    }

    #[test]
    fn response_shape_carries_all_correlation_fields() {
        let resp: CommandResponse<FakeResult> = CommandResponse {
            request_id: RequestId(42),
            session: Some(SessionId(7)),
            generation: Generation(3),
            result: Ok(CommandOutcome::Accepted {
                session: Some(SessionId(7)),
                generation: Generation(4),
            }),
        };
        assert_eq!(resp.request_id, RequestId(42));
        assert_eq!(resp.session, Some(SessionId(7)));
        assert_eq!(
            resp.result.ok().and_then(|o| o.accepted_generation()),
            Some(Generation(4)),
            "Accepted carries the NEW generation for generation-bumping commands (§4.2)"
        );
    }
}
