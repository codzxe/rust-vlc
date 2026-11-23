# AUDIT: crates/playlists/src/pls.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 446 / 5 prod fns (parse, parse_detailed, parse_bytes, classify_key, split_index) — no in-file tests; 14 integration tests in tests/pls_suite.rs |
| File verdict | REAL — genuine dirty-world-tolerant parser, executed (14/14 integration tests + 2 fuzz-lite no-panic harnesses pass); UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — `parse_pls`/`pls::` has zero callers outside the playlists crate (rg over crates/, apps/, tools/). The `playlist.load` command that would consume it is declared in the catalog (crates/commands/src/catalog/command.rs:359-366) but the desktop engine host refuses it: shell.rs:556 `unreachable!("playlist.load is not served by this engine build")` and the shell test asserts `ErrorCode::UnsupportedForHost` for `playlist.load`. The streaming crate's remote-playlist sniffer handles only M3U/XSPF (remote_playlist.rs:48-53). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/pls_suite-3d474893475cc239` → 14 passed / 0 failed exit 0 (same for pls_suite-03350df70a330eba); rg caller search; sed quote verification |

## 1. One-paragraph verdict
This is a real, careful PLS parser: INI section semantics, 1-indexed sparse-tolerant FileN/TitleN/LengthN, typed non-fatal warnings for every dirty-world shape (bad index, empty location, duplicate key, orphan metadata, lying NumberOfEntries, unexpected Version), fatal only for the 64 KiB line cap and non-UTF-8. I ran the full integration suite: 14/14 pass including the committed dirty.pls fixture and two fuzz-lite no-panic harnesses. The defects are small: the documented "last value wins" for duplicate keys is not upheld for a duplicate `FileN=` with an empty value (the old location silently survives), `NumberOfEntries`/`Version` duplicates never warn while File/Title/Length duplicates do, and the whole parser is dead code in the shipped build — the command that would load a .pls is declared but refused by the engine host.

## 2. What actually works (verified)
— verified by `./target/debug/deps/pls_suite-3d474893475cc239` → `test result: ok. 14 passed; 0 failed` (exit 0): goldens (spec-shaped, sparse indexes), CRLF/BOM/case tolerance, empty/comment-only inputs, pre-section keys with one warning, dirty fixture with exact damage notes, non-finite and negative lengths, overflowing index/count, values containing `=`, garbage bytes → typed InvalidUtf8, over-cap line → typed LineTooLong, lossless M3U bridge, and fuzz-lite (random bytes + mutations of a valid PLS, no panic).

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/pls.rs:193-198 (parse entry) / apps/desktop/src/shell.rs:556
Code: (shell.rs:555-556)
```
                    Some("playlist.load")
                );
            }
            Ok(_) => unreachable!("playlist.load is not served by this engine build"),
```
What is wrong: the parser is unreachable from shipped entry points. `parse_pls` has no production caller; the `playlist.load` wire command exists in the catalog but the desktop engine host answers `UnsupportedForHost`, and the desktop drop-path that classifies `.pls` files (apps/desktop/src/files.rs:198 `Command::PlaylistLoad`) therefore ends in a typed refusal, never in this parser. 446 lines of tested code with no live consumer.
Confidence: HIGH — Verification: read + rg

F-002 [S4]
Where: crates/playlists/src/pls.rs:299-310 vs doc 32-36
Code:
```
            KeyKind::File(idx) => {
                let e = entries.entry(idx).or_default();
                if e.file_set {
                    warnings.push(PlsWarning::DuplicateKey { line: line_no });
                }
                e.file_set = true;
                if value.is_empty() {
                    warnings.push(PlsWarning::EmptyLocation { line: line_no });
                } else {
                    e.file = Some(value.to_string());
                }
```
What is wrong: the module doc promises "Duplicate keys for one index → [`PlsWarning::DuplicateKey`], last value wins" — but a duplicate `FileN=` with an EMPTY value does not overwrite: `e.file` keeps the previous location while the warning says the last value won. A consumer trusting the warning semantics believes the entry points at the new (empty) location; it points at the old one. Same-class inconsistency: `NumberOfEntries` and `Version` duplicates overwrite silently with NO DuplicateKey warning (L290-298) while File/Title/Length duplicates warn.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playlists/src/pls.rs:220-225
Code:
```
        if raw.len() > MAX_LINE_BYTES {
            return Err(PlsError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
```
What is wrong: the cap is checked on `raw` BEFORE the `\r` strip (L230), so a CRLF-terminated line of exactly MAX_LINE_BYTES content bytes is rejected as 65537 bytes. The reported `length` also includes the `\r`. Off-by-one against the doc's "line's byte length excluding the terminator" (L66-67) — `\r` IS part of the terminator in CRLF files. Cosmetic bound pedantry; the cap still bounds.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse | 193-195 | REAL | — | discards warnings |
| parse_detailed | 198-371 | REAL | S4 | the whole engine; BOM, sections, sparse BTreeMap, finalize + cross-checks; F-002 dup semantics |
| parse_bytes | 375-378 | REAL | — | typed InvalidUtf8; fuzz-pinned |
| classify_key | 403-429 | REAL | — | ASCII-lowercase dispatch; Count/Version exact-match |
| split_index | 434-446 | REAL | — | non-digit → BadIndex; `file0`/overflow → BadIndex; bare `file` → Unknown (documented choice) |

## 5. Test honesty
| Test (tests/pls_suite.rs) | Would it fail if code were broken? | Reason |
|---|---|---|
| golden_basic_spec_shaped / golden_sparse_indexes_case_and_comments | YES | exact entries + warnings |
| crlf_bom_and_case_tolerance | YES | |
| empty_and_comment_only_inputs | YES | empty playlist, no warnings |
| pre_section_keys_parse_with_one_warning | YES | one-time warning |
| dirty_fixture_degrades_honestly | YES | exact damage-note list on the committed fixture |
| non_finite_and_negative_lengths | YES | NaN/inf/-1 handling |
| overflowing_index_and_count | YES | BadIndex + EntryCountMismatch |
| value_with_equals_and_unknown_keys | YES | split at FIRST `=` |
| garbage_bytes_are_typed_invalid_utf8 | YES | |
| over_cap_line_is_a_typed_error | YES | 64 KiB cap |
| pls_bridges_losslessly_through_m3u | YES | roundtrip through the shared vocabulary |
| fuzz_lite_random_bytes_no_panic / fuzz_lite_mutations_of_valid_pls_no_panic | WEAK-POSITIVE | no-panic pins only (that is their stated job) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The parser never panics on any byte input (pinned by fuzz-lite)" | pls.rs:45 | REAL | ran both fuzz-lite tests (ok) |
| "Duplicate keys … last value wins" | pls.rs:34-35 | FALSE-in-one-case | F-002: empty duplicate File keeps the old value |
| "a trailing `\r` is stripped" / length excludes terminator | pls.rs:6-8, 66-67 | REAL with S4 pedantry | F-003: cap checked pre-strip |
| "bridges losslessly through [`Playlist::to_m3u_string`]" | pls.rs:51-52 | REAL | pls_bridges_losslessly_through_m3u (ran, ok) |

## 7. Dead / duplicated / unreachable code
The module is dead in the shipped build (F-001). No internal duplication; warning vocabulary mirrors m3u.rs's policy by design.

## 8. Dependencies & flags
std (BTreeMap) + thiserror + crate::m3u::MAX_LINE_BYTES (64 KiB) + crate::types. No features, no unsafe.

## 9. Open questions for the next agent
- When `playlist.load` gets a serving host, does it route by extension (probe.rs's Pls) or content sniffing? Today neither runs.
- Should `LengthN` accept values < -1 (e.g. -5)? Currently stored verbatim as Some(-5.0); the M3U parser's posture is the same — confirm the UI layer tolerates it.
