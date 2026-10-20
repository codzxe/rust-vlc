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
/// seam the engine's open path uses (doc 01 §2 step 2).
///
/// # Panics (test contract)
/// Only on an unparseable file URI (a temp/fixture path is always
/// valid) — a broken path is a test failure, reported loudly.
#[allow(clippy::missing_panics_doc)]
pub fn open_via_input(path: &Path) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
    let text = match path.to_str() {
        Some(text) => format!("file://{text}"),
        None => unreachable!("fixture paths are UTF-8 ({path:?})"),
    };
    let uri = match media_types::ids::SourceUri::parse(&text) {
        Ok(uri) => uri,
        Err(err) => unreachable!("fixture path is a valid file URI ({path:?}): {err}"),
    };
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
