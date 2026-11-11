//! Seam-behaviour acceptance tests for `platform-windows` (card
//! 7.A1): the typed fallbacks, capability negotiation, event-queue
//! plumbing and id minting — every assertion host-runnable on any
//! machine.
//!
//! This suite runs on a Linux host, which is exactly the "wrong-OS
//! build" case the crate's honesty contract covers: every seam must
//! be the typed fallback and [`WindowsPlatform::discover`] must say
//! `WrongOs` for every feature — the compile-time cfg story
//! verified at runtime from the outside. Real Windows behavior is
//! `[VERIFY]` (no Windows SDK in this sandbox; the FFI surface is
//! compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc` only).

use platform_windows::WindowsPlatform;
use platform_windows::audio::{AudioDevices, UnavailableAudioDevices, stable_device_id};
use platform_windows::capability::{Capability, Feature, PlatformReport, UnavailableReason};
use platform_windows::dialog::{
    FileDialogRequest, FileDialogs, FileFilter, FilePick, UnavailableDialogs,
};
use platform_windows::error::PlatformError;
use platform_windows::events::{DeviceEventQueue, PlatformEvent};
use platform_windows::gpu::{GpuAdapterInfo, GpuAdapters, UnavailableGpuAdapters};

use testkit::assert_err;

// ---------------------------------------------------------------------------
// Typed fallbacks: every method answers the reason, never a fake
// success (the graceful-degradation contract).
// ---------------------------------------------------------------------------

#[test]
fn unavailable_audio_devices_returns_the_typed_reason() {
    let reason = PlatformError::NotCompiledForThisOs {
        crate_os: "windows",
    };
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
        api: "CreateDXGIFactory1",
        code: Some(0x8007_000E),
        detail: "no D3D runtime".to_string(),
    };
    let mut adapters = UnavailableGpuAdapters::because(reason.clone());
    assert_eq!(adapters.reason(), &reason);
    let err = assert_err(adapters.enumerate(), "enumerate must fail honestly");
    assert_eq!(err, reason);
}

#[test]
fn unavailable_dialogs_returns_the_typed_reason() {
    let reason = PlatformError::CallFailed {
        api: "CoInitializeEx",
        code: Some(0x8001_0106),
        detail: "the thread already joined the multithreaded apartment".to_string(),
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
// the shape WindowsPlatform::discover wires when an integration is
// absent.
#[test]
fn typed_fallbacks_satisfy_the_seam_traits() {
    let reason = PlatformError::NotCompiledForThisOs {
        crate_os: "windows",
    };
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
        FilePick::Paths(vec!["C:\\a".into()]),
        FilePick::Paths(vec!["C:\\a".into()])
    );
    assert_ne!(FilePick::Cancelled, FilePick::Paths(Vec::new()));
}

// ---------------------------------------------------------------------------
// Capability negotiation.
// ---------------------------------------------------------------------------

#[test]
fn capability_classifies_and_converts_probe_errors() {
    assert!(Capability::Available.is_available());
    let wrong_os = Capability::Unavailable(UnavailableReason::WrongOs {
        crate_os: "windows",
    });
    assert!(!wrong_os.is_available());

    // Each typed error maps to the reason class it proves.
    let cap = Capability::from_probe_error(&PlatformError::NotCompiledForThisOs {
        crate_os: "windows",
    });
    assert_eq!(cap, wrong_os);

    // A COM failure is the service-not-reachable class, stringified.
    let cap = Capability::from_probe_error(&PlatformError::CallFailed {
        api: "IMMDeviceEnumerator::EnumAudioEndpoints",
        code: Some(0x8000_4005),
        detail: "E_FAIL".to_string(),
    });
    match cap {
        Capability::Unavailable(UnavailableReason::ServiceNotReachable { detail, .. }) => {
            assert!(
                detail.contains("EnumAudioEndpoints"),
                "detail must carry the api: {detail}"
            );
        }
        other => unreachable!("call failure must map to ServiceNotReachable, got {other:?}"),
    }

    // The Windows lanes load no libraries at runtime (ole32/dxgi are
    // link-time OS components), but the LibraryNotFound class is
    // still reachable through the shared vocabulary.
    let cap = Capability::from_probe_error(&PlatformError::LibraryUnavailable {
        names: vec!["dxgi.dll".to_string()],
        detail: "not loadable".to_string(),
    });
    assert_eq!(
        cap,
        Capability::Unavailable(UnavailableReason::LibraryNotFound {
            names: vec!["dxgi.dll".to_string()],
            detail: "not loadable".to_string(),
        })
    );
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
        Capability::Unavailable(UnavailableReason::WrongOs {
            crate_os: "windows",
        }),
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
// Event plumbing: the bounded queue the WASAPI callbacks push into
// (simulated inputs here — the production path is identical).
// ---------------------------------------------------------------------------

#[test]
fn event_queue_drains_fifo_and_empties() {
    let queue = DeviceEventQueue::new();
    let a = stable_device_id("{0.0.0.00000000}.{aaaa-bbbb-cccc-dddd}");
    let b = stable_device_id("{0.0.0.00000000}.{eeee-ffff-0000-1111}");
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
    let a = stable_device_id("endpoint-a");
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
fn event_queue_is_thread_safe_for_callback_threads() {
    // Simulated WASAPI-callback pressure: concurrent pushers from
    // foreign "callback threads" (the sanctioned test double — the
    // production callbacks push from the COM dispatch thread).
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
    // A WASAPI-style endpoint id.
    assert_eq!(
        stable_device_id("{0.0.0.00000000}.{e32f9c65-1234-4a55-bc86-8ed0d9d1c021}").handle(),
        0x5b40_ed9b_b4ce_1081
    );
    assert_eq!(stable_device_id("same"), stable_device_id("same"));
    assert_ne!(
        stable_device_id("endpoint-a"),
        stable_device_id("endpoint-b")
    );
    assert_ne!(stable_device_id(""), stable_device_id("a"));
}

// ---------------------------------------------------------------------------
// The GPU vocabulary: WARP detection + vendor labeling.
// ---------------------------------------------------------------------------

#[test]
fn gpu_adapter_info_labels_warp_and_well_known_vendors() {
    // The WARP software adapter (Microsoft Basic Render Driver).
    let warp = GpuAdapterInfo {
        vendor_id: 0x1414,
        device_id: 0x008C,
        name: "Microsoft Basic Render Driver".to_string(),
        dedicated_video_memory: Some(0),
        is_software: true,
    };
    assert!(warp.is_software);
    let cases = [
        (0x8086u32, "Intel"),
        (0x1002u32, "AMD"),
        (0x10deu32, "NVIDIA"),
        (0x1414u32, "unknown"), // Microsoft is not in the PCI label table
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
            PlatformError::NotCompiledForThisOs {
                crate_os: "windows",
            },
            "this build carries no windows integration",
        ),
        (
            // HRESULTs are stored as their signed i32 form (see
            // `com::RPC_E_CHANGED_MODE`), exactly as the real
            // CoInitializeEx path reports them.
            PlatformError::CallFailed {
                api: "CoInitializeEx",
                code: Some(0x8001_0106u32 as i32 as i64),
                detail: "changed mode".to_string(),
            },
            "CoInitializeEx failed with status -2147417850: changed mode",
        ),
        (
            PlatformError::InvalidEncoding {
                api: "wide-string conversion",
            },
            "wide-string conversion returned a string that is not valid Unicode",
        ),
        (
            PlatformError::SpawnFailed {
                program: "lumen-cid-open thread".to_string(),
                detail: "resource unavailable".to_string(),
            },
            "could not execute lumen-cid-open thread: resource unavailable",
        ),
    ];
    for (err, want) in cases {
        let text = err.to_string();
        assert!(text.contains(want), "expected {want:?} inside {text:?}");
    }
}

// ---------------------------------------------------------------------------
// The wrong-OS build: discover() on this (non-Windows) host wires
// every typed fallback and reports WrongOs — the compile-time cfg
// story verified from the outside.
// ---------------------------------------------------------------------------

#[test]
fn discover_on_a_foreign_host_reports_wrong_os_everywhere() {
    let mut platform = WindowsPlatform::discover();
    let report = platform.report();
    for feature in [
        Feature::AudioEnumeration,
        Feature::AudioNotifications,
        Feature::GpuEnumeration,
        Feature::FileDialogs,
    ] {
        assert_eq!(
            report.get(feature),
            Capability::Unavailable(UnavailableReason::WrongOs {
                crate_os: "windows"
            }),
            "a non-Windows build must say WrongOs for {feature:?}"
        );
    }
    // The seams refuse with exactly the same typed reason.
    let reason = PlatformError::NotCompiledForThisOs {
        crate_os: "windows",
    };
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
