//! `file:` URI resolution (card 2.A1): SourceUri → canonical local path.
//!
//! RFC 8089-shaped local file URIs: `file:///abs/path`,
//! `file://localhost/abs/path`, `file:/abs/path`. The path component is
//! percent-decoded (RFC 3986 `%XX`), required to be absolute and UTF-8,
//! and canonicalized through `std::fs::canonicalize` so the stream that
//! is opened is the stream that was validated (symlinks resolved, MP §12
//! safe path handling).
//!
//! Everything this module cannot serve — non-`file` schemes, remote
//! authorities, relative paths, query/fragment suffixes, malformed
//! escapes, NUL bytes, non-UTF-8 bytes — is a typed rejection, never a
//! best-effort guess (MP §8: no silent failure).
