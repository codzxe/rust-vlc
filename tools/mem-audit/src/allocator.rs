//! The counting global allocator + measurement windows (card W5-15).
//!
//! # What is counted
//!
//! [`Counting`] wraps [`std::alloc::System`] as the process-wide
//! global allocator (installed by `main.rs`) and maintains four
//! monotonic atomic counters per [`CounterSet`]:
//!
//! - `alloc_bytes` — total bytes requested through `alloc` /
//!   `alloc_zeroed` / the growth half of `realloc`;
//! - `alloc_calls` — total allocation calls (same three entry points);
//! - `dealloc_bytes` — total bytes released through `dealloc` and the
//!   shrink half of `realloc`;
//! - `dealloc_calls` — total deallocation calls.
//!
//! Sizes are the **requested** `Layout::size()` values (what the
//! program asked for), not the system allocator's internal block
//! sizes — the honest, allocator-independent metric. `realloc` is
//! implemented explicitly (not left to the default alloc+copy+dealloc
//! emulation) so the call counts reflect real calls, not emulated
//! halves.
//!
//! # Live bytes and the peak
//!
//! `live = alloc_bytes − dealloc_bytes` (requested-size arithmetic).
//! `peak_live` is a running max over every allocation site: only
//! `alloc`-family entries can grow `live`, so the max is complete
//! without touching `dealloc`. Under multithreaded load the
//! load/add/load sequences can observe transiently inconsistent
//! deltas (clamped at zero); this instrument's audited path is
//! single-threaded (the walk driver spawns nothing — see `walk.rs`),
//! and the clamp is documented rather than hidden.
//!
//! # CounterSet: one global, any number of private ones
//!
//! The instrument reads the process-wide [`GLOBAL_COUNTERS`] set;
//! the unit tests drive **private stack sets** through the same
//! `GlobalAlloc` impl — a libtest harness's own result plumbing
//! allocates asynchronously on its coordinator thread, so exact-delta
//! tests against the global set would be racy against the harness
//! itself. Private sets make the counter arithmetic testable
//! deterministically; the global set is what `walk.rs` measures with.
//!
//! # Windows
//!
//! Counters are monotonic for the whole process; per-phase
//! attribution is by [`Snapshot`] deltas taken at phase boundaries
//! (never counter resets — a reset would misattribute later `dealloc`
//! calls of objects allocated before the reset). A [`Window`] pairs
//! the two snapshots and re-seeds `peak_live` at its start, so each
//! window reports its own peak. Windows over the SAME set MUST NOT
//! overlap: the peak re-seed at window start would clobber a
//! concurrent window's maximum. The driver opens windows sequentially
//! (open phase, then walk phase, then printing — printing is always
//! outside every window).
//!
//! A window's `peak_live` includes the process baseline that was
//! live when the window opened (the tool's own allocations, argv,
//! stdout buffers); `growth` (peak − baseline) is the attribution
//! column that isolates the audited code's own footprint. Both are
//! reported — never just one.
//!
//! # The unsafe blocks (the one sanctioned surface)
//!
//! `GlobalAlloc` is an `unsafe trait` and `#[global_allocator]` is
//! the only std hook for a process-wide allocator — there is no safe
//! Rust construction for a counting global allocator. Every unsafe
//! block here is exactly "call the same method on the wrapped
//! `System`" with the pointer/layout contract inherited verbatim
//! from [`GlobalAlloc`]'s own safety rules; the counters are updated
//! only on the success paths (non-null pointers), so a failed inner
//! allocation is neither counted nor double-freed.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

/// The process-wide counter set the instrument measures with.
pub static GLOBAL_COUNTERS: CounterSet = CounterSet::new();

/// One set of counting allocator counters (see the module docs).
pub struct CounterSet {
    /// Total requested bytes allocated (monotonic).
    alloc_bytes: AtomicU64,
    /// Total allocation calls (monotonic).
    alloc_calls: AtomicU64,
    /// Total requested bytes deallocated (monotonic).
    dealloc_bytes: AtomicU64,
    /// Total deallocation calls (monotonic).
    dealloc_calls: AtomicU64,
    /// Running maximum of live requested bytes; re-seeded at each
    /// window open over this set (windows never overlap).
    peak_live: AtomicU64,
}

impl CounterSet {
    /// An all-zero set (const — the global is built this way).
    #[must_use]
    pub const fn new() -> CounterSet {
        CounterSet {
            alloc_bytes: AtomicU64::new(0),
            alloc_calls: AtomicU64::new(0),
            dealloc_bytes: AtomicU64::new(0),
            dealloc_calls: AtomicU64::new(0),
            peak_live: AtomicU64::new(0),
        }
    }

    /// Read the four counters (relaxed loads; the driver is the only
    /// writer context that matters — see the module docs).
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            alloc_bytes: self.alloc_bytes.load(Ordering::Relaxed),
            alloc_calls: self.alloc_calls.load(Ordering::Relaxed),
            dealloc_bytes: self.dealloc_bytes.load(Ordering::Relaxed),
            dealloc_calls: self.dealloc_calls.load(Ordering::Relaxed),
        }
    }

    /// The current running peak (since the last window open on this
    /// set).
    #[must_use]
    pub fn peak_live(&self) -> u64 {
        self.peak_live.load(Ordering::Relaxed)
    }

    /// Count one successful allocation of `size` bytes and fold it
    /// into the running peak (the shared body of `alloc` /
    /// `alloc_zeroed` / the growth half of `realloc`).
    fn count_alloc(&self, size: usize) {
        let size = size as u64;
        let bytes = self.alloc_bytes.fetch_add(size, Ordering::Relaxed) + size;
        self.alloc_calls.fetch_add(1, Ordering::Relaxed);
        let freed = self.dealloc_bytes.load(Ordering::Relaxed);
        // i64 arithmetic: under a multithreaded race the just-added
        // bytes can be observed before a peer's matching dealloc
        // lands; clamp instead of wrapping (module docs: honesty over
        // false precision).
        let live = (bytes as i64 - freed as i64).max(0) as u64;
        self.peak_live.fetch_max(live, Ordering::Relaxed);
    }

    /// Count one deallocation of `size` bytes.
    fn count_dealloc(&self, size: usize) {
        self.dealloc_bytes.fetch_add(size as u64, Ordering::Relaxed);
        self.dealloc_calls.fetch_add(1, Ordering::Relaxed);
    }
}

/// One monotonic read of the four counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    /// `alloc_bytes` at the read.
    pub alloc_bytes: u64,
    /// `alloc_calls` at the read.
    pub alloc_calls: u64,
    /// `dealloc_bytes` at the read.
    pub dealloc_bytes: u64,
    /// `dealloc_calls` at the read.
    pub dealloc_calls: u64,
}

impl Snapshot {
    /// Live requested bytes at this snapshot (clamped at zero — see
    /// the module docs on multithreaded transients).
    #[must_use]
    pub fn live_bytes(&self) -> u64 {
        self.alloc_bytes.saturating_sub(self.dealloc_bytes)
    }
}

/// Read the process-wide counters (the instrument's set).
#[must_use]
pub fn snapshot() -> Snapshot {
    GLOBAL_COUNTERS.snapshot()
}

/// The process-wide counting allocator (installed by `main.rs`).
///
/// Points at the static [`GLOBAL_COUNTERS`]; zero-sized in practice —
/// every reportable number lives in the counter set so [`Snapshot`]
/// reads never need a handle.
pub struct Counting {
    counters: &'static CounterSet,
}

/// The process-wide counting allocator value (the `#[global_allocator]`
/// site — see `main.rs`).
#[must_use]
pub const fn counting_global() -> Counting {
    Counting {
        counters: &GLOBAL_COUNTERS,
    }
}

// SAFETY: every method forwards pointer+layout verbatim to the wrapped
// `System` allocator and only touches the counters on the success
// paths. The GlobalAlloc contract for each forwarded call (alignment,
// non-null on success, dealloc pointer provenance) is inherited
// unchanged from `System`'s own implementation — this wrapper adds no
// pointer arithmetic, no storage, and no reordering of the inner call.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `layout` is the caller's (GlobalAlloc contract); the
        // inner System call has the same contract as this method.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.counters.count_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`, for the zeroed flavor.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            self.counters.count_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller guarantees `ptr` came from this
        // allocator's alloc family with this `layout` (GlobalAlloc
        // contract); System.dealloc has the identical contract.
        self.counters.count_dealloc(layout.size());
        // SAFETY: as above.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: caller-guaranteed provenance + the same contract on
        // the inner call. Counted as the real single call it is:
        // growth adds (new − old) bytes and a call, shrink subtracts
        // (old − new) bytes and a call — so emulated reallocs (which
        // std performs as alloc+copy+dealloc against the raw System)
        // never double-count.
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if !out.is_null() {
            let old = layout.size();
            if new_size > old {
                self.counters.count_growth(new_size - old);
            } else if new_size < old {
                self.counters.count_dealloc(old - new_size);
            }
        }
        out
    }
}

impl CounterSet {
    /// The realloc-growth half of `count_alloc` (bytes + call + peak
    /// fold; the delta form — `size` is the growth, not the total).
    fn count_growth(&self, size: usize) {
        let size = size as u64;
        let bytes = self.alloc_bytes.fetch_add(size, Ordering::Relaxed) + size;
        self.alloc_calls.fetch_add(1, Ordering::Relaxed);
        let freed = self.dealloc_bytes.load(Ordering::Relaxed);
        let live = (bytes as i64 - freed as i64).max(0) as u64;
        self.peak_live.fetch_max(live, Ordering::Relaxed);
    }
}

/// One measured phase: the delta between two counter snapshots plus
/// the peak re-seeded at the phase start.
///
/// Open/close discipline lives with the caller (`walk.rs`): windows
/// are sequential, and all formatting/printing happens after
/// `close` — during a window the driver performs no allocation of
/// its own (only stack arithmetic), so every counted byte in the
/// walk window is the audited code's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    /// Bytes allocated during the phase.
    pub alloc_bytes: u64,
    /// Allocation calls during the phase.
    pub alloc_calls: u64,
    /// Bytes deallocated during the phase.
    pub dealloc_bytes: u64,
    /// Deallocation calls during the phase.
    pub dealloc_calls: u64,
    /// Max live requested bytes observed during the phase (includes
    /// the baseline below).
    pub peak_live: u64,
    /// Live requested bytes when the phase opened.
    pub baseline_live: u64,
}

impl Phase {
    /// Peak growth over the phase baseline (peak − baseline).
    #[must_use]
    pub fn growth(&self) -> u64 {
        self.peak_live.saturating_sub(self.baseline_live)
    }
}

/// A sequential measurement window over one counter set (see the
/// module docs).
pub struct Window<'a> {
    set: &'a CounterSet,
    start: Snapshot,
}

impl Window<'_> {
    /// Open on the process-wide set (the instrument's windows).
    #[must_use]
    pub fn open() -> Window<'static> {
        Window::open_on(&GLOBAL_COUNTERS)
    }
}

impl<'a> Window<'a> {
    /// Open on an explicit set: read the counters and re-seed the
    /// peak at current live. Windows over the same set must not
    /// overlap.
    #[must_use]
    pub fn open_on(set: &'a CounterSet) -> Window<'a> {
        let start = set.snapshot();
        set.peak_live.store(start.live_bytes(), Ordering::Relaxed);
        Window { set, start }
    }

    /// Close: the delta + the window's peak.
    #[must_use]
    pub fn close(&self) -> Phase {
        let end = self.set.snapshot();
        Phase {
            alloc_bytes: end.alloc_bytes.saturating_sub(self.start.alloc_bytes),
            alloc_calls: end.alloc_calls.saturating_sub(self.start.alloc_calls),
            dealloc_bytes: end.dealloc_bytes.saturating_sub(self.start.dealloc_bytes),
            dealloc_calls: end.dealloc_calls.saturating_sub(self.start.dealloc_calls),
            peak_live: self.set.peak_live(),
            baseline_live: self.start.live_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive one counter set through the GlobalAlloc impl directly
    /// (the same code path `#[global_allocator]` takes) with a
    /// private set: exact deltas, zero ambient noise (the libtest
    /// harness allocates on its own schedule — module docs).
    fn private_set() -> &'static CounterSet {
        // A leaked private set per call: tests are few and tiny; the
        // leak is the price of 'static without unsafe.
        Box::leak(Box::new(CounterSet::new()))
    }

    #[allow(unsafe_code)]
    fn alloc_on(set: &'static CounterSet, size: usize) -> *mut u8 {
        let counting = Counting { counters: set };
        let layout = Layout::from_size_align(size, 1).unwrap_or(Layout::new::<u8>());
        // SAFETY: valid layout (size ≥ 1, align 1); the caller
        // deallocates through dealloc_on with the same size.
        unsafe { counting.alloc(layout) }
    }

    #[allow(unsafe_code)]
    fn dealloc_on(set: &'static CounterSet, ptr: *mut u8, size: usize) {
        let counting = Counting { counters: set };
        let layout = Layout::from_size_align(size, 1).unwrap_or(Layout::new::<u8>());
        // SAFETY: ptr came from alloc_on on the same set/size.
        unsafe { counting.dealloc(ptr, layout) };
    }

    /// A counted allocation of `size` bytes is exactly one call of
    /// exactly `size` requested bytes (the counter arithmetic's
    /// ground truth).
    #[test]
    fn counted_allocation_moves_counters_by_exact_size() {
        let set = private_set();
        let before = set.snapshot();
        let ptr = alloc_on(set, 4096);
        assert!(!ptr.is_null());
        let after = set.snapshot();
        assert_eq!(after.alloc_bytes - before.alloc_bytes, 4096);
        assert_eq!(after.alloc_calls - before.alloc_calls, 1);
        dealloc_on(set, ptr, 4096);
        let freed = set.snapshot();
        assert_eq!(freed.dealloc_bytes - after.dealloc_bytes, 4096);
        assert_eq!(freed.dealloc_calls - after.dealloc_calls, 1);
    }

    /// The peak tracks concurrent live bytes and survives frees (it is
    /// a max, not a gauge); a re-opened window re-seeds from the
    /// current live level.
    #[test]
    fn peak_tracks_concurrent_live_and_reseeds() {
        let set = private_set();
        let window = Window::open_on(set);
        let first = alloc_on(set, 1024);
        let second = alloc_on(set, 2048);
        let phase = window.close();
        assert_eq!(
            phase.peak_live - phase.baseline_live,
            1024 + 2048,
            "peak covers exactly the two held allocations"
        );
        dealloc_on(set, first, 1024);
        dealloc_on(set, second, 2048);
        // After the drops, a fresh window's baseline is back at the
        // set's floor and its peak starts there (re-seed semantics).
        let window2 = Window::open_on(set);
        let phase2 = window2.close();
        assert_eq!(phase2.peak_live, phase2.baseline_live);
    }

    /// Phase deltas of adjacent windows compose without double
    /// counting (the attribution discipline `walk.rs` relies on).
    #[test]
    fn adjacent_windows_compose() {
        let set = private_set();
        let w1 = Window::open_on(set);
        let a = alloc_on(set, 512);
        let p1 = w1.close();
        let w2 = Window::open_on(set);
        let b = alloc_on(set, 128);
        dealloc_on(set, a, 512);
        dealloc_on(set, b, 128);
        let p2 = w2.close();
        assert_eq!(p1.alloc_calls, 1, "exactly one allocation in window 1");
        assert_eq!(p2.alloc_calls, 1, "exactly one allocation in window 2");
        assert_eq!(p1.dealloc_calls, 0);
        assert_eq!(p2.dealloc_calls, 2, "both allocations freed in window 2");
    }
}
