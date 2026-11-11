//! Injectable wall-clock + sleep port (cards 6.D1/6.D2).
//!
//! The buffering model (pacing, rebuffer *durations*), the backoff
//! schedule (sleeps between retries) and the HLS live reload pacing
//! all need **wall-clock time** — but engine code never reads ambient
//! time to make a *decision*: every decision takes `now` from its
//! caller, and only the *waiting* itself touches a real clock (the
//! media-clock injection doctrine, doc 03 §8, restated for the
//! streaming layer by recovery.rs's "time doctrine" precedent in the
//! playback engine).
//!
//! [`WallClock`] is the two-method port both halves need:
//!
//! - [`WallClock::now_ms`] — the monotonic millisecond reading a
//!   policy compares deadlines against;
//! - [`WallClock::sleep_ms`] — the bounded wait a *pull-driven*
//!   consumer performs when the honest next step is "not yet"
//!   (HLS reload pacing, retry backoff). Production code sleeps the
//!   calling thread (the input worker, doc 01 §4.1 — blocking there
//!   is legal; the `SourceStream` contract bounds it per call);
//!   tests inject [`FakeClock`], whose sleep **advances virtual
//!   time instantly**, so a paced live session or a backing-off
//!   retry runs in microseconds of real time and stays
//!   deterministic.
//!
//! Monotonic: the production clock is built on [`std::time::Instant`]
//! (never `SystemTime` — wall-clock skew and NTP steps must not move
//! a deadline backwards). The epoch is the process's first use of
//! the clock, not the Unix epoch: only *differences* are meaningful
//! and the port keeps them small (`u64` milliseconds).
//!
//! # Bounded by contract
//!
//! Every sleep issued through this port is bounded by a caller's
//! explicit budget (a backoff ceiling, a reload interval): the port
//! itself imposes no additional cap, but no unbounded wait may be
//! expressed with it — `sleep_ms(u64::MAX)` is a caller bug.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The monotonic wall-clock + bounded-sleep port (module docs).
///
/// `Send + Sync` so one shared clock serves the fetch port's
/// worker-thread callers; cheap to hold behind an [`Arc`].
pub trait WallClock: Send + Sync {
    /// Milliseconds since the port's monotonic epoch (only
    /// differences are meaningful).
    fn now_ms(&self) -> u64;

    /// Sleep **exactly** `ms` monotonic milliseconds (a bounded wait
    /// the caller budgeted; the fake advances virtual time instead).
    fn sleep_ms(&self, ms: u64);
}

/// The production clock: process-wide monotonic [`Instant`] base +
/// real [`std::thread::sleep`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// A shared handle (one lazily-created `Arc` per process — the
    /// resource constructors that default to wall time share it).
    pub fn shared() -> Arc<dyn WallClock> {
        static SHARED: OnceLock<Mutex<Option<Arc<dyn WallClock>>>> = OnceLock::new();
        let mut guard = SHARED
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.get_or_insert_with(|| Arc::new(SystemClock)).clone()
    }
}

impl WallClock for SystemClock {
    fn now_ms(&self) -> u64 {
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        EPOCH
            .get_or_init(Instant::now)
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64
    }

    fn sleep_ms(&self, ms: u64) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
}

/// The deterministic test clock: virtual milliseconds plus an
/// instant "sleep" that advances them (and records the waits, so
/// tests can assert the schedule the code under test issued —
/// the pacing/backoff proofs).
///
/// Clone shares the same virtual time (the handle is an `Arc`), so a
/// test can hold one clone to read/advance while the session under
/// test holds another.
#[derive(Debug, Clone, Default)]
pub struct FakeClock {
    state: Arc<Mutex<FakeClockState>>,
}

#[derive(Debug, Default)]
struct FakeClockState {
    now_ms: u64,
    /// The sleeps issued through this clock, in order (`ms` each).
    sleeps: Vec<u64>,
}

impl FakeClock {
    /// A clock frozen at virtual millisecond 0.
    pub fn new() -> FakeClock {
        FakeClock::default()
    }

    /// Force the virtual reading (test setup — e.g. pre-age a
    /// deadline).
    pub fn set_ms(&self, ms: u64) {
        let mut state = self.lock();
        state.now_ms = ms;
    }

    /// Advance virtual time without sleeping (the test-driven tick).
    pub fn advance_ms(&self, ms: u64) {
        let mut state = self.lock();
        state.now_ms = state.now_ms.saturating_add(ms);
    }

    /// The sleeps issued through this clock so far (assertion
    /// surface: the backoff/pacing schedule).
    pub fn sleeps(&self) -> Vec<u64> {
        self.lock().sleeps.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FakeClockState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl WallClock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.lock().now_ms
    }

    fn sleep_ms(&self, ms: u64) {
        if ms > 0 {
            let mut state = self.lock();
            state.now_ms = state.now_ms.saturating_add(ms);
            state.sleeps.push(ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fake clock's sleep advances virtual time and is recorded.
    #[test]
    fn fake_clock_sleep_advances_and_records() {
        let clock = FakeClock::new();
        clock.advance_ms(500);
        assert_eq!(clock.now_ms(), 500);
        clock.sleep_ms(250);
        clock.sleep_ms(125);
        assert_eq!(clock.now_ms(), 875);
        assert_eq!(clock.sleeps(), vec![250, 125]);
        // A zero sleep is a no-op on both surfaces.
        clock.sleep_ms(0);
        assert_eq!(clock.now_ms(), 875);
        assert_eq!(clock.sleeps().len(), 2);
    }

    /// Clones share virtual time (the session and the test read the
    /// same clock).
    #[test]
    fn fake_clock_clones_share_time() {
        let clock = FakeClock::new();
        let handle = clock.clone();
        handle.sleep_ms(1000);
        assert_eq!(clock.now_ms(), 1000);
        clock.set_ms(5);
        assert_eq!(handle.now_ms(), 5);
    }

    /// The system clock is monotonic across calls and never panics
    /// on a zero sleep.
    #[test]
    fn system_clock_is_monotonic() {
        let clock = SystemClock;
        let first = clock.now_ms();
        clock.sleep_ms(0);
        let second = clock.now_ms();
        assert!(second >= first, "monotonic: {second} >= {first}");
    }

    /// Both clocks satisfy the port bounds (shareable across worker
    /// threads — the fetch port's calling contract).
    #[test]
    fn clocks_are_send_sync() {
        fn assert_bounds<T: WallClock>() {}
        assert_bounds::<SystemClock>();
        assert_bounds::<FakeClock>();
        let shared: Arc<dyn WallClock> = SystemClock::shared();
        let moved = std::thread::spawn(move || shared.now_ms());
        let reading = match moved.join() {
            Ok(reading) => reading,
            Err(_) => unreachable!("the clock crossed the thread boundary"),
        };
        assert!(reading < u64::MAX, "a sane monotonic reading: {reading}");
    }
}
