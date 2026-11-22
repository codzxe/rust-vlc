# AUDIT: crates/input/src/uri.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 337 / 5 production items + 5 `#[test]` fns (multi-case) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | **YES** — `lumen open file:///…` → `parse::open_command` classifies URL-shaped sources (apps/cli/src/parse.rs:496-510) → `SourceSpec::Url` → `open_source` (apps/cli/src/demux_host.rs:419) → `input::resolve_file(&parsed)` → `FileAccess::open`. Also reached engine-side via `crate::uri::resolve_file` from `file.rs:190`. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | rg for `resolve_file` consumers (demux_host.rs:419, file.rs:190, lib.rs:76); read of the CLI URL-vs-path classifier (parse.rs:486-510); sed quote verification; manual percent-decode edge arithmetic |

## 1. One-paragraph verdict

The hostile question — "SourceUri parsing honesty" — answers: this is honest parsing. `resolve_file` implements RFC 8089 local-file URIs with typed refusals for everything it cannot serve: non-`file` schemes (shared plugin-api vocabulary), remote authorities (only empty/`localhost` accepted, case-insensitive), query/fragment suffixes, relative paths, malformed `%XX` escapes, NUL bytes, non-UTF-8 bytes, and UNC `//`-prefixed network paths. Percent-decoding is a correct byte-level loop (edge arithmetic hand-checked: `%`, `%4`, `%41` at end-of-string all classified correctly), and the decoded path is canonicalized through `std::fs::canonicalize` so what opens is what was validated (symlink resolution; dangling symlinks surface typed `NotFound`). The two defects are small: the unconditional Windows drive-strip (`/C:/…` → `C:/…`) silently violates the module's own "required to be absolute" invariant on POSIX (a documented trade-off, but the invariant text and the rule contradict each other), and one of the module's accepted URI shapes (`file:/abs/path`, single slash) is unreachable from the shipped CLI because the CLI's URL classifier requires `://`.

## 2. What actually works (verified)

- The full rejection table is test-pinned: 14 shape-rejection cases with distinct context codes (`input.uri.remote-host`, `suffix-unsupported`, `relative-path`, `percent-decode`, `nul-in-path`, `not-utf8-path`, `unc-path`) at 241-265.
- `resolve_file_canonicalizes_and_rejects_missing` (293-335) exercises the real filesystem: missing path → typed `Inaccessible/NotFound`, real file → canonical round-trip through the shared `SourceUri::from_file_path` constructor (so the test cannot pass while hosts differ — the comment at 306-308 explains why), dangling symlink → typed NotFound.
- Drive-form tests pin the exact rule (`/C:x` stays `/C:x` — the boundary condition matters).

## 3. Findings

### F-001 [S4] — The unconditional drive-strip contradicts the module's own absolute-path invariant on POSIX

Where: crates/input/src/uri.rs:3-6 (the invariant) vs 102-117 (the strip).

Code (uri.rs:3-6, verbatim):
```rust
//! RFC 8089-shaped local file URIs: `file:///abs/path`,
//! `file://localhost/abs/path`, `file:/abs/path`. The path component is
//! percent-decoded (RFC 3986 `%XX`), required to be absolute and UTF-8,
//! and canonicalized through `std::fs::canonicalize` so the stream that
```

Code (uri.rs:107-114, verbatim):
```rust
    let bytes = decoded.as_bytes();
    let decoded = if bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
        && (bytes.len() == 3 || bytes[3] == b'/' || bytes[3] == b'\\')
    {
        decoded[1..].to_string()
```

What is wrong: the absolute check (`path_part.starts_with('/')`, 85-87) runs *before* the drive strip. On POSIX, `file:///C:/x` therefore produces the PathBuf `C:/x` — a *relative* path ("C:" directory in the CWD) — violating "required to be absolute". The module docs (17-22) openly record the trade-off ("a literal `/C:/…` filesystem path is pathological"), and `canonicalize` still resolves to a real file or fails typed, so there is no symlink-escape (canonicalization is the opened truth). But the two statements — "required to be absolute" and the unconditional strip — cannot both hold on POSIX. Hygiene: the invariant text should say "absolute after drive-form normalization".

Confidence: HIGH — Verification: read + path-semantics reasoning (no execution on POSIX or Windows — UNVERIFIED by run).

### F-002 [S4] — The `file:/abs/path` single-slash form is accepted here but unreachable from the shipped CLI

Where: crates/input/src/uri.rs:172 (test: `path_of("file:/a/b/c.mkv")` → `/a/b/c.mkv`) vs apps/cli/src/parse.rs:498-506.

Code (apps/cli/src/parse.rs:498-503, verbatim):
```rust
    let is_url = source.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    });
```

What is wrong: the CLI classifies a source as a URL only when it contains `://`. `lumen open 'file:/a/b/c.mkv'` (a form this module accepts and tests) is classified as a *plain path*, so it becomes `SourceSpec::File { path: "file:/a/b/c.mkv" }` — the literal string is treated as a filesystem path (`file_uri(path)` at demux_host.rs:398), producing a `file:` URI of a file literally named `file:/a/b/c.mkv`. The form is accepted in-crate but no shipped entry point can reach that acceptance; a user typing the RFC-legal single-slash spelling gets a confusing not-found for a different file. The parse.rs comment (504-510) justifies the split to avoid duplicating the resolver — reasonable — but the divergence between "shapes the parser accepts" and "shapes the CLI routes" is unrecorded anywhere user-visible.

Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-30 | REAL-UNPROVEN | S4 | F-001 invariant wording |
| `resolve_file` | 49-52 | REAL-UNPROVEN | — | parse + canonicalize; typed `open_io` mapping |
| `parse_file_path` | 57-119 | REAL-UNPROVEN | S4 | scheme gate, authority (empty/localhost, case-insensitive), suffix rejection on encoded form, absolute check, decode, NUL/UTF-8/UNC checks, drive strip (F-001) |
| `percent_decode` | 123-142 | REAL-UNPROVEN | — | byte-exact loop; edge arithmetic correct (hand-checked) |
| `hex_digit` | 145-152 | REAL-UNPROVEN | — | ASCII-only, typed error |
| tests (5) | 169-335 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (5 `#[test]`, multi-case)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `accepted_uri_shapes` (170-208) | YES | 8 shapes incl. %20, %2F (decodes to separator — deliberate), %25, %0A (newline legal, only NUL refused) |
| `non_file_schemes_are_typed_rejections` (213-236) | YES | 4 schemes; kind + code + format name asserted |
| `uri_shape_rejections_are_typed` (240-265) | YES | 14 cases, each with its own context code |
| `windows_drive_forms_resolve_as_drives` (275-287) | YES | 6 cases incl. the exact-boundary negative (`/C:x` untouched) |
| `resolve_file_canonicalizes_and_rejects_missing` (293-335) | YES | real FS: missing, canonical round-trip via the shared constructor, dangling symlink |

No test runs on Windows (the drive form's home platform) — the tests assert the *string* normalization only; actual `PathBuf`/canonicalize behavior on Windows is UNVERIFIED (no Windows host per adaptations).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "required to be absolute and UTF-8" | uri.rs:5 | FALSE on POSIX for drive forms | F-001 |
| "no symlink escapes" (MP §12 / doc 01 §2.4) | uri.rs:7-8 | REAL-UNPROVEN | canonicalize-before-open is the mechanism; dangling-symlink test green in W00 run (UNVERIFIED by me) |
| "Everything this module cannot serve … is a typed rejection, never a best-effort guess" | uri.rs:24-28 | REAL-UNPROVEN | the rejection table + tests |
| `open x.mp4` and `open file:///…/x.mp4` are one behavior | apps/cli/src/parse.rs:504-506 | REAL (for `///` form) / divergence for `/` form | F-002 |

## 7. Dead / duplicated / unreachable code

The `file:/abs` single-slash acceptance branch (77: `None => rest`) is unreachable from the shipped CLI (F-002) — reachable only by direct library callers/tests.

## 8. Dependencies & flags

Depends on `media_types` (SourceUri, EngineError), `plugin_api::access::scheme_not_served`, `crate::error`. No feature gates.

## 9. Open questions for the next agent

- `SourceUri::parse` / `from_file_path` (crates/media-types) — the constructor's own validation is the other half of the round-trip (wave 2 territory); uri.rs trusts its guarantees (`split_once(':')` comment at 61-63).
- `file_uri_from_canonical` (apps/cli demux_host) — the shared-canonical tail; W13.
- Windows behavior of the drive form end-to-end — needs a Windows host; flag to the platform lanes.
