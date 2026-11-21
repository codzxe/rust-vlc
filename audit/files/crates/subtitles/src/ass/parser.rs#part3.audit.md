# AUDIT: crates/subtitles/src/ass/parser.rs#part3 (lines 979–1469)
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-21 |
| Lines / functions | 491 audited (chunk part3 of 1470; assignment loc split 489/489/491) / 17 tests + in-test `check` closure + xorshift `next` |
| File verdict | REAL (this chunk is the test module; executed in baseline) — the production code it pins is UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | Tests: YES via `cargo test --workspace` (baseline run #3: all 17 `ass::parser::tests` ok). Production under test: NO — no Cargo.toml edge from apps/cli or apps/desktop-tauri to `subtitles` (only fuzz/Cargo.toml:50). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read tool 955-1204, 1205-1470 (chunk + boundary context), 785-934 (production fns pinned by these tests, for mutation analysis); `rg -n "mod tests|fn |#\[test\]"` → item map (mod tests starts :941; 17 test fns); `rg -n "200_000|Picture|Sound|Movie|Command|budget"` → no budget enforcement in production, 6 non-text prefixes at :556-566; baseline `orchrun-test-workspace-nofailfast.log.txt` → 17/17 `ass::parser::tests` ok incl. `fuzz_lite_survives_junk_and_mutations`; `sed -n '947,961p' \| wc -c` → fixture 674 source bytes (fuzz input ceiling ~4.1 KB) |

## Chunk boundary note
wave-00 assignments.tsv splits parser.rs 489/489/491 → part1 = 1–489 (A09), part2 = 490–978 (A10), part3 = 979–1469 (mine). Part3 is the test module: `mod tests` opens at :941 (part2) — my range starts mid-test inside `parses_script_info_and_style` (header 976–978 belongs to part2; body 979–997 is mine). Fixtures `FFMPEG_LIKE` (:947) and `DEFAULT_STYLE_AND_EVENTS` (:967) and the `plain_text` wrapper (:933) are part2's; my tests reference them. My range ends at :1469 (`}` closing `mod tests`); :1470 is a trailing blank/EOF line.

## 1. One-paragraph verdict
This is the strongest test code I have audited in the wave: seventeen executed tests that pin the parser with exact-equality damage vectors, per-counter statistics, spec-correct alignment conventions (numpad 1–9 and SSA-legacy 1–11 with bare-4/8 invalid — both verified against the SSA/ASS convention), overflow-proof time arithmetic via both positive and negative cases, and a genuine 2000-run deterministic mutation fuzzer whose invariants (dense 1-based ids, start≤end, damage line numbers within document bounds) would catch a wide class of regressions. All 17 ran green in the baseline (`ass::parser::tests` … ok, 17/17). The two defects are honesty-level: the fuzzer's "200 KB budget" assertions are mathematically incapable of failing (inputs are bounded at ~4.1 KB by construction, and `parse()` enforces no budget at all), and three of the six non-text event prefixes plus the non-final-`Text` comma path are implemented in production (:556–566, :588–592) but pinned by no test.

## 2. What actually works (verified)
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt` — all 17 `ass::parser::tests::*` `ok` (grep count = 17), inside the subtitles lib run "running 267 tests … 267 passed; 0 failed" (:5269+). The matrix evidence string's "2000 … fuzz-lite runs clean" is `RUNS: u64 = 2000` (:1392) — real and executed.
— Mutation-sensitivity verified by reading the pinned production code (part2 range, read-only for verification): `parse_time` (:785–816) is fully `checked_*` arithmetic with an explicit `i64::MAX` guard (:812) — the tests' overflow (`9999999999999:00:00.00` → None), 7-digit-fraction, bare-dot, negative and lenient-minute cases each target a distinct branch; `map_alignment` (:825–863) numpad 1–9 and legacy `v&3`/`v>=9`/`v>=5` decode exactly what `alignment_conventions`/`v4_legacy_section_uses_legacy_alignment` assert, including the SSA convention that bare 4/8 and 12 are invalid; `parse_ass_colour` (:873–894) BGR order, alpha inversion (`255 - ass_alpha`), trailing-`&`, `&h` prefix, and the negative-decimal two's-complement wrap (`-1` → 0xFFFFFFFF → white/transparent) all match the pins at :1044–1064; `parse_decimal_milli` (:898–928) sign, `.5`, 4-digit-fraction rejection match :1361–1369.
— Damage-ledger pinning is exact-equality (`assert_eq!(doc.damage, vec![…3 items…]`, :1226–1245), not `.any()` — a reordered or extra damage event fails the test.
— G-2 override-tag claims pinned end-to-end: `tolerated_field_damage_and_counters` (:1258–1295) asserts the styled segment carries the Default style's font name/size/colour (inheritance), `applied_override_tags == 2`, `dropped_tags.get("i") == None` — this would fail if the G-2 engine regressed to dropping `\i`.
— SSA legacy semantics verified against the SSA V4 alignment convention (1–3 bottom, +4 top, +8 mid; 4/8 bare invalid): `legacy(5)=Top/Left`, `legacy(11)=Middle/Right`, `legacy(4/8/12)=None` — all correct.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/ass/parser.rs:1385-1389, 1402-1404, 1413
Code:
```
    /// Deterministic fuzz-lite (card 5.M2 AC): LCG-seeded junk,
    /// truncations and byte mutations of a valid document, all
    /// under the 200 KB budget — zero panics, and the invariants
```
```
        let check = |input: &str| {
            let bytes = input.as_bytes();
            assert!(bytes.len() < 200_000);
```
```
                assert!(cue.text.total_len() < 200_000);
```
What is wrong: the two `< 200_000` assertions are tautologies. Junk-mode inputs are `next() % 4096` bytes (:1433); mutation-mode inputs are the 674-source-byte `FFMPEG_LIKE` fixture + one ~64-byte line + at most 23 single-byte insertions (:1441–1454) — every input is < ~4.2 KB by construction, three orders of magnitude under the asserted bound. No input the harness can generate exercises anything near 200 KB, and `rg "200_000|budget"` shows `parse()` enforces no input budget at all (the only occurrences of the constant are these two asserts).
Why it matters: a test that cannot fail is not evidence; the doc comment sells "under the 200 KB budget" as if budget behavior were being verified. Anyone reading the test (or the matrix evidence citing "fuzz-lite runs clean") believes size-limit behavior is proven when nothing of the sort runs.
What correct looks like: either delete the two asserts and the budget phrase, or add a mode that generates inputs crossing the actual limit (e.g. a 250 KB junk document) and pin the real policy (accept, truncate, or typed damage — whichever the parser's contract is).
Confidence: HIGH — Verification: read + input-bound arithmetic (fixture measured: `sed -n '947,961p' | wc -c` → 674; junk cap 4095 at :1433; ≤23 inserts at :1441).

### F-002 [S4]
Where: crates/subtitles/src/ass/parser.rs:1372-1383 (contrast production :556-566 and :588-592)
Code:
```
             Command: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,!some command\n\
             Picture: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,image.bmp\n";
```
What is wrong: production dispatches six non-text prefixes (`Comment:`, `Picture:`, `Sound:`, `Movie:`, `Command:`, `Unknown:`) and a non-final-`Text` comma rule (:588–592), but the tests pin only `Comment` (via :1262), `Command` and `Picture` (:1377–1378). `Sound:`, `Movie:`, `Unknown:` lines and a `Text` field that is not last with commas in it are unpinned — a regression in those branches (e.g. `Unknown:` accidentally rendered as a cue) would pass this suite.
Why it matters: coverage gap, not a behavior bug — the unpinned branches read correct.
What correct looks like: extend `non_text_events_counted_not_damaged` with `Sound:`/`Movie:`/`Unknown:` lines and add one non-final-`Text` comma case.
Confidence: HIGH — Verification: read (both sites).

## 4. Per-function table (part3: lines 979–1469)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parses_script_info_and_style` (body; header part2's) | 976-997 | REAL | — | pins info lookup, style fields, BGR white, alignment, flavour; damage-empty assert |
| `parses_dialogue_times_and_text` | 999-1020 | REAL | — | pins µs-exact times, plain text, `\N` hard newline counter, positioning inheritance |
| `time_parsing_edges` | 1022-1040 | REAL | — | 15 cases incl. overflow, 7-digit fraction, bare dot, negative, lenient minutes; each targets a distinct `parse_time` branch (verified by reading :785-816) |
| `colour_forms` | 1042-1065 | REAL | — | 12 forms; BGR, alpha inversion, trailing `&`, decimal, `-1` wrap, 9-digit reject — all verified against :873-894 |
| `alignment_conventions` | 1067-1119 | REAL | — | numpad 1/2/5/9 + 0/10 invalid; legacy 1/2/5/7/9/11 + 4/8/12 invalid — matches SSA convention exactly |
| `v4_legacy_section_uses_legacy_alignment` | 1121-1144 | REAL | — | [V4 Styles] → legacy flavour, raw_alignment 11 → Middle/Right, Bold -1 → true; cue inherits |
| `custom_event_format_orders_fields` | 1146-1162 | REAL | — | custom Format order; UnknownStyle damage first; non-final Text single-field (no-comma case) |
| `text_commas_absorbed_into_text_field` | 1164-1172 | REAL | — | final-Text comma absorption pinned |
| `text_rules_override_blocks_and_escapes` | 1174-1214 | REAL | — | G-2 pins: `\b` applied not dropped, `{}` preserved quirk, unterminated `{`, `\N`/`\n`/`\h`, `\k` dropped+counted, `\pos` applied, UTF-8 scalar walks (日本語), `{ }` stripped |
| `malformed_times_skip_the_cue_with_typed_damage` | 1216-1246 | REAL | — | exact-equality 3-entry damage vec; line numbers 7/9/10 match the 6-line fixture doc comment |
| `zero_duration_cue_is_legal` | 1248-1256 | REAL | — | start==end legal, interval empty, no damage |
| `tolerated_field_damage_and_counters` | 1258-1295 | REAL | — | BadField Layer/MarginL, margin/name/effect override counters, comment counter, G-2 styled run with style-inherited font/size/colour |
| `unknown_sections_and_bom_and_crlf` | 1297-1314 | REAL | — | BOM, CRLF, `;` comment, 2 skipped sections, title |
| `style_line_damage_paths` | 1316-1357 | REAL | — | BadFieldCount(6,1) skip; kept style with 0-size/None-colour/default-alignment; BadField×2 + BadColour + BadAlignment |
| `decimal_milli_edges` | 1359-1370 | REAL | — | 9 cases incl. `.5`, `-3.25`, `+7`, 4-digit reject — verified vs :898-928 |
| `non_text_events_counted_not_damaged` | 1372-1383 | REAL | S4 | F-002: Sound/Movie/Unknown unpinned |
| `fuzz_lite_survives_junk_and_mutations` (+`check`, xorshift `next`) | 1385-1468 | REAL | S3 | F-001: 2000 real runs, junk/mutate/truncate/lossy-UTF-8 modes, dense-id + interval + damage-line invariants — but the two budget asserts are vacuous; lossy-decode repair honestly commented (:1460-1464) |

## 5. Test honesty (this chunk IS the tests; mutation reasoning per test)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_script_info_and_style | YES | field-by-field equality incl. Option colours and flavour |
| parses_dialogue_times_and_text | YES | µs-exact times + text equality + stats counter |
| time_parsing_edges | YES | per-branch positives/negatives; overflow case fails if `checked_mul` removed (release-mode wrap → wrong Some) |
| colour_forms | YES | byte-exact RGBA expectations incl. alpha-inversion corner `-1` |
| alignment_conventions | YES | 9 pinned mappings + 3 invalid rejections |
| v4_legacy_section_uses_legacy_alignment | YES | flavour dispatch + raw value + bold convention + cue inheritance |
| custom_event_format_orders_fields | YES | custom Format ordering + UnknownStyle |
| text_commas_absorbed_into_text_field | YES | exact text equality with embedded commas |
| text_rules_override_blocks_and_escapes | YES | counters + exact strings for 12 quirks (would catch G-2 regressions) |
| malformed_times_skip_the_cue_with_typed_damage | YES | exact damage vec equality (order + content) |
| zero_duration_cue_is_legal | YES | len==1 + interval.is_empty + damage empty |
| tolerated_field_damage_and_counters | YES | counter equalities + styled-segment structural equality |
| unknown_sections_and_bom_and_crlf | YES | skipped_sections count + title + cue text |
| style_line_damage_paths | YES | style count, first-damage shape, filtered counts |
| decimal_milli_edges | YES | 6 positives + 3 rejects |
| non_text_events_counted_not_damaged | PARTLY | pins Command/Picture; Sound/Movie/Unknown branches unpinned (F-002) |
| fuzz_lite_survives_junk_and_mutations | YES (core invariants) / NO (budget asserts) | dense-id/interval/line-range asserts are real; the two `< 200_000` asserts cannot fail (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "2000+2048 fuzz-lite runs clean" | feature-matrix.toml `subtitle-ass-ssa` evidence | REAL (the 2000 half) | `RUNS = 2000` (:1392); baseline `fuzz_lite_survives_junk_and_mutations … ok` (17/17 grep). The 2048 half lives in ass/fuzz.rs (another agent) |
| "G-2: 267/0 lib tests incl. per-tag suites" | same row | REAL | baseline :5269+ "running 267 tests … 267 passed; 0 failed" |
| "3/3 real-ffmpeg ASS docs exact" | same row | REAL-UNPROVEN (here) | tests/ass_ffmpeg_ac.rs 4/4 ok in baseline (:5542) — "3/3" vs 4 tests is a stale count or counts documents not tests; that file is not in my chunk |
| "all under the 200 KB budget — zero panics" | in-file doc comment :1385-1389 | FALSE (the budget phrase) | F-001: no budget is exercised or enforced; zero-panics half IS proven (2000 runs, baseline) |
| row status windows/linux/macos = "partial" | `subtitle-ass-ssa` | HONEST-BUT-UNREACHABLE | partial is the right word for the styling subset; no shipped binary links the crate at all (rg Cargo.tomls) |

## 7. Dead / duplicated / unreachable code
No dead code in the chunk. The tested production code (parser part1/part2, tags.rs) is unreachable from shipped entry points (crate linked only by fuzz/Cargo.toml).

## 8. Dependencies & flags
Tests are cfg(test) (mod tests :940-941); std-only; no features; no `#[ignore]`; one `lumen-allow-literal` annotation at :1007 consistent with the repo's lint-literals gate.

## 9. Open questions for the next agent
- parser.rs part1/part2 owner: `parse_ass_colour:885` accepts positive decimals > u32::MAX via `dec as u32` truncation (documented for negatives; silent mod-2^32 for huge positives — unpinned by any test; libass-comparable?).
- ass/tags.rs owner (A02): the `{}`-preserved quirk (:1185) — does libass strip empty override blocks? If yes, this pin documents a divergence that should live in the honesty notes.
- ass/fuzz.rs owner: does the "2048" sweep generate wide inputs (approaching any real limit)? If yes, F-001's budget gap is covered there and my S3 stands only against this file.
