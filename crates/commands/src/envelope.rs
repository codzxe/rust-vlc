//! The §1.6 envelope: ids, request, response, outcome (doc 05).
//!
//! # Wire forms (card 1.D4, doc 05 §5.1)
//!
//! Every type here derives `Serialize`/`Deserialize`/`JsonSchema` with
//! `rename_all = "camelCase"`, per the §1.6 sketch comment. Two details
//! the 1.D3 worklog pre-recorded for this card: the Rust field
//! `generation` keeps its keyword-safe name but serializes as `gen`
//! (§1.6 sketch, §5.1 examples), and the `result` field of
//! [`CommandResponse`] serializes a success as the bare outcome object
//! (`{"accepted": …}` / `{"completed": …}`, §5.1 example) and a failure
//! as `{"error": …}` — std's `Result` shape (`{"ok":…}`) is NOT the
//! documented wire form, hence the [`result_wire`] adapter.

use crate::error::CommandError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Monotonic per host connection (doc 05 §4.1): assigned by the sender,
/// starting at 1, strictly increasing; reuse is forbidden. The engine
/// echoes it and never interprets it (the control server's anti-replay
/// check is that host's business).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct RequestId(pub u64);

/// Engine-assigned session handle (doc 05 §4.2): `SessionId` ↔ the
/// engine's session epoch (doc 03 §5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct SessionId(pub u64);

/// Per-session pipeline generation (doc 05 §4.2): bumped atomically at
/// accept time of generation-bumping commands and engine-initiated
/// invalidations. `Generation` ↔ the engine's pipeline generation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct Generation(pub u64);

/// One host → engine request (doc 05 §1.6). Generic over the command
/// payload `C` — card 1.D4 instantiates `CommandRequest<Command>` with
/// the catalog enum; the envelope semantics here are payload-blind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest<C> {
    pub request_id: RequestId,
    /// Flattened: the concrete `Command` (card 1.D4) is adjacently
    /// tagged (`cmd`/`args`), so the wire object is
    /// `{ "requestId": 42, "cmd": "playback.seek", "args": { … } }`
    /// — exactly the §5.1 example.
    #[serde(flatten)]
    pub command: C,
}

/// The truthful-UI outcome split (doc 05 §1.4 — hard contract):
/// `Accepted` is "validated and queued/started", `Completed` is "the
/// engine confirms it happened (in this response)".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandOutcome<R> {
    /// Class A: queued/started. `generation` is the generation under which the
    /// completion event will arrive — the NEW generation for generation-bumping
    /// commands, the current generation otherwise (§4.2).
    #[serde(rename_all = "camelCase")]
    Accepted {
        session: Option<SessionId>,
        #[serde(rename = "gen")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(bound(serialize = "R: Serialize"))]
#[serde(bound(deserialize = "R: Deserialize<'de>"))]
pub struct CommandResponse<R> {
    pub request_id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    #[serde(rename = "gen")]
    pub generation: Generation,
    #[serde(with = "result_wire")]
    #[schemars(with = "result_wire::ResultWire<CommandOutcome<R>, CommandError>")]
    pub result: Result<CommandOutcome<R>, CommandError>,
}

/// One engine → host event (doc 05 §1.6/§3.1). Generic over the event
/// payload `E` until the catalog lands. `seq` is engine-global and
/// strictly increasing; `generation` is present only for pipeline-scoped
/// events (the staleness rule applies exactly to those); `origin` names
/// the request that caused the event when attributable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Event<E> {
    pub seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    #[serde(rename = "gen", skip_serializing_if = "Option::is_none")]
    pub generation: Option<Generation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<RequestId>,
    /// Flattened: the concrete `EventKind` (card 1.D4) is internally
    /// tagged (`kind`), so payload fields sit beside the envelope fields
    /// — `{ "seq": …, "kind": "timing.seek-completed", … }` (§5.1).
    #[serde(flatten)]
    pub kind: E,
}

impl<E> Event<E> {
    /// Whether the event is pipeline-scoped (subject to the §4.2
    /// staleness rule): it must carry BOTH a session and a generation.
    pub fn is_pipeline_scoped(&self) -> bool {
        self.session.is_some() && self.generation.is_some()
    }
}

/// Serde/schemars adapter for the `result` field (§5.1 wire form): a
/// success serializes as the bare [`CommandOutcome`] object, a failure
/// as `{ "error": <CommandError> }`. Std's `Result` (`{"ok":…}`) is
/// not the documented shape, so the field carries `#[serde(with =
/// "result_wire")]` and the schema mirrors the same union via
/// `#[schemars(with = …)]` — the emitted JSON Schema and the runtime
/// wire form cannot drift apart.
pub mod result_wire {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde::Serialize;

    /// The on-the-wire union of the two response shapes.
    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(untagged)]
    pub enum ResultWire<O, E> {
        /// Success: the bare outcome object (`accepted`/`completed`).
        Ok(O),
        /// Failure: `{ "error": … }`.
        Err(ErrorEnvelope<E>),
    }

    /// The failure wrapper.
    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct ErrorEnvelope<E> {
        pub error: E,
    }

    /// `#[serde(with = "result_wire")]` — serialize half.
    ///
    /// # Errors
    /// Forwards the underlying serializer's error.
    pub fn serialize<S, O, E>(result: &Result<O, E>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        O: Serialize,
        E: Serialize,
    {
        match result {
            Ok(outcome) => ResultWire::<&O, &E>::Ok(outcome).serialize(serializer),
            Err(error) => ResultWire::<&O, &E>::Err(ErrorEnvelope { error }).serialize(serializer),
        }
    }

    /// `#[serde(with = "result_wire")]` — deserialize half.
    ///
    /// # Errors
    /// Forwards the underlying deserializer's error (including the
    /// untagged-union error when the value is neither shape).
    pub fn deserialize<'de, D, O, E>(deserializer: D) -> Result<Result<O, E>, D::Error>
    where
        D: serde::Deserializer<'de>,
        O: Deserialize<'de>,
        E: Deserialize<'de>,
    {
        match ResultWire::<O, E>::deserialize(deserializer)? {
            ResultWire::Ok(outcome) => Ok(Ok(outcome)),
            ResultWire::Err(envelope) => Ok(Err(envelope.error)),
        }
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
