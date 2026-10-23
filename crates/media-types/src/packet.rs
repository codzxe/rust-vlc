//! Packets: compressed data in flight (doc 01 §3.3).

use crate::ids::{GenerationId, TrackId};
use crate::time::{Duration, Stamp};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, Range};
use std::sync::Arc;

/// Zero-copy-friendly shared byte buffer (std-only, doc 01 §3.3).
///
/// One allocation, arbitrarily many cheap clones: the demuxer hands the
/// same bytes to the decoder, the recorder and the probe without
/// copying.
///
/// A handle is a **view**: the shared backing allocation (an [`Arc`]
/// around the payload's heap block) plus the byte range the view
/// exposes. W6-05 (memory-audit register V-2) grew this from a bare
/// `Arc<[u8]>` to that `(Arc, offset, len)` shape so one pooled
/// allocation can host many varying-size payloads at different offsets
/// — the per-packet spill-copy elimination for e.g. MJPEG streams
/// (D-1, D-2):
///
/// * [`from_vec`](Self::from_vec) moves the payload's heap block in
///   behind the refcount: the bytes are **not** copied (the pre-W6-05
///   `Arc<[u8]>` conversion always copied the `Vec` into a second
///   payload-sized block — the lie the old "no copy" doc-comment told);
/// * [`build`](Self::build) allocates the payload once and fills it in
///   place (no post-fill copy — the W5-19 cold-path constructor);
/// * [`from_pool_range`](Self::from_pool_range) and
///   [`slice`](Self::slice) mint O(1) sub-views: a refcount bump plus
///   two integers, no allocation, no copy;
/// * [`clone`](Clone) is the refcount bump only (proved by the counting
///   allocator in `tests/shared_bytes_alloc.rs`).
///
/// The view's slice semantics are exactly its range: [`Deref`],
/// [`as_slice`](Self::as_slice), [`AsRef`], [`len`](Self::len) and the
/// [`Debug`] preview all expose `pool[offset..offset + len]`, never the
/// backing allocation.
#[derive(Clone)]
pub struct SharedBytes {
    /// The shared payload block. `Vec` (not `[u8]`) is what makes
    /// `from_vec`/`build` copy-free in safe Rust — `Arc::new(vec)` moves
    /// the `Vec`'s already-allocated heap block behind the refcount
    /// instead of copying the bytes into a fresh `Arc<[u8]>` block — and
    /// what lets an exclusive owner (see [`get_mut`](Self::get_mut))
    /// reuse or resize the allocation in place.
    buf: Arc<Vec<u8>>,
    /// The exposed byte range of `buf` (the view). Invariant, asserted
    /// at every construction site: `0 <= start <= end <= buf.len()`.
    range: Range<usize>,
}

impl SharedBytes {
    /// Wrap an owned buffer: the payload's heap allocation **moves in**
    /// behind the refcount — the bytes are not copied.
    ///
    /// Cost: one allocation event of constant size (the `Arc` header
    /// block that carries the `Vec`; 40 bytes on 64-bit), independent
    /// of payload length — measured in `tests/shared_bytes_alloc.rs`.
    /// The pre-W6-05 implementation (`Vec` → `Arc<[u8]>`) allocated a
    /// second payload-sized block and copied every byte into it (W5-19
    /// measured the copy; W5-15 registered it as V-2).
    pub fn from_vec(bytes: Vec<u8>) -> SharedBytes {
        let len = bytes.len();
        SharedBytes {
            buf: Arc::new(bytes),
            range: 0..len,
        }
    }

    /// Wrap static bytes as a shareable buffer.
    ///
    /// Honesty note (W6-05 audit): this **copies** the bytes into a
    /// fresh refcounted heap block — a `&'static [u8]` cannot be
    /// adopted behind an `Arc` without `unsafe`, which this crate
    /// forbids. The pre-W6-05 implementation copied too (the
    /// `Arc::from(&[u8])` conversion allocates and copies), so the old
    /// "no allocation, no copy" doc-comment was wrong in both halves.
    /// Measured delta of this representation: one additional
    /// constant-size header event. Empty slices — the only production
    /// shape (demux `riff.rs` zero-size payloads) — pay the tiny header
    /// event and no copy.
    pub fn from_static(bytes: &'static [u8]) -> SharedBytes {
        let len = bytes.len();
        SharedBytes {
            buf: Arc::new(bytes.to_vec()),
            range: 0..len,
        }
    }

    /// Allocate the payload **once** and fill it in place (the W5-19
    /// cold-path constructor: no intermediate buffer, no post-fill
    /// copy).
    ///
    /// `fill` receives the whole payload slice (it runs even when `len`
    /// is zero, with an empty slice — no special-case magic callers must
    /// remember) and should overwrite every byte it cares about. Safe
    /// Rust cannot hand out `&mut [u8]`
    /// over uninitialized memory, so the block is zero-initialized
    /// first — a memset, never a copy of already-written bytes — and
    /// the filled bytes are never moved again. The pre-W6-05 pattern
    /// (build a `Vec`, then `from_vec` into an `Arc<[u8]>`) paid one
    /// extra payload-sized allocation and one full memcpy *after* the
    /// fill.
    ///
    /// Total cost: one payload-sized allocation event plus the
    /// constant-size `Arc` header (two events; the exact measured
    /// numbers are pinned in `tests/shared_bytes_alloc.rs`).
    pub fn build<F>(len: usize, fill: F) -> SharedBytes
    where
        F: FnOnce(&mut [u8]),
    {
        let mut bytes = vec![0u8; len];
        fill(&mut bytes);
        SharedBytes {
            buf: Arc::new(bytes),
            range: 0..len,
        }
    }

    /// Mint a `(Arc, offset, len)` view of a shared pool:
    /// `pool[offset..offset + len]` (the demux V-1 upgrade path, W5-14
    /// handoff).
    ///
    /// Varying-size payloads — e.g. MJPEG frames — share one pooled
    /// allocation at different offsets, so a packet costs one refcount
    /// bump and two integers: no per-packet allocation, no copy. The
    /// pool's memory stays alive as long as any view (or the owner's
    /// `Arc`) references it. A pool owner that keeps its own `Arc` can
    /// reclaim the block for reuse once every view is dropped
    /// (`Arc::strong_count` / `Arc::try_unwrap` — the recycle policy
    /// belongs to the owner, not this type).
    ///
    /// O(1).
    ///
    /// # Panics
    /// When `offset + len` overflows `usize` or exceeds `pool.len()` —
    /// immediately, in debug and release builds alike. This is an
    /// internal invariant type following packet.rs's error style of
    /// infallible constructors with documented contracts: the caller is
    /// engine code that has already bounds-checked the chunk it read,
    /// so an out-of-range request is a bug to surface, never to clamp
    /// (release builds clamp nothing).
    pub fn from_pool_range(pool: Arc<Vec<u8>>, offset: usize, len: usize) -> SharedBytes {
        assert!(
            offset.checked_add(len).is_some_and(|end| end <= pool.len()),
            "SharedBytes::from_pool_range: offset {offset} + len {len} exceeds pool len {}",
            pool.len()
        );
        SharedBytes {
            buf: pool,
            range: offset..offset + len,
        }
    }

    /// A sub-view of this view's bytes: `self[range]` as a new
    /// [`SharedBytes`] sharing the same allocation — O(1), no
    /// allocation, no copy (a refcount bump plus two integers).
    ///
    /// The range is **relative to this view** (the [`Deref`] slice), so
    /// views compose: `pool_view.slice(4..8).slice(1..2)` addresses
    /// `pool[5..6]`.
    ///
    /// # Panics
    /// When `range` is not contained in this view
    /// (`range.start <= range.end <= self.len()`) — immediately, in
    /// debug and release builds alike; never clamped (same internal
    /// invariant contract as
    /// [`from_pool_range`](Self::from_pool_range)).
    pub fn slice(&self, range: Range<usize>) -> SharedBytes {
        assert!(
            range.start <= range.end && range.end <= self.len(),
            "SharedBytes::slice: range {range:?} not contained in view of len {}",
            self.len()
        );
        let start = self.range.start + range.start;
        let end = self.range.start + range.end;
        SharedBytes {
            buf: Arc::clone(&self.buf),
            range: start..end,
        }
    }

    /// The bytes of this view.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.range.start..self.range.end]
    }

    /// Length in bytes of this view.
    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    /// Empty view.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Unique mutable access to this view's bytes, when this handle's
    /// allocation has **no** other owner (refcount 1: no sibling view,
    /// no clone, no pool handle). The zero-allocation
    /// buffer-recycling primitive (D-1): a producer that handed a view
    /// downstream and got it back (the consumer dropped its handle) can
    /// refill the same bytes in place instead of allocating a fresh
    /// block per quantum/frame.
    ///
    /// Returns `None` while any other handle is alive — never clones,
    /// never copies, never blocks: the caller rotates to another buffer
    /// or allocates (its policy, typed and bounded). With the W6-05
    /// pooled representation this succeeds exactly when the *whole*
    /// allocation is exclusively owned, which is the only sound moment
    /// to write through a shared view.
    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        let bytes = Arc::get_mut(&mut self.buf)?;
        Some(&mut bytes[self.range.start..self.range.end])
    }
}

impl Deref for SharedBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buf[self.range.start..self.range.end]
    }
}

/// Content equality — the pre-W6-05 `derive` on `Arc<[u8]>` compared (and
/// hashed) the pointed-to bytes, not the pointers; this keeps that
/// contract while the view's range decides *which* bytes are compared.
impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for SharedBytes {}

/// Content hash, consistent with [`PartialEq`] (equal views hash equal,
/// regardless of which allocation or range backs them).
impl Hash for SharedBytes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug never dumps the payload: packet bytes are media data, but
        // a firehose in logs is a bug class of its own. Cap at 8 bytes —
        // of the VIEW (the range semantics), never the backing pool.
        let cap = &self.as_slice()[..self.len().min(8)];
        let hex: Vec<String> = cap.iter().map(|b| format!("{b:02x}")).collect();
        write!(f, "SharedBytes(len={}, {})", self.len(), hex.join(" "))
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
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
    use std::collections::hash_map::DefaultHasher;

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

    // -----------------------------------------------------------------
    // W6-05: representation size (recorded, honest numbers).
    //
    // Before (main @ 62c48b6, measured via the /tmp size probe on this
    // toolchain): SharedBytes = 16 (fat `Arc<[u8]>`), Packet = 96,
    // Option<Packet> = 96.
    // After: SharedBytes = thin `Arc<Vec<u8>>` (8) + `Range<usize>` (16)
    // = 24 — the safe minimum that carries the (pool, offset, len) view.
    // A 16-byte variant was possible with u32 range halves but would cap
    // the type at 4 GiB allocations; rejected (no hidden limits on the
    // hottest type). See the worklog W6-05 entry for the full table.
    // -----------------------------------------------------------------
    #[test]
    fn size_of_recorded_before_and_after() {
        assert_eq!(
            std::mem::size_of::<SharedBytes>(),
            std::mem::size_of::<Arc<Vec<u8>>>() + 2 * std::mem::size_of::<usize>(),
            "thin Arc + Range<usize> (24 on 64-bit; was 16 pre-W6-05)"
        );
        assert_eq!(std::mem::size_of::<SharedBytes>(), 24); // 64-bit pin
    }

    #[test]
    fn range_views_expose_only_their_range() {
        let pool: Vec<u8> = (0u8..32).collect();
        let pool = Arc::new(pool);
        let a = SharedBytes::from_pool_range(Arc::clone(&pool), 0, 4);
        let b = SharedBytes::from_pool_range(Arc::clone(&pool), 4, 8);
        let c = SharedBytes::from_pool_range(Arc::clone(&pool), 28, 4);

        assert_eq!(a.as_slice(), &[0, 1, 2, 3]);
        assert_eq!(b.as_slice(), &(4..12).collect::<Vec<u8>>());
        assert_eq!(c.as_slice(), &[28, 29, 30, 31]);
        assert_eq!(b.len(), 8);
        assert!(!b.is_empty());
        // Deref semantics = the range: indexing, iteration, coercion.
        assert_eq!(b[0], 4);
        assert_eq!(&b[..4], &[4, 5, 6, 7]);
        assert_eq!(b.iter().sum::<u8>(), (4..12).sum::<u8>());
        let coerced: &[u8] = &b;
        assert_eq!(coerced.len(), 8);
        // AsRef and Deref agree.
        assert_eq!(b.as_ref(), coerced);
        // An empty view is legal and well-formed.
        let empty = SharedBytes::from_pool_range(Arc::clone(&pool), 7, 0);
        assert!(empty.is_empty());
        assert_eq!(empty.as_slice(), &[]);
    }

    #[test]
    fn subslicing_composes_relative_to_the_view() {
        let pool = SharedBytes::from_vec((0u8..16).collect::<Vec<u8>>());
        let v = pool.slice(4..12);
        assert_eq!(v.as_slice(), &(4..12).collect::<Vec<u8>>());
        let w = v.slice(2..5);
        assert_eq!(w.as_slice(), &[6, 7, 8]);
        // The sub-view shares the pool allocation, not a copy (same
        // block, six bytes in).
        assert_eq!(
            w.as_slice().as_ptr(),
            pool.as_slice().as_ptr().wrapping_add(6)
        );
        // Empty sub-views compose too.
        assert!(v.slice(3..3).is_empty());
    }

    #[test]
    fn views_keep_the_allocation_alive_past_the_original_handle() {
        let pool = Arc::new((0u8..64).collect::<Vec<u8>>());
        let view = SharedBytes::from_pool_range(Arc::clone(&pool), 16, 8);
        assert_eq!(view.as_slice(), &(16..24).collect::<Vec<u8>>());
        // Every live view (and the owner's Arc) holds one reference.
        let base = Arc::strong_count(&pool);
        let extra = SharedBytes::from_pool_range(Arc::clone(&pool), 0, 4);
        assert_eq!(Arc::strong_count(&pool), base + 1);
        let clone = extra.clone();
        assert_eq!(Arc::strong_count(&pool), base + 2);
        assert_eq!(extra.as_slice().as_ptr(), clone.as_slice().as_ptr());
        drop(clone);
        drop(extra);
        assert_eq!(Arc::strong_count(&pool), base);
        drop(view);
        // Dropping every other handle leaves a late view the sole
        // owner — the Arc lifetime-sharing contract.
        let sole = SharedBytes::from_pool_range(Arc::clone(&pool), 40, 4);
        drop(pool);
        assert_eq!(sole.as_slice(), &[40, 41, 42, 43]);
        // ...and grants it exclusive mutable access (the recycle path).
        let mut sole = sole;
        let bytes = sole
            .get_mut()
            .unwrap_or_else(|| unreachable!("sole owner must be mutable"));
        bytes.fill(0xEE);
        assert_eq!(sole.as_slice(), &[0xEE; 4]);
    }

    #[test]
    fn get_mut_refuses_to_write_behind_live_views() {
        let mut pool = SharedBytes::from_vec(vec![1u8; 8]);
        let view = pool.slice(0..4);
        assert!(pool.get_mut().is_none(), "a live view pins the pool");
        drop(view);
        let bytes = pool
            .get_mut()
            .unwrap_or_else(|| unreachable!("last handle must be mutable"));
        bytes.fill(9);
        assert_eq!(pool.as_slice(), &[9u8; 8]);
    }

    #[test]
    fn out_of_range_construction_panics_never_clamps() {
        let pool = Arc::new(vec![0u8; 16]);
        let view = SharedBytes::from_pool_range(Arc::clone(&pool), 4, 8);

        // (Arc, offset, len) constructor: end past the pool.
        let over = std::panic::catch_unwind(|| {
            let _ = SharedBytes::from_pool_range(Arc::clone(&pool), 8, 9);
        });
        assert!(over.is_err(), "offset+len > pool len must panic");
        // offset + len overflow.
        let overflow = std::panic::catch_unwind(|| {
            let _ = SharedBytes::from_pool_range(Arc::clone(&pool), usize::MAX, 2);
        });
        assert!(overflow.is_err(), "offset+len overflow must panic");
        // Slicing past the view.
        let past = std::panic::catch_unwind(|| {
            let _ = view.slice(0..9);
        });
        assert!(past.is_err(), "slice past the view must panic");
        // Inverted range (the lint deny is silenced here because the
        // inversion is the very thing under test).
        let inverted = std::panic::catch_unwind(|| {
            #[allow(clippy::reversed_empty_ranges)]
            let _ = view.slice(3..1);
        });
        assert!(inverted.is_err(), "inverted slice range must panic");
        // Nothing above clamped into a wrong-but-valid view: the pool and
        // the surviving view are untouched.
        assert_eq!(view.as_slice(), &pool[4..12]);
    }

    #[test]
    fn equality_and_hash_are_content_not_identity() {
        // Same content from different allocations: equal (the pre-W6-05
        // derive on Arc<[u8]> compared contents too).
        let a = SharedBytes::from_vec(vec![1, 2, 3]);
        let b = SharedBytes::from_vec(vec![1, 2, 3]);
        assert_eq!(a, b);
        // Different content: unequal — even over the same allocation.
        let pool = SharedBytes::from_vec((0u8..16).collect::<Vec<u8>>());
        assert_ne!(pool.slice(0..4), pool.slice(4..8));
        // A view equals a whole buffer with the same content.
        let twin = SharedBytes::from_vec(vec![0, 1, 2, 3]);
        assert_eq!(pool.slice(0..4), twin);
        // Hash consistency with equality (DefaultHasher has fixed keys,
        // so the assertions are deterministic within a build).
        let mut h1 = DefaultHasher::new();
        a.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        b.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
        let mut h3 = DefaultHasher::new();
        pool.slice(0..4).hash(&mut h3);
        let mut h4 = DefaultHasher::new();
        twin.hash(&mut h4);
        assert_eq!(h3.finish(), h4.finish());
        let mut h5 = DefaultHasher::new();
        pool.slice(4..8).hash(&mut h5);
        assert_ne!(h3.finish(), h5.finish());
    }

    #[test]
    fn build_fills_in_place() {
        let sb = SharedBytes::build(11, |buf| {
            assert_eq!(buf.len(), 11);
            for (i, b) in buf.iter_mut().enumerate() {
                *b = i as u8 + 10;
            }
        });
        assert_eq!(sb.as_slice(), &(10..21).collect::<Vec<u8>>());
        assert_eq!(sb.len(), 11);
        // Zero-length build is well-formed (fill still runs, with an
        // empty slice — no special-case magic).
        let empty = SharedBytes::build(0, |buf| assert!(buf.is_empty()));
        assert!(empty.is_empty());
    }

    #[test]
    fn from_static_copies_content_faithfully() {
        let sb = SharedBytes::from_static(b"lumen");
        assert_eq!(sb.as_slice(), b"lumen");
        assert_eq!(sb.len(), 5);
        let empty = SharedBytes::from_static(b"");
        assert!(empty.is_empty());
    }

    #[test]
    fn debug_shows_the_view_not_the_pool() {
        let pool = SharedBytes::from_vec((0u8..32).collect::<Vec<u8>>());
        let view = pool.slice(24..32);
        let d = format!("{view:?}");
        // The preview must start at the view's first byte (24 = 0x18).
        assert!(d.contains("len=8"), "{d}");
        assert!(d.contains("18 19 1a 1b 1c 1d 1e 1f"), "{d}");
    }

    #[test]
    fn pool_owner_can_reclaim_through_try_unwrap() {
        // The V-1 recycle story: fill a pool as a plain Vec, share it as
        // views, then take the allocation back once drained.
        let mut arena = vec![0u8; 64];
        arena.fill(0x55);
        let shared = Arc::new(arena);
        let a = SharedBytes::from_pool_range(Arc::clone(&shared), 0, 32);
        let b = SharedBytes::from_pool_range(Arc::clone(&shared), 32, 32);
        assert!(Arc::try_unwrap(Arc::clone(&shared)).is_err());
        drop(a);
        drop(b);
        let mut reclaimed = Arc::try_unwrap(shared)
            .unwrap_or_else(|_| unreachable!("all views dropped — pool must unwrap"));
        assert_eq!(reclaimed.len(), 64);
        reclaimed.clear(); // the block is reusable as a pool again
        assert!(reclaimed.is_empty());
    }
}
