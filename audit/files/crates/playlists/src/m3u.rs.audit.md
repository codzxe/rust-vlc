# AUDIT: crates/playlists/src/m3u.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 894 (all production; tests live in tests/m3u_suite.rs + tests/hls_attrs.rs + src/fuzz.rs) / 24 production fns + 1 closure |
| File verdict | UNREACHABLE (real, fully implemented parser — no shipped entry point calls it) |
| Reachable from shipped entry point? | NO. Only `crates/streaming` and `fuzz/` depend on `playlists` (glob search of all Cargo.tomls). Within streaming, m3u::parse_detailed is reached only via `playlists::parse_media_playlist` (crates/playlists/src/hls.rs:223) used by `crates/streaming/src/hls/media.rs:637` — and lumen-cli never enters streaming::hls (no reference in apps/cli/src; demux_host.rs:427 opens http(s) via RangedResource + media probe only; `playlist.*` commands are refused at apps/cli/src/dispatch.rs:191-192 "the playlist model is a Phase-4 card"). apps/desktop does not depend on playlists at all. `remote_playlist.rs:178` (the other parse_bytes caller) has zero callers itself. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (callers/STREAM-INF/Cargo.toml globs), sed -n (quotes), baseline citations |

## 1. One-paragraph verdict
This is a genuinely complete, disciplined M3U/HLS-media-subset parser: correct line splitting (BOM/CRLF tolerance, byte-index split is safe because 0x0A is always a char boundary), a strict HLS attribute-list scanner (quoted strings, no duplicate names, junk-after-quote rejection), stateful KEY/MAP stamping with delta-encoded canonical writer, and a hand-verified-correct RFC 3986 §5 join (`remove_dot_segments` checked by hand against `/a/../b`, `/../a`, `a/..`, `a/.`, `a//b` — all correct). Dirty-world posture is real: malformed EXTINF/BAD EXT-X tags degrade to typed warnings with entries surviving; non-UTF-8 is a typed refusal; 64 KiB line cap is enforced. All 18 m3u_suite + 20 hls_attrs + 46 lib tests pass in the baseline. The killer fact is reachability: nothing shipped parses a playlist with it — the crate is a substrate whose consumer ("Phase-4 card" playlist model) has not landed, and the only in-tree production consumer (streaming's HLS media path) is itself unreachable from lumen-cli. Hostile-focus answer on directives: `#EXTM3U` is optional/ignored-anywhere (correct per the plain-M3U tolerance the doc pins), `#EXTINF` parses duration+title with first-comma split and `-1` preserved, and `#EXT-X-STREAM-INF` is NOT modeled here — it survives only as a preserved unknown-tag directive on the next entry; the actual master-playlist (playlist-of-playlists) model lives in `crates/playlists/src/hls.rs:447-528` `parse_master_playlist`, which re-scans the raw text with its own line loop instead of reusing this parser's directive output.

## 2. What actually works (verified)
— verified by orchestrator baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE playlists): `tests/m3u_suite.rs ... 18 passed; 0 failed`, `tests/hls_attrs.rs ... 20 passed; 0 failed`, lib unittests `46 passed; 0 failed`.
— verified by code read: `remove_dot_segments` (L851-894) hand-traced on 5 inputs, all RFC 3986 §5.2.4-correct; `parse_attribute_list` (L467-496) rejects duplicate names, trailing commas, unterminated quotes; `parse_ext_x_key` (L569-610) enforces METHOD=NONE-without-URI and URI-required-otherwise.
— roundtrip law is honestly scoped: the doc's `parse(write(parse(x))) == parse(x)` compares `Playlist` (parse discards warnings); the pin (tests/m3u_suite.rs:20-32) asserts `second.playlist == first.playlist` — consistent, not inflated.

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/m3u.rs:1-894 (whole file) + apps/cli/src/dispatch.rs:191-192
Code:
```
    if id.starts_with("playlist.") {
        "the playlist model is a Phase-4 card"
```
What is wrong: the parser is production-quality but dead in every shipped lane. No Cargo.toml in the tree depends on `playlists` except `crates/streaming/Cargo.toml:42` and `fuzz/Cargo.toml:51`; lumen-cli refuses all `playlist.*` commands; desktop never links it; streaming's only m3u entry (hls/media.rs:637) is not invoked by the CLI open path; remote_playlist.rs (the other consumer) has no callers. lib.rs:5 admits it is "the substrate for the upcoming playlist panel" — honest, but per the release-lane rule this is UNREACHABLE code shipped in the library graph. Why it matters: ~900 loc of parser plus its 3 sibling parsers (pls/xspf/cue) and the probe module ride along compiled-but-uncalled; any feature-matrix row claiming playlist loading on a shipped platform is false until the Phase-4 card lands. What correct looks like: either land the consumer or gate the claim. Confidence: HIGH — Verification: read + rg over every Cargo.toml and apps/ tree.

F-002 [S4]
Where: crates/playlists/src/m3u.rs:97-100 vs 157-162
Code:
```
/// Maximum accepted line length in bytes, terminator excluded (D-1). A
```
vs
```
        if raw.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong {
                line: line_no,
                length: raw.len(),
```
What is wrong: the doc says "terminator excluded", but `raw` is measured before `strip_suffix('\r')` (L167) — the CR half of a CRLF terminator counts toward the 64 KiB cap, so a 65536-byte content line is accepted with LF and rejected with CRLF. One-byte doc/code drift, no realistic playlist hits it. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playlists/src/m3u.rs:178-182
Code:
```
            if let Some(after_tag) = after_hash.strip_prefix("EXTINF") {
                let Some(body) = after_tag.strip_prefix(':') else {
                    warnings.push(ParseWarning::ExtinfMissingColon { line: line_no });
                    continue;
                };
```
What is wrong: prefix-matching (not exact-match) means any tag beginning `EXTINF` (a hypothetical `#EXTINF-NEW:1,t`) is consumed as a damaged EXTINF line with a warning instead of being preserved as an unknown directive like every other unknown `#EXT` tag. No tag in the M3U/HLS/VLC ecosystem currently starts with `EXTINF` besides `#EXTINF` itself, so the blast radius is empty today. Confidence: HIGH mechanics / LOW impact — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse | 114-116 | REAL | — | thin wrapper |
| parse_detailed | 119-309 | REAL | — | state machine verified: KEY/MAP stateful, BYTERANGE/DISCONTINUITY per-segment, pending-flush correct, dangling warning at EOF |
| parse_bytes | 313-316 | REAL | — | strict UTF-8, typed refusal |
| write | 325-388 | REAL | — | canonical order, delta KEY/MAP, `#EXT-X-KEY:METHOD=NONE` on clear |
| write_ext_x_key | 392-419 | REAL | — | canonical attr order, hex IV |
| write_byte_range | 422-428 | REAL | — | |
| AttrValue::as_quoted/as_hex/as_enumerated | 447-458, 632-638 | REAL | — | type-narrowing accessors |
| parse_attribute_list | 467-496 | REAL | — | dup-name rejection, trailing-comma rejection |
| scan_attr_value | 501-526 | REAL | — | quoted/no-escape per HLS spec, int/hex/enum typing |
| decode_hex | 529-541 | REAL | — | even-length, non-ASCII safe (to_digit) |
| parse_decimal | 544-549 | REAL | — | strict u64 |
| parse_byte_range | 552-564 | REAL | — | n[@o] verbatim |
| parse_ext_x_key | 569-610 | REAL | — | METHOD=NONE clearing, URI required |
| parse_ext_x_map | 614-629 | REAL | — | URI required, quoted BYTERANGE |
| parse_duration | 643-652 | REAL | — | finite-only, `-1` preserved |
| resolve_uri | 677-749 | REAL | — | RFC 3986 §5 join; windows/UNC passthrough; `//` scheme-borrow |
| up_to_hash / after_hash | 752-762 | REAL | — | |
| uri_scheme | 769-786 | REAL | — | RFC scheme grammar |
| is_windows_path | 789-795 | REAL | — | drive + UNC |
| split_base | 807-843 | REAL | — | prefix/path/authority split |
| remove_dot_segments | 851-894 | REAL | — | hand-traced correct, incl. `..`-above-root no-op |

## 5. Test honesty
No tests in this file. External suites: m3u_suite.rs (18) — golden fixtures, dirty corpus, roundtrip pins, non-finite durations; hls_attrs.rs (20) — attribute-list edges; src/fuzz.rs — fuzz-lite "never panics" harness. m3u_suite's `assert_roundtrip` (L20-32) would fail if write() emitted non-re-parsing output or if directive/KEY delta-encoding lost state — real assertions, all baseline-passing. Would fail if broken: YES for the parser core. Not covered: CRLF line at exactly MAX_LINE_BYTES (F-002), `#EXTINF`-prefixed unknown tags (F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Roundtrip law: parse(write(parse(x))) == parse(x) for every input x that parses (pinned by tests, including the dirty corpus, ffmpeg-generated HLS fixtures, and fuzz-lite)" | crates/playlists/src/lib.rs:27-29 | REAL | tests/m3u_suite.rs:20-32 asserts playlist equality incl. DIRTY corpus (L195); baseline 18/18 |
| "#EXTM3U — header; optional and ignored wherever it appears" | m3u.rs:8-10 | REAL | L173-177 exact-match + continue |
| "Unknown #EXT-X-* tags (VERSION, TARGETDURATION, MEDIA-SEQUENCE, PLAYLIST-TYPE, …) stay ordered directives — preserved verbatim" | m3u.rs:44-46 | REAL | L247-260; #EXT-X-STREAM-INF included (see hostile focus, §1) |
| "The parser never panics on any byte input (pinned by fuzz-lite)" | m3u.rs:69 | REAL-UNPROVEN | fuzz.rs exists and baseline runs it, but I did not execute the fuzz harness myself (cargo-build-budget); no panic found by read |
| streaming Cargo.toml:37-38 "the HLS session types the playlist through the 4.D1/5.O1a M3U parser's HLS model instead of forking a second M3U parser (the wave plan's REUSE rule)" | crates/streaming/Cargo.toml:37-39 | HALF-FALSE | media playlists reuse m3u::parse_detailed (hls.rs:223), but master playlists are parsed by a second hand-rolled line loop in hls.rs:447-528 (own `split_attr_list`, own tag dispatch) — a fork for the playlist-of-playlists case |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped binaries (F-001) — not dead code in-tree (streaming+hls tests exercise it). Duplication note: `hls.rs::parse_master_playlist` re-implements line scanning/tag dispatch that this file already models as directives (see §6 last row).

## 8. Dependencies & flags
Crate is std + thiserror only (playlists/Cargo.toml — see its own audit). No feature flags. No unsafe.

## 9. Open questions for the next agent
- Does any feature-matrix row claim shipped playlist loading (m3u/pls/xspf) on a platform? If yes it is FALSE until the Phase-4 consumer lands (F-001).
- hls.rs `parse_master_playlist` line loop: does it enforce a per-line byte cap like m3u.rs's MAX_LINE_BYTES, or only the whole-body cap? (Its error doc mentions "Parse (line-cap violation)" — verify while auditing hls.rs.)
- Who consumes `probe` (crates/playlists/src/probe.rs) — same reachability question as this file.
