# AUDIT: crates/playlists/src/m3u.rs#part2 (lines 448–894)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 447 (of 895) / 16 fns + 2 types |
| File verdict | UNREACHABLE (real, heavily tested helpers — no shipped entry point reaches them) |
| Reachable from shipped entry point? | NO — consumers are m3u.rs part1 / hls.rs (playlists internal), streaming hls+dash lanes (unreachable), `remote_playlist.rs` (orphan), tests, fuzz targets |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "resolve_uri" crates/playlists/tests/hls_attrs.rs` (36 assertions incl. RFC 3986 normal+abnormal suites); `rg -n "AttrValue::\|as_int" crates/playlists/src` (Int payload never read); `rg -n "playlists" apps/cli/Cargo.toml crates/media-probe/Cargo.toml` (no shipped consumer — media-probe probes containers, not playlists); `sed -n` on all quoted ranges; baseline INDEX #3 (playlists suites green). |

## 1. One-paragraph verdict
Part2 is the HLS attribute-list machinery (strict, spec-shaped: duplicate names, trailing commas, junk-after-quote, odd hex, overflow all refuse) plus the crate's RFC 3986 `resolve_uri` join — the join is pinned against both the RFC's normal and abnormal example vectors and behaves correctly everywhere I traced it except one documented-promise violation: a **backslash Windows drive base** (`C:\Music\list.m3u`) joined with a relative reference yields `C:s.mp3` — the directory silently vanishes because the base is parsed as URI-scheme `C:` and the directory search only knows `/`. Two smaller holes: `parse_duration` silently accepts negative EXTINF values (the crate's own XSPF parser refuses them typed), and `#EXT-X-KEY` records an `IV` of any even hex length rather than the spec's 16 bytes. None of it is reachable from the shipped binary.

## 2. What actually works (verified)
— `resolve_uri` correctness: RFC 3986 §5.4 **normal** and **abnormal** example suites pinned by `resolve_uri_rfc3986_normal_examples` (hls_attrs.rs:619) and `…_abnormal_examples` (:653); HLS-shaped and POSIX/local-base cases pinned by `resolve_uri_hls_shaped_cases` (:682) and `resolve_uri_windows_posix_and_local_bases` (:732, incl. `..`-cannot-climb-above-root). Green in baseline `cargo test --workspace` (INDEX #3, 2026-11-21). My own traces of `remove_dot_segments` (`a/..`→`.`, `a//b` preserved, `/a/../../b`→`/b`, `a/b/..`→`a/`) agree with the RFC/POSIX expectations and the tests.
— Attribute strictness traced case-by-case: duplicate name → None (:480-482), trailing comma → None (:486), unterminated quote → None (:503), junk after quote → None (:509), non-numeric/overflow integer → None (:519), odd/non-hex → None (:530-538), `METHOD=NONE`+URI → None (:591-593), non-NONE without URI → None (:602) — all match the doc at :461-466.

## 3. Findings

### F-001 [S3] Backslash Windows-drive BASE joined with a relative reference drops the whole directory
Where: crates/playlists/src/m3u.rs:720-735 (with :789-795 and :769-786)
Code:
```
    } else {
        // Merge onto the base's directory, then remove dot segments.
        let dir = match b.path.rfind('/') {
            Some(i) => &b.path[..=i],
            None => "",
        };
        (remove_dot_segments(&format!("{dir}{r_path}")), r_query)
    };
```
What is wrong: `is_windows_path` guards only the *reference* (:680-682); the *base* `C:\Music\list.m3u` parses through `uri_scheme` (:769-786) as scheme `"C"` with path `\Music\list.m3u` (`split_base` :835-842), and the directory search at :731 looks for `/` only — there is none — so `dir` is `""` and the join yields `C:s.mp3`: a drive-relative path that resolves to the current directory of drive C on Windows, not the playlist's sibling. The module doc promises "A local-path base (no scheme) works too" (:670-671) and "Separators are never rewritten in either direction (`C:\a\b` stays backslashed)" (:667-669); forward-slash Windows bases (`C:/Music/list.m3u`) join correctly, backslash ones are silently wrong.
Why it matters: every consumer of `resolve_uri` with a local Windows playlist base produces wrong URLs/paths; the wrongness is silent (no warning channel exists here). The test suite pins windows paths as *references* (:734-739) and includes `C:\Music\list.m3u` only in the no-panic fuzz-lite base list (hls_attrs.rs:938) — the join result is unpinned.
What correct looks like: in `split_base` (or `resolve_uri`), detect `is_windows_path(base)` and use `\\` (or the base's own last separator of either kind) for the directory split — or document backslash bases as unsupported and refuse them visibly.
Confidence: HIGH — Verification: read (full trace: base `C:\Music\list.m3u` + `s.mp3` → prefix `C:`, path `\Music\list.m3u`, `rfind('/')`→None → `C:s.mp3`); execution UNVERIFIED (no cargo).

### F-002 [S4] `parse_duration` silently accepts negative EXTINF values
Where: crates/playlists/src/m3u.rs:643-652
Code:
```
    match trimmed.parse::<f64>() {
        Ok(d) if d.is_finite() => Some(d),
        _ => None,
    }
```
What is wrong: only emptiness/malformedness/non-finiteness are refused; `-5` parses to `Some(-5.0)` and the part1 caller warns `BadDuration` only on `None` (m3u.rs:188-190), so a negative duration lands silently in `Segment.duration`. The crate's own XSPF parser refuses negative durations typed (`xspf.rs:897-900`: `ms >= 0.0` else `BadDuration`) — the two playlist parsers disagree on policy, and downstream sums (e.g. `live.rs::live_edge_wallclock_ms` summing EXTINF values) compute backwards edges from negative entries.
Why it matters: inconsistent cross-format policy; a nonsensical-but-parseable field bypasses the crate's "honest degradation" warning discipline.
What correct looks like: `Ok(d) if d.is_finite() && d >= 0.0` (matching xspf.rs), or a dedicated warning variant.
Confidence: HIGH — Verification: read (both parsers compared).

### F-003 [S4] `#EXT-X-KEY` IV accepted at any even hex length; KEYFORMATVERSIONS unvalidated
Where: crates/playlists/src/m3u.rs:581 and :583
Code:
```
            "IV" => iv = Some(val.as_hex()?),
            "KEYFORMAT" => keyformat = Some(val.as_quoted()?),
            "KEYFORMATVERSIONS" => keyformat_versions = Some(val.as_quoted()?),
```
What is wrong: RFC 8216 §4.3.2.4 specifies `IV` as a 128-bit hexadecimal-sequence; any even-length hex (e.g. `IV=0x1234`) is recorded verbatim into `HlsKey.iv` with no length check, and `KEYFORMATVERSIONS` content is not validated (spec: positive integers separated by `/`).
Why it matters: the model carries data that violates its own spec shape; blast radius is nil today because every encrypted shape is refused downstream (`streaming.hls.encrypted`), which is exactly why this stays S4.
What correct looks like: `iv` requires exactly 16 decoded bytes else the line is malformed (consistent with the file's strictness everywhere else).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part2 range; boundary: `AttrValue::as_quoted` :446-452 straddles — body at :448-452 is mine)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AttrValue::as_quoted`/`as_hex` | 447-458 | REAL | — | type discriminators |
| `parse_attribute_list` | 467-496 | REAL | — | strict per doc; the `None => return None` arm's "unreachable" comment verified accurate (:489) |
| `scan_attr_value` | 501-526 | REAL | — | quoted/int/hex/enum typing; `Int`'s u64 payload is never read by anyone — the variant works purely as a strictness tag (e.g. `URI=123` unquoted-digits → not-Quoted → malformed ✓); could be a unit variant |
| `decode_hex` | 529-541 | REAL | — | even-length, case-insensitive, non-hex → None |
| `parse_decimal` | 544-549 | REAL | — | strict u64 |
| `parse_byte_range` | 552-564 | REAL | — | `<n>[@<o>]`, empty parts → None ✓ |
| `parse_ext_x_key` | 569-610 | REAL | S4 | F-003; NONE+URI refused, non-NONE requires URI ✓; unknown attrs ignored per spec ✓ |
| `parse_ext_x_map` | 614-629 | REAL | — | URI required, BYTERANGE quoted-parsed ✓ |
| `AttrValue::as_enumerated` | 631-638 | REAL | — | discriminator |
| `parse_duration` | 643-652 | REAL | S4 | F-002 |
| `resolve_uri` | 677-749 | REAL | S3 | F-001; empty-ref, query-only, network-path, root-relative, merge — all RFC-correct and test-pinned |
| `up_to_hash`/`after_hash` | 752-762 | REAL | — | trivial |
| `uri_scheme` | 769-786 | REAL | — | correct ALPHA-first grammar; single-letter schemes returned (windows filtering is the caller's job — done for references, not bases → F-001) |
| `is_windows_path` | 789-795 | REAL | — | `\\` UNC + `X:/`/`X:\` ✓ |
| `split_base` | 807-843 | REAL | — | prefix/path/authority split correct incl. authority-only and scheme-less bases |
| `remove_dot_segments` | 851-894 | REAL | — | stack algorithm verified against RFC 3986 examples + edge cases (`a/..`→`.`, `//` preserved, climb-above-root clamped by design, :888-892 POSIX `.` fallback) |

## 5. Test honesty
Tests live in `crates/playlists/tests/hls_attrs.rs` (36 `resolve_uri` assertions incl. the RFC's own normal+abnormal vectors) and `m3u_suite.rs`. Mutation check: inverting the `..` pop or the trailing-`.` fallback breaks the abnormal-examples test; weakening `scan_attr_value`'s junk-after-quote check breaks the attribute tests. **Unpinned:** F-001's backslash-base join (fuzz-lite only asserts no-panic), F-002's negative durations, F-003's IV length.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A local-path base (no scheme) works too: resolve_uri("d/l.m3u","s.mp3") == "d/s.mp3"" | m3u.rs:670-671 | REAL for POSIX, FALSE for backslash Windows bases | F-001: `C:\Music\list.m3u` + `s.mp3` → `C:s.mp3` |
| "Windows drive paths … are not URI references: returned verbatim, never mangled" / "Separators are never rewritten" | m3u.rs:665-669 | REAL for references; the *base* side mangles the join | F-001 |
| "`..` never climbs above root" | m3u.rs:662 | REAL | :865-869 pop-on-empty no-op; test :763-764 |
| "Returns None on any syntax damage (… non-numeric integer, odd-length hex, overflow)" | m3u.rs:464-466 | REAL | traced case-by-case (§2) |
| "Empty, malformed and non-finite … all return None" (durations) | m3u.rs:640-642 | REAL as stated — negatives are the undocumented gap | F-002 |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points: `apps/cli` depends on `streaming` and `media-probe` but neither reaches playlist parsing (media-probe probes containers only; CLI has no playlist-open path — `dispatch.rs` "the playlist model is a Phase-4 card"). Real consumers: `crates/playlists/tests/*` (5 suites), fuzz targets `fuzz_playlist_m3u`/`fuzz_hls_playlist`, `streaming/src/remote_playlist.rs` (orphan), and the streaming hls/dash lanes (unreachable, per my other audits). `AttrValue::Int`'s payload is never read (see table) — a unit variant would express the same strictness.

## 8. Dependencies & flags
No external deps in this range (pure `str`/`Vec` work); `MAX_LINE_BYTES` shared with the crate. No feature gates. No unsafe. Workspace build clean (baseline #1), playlists suites green (baseline #3).

## 9. Open questions for the next agent
- A03 (m3u part1): the EXTINF caller at :178-192 — confirm no later negative-duration guard exists downstream of `Segment.duration` in your range (F-002 depends on there being none).
- Whoever wires the eventual playlist-open path: F-001 must be fixed before any Windows local-playlist use; the silent `C:s.mp3` output is a file-not-found factory.
- Wave-16 fuzz owner: the fuzz-lite base list already contains `C:\Music\list.m3u` (hls_attrs.rs:938) — extend the assertion from no-panic to join-correctness once F-001 is fixed.
