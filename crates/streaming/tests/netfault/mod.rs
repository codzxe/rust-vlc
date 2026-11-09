//! The synthetic network fault proxy (issue #14, cards 6.F1/6.F2) —
//! shared test support for the interrupted-network scenarios.
//!
//! A **deterministic fault-injection layer over the fetch port**:
//! [`FaultProxy`] wraps any [`HttpFetch`](streaming::fetch::HttpFetch)
//! (in practice [`MockClient`](streaming::mockhttp::MockClient) over
//! a real [`MockServer`](streaming::mockhttp::MockServer) socket) and
//! applies a scripted per-fetch fault, **at byte offsets** where the
//! fault class is positional:
//!
//! | [`Fault`]             | Wire shape                                        | Typed result                                              |
//! | --------------------- | ------------------------------------------------- | --------------------------------------------------------- |
//! | [`Fault::TruncateAt`] | `Content-Length` advertised, then a clean close   | [`NetworkError::BodyTruncated`] `{expected, received}`     |
//! | [`Fault::ResetAfter`] | a TCP RST mid-body                                | [`NetworkError::Io`] (`ErrorKind::ConnectionReset`)        |
//! | [`Fault::DropAfter`]  | the black hole: bytes stop, the read deadline hit | [`NetworkError::ReadTimeout`]                              |
//! | [`Fault::Delay`]      | wire latency (paced through the injectable clock) | success, after `clock.sleep_ms(ms)`                        |
//!
//! An offset fault only fires when the inner body actually **reaches**
//! its offset (`body.len() > at`) — a fault scripted past the end of
//! the body is a network that healed before the cut, and passes
//! through untouched. This is what makes the proxy *exactly
//! scripted*: `fired()` counts every fault that bit, `fetched()` every
//! fetch the proxy saw, and exhausted scripts pass through clean
//! (the "server had a bad moment, then recovered" shape 6.D2's
//! bounded retry needs).
//!
//! Delay goes **through the injectable clock**
//! ([`WallClock`](streaming::clock::WallClock) /
//! [`FakeClock`](streaming::clock::FakeClock)), not a real sleep: with
//! the fake clock a scripted `Delay{ms: 900}` advances virtual time
//! by exactly 900 ms and is recorded in `clock.sleeps()` — the
//! scenario tests assert the pacing without waiting it.
//!
//! Style follows `mockhttp`'s own scripted handles
//! ([`ScriptedRoute`](streaming::mockhttp::ScriptedRoute) /
//! [`ScriptedStatus`](streaming::mockhttp::ScriptedStatus)): shared
//! state behind the handle, lock-poisoning recovery that never
//! panics, counters for assertions. Test-support only — nothing here
//! is compiled into production paths.

use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::Mutex;

use streaming::clock::WallClock;
use streaming::error::NetworkError;
use streaming::fetch::{HttpFetch, HttpRequest, HttpResponse};

/// One scripted network fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Advertise the full length, deliver only the first `at` body
    /// bytes, then close cleanly — the mid-body truncation shape.
    TruncateAt {
        /// The byte offset the body cuts at.
        at: usize,
    },
    /// Deliver `at` body bytes, then abort the connection the way a
    /// TCP RST does — an I/O error, connection-reset flavor.
    ResetAfter {
        /// The byte offset the reset bites at.
        at: usize,
    },
    /// Deliver `at` body bytes, then go silent until the read
    /// deadline fires — the dropped-network black hole.
    DropAfter {
        /// The byte offset the silence starts at.
        at: usize,
    },
    /// Serve the response `ms` late, paced through the injectable
    /// clock (virtual and recorded under `FakeClock`).
    Delay {
        /// The wire delay in milliseconds.
        ms: u64,
    },
}

/// The per-fetch fault script: fetch `i` gets `faults[i]`; once the
/// script is exhausted every fetch passes through untouched (the
/// network healed). Shared behind the handle like `mockhttp`'s
/// scripted routes — the value installed in a [`FaultProxy`] keeps
/// living in the test for its assertions.
#[derive(Debug, Clone, Default)]
pub struct FaultScript {
    state: Arc<Mutex<FaultScriptState>>,
}

#[derive(Debug, Default)]
struct FaultScriptState {
    faults: Vec<Fault>,
    next: usize,
    fetched: usize,
    fired: usize,
}

impl FaultScript {
    /// A script applying `faults` in order, then passing through.
    #[must_use]
    pub fn new(faults: Vec<Fault>) -> FaultScript {
        FaultScript {
            state: Arc::new(Mutex::new(FaultScriptState {
                faults,
                next: 0,
                fetched: 0,
                fired: 0,
            })),
        }
    }

    /// A script with a single fault, then pass-through (the common
    /// "one bad fetch, then the network recovers" shape).
    #[must_use]
    pub fn once(fault: Fault) -> FaultScript {
        FaultScript::new(vec![fault])
    }

    /// A script that never heals: `fault` on every fetch (the
    /// persistent-outage arm of the bounded-backoff proofs).
    #[must_use]
    pub fn forever(fault: Fault) -> FaultScript {
        // 64 entries: more than any retry budget in this crate's
        // policies (max_retries default 4), still finite so a buggy
        // caller cannot spin forever on an exhausted script.
        FaultScript::new(vec![fault; 64])
    }

    /// The next fault, if the script still has one (internal).
    fn take(&self) -> Option<Fault> {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.fetched += 1;
        let fault = state.faults.get(state.next).copied();
        if fault.is_some() {
            state.next += 1;
        }
        fault
    }

    /// Count a fault that actually bit (internal).
    fn record_fired(&self) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.fired += 1;
    }

    /// How many fetches the proxy has seen (scripted or not).
    pub fn fetched(&self) -> usize {
        match self.state.lock() {
            Ok(guard) => guard.fetched,
            Err(poisoned) => poisoned.into_inner().fetched,
        }
    }

    /// How many scripted faults actually fired (an offset fault
    /// scripted past the body's end does not count — it never bit).
    pub fn fired(&self) -> usize {
        match self.state.lock() {
            Ok(guard) => guard.fired,
            Err(poisoned) => poisoned.into_inner().fired,
        }
    }
}

/// The fault-injection proxy: an [`HttpFetch`] decorator that applies
/// a [`FaultScript`] to the inner transport's successful responses.
///
/// Inner errors pass through untouched (the proxy faults responses,
/// not failures); a faulted response is discarded and surfaces as the
/// fault class's typed [`NetworkError`] — never a panic, never a
/// silent partial body.
pub struct FaultProxy {
    inner: Arc<dyn HttpFetch>,
    script: FaultScript,
    clock: Arc<dyn WallClock>,
}

impl FaultProxy {
    /// Wrap `inner` with `script`, pacing [`Fault::Delay`] through
    /// `clock` (the injectable clock — `FakeClock` in tests).
    #[must_use]
    pub fn new(
        inner: Arc<dyn HttpFetch>,
        script: FaultScript,
        clock: Arc<dyn WallClock>,
    ) -> FaultProxy {
        FaultProxy {
            inner,
            script,
            clock,
        }
    }
}

impl HttpFetch for FaultProxy {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        let fault = self.script.take();
        let response = self.inner.fetch(request)?;
        // The fault applies to the successful response the healthy
        // network would have delivered (the proxy is deterministic:
        // no socket-timing races to reconstruct what "half a body"
        // means — the offset decides).
        let len = response.body.len();
        match fault {
            Some(Fault::Delay { ms }) => {
                self.script.record_fired();
                self.clock.sleep_ms(ms);
                Ok(response)
            }
            Some(Fault::TruncateAt { at }) if len > at => {
                self.script.record_fired();
                Err(NetworkError::BodyTruncated {
                    expected: len as u64,
                    received: at as u64,
                })
            }
            Some(Fault::ResetAfter { at }) if len > at => {
                self.script.record_fired();
                Err(NetworkError::Io {
                    source: Error::new(
                        ErrorKind::ConnectionReset,
                        "fault-proxy: scripted connection reset mid-body",
                    ),
                })
            }
            Some(Fault::DropAfter { at }) if len > at => {
                self.script.record_fired();
                Err(NetworkError::ReadTimeout)
            }
            // No script left, or the fault's offset sits past the
            // body's end: the network healed before the cut — the
            // response passes through untouched.
            _ => Ok(response),
        }
    }
}
