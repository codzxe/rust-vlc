//! Cross-crate wiring proof for card 1.C3: `testkit` is a
//! **[dev-dependencies]**-only crate (MP §3.7). This file is compiled as
//! part of `media-types`' test suite and uses the kit's helpers against
//! the production vocabulary — the concrete demonstration of the
//! direction contract that card 1.E1's `check-architecture` will later
//! enforce mechanically.

use media_types::prelude::*;
use testkit::{HardwareClass, Must, assert_contains, assert_err, assert_nanos_near};

#[test]
fn testkit_helpers_exercise_the_vocabulary() {
    // Assert helpers against real L0 types.
    let ts = Timestamp::from_parts(90_000, 1, 90_000).must();
    let nanos = ts.as_nanos().must();
    assert_nanos_near(nanos.0, 1_000_000_000, 1, "one second round trip");

    // Error paths are first-class (MP §8).
    let err = assert_err(Timestamp::from_parts(1, 0, 90_000).map(|_| ()), "zero base");
    assert_contains(&err.to_string(), "zero_component", "actionable diagnostic");

    // Evidence recording labeled per doc 10 §7.1.
    let record = HardwareClass::Mock;
    assert_eq!(record.as_str(), "mock");
}

#[test]
fn generation_and_staleness_through_the_kit() {
    let g = GenerationId::FIRST;
    let g1 = g.next().must();
    let packet = Packet {
        track: TrackId::new(0),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: Some(Duration::Unknown),
        data: media_types::packet::SharedBytes::from_static(&[]),
        keyframe: false,
        discontinuity: false,
        generation: g,
    };
    assert!(!packet.is_stale(g));
    assert!(packet.is_stale(g1));
}
