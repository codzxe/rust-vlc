//! Per-instance host state: the `Store` data every import closes
//! over, plus the host-shared security ledger (doc 06 §5.3).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use plugin_api::PluginKind;

use crate::fsjail::PathJail;
use crate::http::HttpTransport;
use crate::limits::TokenBucket;
use wasmi::StoreLimits;

/// Embedder-provided services behind the import seam. v1 defaults
/// are honest empty seams: no jail roots, no transport, empty device
/// inventory.
#[derive(Default)]
pub struct HostServices {
    /// The `fs-sandboxed-read` jail (None = the permission may be
    /// declared, but no roots are granted — every open is denied).
    pub jail: Option<PathJail>,
    /// The HTTP transport (default [`NoTransport`]).
    pub transport: Option<Box<dyn HttpTransport>>,
    /// Device inventory snapshot text (read-only, `devices_list`).
    pub device_inventory: String,
}

impl std::fmt::Debug for HostServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostServices")
            .field(
                "jail",
                &self.jail.as_ref().map(|j| j.roots().len()),
            )
            .field("transport", &self.transport.is_some())
            .field("device_inventory_bytes", &self.device_inventory.len())
            .finish()
    }
}

impl HostServices {
    /// The all-empty v1 posture (typed denials everywhere a service
    /// is missing — never a panic, never a fabricated success).
    pub fn empty() -> HostServices {
        HostServices::default()
    }
}

/// A security event (doc 06 §5.3): the import that was attempted and
/// the *shapes* of its arguments — never raw buffers, never paths
/// that could carry secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityEvent {
    /// Import, e.g. `lumen:v1.http::http_get`.
    pub import: String,
    /// Why it was denied.
    pub reason: String,
    /// Argument shapes, e.g. `url_len=34`.
    pub arg_shape: String,
}

impl SecurityEvent {
    /// Record shape for the escalation log.
    pub fn new(import: &str, reason: impl Into<String>, arg_shape: impl Into<String>) -> Self {
        SecurityEvent {
            import: import.to_string(),
            reason: reason.into(),
            arg_shape: arg_shape.into(),
        }
    }
}

/// The session-wide escalation ledger shared by every instance of an
/// extension (doc 06 §5.3): events + the count that trips the
/// disable threshold. `Arc`-shared so the ledger outlives any single
/// (faulted, dropped) instance.
#[derive(Debug, Default)]
pub struct SecurityLedger {
    events: Mutex<Vec<SecurityEvent>>,
    escalations: AtomicU32,
}

impl SecurityLedger {
    /// A fresh, empty ledger.
    pub fn new() -> Arc<SecurityLedger> {
        Arc::new(SecurityLedger::default())
    }

    /// Record a permission escalation. Returns the escalation count
    /// after this event (the caller compares against the disable
    /// threshold, doc 06 §5.3 `[PROPOSAL: 3 per session]`).
    pub fn record_escalation(self: &Arc<Self>, event: SecurityEvent) -> u32 {
        let mut events = self.events.lock().unwrap_or_else(|p| p.into_inner());
        // Bounded log (D-2): the most recent 256 events.
        if events.len() >= 256 {
            events.remove(0);
        }
        events.push(event);
        self.escalations.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Total escalations this session.
    pub fn escalation_count(&self) -> u32 {
        self.escalations.load(Ordering::SeqCst)
    }

    /// A snapshot of the recorded events (install-UI / conformance).
    pub fn events(&self) -> Vec<SecurityEvent> {
        self.events
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

/// The manifest identity an instance must report honestly
/// (F1: the guest's `report_manifest` is checked against this).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestExpectation {
    /// Expected plugin id.
    pub id: String,
    /// Expected kind.
    pub kind: PluginKind,
    /// Expected version.
    pub version: String,
    /// Expected api major.
    pub api_version: u32,
}

/// One open sandboxed file (host side; the guest sees only the u32
/// index).
struct OpenFile {
    _path: String,
    content: Vec<u8>,
    pos: usize,
}

/// Per-instance handle table (doc 06 §5.4 row 7: 8 open handles).
#[derive(Default)]
pub struct HandleTable {
    files: Vec<Option<OpenFile>>,
}

impl HandleTable {
    /// Insert an opened file, returning its handle index.
    ///
    /// # Errors
    /// `E_LIMIT`-shaped string when the table is at its cap.
    pub fn insert(&mut self, path: String, content: Vec<u8>, cap: u32) -> Result<u32, String> {
        let len = self.files.len() as u32;
        if len >= cap {
            return Err(format!("handle table full ({len}/{cap})"));
        }
        self.files.push(Some(OpenFile {
            _path: path,
            content,
            pos: 0,
        }));
        Ok(len)
    }

    /// Read `max` bytes from handle `h` into a fresh buffer
    /// (bounded by the caller's transfer cap). `Ok(None)` = unknown
    /// handle.
    pub fn read(&mut self, h: u32, max: usize) -> Option<Vec<u8>> {
        let slot = self.files.get_mut(h as usize)?;
        let file = slot.as_mut()?;
        let remaining = file.content.len().saturating_sub(file.pos);
        let n = remaining.min(max);
        let out = file.content[file.pos..file.pos + n].to_vec();
        file.pos += n;
        Some(out)
    }

    /// Total size of handle `h`'s content.
    pub fn size(&self, h: u32) -> Option<u64> {
        self.files
            .get(h as usize)?
            .as_ref()
            .map(|f| f.content.len() as u64)
    }

    /// Close handle `h`; double-close is a typed `E_HANDLE`, closing
    /// an unknown handle too. `Err` carries the diagnostic shape.
    pub fn close(&mut self, h: u32) -> Result<(), String> {
        let slot = self.files.get_mut(h as usize).ok_or_else(|| {
            format!("handle {h} was never opened by this instance (ownership check, doc 06 §9.4)")
        })?;
        slot.take().ok_or_else(|| format!("handle {h} is already closed"))?;
        Ok(())
    }

    /// How many handles are open right now.
    pub fn open_count(&self) -> u32 {
        self.files.iter().filter(|s| s.is_some()).count() as u32
    }
}

/// Bounded in-memory log records (doc 06 §5.6 crash-report posture:
/// bounded local log; the last 64 records survive).
#[derive(Debug, Default)]
pub struct BoundedLog {
    records: VecDeque<(u8, String)>,
}

impl BoundedLog {
    fn push(&mut self, level: u8, line: String) {
        if self.records.len() >= 64 {
            self.records.pop_front();
        }
        self.records.push_back((level, line));
    }

    /// Recent records, oldest first (a bounded 64-entry clone — the
    /// ring's honest flat view; call frequency is UI/test-side, not
    /// hot-path).
    pub fn records(&self) -> Vec<(u8, String)> {
        self.records.iter().cloned().collect()
    }
}

/// A UI panel descriptor accepted through `lumen:v1.ui` (v1: recorded
/// host-side; the command-layer rendering protocol is doc 05/08
/// territory — this is the `ui-surface` seam).
#[derive(Debug, Clone)]
pub struct PanelRecord {
    /// The descriptor text the guest submitted (bounded).
    pub descriptor: String,
}

/// The `Store` data type: everything an import needs, owned by the
/// instance's wasmi `Store`.
pub struct InstanceState {
    /// wasmi store limits (the memory cap + grow-trap posture).
    pub store_limits: StoreLimits,
    /// I/O token bucket (rate/burst).
    pub io_bucket: TokenBucket,
    /// The resource limits this instance was spawned under.
    pub limits: crate::limits::ResourceLimits,
    /// The fs handle table.
    pub handles: HandleTable,
    /// HTTP rate window (timestamps of the last minute's requests).
    http_window: VecDeque<std::time::Instant>,
    /// The http allowlist (from the manifest).
    pub http_allowlist: Vec<String>,
    /// Log records.
    pub logs: BoundedLog,
    /// Accepted panels.
    pub panels: Vec<PanelRecord>,
    /// The manifest the guest last reported (F1 honesty check).
    pub manifest_report: Option<String>,
    /// What the guest is expected to report.
    pub expectation: ManifestExpectation,
    /// Session-shared escalation ledger.
    pub ledger: Arc<SecurityLedger>,
    /// Embedder services.
    pub services: HostServices,
    /// First-use deprecation notices already emitted.
    pub deprecation_notices_sent: Vec<&'static str>,
    /// Non-escalation import observations (conformance / tests):
    /// statuses returned by imports, in order.
    pub import_trace: Vec<(String, i32)>,
}

impl std::fmt::Debug for InstanceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceState")
            .field("expectation", &self.expectation)
            .field("open_handles", &self.handles.open_count())
            .field("escalations", &self.ledger.escalation_count())
            .field("panels", &self.panels.len())
            .finish_non_exhaustive()
    }
}

impl InstanceState {
    /// Assemble the state for a spawn.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        limits: crate::limits::ResourceLimits,
        store_limits: StoreLimits,
        expectation: ManifestExpectation,
        grants: &crate::api::Grants,
        ledger: Arc<SecurityLedger>,
        services: HostServices,
    ) -> InstanceState {
        InstanceState {
            store_limits,
            io_bucket: TokenBucket::new(limits.io_burst_bytes, limits.io_rate_bytes_per_sec),
            limits,
            handles: HandleTable::default(),
            http_window: VecDeque::new(),
            http_allowlist: grants.http_hosts.clone(),
            logs: BoundedLog::default(),
            panels: Vec::new(),
            manifest_report: None,
            expectation,
            ledger,
            services,
            deprecation_notices_sent: Vec::new(),
            import_trace: Vec::new(),
        }
    }

    /// Log a guest record (bounded; the level was already validated).
    pub fn log_record(&mut self, level: u8, line: String) {
        self.logs.push(level, line);
    }

    /// Record an import's returned status (conformance trace).
    pub fn trace_import(&mut self, import: &str, status: i32) {
        if self.import_trace.len() >= 1024 {
            self.import_trace.remove(0);
        }
        self.import_trace.push((import.to_string(), status));
    }

    /// Admit one HTTP request into the rolling-minute window
    /// (doc 06 §5.4 row 6). `false` = over the per-minute cap.
    pub fn http_admit(&mut self) -> bool {
        let now = std::time::Instant::now();
        while let Some(front) = self.http_window.front() {
            if now.duration_since(*front).as_secs() >= 60 {
                self.http_window.pop_front();
            } else {
                break;
            }
        }
        if self.http_window.len() >= self.limits.http_requests_per_min as usize {
            return false;
        }
        self.http_window.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> InstanceState {
        InstanceState::new(
            crate::limits::ResourceLimits::test_tiny(),
            wasmi::StoreLimitsBuilder::new()
                .memory_size(1024 * 1024)
                .trap_on_grow_failure(true)
                .build(),
            ManifestExpectation {
                id: "ext.test.x".into(),
                kind: PluginKind::MetadataProvider,
                version: "0.1.0".into(),
                api_version: 1,
            },
            &crate::api::Grants::default(),
            SecurityLedger::new(),
            HostServices::empty(),
        )
    }

    #[test]
    fn handle_table_caps_and_ownership() {
        let mut st = state();
        let a = ok(st.handles.insert("a".into(), vec![1, 2, 3], 2));
        let _b = ok(st.handles.insert("b".into(), vec![4], 2));
        assert!(st.handles.insert("c".into(), vec![], 2).is_err());
        // Bogus handles are refused (pointer smuggling, §9.4).
        assert!(st.handles.read(99, 4).is_none());
        assert!(st.handles.size(42).is_none());
        assert!(st.handles.close(42).is_err());
        // Real reads work and advance.
        assert_eq!(some(st.handles.read(a, 2)), vec![1, 2]);
        assert_eq!(some(st.handles.read(a, 2)), vec![3]);
        assert_eq!(some(st.handles.read(a, 2)), Vec::<u8>::new());
        // Double close refused.
        assert!(st.handles.close(a).is_ok());
        assert!(st.handles.close(a).is_err());
    }

    #[test]
    fn http_window_caps_at_limit() {
        let mut st = state(); // http_requests_per_min = 2 in tiny limits
        assert!(st.http_admit());
        assert!(st.http_admit());
        assert!(!st.http_admit(), "third request in the minute refused");
    }

    #[test]
    fn ledger_counts_escalations() {
        // `SecurityLedger::new()` hands out the Arc share every
        // instance carries (the ledger outlives faulted instances).
        let ledger = SecurityLedger::new();
        assert_eq!(
            ledger.record_escalation(SecurityEvent::new(
                crate::api::IMP_HTTP_GET,
                "host not in allowlist",
                "url_len=30"
            )),
            1
        );
        assert_eq!(
            ledger.record_escalation(SecurityEvent::new(
                crate::api::IMP_FS_OPEN,
                "symlink escape",
                "path_len=12"
            )),
            2
        );
        assert_eq!(ledger.escalation_count(), 2);
        let events = ledger.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].arg_shape, "url_len=30");
        assert!(!format!("{events:?}").contains("musicbrainz"));
    }

    #[test]
    fn bounded_log_keeps_last_64() {
        let mut st = state();
        for i in 0..100 {
            st.log_record(2, format!("line {i}"));
        }
        let records = st.logs.records();
        assert_eq!(records.len(), 64);
        assert_eq!(records[0].1, "line 36");
        assert_eq!(records[63].1, "line 99");
    }

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    /// The Option twin of [`ok`].
    fn some<T>(o: Option<T>) -> T {
        match o {
            Some(v) => v,
            None => unreachable!("expected Some, got None"),
        }
    }
}
