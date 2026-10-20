//! Typed error constructors for the input layer (card 2.A1).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) — this
//! module never redefines or wraps it. It mints
//! [`EngineError`](media_types::EngineError) values with input-specific
//! context codes and folds [`std::io::Error`] into the taxonomy with the
//! OS error preserved in the `source` chain. Every error this crate
//! mints is stage [`PipelineStage::Open`](media_types::PipelineStage::Open):
//! sources fail while being opened or read (doc 03 §4.1(d)).
//!
//! # OS-error folding (documented honesty)
//!
//! `AccessDenied` has five reasons; errno has hundreds. `NotFound` and
//! `PermissionDenied` map exactly; `FilesystemLoop` resolves to
//! "nothing exists at the address" (`NotFound`). Every other open/read
//! I/O failure folds into `Inaccessible` with the best available reason
//! while the untouched OS error rides the `source` chain — the chain
//! carries the truth, the kind slot is the closest taxonomy fit.
//!
//! # No secrets
//!
//! Constructors take no path or URI text: `Display` stays structured
//! data and redaction stays `SourceUri::redacted`'s job (MP §12).

use std::io;

use media_types::error::{AccessDenied, CorruptDetail, ErrorContext, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every input error carries (doc 03 §4.1(d)).
const STAGE: PipelineStage = PipelineStage::Open;

/// Mint a stage-`Open` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// Nothing exists at the address (missing file, dangling symlink,
/// unresolvable path).
pub fn missing() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.file.missing",
    )
}

/// The file exists but permissions refuse it.
pub fn permission() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "input.file.permission",
    )
}

/// The path exists but is not a regular file (directory, FIFO, device
/// node). Devices such as `/dev/zero` would be unbounded reads — MP §12
/// bounds memory, doc 01 §2.4 bounds read-ahead.
pub fn not_a_regular_file() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.file.not-a-regular-file",
    )
}

/// The source shrank below its open-time length mid-read (card 2.A1 AC:
/// typed error, never a silent EOF, never a panic).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "input.file.truncated",
    )
}

/// A seek target beyond the current end of the source.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.out-of-range",
    )
}

/// A relative seek computed to a position before the start of the
/// stream.
pub fn seek_before_start() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.before-start",
    )
}

/// A relative seek overflowed the addressable position range.
pub fn seek_overflow() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.overflow",
    )
}

/// `file://<host>/…` with a non-local authority: remote file hosts are
/// not local access (network protocols belong to `streaming`, Phase 6).
pub fn remote_host() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.remote-host",
    )
}

/// `file:<relative>` — RFC 8089 file URI paths are absolute; relative
/// paths are a caller-side concern (the command layer's `SourceSpec`
/// carries `PathBuf`s), not something this crate guesses a base for.
pub fn relative_path() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.relative-path",
    )
}

/// A `?query` or `#fragment` suffix on a `file:` URI: no semantics are
/// defined for local files, so they are rejected rather than ignored
/// (MP §8: no silent failure).
pub fn suffix_unsupported() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.suffix-unsupported",
    )
}

/// A malformed percent-escape (`%G1`, a trailing `%`).
pub fn bad_percent_escape() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.percent-decode",
    )
}

/// A percent-decoded NUL byte: no filesystem path may contain one.
pub fn nul_in_path() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.nul-in-path",
    )
}

/// A percent-decoded path that is not UTF-8 (v0 limitation: paths are
/// UTF-8 so the crate stays free of platform `OsStr` byte APIs, ARCH-6).
pub fn not_utf8_path() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.uri.not-utf8-path",
    )
}

/// A buffer capacity of zero: a value this crate does not support.
pub fn capacity_zero() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "input.buffer.capacity-zero",
    )
}

/// A buffer capacity above the hard ceiling: bounded memory is a
/// contract, not a suggestion (MP §12).
pub fn capacity_over_budget(requested: usize, budget: usize) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: budget as u64,
            attempted: requested as u64,
        },
        "input.buffer.capacity-out-of-range",
    )
}

/// A single read request above the hard ceiling: the caller must split
/// it; the reader never allocates past the ceiling (MP §12).
pub fn read_over_budget(requested: usize, budget: usize) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: budget as u64,
            attempted: requested as u64,
        },
        "input.read.request-over-limit",
    )
}

/// Fold a `canonicalize`/`open` I/O failure into the taxonomy (see the
/// module docs for the folding policy; the OS error is preserved).
/// `FilesystemLoop` is not matched: `io_error_more` is unstable on this
/// toolchain, and a symlink loop is reported by the OS with its own
/// `io::ErrorKind`, folding into the generic arm below.
pub fn open_io(err: io::Error) -> EngineError {
    match err.kind() {
        io::ErrorKind::NotFound => missing().with_source(err),
        io::ErrorKind::PermissionDenied => permission().with_source(err),
        _ => mint(
            ErrorKind::Inaccessible {
                reason: AccessDenied::PermissionDenied,
            },
            "input.file.open-io",
        )
        .with_source(err),
    }
}

/// Fold a mid-read I/O failure (doc 01 §3.4: `Inaccessible` covers
/// sources that cannot be opened *or read*).
pub fn read_io(err: io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "input.file.read-io",
    )
    .with_source(err)
}

/// Fold a seek I/O failure.
pub fn seek_io(err: io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "input.seek.io",
    )
    .with_source(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every constructor mints stage `Open` with the kind and code it
    /// claims — the whole vocabulary, one table.
    #[test]
    fn constructors_mint_stage_open_kinds_and_codes() {
        let cases: [(EngineError, ErrorKind, &str); 14] = [
            (
                missing(),
                inaccessible(AccessDenied::NotFound),
                "input.file.missing",
            ),
            (
                permission(),
                inaccessible(AccessDenied::PermissionDenied),
                "input.file.permission",
            ),
            (
                not_a_regular_file(),
                ErrorKind::Unsupported { format: None },
                "input.file.not-a-regular-file",
            ),
            (
                truncated(100, 40),
                ErrorKind::Corrupt {
                    detail: CorruptDetail::Truncated {
                        expected: 100,
                        found: 40,
                    },
                },
                "input.file.truncated",
            ),
            (
                seek_out_of_range(),
                inaccessible(AccessDenied::NotFound),
                "input.seek.out-of-range",
            ),
            (
                seek_before_start(),
                inaccessible(AccessDenied::NotFound),
                "input.seek.before-start",
            ),
            (
                seek_overflow(),
                inaccessible(AccessDenied::NotFound),
                "input.seek.overflow",
            ),
            (
                remote_host(),
                ErrorKind::Unsupported { format: None },
                "input.uri.remote-host",
            ),
            (
                relative_path(),
                ErrorKind::Unsupported { format: None },
                "input.uri.relative-path",
            ),
            (
                suffix_unsupported(),
                ErrorKind::Unsupported { format: None },
                "input.uri.suffix-unsupported",
            ),
            (
                bad_percent_escape(),
                ErrorKind::Unsupported { format: None },
                "input.uri.percent-decode",
            ),
            (
                nul_in_path(),
                ErrorKind::Unsupported { format: None },
                "input.uri.nul-in-path",
            ),
            (
                not_utf8_path(),
                ErrorKind::Unsupported { format: None },
                "input.uri.not-utf8-path",
            ),
            (
                capacity_zero(),
                ErrorKind::Unsupported { format: None },
                "input.buffer.capacity-zero",
            ),
        ];
        for (err, kind, code) in cases {
            assert_eq!(err.stage, STAGE, "{code}");
            assert_eq!(err.kind, kind, "{code}");
            assert_eq!(err.context.code, code);
        }
    }

    fn inaccessible(reason: AccessDenied) -> ErrorKind {
        ErrorKind::Inaccessible { reason }
    }

    /// The limit constructors carry budget/attempted in the kind (the
    /// card's `limit` class).
    #[test]
    fn limit_constructors_carry_budget_and_attempted() {
        let cap = capacity_over_budget(20_000_000, 16_777_216);
        match cap.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::Memory);
                assert_eq!(budget, 16_777_216);
                assert_eq!(attempted, 20_000_000);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        let read = read_over_budget(100, 40);
        match read.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::Memory);
                assert_eq!(budget, 40);
                assert_eq!(attempted, 100);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    /// Open-time I/O mapping: the exact cases plus the documented fold,
    /// each with the OS error preserved in the source chain.
    #[test]
    fn open_io_maps_kinds_and_keeps_the_chain() {
        let not_found = open_io(io::Error::from(io::ErrorKind::NotFound));
        assert_eq!(not_found.context.code, "input.file.missing");
        assert_eq!(
            not_found.kind,
            inaccessible(AccessDenied::NotFound),
            "NotFound maps exactly"
        );
        assert!(
            std::error::Error::source(&not_found).is_some(),
            "chain kept"
        );

        let denied = open_io(io::Error::from(io::ErrorKind::PermissionDenied));
        assert_eq!(denied.context.code, "input.file.permission");
        assert_eq!(
            denied.kind,
            inaccessible(AccessDenied::PermissionDenied),
            "PermissionDenied maps exactly"
        );

        let other = open_io(io::Error::new(io::ErrorKind::Unsupported, "os error 999"));
        assert_eq!(other.context.code, "input.file.open-io");
        assert_eq!(
            other.kind,
            inaccessible(AccessDenied::PermissionDenied),
            "documented fold for unmapped kinds"
        );
        let chained = std::error::Error::source(&other).map(|e| e.to_string());
        assert_eq!(
            chained.map(|s| s.contains("os error 999")),
            Some(true),
            "the OS truth rides the chain"
        );
    }

    /// Mid-read and seek I/O folds carry their own codes.
    #[test]
    fn read_and_seek_io_have_distinct_codes() {
        let read = read_io(io::Error::new(io::ErrorKind::Unsupported, "eio"));
        assert_eq!(read.context.code, "input.file.read-io");
        let seek = seek_io(io::Error::new(io::ErrorKind::Unsupported, "espipe"));
        assert_eq!(seek.context.code, "input.seek.io");
    }

    /// `Display` is structured and carries no caller data (MP §12).
    #[test]
    fn display_is_structured_only() {
        let e = truncated(1024, 512);
        let text = e.to_string();
        assert_eq!(
            text,
            "[open] truncated data: expected 1024 bytes, found 512 (input.file.truncated)"
        );
    }
}
