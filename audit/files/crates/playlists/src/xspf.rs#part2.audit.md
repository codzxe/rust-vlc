# AUDIT: crates/playlists/src/xspf.rs#part2 (lines 519–1037)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 519 (of 1037) / 23 fns + 5 types/enums |
| File verdict | UNREACHABLE (real, fuzz-tested scanner — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — callers are `crates/playlists/tests/xspf_suite.rs`, the `playlists::fuzz` module (fuzz target `fuzz_playlist_xspf`), and `streaming/src/remote_playlist.rs` (itself uncalled) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "parse_xspf\|xspf::" --type rust apps crates fuzz tests` (callers, see above); `rg -n "fn .*\(" crates/playlists/tests/xspf_suite.rs` (12 tests incl. 2 fuzz-lite sweeps); `sed -n` on all quoted ranges; baseline INDEX #3 (playlists tests green). |

## 1. One-paragraph verdict
Part2 is the XSPF byte scanner (tags, attributes, CDATA, comments, PIs, entity decoding) plus the tree-construction helpers (frames, track builder, field application) — and it is the cleanest hostile-input code I have audited in this wave: every slice index is anchored on an ASCII delimiter or a `char::len_utf8` step (no panic path found by trace), every damage construct resyncs with a typed warning, all loops provably advance, and the never-panic contract is pinned by two deterministic fuzz-lite sweeps that ran green in the baseline. The single defect is a doc contradiction: "out-of-range entities are kept verbatim" is false for numeric references that are invalid XML characters but valid Unicode scalars (`&#1;`, `&#xFFFE;`) — they are decoded into the entry text. As with the rest of the playlists crate's consumers, nothing in the shipped `lumen` binary can reach this code.

## 2. What actually works (verified)
— No-panic trace: audited every `take()`/slice call — ends come from `find` of ASCII markers (`<`, `>`, `-->`, `]]>`, `?>`, quotes, `;`, `&`), `pos ± n` after 1-byte ASCII (`<`, `</`, `/`, `=`, quotes), or `char::len_utf8` steps; all char boundaries by construction. `advance_char` maps `None` peek to step 0. Infinite loops excluded: every `next_token`/attr-loop iteration consumes ≥1 byte or breaks (`resync_attr` deliberately consumes `/` — the comment cites the fuzz-caught `<f/creator>` spin, xspf.rs:755-758).
— Execution evidence: `crates/playlists/tests/xspf_suite.rs` — `golden_basic_entities_and_fields` (:31), `golden_namespaced_prefixes_cdata_and_subsets` (:71), `malformed_markup_degrades_not_dies` (:224), `over_cap_line_is_a_typed_error` (:309), `fuzz_lite_random_bytes_no_panic` (:402), `fuzz_lite_mutations_of_valid_xspf_no_panic` (:428) — all green in baseline `cargo test --workspace` (INDEX #3, 2026-11-21). These directly exercise part2's scanner on entity, CDATA, namespace, truncation, and random-byte inputs.

## 3. Findings

### F-001 [S4] "Out-of-range entities are kept verbatim" is false for XML-invalid characters — they decode
Where: crates/playlists/src/xspf.rs:1026-1031 (doc at :29-32)
Code:
```
            match char::from_u32(value) {
                // XML forbids NUL even as a numeric reference.
                Some('\0') | None => None,
                Some(c) => Some(c),
            }
```
What is wrong: the module doc promises "Unknown/malformed/out-of-range entities are kept **verbatim** with a `BadEntity` note (… no invented characters)" (:29-32), but the range check is `char::from_u32` — Unicode-scalar validity, not XML 1.0 character validity. `&#1;`–`&#8;`, `&#11;`, `&#12;`, `&#14;`–`&#31;`, `&#xFFFE;`, `&#xFFFF;` are all invalid XML characters yet valid `char`s: they are decoded into location/title/annotation text instead of staying verbatim with a warning. (NUL is correctly refused; surrogates `&#xD800;` correctly fail `from_u32`.)
Why it matters: control characters and noncharacters land silently in parsed fields — exactly the "invented characters" the doc rules out; also a (minor) smuggling vector into downstream filenames/URLs for whatever eventually consumes `Entry::location`.
What correct looks like: `let ok = matches!(value, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF); ok.then(|| char::from_u32(value))…`.
Confidence: HIGH — Verification: read (XML 1.0 §2.2 Char production vs `char::from_u32` semantics); execution UNVERIFIED (no cargo).

## 4. Per-function table (part2 range; boundary: part1 owns through `Scanner` struct :508-513, `parse*` fns and `Token`)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Scanner::new` (tail) | 515-523 | REAL | — | pos 0, line 1 |
| `take` | 525-533 | REAL | — | newline count on consumed slice; boundary discipline holds (see §2) |
| `peek_char`/`advance_char` | 535-542 | REAL | — | len_utf8 step; None→0 |
| `skip_ws`/`rest_starts_with` | 544-552 | REAL | — | trivial |
| `skip_after` | 554-566 | REAL | — | marker-or-EOF with typed warning; ASCII markers |
| `next_token` | 568-641 | REAL | — | text runs entity-decoded; CDATA raw (correct XML); comment/PI/decl dispatch order `<!--` before `<!` before `</`; `<`+non-name → literal text + BadTag; all paths advance |
| `scan_end_tag` | 643-675 | REAL | — | resync to `>` on junk; empty name → one MalformedEndTag + empty-name End token (part1 :418-420 handles) |
| `scan_start_tag` | 677-753 | REAL | — | both quote styles, EOF-tolerant open quote, empty attr name skipped, unterminated tag salvaged; zero-consumption iterations impossible |
| `resync_attr` | 755-766 | REAL | — | consumes `/` to avoid the documented fuzz spin |
| `Frame`/`FrameKind`/`FieldKind` | 771-806 | REAL | — | data; `from_name` covers the 5 documented fields |
| `TrackBuilder` + `is_set` | 808-834 | REAL | — | `_set` flags distinguish present-empty from absent (drives DuplicateField/TrackWithoutLocation) |
| `close_frame` | 836-861 | REAL | — | forced→UnclosedElement; Field applies unless dup; Track emits entry |
| `apply_field` | 864-904 | REAL | — | location keeps empty-string-but-set (asymmetric with title's `non_empty` — deliberate, structural presence); duration: ms→s, finite+non-negative else BadDuration; `NaN`/`inf` refused via is_finite ✓ |
| `track_entry` | 907-925 | REAL | — | creator→annotation directive order matches doc :51-53 |
| `non_empty`/`local_name`/`is_name_start`/`is_name_char` | 929-947 | REAL | — | local name after last `:`; Unicode-alphabetic name starts (tolerant superset of XML) |
| `line_over_cap` | 951-970 | REAL | — | byte-accurate, first violation only, 1-based lines; BOM stripped upstream |
| `decode_entities` | 975-1008 | REAL | — | fast path no-`&`; entity scan bounded (name ≤32); verbatim+BadEntity fallbacks correct for `&;`, `&amp`, `&#…40digits;` |
| `decode_one` | 1012-1033 | REAL | S4 | F-001; NUL+surrogates refused; `X`-prefix hex tolerated (XML allows only `x`) |
| `strip_hex` | 1035-1037 | REAL | — | `x` or `X` |

## 5. Test honesty
No in-file tests in part2's range; the integration suite `crates/playlists/tests/xspf_suite.rs` (12 fns) is the test surface. Reasoned mutations: breaking `decode_entities`' verbatim fallback fails `golden_basic_entities_and_fields` (:31 asserts decoded `&amp;`-class values); breaking the resync (`resync_attr` stopping at `/`) reintroduces the documented `<f/creator>` hang, which `fuzz_lite_mutations_of_valid_xspf_no_panic` (:428) would catch by timeout/panic; breaking the line cap fails `over_cap_line_is_a_typed_error` (:309). Gap: no test pins F-001's XML-invalid-character refs (all suite entities are legal ones).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Unknown/malformed/out-of-range entities are kept verbatim with a BadEntity note" | xspf.rs:29-32 | FALSE for XML-invalid chars | F-001: `&#1;`/`&#xFFFE;` decode |
| "Attributes are parsed (both quote styles, entity-decoded values)" | xspf.rs:23-25 | REAL | :719-741 (double+single quotes, EOF-tolerant) |
| "comments …, processing instructions …, CDATA sections, and `<!…>` declarations … skipped naively to the next `>`" | xspf.rs:26-28 | REAL | :585-623 (dispatch order verified) |
| "Never panics on any input (fuzz-pinned)" | xspf.rs:36 | REAL | trace §2 + fuzz-lite suites green in baseline #3 |
| "duration (milliseconds in XSPF) → seconds" | xspf.rs:49-50 | REAL | :899 `ms / 1000.0` |
| 5.O1b XSPF parse card | docs/13-agent-wave-program.md (wave 5.O) | REAL-UNPROVEN + UNREACHABLE | tests green (baseline #3); no shipped consumer |

## 7. Dead / duplicated / unreachable code
`xspf.rs` is reachable only from: (a) `crates/playlists/tests/xspf_suite.rs`, (b) `playlists::fuzz` → `fuzz/fuzz_targets/fuzz_playlist_xspf.rs` (cargo-fuzz, not shipped), (c) `streaming/src/remote_playlist.rs:194` (`parse_xspf_bytes`) — whose `open_remote_playlist` has zero callers (verified in my master.rs and resolve.rs audits). The `lumen` CLI has no playlist-open path (`apps/cli/src/dispatch.rs`: "the playlist model is a Phase-4 card" refusal).

## 8. Dependencies & flags
`crate::m3u::MAX_LINE_BYTES` (shared 64 KiB line cap), `crate::types::{Entry, HlsSegment, Playlist}`, `thiserror`. No feature gates (`lib.rs:75 mod xspf` unconditional). No unsafe. Part of workspace build (baseline #1 clean, #3 tests green).

## 9. Open questions for the next agent
- A09 (xspf part1): the `parse_detailed` state machine (:273-475) is yours — my scanner-side trace assumed its `close_frame` ordering; the `skip_depth`/nested-track interplay with `Token::End` (`:414-417` decrements `skip_depth` on ANY end tag including mismatched ones) deserves your explicit check.
- Wave-16 fuzz owner: consider adding `&#1;`/`&#xFFFE;` to the corpus — F-001 means they currently decode instead of warning.
