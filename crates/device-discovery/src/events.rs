//! The bounded volume-event queue: where mount-table diffs land (card
//! 7.D1; the event-plumbing half of the volume lane).
//!
//! The design mirrors the wave-22 platform crates' `DeviceEventQueue`
//! deliberately (same bounds, same counters, same drain shape) so the
//! Lead's seam unification [PROPOSAL] can collapse the two. The queue
//! is `Mutex` + `VecDeque` with a **hard capacity**: when full, the
//! oldest event is dropped and a monotonic `dropped` counter
//! increments (engineering directive D-3: bounded memory everywhere; a
//! mount storm can never grow the process). Consumers `drain` the
//! whole batch — the natural shape for a UI tick or an engine poll.
//!
//! The queue is deliberately dumb: it knows nothing about mount
//! tables or OS APIs. The monitor translates real enumeration diffs
//! into [`VolumeEvent`]s and pushes; tests drive the same `push` path
//! with simulated sources (the mission's sanctioned test double — the
//! *production* path is identical).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::volume::{Volume, VolumeId};

/// One volume change, already translated into engine vocabulary
/// (ARCH-6: no OS types leak upward) and catalog-shaped: the
/// `devices.volume-added` / `devices.volume-removed` payloads are a
/// mechanical field copy of these variants (doc 05 §6.2; see the
/// wire-mapping note in `src/volume.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeEvent {
    /// A volume appeared (or its label/kind changed — a change is
    /// the removed+added pair the catalog's two rows can express).
    Added {
        /// The volume as enumerated.
        volume: Volume,
    },
    /// A volume went away.
    Removed {
        /// The id that disappeared.
        id: VolumeId,
    },
}

impl VolumeEvent {
    /// The volume's id this event speaks about.
    #[must_use = "pure accessor"]
    pub fn volume_id(&self) -> &VolumeId {
        match self {
            VolumeEvent::Added { volume } => &volume.id,
            VolumeEvent::Removed { id } => id,
        }
    }

    /// Whether this is an [`VolumeEvent::Added`] event.
    #[must_use = "pure classification"]
    pub fn is_added(&self) -> bool {
        matches!(self, VolumeEvent::Added { .. })
    }

    /// Whether this is a [`VolumeEvent::Removed`] event.
    #[must_use = "pure classification"]
    pub fn is_removed(&self) -> bool {
        matches!(self, VolumeEvent::Removed { .. })
    }
}

/// The bounded, thread-safe event sink shared between the watcher
/// thread and the consumer.
#[derive(Debug)]
pub struct VolumeEventQueue {
    inner: Mutex<VecDeque<VolumeEvent>>,
    capacity: usize,
    dropped: AtomicU64,
    pushed: AtomicU64,
}

impl VolumeEventQueue {
    /// A queue that retains at most `capacity` events (minimum 1 — a
    /// zero capacity is clamped up, never a silent black hole).
    #[must_use = "constructs the shared queue"]
    pub fn with_capacity(capacity: usize) -> Arc<VolumeEventQueue> {
        Arc::new(VolumeEventQueue {
            inner: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
            dropped: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        })
    }

    /// The default capacity: 128 events (one hotplug storm's worth; a
    /// UI tick drains far fewer).
    #[must_use = "constructs the shared queue"]
    pub fn new() -> Arc<VolumeEventQueue> {
        VolumeEventQueue::with_capacity(128)
    }

    /// Push one event. Returns `false` when the queue was full and
    /// the oldest event was dropped to make room (the caller may log
    /// or count; the `dropped` counter has already moved).
    pub fn push(&self, event: VolumeEvent) -> bool {
        self.pushed.fetch_add(1, Ordering::Relaxed);
        // Lock discipline: short critical section, no callbacks, no
        // allocation — push from the watcher thread is safe.
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
    pub fn drain(&self) -> Vec<VolumeEvent> {
        if let Ok(mut queue) = self.inner.lock() {
            return queue.drain(..).collect();
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

/// The mount-table diff: the pure heart of the watcher (testable
/// without any thread or OS — the monitor thread is a driver around
/// this).
///
/// Semantics:
/// * an id in `after` but not `before` → [`VolumeEvent::Added`];
/// * an id in `before` but not `after` → [`VolumeEvent::Removed`];
/// * an id in both whose label, kind or root changed → the
///   removed+added pair (the catalog's two rows are the only way to
///   express a change; the UI rebuilds the row from the Added payload).
///
/// Events are ordered removed-first (a changed volume's old row must
/// not linger in a UI that appends the new one), then Added events in
/// `after` order.
#[must_use = "pure diff over snapshots"]
pub fn diff_volumes(before: &[Volume], after: &[Volume]) -> Vec<VolumeEvent> {
    let mut events = Vec::new();
    // Changed volumes first (removed half), in `before` order.
    for old in before {
        match after.iter().find(|v| v.id == old.id) {
            None => events.push(VolumeEvent::Removed { id: old.id.clone() }),
            Some(new) => {
                if new.label != old.label || new.kind != old.kind || new.root != old.root {
                    events.push(VolumeEvent::Removed { id: old.id.clone() });
                }
            }
        }
    }
    // Then every volume that is new or changed (added half), in
    // `after` order.
    for new in after {
        let old = before.iter().find(|old| old.id == new.id);
        let changed = old.is_some_and(|old| {
            new.label != old.label || new.kind != old.kind || new.root != old.root
        });
        if old.is_none() || changed {
            events.push(VolumeEvent::Added {
                volume: new.clone(),
            });
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::volume::VolumeKind;

    fn volume(id: &str, label: &str) -> Volume {
        Volume {
            id: VolumeId::new(id),
            label: label.to_string(),
            kind: VolumeKind::Removable,
            root: format!("/media/{id}"),
        }
    }

    #[test]
    fn queue_bounds_and_counts_drop_oldest() {
        let queue = VolumeEventQueue::with_capacity(2);
        assert!(queue.push(VolumeEvent::Added {
            volume: volume("a", "A")
        }));
        assert!(queue.push(VolumeEvent::Added {
            volume: volume("b", "B")
        }));
        // Full: the oldest ("a") is dropped, push reports it.
        assert!(!queue.push(VolumeEvent::Added {
            volume: volume("c", "C")
        }));
        assert_eq!(queue.dropped(), 1);
        assert_eq!(queue.pushed(), 3);
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].volume_id().as_str(), "b");
        assert!(queue.is_empty());
    }

    #[test]
    fn zero_capacity_is_clamped_not_a_black_hole() {
        let queue = VolumeEventQueue::with_capacity(0);
        assert_eq!(queue.capacity(), 1);
        assert!(queue.push(VolumeEvent::Removed {
            id: VolumeId::new("x")
        }));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn diff_reports_adds_and_removals() {
        let before = [volume("a", "A"), volume("b", "B")];
        let after = [volume("b", "B"), volume("c", "C")];
        let events = diff_volumes(&before, &after);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            VolumeEvent::Removed {
                id: VolumeId::new("a")
            }
        );
        assert_eq!(
            events[1],
            VolumeEvent::Added {
                volume: volume("c", "C")
            }
        );
    }

    #[test]
    fn diff_expresses_changes_as_removed_then_added() {
        let before = [volume("usb", "old label")];
        let mut renamed = volume("usb", "new label");
        renamed.kind = VolumeKind::Optical;
        let after = [renamed];
        let events = diff_volumes(&before, &after);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], VolumeEvent::Removed { .. }));
        match &events[1] {
            VolumeEvent::Added { volume } => {
                assert_eq!(volume.label, "new label");
                assert_eq!(volume.kind, VolumeKind::Optical);
            }
            other => unreachable!("second event is the re-add, got {other:?}"),
        }
    }

    #[test]
    fn diff_of_identical_snapshots_is_empty() {
        let snapshot = [volume("a", "A"), volume("b", "B")];
        assert!(diff_volumes(&snapshot, &snapshot).is_empty());
    }

    #[test]
    fn volume_id_accessor_covers_both_variants() {
        let added = VolumeEvent::Added {
            volume: volume("k", "K"),
        };
        let removed = VolumeEvent::Removed {
            id: VolumeId::new("k"),
        };
        assert_eq!(added.volume_id(), removed.volume_id());
    }
}
