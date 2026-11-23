# AUDIT: crates/input/src/uri.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 336 / 4 production fns + 5 tests |
| File verdict | REAL (reachable from the shipped CLI file lane, executed) |
| Reachable from shipped entry point? | YES — `lumen open file:///…` → apps/cli/src/parse.rs SourceSpec::Url → apps/cli/src/demux_host.rs:419-421 `input::resolve_file(&parsed)` → uri.rs:49; also crates/input/src/file.rs:190 (FileAccess) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p input --lib uri::` -> 5 passed / 0 failed (exit 0); sed quote verification; cross-check vs streaming/src/schemes.rs |

## 1. One-paragraph verdict
This is the input-side `file:` URI resolver and it is the real, shipped one: strict RFC 8089 grammar where every deviation is a typed refusal with its own dotted code — 14 rejection rows test-pinned including remote hosts, userinfo-bearing authorities (`file://user@localhost/a` → `input.uri.remote-host`), query/fragment suffixes, relative paths, bad/truncated percent-escapes, NUL, non-UTF-8, and both UNC forms; percent-decoding is strict `%HH` with correct boundary checks, decode runs before the NUL/UTF-8/UNC/drive interpretations, and `resolve_file` canonicalizes through the real filesystem (missing file and dangling-symlink rows run real I/O). All 5 tests pass under my run. The one genuine defect is drift: streaming's schemes.rs claims to mirror this resolver "exactly" but accepts query/fragment and NUL into the path where this file refuses them — two grammars for the same string, one shipped (this one), one not.

## 2. What actually works (verified)
— verified by `cargo test -p input --lib uri::` -> `test result: ok. 5 passed; 0 failed` (accepted shapes incl. `%2F`→separator, `%25`, decoded newline; 4 non-file schemes → `plugin.access.scheme-not-served`; 14 typed rejections with exact codes; 6 drive-form rows; real canonicalize roundtrip + missing + dangling-symlink).
— verified by read of the call chain: demux_host.rs:419-421 `let path = input::resolve_file(&parsed)` in the shipped `file` scheme arm.

## 3. Findings
F-001 [S3]
Where: crates/input/src/uri.rs:82-91 vs crates/streaming/src/schemes.rs:418-434 (cross-file)
Code (this file):
```
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
```
What is wrong: the shipped resolver rejects query/fragment suffixes (`input.uri.suffix-unsupported`) and decoded NUL (`input.uri.nul-in-path`); streaming's `classify_file_uri` — whose doc claims the rules "mirror input's resolver exactly … so Phase-6 dispatch and `input` cannot drift" — has neither check and passes `file:///a.wav?x=1` and `file:///a%00.wav` through as `LocalFile` paths. The two named mirror rules (UNC, drive-form) do match byte-for-byte (uri.rs:107-117 ≡ schemes.rs:442-454), but the rest of the grammar diverges. Today users only see this file's behavior (the CLI uses input's resolver; streaming's classify is uncalled), so impact is latent — but any future registry-based composition gets different answers for the same URI. What correct looks like: one shared parser, or schemes.rs delegating to input's grammar.
Confidence: HIGH — Verification: read (both files compared rule-by-rule; schemes.rs audit F-007 records the other side).
F-002 [S4]
Where: crates/input/src/uri.rs:102-117 and crates/streaming/src/schemes.rs:429-434
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
What is wrong: the UNC rule, drive rule, percent-decoder, and authority fold are each hand-duplicated across two crates (input/uri.rs and streaming/schemes.rs) rather than shared — the duplication is exactly what made F-001's drift possible. The two copies agree today only because tests in both files pin the same rows.
Confidence: HIGH — Verification: read (side-by-side).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| resolve_file | 49-52 | REAL | — | parse + std::fs::canonicalize; typed `open_io` fold; reachable from CLI |
| parse_file_path | 57-119 | REAL | S3 | Authority scan terminates at `/`,`?`,`#`; userinfo/IPv6-bracket authorities → remote-host; suffix check on encoded form (correct RFC order); NUL/UTF-8/UNC/drive on decoded form; F-001 drift |
| percent_decode | 123-142 | REAL | — | Boundary check `i + 2 >= len` correct for `%` at end and 1-digit tails; alloc bounded by input length |
| hex_digit | 145-152 | REAL | — | ASCII-only, `hi*16+lo` ≤ 255, no overflow |
| tests (5) | 154-336 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accepted_uri_shapes | YES | 9 rows incl. %2F, %25, decoded newline, case folding |
| non_file_schemes_are_typed_rejections | YES | 4 schemes, exact kind + code + scheme name in FormatInfo |
| uri_shape_rejections_are_typed | YES | 14 rows, exact code each |
| windows_drive_forms_resolve_as_drives | YES | 6 rows incl. the exact-boundary negative `/C:x` |
| resolve_file_canonicalizes_and_rejects_missing | YES | Real filesystem: missing → `input.file.missing`/NotFound; real file roundtrip via the shared `from_file_path` constructor (cannot pass while hosts differ); dangling symlink → NotFound |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The path component is percent-decoded, required to be absolute and UTF-8, and canonicalized" | uri.rs:5-8 | REAL | parse_file_path + resolve_file; tests |
| "symlinks resolved — MP §12 safe path handling" | uri.rs:7-8 | REAL | canonicalize at :51; dangling-symlink test |
| "query/fragment suffixes, malformed escapes, NUL bytes, non-UTF-8 bytes — a typed rejection, never a best-effort guess" | uri.rs:24-28 | REAL | 14-row rejection test |
| "Non-`file` schemes reuse plugin_api::access::scheme_not_served" | uri.rs:28-30 | REAL | :59 + non-file test |
| (cross) schemes.rs "Two rules mirror input's resolver exactly … cannot drift" | schemes.rs:34-40 | FALSE | F-001: suffix/NUL rules differ between the "mirrors" |

## 7. Dead / duplicated / unreachable code
- No dead code in this file. Duplication with streaming/schemes.rs documented in F-002.
- Note: `SourceUri` itself (raw()/scheme()/parse) is media-types' type — wave 2's audit territory; this file re-derives the path from `raw()` with `split_once(':')` relying on SourceUri's scheme-normalization guarantee (documented at :61-63).

## 8. Dependencies & flags
- No feature gates. Deps: media-types (SourceUri, EngineError), plugin-api (scheme_not_served), crate::error. std-only.

## 9. Open questions for the next agent
- Hostile-focus notes for the record: percent-encoding = strict `%HH`, bad/truncated escapes typed (`input.uri.percent-decode`), decode-before-interpret; userinfo = rejected via the authority rule (`file://user@localhost/a` → `input.uri.remote-host`); IPv6 literals in a `file:` authority (`file://[::1]/a`) → `input.uri.remote-host` (the bracket literal is treated as a host string, never parsed as an address — no socket is ever opened here). IPv6 handling inside `http(s)` URLs is the fetch port's refusal (fetch.rs:253-257), not this file.
- If the CLI ever adopts SchemeRegistry::classify (schemes.rs F-001), the F-001 drift becomes user-visible — resolve before that wiring.
