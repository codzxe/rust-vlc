# AUDIT: crates/input/src/uri.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 336 / 4 units (resolve_file, parse_file_path, percent_decode, hex_digit) + 5 in-file tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/demux_host.rs:419` `input::resolve_file(&parsed)` in the open lane → the MP4/AVI smoke opens ran through it; second consumer `crates/input/src/file.rs:190` (FileReader); re-exported at `crates/input/src/lib.rs:76`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of full file; `sed -n` quote verification (3-8, 102-118, 127-135); hand-recomputation of all 14 rejection cases + 6 drive cases + 9 accept cases against the parser; `rg` callers (`apps/cli/src/demux_host.rs:311,419`, `parse.rs:496`, `input/src/file.rs:190`); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:2634-2638` (all 5 `uri::tests` ok, in the input lib's 33 passed) and `mp4-smoke-open-session.txt` (the open lane that consumes resolve_file executed: demux produced 2 tracks). |

## 1. One-paragraph verdict
This is the `file:` URI resolver behind the shipped open lane, and it is the cleanest file audited in this assignment: a small pure parser (scheme/authority/UNC/query-fragment/relative/NUL/UTF-8/percent rules, each with its own typed error code), canonicalization through `std::fs::canonicalize` (symlinks resolved — MP §12), and a test suite that ran green in the no-fail-fast baseline and pins every accepted shape, all 14 rejection codes, the drive-form boundary (`/C:x` stays untouched), and the filesystem round-trip including a dangling symlink. I recomputed every test vector by hand against the parser and found no divergence; the mutation candidates I reasoned through (pass-through `%`, greedy drive strip, dropped UNC check, skipped canonicalize) are each caught by an existing assertion. The single finding is a documented trade-off with an undocumented consequence: the unconditional Windows drive strip can convert an absolute POSIX path (`/C:/x`) into a *relative* one (`C:/x`) on non-Windows platforms, quietly breaking the module's own "required to be absolute" invariant — pathological input, no security impact (canonicalize still resolves and validates), but the doc records the trade-off's rationale without recording this effect.

## 2. What actually works (verified)
— Executed in the shipped lane: `apps/cli/src/demux_host.rs:419` calls `resolve_file` for every `file:` open; baseline `mp4-smoke-open-session.txt` shows the open succeeding to demux (2 tracks, duration, seekable) — the URI → canonical-path → stream chain ran.
— All 5 tests executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:2634-2638` — `uri::tests::{accepted_uri_shapes, non_file_schemes_are_typed_rejections, uri_shape_rejections_are_typed, windows_drive_forms_resolve_as_drives, resolve_file_canonicalizes_and_rejects_missing}`.
— Percent-decoder bounds verified by hand: `%` at the last two byte positions is rejected (`i + 2 >= bytes.len()` is exactly the "no full escape remains" condition, 129); `hex * 16 + lo` over ASCII hex only (145-151); `%2F`→`/`, `%25`→`%`, `%0A`→newline all recomputed and matching the assertions (189-207).
— Authority handling recomputed: empty and case-insensitive `localhost` accepted (72), `user@localhost` and any other host → `input.uri.remote-host` (73); both UNC shapes (`file:////s/share`, `file://localhost//s/share`) land on `input.uri.unc-path` (99-101) — traced through `strip_prefix("//")` + `find(['/','?','#'])` by hand for all test vectors.
— Rejection-before-decode ordering: `?`/`#` rejected on the *encoded* form (82) so `%3F` in a filename survives — documented at 80-81 and exercised implicitly by the accept cases.

## 3. Findings

### F-001 [S4]
Where: crates/input/src/uri.rs:107-117 (invariant claimed at 5)
Code:
```
    let bytes = decoded.as_bytes();
    let decoded = if bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
        && (bytes.len() == 3 || bytes[3] == b'/' || bytes[3] == b'\\')
    {
        decoded[1..].to_string()
    } else {
        decoded
    };
```
(the invariant it can break:)
```
//! percent-decoded (RFC 3986 `%XX`), required to be absolute and UTF-8,
```
What is wrong: the drive strip is unconditional on platform (documented at 17-22 as a recorded trade-off), but the recorded trade-off discusses only that a literal `/C:/…` POSIX path is "pathological" — it does not record that on POSIX the *output* `C:/x` is a **relative** path. The absoluteness check (85, `path_part.starts_with('/')`) runs before the strip, so `file:///C:/x` on Linux yields the relative `PathBuf("C:/x")`, and `std::fs::canonicalize` then resolves it against the process cwd, not the filesystem root.
Why it matters: the module's stated invariant ("required to be absolute") is violated post-strip on non-Windows platforms, and the resolved target becomes cwd-dependent — a Linux user opening `file:///C:/media/x` gets `<cwd>/C:/media/x` if such a tree happens to exist, or a typed `NotFound` otherwise. No symlink escape results (canonicalize still fully resolves whatever it opens — the opened stream is the validated one), and the input is pathological, hence S4.
What correct looks like: after the strip, re-assert absoluteness on the target platform (`std::path::is_absolute()`) and reject with `input.uri.relative-path` when a non-Windows host produces a relative result — or cfg-gate the strip and record the POSIX effect in the trade-off note.
Confidence: HIGH — Verification: read + trace; behavior UNVERIFIED at runtime on a POSIX host (no recorded run opens a drive-form URI outside the pure-parse tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-30 | REAL | — | The scheme/shape/rejection inventory matches the code exactly (recomputed); the absoluteness claim has the F-001 caveat. |
| `resolve_file` | 49-52 | REAL | — | parse → PathBuf → canonicalize with typed io mapping; executed by the CLI open lane. |
| `parse_file_path` | 57-119 | REAL | S4 | Scheme check via normalized scheme (58); split-once not prefix-strip (documented why, 61-63); authority window; encoded-form suffix rejection; absoluteness check; **drive strip can relativize on POSIX (F-001)**. |
| `percent_decode` | 123-142 | REAL | — | Bounds-correct (verified by hand); typed bad-escape errors. |
| `hex_digit` | 145-152 | REAL | — | ASCII hex only; typed error otherwise. |

## 5. Test honesty (5 tests, all executed ok — baseline :2634-2638)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `accepted_uri_shapes` (169-208) | YES | 9 vectors incl. all three authority shapes, scheme case, `%20`/`%2F`/`%25`/`%0A` decodes — a broken decoder or authority rule fails an assert with the URI in the message. |
| `non_file_schemes_are_typed_rejections` (212-236) | YES | 4 schemes; asserts stage=Open, kind=Unsupported, code `plugin.access.scheme-not-served`, and the scheme name inside the error payload (229-233) — a wrong vocabulary or wrong scheme extraction fails. |
| `uri_shape_rejections_are_typed` (239-265) | YES | 14 (uri, code) pairs covering every rejection arm — remote-host (2 shapes), suffix (2), relative (3), percent-decode (3), nul, not-utf8, unc (2). Dropping any arm fails its pair. |
| `windows_drive_forms_resolve_as_drives` (274-287) | YES | 6 cases incl. the exactness boundary `file:///C:x` → `/C:x` (untouched) — a greedy strip (`/X:` without the boundary check) fails this row; `C:%5C…` covers the encoded-backslash spelling. |
| `resolve_file_canonicalizes_and_rejects_missing` (292-335) | YES | Missing path → typed `input.file.missing`/NotFound; a real temp file round-trips through the shared constructor (`SourceUri::from_file_path`, 312 — no hand-rolled `format!`, so platform drift can't hide); dangling symlink → typed NotFound; non-unix fallback stub is honest. |
| Mutation checks (reasoned, not run): `%` passthrough; greedy drive strip; dropped UNC check; skipped canonicalize | ALL CAUGHT | By the vectors cited above. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "required to be absolute and UTF-8, and canonicalized … symlinks resolved — MP §12" | module docs 5-8 | REAL (absoluteness caveat) | Checks at 85, 92, 51; canonicalize at 51; symlink-loop/missing → typed Inaccessible (test 293-302). The drive strip can relativize on POSIX post-check (F-001). |
| "Everything this module cannot serve — non-file schemes, remote authorities, UNC …, relative paths, query/fragment suffixes, malformed escapes, NUL bytes, non-UTF-8 bytes — is a typed rejection" | module docs 24-28 | REAL | All 8 rejection classes present with distinct codes and pinned by `uri_shape_rejections_are_typed` (baseline ok). |
| "Unconditional on platform — the recorded trade-off … the URI grammar is platform-blind" | module docs 17-22 | REAL | The strip at 107-117 is unconditional; the trade-off is recorded; the POSIX relative-path consequence is the unrecorded half (F-001). |
| "`%2F` decodes to a path separator" (deliberate) | test 193-197 | REAL | Decoder is generic; documented in-test. RFC 3989-conformant decode-then-use; no path-traversal hole because the decoded path must still be absolute + canonicalized + existent. |

## 7. Dead / duplicated / unreachable code
None. Both consumers (CLI demux_host, input file reader) are live; no duplication.

## 8. Dependencies & flags
- Deps: `media-types` (SourceUri/EngineError), `plugin-api` (scheme_not_served — shared vocabulary). No feature gates; no unsafe.
- The `lumen-allow-literal` annotations on test vectors (e.g. 189-190, 277-282) are the lint-literal discipline (baseline gate #13 found one violation elsewhere; none here).

## 9. Open questions for the next agent
- `crates/input/src/error.rs` (the typed-code constructors `remote_host`/`suffix_unsupported`/…) is unaudited — the codes asserted here are minted there.
- The canonicalize→open TOCTOU window (resolve_file canonicalizes; the demuxer opens later by path) is inherent to the design — if MP §12 requires opening by the canonical path *atomically*, that belongs to the FileReader consumer (file.rs:190), not this module; whoever audits file.rs should check it opens the resolved path verbatim.
- Windows runtime behavior of the drive strip (does `PathBuf("C:/x")` behave as documented on a real Windows host?) is compile-checked by tests only via the pure parser — no Windows run exists in baseline evidence.
