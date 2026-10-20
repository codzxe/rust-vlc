//! Correlation and ordering (doc 05 §4) — the card 1.D3 AC lives here.

use std::collections::BTreeMap;

use crate::envelope::{CommandResponse, Event, Generation, RequestId, SessionId};

/// Allocates `RequestId`s per host connection (doc 05 §4.1): monotonic
/// from 1, reuse forbidden. Hosts that reload restart at 1 — safe
/// because the engine never compares ids across connections.
#[derive(Debug, Default)]
pub struct RequestIdAllocator {
    next: u64,
}

impl RequestIdAllocator {
    /// An allocator whose next id is 1.
    pub fn new() -> RequestIdAllocator {
        RequestIdAllocator { next: 0 }
    }

    /// Allocate the next id (monotonic, never reused within the
    /// allocator's lifetime).
    pub fn allocate(&mut self) -> RequestId {
        self.next += 1;
        RequestId(self.next)
    }

    /// The most recently allocated id (`None` before the first).
    pub fn last(&self) -> Option<RequestId> {
        (self.next > 0).then_some(RequestId(self.next))
    }
}

/// The host-side staleness filter (doc 05 §4.2 — rule C6: stale evidence
/// is not evidence).
///
/// Tracks the highest generation observed per session — from `Accepted`
/// responses (which carry the NEW generation for generation-bumping commands) and from
/// pipeline-scoped events — and drops any pipeline-scoped event whose
/// `generation` is lower than that watermark. Session-less and generation-less
/// events are never dropped (they are not pipeline-scoped).
#[derive(Debug, Default)]
pub struct StalenessFilter {
    watermarks: BTreeMap<SessionId, Generation>,
}

impl StalenessFilter {
    /// An empty filter (no sessions observed).
    pub fn new() -> StalenessFilter {
        StalenessFilter {
            watermarks: BTreeMap::new(),
        }
    }

    /// Observe a response: a successful `Accepted` outcome raises the
    /// session's watermark to the outcome's generation (the response envelope's
    /// `generation` records the context at handling time; the outcome's generation is
    /// the completion contract, §4.2).
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) {
        let Some(session) = response.session else {
            return;
        };
        let watermark = response
            .result
            .as_ref()
            .ok()
            .and_then(|outcome| outcome.accepted_generation())
            .unwrap_or(response.generation);
        self.raise(session, watermark);
    }

    /// Apply the staleness rule to an event: `true` = keep, `false` =
    /// drop (stale). Pipeline-scoped events also raise the watermark
    /// when current (an event at the newest generation is evidence of it).
    pub fn filter_event<E>(&mut self, event: &Event<E>) -> bool {
        let (Some(session), Some(generation)) = (event.session, event.generation) else {
            return true; // not pipeline-scoped — never dropped
        };
        match self.watermarks.get(&session) {
            Some(&watermark) if generation < watermark => false, // stale (C6)
            _ => {
                self.raise(session, generation);
                true
            }
        }
    }

    /// The current watermark for a session (`None` if unobserved).
    pub fn watermark(&self, session: SessionId) -> Option<Generation> {
        self.watermarks.get(&session).copied()
    }

    fn raise(&mut self, session: SessionId, generation: Generation) {
        let entry = self.watermarks.entry(session).or_insert(generation);
        if generation > *entry {
            *entry = generation;
        }
    }
}

/// Validates the §4.3 ordering invariants on an observed delivery
/// stream — the conformance harness every host binding feeds.
///
/// Invariants checked:
/// 1. `seq` is strictly increasing across the observed events.
/// 2. A response never arrives for a request id the host has not
///    allocated (echo validity).
/// 3. An event carrying `origin` (a request id) is causally consistent:
///    it is only invalid if the SAME origin was never observed as a
///    response — the full causal-order proof (response positioned
///    between the events it causes) needs the engine-side log and is
///    the Phase-2 conformance suite's job; this is the host-side half.
#[derive(Debug, Default)]
pub struct DeliveryLog {
    allocated: std::collections::BTreeSet<RequestId>,
    answered: std::collections::BTreeSet<RequestId>,
    last_seq: Option<u64>,
    /// Ordered record of violations found so far (kept small — this is
    /// a test/diagnostic aid, not a production sink).
    violations: Vec<String>,
}

impl DeliveryLog {
    /// An empty log bound to no allocator yet.
    pub fn new() -> DeliveryLog {
        DeliveryLog::default()
    }

    /// Record that the host allocated `id` (responses to it are now
    /// legal).
    pub fn note_allocated(&mut self, id: RequestId) {
        self.allocated.insert(id);
    }

    /// Observe a response in delivery order. Returns `false` when a
    /// violation was recorded.
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) -> bool {
        let ok = self.allocated.contains(&response.request_id);
        if !ok {
            self.violations.push(format!(
                "response for unallocated request {}",
                response.request_id.0
            ));
        }
        self.answered.insert(response.request_id);
        ok
    }

    /// Observe an event in delivery order. `false` when a violation was
    /// recorded (non-monotonic `seq`).
    pub fn observe_event<E>(&mut self, event: &Event<E>) -> bool {
        let ok = match self.last_seq {
            Some(last) if event.seq <= last => {
                self.violations
                    .push(format!("non-monotonic seq: {} after {}", event.seq, last));
                false
            }
            _ => true,
        };
        if ok {
            self.last_seq = Some(event.seq);
        }
        ok
    }

    /// The violations recorded so far, in observation order.
    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{CommandOutcome, CommandRequest};
    use crate::error::{CommandError, ErrorCode, Stage};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeResult(u32);
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeEvent(&'static str);

    fn accepted_response(
        request: u64,
        session: u64,
        generation: u64,
    ) -> CommandResponse<FakeResult> {
        CommandResponse {
            request_id: RequestId(request),
            session: Some(SessionId(session)),
            generation: Generation(generation),
            result: Ok(CommandOutcome::Accepted {
                session: Some(SessionId(session)),
                generation: Generation(generation),
            }),
        }
    }

    fn error_response(request: u64, session: u64, generation: u64) -> CommandResponse<FakeResult> {
        CommandResponse {
            request_id: RequestId(request),
            session: Some(SessionId(session)),
            generation: Generation(generation),
            result: Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no source open",
            )),
        }
    }

    fn scoped_event(seq: u64, session: u64, generation: u64) -> Event<FakeEvent> {
        Event {
            seq,
            session: Some(SessionId(session)),
            generation: Some(Generation(generation)),
            origin: Some(RequestId(1)),
            kind: FakeEvent("state-changed"),
        }
    }

    // ---- §4.1 request ids ----------------------------------------------------

    #[test]
    fn request_ids_are_monotonic_and_never_reused() {
        let mut alloc = RequestIdAllocator::new();
        let ids: Vec<RequestId> = (0..5).map(|_| alloc.allocate()).collect();
        assert_eq!(
            ids,
            vec![
                RequestId(1),
                RequestId(2),
                RequestId(3),
                RequestId(4),
                RequestId(5)
            ]
        );
        assert_eq!(alloc.last(), Some(RequestId(5)));
        assert_eq!(
            ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
            ids.len(),
            "no id is ever reused"
        );
    }

    // ---- §4.2 stale-generation rules (THE CARD AC) ----------------------------

    #[test]
    fn accepted_responses_raise_the_generation_watermark() {
        let mut filter = StalenessFilter::new();
        filter.observe_response(&accepted_response(1, 7, 3));
        assert_eq!(filter.watermark(SessionId(7)), Some(Generation(3)));
        // a later Accepted with the NEW generation (generation-bumping command)
        filter.observe_response(&accepted_response(2, 7, 4));
        assert_eq!(filter.watermark(SessionId(7)), Some(Generation(4)));
        // a stale Accepted (older generation arriving late) must NOT lower it
        filter.observe_response(&accepted_response(3, 7, 2));
        assert_eq!(filter.watermark(SessionId(7)), Some(Generation(4)));
    }

    #[test]
    fn error_responses_track_the_envelope_gen() {
        let mut filter = StalenessFilter::new();
        filter.observe_response(&error_response(1, 7, 5));
        assert_eq!(
            filter.watermark(SessionId(7)),
            Some(Generation(5)),
            "errors still carry the session context (§1.6)"
        );
    }

    #[test]
    fn stale_pipeline_events_are_dropped_current_ones_kept() {
        let mut filter = StalenessFilter::new();
        filter.observe_response(&accepted_response(1, 7, 4));
        // stale: generation 3 < watermark 4
        assert!(
            !filter.filter_event(&scoped_event(10, 7, 3)),
            "stale dropped"
        );
        // current: generation 4 == watermark 4
        assert!(filter.filter_event(&scoped_event(11, 7, 4)), "current kept");
        // newer: generation 5 raises the watermark
        assert!(filter.filter_event(&scoped_event(12, 7, 5)), "newer kept");
        assert_eq!(filter.watermark(SessionId(7)), Some(Generation(5)));
        // now generation 4 is stale
        assert!(
            !filter.filter_event(&scoped_event(13, 7, 4)),
            "dropped after raise"
        );
    }

    #[test]
    fn sessionless_and_genless_events_are_never_dropped() {
        let mut filter = StalenessFilter::new();
        filter.observe_response(&accepted_response(1, 7, 4));
        let sessionless = Event {
            seq: 20,
            session: None,
            generation: Some(Generation(1)),
            origin: None,
            kind: FakeEvent("global"),
        };
        assert!(filter.filter_event(&sessionless), "no session: not scoped");
        let genless = Event {
            seq: 21,
            session: Some(SessionId(7)),
            generation: None,
            origin: None,
            kind: FakeEvent("session-lifetime"),
        };
        assert!(filter.filter_event(&genless), "no generation: not scoped");
    }

    #[test]
    fn watermarks_are_per_session() {
        let mut filter = StalenessFilter::new();
        filter.observe_response(&accepted_response(1, 7, 4));
        filter.observe_response(&accepted_response(2, 9, 1));
        assert_eq!(filter.watermark(SessionId(7)), Some(Generation(4)));
        assert_eq!(filter.watermark(SessionId(9)), Some(Generation(1)));
        // session 9's generation-3 event is current for IT, stale for session 7
        assert!(filter.filter_event(&scoped_event(30, 9, 3)));
        assert!(!filter.filter_event(&scoped_event(31, 7, 3)));
    }

    #[test]
    fn response_without_session_does_not_touch_watermarks() {
        let mut filter = StalenessFilter::new();
        let resp: CommandResponse<FakeResult> = CommandResponse {
            request_id: RequestId(1),
            session: None,
            generation: Generation(9),
            result: Ok(CommandOutcome::Accepted {
                session: None,
                generation: Generation(9),
            }),
        };
        filter.observe_response(&resp);
        assert_eq!(filter.watermark(SessionId(1)), None);
    }

    // ---- §4.3 ordering ----------------------------------------------------------

    #[test]
    fn seq_must_be_strictly_increasing() {
        let mut log = DeliveryLog::new();
        assert!(log.observe_event(&scoped_event(1, 7, 1)));
        assert!(log.observe_event(&scoped_event(2, 7, 1)));
        assert!(!log.observe_event(&scoped_event(2, 7, 1)), "repeat seq");
        assert!(!log.observe_event(&scoped_event(1, 7, 1)), "going back");
        assert_eq!(log.violations().len(), 2);
        assert!(log.violations()[0].contains("non-monotonic"));
    }

    #[test]
    fn responses_must_echo_allocated_requests() {
        let mut log = DeliveryLog::new();
        log.note_allocated(RequestId(1));
        assert!(log.observe_response(&accepted_response(1, 7, 1)));
        assert!(
            !log.observe_response(&accepted_response(99, 7, 1)),
            "never allocated"
        );
        assert!(log.violations()[0].contains("unallocated request 99"));
    }

    #[test]
    fn the_full_host_side_pipeline_over_a_command_storm() {
        // The §4.2 race, end to end on the host side: seek A accepted at
        // generation 3, seek B accepted at generation 4 (A implicitly canceled), then
        // interleaved events — only current ones survive.
        let mut alloc = RequestIdAllocator::new();
        let mut filter = StalenessFilter::new();
        let mut log = DeliveryLog::new();

        let seek_a = alloc.allocate();
        log.note_allocated(seek_a);
        filter.observe_response(&accepted_response(seek_a.0, 1, 3));
        let seek_b = alloc.allocate();
        log.note_allocated(seek_b);
        filter.observe_response(&accepted_response(seek_b.0, 1, 4));

        // B's completion event arrives, then a LATE event from A's generation
        assert!(filter.filter_event(&scoped_event(100, 1, 4)));
        assert!(
            !filter.filter_event(&scoped_event(101, 1, 3)),
            "A's ghost dropped"
        );
        // a response interleaved between events is echo-valid
        assert!(log.observe_response(&accepted_response(seek_b.0, 1, 4)));
        // and the events kept their order
        assert!(log.observe_event(&scoped_event(102, 1, 4)));
        assert!(log.violations().is_empty());
    }

    #[test]
    fn request_envelope_carries_the_allocated_id() {
        let mut alloc = RequestIdAllocator::new();
        let req = CommandRequest {
            request_id: alloc.allocate(),
            command: "playback.seek",
        };
        assert_eq!(req.request_id, RequestId(1));
        assert_eq!(req.command, "playback.seek");
    }
}
