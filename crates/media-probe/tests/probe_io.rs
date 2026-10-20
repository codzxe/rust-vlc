//! The probe-depth AC over the **real access stack** (card 2.A2; doc 03
//! §4.2(f)'s instrumented-stream property test), plus the doc's
//! integration trio — wrong extension, empty file, 1-byte file — and
//! the truncation sweep that pins the sub-magic cut to a typed unknown.
//!
//! Every test here probes through `input`'s `FileAccess` →
//! `FileStream` (card 2.A1's composition: the open path the engine
//! will run), with [`CountingStream`](media_probe::contract::CountingStream)
//! wrapping the stream so the per-request read ledger is observable —
//! the same instrumentation shape as `input`'s own
//! `contract::assert_bounded_read`, applied at the probe's window
//! instead of the reader's capacity.

mod common;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use common::{fixture, open_via_input, probe_file};
use media_probe::contract::CountingStream;
use media_probe::contract::{assert_position_restored, assert_probe_depth_bounded};
use media_probe::{ProbeConfig, ProbeRegistry, probe_stream};
use media_types::PipelineStage;
use plugin_api::SourceStream;
use testkit::Must;

/// Open a fixture through the real access stack **and** wrap it in the
/// counting adapter — the instrumented real source the depth-bound AC
/// asserts over.
fn open_counting(path: &Path) -> CountingStream {
    CountingStream::new(match open_via_input(path) {
        Ok(stream) => stream,
        Err(err) => unreachable!("fixture must open ({path:?}): {err}"),
    })
}

/// A unique scratch directory removed recursively on drop (the input
/// crate's tests/common pattern: pid + monotonic counter, recursive
/// cleanup, no `tempfile` dependency — C5).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let unique = format!("lumen-2a2-{tag}-{}-{n}", std::process::id());
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).must();
        TempDir(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).must();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A tiny deterministic byte generator (xorshift64*, the fixtures
/// writer's own PRNG family) — garbage inputs without a dependency.
fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed | 1;
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let word = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        for byte in word.to_le_bytes() {
            if out.len() < len {
                out.push(byte);
            }
        }
    }
    out
}

fn config(bound: usize) -> ProbeConfig {
    match ProbeConfig::new(bound, 60) {
        Ok(config) => config,
        Err(err) => unreachable!("{err}"),
    }
}

// ---------------------------------------------------------------------------
// The depth bound over the real stack (the card's AC)
// ---------------------------------------------------------------------------

#[test]
fn depth_bound_holds_over_the_real_input_stack_for_every_window() {
    let path = fixture("generated/valid/avi_mjpeg_pcm.avi");
    let registry = ProbeRegistry::with_builtins();
    for bound in [12usize, 64, 120, 1024, 4096, 65_536, 262_144] {
        let mut stream = open_counting(&path);
        let report = match probe_stream(&registry, &mut stream, &config(bound)) {
            Ok(report) => report,
            Err(err) => unreachable!("parent must probe at {bound}: {err}"),
        };
        assert_eq!(report.format.as_str(), "avi");
        assert!(report.bytes_examined <= bound);
        // The instrumented proof: total served ≤ the window, and every
        // single request asked for at most the remaining budget.
        assert_probe_depth_bounded(&stream, bound);
        assert_position_restored(&stream, plugin_api::SourcePosition(0));
        // The file is far larger than every bound here, so the fill is
        // exactly ONE window-sized request (the buffered reader serves
        // it from its fixed read-ahead, possibly across refills).
        assert_eq!(stream.requests(), &[bound], "one window-sized fill");
    }
}

#[test]
fn depth_bound_holds_at_the_doc_ceiling_over_the_real_stack() {
    // The hard cap itself (1 MiB) over a 64 KiB-capacity FileStream:
    // the buffered reader serves the larger ask across refills and the
    // probe never asks for more than the remaining budget.
    let path = fixture("generated/valid/mp4_mjpeg_pcm.mp4");
    let registry = ProbeRegistry::with_builtins();
    let bound = 1024 * 1024;
    let mut stream = open_counting(&path);
    let report = match probe_stream(&registry, &mut stream, &config(bound)) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "mp4");
    // The file (424_452 B) is smaller than the ceiling: the window is
    // the whole leading extent, one fill + one EOS probe.
    assert_eq!(report.bytes_examined, 424_452);
    assert_eq!(stream.requests(), &[bound, bound - 424_452]);
    assert_probe_depth_bounded(&stream, bound);
    assert_position_restored(&stream, plugin_api::SourcePosition(0));
}

#[test]
fn the_probe_never_consumes_real_stream_bytes() {
    // Doc 03 §4.2(e)'s stronger form over the real stack: after
    // probing, the stream serves the file's leading bytes again —
    // exactly what the selected demuxer will read as its prefix (the
    // 2.A1+2.A2 composition the engine's open path depends on).
    let path = fixture("generated/valid/avi_mjpeg_pcm.avi");
    let bytes = std::fs::read(&path).must();
    let registry = ProbeRegistry::with_builtins();
    for bound in [32usize, 4096, 65_536] {
        let mut stream = open_counting(&path);
        let report = match probe_stream(&registry, &mut stream, &config(bound)) {
            Ok(report) => report,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(report.bytes_examined, bound);
        let replay = match stream.read(bound) {
            Ok(bytes) => bytes,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(replay, bytes[..bound], "the prefix is served again");
        assert_eq!(stream.position().0, bound as u64);
    }
}

#[test]
fn the_real_ladder_climbs_with_the_window() {
    // The scored rungs as a function of the window, over real I/O —
    // the integration twin of the signature unit ladders.
    let registry = ProbeRegistry::with_builtins();
    let cases: [(&str, usize, u8); 8] = [
        ("generated/valid/avi_mjpeg_pcm.avi", 12, 70), // magic only
        ("generated/valid/avi_mjpeg_pcm.avi", 24, 80), // + LIST hdrl
        ("generated/valid/avi_mjpeg_pcm.avi", 32, 90), // + avih header
        ("generated/valid/avi_mjpeg_pcm.avi", 312, 95), // + both strl
        ("generated/valid/mp4_mjpeg_pcm.mp4", 28, 70), // ftyp only
        ("generated/valid/mp4_mjpeg_pcm.mp4", 36, 80), // + moov header
        ("generated/valid/mp4_mjpeg_pcm.mp4", 44, 90), // + mvhd header
        ("generated/valid/mp4_mjpeg_pcm.mp4", 2196, 95), // + both traks
    ];
    for (relative, bound, score) in cases {
        let mut stream = open_counting(&fixture(relative));
        let report = match probe_stream(&registry, &mut stream, &config(bound)) {
            Ok(report) => report,
            Err(err) => unreachable!("{relative} at {bound}: {err}"),
        };
        assert_eq!(
            report.confidence.as_u8(),
            score,
            "{relative} at window {bound}"
        );
        assert_eq!(report.bytes_examined, bound);
        assert_probe_depth_bounded(&stream, bound);
    }
}

// ---------------------------------------------------------------------------
// Doc 03 §4.2(f): wrong extension, empty file, 1-byte file
// ---------------------------------------------------------------------------

#[test]
fn extension_is_a_hint_not_truth() {
    let avi_bytes = std::fs::read(fixture("generated/valid/avi_mjpeg_pcm.avi")).must();
    let mp4_bytes = std::fs::read(fixture("generated/valid/mp4_mjpeg_pcm.mp4")).must();
    let dir = TempDir::new("ext");
    let default = ProbeConfig::default();

    // Valid AVI content wearing an .mp4 name: content wins.
    let named = dir.write("clip.mp4", &avi_bytes);
    let report = match probe_file(&named, &default) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "avi");
    assert_eq!(report.confidence.as_u8(), 95);

    // Valid MP4 content wearing an .avi name.
    let named = dir.write("clip.avi", &mp4_bytes);
    let report = match probe_file(&named, &default) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "mp4");
    assert_eq!(report.confidence.as_u8(), 95);

    // And on the no-hit path the extension rides the error's source
    // chain as an advisory hint — never the scoring.
    let garbage = dir.write("movie.avi", &seeded_bytes(0x2A2A_2A2A, 4096));
    let err = match probe_file(&garbage, &default) {
        Ok(report) => unreachable!("garbage must not probe: {report:?}"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "probe.no-demuxer");
    let hint = match std::error::Error::source(&err) {
        Some(source) => source.to_string(),
        None => unreachable!("hints ride the chain"),
    };
    assert!(hint.contains("extension=avi"), "got: {hint}");
    assert!(hint.contains("size=4096"), "got: {hint}");
}

#[test]
fn empty_and_one_byte_files_are_typed_unknowns() {
    let dir = TempDir::new("tiny");
    let default = ProbeConfig::default();
    for (name, bytes) in [("empty.avi", &[][..]), ("one-byte.mp4", &b"R"[..])] {
        let path = dir.write(name, bytes);
        let err = match probe_file(&path, &default) {
            Ok(report) => unreachable!("{name} must not probe: {report:?}"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "probe.no-demuxer", "{name}");
        assert_eq!(err.stage, PipelineStage::Probe, "{name}");
    }
}

#[test]
fn pure_garbage_files_are_typed_unknowns() {
    // The card's "garbage → unknown/low" axis over a real file: seeded
    // pseudorandom bytes carry no container magic, so the verdict is
    // the typed Unsupported — never a guess. (The corpus's garbage0
    // fixtures are seeded APPENDS past a pristine header and therefore
    // probe like their parents — the honest split is recorded in the
    // matrix's doc header.)
    let dir = TempDir::new("garbage");
    let default = ProbeConfig::default();
    for seed in [0x2A2A_2A2Au64, 0xDEAD_BEEF, 1] {
        let path = dir.write(&format!("garbage-{seed}.bin"), &seeded_bytes(seed, 8192));
        let err = match probe_file(&path, &default) {
            Ok(report) => unreachable!("garbage must not probe: {report:?}"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "probe.no-demuxer", "seed {seed}");
        assert_eq!(err.stage, PipelineStage::Probe, "seed {seed}");
        match &err.kind {
            media_types::ErrorKind::Unsupported { format } => {
                if let Some(info) = format {
                    assert!(
                        info.name.is_none(),
                        "seed {seed}: pure garbage names no format"
                    );
                }
            }
            other => unreachable!("seed {seed}: wrong kind: {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// The truncation sweep: where exactly "past the magic" turns unknown
// ---------------------------------------------------------------------------

#[test]
fn sub_magic_truncations_are_typed_unknowns_over_real_files() {
    // Every committed trunc fixture keeps at least the magic (2.A3's
    // trunc0 cuts *after* it), so the sub-magic axis is pinned here
    // over real files: cuts 0..11 of the AVI parent cannot carry the
    // RIFF+form magic and must be typed unknowns; the cut at exactly
    // 12 is the card's "trunc-after-magic → still AVI".
    let parent = std::fs::read(fixture("generated/valid/avi_mjpeg_pcm.avi")).must();
    let dir = TempDir::new("trunc");
    let default = ProbeConfig::default();
    for cut in [0usize, 1, 4, 8, 11] {
        let path = dir.write(&format!("cut-{cut}.avi"), &parent[..cut]);
        let err = match probe_file(&path, &default) {
            Ok(report) => unreachable!("cut {cut} must not probe: {report:?}"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "probe.no-demuxer", "cut {cut}");
        assert_eq!(err.stage, PipelineStage::Probe, "cut {cut}");
    }

    let path = dir.write("magic-only.avi", &parent[..12]);
    let report = match probe_file(&path, &default) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "avi");
    assert_eq!(report.confidence.as_u8(), 70);
    assert_eq!(report.bytes_examined, 12);
    assert!(report.track_hints.is_empty());
}

#[test]
fn truncated_files_prove_the_eos_fill_is_bounded() {
    // A window larger than the (truncated) file: the fill reads
    // exactly the file, asks once more for the remainder (EOS probe),
    // and never spins — the unit-suite EOS proof, over real I/O.
    let registry = ProbeRegistry::with_builtins();
    let path = fixture("generated/malformed/avi_mjpeg_pcm__trunc1.avi"); // 52 bytes
    let mut stream = open_counting(&path);
    let cfg = config(65_536);
    let report = match probe_stream(&registry, &mut stream, &cfg) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "avi");
    assert_eq!(report.confidence.as_u8(), 90);
    assert_eq!(report.bytes_examined, 52);
    assert_eq!(stream.requests(), &[65_536, 65_536 - 52]);
    assert_probe_depth_bounded(&stream, 65_536);
    assert_position_restored(&stream, plugin_api::SourcePosition(0));
    // The 52 surviving bytes are still all there after the probe.
    let replay = match stream.read(52) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("{err}"),
    };
    let want = std::fs::read(&path).must();
    assert_eq!(replay, want);
}

// ---------------------------------------------------------------------------
// Configured-depth typing over the real stack
// ---------------------------------------------------------------------------

#[test]
fn the_ceiling_is_enforced_before_any_byte_is_read() {
    // A configured lookahead above the 1 MiB hard cap is a typed
    // ResourceLimit at construction — the stream is never even opened
    // for it (the config is validated at the boundary; no code path
    // probes more).
    let err = match ProbeConfig::new(1024 * 1024 + 1, 60) {
        Ok(_) => unreachable!("over-ceiling must refuse"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "probe.lookahead-over-ceiling");
    assert_eq!(err.stage, PipelineStage::Probe);
}
