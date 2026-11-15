//! The GUI-failure diagnostics core (w30.6, the "shell-face" lane;
//! audit E §A.2/§F.2 — `tmp/audit/E-desktop-tauri.md`).
//!
//! A GUI-subsystem binary (`windows_subsystem = "windows"`, the w28
//! "normal app" rule) has NO failure face on a double-click launch:
//! Windows allocates no console, Rust std maps the null stderr handle
//! to a silent sink, so `eprintln!` + `exit(1)` is indistinguishable
//! from "nothing happened" — the exact shape of the v0.1.2 user report
//! ("brief loading then nothing"). This module is the std-only half of
//! the fix: a **deterministic crash log** every failure path can reach,
//! plus a **panic hook** so even exit-101 panics leave the same
//! diagnosable artifact. The Windows-native half (the MessageBox shown
//! when nothing else can display the failure) lives in the excluded
//! crate's `main.rs` — it needs `unsafe` FFI, which this crate forbids
//! by C5/boundaries (`std-only, no unsafe`).
//!
//! # The sink contract
//!
//! One fixed, findable file — `<OS temp dir>/lumen/lumen-tauri-crash.log`
//! — latest failure wins (overwrite, not append: a user attaching the
//! log to a bug report must not paste six months of panics). The
//! stamped variant exists for rotation and for tests that must not
//! clobber the real sink. On Windows the temp dir is per-user
//! (`%TEMP%`); on shared Unix hosts the fixed path can collide across
//! users — accepted for a best-effort diagnostic sink (the log is
//! never a correctness input).
//!
//! # The best-effort contract
//!
//! [`write_crash_log`] NEVER panics and NEVER masks the failure being
//! reported: every IO error (temp dir read-only, disk full, a file
//! squatting on the directory name) is swallowed, and the INTENDED
//! path is returned regardless — so the caller can still name the
//! location in its stderr receipt and dialog. Pinned by test below
//! (a sabotaged sink returns the path, no panic).
//!
//! # Std-only note
//!
//! No `chrono`/`time`: the UTC timestamp is unix seconds → ISO 8601 by
//! the civil-from-days integer algorithm (Howard Hinnant, public
//! domain), unit-tested against known instants including a leap day.

use std::any::Any;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The per-product namespace directory under the OS temp dir (the sink
/// contract above: one fixed place support can tell a user to look).
// lumen-allow-literal: crash-sink directory name — the per-product namespace under the OS temp dir (audit E §F.2); deterministic findability is the feature, not a display surface
const CRASH_LOG_DIR: &str = "lumen";

/// The sink file name: fixed, latest-wins, printed by the failure
/// dialog and by the stderr receipt.
// lumen-allow-literal: crash-sink file name — the fixed latest-wins log the failure dialog prints and CI receipts name (audit E §F.2); the sink's findability contract, not a config-escaped identity
const CRASH_LOG_FILE: &str = "lumen-tauri-crash.log";

/// The stage the panic hook reports under (startup failures report
/// `"startup"`; the value is caller-chosen, this pins the hook's).
const STAGE_PANIC: &str = "panic";

/// The deterministic crash-log sink (the sink contract above):
/// `<temp_dir>/lumen/lumen-tauri-crash.log`, overwriting on every
/// write so the latest failure is the one a user attaches.
pub fn crash_log_path() -> PathBuf {
    std::env::temp_dir()
        .join(CRASH_LOG_DIR)
        .join(CRASH_LOG_FILE)
}

/// The rotatable sibling of [`crash_log_path`]: same directory, a
/// `-<stamp>` infix in the file name. Tests use it to avoid clobbering
/// the real sink; a caller wanting to keep N failures can stamp with
/// the unix seconds of the moment.
pub fn crash_log_path_with_stamp(stamp: &str) -> PathBuf {
    let stem = CRASH_LOG_FILE
        .strip_suffix(".log")
        .unwrap_or(CRASH_LOG_FILE);
    std::env::temp_dir()
        .join(CRASH_LOG_DIR)
        .join(format!("{stem}-{stamp}.log"))
}

/// Write the crash log for `error` to the default sink and return the
/// path written to (best-effort contract: on IO failure the INTENDED
/// path is returned anyway, never an error, never a panic — the caller
/// surfaces it, this fn must not become a second failure).
///
/// The body records the product version (this crate's package version,
/// the library half of the shipping pair), the platform, `stage` (the
/// caller's lifecycle word — `"startup"` from the shell's main,
/// `"panic"` from the hook), a UTC timestamp, the FULL `source()` chain
/// of the error, and the how-to-report line.
pub fn write_crash_log(stage: &str, error: &dyn Error) -> PathBuf {
    write_crash_log_at(&crash_log_path(), stage, &error_chain(error))
}

/// Compose and (best-effort) write a crash log at an explicit `path` —
/// the writer behind [`write_crash_log`], private because the public
/// API pins the one deterministic sink; tests use it for isolated
/// sinks and the sabotaged-sink contract.
fn write_crash_log_at(path: &Path, stage: &str, error_text: &str) -> PathBuf {
    let body = crash_log_body(stage, error_text);
    if let Some(dir) = path.parent() {
        // Best-effort contract: a failing sink must never mask the
        // failure being reported nor panic. Errors are swallowed; the
        // write below then fails the same honest way.
        if std::fs::create_dir_all(dir).is_err() {
            // fall through
        }
    }
    if std::fs::write(path, body).is_err() {
        // swallowed by the same contract — see the module docs
    }
    path.to_path_buf()
}

/// The full log body: banner, version, platform, stage, UTC time, the
/// caller's error text, the how-to-report line.
fn crash_log_body(stage: &str, error_text: &str) -> String {
    let (secs, nanos) = unix_now();
    let mut body = String::new();
    // lumen-allow-literal: log banner — the self-describing first line of the sink file (matches the file name, so an attached log names its product); diagnostics prose, not a display surface
    body.push_str("lumen-tauri crash log\n");
    body.push_str("version: ");
    body.push_str(env!("CARGO_PKG_VERSION"));
    body.push_str("\nplatform: ");
    body.push_str(platform_label());
    body.push_str("\nstage: ");
    body.push_str(stage);
    body.push_str("\ntime: ");
    body.push_str(&iso_utc_from_unix_secs(secs));
    body.push_str(&format!(" (unix {secs}.{nanos:09}s)"));
    body.push_str("\n\n");
    body.push_str(error_text);
    body.push_str("\n\nHow to report: attach this file to your bug report or support request.\n");
    body
}

/// The error chain as log lines: the error, then every `source()` hop
/// one `caused by:` line deep (the `{:#}` Display discipline, walked
/// by hand so each cause is its own grep-able line).
fn error_chain(error: &dyn Error) -> String {
    let mut chain = format!("error: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        chain.push_str("\ncaused by: ");
        chain.push_str(&cause.to_string());
        source = cause.source();
    }
    chain
}

/// Install the process-global panic hook (idempotence note: calling it
/// twice chains hooks — the shell's `main` calls it exactly once,
/// before anything can fail).
///
/// The hook WRAPS, never replaces: the panic payload + location are
/// written to [`crash_log_path`] first (via [`write_crash_log`], so a
/// panic and a startup failure leave the SAME artifact shape), then
/// the previous hook runs — std's default keeps printing the payload
/// to stderr, so an attached console/redirected handle still sees the
/// classic face. The hook itself is panic-free by construction
/// (`payload_text` downcasts, the writer swallows IO).
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = payload_text(info.payload());
        let failure = PanicFailure { location, payload };
        let _sink = write_crash_log(STAGE_PANIC, &failure);
        // The wrapped hook keeps whatever face existed before us.
        previous(info);
    }));
}

/// A panic re-typed as an [`Error`] so the hook reuses the writer's
/// ONE chain-walking path (no source; the payload is the leaf).
#[derive(Debug)]
struct PanicFailure {
    location: String,
    payload: String,
}

impl fmt::Display for PanicFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panic at {}: {}", self.location, self.payload)
    }
}

impl Error for PanicFailure {}

/// The panic payload as text: `panic!("literal")` payloads are
/// `&'static str`, formatted ones are `String`, anything else (a
/// custom type) has no printable text — the location line still
/// identifies the site. Never panics.
fn payload_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// The compile-time platform label for the log header.
fn platform_label() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}

/// (unix seconds, subsecond nanos) now, degrading to the epoch on a
/// pre-epoch clock rather than failing (a wrong-by-construction
/// timestamp must not kill the crash log).
fn unix_now() -> (u64, u32) {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => (since.as_secs(), since.subsec_nanos()),
        Err(_) => (0, 0),
    }
}

/// Unix seconds → `YYYY-MM-DDTHH:MM:SSZ` (UTC, no deps). The
/// days→civil half is Howard Hinnant's `civil_from_days` (public
/// domain; the same integer math libc `gmtime` cores use), pinned
/// below against the epoch, an end-of-day, a leap day and a 2023
/// instant.
fn iso_utc_from_unix_secs(secs: u64) -> String {
    const SECS_PER_DAY: u64 = 86_400;
    let days = (secs / SECS_PER_DAY) as i64;
    let sod = secs % SECS_PER_DAY;
    // civil_from_days: z counts days since 1970-03-01 (a March-based
    // year puts the leap day at the END, so no month-length table).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        sod / 3_600,
        (sod % 3_600) / 60,
        sod % 60
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use testkit::{Must, assert_contains};

    /// Tests that read or write the REAL default sink serialize on one
    /// lock: the lib's unit tests run as parallel threads of one
    /// process, and the sink is a fixed latest-wins FILE by design —
    /// unserialized asserts would read mid-overwrite.
    static DEFAULT_SINK: Mutex<()> = Mutex::new(());

    // -- the typed three-level chain fixture (source()-linked) --------

    #[derive(Debug)]
    struct RootCause;

    impl fmt::Display for RootCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "the root cause")
        }
    }

    impl Error for RootCause {}

    #[derive(Debug)]
    struct MiddleCause {
        root: RootCause,
    }

    impl fmt::Display for MiddleCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "the middle cause")
        }
    }

    impl Error for MiddleCause {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.root)
        }
    }

    #[derive(Debug)]
    struct StartupFailure {
        middle: MiddleCause,
    }

    impl fmt::Display for StartupFailure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "the startup failure")
        }
    }

    impl Error for StartupFailure {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.middle)
        }
    }

    #[test]
    fn crash_log_path_is_the_deterministic_temp_sink() {
        let path = crash_log_path();
        let dir = std::env::temp_dir().join(CRASH_LOG_DIR);
        assert!(path.starts_with(&dir), "sink must sit under {dir:?}");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(CRASH_LOG_FILE)
        );
    }

    #[test]
    fn with_stamp_rotates_alongside_without_clobbering_the_default() {
        let plain = crash_log_path();
        let stamped = crash_log_path_with_stamp("w30");
        assert_eq!(stamped.parent(), plain.parent());
        assert_ne!(stamped, plain, "a stamp must move the file name");
        assert_ne!(
            stamped,
            crash_log_path_with_stamp("w31"),
            "distinct stamps must not collide"
        );
        let name = stamped.file_name().and_then(|n| n.to_str()).must();
        // lumen-allow-literal: test expectation — the stamped sink name under test (pins the exact rotation shape against the constant above)
        assert_eq!(name, "lumen-tauri-crash-w30.log");
    }

    #[test]
    fn iso_utc_pins_known_instants() {
        assert_eq!(iso_utc_from_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc_from_unix_secs(86_399), "1970-01-01T23:59:59Z");
        assert_eq!(iso_utc_from_unix_secs(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(
            iso_utc_from_unix_secs(1_700_000_000),
            "2023-11-14T22:13:20Z"
        );
    }

    #[test]
    fn write_crash_log_records_version_stage_and_full_chain() {
        let _sink = DEFAULT_SINK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let failure = StartupFailure {
            middle: MiddleCause { root: RootCause },
        };
        let path = write_crash_log("startup", &failure);
        assert_eq!(path, crash_log_path());
        let log = fs::read_to_string(&path).unwrap_or_default();
        assert!(!log.is_empty(), "a successful write must be readable");
        assert_contains(&log, env!("CARGO_PKG_VERSION"), "version line");
        assert_contains(&log, "platform: ", "platform line");
        assert_contains(&log, "stage: startup", "stage line");
        // The time line, round-trip pinned: the logged ISO stamp must
        // re-derive from the logged unix seconds (era-independent —
        // the calendar math itself is pinned by
        // iso_utc_pins_known_instants above).
        let time_line = log.lines().find(|line| line.starts_with("time: ")).must();
        let tail = time_line.strip_prefix("time: ").must();
        let (iso, unix) = tail.split_once(" (unix ").must();
        assert!(unix.ends_with("s)"), "sub-second seconds tail: {unix:?}");
        let secs: u64 = unix
            .trim_end_matches("s)")
            .split('.')
            .next()
            .must()
            .parse()
            .must();
        assert_eq!(iso, iso_utc_from_unix_secs(secs));
        assert_contains(&log, "error: the startup failure", "chain head");
        assert_contains(&log, "caused by: the middle cause", "chain hop 1");
        assert_contains(&log, "caused by: the root cause", "chain hop 2");
        assert_contains(&log, "How to report", "how-to-report line");
    }

    #[test]
    fn a_failing_sink_returns_the_intended_path_and_never_panics() {
        // The sink is sabotaged deterministically: a regular FILE
        // occupies the parent-dir slot, so create_dir_all and write
        // both fail with typed IO errors — the contract under test is
        // "swallow, return the intended path anyway, no panic".
        let base = std::env::temp_dir().join(format!("shell-face-blocker-{}", std::process::id()));
        let _ignore = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).must();
        let blocker = base.join("blocker");
        fs::write(&blocker, b"occupied").must();
        let intended = blocker.join("nested").join("crash.log");
        let returned = write_crash_log_at(&intended, "startup", "error: probe");
        assert_eq!(returned, intended, "the intended path comes back");
        assert!(
            !intended.exists(),
            "no log can exist under a regular-file parent"
        );
        fs::remove_dir_all(&base).must();
    }

    #[test]
    // The one annotated `panic!` in the crate: the hook under test can
    // only be exercised by a REAL panic, and the workspace lint set
    // denies `panic!` even in tests (testkit's header) — the item-level
    // allow is the media-types `error.rs` precedent.
    #[allow(clippy::panic)]
    fn panic_hook_writes_the_log_and_still_runs_the_previous_hook() {
        let _sink = DEFAULT_SINK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::panic::take_hook();
        static WRAPPED_HOOK_RAN: AtomicBool = AtomicBool::new(false);
        std::panic::set_hook(Box::new(|_info| {
            WRAPPED_HOOK_RAN.store(true, Ordering::SeqCst);
        }));
        install_panic_hook();
        let marker = format!("shell-face hook probe {}", std::process::id());
        let caught = std::panic::catch_unwind(move || {
            // A REAL panic, not the assert family: the payload must be
            // a formatted `String` (both payload shapes are handled by
            // payload_text; this one exercises the String arm).
            panic!("payload {marker}")
        });
        assert!(caught.is_err(), "the probe must actually panic");
        std::panic::set_hook(previous);
        assert!(
            WRAPPED_HOOK_RAN.load(Ordering::SeqCst),
            "the wrapped (pre-existing) hook must still run"
        );
        let log = fs::read_to_string(crash_log_path()).unwrap_or_default();
        assert!(!log.is_empty(), "the hook must have written the sink");
        assert_contains(&log, "stage: panic", "the hook's stage line");
        assert_contains(&log, "shell-face hook probe", "the panic payload");
        assert_contains(&log, "diagnostics.rs", "the panic location file");
    }
}
