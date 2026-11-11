//! The bounded hardware **surface pool** (cards 7.B1–7.B3) — one
//! shared frame-slot model for every lane (VA-API surfaces,
//! VideoToolbox `CVPixelBuffer`s retained by the session, Media
//! Foundation DXVA surfaces).
//!
//! # Why a pool, and why *bounded*
//!
//! Every OS decoder API hands out a finite set of output surfaces
//! and stalls (or errors) when the set is exhausted. Lumen's
//! invariant is stronger (MP §12, D-2/D-3): decode-side memory is
//! **bounded by construction** — the pool has a hard capacity
//! ([`HwSurfacePool::MAX_SURFACES`]), `acquire` is fallible, and
//! exhaustion is a *typed* condition the adapter reports
//! ([`PoolExhausted`]), never an unbounded allocation, never a
//! silent overwrite of a frame already handed to the presenter.
//!
//! # Slot lifecycle
//!
//! ```text
//!            acquire()             emitted to the codec layer
//!   Free ───────────────▶ Decoding ───────────────▶ Emitted
//!    ▲                        │ emit()                    │
//!    └────────────────────────┴──── release(handle) ───────┘
//! ```
//!
//! [`HwSurfacePool::release`] is the **present-side contract's**
//! hook: the renderer that consumed an emitted surface returns it.
//! That side of the contract is the 7.B/R5 renderer follow-up
//! `[PROPOSAL]`; until it lands, a lane that exhausts its pool
//! reports the typed error below (honest back-pressure, not a
//! leak-and-limp).
//!
//! The handle minted by `acquire` is the `surface: u64` of
//! [`super::H264HwFrame`] — media-types'
//! `FrameStorage::OpaqueHandle` payload. It is *not* a pointer:
//! it is a pool slot id, stable for the slot's lifetime, so no
//! raw address ever crosses the safe API (the lane keeps its own
//! id → OS-resource mapping).

use std::fmt;

/// Hard per-session surface cap (MP §12: bounded memory by
/// construction). 64 exceeds any real decoder's useful DPB +
/// present queue (VA-API drivers typically serve 17–21; VideoToolbox
/// pools default lower) while keeping worst-case surface memory
/// visible at configure time.
pub const MAX_SURFACES: usize = 64;

/// A pool slot's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    /// Acquirable.
    Free,
    /// Owned by an in-flight decode.
    Decoding,
    /// Handed to the codec layer; awaiting the present-side
    /// release.
    Emitted,
}

/// The typed pool-exhaustion condition (never a panic, never an
/// unbounded grow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolExhausted {
    /// Pool capacity at the moment of exhaustion.
    pub capacity: usize,
    /// How many slots were already emitted (awaiting release).
    pub emitted: usize,
}

impl fmt::Display for PoolExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "surface pool exhausted: {}/{} emitted awaiting release",
            self.emitted, self.capacity
        )
    }
}

impl std::error::Error for PoolExhausted {}

/// One bounded pool of decode surface slots.
///
/// Generic over the lane's own OS-resource per slot (`VASurfaceID`,
/// retained `CVPixelBufferRef`, MF surface handle) so the bookkeeping
/// stays in one tested place and each lane owns only its mapping.
#[derive(Debug)]
pub struct HwSurfacePool<T> {
    slots: Vec<(SlotState, Option<T>)>,
    /// Pool generation — bumped by [`Self::drain`], so every handle
    /// minted before a drain is permanently stale (no aliasing).
    generation: u64,
    /// Configured capacity (≤ [`MAX_SURFACES`]).
    capacity: usize,
}

impl<T> HwSurfacePool<T> {
    /// A pool with `capacity` slots, all free. Capacity above
    /// [`MAX_SURFACES`] is clamped with the cap applied (the bound
    /// is a safety invariant, not a suggestion).
    #[must_use = "constructs the bounded pool"]
    pub fn new(capacity: usize) -> HwSurfacePool<T> {
        let capacity = capacity.min(MAX_SURFACES);
        HwSurfacePool {
            slots: Vec::with_capacity(capacity),
            generation: 0,
            capacity,
        }
    }

    /// The configured capacity.
    #[must_use = "pure accessor"]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Slots currently handed out (decoding + emitted).
    #[must_use = "pure accessor"]
    pub fn in_use(&self) -> usize {
        self.slots
            .iter()
            .filter(|(state, _)| !matches!(state, SlotState::Free))
            .count()
    }

    /// Slots emitted and awaiting present-side release.
    #[must_use = "pure accessor"]
    pub fn emitted(&self) -> usize {
        self.slots
            .iter()
            .filter(|(state, _)| matches!(state, SlotState::Emitted))
            .count()
    }

    /// Take a free slot for `resource` (the lane's OS handle for
    /// the surface being created into it).
    ///
    /// # Errors
    /// [`PoolExhausted`] when no slot is free — the caller reports
    /// the typed condition upstream (never allocates past the cap).
    pub fn acquire(&mut self, resource: T) -> Result<u64, PoolExhausted> {
        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, (state, _))| matches!(state, SlotState::Free))
        {
            slot.0 = SlotState::Decoding;
            slot.1 = Some(resource);
            Ok(self.handle_of(index))
        } else if self.slots.len() < self.capacity {
            let index = self.slots.len();
            self.slots.push((SlotState::Decoding, Some(resource)));
            Ok(self.handle_of(index))
        } else {
            Err(PoolExhausted {
                capacity: self.capacity,
                emitted: self.emitted(),
            })
        }
    }

    /// Mark a decoding slot as emitted (its frame was handed to the
    /// codec layer). The lane resource **stays** in the slot — the
    /// present side resolves `surface` handles back to OS resources
    /// via [`Self::resource`] until it calls [`Self::release`].
    /// `false` when the handle is not a live decoding slot (stale or
    /// already emitted — a no-op, never a panic).
    pub fn emit(&mut self, handle: u64) -> bool {
        let Some(index) = self.split_handle(handle) else {
            return false;
        };
        match self.slots.get_mut(index) {
            Some((state @ SlotState::Decoding, _)) => {
                *state = SlotState::Emitted;
                true
            }
            _ => false,
        }
    }

    /// Return an emitted (or decoding — flush/teardown paths) slot
    /// to the free set and drop its resource. `true` when the slot
    /// was live. This is the present-side release hook.
    pub fn release(&mut self, handle: u64) -> bool {
        let Some(index) = self.split_handle(handle) else {
            return false;
        };
        match self.slots.get_mut(index) {
            Some((state, resource)) if !matches!(state, SlotState::Free) => {
                *state = SlotState::Free;
                *resource = None;
                true
            }
            _ => false,
        }
    }

    /// Whether any live (decoding or emitted) slot currently holds
    /// `resource` — the lane's "surface busy" predicate (a surface
    /// referenced by an emitted slot or an in-flight decode must
    /// not be picked as a new render target).
    #[must_use = "pure predicate"]
    pub fn holds_resource(&self, resource: &T) -> bool
    where
        T: PartialEq,
    {
        self.slots.iter().any(|(state, held)| {
            !matches!(state, SlotState::Free) && held.as_ref() == Some(resource)
        })
    }

    /// The lane resource currently held for a live slot (borrowed).
    #[must_use = "pure accessor"]
    pub fn resource(&self, handle: u64) -> Option<&T> {
        let index = self.split_handle(handle)?;
        self.slots
            .get(index)
            .and_then(|(_, resource)| resource.as_ref())
    }

    /// Retire every slot (stream re-negotiation / teardown): all
    /// handles minted so far become permanently invalid (the pool
    /// generation advances past them — ids never alias), and the
    /// pool is empty-but-warm for the next generation.
    pub fn drain(&mut self) {
        self.slots.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Pack (pool generation, slot index) into the opaque handle.
    /// Layout: high 32 bits generation, low 32 bits slot index —
    /// the generation half makes stale-handle aliasing impossible
    /// even after `drain`.
    fn handle_of(&self, index: usize) -> u64 {
        (self.generation << 32) | u32::try_from(index).unwrap_or(u32::MAX) as u64
    }

    /// Split a handle back into (slot index, generation). Returns
    /// `None` unless the handle belongs to the current pool
    /// generation and addresses a live slot.
    fn split_handle(&self, handle: u64) -> Option<usize> {
        if handle >> 32 != self.generation {
            return None;
        }
        let index = usize::try_from(handle & 0xFFFF_FFFF).ok()?;
        (index < self.slots.len()).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(n: usize) -> HwSurfacePool<u32> {
        HwSurfacePool::new(n)
    }

    #[test]
    fn acquire_emit_release_lifecycle_is_bounded_and_typed() {
        let mut p = pool(2);
        let a = p.acquire(0xA).unwrap_or_else(|e| unreachable!("{e}"));
        let b = p.acquire(0xB).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(p.in_use(), 2);
        // Capacity reached: typed exhaustion, never an allocation.
        match p.acquire(0xC) {
            Err(PoolExhausted { capacity, emitted }) => {
                assert_eq!(capacity, 2);
                assert_eq!(emitted, 0);
            }
            Ok(_) => unreachable!("pool must be exhausted at capacity"),
        }
        // Emit moves the slot to Emitted, keeping the OS resource
        // resolvable for the present side.
        assert!(p.emit(a));
        assert_eq!(p.resource(a), Some(&0xA), "emitted slots stay resolvable");
        assert!(!p.emit(a), "double emit is a no-op");
        assert_eq!(p.emitted(), 1);
        assert_eq!(p.in_use(), 2);
        // Still exhausted: emitted slots are not free.
        assert!(p.acquire(0xD).is_err());
        // Present-side release frees exactly that slot.
        assert!(p.release(a));
        assert!(!p.release(a), "double release is a no-op");
        // The freed slot is recycled for the next surface — the
        // release contract says the caller is done with the handle,
        // so reuse is the point (the *drain* invariant is what makes
        // stale handles impossible; see the drain test).
        let c = p.acquire(0xE).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(c, a, "released slots recycle");
        assert_eq!(p.resource(c), Some(&0xE), "the mapping is the new resource");
        assert_eq!(p.in_use(), 2);
        // Resource lookup serves the live mapping.
        assert_eq!(p.resource(b), Some(&0xB));
    }

    #[test]
    fn capacity_is_clamped_to_the_safety_cap() {
        assert_eq!(
            HwSurfacePool::<u8>::new(MAX_SURFACES + 10).capacity(),
            MAX_SURFACES
        );
        assert_eq!(HwSurfacePool::<u8>::new(3).capacity(), 3);
        assert_eq!(HwSurfacePool::<u8>::new(0).capacity(), 0);
        // A zero-capacity pool fails every acquire, typed.
        assert!(HwSurfacePool::<u8>::new(0).acquire(1).is_err());
    }

    #[test]
    fn drain_invalidates_every_prior_handle() {
        let mut p = pool(4);
        let a = p.acquire(1).unwrap_or_else(|e| unreachable!("{e}"));
        let b = p.acquire(2).unwrap_or_else(|e| unreachable!("{e}"));
        p.emit(a);
        p.drain();
        assert_eq!(p.in_use(), 0);
        // Pre-drain handles are stale: emit/release are no-ops.
        assert!(!p.emit(a));
        assert!(!p.release(b));
        // Post-drain handles carry a new generation: no aliasing,
        // even when the slot index is reused.
        let c = p.acquire(3).unwrap_or_else(|e| unreachable!("{e}"));
        assert_ne!(c, a);
        assert_eq!(c >> 32, (a >> 32) + 1, "generation advanced");
    }

    #[test]
    fn garbage_handles_are_noops_never_panics() {
        let mut p = pool(2);
        for bogus in [u64::MAX, 0xDEAD_BEEF_0000_0001, 7, 1 << 32] {
            assert!(!p.emit(bogus));
            assert!(!p.release(bogus));
            assert_eq!(p.resource(bogus), None);
        }
    }

    #[test]
    fn steady_state_acquire_release_recycles_without_growth() {
        let mut p = pool(4);
        // Fill the pool, emit everything, release everything, then
        // repeat: the pool must serve the next round from recycled
        // slots without ever exceeding its cap.
        for round in 0..100u32 {
            let mut handles = Vec::new();
            for i in 0..4u32 {
                let h = p
                    .acquire(round * 100 + i)
                    .unwrap_or_else(|e| unreachable!("{e}"));
                handles.push(h);
            }
            assert!(p.acquire(0).is_err(), "cap holds at round {round}");
            for h in &handles {
                assert!(p.emit(*h));
            }
            assert_eq!(p.emitted(), 4);
            for h in &handles {
                assert!(p.release(*h));
            }
            assert_eq!(p.in_use(), 0);
        }
        // After 400 acquire/emit/release cycles the pool is still
        // four slots deep and fully reusable.
        assert_eq!(p.capacity(), 4);
        assert!(p.acquire(0xFFFF).is_ok());
    }
}
