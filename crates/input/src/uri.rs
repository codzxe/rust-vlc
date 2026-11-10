//! `file:` URI resolution (card 2.A1): SourceUri → canonical local path.
//!
//! RFC 8089-shaped local file URIs: `file:///abs/path`,
//! `file://localhost/abs/path`, `file:/abs/path`. The path component is
//! percent-decoded (RFC 3986 `%XX`), required to be absolute and UTF-8,
//! and canonicalized through `std::fs::canonicalize` so the stream that
//! is opened is the stream that was validated (symlinks resolved — MP
//! §12 safe path handling, doc 01 §2.4 "no symlink escapes").
//!
//! Everything this module cannot serve — non-`file` schemes, remote
//! authorities, relative paths, query/fragment suffixes, malformed
//! escapes, NUL bytes, non-UTF-8 bytes — is a typed rejection, never a
//! best-effort guess (MP §8: no silent failure). Non-`file` schemes
//! reuse [`plugin_api::access::scheme_not_served`]: one vocabulary for
//! "this access implementation does not serve that scheme".

use std::path::PathBuf;

use media_types::EngineError;
use media_types::ids::SourceUri;
use plugin_api::access::scheme_not_served;

use crate::error;

/// Resolve a `file:` URI to the canonical absolute path of the local
/// file it names. The path must exist (canonicalization requires it);
/// a missing or unresolvable path is a typed `Inaccessible` error.
///
/// # Errors
/// [`EngineError`] (stage `Open`) with kind `Unsupported` for
/// non-`file` schemes and every URI-shape rejection (see the module
/// docs), or kind `Inaccessible` when canonicalization fails (missing
/// file, permission, symlink loop).
pub fn resolve_file(uri: &SourceUri) -> Result<PathBuf, EngineError> {
    let path = PathBuf::from(parse_file_path(uri)?);
    std::fs::canonicalize(&path).map_err(error::open_io)
}

/// Parse the URI's path component into a decoded path string (pure: no
/// filesystem access). Split out so the parsing rules are unit-testable
/// without fixtures.
fn parse_file_path(uri: &SourceUri) -> Result<String, EngineError> {
    if uri.scheme().as_str() != "file" {
        return Err(scheme_not_served(uri));
    }
    // `SourceUri::parse` guarantees the first `:` ends the scheme, and
    // the scheme is normalized lowercase while `raw` keeps its original
    // spelling — so split, do not prefix-strip.
    let raw = uri.raw();
    let (_, rest) = raw.split_once(':').unwrap_or((raw, ""));
    let path_part = match rest.strip_prefix("//") {
        Some(after_authority) => {
            let end = after_authority
                .find(['/', '?', '#'])
                .unwrap_or(after_authority.len());
            let authority = &after_authority[..end];
            if !(authority.is_empty() || authority.eq_ignore_ascii_case("localhost")) {
                return Err(error::remote_host());
            }
            &after_authority[end..]
        }
        None => rest,
    };
    // No query/fragment semantics exist for local files: reject rather
    // than silently drop (MP §8). The check runs on the *encoded* form —
    // a literal `?`/`#` in a filename must arrive percent-encoded.
    if path_part.find(['?', '#']).is_some() {
        return Err(error::suffix_unsupported());
    }
    if !path_part.starts_with('/') {
        return Err(error::relative_path());
    }
    let decoded = percent_decode(path_part)?;
    if decoded.contains(&0) {
        return Err(error::nul_in_path());
    }
    String::from_utf8(decoded).map_err(|_| error::not_utf8_path())
}

/// RFC 3986 percent-decoding over the path bytes: `%HH` with ASCII hex
/// digits becomes one byte; everything else passes through.
fn percent_decode(text: &str) -> Result<Vec<u8>, EngineError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(error::bad_percent_escape());
            }
            let hi = hex_digit(bytes[i + 1])?;
            let lo = hex_digit(bytes[i + 2])?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// One ASCII hex digit to its value.
fn hex_digit(byte: u8) -> Result<u8, EngineError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(error::bad_percent_escape()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ErrorKind;
    use testkit::Must;

    fn parse(uri: &str) -> SourceUri {
        SourceUri::parse(uri).must()
    }

    fn path_of(uri: &str) -> Result<String, EngineError> {
        parse_file_path(&parse(uri))
    }

    /// The accepted RFC 8089 shapes, including percent-decoding.
    #[test]
    fn accepted_uri_shapes() {
        assert_eq!(path_of("file:///a/b/c.mkv").must(), "/a/b/c.mkv");
        assert_eq!(path_of("file:/a/b/c.mkv").must(), "/a/b/c.mkv");
        assert_eq!(
            path_of("file://localhost/a/b/c.mkv").must(),
            "/a/b/c.mkv",
            "localhost authority is local"
        );
        assert_eq!(
            path_of("file://LOCALHOST/a/b").must(),
            "/a/b",
            "authority comparison is case-insensitive"
        );
        assert_eq!(
            path_of("FILE:///a/b").must(),
            "/a/b",
            "scheme case is normalized by SourceUri"
        );
        assert_eq!(
            path_of("file:///tmp/a%20b.mkv").must(), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "/tmp/a b.mkv", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "percent-encoded space decodes"
        );
        assert_eq!(
            path_of("file:///tmp/a%2Fb").must(), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "/tmp/a/b", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "%2F decodes to a path separator"
        );
        assert_eq!(
            path_of("file:///tmp/100%25.mkv").must(), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "/tmp/100%.mkv", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "%25 decodes to the escape character itself"
        );
        assert_eq!(
            path_of("file:///tmp/a%0Ab").must(), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "/tmp/a\nb", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "a decoded newline is a legal (if odd) filename byte — only NUL is rejected"
        );
    }

    /// Non-`file` schemes are typed rejections through the shared
    /// plugin-api vocabulary.
    #[test]
    fn non_file_schemes_are_typed_rejections() {
        for uri in [
            "http://example.com/a", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "https://x/y", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "rtsp://cam/1",
            "dvd://a",
        ] {
            let err = testkit::assert_err(path_of(uri), uri);
            assert_eq!(err.stage, media_types::PipelineStage::Open);
            assert!(
                matches!(err.kind, ErrorKind::Unsupported { .. }),
                "{uri}: {}",
                err
            );
            assert_eq!(err.context.code, "plugin.access.scheme-not-served");
            match err.kind {
                ErrorKind::Unsupported { format } => {
                    let info = format.must();
                    assert_eq!(info.name, Some(uri.split(':').next().must().to_string()));
                }
                _ => unreachable!("checked above"),
            }
        }
    }

    /// Every URI-shape rejection is typed with its own code.
    #[test]
    fn uri_shape_rejections_are_typed() {
        let cases = [
            ("file://nas/share/a.mkv", "input.uri.remote-host"),
            ("file://user@localhost/a", "input.uri.remote-host"),
            ("file:///a/b?token=x", "input.uri.suffix-unsupported"),
            ("file:///a/b#frag", "input.uri.suffix-unsupported"),
            ("file:relative/x.mkv", "input.uri.relative-path"),
            ("file:", "input.uri.relative-path"),
            ("file://", "input.uri.relative-path"),
            ("file:///a%G1b", "input.uri.percent-decode"),
            ("file:///a%", "input.uri.percent-decode"),
            ("file:///a%1", "input.uri.percent-decode"),
            ("file:///a%00b", "input.uri.nul-in-path"),
            ("file:///a%FFb", "input.uri.not-utf8-path"),
        ];
        for (uri, code) in cases {
            let err = testkit::assert_err(path_of(uri), uri);
            assert_eq!(err.context.code, code, "{uri}");
            assert!(
                matches!(err.kind, ErrorKind::Unsupported { .. }),
                "{uri}: {err}"
            );
        }
    }

    /// `resolve_file` is `parse_file_path` + canonicalize: a missing
    /// path surfaces the typed `Inaccessible`/`NotFound` error, and a
    /// real path surfaces its canonical form (symlinks resolved).
    #[test]
    fn resolve_file_canonicalizes_and_rejects_missing() {
        let missing = resolve_file(&parse("file:///nonexistent-lumen-2a1/nope.mkv")); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        let err = testkit::assert_err(missing, "missing path");
        assert_eq!(err.context.code, "input.file.missing");
        assert_eq!(
            err.kind,
            ErrorKind::Inaccessible {
                reason: media_types::error::AccessDenied::NotFound
            }
        );

        // A real file canonicalizes to itself (the temp dir is already
        // absolute; canonicalize also resolves any symlinked parents).
        let tmp = std::env::temp_dir().canonicalize().must();
        let file = tmp.join("lumen-2a1-uri-resolve.tmp"); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        std::fs::write(&file, b"x").must();
        let uri = format!("file://{}", file.to_str().must());
        let resolved = resolve_file(&parse(&uri)).must();
        assert_eq!(resolved, file, "canonical path round-trips");
        let _ = std::fs::remove_file(&file);

        // A dangling symlink names nothing.
        let dangling = tmp.join("lumen-2a1-dangling.tmp"); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        #[cfg(unix)]
        std::os::unix::fs::symlink("nowhere-lumen-2a1", &dangling).must(); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        #[cfg(not(unix))]
        std::fs::write(&dangling, b"stub-not-a-symlink-on-this-platform").must();
        let uri = format!("file://{}", dangling.to_str().must());
        let err = testkit::assert_err(resolve_file(&parse(&uri)), "dangling symlink");
        assert!(
            matches!(
                err.kind,
                ErrorKind::Inaccessible {
                    reason: media_types::error::AccessDenied::NotFound
                }
            ),
            "dangling symlink: {err}"
        );
        let _ = std::fs::remove_file(&dangling);
    }
}
