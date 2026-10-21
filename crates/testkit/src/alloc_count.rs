//! The counting-allocation instrument (ED-FIX-2; Lead decision ED-b:
//! "counting GlobalAlloc wrapper is std-only and allowed as a test
//! instrument"). This is the D-6 "runtime green" proof primitive the
//! whole workspace reuses for zero-allocation claims: a
//! `#[global_allocator]` wrapper that counts bytes and events, plus a
//! snapshot/delta guard that asserts a hot path allocates **nothing**
//! — measured, not argued.
//!
//! # Usage (one dedicated integration-test binary per crate — the
//! global allocator is process-global)
//!
//! ```no_run
//! # use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};
//! #[global_allocator]
//! static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();
//!
//! #[test]
//! fn hot_path_allocates_nothing() {
//!     let _serial = serialize_tests(); // whole-binary serialization
//!     // ... warm up (first call sizes the pools) ...
//!     let guard = ALLOC.state().guard();
//!     // ... N hot-path iterations ...
//!     guard.assert_zero("context");
//! }
//! ```
//!
//! # Safety of the `unsafe impl`
//!
//! `GlobalAlloc` is an `unsafe trait`, so implementing it needs
//! `#[allow(unsafe_code)]` on this module. The workspace denies
//! `unsafe_code` everywhere (AGENTS.md §3.3: no unsafe in application
//! or core crates) — testkit is the dev-only test crate, and every
//! method here forwards verbatim to [`std::alloc::System`] and only
//! bumps atomics on the way: no allocation inside the allocator, no
//! layout arithmetic, no pointer synthesis, no reentrancy. The trait's
//! safety contract (return valid-for-layout pointers, only dealloc
//! what alloc returned) is inherited from `System` unchanged.
//!
//! # Thread-noise discipline
//!
//! The counters are process-global and counting is unconditional
//! (never "paused" — pausing would *hide* concurrent allocations and
//! false-pass). Tests therefore serialize the whole binary through
//! [`serialize_tests`]: while one test measures, no other test thread
//! in this process runs at all. Other test binaries are separate
//! processes and cannot interfere.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};
use std::sync::MutexGuard;

/// The whole-binary serializer for counting-allocator test binaries.
///
/// Every test in the binary takes this lock first and holds it for its
/// entire body (`MutexGuard` in a `let _` binding): tests in the
/// binary run strictly one at a time even under `--test-threads=N`,
/// so no sibling test thread can allocate inside a measured window.
/// Other binaries are separate processes (separate allocator) and are
/// unaffected.
///
/// Locking an uncontended `std::sync::Mutex` allocates nothing
/// (futex-based; the queue is inside the static).
pub fn serialize_tests() -> MutexGuard<'static, ()> {
    static SERIALIZE: Mutex<()> = Mutex::new(());
    match SERIALIZE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The counters behind [`CountingGlobalAlloc`]. All fields are plain
/// atomics — reading a snapshot allocates nothing.
#[derive(Debug, Default)]
pub struct AllocState {
    /// Allocation events (alloc + alloc_zeroed + realloc) so far.
    alloc_calls: AtomicU64,
    /// Deallocation events so far.
    dealloc_calls: AtomicU64,
    /// Cumulative bytes requested (monotonic, saturating).
    total_bytes: AtomicU64,
    /// Requested bytes currently alive (signed: realloc shrinks
    /// transiently step down; test-only bookkeeping).
    live_bytes: AtomicIsize,
}

impl AllocState {
    /// A const-constructible empty state (for the const
    /// `#[global_allocator]` static).
    #[must_use = "a state nobody reads proves nothing"]
    pub const fn new() -> AllocState {
        AllocState {
            alloc_calls: AtomicU64::new(0),
            dealloc_calls: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            live_bytes: AtomicIsize::new(0),
        }
    }

    fn note_alloc(&self, size: usize) {
        self.alloc_calls.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
        self.live_bytes.fetch_add(size as isize, Ordering::Relaxed);
    }

    fn note_dealloc(&self, size: usize) {
        self.dealloc_calls.fetch_add(1, Ordering::Relaxed);
        self.live_bytes.fetch_sub(size as isize, Ordering::Relaxed);
    }

    /// The point-in-time counter snapshot (allocates nothing).
    #[must_use = "observable evidence"]
    pub fn snapshot(&self) -> AllocSnapshot {
        AllocSnapshot {
            alloc_calls: self.alloc_calls.load(Ordering::Relaxed),
            dealloc_calls: self.dealloc_calls.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            live_bytes: self.live_bytes.load(Ordering::Relaxed),
        }
    }

    /// Begin a measured window: snapshot now, assert the delta at
    /// [`CountingGuard::assert_zero`]. The guard itself allocates
    /// nothing.
    #[must_use = "an unasserted guard proves nothing"]
    pub fn guard(&self) -> CountingGuard<'_> {
        CountingGuard {
            state: self,
            start: self.snapshot(),
        }
    }
}

/// One point-in-time reading of the [`AllocState`] counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocSnapshot {
    /// Allocation events so far.
    pub alloc_calls: u64,
    /// Deallocation events so far.
    pub dealloc_calls: u64,
    /// Cumulative requested bytes so far.
    pub total_bytes: u64,
    /// Requested bytes alive right now.
    pub live_bytes: isize,
}

/// What happened inside one measured window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocDelta {
    /// Allocation events inside the window.
    pub alloc_calls: u64,
    /// Deallocation events inside the window.
    pub dealloc_calls: u64,
    /// Net bytes allocated inside the window.
    pub total_bytes: u64,
    /// Change in live bytes inside the window.
    pub live_bytes: isize,
}

/// A measured window over the counting allocator: created by
/// [`AllocState::guard`], asserted by [`CountingGuard::assert_zero`].
///
/// The intended discipline (D-6: runtime green, not syntactic green):
/// warm the pools **before** creating the guard, run the hot path N
/// times inside it, then assert zero. A failure prints the exact
/// byte/call deltas plus the context string.
pub struct CountingGuard<'a> {
    state: &'a AllocState,
    start: AllocSnapshot,
}

impl CountingGuard<'_> {
    /// The window's measured delta.
    #[must_use = "observable evidence"]
    pub fn delta(&self) -> AllocDelta {
        let now = self.state.snapshot();
        AllocDelta {
            alloc_calls: now.alloc_calls - self.start.alloc_calls,
            dealloc_calls: now.dealloc_calls - self.start.dealloc_calls,
            total_bytes: now.total_bytes - self.start.total_bytes,
            live_bytes: now.live_bytes - self.start.live_bytes,
        }
    }

    /// Assert the window allocated nothing: zero events, zero bytes.
    /// Deallocation inside the window is legal (buffers returning to
    /// pools) and not asserted against.
    ///
    /// # Panics
    /// When any allocation event or byte was requested inside the
    /// window (the whole point of the instrument — this is the
    /// zero-allocation contract failing loudly, never silently).
    #[track_caller]
    pub fn assert_zero(&self, context: &str) {
        let delta = self.delta();
        assert!(
            delta.alloc_calls == 0 && delta.total_bytes == 0,
            "{context}: allocated {} bytes across {} allocation events \
             (dealloc events: {}) — the path is NOT zero-allocation",
            delta.total_bytes,
            delta.alloc_calls,
            delta.dealloc_calls
        );
    }
}

/// The counting global allocator. Wrap [`System`] exactly; count on
/// the way through. See the [module docs](self) for the safety
/// argument and the serialization discipline.
pub struct CountingGlobalAlloc {
    state: AllocState,
}

impl CountingGlobalAlloc {
    /// The const constructor for the `#[global_allocator]` static.
    #[must_use = "pure constructor"]
    pub const fn new() -> CountingGlobalAlloc {
        CountingGlobalAlloc {
            state: AllocState::new(),
        }
    }

    /// The counters (snapshot/guard access).
    #[must_use = "observable evidence"]
    pub fn state(&self) -> &AllocState {
        &self.state
    }
}

// SAFETY (the whole of it): every method delegates the pointer work
// to `System` unchanged — same layouts in, its pointers out — and the
// only added behavior is atomic counter arithmetic, which cannot
// allocate, lock, or reenter the allocator. The trait's contract is
// satisfied exactly as `System` satisfies it.
#[allow(unsafe_code)] // Lead decision ED-b; argument above + module docs
unsafe impl GlobalAlloc for CountingGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded verbatim (module docs).
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.state.note_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded verbatim (module docs).
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            self.state.note_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.state.note_dealloc(layout.size());
        // SAFETY: forwarded verbatim (module docs).
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Counted as one allocation event of `new_size` and one
        // deallocation event of the old size: a grow inside a measured
        // window is visible as both a call delta and a byte delta.
        // SAFETY: forwarded verbatim (module docs).
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if !out.is_null() {
            self.state.note_dealloc(layout.size());
            self.state.note_alloc(new_size);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Counter bookkeeping (the wiring through a real global allocator
    // is proven by `tests/alloc_count_e2e.rs`, which installs this
    // type as the process allocator — a unit test cannot, the
    // allocator is process-global).

    #[test]
    fn counters_track_note_pairs_and_the_delta_arithmetic() {
        let state = AllocState::new();
        let guard = state.guard();
        state.note_alloc(512);
        state.note_alloc(64);
        state.note_dealloc(512);
        let delta = guard.delta();
        assert_eq!(delta.alloc_calls, 2);
        assert_eq!(delta.dealloc_calls, 1);
        assert_eq!(delta.total_bytes, 576);
        assert_eq!(delta.live_bytes, 64);
        // Snapshots agree with the notes.
        let snap = state.snapshot();
        assert_eq!(snap.alloc_calls, 2);
        assert_eq!(snap.dealloc_calls, 1);
        assert_eq!(snap.total_bytes, 576);
        assert_eq!(snap.live_bytes, 64);
        // A window opened after the notes is silent.
        let quiet = state.guard();
        quiet.assert_zero("nothing happened in this window");
    }

    #[test]
    fn assert_zero_fails_loudly_on_a_nonzero_window() {
        let state = AllocState::new();
        let guard = state.guard();
        state.note_alloc(32);
        // Proven via catch_unwind so this test stays green while the
        // failure path is exercised (the same shape as the assert
        // helpers above).
        let caught = std::panic::catch_unwind(|| {
            guard.assert_zero("must fail");
        });
        assert!(caught.is_err(), "the instrument must fail loudly");
    }

    #[test]
    fn const_constructor_matches_default() {
        let a = AllocState::new();
        let b = AllocState::default();
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
