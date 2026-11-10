//! Resource limits (doc 06 §5.4) and the host-level accounting
//! ceilings (§5.4 last two rows).
//!
//! Every number here is `[PROPOSAL — doc 06 §5.4 marks the whole
//! table proposed]`; they are constants in one place so a Lead tune
//! is a one-file diff. wasmi gives us two enforcement primitives
//! (ADR-002): the store-level [`wasmi::StoreLimits`] memory cap and
//! fuel metering; everything else is host-side accounting.
//!
//! CPU budgets: doc 06 §5.4 specifies wall-clock deadlines with fuel
//! as the wasmi-path mechanism — with wasmi-only (ADR-002 decision 4)
//! the fuel budget **is** the deadline, deterministically: a tight
//! loop is killed at exactly `call_fuel` burned instructions
//! (§7.4's "killed within 2× budget" holds with factor 1).

use plugin_api::PluginKind;

/// Per-instance resource limits (doc 06 §5.4 rows 1–7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Linear-memory ceiling in bytes (row 1). Enforced by the
    /// wasmi store limiter; growth past the cap traps and faults the
    /// instance (§5.4 `MemoryOom`).
    pub memory_bytes: u64,
    /// Fuel granted to `instantiate_and_start` (row 3, the 500 ms
    /// startup deadline expressed as fuel).
    pub startup_fuel: u64,
    /// Fuel granted per dispatched call (row 2, the 100 ms
    /// per-call deadline expressed as fuel).
    pub call_fuel: u64,
    /// Largest single import buffer transfer in bytes (row 4).
    pub io_transfer_bytes: u64,
    /// Sustained I/O refill rate, bytes per second (row 5).
    pub io_rate_bytes_per_sec: u64,
    /// I/O burst capacity in bytes (row 5).
    pub io_burst_bytes: u64,
    /// HTTP requests per rolling minute (row 6).
    pub http_requests_per_min: u32,
    /// Open file handles per instance (row 7).
    pub max_open_handles: u32,
    /// Logged manifest-report / panel-descriptor / log-record byte
    /// caps (v1 ABI bounds, see `api.rs`).
    pub max_descriptor_bytes: u64,
}

/// Default linear-memory ceilings per doc 06 §5.4 row 1: 32 MiB for
/// the metadata/playlist/discovery kinds, 64 MiB for `ui-panel`
/// (and the filter kinds, which doc 06 sizes with ui-panel at 64 MiB).
pub const MEM_DEFAULT_BYTES: u64 = 32 * 1024 * 1024;
/// 64 MiB — the ui-panel/filter memory class (doc 06 §5.4 row 1).
pub const MEM_PANEL_BYTES: u64 = 64 * 1024 * 1024;

/// Fuel for ~100 ms of interpreted work `[PROPOSAL — calibration:
/// wasmi burns roughly 2–4 fuel per simple instruction and interprets
/// on the order of 1e8–1e9 fuel/second on 2026 desktop hardware;
/// 3e8 sits in that band]`. Tests never depend on the wall-clock
/// mapping: they set explicit tiny budgets (deterministic fuel
/// semantics, §7.4).
pub const CALL_FUEL: u64 = 300_000_000;

/// Fuel for the 500 ms instantiation/start deadline (doc 06 §5.4
/// row 3) `[PROPOSAL — same calibration note as [`CALL_FUEL`]]`.
pub const STARTUP_FUEL: u64 = 1_500_000_000;

/// 16 MiB — the single-transfer I/O cap (doc 06 §5.4 row 4).
pub const IO_TRANSFER_BYTES: u64 = 16 * 1024 * 1024;

/// 1 MiB/s sustained I/O refill (doc 06 §5.4 row 5).
pub const IO_RATE_BYTES_PER_SEC: u64 = 1024 * 1024;

/// 16 MiB I/O burst (doc 06 §5.4 row 5).
pub const IO_BURST_BYTES: u64 = 16 * 1024 * 1024;

/// 32 HTTP requests per rolling minute (doc 06 §5.4 row 6).
pub const HTTP_REQUESTS_PER_MIN: u32 = 32;

/// 8 open handles per instance (doc 06 §5.4 row 7).
pub const MAX_OPEN_HANDLES: u32 = 8;

/// 64 KiB — one ABI descriptor/report/record cap (v1 import shapes;
/// keeps a single guest call from dragging megabytes through the
/// seam in one shot; larger payloads are chunked by the guest).
pub const MAX_DESCRIPTOR_BYTES: u64 = 64 * 1024;

/// 4 active instances per host (doc 06 §5.4 row 8).
pub const HOST_MAX_ACTIVE_INSTANCES: usize = 4;

/// 256 MiB total extension heap across instances (doc 06 §5.4 row 9).
pub const HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024;

/// Escalation-attempt threshold per session before the extension is
/// disabled (doc 06 §5.3 `[PROPOSAL: threshold 3 per session]`).
pub const ESCALATION_DISABLE_THRESHOLD: u32 = 3;

impl ResourceLimits {
    /// The doc 06 §5.4 defaults for an extension of `kind`.
    ///
    /// Memory class by kind: `ui-panel`, `audio-filter` and
    /// `video-filter` get the 64 MiB class; every other
    /// v1-extension-legal kind gets 32 MiB. Everything else is
    /// kind-independent in v1.
    pub fn for_kind(kind: PluginKind) -> ResourceLimits {
        let memory_bytes = match kind {
            PluginKind::UiPanel | PluginKind::AudioFilter | PluginKind::VideoFilter => {
                MEM_PANEL_BYTES
            }
            _ => MEM_DEFAULT_BYTES,
        };
        ResourceLimits {
            memory_bytes,
            startup_fuel: STARTUP_FUEL,
            call_fuel: CALL_FUEL,
            io_transfer_bytes: IO_TRANSFER_BYTES,
            io_rate_bytes_per_sec: IO_RATE_BYTES_PER_SEC,
            io_burst_bytes: IO_BURST_BYTES,
            http_requests_per_min: HTTP_REQUESTS_PER_MIN,
            max_open_handles: MAX_OPEN_HANDLES,
            max_descriptor_bytes: MAX_DESCRIPTOR_BYTES,
        }
    }

    /// A deliberately tiny budget for deterministic limit tests
    /// (§7.4-style breach suites): 1 MiB memory, 10_000 call fuel,
    /// 1 KiB transfers, 1 byte/s refill with 1 KiB burst.
    pub fn test_tiny() -> ResourceLimits {
        ResourceLimits {
            memory_bytes: 1024 * 1024,
            startup_fuel: 50_000,
            call_fuel: 10_000,
            io_transfer_bytes: 1024,
            io_rate_bytes_per_sec: 1,
            io_burst_bytes: 1024,
            http_requests_per_min: 2,
            max_open_handles: 2,
            max_descriptor_bytes: 256,
        }
    }
}

/// A refill-rate token bucket (doc 06 §5.4 row 5): capacity
/// `burst`, refilled at `rate` bytes/second. Pure host-side; a guest
/// exceeding it observes the typed `E_QUOTA` status, never a hang.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: u64,
    rate_per_sec: u64,
    tokens: u64,
    last: std::time::Instant,
}

impl TokenBucket {
    /// New bucket, full.
    pub fn new(capacity: u64, rate_per_sec: u64) -> TokenBucket {
        TokenBucket {
            capacity,
            rate_per_sec,
            tokens: capacity,
            last: std::time::Instant::now(),
        }
    }

    /// Refill from elapsed wall time, then try to take `n` tokens.
    /// Returns `false` (leaving the bucket untouched) when short.
    pub fn take(&mut self, n: u64) -> bool {
        self.refill();
        if self.tokens >= n {
            self.tokens -= n;
            true
        } else {
            false
        }
    }

    /// Whether a single transfer of `n` bytes can ever be legal
    /// (bucket capacity bounds one-shot transfers).
    pub fn fits_capacity(&self, n: u64) -> bool {
        n <= self.capacity
    }

    fn refill(&mut self) {
        let elapsed = self.last.elapsed();
        let nanos = elapsed.as_nanos();
        // bytes = rate * nanos / 1e9, computed in u128 to avoid
        // overflow on long stalls; capped at capacity.
        let gained = (self.rate_per_sec as u128 * nanos) / 1_000_000_000u128;
        if gained > 0 {
            self.tokens = (self.tokens + gained as u64).min(self.capacity);
            self.last = std::time::Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_class_follows_doc06_row1() {
        assert_eq!(
            ResourceLimits::for_kind(PluginKind::UiPanel).memory_bytes,
            MEM_PANEL_BYTES
        );
        assert_eq!(
            ResourceLimits::for_kind(PluginKind::AudioFilter).memory_bytes,
            MEM_PANEL_BYTES
        );
        assert_eq!(
            ResourceLimits::for_kind(PluginKind::MetadataProvider).memory_bytes,
            MEM_DEFAULT_BYTES
        );
        assert_eq!(
            ResourceLimits::for_kind(PluginKind::PlaylistParser).memory_bytes,
            MEM_DEFAULT_BYTES
        );
    }

    #[test]
    fn bucket_burst_then_throttle() {
        // Capacity 10, rate 1/sec: burst drains, immediate retry is
        // refused, and after >10s the bucket is full again.
        let mut bucket = TokenBucket::new(10, 1);
        assert!(bucket.take(10));
        assert!(!bucket.take(1), "burst spent, no refill yet");
        assert!(bucket.fits_capacity(10));
        assert!(!bucket.fits_capacity(11));
    }

    #[test]
    fn bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(1000, 1000);
        assert!(bucket.take(1000));
        assert!(!bucket.take(1));
        // Sleep 5ms → ≥5 tokens refilled at 1000/s.
        std::thread::sleep(std::time::Duration::from_millis(6));
        assert!(bucket.take(5), "refill happened");
    }
}
