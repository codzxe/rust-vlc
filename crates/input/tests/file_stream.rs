//! File-access integration (card 2.A1 AC): the full stream contract
//! over real files, typed rejections for missing files, directories,
//! devices and non-file schemes, and honest growth behavior.

mod common;

use common::{TempDir, TempFile, pattern, running_privileged};
use input::FileAccess;
use input::contract::assert_stream_contract;
use media_types::error::AccessDenied;
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind};
use plugin_api::{Access, SourceStream};
use testkit::Must;

fn open(uri: &str) -> Result<Box<dyn SourceStream>, EngineError> {
    let access = FileAccess::new();
    let parsed = SourceUri::parse(uri).must();
    access.open(&parsed)
}

/// Read/seek/EOF over a real file, through the plugin-api trait: the
/// shared contract check (varied chunk sizes, position consistency,
/// clean sticky EOF, typed seek errors both sides of the length).
#[test]
fn real_file_passes_the_stream_contract() {
    let data = pattern(5000);
    let temp = TempFile::create("contract", &data).must();
    let mut stream = open(&temp.file_uri()).must();
    assert_stream_contract(stream.as_mut(), &data);
}

/// A missing file is the typed Inaccessible/NotFound error.
#[test]
fn missing_file_is_typed_not_found() {
    let err = testkit::assert_err(open("file:///nonexistent-lumen-2a1/absent.mkv"), "missing");
    assert_eq!(err.context.code, "input.file.missing");
    assert_eq!(
        err.kind,
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound
        }
    );
}

/// A directory is not a byte source: typed rejection (unbounded reads
/// from devices are the neighbor hazard this guard closes).
#[test]
fn directory_is_a_typed_rejection() {
    let dir = TempDir::create("not-a-file").must();
    let err = testkit::assert_err(open(&dir.file_uri()), "directory");
    assert_eq!(err.context.code, "input.file.not-a-regular-file");
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }), "{err}");
}

/// A device node (`/dev/null`, unix) is rejected the same way: opening
/// it as a byte source would be an unbounded/meaningless read.
#[cfg(unix)]
#[test]
fn device_node_is_a_typed_rejection() {
    let err = testkit::assert_err(open("file:///dev/null"), "device node");
    assert_eq!(err.context.code, "input.file.not-a-regular-file");
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }), "{err}");
}

/// Non-file schemes are refused before any filesystem access, through
/// the shared plugin-api vocabulary.
#[test]
fn non_file_scheme_is_refused_typed() {
    let access = FileAccess::new();
    for uri in ["http://example.com/a.mkv", "rtsp://cam/1", "dvd:///dev/sr0"] {
        let parsed = SourceUri::parse(uri).must();
        assert!(!access.accepts(&parsed), "{uri}");
        let err = testkit::assert_err(access.open(&parsed), uri);
        assert_eq!(err.context.code, "plugin.access.scheme-not-served");
        assert!(
            matches!(err.kind, ErrorKind::Unsupported { .. }),
            "{uri}: {err}"
        );
    }
}

/// A permission-denied file surfaces the typed Inaccessible/
/// PermissionDenied error (unix; skipped when effectively root).
#[cfg(unix)]
#[test]
fn unreadable_file_is_typed_permission_denied() {
    use std::os::unix::fs::PermissionsExt;
    let data = pattern(64);
    let temp = TempFile::create("denied", &data).must();
    let mut perms = std::fs::metadata(&temp.path).must().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&temp.path, perms).must();
    match open(&temp.file_uri()) {
        Err(err) => {
            assert_eq!(err.context.code, "input.file.permission", "{err}");
            assert_eq!(
                err.kind,
                ErrorKind::Inaccessible {
                    reason: AccessDenied::PermissionDenied
                }
            );
        }
        Ok(_) => {
            // Root (or an equivalent privilege) bypasses mode bits; the
            // assertion is unenforceable in that environment. Say so
            // loudly instead of silently passing.
            assert!(
                running_privileged(),
                "open succeeded on a mode-000 file without detected privilege"
            );
        }
    }
}

/// A file that grows after open keeps reading past its open-time length
/// — no false truncation error (the reference only fires when the live
/// length drops below it).
#[test]
fn growing_file_reads_past_the_open_time_length() {
    let data = pattern(1000);
    let temp = TempFile::create("growing", &data).must();
    let mut stream = open(&temp.file_uri()).must();
    let head = stream.read(600).must();
    assert_eq!(head.len(), 600);

    let appended: Vec<u8> = (1000..1500).map(|i| (i % 251) as u8).collect();
    temp.append(&appended).must();

    let tail = stream.read(1000).must();
    assert_eq!(tail.len(), 900, "600 consumed of 1000 + 500 growth");
    let mut expected_tail = data[600..].to_vec();
    expected_tail.extend_from_slice(&appended);
    assert_eq!(tail, expected_tail, "growth is served, not truncated");
    assert_eq!(stream.size(), Some(1500), "size reports the live truth");
}

/// Empty file: legal, immediately EOF, size zero.
#[test]
fn empty_file_is_a_clean_immediate_eof() {
    let temp = TempFile::create("empty", b"").must();
    let mut stream = open(&temp.file_uri()).must();
    assert_eq!(stream.size(), Some(0));
    assert!(stream.read(16).must().is_empty());
    assert!(stream.read(16).must().is_empty());
    let end = stream.seek(plugin_api::SeekOrigin::Start(0)).must();
    assert_eq!(end.0, 0);
}

/// A single read request above the hard ceiling is refused with the
/// typed ResourceLimit error before any allocation or I/O — the
/// ceiling is part of the stream's public contract, not a unit-test
/// curiosity.
#[test]
fn oversized_read_requests_are_typed_limit_errors() {
    let temp = TempFile::create("ceiling", &pattern(32)).must();
    let mut stream = open(&temp.file_uri()).must();
    let before = stream.position();
    let err = testkit::assert_err(
        stream.read(input::MAX_READ_BYTES + 1),
        "read above the ceiling",
    );
    assert_eq!(err.context.code, "input.read.request-over-limit");
    match err.kind {
        ErrorKind::ResourceLimit {
            limit,
            budget,
            attempted,
        } => {
            assert_eq!(limit, media_types::error::LimitKind::Memory);
            assert_eq!(budget, input::MAX_READ_BYTES as u64);
            assert_eq!(attempted, (input::MAX_READ_BYTES + 1) as u64);
        }
        other => unreachable!("wrong kind: {other:?}"),
    }
    assert_eq!(stream.position(), before, "a refused read never moves");
    // The stream stays usable afterwards.
    let head = stream.read(8).must();
    assert_eq!(head, pattern(32)[..8]);
}

/// The real file passes the borrow-twin contract (card 3.B3):
/// `read_into` through the plugin-api trait object over a real
/// `FileAccess` stream — varied caller buffers, position tracking,
/// clean EOF, and byte-agreement with `read`.
#[test]
fn real_file_passes_the_read_into_contract() {
    let data = pattern(5000);
    let temp = TempFile::create("read-into-contract", &data).must();
    let mut stream = open(&temp.file_uri()).must();
    input::contract::assert_read_into_contract(stream.as_mut(), &data);
}

/// The read-ahead budget join over a real file (card 3.B3, design
/// A4): `with_read_ahead_budget` sizes the window (at or below the
/// ceiling) and the opened stream serves reads through it — the
/// product.toml default (32 MiB) opens cleanly at the 16 MiB ceiling.
#[test]
fn read_ahead_budget_opens_and_serves() {
    let data = pattern(4096);
    let temp = TempFile::create("budget", &data).must();
    let access = input::FileAccess::with_read_ahead_budget(32 * 1024 * 1024).must();
    assert_eq!(access.buffer_capacity(), input::MAX_BUFFER_BYTES);
    let uri = SourceUri::parse(&temp.file_uri()).must();
    let mut stream = access.open(&uri).must();
    let mut out = vec![0u8; 4096];
    let got = stream.read_into(&mut out).must();
    assert_eq!(got, 4096);
    assert_eq!(out, data);
}
