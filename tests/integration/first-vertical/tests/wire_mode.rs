//! The wire session: the serialized command layer across a real
//! process boundary (`lumen --control-stdio --json` — NDJSON
//! requests in, NDJSON response envelopes + events out).

mod common;

use common::assert_green;
use first_vertical::scenarios::scenario_wire_session;

/// `wire-session`: the catalog's own serde forms end to end — every
/// request answered with its echoed envelope, the event stream
/// causally ordered, the truthful not-ready refusals typed on the
/// wire, and the whole run byte-deterministic across the process
/// boundary.
#[test]
fn wire_session_through_the_binary() {
    assert_green(&scenario_wire_session());
}
