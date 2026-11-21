# AUDIT: crates/playlists/src/pls.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 446 / 10 (2 error enums — 2+12 variants, 1 result struct, 3 parse fns, 2 private types, 2 classification fns). No in-file tests: the suite lives in `crates/playlists/tests/pls_suite.rs` (wave-16 A04's file) + crate `fuzz.rs` + `fuzz/fuzz_targets/fuzz_playlist_pls.rs` (wave-16 A07). |
| File verdict | UNREACHABLE (real, spec-faithful, fuzz-pinned parser — zero production callers) |
| Reachable from shipped entry point? | NO. `parse_pls`/`parse_pls_bytes`/`parse_pls_detailed` callers: lib.rs re-export, `fuzz.rs:41` (crate fuzz), `tests/pls_suite.rs`. `streaming/remote_playlist.rs` handles M3U+XSPF only (its own doc: "PLS/MPD remote open are not in 6.E2's scope"); `apps/desktop/src/files.rs:53-54` classifies `.pls` paths but routes to `playlist.load`, which that shell's own tests say "is not served by this engine build" (shell.rs:556,836); `apps/desktop-tauri/src` never references playlists. `lumen` CLI has no playlist path at all. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "pls\|parse_pls" crates/playlists crates/streaming/src apps` → caller census (above); `sed -n` on all quoted ranges; `rg -n -A16 "Running tests/pls_suite" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 14/14 ok at 4387-4404; `sed -n '1165,1181p' docs/feature-matrix.toml` → plfmt-pls row |

## 1. One-paragraph verdict
A disciplined, spec-faithful INI-style parser: first-`=` split, ASCII-case-insensitive keys, sparse 1-based indexes ordered by BTreeMap, INI section semantics with foreign sections ignored, an honest 12-variant warning vocabulary that never embeds content (MP §12 verified variant-by-variant), and a genuinely adversarial external suite — golden fixtures, an exact warning-sequence assertion over a dirty corpus, D-1 cap boundary tests both sides, a lossless PLS→M3U→PLS round-trip, and two deterministic fuzz-lite harnesses with byte budgets — all 14 green in the baseline no-fail-fast run. The problems: the parse entry is a shipped-binary orphan (the matrix nonetheless stamps `plfmt-pls: implemented` on all desktop platforms), the line-cap check runs on the raw slice before the documented `\r` strip — so CRLF lines carry an effective cap one byte lower than LF lines and the error's `length` field overstates by one — and negative durations other than the `-1` live marker flow into `Entry::duration` with no warning at all.

## 2. What actually works (verified)
— `tests/pls_suite.rs` 14/14 green: orchrun-test-workspace-nofailfast.log.txt:4387-4404 (golden/sparse/CRLF+BOM/dirty-corpus exact-warning-sequence/non-finite/overflow/first-equals/garbage-bytes/over-cap/at-cap/bridge/fuzz-lite ×2).
— Redaction discipline verified by reading every `PlsError`/`PlsWarning` variant (64-178): line numbers and counts only, no keys/values/locations — matches the MP §12 claim in the doc comment (61-62, 86-87).
— Panic-safety verified by construction + fuzz: slicing only at ASCII `=`/`\n` boundaries (`find('=')`, byte position of 1-byte ASCII), `strip_prefix/suffix` on chars, `parse::<usize/f64>` for numbers; both fuzz-lite harnesses and the crate `fuzz::tests` (empty_and_marker_inputs_survive, hostile_wide_inputs_survive — green in the playlists lib run) hammer `parse_pls_bytes` with no panic.

## 3. Findings

### F-001 [S3] Parser is a shipped-binary orphan while the matrix stamps it "implemented"
Where: crates/playlists/src/pls.rs:193-195
Code:
```rust
pub fn parse(input: &str) -> Result<Playlist, PlsError> {
    Ok(parse_detailed(input)?.playlist)
}
```
What is wrong: no production call chain reaches this function. Remote playlist open is M3U+XSPF only (remote_playlist.rs scope note); the desktop shell classifies `.pls` drops but its `playlist.load` command is unserved in this build; the tauri app never imports the crate; the CLI has no playlist lane.
Why it matters: docs/feature-matrix.toml `plfmt-pls` (line ~1172) claims `status = implemented` on windows/linux/macos — the evidence sentence is true (fixtures, warning asserts, fuzz) but a user of the shipped binary cannot parse a single PLS file; "implemented" reads as product capability, and this is the P0-priority row of the playlist-format family.
What correct looks like: serve `playlist.load` through a host that parses with this crate (the desktop shell already classifies the suffix), or annotate the row "library-complete; no shipped host wires it".
Confidence: HIGH — Verification: rg census (header) + read shell.rs/files.rs/remote_playlist.rs scope notes.

### F-002 [S4] Line-cap check runs before the `\r` strip — CRLF lines get a cap one byte lower, and the reported length overstates by one
Where: crates/playlists/src/pls.rs:220-231
Code:
```rust
        if raw.len() > MAX_LINE_BYTES {
            return Err(PlsError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
        }
        rest = tail;

        // CRLF tolerance, then whitespace trim (values keep their internal
        // bytes verbatim).
        let line = raw.strip_suffix('\r').unwrap_or(raw);
```
What is wrong: the module doc (line 6) says "a trailing `\r` is stripped" — i.e. the line's content excludes the CR — but the cap check tests `raw.len()` BEFORE the strip. A CRLF line whose content is exactly MAX_LINE_BYTES bytes measures MAX+1 and is refused, while the identical LF-terminated line parses; the error's `length` field (documented "excluding the terminator", 66-67) also counts the `\r`.
Why it matters: boundary behavior differs by line-ending style, and the at-cap boundary test (`over_cap_line_is_a_typed_error`, pls_suite.rs:247) only exercises LF — the CRLF off-by-one is untested and unprovable green.
What correct looks like: `let line = raw.strip_suffix('\r').unwrap_or(raw); if line.len() > MAX_LINE_BYTES { … length: line.len() }` — check and report on the stripped line.
Confidence: HIGH — Verification: read both orders; hand-traced a MAX-byte CRLF value line → refused.

### F-003 [S4] Negative durations other than the `-1` live marker pass into `Entry::duration` with no warning
Where: crates/playlists/src/pls.rs:334-337
Code:
```rust
                    match value.parse::<f64>() {
                        Ok(d) if d.is_finite() => e.length = Some(d),
                        _ => warnings.push(PlsWarning::BadLength { line: line_no }),
                    }
```
What is wrong: the only accepted negative per the PLS convention is `-1` (module doc line 19-20: "-1 is the unknown/live marker"); `Length1=-5` (or -0.5, or -9999) parses finite, is stored verbatim, and emits no `PlsWarning` — while the equally-garbage `abc` gets `BadLength`. The suite pins only `-1` (pls_suite.rs:163-168) and never probes other negatives.
Why it matters: a lying duration flows silently into the shared `Entry` vocabulary and onward into any UI/bridge consumer (`to_m3u_string` round-trips it); the parser's own dirty-world posture is warn-on-garbage, and this garbage class is silent.
What correct looks like: warn (a `NegativeLength` variant or `BadLength`) for finite values < 0 other than -1 — or document that all finite numerics are passed through uninterpreted.
Confidence: HIGH — Verification: read the branch; traced `-5` → `Some(-5.0)`, zero warnings; suite has no such case.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum PlsError` (2 variants) | 64-84 | REAL | — | LineTooLong{line,length} + InvalidUtf8; redaction clean; `length` nit → F-002. |
| `enum PlsWarning` (12 variants) | 89-178 | REAL | — | Every dirty-world case the module doc names has a variant; line numbers only. |
| `struct PlsParsed` | 184-190 | REAL | — | playlist + warnings; documented ordering (line-scan then finalize findings) honored by code (203, 349, 364). |
| `parse` | 193-195 | REAL (unreachable) | S3 | Warning-discarding wrapper; F-001. |
| `parse_detailed` | 198-371 | REAL | S4 | The core: BOM-once, CRLF+trim, comments (`;`/`#` — spec says `;` only; `#` is tolerant superset, undocumented delta, harmless), INI sections incl. foreign-section ignore, first-`=` split, sparse BTreeMap order, orphan-metadata finalize, count cross-check. F-002 (cap before strip); F-003 (negative lengths); no panics (ASCII-boundary slicing only). |
| `parse_bytes` | 375-378 | REAL | — | UTF-8 gate typed; delegates. Fuzz-pinned. |
| `struct PartialEntry` | 382-390 | REAL | — | Six-flag sparse slot; `first_meta_line` gives orphan warnings a line. |
| `enum KeyKind` | 393-401 | REAL | — | File/Title/Length/Count/Version/BadIndex/Unknown. |
| `classify_key` | 403-429 | REAL | — | Exact `numberofentries`/`version` matches; prefix+suffix for indexed keys; allocates a lowercase String per key line (O(total input), fine). |
| `split_index` | 434-446 | REAL | — | Bare `file` → Unknown (correct: not damage); non-digit suffix → BadIndex; `0` → BadIndex; overflow (`parse::<usize>` err) → BadIndex — all pinned by suite (overflowing_index_and_count green). |

## 5. Test honesty (suite: crates/playlists/tests/pls_suite.rs — external, wave-16 A04's file; executed evidence for THIS file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| golden_basic_spec_shaped / golden_sparse | YES | Full `Entry` equality ×3 (location/duration/title incl. `-1.0` live marker and fractional 321.5), index-order not line-order. |
| dirty_fixture_degrades_honestly | YES | Exact 10-element warning SEQUENCE with exact line numbers + entry outcomes (last-wins duplicate, empty location kept, foreign-section ignored). The strongest assertion in the suite. |
| crlf_bom_and_case_tolerance / pre_section_keys / value_with_equals / non_finite / overflowing / garbage_bytes / over_cap | YES | Each pins an exact variant/field/line; at-cap boundary tested on the LF side only (F-002's CRLF side untested). |
| pls_bridges_losslessly_through_m3u | YES | Round-trip equality through the M3U writer — catches any field the PLS model fails to carry. |
| fuzz_lite ×2 | YES (panic class) | Deterministic seeds, byte budgets asserted, parse-count floors (>200/>100) keep the harness honest. |
All 14 green: orchrun-test-workspace-nofailfast.log.txt:4387-4404.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parse PLS playlists — status implemented (win/linux/macos); golden truth-sets … fuzz-lite no-panic; 46/0" | docs/feature-matrix.toml `plfmt-pls` (~1172) | REAL-in-library / UNREACHABLE-in-product | 14/14 suite green (log 4387-4404), playlists lib 46/0 — but zero shipped callers (F-001 census). |
| "a parsed PLS therefore bridges losslessly through `Playlist::to_m3u_string`" | pls.rs module docs 51-52 | REAL | pls_bridges_losslessly_through_m3u green. |
| "The parser never panics on any byte input (pinned by fuzz-lite)" | pls.rs module docs 45 | REAL | Both fuzz-lite harnesses + crate fuzz tests green; no panic path found by reading (ASCII-boundary slicing). |
| "Line-based text (LF or CRLF; a trailing `\r` is stripped)" + "over the 64 KiB cap is fatal" | pls.rs module docs 6, 41-42 | FALSE at the boundary | F-002: the strip happens after the cap check — CRLF content at exactly the cap is fatal. |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped hosts (F-001). Internally everything is consumed (parse→parse_detailed; classify_key→split_index; PartialEntry only here). No duplication.

## 8. Dependencies & flags
std + thiserror + in-crate `m3u::MAX_LINE_BYTES`/`types::{Entry, HlsSegment, Playlist}` — matches the crate's no-external-deps posture. No feature gates, no unsafe, no unwrap in production paths (the two `unwrap_or`/`unwrap_or_default` are total). Clippy -D warnings clean in baseline (consistent).

## 9. Open questions for the next agent
- wave-16 A04 (pls_suite.rs owner): add the CRLF-at-cap boundary case (F-002) and a non-`-1` negative-length case (F-003) — both are one-liners against the existing fixtures.
- apps/desktop owner: `playlist.load` unserved — the `.pls` classification in files.rs is currently a promise no shipped host keeps (F-001's wiring gap).
- m3u.rs owner (wave-10 A0?): does the M3U parser share the cap-before-strip ordering (F-002) or the negative-EXTINF silence (F-003)? Same pattern worth checking there.
