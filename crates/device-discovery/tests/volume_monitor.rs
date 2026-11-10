//! Card 7.D1 integration: the volume watcher's public API end to end —
//! a host-shaped driver over a scripted source (the sanctioned
//! test-only simulated input; production sources are the platform-*
//! integrations that land with their cards). Proves: the initial
//! mount set arrives as Added events; a scripted removal arrives as a
//! Removed event; the event vocabulary maps 1:1 onto the catalog's
//! `devices.volume-added` / `devices.volume-removed` payload shape
//! (the "mechanical field copy" the emission ledger's host relay
//! will perform); concurrent drain is safe; shutdown joins.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use device_discovery::events::{VolumeEvent, VolumeEventQueue};
use device_discovery::volume::{Volume, VolumeId, VolumeKind, VolumeSource};
use device_discovery::{DiscoveryContext, MonitorConfig, VolumeMonitor};

/// Scripted source: an initial snapshot, then one edit, then silence.
struct ScriptedMounts {
    initial: Vec<Volume>,
    edited: Vec<Volume>,
    served_edit: bool,
}

impl VolumeSource for ScriptedMounts {
    fn enumerate(
        &mut self,
        _ctx: &DiscoveryContext,
    ) -> Result<Vec<Volume>, device_discovery::error::DiscoveryError> {
        if self.served_edit {
            // Steady state: the edited table, forever.
            return Ok(self.edited.clone());
        }
        self.served_edit = true;
        Ok(self.initial.clone())
    }
}

fn volume(id: &str, label: &str, kind: VolumeKind, root: &str) -> Volume {
    Volume {
        id: VolumeId::new(id),
        label: label.to_string(),
        kind,
        root: root.to_string(),
    }
}

/// The catalog payload shape (doc 05 §6.2, the 7.C1 additive rows):
/// what the engine-side relay copies out of a `VolumeEvent`.
#[derive(Debug, PartialEq, Eq)]
struct VolumeAddedWire {
    id: String,
    label: String,
    volume_kind: String,
}

#[derive(Debug, PartialEq, Eq)]
struct VolumeRemovedWire {
    id: String,
}

/// The mechanical relay: field copy, no re-derivation (this is the
/// contract the emission ledger's "host wiring" step depends on).
fn relay_added(event: &VolumeEvent) -> Option<VolumeAddedWire> {
    match event {
        VolumeEvent::Added { volume } => Some(VolumeAddedWire {
            id: volume.id.to_string(),
            label: volume.label.clone(),
            volume_kind: volume.kind.as_str().to_string(),
        }),
        VolumeEvent::Removed { .. } => None,
    }
}

fn relay_removed(event: &VolumeEvent) -> Option<VolumeRemovedWire> {
    match event {
        VolumeEvent::Removed { id } => Some(VolumeRemovedWire { id: id.to_string() }),
        VolumeEvent::Added { .. } => None,
    }
}

/// Drain `queue` until `pred` holds over the accumulated events, or
/// the deadline passes (returns what arrived either way).
fn drain_until(
    queue: &VolumeEventQueue,
    deadline: Duration,
    mut pred: impl FnMut(&[VolumeEvent]) -> bool,
) -> Vec<VolumeEvent> {
    let end = std::time::Instant::now() + deadline;
    let mut seen = Vec::new();
    while std::time::Instant::now() < end {
        seen.extend(queue.drain());
        if pred(&seen) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    seen
}

#[test]
fn watcher_reports_initial_mounts_then_the_scripted_removal() {
    let initial = vec![
        volume("fixed-0", "System", VolumeKind::Fixed, "/"),
        volume(
            "usb-7",
            "Sandisk 32G",
            VolumeKind::Removable,
            "/media/Sandisk 32G",
        ),
        volume("nfs-1", "nas", VolumeKind::Network, "/mnt/nas"),
    ];
    let edited = vec![
        volume("fixed-0", "System", VolumeKind::Fixed, "/"),
        volume("nfs-1", "nas", VolumeKind::Network, "/mnt/nas"),
    ];
    let mut handle = VolumeMonitor::spawn(
        Box::new(ScriptedMounts {
            initial,
            edited,
            served_edit: false,
        }),
        MonitorConfig::with_limits(Duration::from_millis(100), 32),
    );

    // Phase 1: the initial set diffses against empty → three Adds,
    // in the real wire vocabulary.
    let events = drain_until(handle.events(), Duration::from_secs(3), |seen| {
        seen.len() >= 3
    });
    let wires: Vec<VolumeAddedWire> = events.iter().filter_map(relay_added).collect();
    assert_eq!(
        wires,
        vec![
            VolumeAddedWire {
                id: "fixed-0".to_string(),
                label: "System".to_string(),
                volume_kind: "fixed".to_string(),
            },
            VolumeAddedWire {
                id: "usb-7".to_string(),
                label: "Sandisk 32G".to_string(),
                volume_kind: "removable".to_string(),
            },
            VolumeAddedWire {
                id: "nfs-1".to_string(),
                label: "nas".to_string(),
                volume_kind: "network".to_string(),
            },
        ],
        "the relay is a field copy onto the catalog payload shape"
    );

    // Phase 2: the next pass serves the edited table → one Removed.
    let events = drain_until(handle.events(), Duration::from_secs(3), |seen| {
        seen.iter().any(VolumeEvent::is_removed)
    });
    let removals: Vec<VolumeRemovedWire> = events.iter().filter_map(relay_removed).collect();
    assert_eq!(
        removals,
        vec![VolumeRemovedWire {
            id: "usb-7".to_string()
        }]
    );
    assert_eq!(handle.errors(), 0, "scripted passes never fail");
    handle.stop();
}

#[test]
fn concurrent_drain_while_the_watcher_pushes_is_safe() {
    // A queue shared with a pushing thread (the storm shape): the
    // consumer drains on its own thread; the bound holds; counters
    // stay consistent; nothing deadlocks or panics.
    let queue = VolumeEventQueue::with_capacity(8);
    let pushed_volume = volume("s-0", "Storm", VolumeKind::Removable, "/media/storm");
    let producer = {
        let queue = Arc::clone(&queue);
        thread::spawn(move || {
            for i in 0..64_u64 {
                if i == 32 {
                    thread::sleep(Duration::from_millis(5)); // let the consumer catch up mid-storm
                }
                queue.push(VolumeEvent::Added {
                    volume: pushed_volume.clone(),
                });
            }
        })
    };
    let consumer = {
        let queue = Arc::clone(&queue);
        thread::spawn(move || {
            let mut drained = 0_usize;
            let end = std::time::Instant::now() + Duration::from_secs(3);
            while drained < 64 && std::time::Instant::now() < end {
                drained += queue.drain().len();
                thread::sleep(Duration::from_millis(1));
            }
            drained
        })
    };
    // Join failures are panics in a helper thread — surface them
    // honestly instead of Must (whose Display bound join errors lack).
    producer
        .join()
        .unwrap_or_else(|_| unreachable!("producer thread panicked"));
    let drained = consumer
        .join()
        .unwrap_or_else(|_| unreachable!("consumer thread panicked"));
    assert_eq!(queue.pushed(), 64);
    // Exactly the pushed set minus whatever the bound dropped while
    // the consumer was away — the invariant is the accounting, not
    // the lucky timing.
    assert_eq!(
        queue.dropped() as usize + queue.len() + drained,
        64,
        "pushed == dropped + retained + drained (bounded accounting)"
    );
}

#[test]
fn stop_is_idempotent_and_drop_safe() {
    let mut handle = VolumeMonitor::spawn(
        Box::new(ScriptedMounts {
            initial: vec![volume("a", "A", VolumeKind::Optical, "/media/cd")],
            edited: vec![],
            served_edit: false,
        }),
        MonitorConfig::with_limits(Duration::from_secs(30), 8),
    );
    handle.stop();
    handle.stop(); // idempotent
    assert!(!handle.is_running());
    drop(handle); // Drop after explicit stop: still clean
}
