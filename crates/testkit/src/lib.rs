//! The Lumen test kit, v0 (doc 03 §8; card 1.C3).
//!
//! **Direction contract (MP §3.7):** this crate depends on production
//! crates; no production crate ever depends on it — `[dev-dependencies]`
//! entries only. Card 1.E1's `check-architecture` enforces the boundary
//! in CI; until then the proof is the manifest survey recorded in the
//! worklog plus the first consumer (`media-types`' dev-dependency).
//!
//! v0 carries the four helpers card 1.C3 names; the deterministic drivers
//! of doc 03 §8 — [`VirtualClock`] *implementation*, `ScriptSource`,
//! sinks, `SessionHarness`, leak instruments — land with the Phase-2
//! engine traits they exercise. What v0 pins now are the *shapes* those
//! drivers will grow into, so Phase-2 tests compile against stable names
//! from day one.
//!
//! # Lint policy note
//!
//! The workspace denies `panic!`/`unwrap`/`expect` even in tests, so the
//! helpers below return [`Result`] and offer [`Must::must`], which fails
//! through `unreachable!` — allowed by the lint set, and semantically
//! right for *fixture contract* violations (a missing fixture is a broken
//! test setup, not a runtime condition).

use std::fmt;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Counting-allocation instrument (ED-FIX-2, Lead decision ED-b)
// ---------------------------------------------------------------------------

pub mod alloc_count;
pub mod faults;

pub use alloc_count::{
    AllocDelta, AllocSnapshot, AllocState, CountingGlobalAlloc, CountingGuard, serialize_tests,
};

// ---------------------------------------------------------------------------
// Assert helpers
// ---------------------------------------------------------------------------

/// Extension trait giving `Result`/`Option` a lint-legal "must exist"
/// accessor for test fixtures and harness preconditions.
pub trait Must {
    type Value;

    /// Unwrap with caller-location context. Use for fixture contracts:
    /// a broken fixture means a broken *test*, never a product bug.
    #[track_caller]
    fn must(self) -> Self::Value;
}

impl<T, E: fmt::Display> Must for Result<T, E> {
    type Value = T;

    #[track_caller]
    fn must(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => unreachable!("testkit must(): {err}"),
        }
    }
}

impl<T> Must for Option<T> {
    type Value = T;

    #[track_caller]
    fn must(self) -> T {
        match self {
            Some(value) => value,
            None => unreachable!("testkit must(): unexpectedly None"),
        }
    }
}

/// Assert `haystack` contains `needle`, with a full-context failure.
#[track_caller]
pub fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context}: expected to find {needle:?} in:\n{haystack}"
    );
}

/// Assert two nanosecond values are within `tolerance` of each other —
/// the A/V-sync assertion primitive (doc 03 §8 tolerance columns).
#[track_caller]
pub fn assert_nanos_near(got: u64, want: u64, tolerance: u64, context: &str) {
    let delta = got.abs_diff(want);
    assert!(
        delta <= tolerance,
        "{context}: {got} ns differs from {want} ns by {delta} ns (tolerance {tolerance} ns)"
    );
}

/// Assert a closure returns `Err` (error paths are tested as first-class
/// outcomes, MP §8) and hand the error back for kind assertions.
#[track_caller]
pub fn assert_err<T, E>(result: Result<T, E>, context: &str) -> E {
    match result {
        Ok(_) => unreachable!("{context}: expected an error, got a success"),
        Err(err) => err,
    }
}

// ---------------------------------------------------------------------------
// External-oracle presence (W25 CI repair)
// ---------------------------------------------------------------------------

/// True when BOTH `ffmpeg` and `ffprobe` are on `PATH`.
///
/// The differential suites (doc 04 corpus plan; D-5 "escape the echo
/// chamber" + D-11 "ffprobe/ffmpeg as the external oracle") compare
/// Lumen against the system ffmpeg. The standing posture —
/// `scripts/oracle_check.sh` header — is that neither the local gate
/// nor CI may HARD-require ffmpeg: tests that need the oracle skip
/// loudly when it is absent instead of panicking on a failed spawn
/// (`testkit must(): No such file or directory`, the W25 CI failure).
///
/// `LUMEN_REQUIRE_ORACLE=1` inverts the posture for local strictness:
/// presence checks then FAIL the test instead of skipping, so a
/// developer who believes the oracle is installed finds out loudly.
pub fn oracle_available() -> bool {
    // lumen-allow-literal: env-var name — test-harness strictness knob (doc 04), not product config
    if std::env::var_os("LUMEN_REQUIRE_ORACLE").is_some_and(|v| v == "1") {
        assert!(
            on_path("ffmpeg") && on_path("ffprobe"),
            // lumen-allow-literal: diagnostic quoting the harness knob’s own name — user-facing text, not config
            "LUMEN_REQUIRE_ORACLE=1 but ffmpeg/ffprobe are not on PATH"
        );
    }
    on_path("ffmpeg") && on_path("ffprobe")
}

/// Emit the standard skip notice and return from the calling test.
///
/// Usage — first lines of an oracle-dependent test:
/// ```text
/// if !testkit::oracle_available() {
///     testkit::skip_no_oracle("mkv fixture differential");
///     return;
/// }
/// ```
pub fn skip_no_oracle(what: &str) {
    eprintln!(
        // lumen-allow-literal: skip diagnostic quoting the harness knob’s name — user-facing text, not config
        "oracle: ffmpeg/ffprobe not on PATH — skipping {what} \
         (install ffmpeg, or set LUMEN_REQUIRE_ORACLE=1 to make absence a failure)"
    );
}

fn on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| path_has(&dir, program))
}

/// Does `dir` contain an executable-looking `program`? Pure and
/// unit-testable (no env mutation — edition 2024 makes that unsafe
/// and the workspace forbids unsafe code).
#[cfg(target_family = "windows")]
fn path_has(dir: &Path, program: &str) -> bool {
    [dir.join(format!("{program}.exe")), dir.join(program)]
        .into_iter()
        .any(|p| p.is_file())
}

#[cfg(not(target_family = "windows"))]
fn path_has(dir: &Path, program: &str) -> bool {
    dir.join(program).is_file()
}

// ---------------------------------------------------------------------------
// Fixture-path helpers
// ---------------------------------------------------------------------------

/// Errors from fixture resolution — actionable, path-carrying, never a
/// bare "not found".
#[derive(Debug)]
pub enum FixtureError {
    /// The workspace root could not be located (no `[workspace]`
    /// manifest above `start`).
    NoWorkspaceRoot { start: PathBuf },
    /// The fixture path does not exist on disk.
    Missing { path: PathBuf },
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureError::NoWorkspaceRoot { start } => {
                write!(f, "no [workspace] Cargo.toml above {}", start.display())
            }
            FixtureError::Missing { path } => {
                write!(f, "fixture missing: {}", path.display())
            }
        }
    }
}

/// Locate the workspace root the way `cargo` does: walk ancestors from
/// `start` looking for a `Cargo.toml` containing a `[workspace]` table.
fn workspace_root_from(start: &Path) -> Result<PathBuf, FixtureError> {
    for dir in start.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest)
            && text
                .lines()
                .any(|line| line.trim_start().starts_with("[workspace]"))
        {
            return Ok(dir.to_path_buf());
        }
    }
    Err(FixtureError::NoWorkspaceRoot {
        start: start.to_path_buf(),
    })
}

/// The workspace root, located from the current directory.
pub fn workspace_root() -> Result<PathBuf, FixtureError> {
    let here = std::env::current_dir().map_err(|_| FixtureError::NoWorkspaceRoot {
        start: PathBuf::from("."),
    })?;
    workspace_root_from(&here)
}

/// Resolve a fixture path against the workspace's `tests/fixtures/`
/// root and verify it exists.
///
/// Fixture layout (doc 04 corpus plan): `tests/fixtures/<area>/<name>`.
/// The existence check happens *here* so a typo'd fixture name fails
/// with the resolved path, not three layers later inside a parser.
pub fn fixture_path(relative: &str) -> Result<PathBuf, FixtureError> {
    let path = workspace_root()?.join("tests/fixtures").join(relative);
    if path.exists() {
        Ok(path)
    } else {
        Err(FixtureError::Missing { path })
    }
}

/// Read a fixture as bytes (existence-checked).
pub fn fixture_bytes(relative: &str) -> Result<Vec<u8>, FixtureError> {
    let path = fixture_path(relative)?;
    std::fs::read(&path).map_err(|_| FixtureError::Missing { path })
}

// ---------------------------------------------------------------------------
// Virtual-time trait stub (doc 03 §8)
// ---------------------------------------------------------------------------

/// Virtual time for deterministic tests (doc 03 §8).
///
/// v0 pins the *shape*: manual advance, monotonic read. The full
/// `PlaybackClock` implementation (scripted drift, auto-advance modes)
/// lands with the Phase-2 clock crate, which will provide
/// `impl PlaybackClock for VirtualClock`. Tests written against this
/// trait today keep compiling when that lands.
pub trait VirtualClock {
    /// Advance virtual time by `by` nanoseconds. Monotonic: the clock
    /// never runs backwards; a zero advance is a no-op.
    fn advance(&mut self, by_ns: u64);

    /// The current virtual time in nanoseconds since session start.
    fn now_ns(&self) -> u64;
}

/// A minimal [`VirtualClock`] for v0 wiring tests — plain monotonic
/// nanoseconds, no drift scripting (Phase 2 replaces this with the full
/// driver; the trait is the stable surface).
#[derive(Debug, Default, Clone)]
pub struct StepClock {
    now: u64,
}

impl StepClock {
    /// A clock at zero.
    pub fn new() -> StepClock {
        StepClock { now: 0 }
    }
}

impl VirtualClock for StepClock {
    fn advance(&mut self, by_ns: u64) {
        self.now = self.now.saturating_add(by_ns);
    }

    fn now_ns(&self) -> u64 {
        self.now
    }
}

// ---------------------------------------------------------------------------
// Evidence recording (doc 10 §7.1 labels + §7.2 envelope)
// ---------------------------------------------------------------------------

/// The hardware class axis of doc 10 §7.1: mock evidence is virtual
/// devices and CI VMs; real evidence is physical hardware on the
/// reference machine. Every artifact carries the label — mock evidence
/// may support `implemented`/`partial` matrix claims, never
/// platform-`verified` ones (doc 10 §7.1, doc 02 R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareClass {
    /// VirtualClock, null/software sinks, CI VMs.
    Mock,
    /// Real audio device, GPU output, physical media.
    Real,
}

impl HardwareClass {
    /// The envelope's lowercase string.
    pub fn as_str(self) -> &'static str {
        match self {
            HardwareClass::Mock => "mock",
            HardwareClass::Real => "real",
        }
    }
}

/// One recorded evidence line: a claim and how it was observed.
#[derive(Debug, Clone)]
pub struct EvidenceEntry {
    /// What is being claimed (stable dotted key, e.g.
    /// `time.conversion.overflow_checked`).
    pub claim: String,
    /// How it was observed (command, test name, probe).
    pub observed_via: String,
}

/// A labeled evidence record per doc 10 §7 — the file the parity tool
/// and the feature matrix consume. v0 writes the human-stable text form;
/// the JSON envelope (`schema: "evidence/1"`) is assembled by the xtask
/// gate from these records, keeping one serialization owner.
#[derive(Debug, Clone)]
pub struct EvidenceRecord {
    /// Short record name (matches the producing card, e.g. `1.C1-time`).
    pub name: String,
    /// Mock vs real (doc 10 §7.1).
    pub hardware_class: HardwareClass,
    /// The claims with their observations.
    pub entries: Vec<EvidenceEntry>,
}

impl EvidenceRecord {
    /// A new record with the given name and label.
    pub fn new(name: &str, hardware_class: HardwareClass) -> EvidenceRecord {
        EvidenceRecord {
            name: name.to_string(),
            hardware_class,
            entries: Vec::new(),
        }
    }

    /// Record one claim and how it was observed.
    pub fn claim(mut self, claim: &str, observed_via: &str) -> Self {
        self.entries.push(EvidenceEntry {
            claim: claim.to_string(),
            observed_via: observed_via.to_string(),
        });
        self
    }

    /// Render the stable text form (the shape the wave-review gate
    /// reads; ordering is preserved as recorded).
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "evidence: {} [hardware_class={}]\n",
            self.name,
            self.hardware_class.as_str()
        ));
        for entry in &self.entries {
            out.push_str(&format!("  {} :: {}\n", entry.claim, entry.observed_via));
        }
        out
    }

    /// Write the record under `dir` as `<name>.txt`, creating the
    /// directory. Evidence files are append-only artifacts: an existing
    /// file with DIFFERENT content is an error (silent overwrite would
    /// falsify history), an identical rewrite is a no-op.
    pub fn write_to(&self, dir: &Path) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.txt", self.name));
        let rendered = self.render();
        if let Ok(existing) = std::fs::read_to_string(&path)
            && existing != rendered
        {
            // Divergent rewrite: surface it loudly rather than falsify.
            return Err(std::io::Error::other(format!(
                "evidence record {} already exists with different content",
                path.display()
            )));
        }
        std::fs::write(&path, rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn must_unwraps_results_and_options_with_context() {
        let ok: Result<u32, String> = Ok(7);
        assert_eq!(ok.must(), 7);
        let some: Option<&str> = Some("x");
        assert_eq!(some.must(), "x");
        let err: Result<u32, String> = Err("broken".to_string());
        // The failure path is exercised through catch_unwind to keep the
        // test green while proving the message carries context.
        let caught = std::panic::catch_unwind(|| {
            let _ = err.must();
        });
        assert!(caught.is_err());
    }

    #[test]
    fn assert_helpers_fail_with_context() {
        assert_contains("hello world", "world", "greeting");
        assert_nanos_near(1000, 1005, 10, "sync");
        let r: Result<u32, String> = Err("nope".to_string());
        let e = assert_err(r, "error path");
        assert_eq!(e, "nope");
        // Failure paths: proven once each via catch_unwind.
        assert!(std::panic::catch_unwind(|| assert_contains("a", "b", "ctx")).is_err());
        assert!(std::panic::catch_unwind(|| assert_nanos_near(0, 100, 10, "ctx")).is_err());
        let ok: Result<u32, String> = Ok(1);
        assert!(
            std::panic::catch_unwind(|| {
                let _ = assert_err(ok, "ctx");
            })
            .is_err()
        );
    }

    #[test]
    fn fixture_path_resolves_and_reports_missing() {
        // The workspace root must be locatable from the test cwd.
        let root = workspace_root().must();
        assert!(root.join("Cargo.toml").is_file());
        // tests/fixtures/ does not exist yet (corpus lands with doc 04's
        // plan) — the error must carry the resolved path.
        match fixture_path("area/nothing.bin") {
            Err(FixtureError::Missing { path }) => {
                assert!(
                    path.ends_with("tests/fixtures/area/nothing.bin"),
                    "{path:?}"
                );
            }
            other => unreachable!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn step_clock_is_monotonic() {
        let mut clock = StepClock::new();
        assert_eq!(clock.now_ns(), 0);
        clock.advance(1_000_000_000);
        clock.advance(500_000_000);
        assert_eq!(clock.now_ns(), 1_500_000_000);
    }

    #[test]
    fn evidence_record_renders_and_writes() {
        let record = EvidenceRecord::new("1.C0-wiring", HardwareClass::Mock)
            .claim("testkit.evidence.render", "unit test")
            .claim("testkit.evidence.label", "doc 10 §7.1");
        let text = record.render();
        assert_contains(&text, "[hardware_class=mock]", "label");
        assert_contains(&text, "testkit.evidence.render :: unit test", "entry");

        let dir = std::env::temp_dir().join(format!("lumen-testkit-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        record.write_to(&dir).must();
        // Idempotent rewrite: same content is a no-op...
        record.write_to(&dir).must();
        // ...divergent content is an error.
        let divergent = EvidenceRecord::new("1.C0-wiring", HardwareClass::Mock)
            .claim("testkit.evidence.changed", "unit test");
        assert!(divergent.write_to(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The oracle-presence probe must be a plain PATH scan: a directory
    /// containing an `ffmpeg` file reports present, an empty one does
    /// not (the W25 CI-runner condition the helper exists for). No env
    /// mutation — edition 2024 makes that unsafe and the workspace
    /// forbids unsafe code.
    #[test]
    fn oracle_path_scan_finds_programs_only_where_they_exist() {
        let dir = std::env::temp_dir().join(format!("lumen-testkit-path-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch dir — test-only
        std::fs::create_dir_all(&dir).must();
        let ffmpeg = dir.join("ffmpeg");
        std::fs::write(&ffmpeg, b"#!/bin/sh\n").must();
        assert!(path_has(&dir, "ffmpeg"), "file present -> found");
        assert!(!path_has(&dir, "ffprobe"), "no file -> not found");
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).must();
        assert!(!path_has(&empty, "ffmpeg"), "empty dir -> not found");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
