//! FX2-A03 regression: the `cli.codec_unavailable` refusal's
//! `landed:` family list derives from the dispatch registry
//! (FIX-PLAN §2 F0.2; wave-13 F-003 / `pipeline_host.rs.audit.md`
//! F-003: the hand-maintained list omitted vp9 and dnxhd while both
//! arms lived in the same file — one binary, four registries
//! answering differently).
//!
//! The pin is the registry contract, in both directions:
//!
//! - every family row's descriptors have live dispatch arms
//!   (probed through `HostDecoderProvider::decoder_for` itself, so
//!   the table cannot overstate the binary);
//! - descriptors no arm accepts refuse with `cli.codec_unavailable`
//!   (so the table cannot understate the binary — the omission that
//!   produced the wave-13 lie);
//! - the refusal message the dispatch actually mints equals the
//!   registry-derived [`codec_unavailable_message`], family for
//!   family, in registry order.

use std::sync::Arc;

use codec_api::{DecoderConfig, ThreadBudget};
use lumen_cli::demux_host::MediaFacts;
use lumen_cli::pipeline_host::{
    HostDecoderProvider, LANDED_DECODER_FAMILIES, codec_unavailable_message, descriptor_is_landed,
};
use media_types::ids::CodecDescriptor;
use media_types::time::{TimeBase, TimeSpan, TrackTiming};
use media_types::{GenerationId, MediaKind, StreamId, Track, TrackFlags, TrackId};
use playback_engine::DecoderProvider;

/// The refusal's human message minted by the real dispatch for
/// `descriptor` — `None` when the dispatch has an arm for it (or the
/// descriptor is malformed).
fn refusal_message(descriptor: &str) -> Option<String> {
    let codec = CodecDescriptor::new(descriptor).ok()?;
    let provider = HostDecoderProvider::new(Arc::new(MediaFacts {
        tracks: Vec::new(),
        facts: Vec::new(),
        container: String::new(),
    }));
    let config = DecoderConfig::new(
        codec.clone(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    );
    let track = Track {
        id: TrackId::new(0),
        stream: StreamId::new(0),
        kind: MediaKind::Video,
        language: None,
        codec,
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: TimeBase { num: 1, den: 1000 },
            gaps: Vec::new(),
        },
    };
    match provider.decoder_for(&track, &config) {
        // An arm exists: either it built, or its own `configure` gate
        // refused with the codec's code — never the catch-all's.
        Ok(_) => None,
        Err(err) if err.context.code.as_str() != "cli.codec_unavailable" => None,
        Err(err) => match &err.source {
            Some(source) => Some(source.to_string()),
            None => Some(String::new()),
        },
    }
}

/// The family names a refusal message reports as landed (parsed out
/// of the real message, never re-derivated).
fn landed_names(message: &str) -> Vec<&str> {
    let Some(list) = message
        .strip_prefix("no decoder for `")
        .and_then(|rest| rest.split_once(" in this build (landed: "))
        .map(|(_, list)| list)
        .and_then(|list| list.strip_suffix(')'))
    else {
        unreachable!("not the refusal shape consumers pin: {message}");
    };
    list.split(", ").collect()
}

/// The refusal string's family set IS the dispatch table's family
/// set: the message the dispatch mints for an unknown codec lists
/// exactly `LANDED_DECODER_FAMILIES`' names, and every row's
/// descriptors have live arms — the wave-13 F-003 drift class (a
/// hand list answering differently from the match beside it) is
/// closed from both directions.
#[test]
fn fx2_a03_refusal_family_set_matches_dispatch_arms() {
    // The message the real dispatch mints equals the registry-derived
    // one, byte for byte (the arm provably uses the derivation).
    let unknown = "zzz-fx2-a03";
    let minted = match refusal_message(unknown) {
        Some(message) => message,
        None => unreachable!("an unknown codec must mint the refusal"),
    };
    assert_eq!(minted, codec_unavailable_message(unknown));
    assert!(
        !descriptor_is_landed(unknown),
        "the exported probe agrees: nothing dispatches `{unknown}`"
    );

    // The landed list is exactly the registry's family names, in
    // registry order.
    let names = landed_names(&minted);
    let registry: Vec<&str> = LANDED_DECODER_FAMILIES
        .iter()
        .map(|row| row.name)
        .collect();
    assert_eq!(
        names, registry,
        "the refusal's family set must equal the table's"
    );

    // Every registry row's descriptors have live arms — the table
    // cannot claim a family the match does not dispatch.
    for row in LANDED_DECODER_FAMILIES {
        for &descriptor in row.descriptors {
            assert!(
                descriptor_is_landed(descriptor),
                "registry row `{}` claims descriptor `{descriptor}` with no dispatch arm",
                row.name,
            );
            assert!(
                refusal_message(descriptor).is_none(),
                "descriptor `{descriptor}` must not mint the refusal",
            );
        }
    }

    // Descriptors no arm accepts refuse — the table cannot omit a
    // family the match dispatches (the vp9/dnxhd omission class:
    // these all answer `false` today; if an arm for one ever lands,
    // its family row must land with it and this list must shrink).
    for absent in [
        "hevc",
        "mpeg2video",
        "mpeg4video",
        "ac3",
        "dts",
        "theora",
        "vp8",
        "ffv1",
        "wmapro",
    ] {
        assert!(
            !descriptor_is_landed(absent),
            "`{absent}` has no dispatch arm"
        );
        let message = match refusal_message(absent) {
            Some(message) => message,
            None => unreachable!("`{absent}` must mint the refusal"),
        };
        let prefix = format!("no decoder for `{absent}` in this build (landed: ");
        assert!(
            message.starts_with(prefix.as_str()),
            "the refusal-prefix shape the consumers pin (tests contain \
             `no decoder for `hevc`` etc.):\n{message}"
        );
    }
}

/// A family the dispatch HAS is never reported as "not landed": the
/// two families the hand-list omitted (vp9, dnxhd — wave-13 F-003)
/// have live arms, never mint `no decoder for \`X\`` themselves, and
/// are listed in the refusal string's landed list — the general
/// form covers every dispatched family the same way.
#[test]
fn fx2_a03_dispatch_family_not_reported_unlanded() {
    // The regression pair: vp9 and dnxhd have live dispatch arms ...
    for family in ["vp9", "dnxhd"] {
        assert!(
            descriptor_is_landed(family),
            "the dispatch HAS a `{family}` arm"
        );
        assert!(
            refusal_message(family).is_none(),
            "a dispatched family never mints `no decoder for `{family}``"
        );
    }

    // ... and the refusal string the binary mints reports them (and
    // every other dispatched family) as landed.
    let minted = match refusal_message("zzz-fx2-a03") {
        Some(message) => message,
        None => unreachable!("an unknown codec must mint the refusal"),
    };
    let names = landed_names(&minted);
    for row in LANDED_DECODER_FAMILIES {
        for &descriptor in row.descriptors {
            assert!(
                descriptor_is_landed(descriptor),
                "family `{}`'s arm must be live before it can be reported landed",
                row.name,
            );
        }
        assert!(
            names.contains(&row.name),
            "the dispatched family `{}` is reported not-landed:\n{minted}",
            row.name,
        );
    }
}
