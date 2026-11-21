# AUDIT: crates/playlists/src/m3u.rs#part1 (lines 1-428)
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = **lines 1-428**: module docs, `MAX_LINE_BYTES`, `Parsed`, `parse`/`parse_detailed`/`parse_bytes`, the canonical `write` + `write_ext_x_key`/`write_byte_range`. Boundary at the `// ------ HLS attribute lists` section banner (430) — 19 lines short of the equal split (447) to keep the format layer and the attribute/URI layer separate; part2 (W10-A10) = 430-894 (attribute-list parsing, `parse_ext_x_key`/`parse_ext_x_map`, `parse_duration`, and the whole RFC 3986 `resolve_uri` block). I read 430-510 for context only. |
| Lines / functions | 428 / 9 (see §4) |
| File verdict | UNREACHABLE (real, fuzz-pinned parser + canonical writer — but no shipped path parses any playlist file) |
| Reachable from shipped entry point? | **NO.** The `playlists` crate's only cross-crate consumer is `streaming` (rg `-ln "playlists::\|use playlists" crates/ apps/`), whose only M3U caller (`remote_playlist.rs`) is itself unreachable (my file 4). The shipped CLI's dispatch answers every `playlist.*` command with a typed refusal — `apps/cli/src/dispatch.rs:190-192`: `id.starts_with("playlist.")` → owner "the playlist model is a Phase-4 card" — and the desktop shell delegates to the same host (`apps/desktop/src/dispatch.rs:211-213`); its own test pins the behavior: dropping `/tmp/queue.m3u` → `UnsupportedForHost` for `playlist.load` (`apps/desktop/src/shell.rs:541-556`, green in baseline). plugin-api's `playlist.rs` does NOT use the crate (doc comments only; plugin-api/Cargo.toml has no `playlists` dep). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `rg -n "^(pub )?(fn\|struct\|enum\|impl)" crates/playlists/src/m3u.rs` (outline; no in-file test module); Read 1-510; `rg -ln "playlists::\|use playlists" crates/ apps/` (consumers: streaming only + own tests); `cat crates/plugin-api/Cargo.toml` (no playlists dep — kills the plugin-api path); `grep -n "PlaylistLoad" apps/desktop/src/` + Read dispatch.rs:230-261, 375-401 (desktop funnel); `sed -n '185,196p' apps/cli/src/dispatch.rs` (CLI typed refusal); `sed -n '537,557p' apps/desktop/src/shell.rs` (unserved pin); `sed -n '4363,4385p' audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (m3u_suite 18/18 ok); `sed -n '1159,1168p' docs/feature-matrix.toml` (playlist-m3u row). |

## 1. One-paragraph verdict
The M3U/HLS-subset parser is careful, honest work: BOM/CRLF tolerance, a 64 KiB per-line cap with a typed error, dirty-world degradation to warnings for every damaged metadata shape, stateful `EXT-X-KEY`/`MAP` stamping, a canonical delta-encoded writer whose round-trip law is pinned against fixtures AND a dirty corpus AND fuzz-lite (18/18 suite green in baseline — including `ten_megabyte_line_is_a_typed_error`, `lone_cr_file_does_not_panic`, `fuzz_lite_random_bytes_10k_no_panic`). The wave's question answers itself for the whole `playlists` crate here: **no shipped binary parses any playlist file** — the CLI refuses `playlist.load` as "a Phase-4 card", the desktop pins that refusal in its own tests, and remote playlists are unwired; the matrix row `playlist-m3u` nevertheless says "partial" on three platforms (F-001). Internal defects are tolerance-policy inconsistencies: silent overwrite of doubled pending metadata (F-002), valueless unknown `EXT-X-*` tags mis-classified as damage (F-003), and a writer that cannot express a `MAP`→absent transition for hand-built playlists (F-004).

## 2. What actually works (verified)
— `tests/m3u_suite.rs` 18/18 `ok` — `orchrun-test-workspace-nofailfast.log.txt:4363-4385`: goldens (basic extended, plain headerless, directives-in-order, comments/blanks), `roundtrip_all_fixtures_and_dirty_corpus`, `bom_and_crlf_tolerance`, `line_cap_boundaries`, `non_finite_durations_are_bad_duration`, `repeated_header_anywhere_is_ignored`, fuzz-lite (5k mutations + 10k random bytes, no panic), `write_canonical_golden_pin`.
— Panic-freedom of the line loop traced: `position(b'\n')` → `split_at(idx)` is char-boundary-safe ('\n' is ASCII); `strip_suffix('\r')` likewise; every `strip_prefix`/`find`/`split_once` on `&str` is boundary-safe by construction.
— Statefulness traced: `cur_key`/`cur_map` persist across entries and are cloned per segment (289-290) — matches the doc's "stamps the effective key on each segment"; `METHOD=NONE` clears via `parse_ext_x_key` returning `Some(None)` (part2, cross-checked at 230-236).

## 3. Findings

### F-001 [S2]
Where: crates/playlists/src/m3u.rs:1-2 vs docs/feature-matrix.toml:1159-1166
Code:
```
//! Extended/plain M3U and the HLS `#EXT-X-*` media-playlist subset
//! (cards 4.D1 + 5.O1a), plus RFC 3986 URI joining.
```
```
id = "playlist-m3u"
...
description = "Parse (and write) M3U/M3U8 playlists incl. attributes"
...
status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }
```
What is wrong: The matrix asserts M3U parse/write is "partial" on windows/linux/macos, citing this file and its 18-test suite as evidence — but every shipped entry point refuses to touch a playlist file: `playlist.load` (and the whole `playlist.*` family) is answered with the typed `unsupported-for-host` refusal owned by "the playlist model is a Phase-4 card" (apps/cli/src/dispatch.rs:190-192); the desktop funnel routes dropped `.m3u` files to exactly that refusal (apps/desktop/src/shell.rs:541-556, test-pinned); the remote-playlist consumer is unwired (my remote_playlist.rs audit). A user cannot load an M3U — local or remote — in any shipped binary.
Why it matters: This is the same claim-vs-product drift as network-dash/network-hls, now on the playlist-format category: three platform columns say "partial" for a capability the product refuses with "Phase-4 card". The matrix's own convention (network-remote-playlists = not_started for implemented code) shows the statuses are not being held to one standard.
What correct looks like: One standard across the matrix: either "partial" means product-reachable-partial (then this row is not_started), or it means library-partial (then network-remote-playlists/auth are wrong the other way).
Confidence: HIGH — Verification: read (CLI/desktop dispatch + refusal-owner table + shell test) + rg consumer map.

### F-002 [S4]
Where: crates/playlists/src/m3u.rs:187-193 (pattern repeated at 220-227, 202-208)
Code:
```
                let dur = parse_duration(dur_text);
                if dur.is_none() {
                    warnings.push(ParseWarning::BadDuration { line: line_no });
                }
                duration = dur;
                title = title_text.map(str::to_string).filter(|t| !t.is_empty());
                mark_pending(line_no, &mut pending_since);
```
What is wrong: A second `#EXTINF` before the location line silently overwrites the first's duration/title — no warning, the earlier metadata just vanishes. Same silent-overwrite for a doubled `#EXT-X-BYTERANGE` (223) and a doubled `#EXT-X-DISCONTINUITY` (folds into one flag, 206). Every OTHER dropped-metadata shape warns (`BadDuration`, `DanglingEntry` at 301-303) — the module's stated posture is "warnings … name dropped metadata" (102-104).
Why it matters: Dirty-world generators do emit doubled tags; the parser's diagnostics are its selling point, and here one damage class is invisible. A user debugging a broken playlist gets no signal that half the EXTINF lines were discarded.
What correct looks like: Warn on overwrite (e.g. `ParseWarning::DuplicateMetadata { line }`) or at least record it in the dirty-corpus test expectations.
Confidence: HIGH — Verification: read (assignment order traced; no warning emission on the overwrite paths).

### F-003 [S4]
Where: crates/playlists/src/m3u.rs:44-46 and 247-259
Code:
```
//! * Unknown `#EXT-X-*` tags (`VERSION`, `TARGETDURATION`,
//!   `MEDIA-SEQUENCE`, `PLAYLIST-TYPE`, …) stay ordered directives —
//!   preserved verbatim, never invented into fields we do not model.
```
```
                        match value {
                            Some(v) => {
                                directives.push((name.to_string(), v.to_string()));
                                mark_pending(line_no, &mut pending_since);
                            }
                            None => {
                                warnings
                                    .push(ParseWarning::DirectiveMissingColon { line: line_no });
                            }
                        }
```
What is wrong: A valueless unknown `#EXT-X-*` tag — `#EXT-X-INDEPENDENT-SEGMENTS`, `#EXT-X-ALLOW-CACHE`, both legal RFC 8216 tags with no value — takes the `None` arm and is reported as `DirectiveMissingColon` damage and dropped, not "preserved verbatim" as the module doc promises. The doc's example list happens to contain only valued tags, so the promise and the code disagree on the valueless case.
Why it matters: Spec-legal input is classified as dirty; a conformance run against a real-world playlist with `EXT-X-INDEPENDENT-SEGMENTS` reports warnings for well-formed lines, training users to ignore the warning channel.
What correct looks like: Valueless unknown `EXT-X-*` tags are either preserved as value-less directives (a `Vec<String>` side list) or silently ignored per RFC 8216 §4.1 "ignore unrecognized tags" — with the doc updated to say which.
Confidence: HIGH — Verification: read (the `None` arm is the only path for a colon-less `EXT-X-` line).

### F-004 [S4]
Where: crates/playlists/src/m3u.rs:345-359 (with 141-142)
Code:
```
        let map = entry.hls.map.as_ref();
        if map != prev_map {
            if let Some(m) = map {
                out.push_str("#EXT-X-MAP:URI=\"");
```
What is wrong: The MAP delta encoder emits a line only when the value is `Some` and changed. A hand-built `Playlist` with `map: Some(entry1)` then `map: None(entry2)` writes no clearing line (HLS has no `#EXT-X-MAP:METHOD=NONE`), so re-parsing stamps entry2 with the stale map — `parse(write(x)) != x` for constructed inputs. The round-trip law is carefully scoped to "every input that parses" (module doc :90-92), and a parsed playlist can never contain that shape (the parser never clears `cur_map`), so the law itself holds — but the public `Playlist` type permits a state the format cannot express, and the writer silently coerces it.
Why it matters: API consumers composing playlists programmatically (the exporter's stated audience, doc 05 §2.7) get silently wrong output — the written file implies an init segment the data said was absent.
What correct looks like: Either `write` refuses the shape typed (a `Result` with a "MAP cannot be cleared mid-playlist" error), or the type makes it unrepresentable.
Confidence: HIGH — Verification: read (parser never clears `cur_map`; writer's `None` arm is a no-op).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-92 | REAL | S4 (F-003) | Format model, dirty-world contract, round-trip law — each claim checked; the unknown-tag promise has the valueless gap (F-003); "never panics on any byte input" verified by suite fuzz tests. |
| `MAX_LINE_BYTES` | 97-100 | REAL | — | 64 KiB; pinned by `line_cap_boundaries` + `ten_megabyte_line_is_a_typed_error`. |
| `Parsed` | 105-111 | REAL | — | Warnings in source order; "warnings never alter the entry set" holds (all warning paths leave entries untouched). |
| `parse` | 113-116 | REAL | — | Warning-discarding wrapper. |
| `parse_detailed` | 118-309 | REAL | S4 (F-002, F-003) | Core loop: BOM-once, per-line cap, CR strip, trim; EXTINF (first-comma split, empty-title→None); EXT-X dispatch (stateful KEY/MAP, pending BYTERANGE/DISCONTINUITY); unknown EXT directives preserved; plain comments ignored; location flush moves metadata + stamps HLS state; dangling-at-EOF warns. Char-boundary-safe throughout (§2). Entry count unbounded but proportional to input (no amplification). |
| `parse_bytes` | 311-316 | REAL | — | UTF-8 typed refusal; `garbage_bytes_are_typed_invalid_utf8` green. |
| `write` | 318-388 | REAL | S4 (F-004) | Canonical output: header, directives in order, delta KEY/MAP, DISCONTINUITY, BYTERANGE, shortest-decimal EXTINF, location; ENDLIST closes. KEY→None emits `METHOD=NONE` (clearable); MAP→None emits nothing (F-004). Round-trip pinned on fixtures + dirty corpus. |
| `write_ext_x_key` | 390-419 | REAL | — | Canonical attr order; Other(method) verbatim; well-formedness rests on parse-origin invariants (documented at 319-321). |
| `write_byte_range` | 421-428 | REAL | — | `<len>[@<off>]`. |

## 5. Test honesty
No in-file tests; the suite is `crates/playlists/tests/m3u_suite.rs` (18 tests, 18/18 `ok` — orchrun log :4363-4385).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `golden_*` (4) | YES | Exact parsed-model equality on goldens incl. directive order. |
| `roundtrip_all_fixtures_and_dirty_corpus` | YES | `parse(write(parse(x))) == parse(x)` over every fixture + the dirty corpus — breaks on any writer drift. |
| `write_canonical_golden_pin` | YES | Byte-exact canonical output. |
| `line_cap_boundaries` / `ten_megabyte_line_is_a_typed_error` | YES | 64 KiB boundary exactness. |
| `bom_and_crlf_tolerance` / `lf_crlf_mix` / `lone_cr_file_does_not_panic` | YES | Terminator matrix incl. the lone-CR whole-file-one-line shape. |
| `non_finite_durations_are_bad_duration` | YES | NaN/Inf EXTINF degrade to warnings. |
| `fuzz_lite_*` (2) | YES (panic class) | 5k mutations + 10k random inputs, no panic. |
| `dirty_fixture_degrades_honestly` / `empty_and_comment_only_inputs` / `repeated_header_anywhere_is_ignored` / `garbage_bytes_are_typed_invalid_utf8` | YES | Each pins a named tolerance/refusal. |
| **Gap**: no test pins doubled-EXTINF/BYTERANGE overwrite (F-002), valueless unknown EXT-X tags (F-003), or the MAP→None constructed shape (F-004). | | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| playlist-m3u windows/linux/macos = "partial" | feature-matrix.toml:1166 | **Drift (F-001)** | Every shipped entry point refuses playlist files (CLI dispatch :190-192; desktop shell test :541-556 green). |
| "38/0" m3u tests, "EXTINF sums vs ffprobe", "RFC 3986 join 23+19 pinned", "10k fuzz runs" | feature-matrix.toml:1167 | REAL | 18/18 m3u_suite green (orchrun :4363-4385); the remaining counts live in hls_attrs.rs/part2 — not re-verified here. |
| "the parser never panics on any byte input (pinned by fuzz-lite)" | m3u.rs:69 | REAL | `fuzz_lite_random_bytes_10k_no_panic` + `fuzz_lite_mutations_of_valid_playlist_5k` green. |
| "Unknown `#EXT-X-*` tags … stay ordered directives — preserved verbatim" | m3u.rs:44-46 | SHALLOW for valueless tags | F-003: valueless unknown tags warn + drop. |
| "roundtrip law `parse(write(parse(x))) == parse(x)` … for every input that parses" | m3u.rs:89-92 | REAL | `roundtrip_all_fixtures_and_dirty_corpus` green; law's scope honestly excludes constructed inputs (F-004 notes the hazard). |

## 7. Dead / duplicated / unreachable code
- Production-unreachable: no shipped path calls `parse`/`parse_bytes`/`write` (header rg; `playlist.*` = typed refusals in both shipped hosts).
- No dead code within part1; all helpers (write_ext_x_key/write_byte_range) have their caller in `write`.

## 8. Dependencies & flags
- `crate::error` (ParseError/ParseWarning), `crate::types` (HlsKey/HlsMap/ByteRange/Entry/Playlist). std-only. No feature gates, no unsafe, no unwrap/expect (workspace lints). The 64 KiB line cap + UTF-8 refusal are the only input bounds — no total-size cap (caller's concern; the streaming layer bounds bodies at 8 MiB).

## 9. Open questions for the next agent
- part2 (W10-A10): does `parse_duration` (643) accept negative values other than `-1` (e.g. `-5`)? Part1 preserves `-1` as "the spec's unknown marker" — if arbitrary negatives pass, `duration: Some(-5.0)` enters the model and the live session's `live_edge_wallclock_ms` casts them to 0. Also: `resolve_uri`'s Windows-path detection (`is_windows_path`, 789) vs a location like `C:` alone.
- Whoever audits `apps/cli/src/dispatch.rs` / command catalog: the refusal owner string says "the playlist model is a Phase-4 card" — Phase 4 has shipped (this crate); the string is stale and misdirects users to a wave that already landed. Worth an S4 on the dispatch file.
