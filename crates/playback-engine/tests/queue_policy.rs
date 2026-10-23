//! Card 3.B3's config/validation property suite (design E4):
//! [`QueuePlan::from_product_limits`] over a seeded integer domain —
//! every accepted plan's caps land inside the schema ranges, every
//! rejected input carries the typed axis and the violated bound — plus
//! the constants test that pins the engine's duplicated floors and
//! ceilings against `product.schema.json` itself (the schema is the
//! authority; the engine's copies must never drift).
//!
//! Convention: hand-rolled seeded XorShift (the repo's own, no
//! proptest — C5); every failure prints its seed.

use std::num::NonZeroUsize;

use media_types::Nanoseconds;
use packet_queue::QueueLimits;
use playback_engine::{
    DEFAULT_STARVE_DEBOUNCE_TICKS, QUEUE_PACKETS_AUDIO_CEILING, QUEUE_PACKETS_FLOOR,
    QUEUE_PACKETS_VIDEO_CEILING, QueuePlan, QueuePlanError, QueuePlanKind,
};
use testkit::Must;

/// Deterministic xorshift* (`limits.rs` convention).
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

fn seeds() -> Vec<u64> {
    let mut seeds = vec![1u64, 2, 3, 4, 0x9E37_79B9_7F4A_7C15];
    if let Ok(extra) = std::env::var("LUMEN_PROP_SEEDS") {
        for part in extra.split(',') {
            if let Ok(seed) = part.trim().parse::<u64>() {
                seeds.push(seed | 1);
            }
        }
    }
    seeds
}

/// The schema ranges as the test's own copy — cross-checked against
/// the real file by `schema_constants_pin_the_engine_copies` below, so
/// this table can never silently drift either.
fn schema_ranges() -> [(QueuePlanKind, u64, u64); 2] {
    [
        (QueuePlanKind::Audio, 64, 8192),
        (QueuePlanKind::Video, 64, 4096),
    ]
}

/// E4 over the seeded domain: accepts map to in-range caps with the
/// documented byte/duration figures and the subtitle lane; rejects
/// carry the typed axis and the exact violated bound — never a clamp.
#[test]
fn from_product_limits_accepts_in_range_and_rejects_typed() {
    for seed in seeds() {
        let mut rng = XorShift(seed | 1);
        for _ in 0..2_000u64 {
            let audio = rng.next() % 10_000;
            let video = rng.next() % 10_000;
            let result = QueuePlan::from_product_limits(audio, video);
            let audio_ok = (QUEUE_PACKETS_FLOOR..=QUEUE_PACKETS_AUDIO_CEILING).contains(&audio);
            let video_ok = (QUEUE_PACKETS_FLOOR..=QUEUE_PACKETS_VIDEO_CEILING).contains(&video);
            match (result, audio_ok, video_ok) {
                (Ok(plan), true, true) => {
                    // Every accepted plan satisfies the schema ranges
                    // and the documented derived figures.
                    assert_eq!(plan.audio.max_packets.get(), audio as usize, "seed {seed}");
                    assert_eq!(plan.video.max_packets.get(), video as usize, "seed {seed}");
                    assert_eq!(plan.audio.max_bytes.get(), 1 << 20, "seed {seed}");
                    assert_eq!(plan.video.max_bytes.get(), 4 << 20, "seed {seed}");
                    assert_eq!(plan.audio.max_duration, Nanoseconds(1_000_000_000));
                    assert_eq!(plan.video.max_duration, Nanoseconds(2_000_000_000));
                    assert_eq!(plan.subtitle.max_packets.get(), 128);
                    assert_eq!(plan.subtitle.max_duration, Nanoseconds(10_000_000_000));
                    assert_eq!(plan.starve_debounce_ticks, DEFAULT_STARVE_DEBOUNCE_TICKS);
                    // The derived thresholds sit inside the caps and
                    // the stall line sits strictly below the resume
                    // line on the packets axis (design B3's validation).
                    let marks = plan.thresholds(&plan.audio);
                    assert!(marks.low.packets < marks.high.packets, "seed {seed}");
                    assert!(marks.high.packets <= plan.audio.max_packets.get());
                    assert!(marks.low.bytes == plan.audio.max_bytes.get());
                    assert!(marks.low.duration == plan.audio.max_duration);
                }
                (Err(err), false, _) => {
                    // The audio axis is the violated one (audio is
                    // checked first): the typed error names it.
                    match err {
                        QueuePlanError::PacketsBelowFloor { kind, value, floor } => {
                            assert_eq!(kind, QueuePlanKind::Audio, "seed {seed}");
                            assert_eq!(value, audio);
                            assert_eq!(floor, QUEUE_PACKETS_FLOOR);
                        }
                        QueuePlanError::PacketsAboveCeiling {
                            kind,
                            value,
                            ceiling,
                        } => {
                            assert_eq!(kind, QueuePlanKind::Audio, "seed {seed}");
                            assert_eq!(value, audio);
                            assert_eq!(ceiling, QUEUE_PACKETS_AUDIO_CEILING);
                        }
                    }
                }
                (Err(err), true, false) => match err {
                    QueuePlanError::PacketsBelowFloor { kind, value, floor } => {
                        assert_eq!(kind, QueuePlanKind::Video, "seed {seed}");
                        assert_eq!(value, video);
                        assert_eq!(floor, QUEUE_PACKETS_FLOOR);
                    }
                    QueuePlanError::PacketsAboveCeiling {
                        kind,
                        value,
                        ceiling,
                    } => {
                        assert_eq!(kind, QueuePlanKind::Video, "seed {seed}");
                        assert_eq!(value, video);
                        assert_eq!(ceiling, QUEUE_PACKETS_VIDEO_CEILING);
                    }
                },
                (Ok(_), false | true, false | true) => {
                    unreachable!("seed {seed}: accept/reject disagrees with the ranges");
                }
                (Err(_), true, true) => {
                    unreachable!("seed {seed}: an in-range pair was rejected");
                }
            }
        }
    }
}

/// The exact boundaries accept (reject-never-clamp has crisp edges):
/// the floor, each ceiling, and the shipped product figures.
#[test]
fn from_product_limits_accepts_the_exact_boundaries() {
    for (kind, floor, ceiling) in schema_ranges() {
        let (audio, video) = match kind {
            QueuePlanKind::Audio => (floor, 512),
            QueuePlanKind::Video => (1024, floor),
        };
        QueuePlan::from_product_limits(audio, video)
            .unwrap_or_else(|e| unreachable!("the floor itself must accept: {e}"));
        let (audio, video) = match kind {
            QueuePlanKind::Audio => (ceiling, 512),
            QueuePlanKind::Video => (1024, ceiling),
        };
        QueuePlan::from_product_limits(audio, video)
            .unwrap_or_else(|e| unreachable!("the ceiling itself must accept: {e}"));
        let (audio, video) = match kind {
            QueuePlanKind::Audio => (floor - 1, 512),
            QueuePlanKind::Video => (1024, floor - 1),
        };
        assert!(
            matches!(
                QueuePlan::from_product_limits(audio, video),
                Err(QueuePlanError::PacketsBelowFloor { .. })
            ),
            "one below the floor must reject typed"
        );
        let (audio, video) = match kind {
            QueuePlanKind::Audio => (ceiling + 1, 512),
            QueuePlanKind::Video => (1024, ceiling + 1),
        };
        assert!(
            matches!(
                QueuePlan::from_product_limits(audio, video),
                Err(QueuePlanError::PacketsAboveCeiling { .. })
            ),
            "one above the ceiling must reject typed"
        );
    }
    // The shipped product figures (product.toml:199-200).
    QueuePlan::from_product_limits(1024, 512)
        .unwrap_or_else(|e| unreachable!("the shipped figures must accept: {e}"));
}

/// The constants test (design E4): the engine's duplicated floors and
/// ceilings are pinned against `product.schema.json` itself — the
/// schema is the authority; a drift in either direction fails loudly.
/// Std-only: the JSON line is located by key and the `minimum` /
/// `maximum` literals are parsed out of it (no JSON dependency in the
/// std-only engine — the schema's row shape is one line, and this test
/// is exactly the tripwire for that shape changing).
#[test]
fn schema_constants_pin_the_engine_copies() {
    let schema_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../product.schema.json");
    let schema = std::fs::read_to_string(schema_path)
        .unwrap_or_else(|err| unreachable!("the committed schema must read: {err}"));

    let extract = |key: &str| -> (u64, u64) {
        // The property row (not the `required` array mention): the
        // line that carries both the key and its range.
        let line = schema
            .lines()
            .find(|line| line.contains(key) && line.contains("\"minimum\""))
            .unwrap_or_else(|| unreachable!("the schema must carry a {key} range row"));
        let number_after = |tag: &str| -> u64 {
            let at = line
                .find(tag)
                .unwrap_or_else(|| unreachable!("the {key} row must carry {tag}: {line}"));
            let rest = &line[at + tag.len()..];
            let digits: String = rest
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            digits.parse::<u64>().unwrap_or_else(|err| {
                unreachable!("a malformed {tag} literal on the {key} row: {err} ({line})")
            })
        };
        (number_after("\"minimum\":"), number_after("\"maximum\":"))
    };

    let (audio_min, audio_max) = extract("queue_packets_audio");
    let (video_min, video_max) = extract("queue_packets_video");
    assert_eq!(
        (QUEUE_PACKETS_FLOOR, QUEUE_PACKETS_AUDIO_CEILING),
        (audio_min, audio_max),
        "the engine's audio constants drifted from product.schema.json"
    );
    assert_eq!(
        (QUEUE_PACKETS_FLOOR, QUEUE_PACKETS_VIDEO_CEILING),
        (video_min, video_max),
        "the engine's video constants drifted from product.schema.json"
    );
}

/// The exhaustive kind mapping (design A3, audit F-1): the lane
/// lookup is **total** over [`WiredKind`] — every wired kind has its
/// caps by construction — and the lane-less truth (data) lives in the
/// single typed conversion `WiredKind::try_from(MediaKind)`, never a
/// silent audio fallback and never a runtime abort in the lane
/// builder.
#[test]
fn for_kind_is_exhaustive_and_honest() {
    use playback_engine::UnwiredKind;

    use media_types::MediaKind;
    use playback_engine::WiredKind;
    let plan = QueuePlan::default();
    let audio = plan.for_kind(WiredKind::Audio);
    let video = plan.for_kind(WiredKind::Video);
    let subtitle = plan.for_kind(WiredKind::Subtitle);
    assert_eq!(audio, plan.audio);
    assert_eq!(video, plan.video);
    assert_eq!(subtitle, plan.subtitle);
    // The three lanes are actually distinct postures.
    assert_ne!(audio.max_packets, video.max_packets);
    assert_ne!(audio.max_duration, subtitle.max_duration);
    // Data has no lane — the typed refusal at the single conversion
    // point (the retired convention panicked in the lane builder
    // instead; audit F-1).
    assert_eq!(
        WiredKind::try_from(MediaKind::Data),
        Err(UnwiredKind {
            kind: MediaKind::Data
        }),
        "data has no lane"
    );
    // And the wired kinds round-trip through their full-kind view.
    for (wired, media) in [
        (WiredKind::Audio, MediaKind::Audio),
        (WiredKind::Video, MediaKind::Video),
        (WiredKind::Subtitle, MediaKind::Subtitle),
    ] {
        assert_eq!(wired.as_media_kind(), media);
        assert_eq!(WiredKind::try_from(media), Ok(wired));
    }
}

/// `QueuePlan::default()` keeps the doc 03 §4.4 figures (the library
/// layer's own defaults — the product posture is a host injection,
/// visible by diff; E5's pinned test, engine side).
#[test]
fn default_keeps_the_doc03_library_figures() {
    let plan = QueuePlan::default();
    assert_eq!(plan.audio.max_packets.get(), 200);
    assert_eq!(plan.audio.max_duration, Nanoseconds(1_000_000_000));
    assert_eq!(plan.audio.max_bytes.get(), 1 << 20);
    assert_eq!(plan.video.max_packets.get(), 300);
    assert_eq!(plan.video.max_duration, Nanoseconds(2_000_000_000));
    assert_eq!(plan.video.max_bytes.get(), 4 << 20);
    assert_eq!(plan.subtitle.max_packets.get(), 128);
    assert_eq!(plan.subtitle.max_bytes.get(), 16 << 20);
    assert_eq!(plan.subtitle.max_duration, Nanoseconds(10_000_000_000));
    assert_eq!(plan.starve_debounce_ticks, DEFAULT_STARVE_DEBOUNCE_TICKS);
    // And `new` carries the same defaults for the un-specified lanes.
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    let plan = QueuePlan::new(limits, limits);
    assert_eq!(plan.audio, limits);
    assert_eq!(plan.video, limits);
    assert_eq!(plan.subtitle.max_packets.get(), 128);
}

/// The thresholds shape (design B2 gap 2): packets is the starvation
/// axis — the byte/duration low marks sit at their caps (never
/// binding), the high marks stay at three quarters, and the stall
/// line sits strictly below the resume line on every axis.
#[test]
fn thresholds_make_packets_the_starvation_axis() {
    let plan = QueuePlan::default();
    for limits in [plan.audio, plan.video, plan.subtitle] {
        let marks = plan.thresholds(&limits);
        assert_eq!(marks.low.packets, (limits.max_packets.get() / 4).max(1));
        assert_eq!(
            marks.low.bytes,
            limits.max_bytes.get(),
            "the byte low mark never binds"
        );
        assert_eq!(
            marks.low.duration, limits.max_duration,
            "the duration low mark never binds"
        );
        assert_eq!(
            marks.high.packets,
            limits.max_packets.get() - limits.max_packets.get() / 4
        );
        assert_eq!(
            marks.high.bytes,
            limits.max_bytes.get() - limits.max_bytes.get() / 4
        );
        assert!(marks.low.packets < marks.high.packets, "stall < resume");
    }
}

/// The error surface is a real `std::error::Error` with a message that
/// names the axis and the bound (the host's typed open failure).
#[test]
fn queue_plan_errors_display_the_axis_and_bound() {
    let err = QueuePlan::from_product_limits(10, 512).err().must();
    let text = err.to_string();
    assert!(text.contains("audio"), "{text}");
    assert!(text.contains("64"), "{text}");
    let err = QueuePlan::from_product_limits(1024, 99_999).err().must();
    let text = err.to_string();
    assert!(text.contains("video"), "{text}");
    assert!(text.contains("4096"), "{text}");
    let boxed: Box<dyn std::error::Error> =
        Box::new(QueuePlan::from_product_limits(0, 512).err().must());
    assert!(!boxed.to_string().is_empty());
}
