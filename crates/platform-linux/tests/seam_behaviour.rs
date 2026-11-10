//! Seam-behaviour acceptance tests for `platform-linux` (card 7.A2):
//! the typed fallbacks, capability negotiation, event-queue plumbing
//! and id minting — every assertion host-runnable on any machine
//! (the OS integrations themselves are exercised in
//! `linux_integrations.rs` on Linux hosts).

use platform_linux::audio::{AudioDevices, UnavailableAudioDevices, stable_device_id};
use platform_linux::capability::{Capability, Feature, PlatformReport, UnavailableReason};
use platform_linux::dialog::{FileDialogRequest, FileDialogs, FilePick, UnavailableDialogs};
use platform_linux::error::PlatformError;
use platform_linux::events::{DeviceEventQueue, PlatformEvent};
use platform_linux::gpu::{GpuAdapterInfo, GpuAdapters, UnavailableGpuAdapters};

use testkit::assert_err;

// ---------------------------------------------------------------------------
// Typed fallbacks: every method answers the reason, never a fake
// success (the graceful-degradation contract).
// ---------------------------------------------------------------------------

#[test]
fn unavailable_audio_devices_returns_the_typed_reason() {
    let reason = PlatformError::NotCompiledForThisOs { crate_os: "linux" };
    let mut devices = UnavailableAudioDevices::because(reason.clone());
    assert_eq!(devices.reason(), &reason);
    let queue = DeviceEventQueue::new();
    let err = assert_err(devices.enumerate(), "enumerate must fail honestly");
    assert_eq!(err, reason);
    let err = assert_err(devices.subscribe(queue), "subscribe must fail honestly");
    assert_eq!(err, reason);
    let err = assert_err(devices.poll(), "poll must fail honestly");
    assert_eq!(err, reason);
}

#[test]
fn unavailable_gpu_adapters_returns_the_typed_reason() {
    let reason = PlatformError::LibraryUnavailable {
        names: vec!["libpulse.so.0".to_string()],
        detail: "cannot open shared object file".to_string(),
    };
    let mut adapters = UnavailableGpuAdapters::because(reason.clone());
    assert_eq!(adapters.reason(), &reason);
    let err = assert_err(adapters.enumerate(), "enumerate must fail honestly");
    assert_eq!(err, reason);
}

#[test]
fn unavailable_dialogs_returns_the_typed_reason() {
    let reason = PlatformError::ServiceUnavailable {
        service: "file dialogs",
        detail: "no backend".to_string(),
    };
    let mut dialogs = UnavailableDialogs::because(reason.clone());
    assert_eq!(dialogs.reason(), &reason);
    let request = FileDialogRequest::default();
    let err = assert_err(dialogs.open_file(&request), "open must fail honestly");
    assert_eq!(err, reason);
    let err = assert_err(dialogs.save_file(&request), "save must fail honestly");
    assert_eq!(err, reason);
}

// The fallbacks are usable through the seam traits as `dyn` objects —
// the shape LinuxPlatform::discover wires when an integration is
// absent. (A `FilePick` variant is constructed here so the type's
// success shape is pinned too.)
#[test]
fn typed_fallbacks_satisfy_the_seam_traits() {
    let reason = PlatformError::NotCompiledForThisOs { crate_os: "linux" };
    let mut audio: Box<dyn AudioDevices> =
        Box::new(UnavailableAudioDevices::because(reason.clone()));
    let mut gpu: Box<dyn GpuAdapters> = Box::new(UnavailableGpuAdapters::because(reason.clone()));
    let mut dialogs: Box<dyn FileDialogs> = Box::new(UnavailableDialogs::because(reason));
    assert!(audio.enumerate().is_err());
    assert!(audio.poll().is_err());
    assert!(gpu.enumerate().is_err());
    assert!(dialogs.open_file(&FileDialogRequest::default()).is_err());
    // The success vocabulary exists and is distinct from errors:
    assert_eq!(FilePick::Cancelled, FilePick::Cancelled);
    assert_eq!(
        FilePick::Paths(vec!["/a".into()]),
        FilePick::Paths(vec!["/a".into()])
    );
    assert_ne!(FilePick::Cancelled, FilePick::Paths(Vec::new()));
}

// ---------------------------------------------------------------------------
// Capability negotiation.
// ---------------------------------------------------------------------------

#[test]
fn capability_classifies_and_converts_probe_errors() {
    assert!(Capability::Available.is_available());
    let wrong_os = Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "linux" });
    assert!(!wrong_os.is_available());

    // Each typed error maps to the reason class it proves.
    let cap =
        Capability::from_probe_error(&PlatformError::NotCompiledForThisOs { crate_os: "linux" });
    assert_eq!(cap, wrong_os);

    let cap = Capability::from_probe_error(&PlatformError::LibraryUnavailable {
        names: vec!["libpulse.so.0".to_string()],
        detail: "no such file".to_string(),
    });
    assert_eq!(
        cap,
        Capability::Unavailable(UnavailableReason::LibraryNotFound {
            names: vec!["libpulse.so.0".to_string()],
            detail: "no such file".to_string(),
        })
    );

    // A missing symbol is a version-skew form of "library not usable"
    // and keeps the symbol visible.
    let cap = Capability::from_probe_error(&PlatformError::SymbolMissing {
        library: "libpulse.so.0".to_string(),
        symbol: "pa_mainloop_new",
    });
    match cap {
        Capability::Unavailable(UnavailableReason::LibraryNotFound { names, detail }) => {
            assert_eq!(names, vec!["libpulse.so.0".to_string()]);
            assert!(
                detail.contains("pa_mainloop_new"),
                "detail must name the symbol: {detail}"
            );
        }
        other => unreachable!("symbol-missing must map to LibraryNotFound, got {other:?}"),
    }

    let cap = Capability::from_probe_error(&PlatformError::ServiceUnavailable {
        service: "PulseAudio server",
        detail: "connection refused".to_string(),
    });
    assert_eq!(
        cap,
        Capability::Unavailable(UnavailableReason::ServiceNotReachable {
            service: "PulseAudio server",
            detail: "connection refused".to_string(),
        })
    );

    // Every other error collapses to the generic service verdict,
    // stringified (never dropped).
    let cap = Capability::from_probe_error(&PlatformError::TimedOut {
        api: "pa_context_connect",
        deadline_ms: 2_000,
    });
    match cap {
        Capability::Unavailable(UnavailableReason::ServiceNotReachable { detail, .. }) => {
            assert!(
                detail.contains("pa_context_connect"),
                "detail must carry the api: {detail}"
            );
        }
        other => unreachable!("timeout must map to ServiceNotReachable, got {other:?}"),
    }
}

#[test]
fn unavailable_reasons_render_honest_strings() {
    let cases: [(UnavailableReason, &str); 4] = [
        (
            UnavailableReason::WrongOs { crate_os: "linux" },
            "this adapter carries the linux integration only",
        ),
        (
            UnavailableReason::LibraryNotFound {
                names: vec!["libpulse.so.0".to_string(), "libpulse.so".to_string()],
                detail: "not found".to_string(),
            },
            "runtime library not found (tried libpulse.so.0, libpulse.so): not found",
        ),
        (
            UnavailableReason::ServiceNotReachable {
                service: "PulseAudio server",
                detail: "refused".to_string(),
            },
            "PulseAudio server not reachable: refused",
        ),
        (
            UnavailableReason::NotYetImplemented {
                detail: "not probed by this adapter",
            },
            "not probed by this adapter",
        ),
    ];
    for (reason, want) in cases {
        let text = reason.to_string();
        assert!(text.contains(want), "expected {want:?} inside {text:?}");
    }
}

#[test]
fn feature_keys_are_the_stable_matrix_vocabulary() {
    assert_eq!(
        Feature::AudioEnumeration.key(),
        "platform.audio.enumeration"
    );
    assert_eq!(
        Feature::AudioNotifications.key(),
        "platform.audio.notifications"
    );
    assert_eq!(Feature::GpuEnumeration.key(), "platform.gpu.enumeration");
    assert_eq!(Feature::FileDialogs.key(), "platform.dialogs");
}

#[test]
fn platform_report_is_a_table_with_honest_defaults() {
    let mut report = PlatformReport::new();
    // An unprobed feature is honestly NotYetImplemented, never
    // assumed available.
    assert_eq!(
        report.get(Feature::AudioEnumeration),
        Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "feature was not probed by this adapter",
        })
    );
    report.set(Feature::AudioEnumeration, Capability::Available);
    assert_eq!(report.get(Feature::AudioEnumeration), Capability::Available);
    // set() replaces in place — the report is a table, not a log.
    report.set(
        Feature::AudioEnumeration,
        Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "linux" }),
    );
    let mut rows = 0;
    for (feature, capability) in report.iter() {
        assert_eq!(feature, Feature::AudioEnumeration);
        assert!(!capability.is_available());
        rows += 1;
    }
    assert_eq!(rows, 1, "set() must replace, not append");
    // Other features stay unprobed independently.
    assert!(!report.get(Feature::FileDialogs).is_available());
}

// ---------------------------------------------------------------------------
// Event plumbing: the bounded queue OS callbacks push into.
// ---------------------------------------------------------------------------

#[test]
fn event_queue_drains_fifo_and_empties() {
    let queue = DeviceEventQueue::new();
    let a = stable_device_id("sink-a");
    let b = stable_device_id("sink-b");
    queue.push(PlatformEvent::AudioDeviceAdded { id: a });
    queue.push(PlatformEvent::AudioDeviceRemoved { id: a });
    queue.push(PlatformEvent::AudioDefaultChanged { id: Some(b) });
    queue.push(PlatformEvent::AudioSetChanged);
    queue.push(PlatformEvent::AudioDeviceStateChanged {
        id: a,
        active: false,
    });
    assert_eq!(queue.len(), 5);
    assert!(!queue.is_empty());
    let drained = queue.drain();
    assert_eq!(
        drained,
        vec![
            PlatformEvent::AudioDeviceAdded { id: a },
            PlatformEvent::AudioDeviceRemoved { id: a },
            PlatformEvent::AudioDefaultChanged { id: Some(b) },
            PlatformEvent::AudioSetChanged,
            PlatformEvent::AudioDeviceStateChanged {
                id: a,
                active: false
            },
        ],
        "drain must be oldest-first"
    );
    assert!(queue.drain().is_empty());
    assert!(queue.is_empty());
}

#[test]
fn event_queue_is_bounded_and_counts_drops() {
    let queue = DeviceEventQueue::with_capacity(3);
    assert_eq!(queue.capacity(), 3);
    let a = stable_device_id("sink-a");
    for round in 0..6u64 {
        queue.push(PlatformEvent::AudioDeviceAdded { id: a });
        assert_eq!(queue.pushed(), round + 1, "pushed is monotonic");
    }
    // Six pushes into capacity three: three retained, three dropped
    // oldest-first.
    assert_eq!(queue.len(), 3);
    assert_eq!(queue.dropped(), 3);
    let drained = queue.drain();
    assert_eq!(drained.len(), 3, "the newest three survive");
    // The drop accounting stays readable after a drain.
    assert_eq!(queue.dropped(), 3);
    assert_eq!(queue.pushed(), 6);
}

#[test]
fn event_queue_capacity_is_clamped_to_one() {
    // A zero capacity is clamped up — never a silent black hole.
    let queue = DeviceEventQueue::with_capacity(0);
    assert_eq!(queue.capacity(), 1);
    assert!(queue.push(PlatformEvent::AudioSetChanged));
    assert!(
        !queue.push(PlatformEvent::AudioSetChanged),
        "second push must report the drop"
    );
    assert_eq!(queue.dropped(), 1);
    assert_eq!(queue.len(), 1);
}

#[test]
fn event_queue_is_thread_safe_for_callback_threads() {
    // Simulated OS-callback pressure: concurrent pushers from foreign
    // "callback threads" (the sanctioned test double — production
    // PulseAudio callbacks push from the mainloop thread).
    let queue = DeviceEventQueue::with_capacity(64);
    let handles: Vec<_> = (0..4)
        .map(|thread| {
            let queue = std::sync::Arc::clone(&queue);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    queue.push(PlatformEvent::AudioSetChanged);
                }
                let _ = thread;
            })
        })
        .collect();
    for handle in handles {
        assert!(
            handle.join().is_ok(),
            "the simulated callback thread must not panic"
        );
    }
    assert_eq!(queue.pushed(), 400);
    assert!(queue.len() <= 64, "the bound must hold under concurrency");
    assert_eq!(
        queue.dropped() + queue.len() as u64,
        400,
        "every push is accounted for"
    );
    queue.drain();
    assert!(queue.is_empty());
}

// ---------------------------------------------------------------------------
// Id minting: stable across calls, distinct across names, pinned to
// the FNV-1a-64 vector (cross-checked against an independent
// implementation — Python — so the constant is evidence, not a
// tautology).
// ---------------------------------------------------------------------------

#[test]
fn stable_device_id_is_deterministic_and_pinned() {
    assert_eq!(stable_device_id("a").handle(), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(
        stable_device_id("alsa_output.pci-0000_00_1f.3.analog-stereo").handle(),
        0x2e60_b52e_4138_dd9a
    );
    // A WASAPI-style endpoint id (the same minting serves all three
    // OS lanes).
    assert_eq!(
        stable_device_id("{0.0.0.00000000}.{e32f9c65-1234-4a55-bc86-8ed0d9d1c021}").handle(),
        0x5b40_ed9b_b4ce_1081
    );
    assert_eq!(stable_device_id("same"), stable_device_id("same"));
    assert_ne!(stable_device_id("sink-a"), stable_device_id("sink-b"));
    assert_ne!(stable_device_id(""), stable_device_id("a"));
}

// ---------------------------------------------------------------------------
// The GPU vocabulary: vendor labeling the report/UI relies on.
// ---------------------------------------------------------------------------

#[test]
fn gpu_adapter_info_labels_well_known_pci_vendors() {
    let info = GpuAdapterInfo {
        vendor_id: 0x8086,
        device_id: 0x9bc4,
        name: "DRM card0 (Intel vendor 0x8086 device 0x9bc4)".to_string(),
        dedicated_video_memory: None,
        is_software: false,
    };
    assert_eq!(info.vendor_name(), "Intel");
    let cases = [
        (0x1002u32, "AMD"),
        (0x10de, "NVIDIA"),
        (0x13b5, "Arm"),
        (0x5143, "Qualcomm"),
        (0x1af4, "Red Hat (virtio)"),
        (0x1234, "Bochs (QEMU)"),
        (0x0000, "unknown"),
        (0xdead, "unknown"),
    ];
    for (vendor, want) in cases {
        assert_eq!(info_label(vendor), want, "vendor {vendor:#06x}");
    }
}

fn info_label(vendor: u32) -> &'static str {
    let info = GpuAdapterInfo {
        vendor_id: vendor,
        device_id: 0,
        name: String::new(),
        dedicated_video_memory: None,
        is_software: false,
    };
    info.vendor_name()
}

// ---------------------------------------------------------------------------
// Error vocabulary: Display is the human form of the typed truth.
// ---------------------------------------------------------------------------

#[test]
fn platform_errors_render_their_details() {
    let cases: [(PlatformError, &str); 8] = [
        (
            PlatformError::NotCompiledForThisOs { crate_os: "linux" },
            "this build carries no linux integration",
        ),
        (
            PlatformError::LibraryUnavailable {
                names: vec!["libpulse.so.0".to_string()],
                detail: "cannot open shared object file".to_string(),
            },
            "required runtime library unavailable (tried libpulse.so.0): \
             cannot open shared object file",
        ),
        (
            PlatformError::SymbolMissing {
                library: "libpulse.so.0".to_string(),
                symbol: "pa_mainloop_new",
            },
            "symbol \"pa_mainloop_new\" missing from libpulse.so.0",
        ),
        (
            PlatformError::ServiceUnavailable {
                service: "PulseAudio server",
                detail: "refused".to_string(),
            },
            "PulseAudio server unavailable: refused",
        ),
        (
            PlatformError::CallFailed {
                api: "pa_mainloop_prepare",
                code: Some(-1),
                detail: "refused".to_string(),
            },
            "pa_mainloop_prepare failed with status -1: refused",
        ),
        (
            PlatformError::CallFailed {
                api: "pa_mainloop_new",
                code: None,
                detail: "NULL".to_string(),
            },
            "pa_mainloop_new failed: NULL",
        ),
        (
            PlatformError::TimedOut {
                api: "pa_context_connect",
                deadline_ms: 2_000,
            },
            "pa_context_connect did not answer within the 2000 ms deadline",
        ),
        (
            PlatformError::SpawnFailed {
                program: "zenity".to_string(),
                detail: "not executable".to_string(),
            },
            "could not execute zenity: not executable",
        ),
    ];
    for (err, want) in cases {
        let text = err.to_string();
        assert!(text.contains(want), "expected {want:?} inside {text:?}");
    }
}
