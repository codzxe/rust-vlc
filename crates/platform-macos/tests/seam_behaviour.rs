//! Seam-behaviour acceptance tests for `platform-macos` (card
//! 7.A3): the typed fallbacks, capability negotiation, event-queue
//! plumbing and id minting — every assertion host-runnable on any
//! machine (the OS integrations themselves are compile-verified via
//! the two Apple cross-check targets only — no macOS SDK in this
//! sandbox).
//!
//! This suite runs on a Linux host, which is exactly the "wrong-OS
//! build" case the crate's honesty contract covers: every seam must
//! be the typed fallback and [`MacOsPlatform::discover`] must say
//! `WrongOs` for every feature — the compile-time cfg story
//! verified at runtime from the outside. Real Apple-hardware
//! behavior is `[VERIFY]` (the FFI surface is compile-verified via
//! `cargo check --target aarch64-apple-darwin` and
//! `x86_64-apple-darwin` only).

use platform_macos::MacOsPlatform;
use platform_macos::audio::{AudioDevices, UnavailableAudioDevices, stable_device_id};
use platform_macos::capability::{Capability, Feature, PlatformReport, UnavailableReason};
use platform_macos::dialog::{
    FileDialogRequest, FileDialogs, FileFilter, FilePick, UnavailableDialogs,
};
use platform_macos::error::PlatformError;
use platform_macos::events::{DeviceEventQueue, PlatformEvent};
use platform_macos::gpu::{GpuAdapterInfo, GpuAdapters, UnavailableGpuAdapters};

use testkit::assert_err;

// ---------------------------------------------------------------------------
// Typed fallbacks: every method answers the reason, never a fake
// success (the graceful-degradation contract).
// ---------------------------------------------------------------------------

#[test]
fn unavailable_audio_devices_returns_the_typed_reason() {
    let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
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
    let reason = PlatformError::CallFailed {
        api: "MTLCopyAllDevices",
        code: None,
        detail: "the Metal framework returned no device array".to_string(),
    };
    let mut adapters = UnavailableGpuAdapters::because(reason.clone());
    assert_eq!(adapters.reason(), &reason);
    let err = assert_err(adapters.enumerate(), "enumerate must fail honestly");
    assert_eq!(err, reason);
}

#[test]
fn unavailable_dialogs_returns_the_typed_reason() {
    let reason = PlatformError::ServiceUnavailable {
        service: "Objective-C runtime",
        detail: "the class NSOpenPanel is not present (framework not linked?)".to_string(),
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
// the shape MacOsPlatform::discover wires when an integration is
// absent.
#[test]
fn typed_fallbacks_satisfy_the_seam_traits() {
    let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
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
        FilePick::Paths(vec!["/Users/demo/a.mkv".into()]),
        FilePick::Paths(vec!["/Users/demo/a.mkv".into()])
    );
    assert_ne!(FilePick::Cancelled, FilePick::Paths(Vec::new()));
}

// ---------------------------------------------------------------------------
// Capability negotiation.
// ---------------------------------------------------------------------------

#[test]
fn capability_classifies_and_converts_probe_errors() {
    assert!(Capability::Available.is_available());
    let wrong_os = Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "macos" });
    assert!(!wrong_os.is_available());

    // Each typed error maps to the reason class it proves.
    let cap =
        Capability::from_probe_error(&PlatformError::NotCompiledForThisOs { crate_os: "macos" });
    assert_eq!(cap, wrong_os);

    // An AppKit refusal is the service-not-reachable class,
    // stringified.
    let cap = Capability::from_probe_error(&PlatformError::ServiceUnavailable {
        service: "Objective-C runtime",
        detail: "the class NSOpenPanel is not present".to_string(),
    });
    match cap {
        Capability::Unavailable(UnavailableReason::ServiceNotReachable { service, .. }) => {
            assert_eq!(service, "Objective-C runtime");
        }
        other => unreachable!("service failure must map to ServiceNotReachable, got {other:?}"),
    }

    // A HAL failure keeps its API visible in the detail.
    let cap = Capability::from_probe_error(&PlatformError::CallFailed {
        api: "AudioObjectGetPropertyDataSize",
        code: Some(-50),
        detail: "paramErr".to_string(),
    });
    match cap {
        Capability::Unavailable(UnavailableReason::ServiceNotReachable { detail, .. }) => {
            assert!(
                detail.contains("AudioObjectGetPropertyDataSize"),
                "detail must carry the api: {detail}"
            );
        }
        other => unreachable!("call failure must map to ServiceNotReachable, got {other:?}"),
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
        Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "macos" }),
    );
    let mut rows = 0;
    for (feature, capability) in report.iter() {
        assert_eq!(feature, Feature::AudioEnumeration);
        assert!(!capability.is_available());
        rows += 1;
    }
    assert_eq!(rows, 1, "set() must replace, not append");
    assert!(!report.get(Feature::FileDialogs).is_available());
}

// ---------------------------------------------------------------------------
// Event plumbing: the bounded queue the CoreAudio listeners push
// into (simulated inputs here — the production path is identical).
// ---------------------------------------------------------------------------

#[test]
fn event_queue_drains_fifo_and_empties() {
    let queue = DeviceEventQueue::new();
    let a = stable_device_id("AppleHDAEngineOutput:0x0,Output,0x1,0x0");
    let b = stable_device_id("AirPodsPro-UID-0x123456789abcdef0");
    queue.push(PlatformEvent::AudioDeviceAdded { id: a });
    queue.push(PlatformEvent::AudioDeviceRemoved { id: a });
    queue.push(PlatformEvent::AudioDefaultChanged { id: Some(b) });
    queue.push(PlatformEvent::AudioSetChanged);
    queue.push(PlatformEvent::AudioDeviceStateChanged {
        id: a,
        active: false,
    });
    assert_eq!(queue.len(), 5);
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
    let a = stable_device_id("device-a");
    for round in 0..6u64 {
        queue.push(PlatformEvent::AudioDeviceAdded { id: a });
        assert_eq!(queue.pushed(), round + 1, "pushed is monotonic");
    }
    assert_eq!(queue.len(), 3);
    assert_eq!(queue.dropped(), 3);
    let drained = queue.drain();
    assert_eq!(drained.len(), 3, "the newest three survive");
    assert_eq!(queue.dropped() + queue.len() as u64, 3);
}

#[test]
fn event_queue_capacity_is_clamped_to_one() {
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
fn event_queue_is_thread_safe_for_listener_threads() {
    // Simulated CoreAudio-listener pressure: concurrent pushers from
    // foreign "HAL threads" (the sanctioned test double — the
    // production listeners push from a HAL-owned thread).
    let queue = DeviceEventQueue::with_capacity(64);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let queue = std::sync::Arc::clone(&queue);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    queue.push(PlatformEvent::AudioSetChanged);
                }
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
}

// ---------------------------------------------------------------------------
// Id minting: stable across calls, distinct across names, pinned to
// the same FNV-1a-64 vector as the other two lanes (cross-lane id
// stability is the point of the shared minting).
// ---------------------------------------------------------------------------

#[test]
fn stable_device_id_is_deterministic_and_pinned() {
    assert_eq!(stable_device_id("a").handle(), 0xaf63_dc4c_8601_ec8c);
    // A CoreAudio-style device UID.
    assert_eq!(
        stable_device_id("AppleHDAEngineOutput:0x0,Output,0x1,0x0").handle(),
        0x1b44_b68b_bc0c_5287
    );
    assert_eq!(stable_device_id("same"), stable_device_id("same"));
    assert_ne!(stable_device_id("device-a"), stable_device_id("device-b"));
    assert_ne!(stable_device_id(""), stable_device_id("a"));
}

// ---------------------------------------------------------------------------
// The GPU vocabulary: Metal has no software adapter; the vendor
// field stays 0 (no PCI notion) and the label table still answers.
// ---------------------------------------------------------------------------

#[test]
fn gpu_adapter_info_answers_the_metal_shape() {
    // A Metal device: vendor 0 (no PCI notion), registryID in the
    // device field, VRAM reported, hardware by construction.
    let apple_gpu = GpuAdapterInfo {
        vendor_id: 0,
        device_id: 0x1234_5678,
        name: "Apple M2".to_string(),
        dedicated_video_memory: Some(15_000_000_000),
        is_software: false,
    };
    assert!(!apple_gpu.is_software);
    assert_eq!(
        apple_gpu.vendor_name(),
        "unknown",
        "vendor 0 is not in the PCI table"
    );

    // The label table itself still classifies the well-known PCI ids
    // (shared with the other lanes — pinned so they cannot drift).
    let cases = [
        (0x8086u32, "Intel"),
        (0x1002u32, "AMD"),
        (0x10deu32, "NVIDIA"),
        (0x13b5u32, "Arm"),
        (0x0000u32, "unknown"),
    ];
    for (vendor, want) in cases {
        let info = GpuAdapterInfo {
            vendor_id: vendor,
            device_id: 0,
            name: String::new(),
            dedicated_video_memory: None,
            is_software: false,
        };
        assert_eq!(info.vendor_name(), want, "vendor {vendor:#06x}");
    }
}

// ---------------------------------------------------------------------------
// Error vocabulary: Display is the human form of the typed truth.
// ---------------------------------------------------------------------------

#[test]
fn platform_errors_render_their_details() {
    let cases: [(PlatformError, &str); 4] = [
        (
            PlatformError::NotCompiledForThisOs { crate_os: "macos" },
            "this build carries no macos integration",
        ),
        (
            PlatformError::CallFailed {
                api: "AudioObjectAddPropertyListener",
                code: Some(-561_017_433),
                detail: "the HAL refused the devices listener".to_string(),
            },
            "AudioObjectAddPropertyListener failed with status -561017433",
        ),
        (
            PlatformError::InvalidEncoding {
                api: "NSString UTF8String",
            },
            "NSString UTF8String returned a string that is not valid Unicode",
        ),
        (
            PlatformError::SymbolMissing {
                library: "libobjc.A.dylib".to_string(),
                symbol: "objc_msgSend",
            },
            "symbol \"objc_msgSend\" missing from libobjc.A.dylib",
        ),
    ];
    for (err, want) in cases {
        let text = err.to_string();
        assert!(text.contains(want), "expected {want:?} inside {text:?}");
    }
}

// ---------------------------------------------------------------------------
// The wrong-OS build: discover() on this (non-macOS) host wires
// every typed fallback and reports WrongOs — the compile-time cfg
// story verified from the outside.
// ---------------------------------------------------------------------------

#[test]
fn discover_on_a_foreign_host_reports_wrong_os_everywhere() {
    let mut platform = MacOsPlatform::discover();
    let report = platform.report();
    for feature in [
        Feature::AudioEnumeration,
        Feature::AudioNotifications,
        Feature::GpuEnumeration,
        Feature::FileDialogs,
    ] {
        assert_eq!(
            report.get(feature),
            Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "macos" }),
            "a non-macOS build must say WrongOs for {feature:?}"
        );
    }
    // The seams refuse with exactly the same typed reason.
    let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
    let err = assert_err(platform.audio().enumerate(), "audio must refuse");
    assert_eq!(err, reason);
    let err = assert_err(platform.gpu().enumerate(), "gpu must refuse");
    assert_eq!(err, reason);
    let err = assert_err(
        platform.dialogs().open_file(&FileDialogRequest::default()),
        "dialogs must refuse",
    );
    assert_eq!(err, reason);
}

// The dialog request vocabulary the seam speaks (shared shape with
// the other lanes — pinned here so the lanes cannot drift).
#[test]
fn file_dialog_request_carries_the_os_neutral_shape() {
    let request = FileDialogRequest::default();
    assert!(request.title.is_none());
    assert!(request.filters.is_empty());
    assert!(request.default_dir.is_none());
    assert!(!request.multiple);

    let filter = FileFilter::new("Video files", &["*.mkv", "*.mp4"]);
    assert_eq!(filter.label, "Video files");
    assert_eq!(
        filter.patterns,
        vec!["*.mkv".to_string(), "*.mp4".to_string()]
    );
}
