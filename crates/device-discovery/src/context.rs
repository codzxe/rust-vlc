//! The bounded context every discovery pass runs under (doc 01 §2.4
//! sketch's `DiscoveryContext`; MP §12 — bounded budgets, directive
//! D-3).
//!
//! The context is advisory to implementations and binding to this
//! crate's own lanes: [`crate::monitor`] clamps its enumerate passes
//! to `max_volumes`, and [`crate::ssdp`] derives its receive windows
//! from `scan_budget`'s shape (its own deadline knob stays explicit —
//! a UDP search window is protocol-driven, see the MX discussion in
//! `src/ssdp.rs`).

use std::time::Duration;

/// Bounds for one discovery pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryContext {
    /// The maximum number of devices one pass may return (the pass
    /// fails with the typed capacity error beyond it — an OS that
    /// reports more volumes than this is a source contract violation,
    /// not a silent truncation).
    pub max_devices: usize,
    /// The advisory wall-clock budget for one pass. Implementations
    /// must not block materially beyond it.
    pub scan_budget: Duration,
}

/// The per-pass device ceiling (D-3: a runaway enumeration can never
/// grow the process — the pass fails loudly instead).
pub const MAX_DEVICES_CEILING: usize = 1024;

impl DiscoveryContext {
    /// The default context: 256 devices, 3 s scan budget (one
    /// mount-table read or one SSDP M-SEARCH window, comfortably).
    #[must_use = "constructs the pass bounds"]
    pub fn new() -> DiscoveryContext {
        DiscoveryContext {
            max_devices: 256,
            scan_budget: Duration::from_secs(3),
        }
    }

    /// A context with explicit bounds, clamped to the ceilings
    /// (`max_devices` to [`MAX_DEVICES_CEILING`]; budgets never below
    /// zero — `Duration` guarantees that — and never above 60 s).
    #[must_use = "constructs the pass bounds"]
    pub fn with_limits(max_devices: usize, scan_budget: Duration) -> DiscoveryContext {
        DiscoveryContext {
            max_devices: max_devices.clamp(1, MAX_DEVICES_CEILING),
            scan_budget: scan_budget.min(Duration::from_secs(60)),
        }
    }
}

impl Default for DiscoveryContext {
    fn default() -> Self {
        DiscoveryContext::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_bounded() {
        let ctx = DiscoveryContext::new();
        assert_eq!(ctx.max_devices, 256);
        assert_eq!(ctx.scan_budget, Duration::from_secs(3));
    }

    #[test]
    fn limits_clamp_to_the_ceilings() {
        let ctx = DiscoveryContext::with_limits(100_000, Duration::from_secs(999));
        assert_eq!(ctx.max_devices, MAX_DEVICES_CEILING);
        assert_eq!(ctx.scan_budget, Duration::from_secs(60));
        let zero = DiscoveryContext::with_limits(0, Duration::ZERO);
        assert_eq!(
            zero.max_devices, 1,
            "a zero ceiling is clamped up, never a black hole"
        );
    }
}
