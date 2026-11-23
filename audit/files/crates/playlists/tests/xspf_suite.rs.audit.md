# AUDIT: crates/playlists/tests/xspf_suite.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 481 / 14 tests + SplitMix64/random_input helpers |
| File verdict | REAL — tests a REAL parser; parser itself UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (test-only file; see xspf.rs audit — parse_xspf has no shipped caller) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/xspf_suite-e93da4a812baa463` -> 14 passed / 0 failed, exit 0 (binary 18:07 > source 17:35); fixture reads (basic/dirty/namespaced verified on disk, pins match line numbers); sed verifications |

## 1. One-paragraph verdict
A genuinely hostile, spec-derived suite: golden entries pinned to exact `Entry` equality (entity decoding, ms→s duration, verbatim relative locations), a nine-warning dirty-fixture pin with exact line numbers that I cross-checked against the fixture bytes, the D-1 bounds (depth 65 fatal with exact `Display` string, 65 553-byte line fatal with exact byte count), a cross-format M3U bridge round-trip, and two deterministic fuzz-lite harnesses with runtime byte-budget assertions. All 14 pass. Its one real hole: no fixture or case puts field-named elements (`title`/`location`/`duration`/`track`) inside an `<extension>` container — the exact shape where the parser misattributes (xspf.rs F-001, empirically proven there). The suite cannot catch that bug; the namespaced fixture's extension contains only `<app:something>`, which cannot collide.

## 2. What actually works (verified)
— `./target/debug/deps/xspf_suite-e93da4a812baa463` -> `14 passed; 0 failed; 0 ignored` (0.00s). A second prebuilt copy (`xspf_suite-ceb136298bb8619f`) also passed 14/14.
— Fixture cross-check: `dirty.xspf` on disk has `<title>First Track` unclosed at line 6, `&badent;` at line 10, `</wrong>` at line 12, second `<playlist>` at line 13 — matching the pinned warning vector (L151-163) exactly.

## 3. Findings

F-001 [S3]
Where: crates/playlists/tests/xspf_suite.rs:18-20 and 71-112
Code:
```
const BASIC: &str = include_str!("fixtures/basic.xspf");
const NAMESPACED: &str = include_str!("fixtures/namespaced.xspf");
const DIRTY: &str = include_str!("fixtures/dirty.xspf");
```
What is wrong: coverage hole that hides an S2 bug — no test input anywhere in the suite has an `<extension>` (track- or playlist-level) containing a field-named child or a `<track>`. The only extension exercised is `namespaced.xspf`'s `<app:something>` (a non-colliding local name). The extension-misattribution bug proven in the xspf.rs audit (probe output: title shadowed to "PWNED-TITLE", location hijacked to "evil://pwned", phantom entry injected) would pass this suite untouched. Why it matters: the suite's job is to pin the parser's attribution behavior; it pins every attribution path EXCEPT the one that is wrong. What correct looks like: a fixture with `<extension><vlc:location>…</vlc:location></extension>` inside a track asserting the extension value is ignored, and a playlist-level extension containing `<track>` asserting no entry. Confidence: HIGH — Verification: read (rg over suite + fixtures for extension/track shapes) + probe run (xspf.rs audit). Severity S3 (test gap) by itself; the bug it hides is the xspf.rs S2.

F-002 [S4]
Where: crates/playlists/tests/xspf_suite.rs:413-414
Code:
```
            assert!(d.playlist.len() <= buf.len());
            assert!(d.warnings.len() <= buf.len() * 4 + 64);
```
What is wrong: the first bound is near-tautological (an entry needs ≥ 15 bytes of markup, so `entries ≤ bytes` can only fail on an allocator-broken parser); only the warnings bound carries information. Harmless sanity, weakly named "bounded". Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| golden_basic_entities_and_fields | 31-68 | REAL | — | exact Entry equality ×2 + spot fields; entities/&#233;/&#x27; decoded |
| golden_namespaced_prefixes_cdata_and_subsets | 71-112 | REAL | S3 | exact 2-warning pin; CDATA raw; first-wins duplicate; extension case too weak (F-001) |
| bom_and_xml_declaration_tolerance | 115-123 | REAL | — | BOM + `<?xml?>` |
| empty_and_markup_only_inputs | 126-142 | REAL | — | 7 inputs, empty + warning-free |
| dirty_fixture_salvages_entries | 147-187 | REAL | — | exact 9-warning sequence w/ line numbers (verified vs fixture bytes); both tracks salvaged |
| prefix_mismatched_end_tags_and_stray_text | 190-207 | REAL | — | `</xspf:location>` closes `<location>`; `</wrong>` noted |
| nested_track_is_skipped_with_note | 210-221 | REAL | — | inner track dropped + NestedTrack |
| malformed_markup_degrades_not_dies | 224-266 | REAL | — | unquoted attr, bad tag, &#x110000; verbatim, unterminated CDATA swallows rest (exact string) |
| garbage_bytes_are_typed_invalid_utf8 | 269-275 | REAL | — | |
| depth_guard_is_fatal_at_65 | 280-306 | REAL | — | exact (line,depth)=(1,65) + Display string; at-guard parses |
| over_cap_line_is_a_typed_error | 309-331 | REAL | — | exact length 17+65536 + Display string |
| xspf_bridges_through_m3u_with_documented_asymmetry | 336-356 | REAL | — | location/title/duration round-trip; creator/annotation as #creator:/#annotation: text |
| SplitMix64 / random_input | 361-399 | REAL | — | deterministic, XML-steering byte pool |
| fuzz_lite_random_bytes_no_panic | 402-425 | REAL | S4 | 800 inputs; ok_count>150; byte budget pinned <100 000 |
| fuzz_lite_mutations_of_valid_xspf_no_panic | 428-480 | REAL | S4 | 150 MINI + 30 BASIC mutations (round%6==0 → 30); budget pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| golden_basic / golden_namespaced | YES | exact Entry/warning equality incl. line numbers |
| empty_and_markup_only | YES | zero-entry + zero-warning asserts would catch spurious damage notes |
| dirty_fixture_salvages | YES | 9-element warning vector in order; entry field equality |
| prefix_mismatched / nested_track / malformed_markup | YES | exact warning kinds + exact salvaged strings |
| garbage_bytes | YES | exhaustively-matched Err variant |
| depth_guard / over_cap_line | YES | exact tuple + exact Display strings (catches message drift) |
| m3u bridge | YES | field-by-field round-trip equality |
| fuzz_lite ×2 | PARTIAL | panic/hang detector + weak bounds (F-002); would catch panics, not misattribution |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "three handcrafted XML fixtures with varied nesting/entities" | xspf_suite.rs:1-3 | REAL | basic/namespaced/dirty exist on disk; pins match |
| "Fuzz budget … at most ~92 KiB … pinned by runtime byte counters" | xspf_suite.rs:7-11 | REAL | `generated < 100_000` asserts at L421-424, L476-479; tests pass |
| "CI-able: `include_str!` only" | xspf_suite.rs:5 | REAL | no fs reads in the suite |

## 7. Dead / duplicated / unreachable code
None. Note: two stale-adjacent prebuilt binaries both pass (ceb136298bb8619f, e93da4a812baa463) — no discrepancy.

## 8. Dependencies & flags
playlists (parse_xspf*, XspfError/Warning, MAX_LINE_BYTES, MAX_XML_DEPTH, Entry, HlsSegment), no other deps, no features, no unsafe, no I/O beyond include_str!.

## 9. Open questions for the next agent
- Add the extension-opacity fixture (F-001) — until then the xspf.rs S2 stands uncaught by CI.
- The `#creator:`/`#annotation:` bridge lines are written by `Playlist::to_m3u_string` (m3u.rs) — does any M3U consumer re-attach them? The test itself documents they are text-only on re-parse (asymmetry honest).
