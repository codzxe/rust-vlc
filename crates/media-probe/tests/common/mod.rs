//! Shared helpers for the media-probe integration suites (card 2.A2):
//! the real-access-stack composition both suites probe through.
//!
//! `input`'s `FileAccess` (card 2.A1) opens the file as a plugin-api
//! `SourceStream` — the engine's open path (doc 01 §2 step 2), not an
//! in-memory stand-in (the demux/codecs test shape).

use std::path::{Path, PathBuf};

use media_probe::{ProbeConfig, ProbeRegistry, ProbeReport, probe_stream};
use media_types::EngineError;
use plugin_api::Access;
use testkit::Must;

/// Open a path through `input`'s `FileAccess` — the plugin-api `Access`
/// seam the engine's open path uses (doc 01 §2 step 2). The URI is
/// built by the shared platform-aware `SourceUri::from_file_path`
/// constructor (POSIX outputs byte-exact; a non-local path is the
/// constructor's typed refusal, propagated — never a mis-built URI).
///
/// # Panics (test contract)
/// Only on a non-UTF-8 path (a broken fixture — testkit's `must`
/// vocabulary); URI-construction failures are returned typed.
#[allow(clippy::missing_panics_doc)]
pub fn open_via_input(path: &Path) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
    let uri = media_types::ids::SourceUri::from_file_path(path.to_str().must())?;
    let access = input::FileAccess::new();
    access.open(&uri)
}

/// Probe a file through the real access stack (uninstrumented — the
/// classification matrix path; `report.bytes_examined` already makes
/// the window observable).
pub fn probe_file(path: &Path, config: &ProbeConfig) -> Result<ProbeReport, EngineError> {
    let mut stream = match open_via_input(path) {
        Ok(stream) => stream,
        Err(err) => unreachable!("fixture must open ({path:?}): {err}"),
    };
    let registry = ProbeRegistry::with_builtins();
    probe_stream(&registry, &mut *stream, config)
}

/// Resolve a fixture under `tests/fixtures/` (testkit's path helper
/// pinned to the corpus root).
pub fn fixture(relative: &str) -> PathBuf {
    testkit::fixture_path(relative).must()
}
