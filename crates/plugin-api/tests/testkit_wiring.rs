//! Cross-crate wiring proof for MP §3.7: `testkit` is a
//! **[dev-dependencies]**-only crate. This file is compiled as part of
//! `plugin-api`'s test suite and uses the kit's helpers against the
//! production vocabulary — the concrete demonstration of the direction
//! contract (no production crate depends on testkit) that
//! `check-architecture` enforces mechanically, following the
//! `crates/media-types` precedent (tests/testkit_wiring.rs).
//!
//! Audit T5 (2026-11-17): the dev-dependency was declared by contract
//! but never exercised — cargo-shear flagged it as dead. This wiring
//! proof closes that finding the way the contract intends (use it,
//! don't remove it).

use plugin_api::kind::PluginKind;
use plugin_api::semver::{Semver, SemverError};
use testkit::{Must, assert_contains, assert_err};

#[test]
fn testkit_helpers_exercise_the_vocabulary() {
    // Semver is the manifest's version grammar — the kit's Must
    // helper is the lint-legal unwrap for fixture-style contracts.
    let version = Semver::parse("1.2.3-rc.1+build.7").must();
    assert_eq!(
        version.precedence(&Semver::parse("1.2.3").must()),
        std::cmp::Ordering::Less,
        "pre-release sorts before its release"
    );

    // Error paths are first-class (MP §8): the kit's assert_err +
    // assert_contains prove the diagnostic is actionable.
    let err: SemverError = assert_err(Semver::parse("1.02.3"), "leading zero is illegal");
    let msg = err.to_string();
    assert_contains(&msg, "leading zero", "names the violated rule");
}

#[test]
fn kind_vocabulary_round_trips_through_the_kit() {
    // PluginKind is the §1.2 registry vocabulary: every one of the
    // sixteen kinds carries a kebab-case manifest name (doc 06 §2.2)
    // and the names are unique — data, never hand-typed copy.
    let mut names: Vec<&str> = PluginKind::ALL.iter().map(|k| k.as_str()).collect();
    names.sort_unstable();
    let distinct = names.iter().collect::<std::collections::BTreeSet<_>>().len();
    assert_eq!(distinct, names.len(), "manifest kind names are unique");
    assert_eq!(names.len(), 16, "all sixteen §1.2 rows present");

    // Spot-check the spellings the §2.3 examples rely on.
    assert_eq!(PluginKind::Access.as_str(), "access");
    assert_eq!(PluginKind::AudioOutput.as_str(), "audio-output");
    assert_eq!(
        PluginKind::ServiceDiscovery.as_str(),
        "service-discovery"
    );

    // Hostile manifests are a typed error, not a panic — the Semver
    // lane above proves the same posture for versions.
    let err: SemverError = assert_err(Semver::parse("01.2.3"), "leading zero is illegal");
    assert_contains(&err.to_string(), "leading zero", "names the violated rule");
}
