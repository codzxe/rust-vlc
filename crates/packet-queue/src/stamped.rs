//! Generation-stamped values (doc 03 §4.4(b)) and the two seams the
//! generic queue needs from its payloads.

use media_types::ids::GenerationId;
use media_types::packet::Packet;
use media_types::time::Nanoseconds;

/// A value stamped with the generation it was produced under
/// (doc 03 §4.4(b)).
///
/// Per doc 03 §5 checking point 3, the *producer* stamps the current
/// token generation onto the item at push time; the queue then treats
/// the stamp as the single authority when comparing against its
/// published generation. For payloads that already carry their
/// generation — [`Packet`] does (doc 01 §3.3) — [`Stamped::stamped`]
/// copies it over, so the two can never drift.
///
/// The doc's field name `gen` is spelled `generation` here: `gen` is a
/// reserved keyword in edition 2024, and the L0 [`Packet`] already
/// spells the concept `generation` — one name, everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stamped<T> {
    /// The generation this item was produced under.
    pub generation: GenerationId,
    /// The payload itself.
    pub value: T,
}

impl<T> Stamped<T> {
    /// Stamp an explicit generation onto a value.
    #[must_use]
    pub fn new(generation: GenerationId, value: T) -> Stamped<T> {
        Stamped { generation, value }
    }

    /// Stamp a value with its own generation ([`Generational`]).
    #[must_use]
    pub fn stamped(value: T) -> Stamped<T>
    where
        T: Generational,
    {
        Stamped {
            generation: value.generation(),
            value,
        }
    }

    /// Discard the stamp, returning the payload.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Whether this item is stale relative to `current` — strictly past
    /// generations only (doc 01 §3.3 `Packet::is_stale` semantics).
    #[must_use]
    pub fn is_stale(&self, current: GenerationId) -> bool {
        self.generation < current
    }
}

/// Payloads that know their own generation.
///
/// Implemented for [`Packet`] (its `generation` field, doc 01 §3.3);
/// frame types will follow the same shape when their crate lands.
pub trait Generational {
    /// The generation this item was produced under.
    fn generation(&self) -> GenerationId;
}

impl Generational for Packet {
    fn generation(&self) -> GenerationId {
        self.generation
    }
}

/// What a queue needs to know about a payload's weight (doc 03 §4.4:
/// queues are bounded in packets, bytes *and* duration).
///
/// Both methods must be pure and total — the queue calls them once per
/// push, outside its critical section, and stores the result; user code
/// never runs under the queue's lock.
pub trait Weigh {
    /// Memory weight counted against the queue's byte limit.
    fn size_bytes(&self) -> usize;

    /// Media duration counted against the queue's duration limit.
    ///
    /// Doc 03 §4.4(f) `[PROPOSAL]`: unknown-duration items count as 0
    /// toward the duration limit — they must not wedge it — but they
    /// still count in packets and bytes.
    fn media_duration(&self) -> Nanoseconds;
}

impl Weigh for Packet {
    fn size_bytes(&self) -> usize {
        // Payload bytes only: the fixed per-packet metadata is a
        // constant the packet cap already accounts for.
        self.data.len()
    }

    fn media_duration(&self) -> Nanoseconds {
        match self.duration {
            Some(media_types::time::Duration::Known(n)) => n,
            _ => Nanoseconds(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::packet::SharedBytes;
    use media_types::time::Duration;
    use testkit::Must;

    fn packet(generation: GenerationId, bytes: usize, dur: Duration) -> Packet {
        Packet {
            track: media_types::ids::TrackId::new(1),
            pts: media_types::time::Stamp::Missing,
            dts: media_types::time::Stamp::Missing,
            duration: Some(dur),
            data: SharedBytes::from_vec(vec![0u8; bytes]),
            keyframe: false,
            discontinuity: false,
            generation,
        }
    }

    #[test]
    fn stamped_stamps_from_the_payload_generation() {
        let g0 = GenerationId::new(7);
        let s = Stamped::stamped(packet(g0, 10, Duration::Unknown));
        assert_eq!(s.generation, g0);
        assert_eq!(s.value.generation(), g0);
    }

    #[test]
    fn stamped_is_stale_is_strictly_past() {
        let g0 = GenerationId::FIRST;
        let s = Stamped::new(g0, 1u8);
        assert!(!s.is_stale(g0));
        assert!(s.is_stale(g0.next().must()));
        // A future stamp is an ordering bug elsewhere, not staleness.
        assert!(!s.is_stale(GenerationId::new(0)));
    }

    #[test]
    fn stamped_into_value_discards_the_stamp() {
        let s = Stamped::new(GenerationId::FIRST, 42u16);
        assert_eq!(s.into_value(), 42u16);
    }

    #[test]
    fn packet_weigh_reports_payload_bytes() {
        let p = packet(GenerationId::FIRST, 1500, Duration::Unknown);
        assert_eq!(Weigh::size_bytes(&p), 1500);
    }

    #[test]
    fn packet_weigh_maps_known_duration() {
        let p = packet(
            GenerationId::FIRST,
            1,
            Duration::Known(Nanoseconds(3_333_333)),
        );
        assert_eq!(Weigh::media_duration(&p), Nanoseconds(3_333_333));
    }

    #[test]
    fn packet_weigh_maps_unknown_duration_to_zero() {
        // Doc 03 §4.4(f): unknown durations count 0 toward the
        // duration limit.
        let p = packet(GenerationId::FIRST, 1, Duration::Unknown);
        assert_eq!(Weigh::media_duration(&p), Nanoseconds(0));
        // And a missing duration field likewise.
        let mut p = packet(GenerationId::FIRST, 1, Duration::Known(Nanoseconds(9)));
        p.duration = None;
        assert_eq!(Weigh::media_duration(&p), Nanoseconds(0));
    }
}
