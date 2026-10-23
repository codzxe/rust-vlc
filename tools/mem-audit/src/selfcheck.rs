//! The instrument's own overhead measurement (card W5-15 item 5:
//! "instrument overhead honesty — the counting allocator's own cost,
//! measured on a no-op run").
//!
//! Two numbers, both measured, never asserted:
//!
//! 1. **The process floor** — what the tool itself (startup, argv,
//!    output machinery) allocated by the time the audit work begins:
//!    the no-op run's totals. This is the additive baseline under
//!    every window's `baseline_live`.
//! 2. **The per-call counter cost** — a paired micro-benchmark: the
//!    same alloc/dealloc loop through the installed counting global
//!    allocator (`std::alloc::alloc` — the instrumented path every
//!    program allocation takes) vs. through `System` directly (the
//!    un-instrumented path), at representative sizes. The delta is
//!    the counters' own cost (four relaxed atomic ops + one
//!    `fetch_max` per alloc; two per dealloc).
//!
//! The unsafe blocks here are the micro-bench's raw allocator calls —
//! the same sanctioned surface as `allocator.rs` (there is no safe
//! Rust way to call a specific allocator; `Vec` always routes through
//! the global one, which is exactly the path under measurement).

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::time::Instant;

use crate::allocator;

/// One micro-bench row: a size class measured both ways.
#[derive(Debug, Clone)]
pub struct OverheadRow {
    /// The allocation size measured (bytes).
    pub size: usize,
    /// Iterations measured per path.
    pub iters: u64,
    /// ns per alloc+dealloc pair through the counting allocator.
    pub counting_ns: f64,
    /// ns per alloc+dealloc pair through `System` directly.
    pub system_ns: f64,
}

impl OverheadRow {
    /// The counter cost: counting − system (ns per pair; may be
    /// negative on timer noise — reported verbatim, never clamped).
    #[must_use]
    pub fn delta_ns(&self) -> f64 {
        self.counting_ns - self.system_ns
    }
}

/// The no-op floor + the micro-bench table.
#[derive(Debug, Clone)]
pub struct SelfCheck {
    /// Bytes allocated by the process before any audit work (the
    /// no-op run's `alloc_bytes`).
    pub floor_alloc_bytes: u64,
    /// Allocation calls ditto.
    pub floor_alloc_calls: u64,
    /// Live bytes ditto.
    pub floor_live: u64,
    /// The measured rows (small → large).
    pub rows: Vec<OverheadRow>,
}

/// Iterations for one size (bounded runtime: ~20–200 ms per row on
/// this class of hardware; documented, not tuned to flatter).
fn iters_for(size: usize) -> u64 {
    if size <= 4096 {
        200_000
    } else if size <= 65_536 {
        50_000
    } else {
        10_000
    }
}

/// One alloc+dealloc pair through the installed global allocator
/// (the counting path — what every program allocation pays).
///
/// # Errors
/// A typed string when the layout is invalid or the allocation
/// failed (both impossible for the fixed size ladder, checked
/// anyway — the bench must not paper over a failed measurement).
#[allow(unsafe_code)]
fn pair_counting(size: usize) -> Result<f64, String> {
    let layout = Layout::from_size_align(size, 1).map_err(|err| format!("layout: {err}"))?;
    let iters = iters_for(size);
    let start = Instant::now();
    for _ in 0..iters {
        // SAFETY: `layout` has size ≥ 16 and alignment 1 (valid by
        // construction above); the returned pointer is passed back to
        // `dealloc` with the identical layout, per std::alloc's
        // contract. Null (allocation failure) aborts the measurement
        // honestly instead of being freed.
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(format!("counting-path alloc of {size} B failed"));
        }
        black_box(ptr);
        // SAFETY: provenance + layout match the alloc above.
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
    Ok(start.elapsed().as_nanos() as f64 / iters as f64)
}

/// One alloc+dealloc pair through `System` directly (the
/// un-instrumented baseline).
#[allow(unsafe_code)]
fn pair_system(size: usize) -> Result<f64, String> {
    let layout = Layout::from_size_align(size, 1).map_err(|err| format!("layout: {err}"))?;
    let iters = iters_for(size);
    let start = Instant::now();
    for _ in 0..iters {
        // SAFETY: as `pair_counting`, against the `System` allocator
        // value (GlobalAlloc is an object-safe trait — calling it on
        // a value is the same contract as the free functions).
        let ptr = unsafe { System.alloc(layout) };
        if ptr.is_null() {
            return Err(format!("system-path alloc of {size} B failed"));
        }
        black_box(ptr);
        // SAFETY: provenance + layout match the alloc above.
        unsafe { System.dealloc(ptr, layout) };
    }
    Ok(start.elapsed().as_nanos() as f64 / iters as f64)
}

/// Run the self-check: the floor snapshot + the micro-bench ladder.
///
/// # Errors
/// Propagates either path's failure (a failed measurement is a
/// result, never a silent zero).
pub fn run() -> Result<SelfCheck, String> {
    let floor = allocator::snapshot();
    let mut rows = Vec::new();
    for size in [16_usize, 256, 4096, 65_536, 1_048_576] {
        // Counting first, then system, same size, same iteration
        // count; the order is fixed and documented (interleaving
        // would double the ladder's timer noise for no gain).
        let counting_ns = pair_counting(size)?;
        let system_ns = pair_system(size)?;
        rows.push(OverheadRow {
            size,
            iters: iters_for(size),
            counting_ns,
            system_ns,
        });
    }
    Ok(SelfCheck {
        floor_alloc_bytes: floor.alloc_bytes,
        floor_alloc_calls: floor.alloc_calls,
        floor_live: floor.live_bytes(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ladder measures real, positive times and the delta is
    /// reported verbatim (small, possibly negative on noise — the
    /// honesty rule; only magnitude sanity is asserted here).
    #[test]
    fn ladder_measures_positive_times() {
        let _serial = crate::testutil::serial();
        let check = run().unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(check.rows.len(), 5, "the five size classes");
        for row in &check.rows {
            assert!(row.counting_ns > 0.0, "{} B counting path", row.size);
            assert!(row.system_ns > 0.0, "{} B system path", row.size);
            // A counter cost beyond 1 µs/pair would mean the
            // instrument is heavier than the thing measured — flag,
            // don't hide.
            assert!(
                row.delta_ns() < 1000.0,
                "{} B delta {} ns/pair is not credible",
                row.size,
                row.delta_ns()
            );
        }
    }
}
