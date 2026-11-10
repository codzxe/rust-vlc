//! The bounded device-event queue: where OS change notifications
//! land (card 7.A1; the event-plumbing half of the notification lane).
//!
//! OS notification callbacks arrive on foreign threads (the OS
//! callback thread on Windows, the PulseAudio event loop here), so
//! the queue is `Mutex` + `VecDeque` with a **hard capacity**: when
//! full, the oldest event is dropped and a monotonic
//! `dropped` counter increments (engineering directive D-3: bounded
//! memory everywhere; a notification flood can never grow the
//! process). Consumers `drain` the whole batch — the natural shape
//! for a UI tick or an engine poll.
//!
//! The queue is deliberately dumb: it knows nothing about PulseAudio
//! or DRM. The OS integration translates native callbacks into
//! [`PlatformEvent`]s and pushes; tests drive the same `push` path
//! with simulated events (the mission's sanctioned test double —
//! the *production* path is identical).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use audio_output::DeviceId;

/// One device-change notification, already translated into engine
/// vocabulary (ARCH-6: no OS types leak upward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEvent {
    /// An audio device appeared.
    AudioDeviceAdded {
        /// The device's stable id (same minting as enumeration).
        id: DeviceId,
    },
    /// An audio device went away.
    AudioDeviceRemoved {
        /// The device's stable id.
        id: DeviceId,
    },
    /// A device transitioned active/inactive (Windows
    /// `DEVICE_STATE_NOTPRESENT`↔`ACTIVE`; disabled/unplugged states
    /// surface here rather than as spurious add/remove pairs).
    AudioDeviceStateChanged {
        /// The device's stable id.
        id: DeviceId,
        /// `true` when the device became active.
        active: bool,
    },
    /// The default audio device changed (`None` = none selected).
    AudioDefaultChanged {
        /// The new default, if the OS reported one.
        id: Option<DeviceId>,
    },
    /// The device set changed in a way the OS did not itemize —
    /// re-enumerate (PulseAudio server events, CoreAudio
    /// `kAudioHardwarePropertyDevices` changes).
    AudioSetChanged,
}

/// The bounded, thread-safe event sink shared between the OS
/// integration and the consumer.
#[derive(Debug)]
pub struct DeviceEventQueue {
    inner: Mutex<VecDeque<PlatformEvent>>,
    capacity: usize,
    dropped: AtomicU64,
    pushed: AtomicU64,
}

impl DeviceEventQueue {
    /// A queue that retains at most `capacity` events (minimum 1 — a
    /// zero capacity is clamped up, never a silent black hole).
    #[must_use = "constructs the shared queue"]
    pub fn with_capacity(capacity: usize) -> Arc<DeviceEventQueue> {
        Arc::new(DeviceEventQueue {
            inner: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
            dropped: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        })
    }

    /// The default capacity: 128 events (one hotplug storm's worth;
    /// a UI tick drains far fewer).
    #[must_use = "constructs the shared queue"]
    pub fn new() -> Arc<DeviceEventQueue> {
        DeviceEventQueue::with_capacity(128)
    }

    /// Push one event. Returns `false` when the queue was full and
    /// the oldest event was dropped to make room (the caller may log
    /// or count; the `dropped` counter has already moved).
    pub fn push(&self, event: PlatformEvent) -> bool {
        self.pushed.fetch_add(1, Ordering::Relaxed);
        // Lock discipline: short critical section, no callbacks, no
        // allocation — push from an OS callback thread is safe.
        if let Ok(mut queue) = self.inner.lock() {
            let dropped_one = queue.len() >= self.capacity;
            if dropped_one {
                queue.pop_front();
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
            queue.push_back(event);
            !dropped_one
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Take every buffered event, oldest first.
    pub fn drain(&self) -> Vec<PlatformEvent> {
        if let Ok(mut queue) = self.inner.lock() {
            let drained: Vec<PlatformEvent> = queue.drain(..).collect();
            return drained;
        }
        Vec::new()
    }

    /// Events currently buffered (without draining).
    #[must_use = "pure observation"]
    pub fn len(&self) -> usize {
        if let Ok(queue) = self.inner.lock() {
            return queue.len();
        }
        0
    }

    /// Whether no event is buffered.
    #[must_use = "pure observation"]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The retained-event bound.
    #[must_use = "pure accessor"]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Monotonic count of events dropped by the bound (oldest-first).
    #[must_use = "pure accessor"]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Monotonic count of events ever pushed.
    #[must_use = "pure accessor"]
    pub fn pushed(&self) -> u64 {
        self.pushed.load(Ordering::Relaxed)
    }
}
