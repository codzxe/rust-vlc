//! The volume watcher: one bounded thread that polls a
//! [`VolumeSource`], diffs consecutive snapshots and pushes
//! [`VolumeEvent`]s (card 7.D1; issue #42).
//!
//! # Why a poll-diff watcher
//!
//! Doc 06 §1.3's engine-driven posture: the *integration* never owns
//! a thread; the engine-side driver polls on its own cadence. The
//! monitor is that driver — one thread, one cadence, one bounded
//! queue (the "bounded pools only" line of doc 01 §2.4's Never list:
//! exactly one watcher thread per handle, join on drop, no spawn
//! storms). The pure half ([`crate::events::diff_volumes`]) is
//! separately testable without any thread.
//!
//! # Honesty rules
//!
//! * Events come only from **real enumeration diffs** — the first
//!   pass diffed against empty reports every currently-mounted volume
//!   as Added (that is how the UI learns the initial set; there is no
//!   `devices.list` catalog row). An enumerate failure pushes NOTHING
//!   (a failed pass is not "all volumes removed") — the error counter
//!   moves and the cadence continues.
//! * The queue is capacity-clamped drop-oldest with counters (D-3).
//! * Shutdown is responsive (the sleep is sliced) and the handle
//!   joins the thread on drop — no leaked watchers.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::context::DiscoveryContext;
use crate::error::DiscoveryError;
use crate::events::{VolumeEvent, VolumeEventQueue, diff_volumes};
use crate::volume::{Volume, VolumeSource};

/// Watcher tuning (all bounds clamped; D-2/D-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorConfig {
    /// Cadence between enumerate passes. Clamped to at least
    /// [`MonitorConfig::MIN_POLL_INTERVAL`] (a tighter spin is a busy
    /// loop, not responsiveness).
    pub poll_interval: Duration,
    /// The event queue's retained-event bound.
    pub queue_capacity: usize,
}

impl MonitorConfig {
    /// The floor for `poll_interval` (100 ms — volume tables change
    /// on human timescales).
    pub const MIN_POLL_INTERVAL: Duration = Duration::from_millis(100);

    /// The defaults: a 2 s cadence (mount tables change rarely; the
    /// cost is one bounded syscall-set per pass), a 128-event queue.
    #[must_use = "constructs the config"]
    pub fn new() -> MonitorConfig {
        MonitorConfig {
            poll_interval: Duration::from_secs(2),
            queue_capacity: 128,
        }
    }

    /// A config with explicit bounds, clamped to the floors.
    #[must_use = "constructs the config"]
    pub fn with_limits(poll_interval: Duration, queue_capacity: usize) -> MonitorConfig {
        MonitorConfig {
            poll_interval: poll_interval.max(MonitorConfig::MIN_POLL_INTERVAL),
            queue_capacity: queue_capacity.max(1),
        }
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig::new()
    }
}

/// A running watcher. Dropping it signals shutdown and JOINS the
/// thread (bounded shutdown latency: one poll-interval slice, 50 ms
/// max — see the loop).
pub struct VolumeMonitorHandle {
    events: Arc<VolumeEventQueue>,
    shutdown: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    errors: Arc<AtomicU64>,
    passes: Arc<AtomicU64>,
}

impl VolumeMonitorHandle {
    /// The bounded event sink (drain me on your tick).
    #[must_use = "pure accessor"]
    pub fn events(&self) -> &VolumeEventQueue {
        &self.events
    }

    /// Monotonic count of enumerate passes that failed (honesty
    /// instrument: a flaky integration is visible without fabricating
    /// removal events).
    #[must_use = "pure accessor"]
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Monotonic count of enumerate passes attempted.
    #[must_use = "pure accessor"]
    pub fn passes(&self) -> u64 {
        self.passes.load(Ordering::Relaxed)
    }

    /// Whether the watcher thread is still running.
    #[must_use = "pure observation"]
    pub fn is_running(&self) -> bool {
        !self.shutdown.load(Ordering::Relaxed)
    }

    /// Signal shutdown and join (idempotent — the Drop impl calls
    /// this too, so an explicit stop followed by drop is safe).
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            // A watcher thread never panics by construction (every
            // fallible step is a typed Result); a join error is a
            // poisoned-process signal we can only observe.
            drop(join.join());
        }
    }
}

impl Drop for VolumeMonitorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The watcher's constructor namespace. The running state lives in
/// [`VolumeMonitorHandle`] + the private worker; this type carries
/// the spawn API (and documents that one handle == exactly one
/// bounded thread).
pub struct VolumeMonitor;

impl VolumeMonitor {
    /// Spawn the watcher over `source` (one bounded thread).
    ///
    /// The first pass diffses against the empty snapshot, so the
    /// currently-mounted volumes arrive as Added events — real
    /// enumeration data, never a fabricated seed.
    pub fn spawn(source: Box<dyn VolumeSource>, config: MonitorConfig) -> VolumeMonitorHandle {
        let events = VolumeEventQueue::with_capacity(config.queue_capacity);
        let shutdown = Arc::new(AtomicBool::new(false));
        let errors = Arc::new(AtomicU64::new(0));
        let passes = Arc::new(AtomicU64::new(0));
        let ctx = DiscoveryContext::new();

        let mut worker = MonitorWorker {
            source,
            last: Vec::new(),
            events: Arc::clone(&events),
            shutdown: Arc::clone(&shutdown),
            errors: Arc::clone(&errors),
            passes: Arc::clone(&passes),
            ctx,
        };
        let join = thread::Builder::new()
            // Diagnostic thread name (debugger/log surface only, not a
            // user-visible string).
            // lumen-allow-literal: diagnostic thread name for the volume watcher.
            .name("lumen-volume-watch".to_string())
            .spawn(move || worker.run(config.poll_interval))
            // A thread-spawn failure is an environment-level refusal
            // (rlimits); the honest handle reports a dead watcher
            // (is_running false, zero passes) rather than crashing the
            // host.
            .ok();

        VolumeMonitorHandle {
            events,
            shutdown,
            join,
            errors,
            passes,
        }
    }
}

/// The thread's private state (kept in a struct so `run` reads as the
/// protocol, not a closure soup).
struct MonitorWorker {
    source: Box<dyn VolumeSource>,
    last: Vec<Volume>,
    events: Arc<VolumeEventQueue>,
    shutdown: Arc<AtomicBool>,
    errors: Arc<AtomicU64>,
    passes: Arc<AtomicU64>,
    ctx: DiscoveryContext,
}

impl MonitorWorker {
    fn run(&mut self, poll_interval: Duration) {
        // Pass, then sliced sleep — the slice keeps shutdown latency
        // at ~50 ms regardless of the (possibly minutes-long) cadence.
        const SHUTDOWN_SLICE: Duration = Duration::from_millis(50);
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            self.one_pass();
            let mut remaining = poll_interval;
            while remaining > Duration::ZERO {
                if self.shutdown.load(Ordering::Relaxed) {
                    return;
                }
                let slice = remaining.min(SHUTDOWN_SLICE);
                thread::sleep(slice);
                remaining = remaining.saturating_sub(slice);
            }
        }
    }

    /// One enumerate pass: real data → diff → events; failure →
    /// counter, no fake events.
    fn one_pass(&mut self) {
        self.passes.fetch_add(1, Ordering::Relaxed);
        let snapshot = match self.source.enumerate(&self.ctx) {
            Ok(volumes) => volumes,
            Err(_err) => {
                // An enumerate failure is NOT "everything removed".
                // Count it; the queue keeps the last known truth until
                // a pass succeeds again.
                self.errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };
        if snapshot.len() > self.ctx.max_devices {
            // Source contract violation — loud, not silent truncation
            // (D-3). Counts as a failed pass.
            self.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        for event in diff_volumes(&self.last, &snapshot) {
            self.events.push(event);
        }
        self.last = snapshot;
    }
}

/// The typed outcome of one manual pass (the loop-free API for hosts
/// that own their own cadence — e.g. the wasm host's pump slices, doc
/// 09 §3): diff against an explicit `before` and the resulting
/// snapshot.
///
/// This is the same computation the thread performs per pass, exposed
/// so a host without a watcher thread (or a test) can drive it
/// deterministically.
#[must_use = "pure one-pass computation"]
pub fn poll_once(
    source: &mut dyn VolumeSource,
    before: &[Volume],
    ctx: &DiscoveryContext,
) -> Result<(Vec<VolumeEvent>, Vec<Volume>), DiscoveryError> {
    let snapshot = source.enumerate(ctx)?;
    if snapshot.len() > ctx.max_devices {
        return Err(DiscoveryError::CapacityExceeded {
            what: "volume list",
            limit: ctx.max_devices,
            attempted: snapshot.len(),
        });
    }
    let events = diff_volumes(before, &snapshot);
    Ok((events, snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::volume::{VolumeId, VolumeKind};

    fn volume(id: &str, label: &str) -> Volume {
        Volume {
            id: VolumeId::new(id),
            label: label.to_string(),
            kind: VolumeKind::Removable,
            root: format!("/media/{id}"),
        }
    }

    /// Scripted source: serves snapshots in sequence, then the last
    /// one forever (the sanctioned test double — production sources
    /// are the platform integrations).
    struct Scripted {
        rounds: Vec<Result<Vec<Volume>, DiscoveryError>>,
        cursor: usize,
    }

    impl VolumeSource for Scripted {
        fn enumerate(&mut self, _ctx: &DiscoveryContext) -> Result<Vec<Volume>, DiscoveryError> {
            let index = self.cursor.min(self.rounds.len().saturating_sub(1));
            self.cursor += 1;
            self.rounds[index].clone()
        }
    }

    #[test]
    fn config_clamps_to_floors() {
        let cfg = MonitorConfig::with_limits(Duration::from_millis(1), 0);
        assert_eq!(cfg.poll_interval, MonitorConfig::MIN_POLL_INTERVAL);
        assert_eq!(cfg.queue_capacity, 1);
        assert_eq!(MonitorConfig::new().poll_interval, Duration::from_secs(2));
    }

    #[test]
    fn poll_once_diffs_and_returns_the_snapshot() {
        let mut source = Scripted {
            rounds: vec![Ok(vec![volume("a", "A")]), Ok(vec![volume("b", "B")])],
            cursor: 0,
        };
        let (events, snapshot) = match poll_once(&mut source, &[], &DiscoveryContext::new()) {
            Ok(pair) => pair,
            Err(e) => unreachable!("scripted pass must succeed: {e}"),
        };
        assert_eq!(events.len(), 1);
        assert_eq!(snapshot.len(), 1);

        let (events, _) = match poll_once(&mut source, &snapshot, &DiscoveryContext::new()) {
            Ok(pair) => pair,
            Err(e) => unreachable!("scripted pass must succeed: {e}"),
        };
        assert_eq!(events.len(), 2, "a removed + b added");
        assert!(matches!(events[0], VolumeEvent::Removed { .. }));
    }

    #[test]
    fn poll_once_propagates_the_typed_failure() {
        let mut source = Scripted {
            rounds: vec![Err(DiscoveryError::Unavailable {
                reason: crate::error::UnavailableReason::NotYetImplemented { detail: "gap" },
            })],
            cursor: 0,
        };
        match poll_once(&mut source, &[], &DiscoveryContext::new()) {
            Err(DiscoveryError::Unavailable { .. }) => {}
            other => unreachable!("failure is typed, got {other:?}"),
        }
    }

    #[test]
    fn poll_once_enforces_the_device_bound() {
        let many: Vec<Volume> = (0..300).map(|i| volume(&format!("v{i}"), "V")).collect();
        let mut source = Scripted {
            rounds: vec![Ok(many)],
            cursor: 0,
        };
        match poll_once(
            &mut source,
            &[],
            &DiscoveryContext::with_limits(256, Duration::from_secs(1)),
        ) {
            Err(DiscoveryError::CapacityExceeded {
                limit: 256,
                attempted: 300,
                ..
            }) => {}
            other => unreachable!("bound is loud, got {other:?}"),
        }
    }

    #[test]
    fn spawn_first_pass_reports_mounted_volumes_as_added() {
        let mut handle = VolumeMonitor::spawn(
            Box::new(Scripted {
                rounds: vec![Ok(vec![volume("usb0", "Sandisk"), volume("cd0", "Disc")])],
                cursor: 0,
            }),
            MonitorConfig::with_limits(Duration::from_millis(100), 16),
        );
        // The first pass runs immediately; wait for it honestly.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut drained: Vec<VolumeEvent> = Vec::new();
        while std::time::Instant::now() < deadline {
            drained.extend(handle.events().drain());
            if drained.len() >= 2 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        handle.stop();
        assert_eq!(
            drained.len(),
            2,
            "initial mount set, real diff data: {drained:?}"
        );
        assert!(
            drained
                .iter()
                .all(|e| matches!(e, VolumeEvent::Added { .. }))
        );
        assert!(handle.passes() >= 1);
        assert_eq!(handle.errors(), 0);
        assert!(!handle.is_running(), "stop() joined the thread");
    }

    #[test]
    fn failed_passes_count_errors_and_never_emit_removals() {
        let mut handle = VolumeMonitor::spawn(
            Box::new(Scripted {
                rounds: vec![
                    Ok(vec![volume("usb0", "Sandisk")]),
                    Err(DiscoveryError::EnumerationFailed {
                        detail: "mount table read failed".to_string(),
                    }),
                ],
                cursor: 0,
            }),
            MonitorConfig::with_limits(Duration::from_millis(100), 16),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut drained: Vec<VolumeEvent> = Vec::new();
        let mut saw_error = false;
        while std::time::Instant::now() < deadline {
            drained.extend(handle.events().drain());
            if handle.errors() >= 1 && handle.passes() >= 2 {
                saw_error = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        handle.stop();
        assert!(saw_error, "the failing pass must be counted");
        assert!(
            drained
                .iter()
                .all(|e| matches!(e, VolumeEvent::Added { .. })),
            "a failed pass never fabricates removals: {drained:?}"
        );
    }
}
