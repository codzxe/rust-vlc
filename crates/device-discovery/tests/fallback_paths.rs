//! Card 7.D1 integration: capability negotiation + fallback paths —
//! the honest-degradation guarantees, exercised through the public
//! assembly ([`device_discovery::Discovery::probe`]) and the typed
//! fallback ([`device_discovery::volume::UnavailableVolumes`]).
//!
//! On this host (the card's sandbox): the volume lanes report the
//! typed NotYetImplemented verdict (a compile-time fact — no platform
//! integration implements the seam yet, by layering) and the SSDP lane
//! probes Available (the ephemeral UDP bind works here; the python
//! probe recorded in the worklog is the same syscalls).

use device_discovery::capability::{Capability, Feature};
use device_discovery::device::ServiceDiscovery;
use device_discovery::error::{DiscoveryError, UnavailableReason};
use device_discovery::volume::{UnavailableVolumes, VolumesService};
use device_discovery::{Discovery, DiscoveryContext};

#[test]
fn probe_reports_the_volume_lanes_as_not_yet_implemented() {
    let discovery = Discovery::probe();
    let report = discovery.report();
    for feature in [Feature::VolumeEnumeration, Feature::VolumeNotifications] {
        match report.get(feature) {
            Capability::Unavailable(UnavailableReason::NotYetImplemented { detail }) => {
                assert!(
                    detail.contains("platform"),
                    "the reason names the follow-up owner: {detail}"
                );
            }
            other => unreachable!("volume lanes are honestly absent, got {other:?}"),
        }
    }
    // And the seam itself answers the same truth (never a fake list).
    let mut volumes = discovery.volumes();
    match volumes.enumerate(&DiscoveryContext::new()) {
        Err(DiscoveryError::Unavailable {
            reason: UnavailableReason::NotYetImplemented { .. },
        }) => {}
        other => unreachable!("the seam never fakes a volume list, got {other:?}"),
    }
}

#[test]
fn probe_reports_every_feature_row_exactly_once() {
    let binding = Discovery::probe();
    let report = binding.report();
    let mut features = report.iter().map(|(f, _)| f).collect::<Vec<_>>();
    features.sort_by_key(|f| f.key());
    assert_eq!(
        features,
        vec![
            Feature::NetworkServiceDiscovery,
            Feature::VolumeEnumeration,
            Feature::VolumeNotifications,
        ],
        "the report is a table: one row per probed feature"
    );
}

#[test]
fn ssdp_lane_probes_available_on_this_host() {
    // Host assertion (this sandbox has a working UDP stack — the
    // recorded probe); a hermetic-CI future that loses sockets will
    // fail here honestly, saying so.
    let discovery = Discovery::probe();
    let verdict = discovery.report().get(Feature::NetworkServiceDiscovery);
    assert!(
        verdict.is_available(),
        "the native UDP socket probe must pass here; got {verdict:?}"
    );
    assert!(
        discovery.ssdp().is_some(),
        "an Available verdict carries the searcher"
    );
}

#[test]
fn volumes_service_over_the_fallback_translates_to_engine_errors() {
    let service = VolumesService::new(Box::new(UnavailableVolumes::because(
        DiscoveryError::Unavailable {
            reason: UnavailableReason::NotYetImplemented {
                detail: "platform cards own the follow-up (issue #42)",
            },
        },
    )));
    let err = match service.discover(&DiscoveryContext::new()) {
        Err(err) => err,
        Ok(devices) => unreachable!("absence is never a fake success: {devices:?}"),
    };
    assert_eq!(err.stage, media_types::PipelineStage::Discovery);
    assert_eq!(err.context.code, "discovery.unavailable");
    let source = match std::error::Error::source(&err) {
        Some(source) => source.to_string(),
        None => unreachable!("the typed cause rides the source chain"),
    };
    assert!(source.contains("platform cards"), "{source}");
}

#[test]
fn capability_from_probe_error_collapses_the_reason_class() {
    let io_probe = DiscoveryError::Io {
        context: "ssdp probe bind",
        kind: std::io::ErrorKind::PermissionDenied,
        detail: "EPERM".to_string(),
    };
    let capability = Capability::from_probe_error(&io_probe);
    match capability {
        Capability::Unavailable(UnavailableReason::DependencyMissing { what, .. }) => {
            assert_eq!(what, "host I/O");
        }
        other => unreachable!("io probes collapse typed, got {other:?}"),
    }
}
