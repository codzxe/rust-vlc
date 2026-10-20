//! Packets: compressed data in flight (doc 01 §3.3).

use crate::ids::{GenerationId, TrackId};
use crate::time::{Duration, Stamp};
use std::sync::Arc;

/// Zero-copy-friendly shared byte buffer (std-only, doc 01 §3.3).
///
/// One allocation, arbitrarily many cheap clones: the demuxer hands the
/// same bytes to the decoder, the recorder and the probe without copying.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SharedBytes(Arc<[u8]>);

impl SharedBytes {
    /// Wrap an owned buffer (the allocation moves in — no copy).
    pub fn from_vec(bytes: Vec<u8>) -> SharedBytes {
        SharedBytes(bytes.into())
    }

    /// Wrap a static byte slice (no allocation, no copy).
    pub fn from_static(bytes: &'static [u8]) -> SharedBytes {
        SharedBytes(bytes.into())
    }

    /// The bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Empty buffer.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug never dumps the payload: packet bytes are media data, but
        // a firehose in logs is a bug class of its own. Cap at 8 bytes.
        let cap = &self.0[..self.0.len().min(8)];
        let hex: Vec<String> = cap.iter().map(|b| format!("{b:02x}")).collect();
        write!(f, "SharedBytes(len={}, {})", self.0.len(), hex.join(" "))
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// One compressed unit of one track, demuxed and headed for decode
/// (doc 01 §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    /// The track this packet belongs to.
    pub track: TrackId,
    /// Presentation timestamp (absent for e.g. some raw streams — that
    /// absence is data, MP §8).
    pub pts: Stamp,
    /// Decode timestamp (absent when equal-or-unknown).
    pub dts: Stamp,
    /// Container-declared duration when known.
    pub duration: Option<Duration>,
    /// The compressed bytes (zero-copy shared).
    pub data: SharedBytes,
    /// Whether the packet starts a keyframe.
    pub keyframe: bool,
    /// Demuxer-marked discontinuity (splice, seek, stream switch).
    /// Arithmetic across it without a clock reset is a
    /// [`TimeError::Discontinuity`](crate::time::TimeError::Discontinuity).
    pub discontinuity: bool,
    /// The generation this packet was demuxed under; stale means dropped
    /// at the earliest checkpoint (doc 01 §3.1).
    pub generation: GenerationId,
}

impl Packet {
    /// Whether the packet is stale relative to `current` — the earliest
    /// checkpoint (demux queue, decoder, filters) drops it on sight.
    pub fn is_stale(&self, current: GenerationId) -> bool {
        self.generation < current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TrackId;
    use crate::time::{Nanoseconds, Timestamp};

    fn sample_packet() -> Packet {
        Packet {
            track: TrackId::new(1),
            pts: Stamp::Present(
                Timestamp::from_parts(900, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}")),
            ),
            dts: Stamp::Present(
                Timestamp::from_parts(880, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}")),
            ),
            duration: Some(Duration::Known(Nanoseconds(3_333_333))),
            data: SharedBytes::from_vec(vec![0x00, 0x01, 0x02, 0x03]),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        }
    }

    #[test]
    fn shared_bytes_clone_shares_without_copying() {
        let a = SharedBytes::from_vec(vec![7u8; 1024]);
        let b = a.clone();
        assert_eq!(a.as_slice(), b.as_slice());
        assert_eq!(a.len(), 1024);
        // Same allocation (Arc semantics): pointer equality of the slice.
        assert_eq!(a.as_slice().as_ptr(), b.as_slice().as_ptr());
    }

    #[test]
    fn shared_bytes_debug_is_capped() {
        let sb = SharedBytes::from_vec(vec![0xAB; 4096]);
        let d = format!("{sb:?}");
        assert!(d.contains("len=4096"), "{d}");
        assert!(d.matches("ab").count() <= 8, "capped: {d}");
    }

    #[test]
    fn packet_staleness_follows_generation_order() {
        let p = sample_packet();
        assert!(!p.is_stale(GenerationId::FIRST));
        assert!(
            p.is_stale(
                GenerationId::FIRST
                    .next()
                    .unwrap_or_else(|e| unreachable!("{e}"))
            )
        );
        // A packet from the future is not "stale" — that is a different
        // (ordering) bug caught elsewhere; staleness is strictly past.
        assert!(!p.is_stale(GenerationId::new(0)));
    }

    #[test]
    fn packet_fields_roundtrip_through_clone() {
        let p = sample_packet();
        let q = p.clone();
        assert_eq!(p, q);
        assert!(q.keyframe);
        assert!(!q.discontinuity);
        assert_eq!(q.data.len(), 4);
    }
}
