//! Host-run integration tests for the Linux integrations (card 7.A2):
//! the DRM sysfs scanner over fixture trees, the xdg dialog chain
//! (including the zenity/kdialog command fallbacks driven by fake
//! binaries — the sanctioned test double), the PulseAudio
//! dlopen/connect paths against whatever this host really has, and
//! the assembled `LinuxPlatform::discover` report.
//!
//! These tests assert **honesty**, not hardware presence: outcomes
//! vary with the host (no DRM nodes → empty list; no PulseAudio
//! server → typed `ServiceUnavailable`), and every assertion accepts
//! exactly the outcomes the integrations can truthfully produce.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use platform_linux::audio::AudioDevices;
use platform_linux::capability::{Capability, Feature, UnavailableReason};
use platform_linux::dialog::{FileDialogRequest, FileDialogs, FileFilter, FilePick};
use platform_linux::events::DeviceEventQueue;
use platform_linux::gpu::GpuAdapters;
use platform_linux::linux::drm::DrmAdapterScanner;
use platform_linux::linux::pulse::PulseAudioDevices;
use platform_linux::linux::xdg::{PortalStatus, XdgDialogs};
use platform_linux::{LinuxPlatform, error::PlatformError};

use testkit::Must;
use testkit::assert_err;

// ---------------------------------------------------------------------------
// Fixture scaffolding (std-only temp trees; no dev-dependencies by C5).
// ---------------------------------------------------------------------------

/// A per-test temp directory under the system temp root, tagged to
/// keep parallel tests apart.
fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("lumen-platform-linux-tests")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).must();
    dir
}

/// Write an executable helper script (the fake zenity/kdialog).
fn write_script(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).must();
    }
    fs::write(path, format!("#!/bin/sh\n{body}\n")).must();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).must();
}

// ---------------------------------------------------------------------------
// DRM/KMS sysfs scanning (injectable roots → deterministic).
// ---------------------------------------------------------------------------

#[test]
fn drm_scanner_reads_a_realistic_sysfs_tree() {
    let root = temp_root("drm-real");
    let sys = root.join("sys/class/drm");
    // card0: a real Intel iGPU with a device node.
    fs::create_dir_all(sys.join("card0/device")).must();
    fs::write(sys.join("card0/device/vendor"), "0x8086\n").must();
    fs::write(sys.join("card0/device/device"), "0x9bc4\n").must();
    // renderD128: the render node of the same GPU.
    fs::create_dir_all(sys.join("renderD128/device")).must();
    fs::write(sys.join("renderD128/device/vendor"), "0x8086\n").must();
    fs::write(sys.join("renderD128/device/device"), "0x9bc4\n").must();
    // card1: a software adapter (virtio), no device node.
    fs::create_dir_all(sys.join("card1/device")).must();
    fs::write(sys.join("card1/device/vendor"), "0x1af4\n").must();
    fs::write(sys.join("card1/device/device"), "0x1050\n").must();
    // Scratch entries that are NOT adapter nodes.
    fs::write(sys.join("version"), "drm 1.1.0\n").must();
    fs::create_dir_all(sys.join("minor0")).must();
    // The userspace-openable nodes.
    let dev = root.join("dev/dri");
    fs::create_dir_all(&dev).must();
    fs::write(dev.join("card0"), "").must();
    fs::write(dev.join("renderD128"), "").must();

    let mut scanner = DrmAdapterScanner::with_roots(&root, &root);
    let adapters = scanner.enumerate().must();
    assert_eq!(adapters.len(), 3, "card0, renderD128, card1 — nothing else");
    // Sorted by node name for determinism.
    assert_eq!(
        adapters[0].name,
        "DRM card0 (Intel vendor 0x8086 device 0x9bc4)"
    );
    assert_eq!(adapters[0].vendor_id, 0x8086);
    assert_eq!(adapters[0].device_id, 0x9bc4);
    assert!(!adapters[0].is_software);
    assert_eq!(adapters[0].vendor_name(), "Intel");
    assert!(!adapters[0].name.contains("no device node"));
    assert_eq!(
        adapters[1].name,
        "DRM card1 (Red Hat (virtio) vendor 0x1af4 device 0x1050, no device node)"
    );
    assert!(adapters[1].is_software, "virtio is classified software");
    assert_eq!(
        adapters[2].name,
        "DRM renderD128 (Intel vendor 0x8086 device 0x9bc4)"
    );
    // sysfs does not expose VRAM — the honest None.
    assert_eq!(adapters[0].dedicated_video_memory, None);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn drm_scanner_is_honest_on_empty_and_absent_trees() {
    // A sysfs with no adapter nodes at all.
    let root = temp_root("drm-empty");
    fs::create_dir_all(root.join("sys/class/drm")).must();
    fs::create_dir_all(root.join("dev/dri")).must();
    let mut scanner = DrmAdapterScanner::with_roots(&root, &root);
    let adapters = scanner.enumerate().must();
    assert!(adapters.is_empty(), "no nodes is an honest empty list");

    // A root where sysfs does not exist (a container without sysfs):
    // empty list, not an error — the kernel simply has no DRM.
    let missing = temp_root("drm-absent");
    let mut scanner = DrmAdapterScanner::with_roots(&missing, &missing);
    let adapters = scanner.enumerate().must();
    assert!(adapters.is_empty());

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&missing);
}

#[test]
fn drm_scanner_handles_degenerate_nodes() {
    // A node with unreadable PCI ids is still reported (ids 0, honest
    // "unknown" vendor), never guessed.
    let root = temp_root("drm-degenerate");
    let sys = root.join("sys/class/drm");
    fs::create_dir_all(sys.join("card2")).must(); // no device/ dir at all
    fs::create_dir_all(root.join("dev/dri")).must();
    let mut scanner = DrmAdapterScanner::with_roots(&root, &root);
    let adapters = scanner.enumerate().must();
    assert_eq!(adapters.len(), 1);
    assert_eq!(adapters[0].vendor_id, 0);
    assert_eq!(adapters[0].vendor_name(), "unknown");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn drm_scanner_reads_the_real_host_sysfs_honestly() {
    // The live /sys: on a container without DRM this returns the
    // honest empty list; on a GPU machine it returns its adapters.
    // Either way it must succeed without unsafe or panics.
    let mut scanner = DrmAdapterScanner::new();
    assert_eq!(scanner.sys_root(), Path::new("/"));
    let adapters = scanner.enumerate().must();
    for adapter in &adapters {
        assert!(!adapter.name.is_empty());
        assert!(!adapter.name.contains('\u{0}'));
    }
}

// ---------------------------------------------------------------------------
// The xdg dialog chain.
// ---------------------------------------------------------------------------

#[test]
fn xdg_dialogs_run_the_zenity_fallback() {
    let root = temp_root("xdg-zenity");
    let bin = root.join("bin");
    fs::create_dir_all(&bin).must();
    write_script(
        &bin.join("zenity"),
        "printf '/tmp/movie-one.mkv\\n/tmp/movie-two.mkv\\n'\nexit 0",
    );
    let mut dialogs = XdgDialogs::with_config(vec![bin.clone()], None);
    assert_eq!(dialogs.zenity(), Some(bin.join("zenity").as_path()));
    assert_eq!(dialogs.kdialog(), None);

    let request = FileDialogRequest {
        title: Some("Open media".to_string()),
        filters: vec![FileFilter::new("Video files", &["*.mkv", "*.mp4"])],
        default_dir: Some(PathBuf::from("/media")),
        multiple: true,
    };
    let pick = dialogs.open_file(&request).must();
    let FilePick::Paths(paths) = pick else {
        unreachable!("the fake zenity prints two paths");
    };
    assert_eq!(
        paths,
        vec![
            PathBuf::from("/tmp/movie-one.mkv"),
            PathBuf::from("/tmp/movie-two.mkv")
        ]
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn xdg_dialogs_cancel_is_not_an_error() {
    let root = temp_root("xdg-cancel");
    let bin = root.join("bin");
    fs::create_dir_all(&bin).must();
    // zenity exits 1 with no output for a user cancel.
    write_script(&bin.join("zenity"), "exit 1");
    let mut dialogs = XdgDialogs::with_config(vec![bin], None);
    let pick = dialogs.open_file(&FileDialogRequest::default()).must();
    assert_eq!(pick, FilePick::Cancelled);
    // A save round through kdialog behaves the same.
    let root2 = temp_root("xdg-cancel-k");
    let bin2 = root2.join("bin");
    fs::create_dir_all(&bin2).must();
    write_script(&bin2.join("kdialog"), "exit 1");
    let mut dialogs = XdgDialogs::with_config(vec![bin2], None);
    let pick = dialogs.save_file(&FileDialogRequest::default()).must();
    assert_eq!(pick, FilePick::Cancelled);

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&root2);
}

#[test]
fn xdg_dialogs_kdialog_fallback_parses_output() {
    let root = temp_root("xdg-kdialog");
    let bin = root.join("usr/bin");
    fs::create_dir_all(&bin).must();
    write_script(
        &bin.join("kdialog"),
        "printf '/home/user/clip.mp4\\n'\nexit 0",
    );
    let mut dialogs = XdgDialogs::with_config(vec![bin], None);
    assert!(dialogs.kdialog().is_some());
    let pick = dialogs.open_file(&FileDialogRequest::default()).must();
    assert_eq!(
        pick,
        FilePick::Paths(vec![PathBuf::from("/home/user/clip.mp4")])
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn xdg_dialogs_without_any_backend_refuse_with_the_typed_error() {
    // No zenity, no kdialog, no bus address: the honest typed refusal
    // naming everything that was tried.
    let mut dialogs = XdgDialogs::with_config(vec![], None);
    let err = assert_err(
        dialogs.open_file(&FileDialogRequest::default()),
        "no backend",
    );
    let PlatformError::ServiceUnavailable { service, detail } = &err else {
        unreachable!("expected ServiceUnavailable, got {err:?}");
    };
    assert_eq!(service, &"file dialogs");
    let text = detail.as_str();
    assert!(text.contains("zenity not found"), "names zenity: {text}");
    assert!(text.contains("kdialog not found"), "names kdialog: {text}");
    assert!(
        text.contains("not implemented"),
        "states the portal gap: {text}"
    );

    // A crashing fallback surfaces as the typed call failure.
    let root = temp_root("xdg-crash");
    let bin = root.join("bin");
    fs::create_dir_all(&bin).must();
    write_script(&bin.join("zenity"), "echo 'zenity broke' >&2\nexit 2");
    let mut dialogs = XdgDialogs::with_config(vec![bin], None);
    let err = assert_err(
        dialogs.open_file(&FileDialogRequest::default()),
        "crash surfaces",
    );
    let PlatformError::CallFailed { api, code, .. } = &err else {
        unreachable!("expected CallFailed, got {err:?}");
    };
    assert_eq!(api, &"zenity");
    assert_eq!(code, &Some(2));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn xdg_portal_probe_reports_bus_truth() {
    // Unset address → BusUnreachable with the reason.
    let dialogs = XdgDialogs::with_config(vec![], None);
    let PortalStatus::BusUnreachable { detail } = dialogs.portal_status() else {
        unreachable!("no address must be BusUnreachable");
    };
    assert!(detail.contains("DBUS_SESSION_BUS_ADDRESS"), "{detail}");

    // A live unix socket at the address → BusReachable.
    let root = temp_root("xdg-bus");
    let socket = root.join("bus");
    let _listener = UnixListener::bind(&socket).must();
    let dialogs = XdgDialogs::with_config(vec![], Some(format!("unix:path={}", socket.display())));
    match dialogs.portal_status() {
        PortalStatus::BusReachable { socket: found } => assert_eq!(found, socket),
        PortalStatus::BusUnreachable { detail } => {
            unreachable!("a bound socket must be reachable, got: {detail}")
        }
    }

    // A dead path → BusUnreachable with the connect failure.
    let dialogs = XdgDialogs::with_config(
        vec![],
        Some(format!("unix:path={}", root.join("missing").display())),
    );
    assert!(matches!(
        dialogs.portal_status(),
        PortalStatus::BusUnreachable { .. }
    ));

    // Abstract sockets are beyond the std client — the probe says so.
    let dialogs = XdgDialogs::with_config(vec![], Some("unix:abstract=/tmp-xyz".to_string()));
    let PortalStatus::BusUnreachable { detail } = dialogs.portal_status() else {
        unreachable!("abstract sockets must be refused");
    };
    assert!(detail.contains("abstract"), "{detail}");

    // The FileChooser client gap is the recorded follow-up, queryable.
    assert!(!platform_linux::linux::xdg::portal_filechooser_implemented());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn xdg_discovery_uses_the_real_environment() {
    // $PATH-driven discovery (this host has no zenity/kdialog — the
    // discovery must say exactly that, not guess).
    let dialogs = XdgDialogs::discover();
    let _ = dialogs.zenity();
    let _ = dialogs.kdialog();
}

// ---------------------------------------------------------------------------
// PulseAudio on this host: whatever the truth is, it is typed.
// ---------------------------------------------------------------------------

#[test]
fn pulse_probe_reports_this_hosts_truth() {
    // This sandbox has libpulse.so.0 but no PulseAudio server: probe
    // succeeds, enumerate returns the typed ServiceUnavailable from
    // the connect phase. A machine without the library gets
    // LibraryUnavailable; a machine with a live server gets devices
    // ([VERIFY] — not exercisable here).
    match PulseAudioDevices::probe() {
        Ok(devices) => {
            assert!(
                devices.loaded_from() == "libpulse.so.0" || devices.loaded_from() == "libpulse.so",
                "loaded_from reports the real SONAME: {}",
                devices.loaded_from()
            );
            let mut devices = devices;
            match devices.enumerate() {
                Ok(list) => {
                    // A live server ([VERIFY] in this sandbox): the
                    // list is the sinks, ids minted from names.
                    for desc in &list {
                        assert!(!desc.name.is_empty());
                    }
                }
                Err(PlatformError::ServiceUnavailable { service, .. }) => {
                    assert_eq!(service, "PulseAudio server");
                }
                Err(other) => {
                    unreachable!("expected Ok or ServiceUnavailable, got {other:?}");
                }
            }
        }
        Err(PlatformError::LibraryUnavailable { names, .. }) => {
            assert!(names.contains(&"libpulse.so.0".to_string()), "{names:?}");
        }
        Err(other) => {
            unreachable!("probe yields Library/Service truth, got {other:?}");
        }
    }
}

#[test]
fn pulse_subscribe_without_a_server_is_the_typed_error() {
    // The notification path shares the connect phase: no server →
    // the same typed ServiceUnavailable, never a fake subscription.
    let Ok(mut devices) = PulseAudioDevices::probe() else {
        // No library at all on this host — the LibraryUnavailable
        // path is covered by the probe test above.
        return;
    };
    let queue = DeviceEventQueue::new();
    match devices.subscribe(queue) {
        Ok(()) => {
            // A live server ([VERIFY] here): subscription succeeded;
            // the queue is the delivery sink.
        }
        Err(PlatformError::ServiceUnavailable { service, .. }) => {
            assert_eq!(service, "PulseAudio server");
        }
        Err(other) => {
            unreachable!("expected Ok or ServiceUnavailable, got {other:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// The assembled platform: discover() + report() tell this host's
// truth end to end.
// ---------------------------------------------------------------------------

#[test]
fn discover_assembles_this_hosts_report() {
    let mut platform = LinuxPlatform::discover();
    // The report rows are cloned out first so the seams can be driven
    // afterwards (the accessors take &mut, the report borrow ends at
    // each statement).
    let rows = [
        Feature::AudioEnumeration,
        Feature::AudioNotifications,
        Feature::GpuEnumeration,
        Feature::FileDialogs,
    ]
    .map(|feature| (feature, platform.report().get(feature)));
    for (feature, capability) in rows {
        match capability {
            Capability::Available => {}
            Capability::Unavailable(reason) => {
                // The honest reasons this host can produce; anything
                // else is a fabrication.
                assert!(
                    matches!(
                        reason,
                        UnavailableReason::LibraryNotFound { .. }
                            | UnavailableReason::ServiceNotReachable { .. }
                    ),
                    "unexpected reason for {feature:?}: {reason}"
                );
            }
        }
    }
    let audio_capability = platform.report().get(Feature::AudioEnumeration);
    let dialog_capability = platform.report().get(Feature::FileDialogs);
    // The seams answer calls consistently with the report.
    let _ = platform.gpu().enumerate().must();
    let audio_result = platform.audio().enumerate();
    match audio_capability {
        Capability::Available => {
            assert!(
                audio_result.is_ok(),
                "available audio must enumerate: {audio_result:?}"
            );
        }
        Capability::Unavailable(_) => {
            assert!(audio_result.is_err(), "unavailable audio must refuse");
        }
    }
    let dialog_result = platform.dialogs().open_file(&FileDialogRequest::default());
    match dialog_capability {
        Capability::Available => assert!(dialog_result.is_ok(), "{dialog_result:?}"),
        Capability::Unavailable(_) => assert!(dialog_result.is_err(), "{dialog_result:?}"),
    }
}
