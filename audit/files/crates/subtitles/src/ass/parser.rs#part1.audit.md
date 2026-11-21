# AUDIT: crates/subtitles/src/ass/parser.rs#part1 (lines 1-550)
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 1-550 (chunk boundary declared: part1 = 1-550 through `parse_style_line`; part2 = 551-939 [A10]; part3 = tests 940-1469 [A01]) / 18 items |
| File verdict | REAL (library code, baseline-executed tests incl. ffmpeg-fixture differential) / UNREACHABLE from shipped build |
| Reachable from shipped entry point? | NO — only reverse-dep of the crate is fuzz/Cargo.toml:50; `parse()` consumed by fuzz harness + tests only |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | `rg "subtitles" **/Cargo.toml` (fuzz only); `rg "AssRenderCue|AssRenderStyle"` (parser never references render types); `rg "model_positioning"` (never called); read of :1-550 + cross-reads :630-646, render.rs:50-51, :222-226; `sed -n` + `od -c` quote verifies (note: bash display ate `[m` in one sed output; `od -c` confirmed `#[must_use]` intact at :269); baseline: orchrun-test-workspace-nofailfast.log.txt:5281-5296 (parser tests ok), :5301 (fuzz-lite ok), :5540 (lib 267/0), :5542-5550 (ass_ffmpeg_ac 4/0) |

## 1. One-paragraph verdict
This is the best-audited half of the ASS story: `parse()` is genuinely infallible (BOM/CRLF/comment tolerant, typed damage for every degradation it recognizes), the spec-default format tables are the correct V4+ orders, and the module doc's honest ledger matches what the code does — including the candid admission that margins never reach the cue model. Its tests are baseline-executed green, including the ffmpeg-fixture differential (orchrun-test-workspace-nofailfast.log.txt:5281-5296, :5301, lib 267/0 at :5540, ass_ffmpeg_ac 4/0 at :5542-5550). But the parse output is a dead end: nothing renders the cues (cross-ref render.rs#part1 F-001), the entry types of the render half are type-INCOMPATIBLE with what this parser stores (contradicting render.rs's "the parser hands over the already-parsed u32"), and edge handling has real gaps vs libass (case-sensitive record prefixes silently skipped uncounted, first-duplicate-wins, wrong legacy-SSA fallback format).

## 2. What actually works (verified)
— `parse()` section walk traced by hand over a synthetic document: BOM strip (:353), `\r` strip + trim per line (:361), blank/`;`-comment skip (:362-363), section-scoped format reset on every header entry (:382-384), unrecognized sections counted into `skipped_sections` (:377-379). Infallible: no panic path in :349-420.
— `DEFAULT_STYLE_FORMAT` (:315-339) is the correct 23-field ASS V4+ spec order (Name…Encoding); `DEFAULT_EVENT_FORMAT` (:342-344) the correct 10-field Events order (Layer…Text).
— `parse_style_line` field mapping (:494-546): Name/Fontname/Fontsize(milli)/PrimaryColour/Bold/Italic/Underline/StrikeOut/Alignment/MarginL/R/V all bound; unknown styling fields fall through `_ => {}` (:545) matching the "deferred" ledger; too-few fields → typed `BadFieldCount` skip (:467-474); extra fields tolerated (only first `format.len()` read — a superset line still parses).
— `section_name` (:440-444): only `[Name]`-shaped lines match; a Dialogue line can never start with `[`.
— Execution: parser tests green in baseline orchrun-test-workspace-nofailfast.log.txt:5281-5296 (16 tests incl. `text_commas_absorbed_into_text_field`, `malformed_times_skip_the_cue_with_typed_damage`), `ass::parser::tests::fuzz_lite_survives_junk_and_mutations` ok :5301, and `tests/ass_ffmpeg_ac.rs` 4/0 at :5542-5550 (`ffmpeg_basic_three_cues_times_and_text`, `ffmpeg_six_cue_ladder`, `ffmpeg_unicode_and_tag_stripping`, `ffmpeg_header_and_style_parse_clean`) — the record's "3/3 real-ffmpeg ASS docs exact" claim is baseline-executed.

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/ass/parser.rs:349-420 (`parse`, the crate's only real entry point)
Code:
```
pub fn parse(input: &str) -> AssDocument {
    let mut doc = AssDocument::default();
    // Tolerate a UTF-8 BOM and both line endings (lines()' split is
    // normalized below with a \r strip).
```
What is wrong: `parse()` is the crate's most-consumed API, and its complete non-test consumer set is: `fuzz/fuzz_targets/fuzz_sub_ass.rs` (survival harness, discards output) and the in-crate test modules. No shipped binary links the crate; the `AssDocument.cues` it produces have no renderer (render.rs#part1 F-001: the render half consumes different, never-populated types). Parse is a well-built component of a pipeline that does not exist.
Why it matters: The matrix row `subtitle-ass-ssa` (partial ×3 desktop platforms) presents this as player capability; nothing in a shipped `lumen` build can invoke a single line of it.
What correct looks like: A sidecar-subtitle probe in the demux/playback path calling `subtitles::ass::parse` (and a render bridge), or UNREACHABLE-status in the matrix.
Confidence: HIGH — Verification: rg census + baseline engine-info features list.

### F-002 [S3]
Where: crates/subtitles/src/ass/parser.rs:85-92
Code:
```
    /// `Fontsize` as an exact fixed-point value in thousandths
    /// (`16` → 16 000; `20.5` → 20 500) — no float anywhere near
    /// layout (MP §8); malformed tokens parse to 0 + damage.
    pub font_size_milli: i64,
    /// `PrimaryColour` as RGBA8, ASS `&HAABBGGRR`/decimal decoded
```
What is wrong: The stored vocabulary (`font_size_milli: i64`, `primary_colour: Option<Color>` decoded at :89-92, `alignment: Alignment` model 3×3) is disjoint from what the render half consumes (`AssRenderStyle { primary_colour: u32 raw literal, font_size_px: u32, alignment: AssAlignment numpad }` — render.rs:217-227). render.rs:50-51 asserts "the parser hands over the already-parsed u32, so the byte order policy lives in exactly one place, here" — FALSE, the parser hands a decoded `Color`, and render.rs:11-12 calls the union "mechanical" when it needs a re-encode (Color→u32), a milli→px rounding policy, and an Alignment round-trip that exist nowhere in the repo.
Why it matters: The documented integration contract between the two halves is factually wrong; anyone building the promised adapter from these docs discovers the types don't line up.
What correct looks like: Parser stores (or exposes) the raw u32 literal + px-int font size alongside the decoded forms, or the render doc stops claiming it.
Confidence: HIGH — Verification: read of both type definitions.

### F-003 [S4]
Where: crates/subtitles/src/ass/parser.rs:463-465
Code:
```
    let Some(rest) = line.strip_prefix("Style:") else {
        return; // unknown line type in a styles section: skipped
    };
```
What is wrong: A styles-section line not exactly `Style:` (case variations `style:`, `Style :`, leading junk) is silently dropped — no `AssDamage` variant, no `AssStats` counter. Same for `Format:` (:391) and, in part2, `Dialogue:`. libass matches record prefixes case-insensitively. The module's whole premise is the honest degradation ledger ("everything tolerated is counted", :190-191); this skip class is tolerated but invisible.
Why it matters: A real-world file with case-drifted records loses all its styles/cues with a clean damage report — the conformance artifact (R-09) would show zero damage while rendering nothing.
What correct looks like: An `AssDamage::UnknownRecord { line_no, kind }` (or at least a stats counter) for unrecognized record types inside Styles/Events sections.
Confidence: MEDIUM — libass case behavior from spec knowledge, not in-repo source.
### F-004 [S4]
Where: crates/subtitles/src/ass/parser.rs:253-254
Code:
```
    /// `[Script Info]` pairs in file order (duplicates kept — the
    /// later key usually wins in consumers; we stay honest).
```
What is wrong: This crate's own consumers do the opposite: `info()` (:269-275) and `style()` (:285-287) both return the FIRST match (`iter().find`). A file with `PlayResX: 0` then `PlayResX: 384` (or a redefined `Default` style — common in fansub edits; libass lets the later definition win) silently uses the stale first value. The doc's parenthetical describes libass, not this code.
Why it matters: Doc says later-wins, code does first-wins — a consumer reading the doc gets wrong expectations; duplicate-redefinition files mis-parse.
What correct looks like: `info`/`style` return the LAST match, or the doc says "first wins here".
Confidence: HIGH — Verification: read of both accessors.

### F-005 [S4]
Where: crates/subtitles/src/ass/parser.rs:394-397
Code:
```
                    let names = match style_format.as_deref() {
                        Some(custom) => Names::Custom(custom),
                        None => Names::Default(DEFAULT_STYLE_FORMAT),
                    };
```
What is wrong: `DEFAULT_STYLE_FORMAT` (23-field ASS V4+) is the fallback for BOTH flavours, including `[V4 Styles]` (SSA v4), whose spec style format is a different 18-field set (TertiaryColour/AlphaLevel instead of OutlineColour/StrikeOut/ScaleX/Y/Spacing/Angle). A legacy SSA file without its own `Format:` line gets every `Style:` line `BadFieldCount`-skipped (18 < 23) — honest damage, wrong fallback.
Why it matters: Legacy SSA support (the row's "SSA" half) degrades to zero styles on Format-less files when a correct v4 fallback would parse them.
What correct looks like: A `DEFAULT_V4_STYLE_FORMAT` (18-field) selected when `flavour == V4Legacy`.
Confidence: MEDIUM — SSA v4 field list from format knowledge; damage path verified by read.

### F-006 [S4]
Where: crates/subtitles/src/ass/parser.rs:283-287
Code:
```
    /// A style by name.
    #[must_use]
    pub fn style(&self, name: &str) -> Option<&AssStyle> {
        self.styles.iter().find(|s| s.name == name)
    }
```
What is wrong: Duplicate style names (same file defining `Default` twice) resolve to the first definition, uncounted — no damage/stats entry distinguishes "redefined" from "unique". Combined with F-004 the wrong style is picked for every cue referencing the redefined name.
Why it matters: Silent wrong-style selection; the honest-degradation design counts far weaker events than this.
What correct looks like: Last-wins lookup or a `DuplicateStyle` damage entry on parse.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `StyleFlavour` (enum) | 68-74 | REAL | — | decides alignment convention; set per styles-section header (:368-375) |
| `AssStyle` (struct, 13 fields) | 80-109 | REAL | S3 | type-vocabulary mismatch vs render half (F-002); keeps `raw_alignment` for the styling card |
| `AssDamage` (enum, 8 variants) | 115-188 | REAL | S4 | each variant documents skip-vs-keep — accurate; but no variant for unrecognized record types (F-003) |
| `AssStats` (struct, 17 counters) | 193-240 | REAL | — | matches record's counter list incl. G-2 additions (applied_override_tags, move_tags_frozen, position_overrides_{applied,unmappable}) |
| `AssStats::count_tag` | 245-247 | REAL | — | pub(super), shared with ass::tags; BTreeMap entry |
| `AssDocument` (struct) | 252-264 | REAL | S4 | duplicates-kept doc misleading (F-004) |
| `AssDocument::info` | 269-275 | REAL | S4 | first-match (F-004) |
| `AssDocument::title` | 279-281 | REAL | — | trivial; used by fuzz harness |
| `AssDocument::style` | 285-287 | REAL | S4 | first-match, uncounted duplicates (F-006) |
| `Names` enum + `position`/`len` | 293-312 | REAL | — | zero-alloc accessor over default-vs-custom format |
| `DEFAULT_STYLE_FORMAT` | 315-339 | REAL | S4 | correct V4+ order; wrongly reused for V4Legacy fallback (F-005) |
| `DEFAULT_EVENT_FORMAT` | 342-344 | REAL | — | correct spec order |
| `parse` | 349-420 | REAL | S2 | infallible walk verified; production-dead (F-001) |
| `parse_format_line` | 423-428 | REAL | — | trims, drops empties; duplicate names → later processing wins silently (cosmetic) |
| `Section` (enum) | 431-437 | REAL | — | trivial |
| `section_name` | 440-444 | REAL | — | `[Name]`-shaped lines only; safe against Dialogue lines |
| `parse_info_line` | 447-452 | REAL | — | colon-less lines silently dropped (no damage; narrow, acceptable); key kept case-sensitive |
| `parse_style_line` | 456-549 | REAL | S4 | field mapping verified (§2); silent un-Style: skip (F-003); BadFieldCount only for too-FEW fields; Alignment parse → map_alignment (body in part2, A10) |

## 5. Test honesty
No tests in part1. The tests that exercise these functions live at :940-1469 (A01's part3) — from the earlier census (`plain_text` helper :933-939 is test-only, built over the tags engine) they feed handcrafted strings plus real ffmpeg-authored fixtures; all ran green in baseline (orchrun log :5281-5296 parser unit tests; :5542-5550 the ass_ffmpeg_ac differential). Part1 functions get mutation coverage only via part3 tests — cross-check with A01.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "[Script Info] key/value (BOM/CRLF/`;`-comment tolerant)" | docs/formats/sub-ass-ssa.md:21-22 | REAL | :353, :361-363 verified by read |
| "[V4+ Styles]/[V4 Styles] Format+Style — Name/Fontname/Fontsize (exact i64 thousandths…), PrimaryColour…, Bold/Italic/Underline (-1 convention), Alignment BOTH conventions…, MarginL/R/V" | docs/formats/sub-ass-ssa.md:22-28 | REAL (plumbing) | :494-546 binds every named field; colour/time/alignment BODIES are part2 (:751+, :785+, :825+) — A10 verifies |
| "[Events] Format+Dialogue in ANY field order (Text absorbs commas…)" | docs/formats/sub-ass-ssa.md:28-29 | REAL (plumbing, body in part2) | `Names::Custom` path :409-414; `parse_event_line` = A10's range |
| "margins still need PlayRes-aware math beyond this parser's scope" | parser.rs:25-28 (module doc) | REAL (honest) | matches :636 `Positioning::new(s.alignment, None, None)` — and contradicts layout.rs:7-9's "seam" claim (layout F-003) |
| "3/3 real-ffmpeg ASS docs exact" | docs/formats/sub-ass-ssa.md:60-67 | REAL | baseline orchrun log :5542-5550: `tests/ass_ffmpeg_ac.rs` 4/0 (ffmpeg basic/ladder/unicode/header fixtures) |
| matrix `subtitle-ass-ssa` partial ×3 | docs/feature-matrix.toml:913 | UNREACHABLE | F-001 census; tests green (orchrun :5540) but crate unshipped |

INDEX-drift note (cross-ref layout.rs F-002, corrected v2): the two INDEX.md-cited workspace runs fail-fast before the `subtitles` package; the full no-fail-fast evidence lives only in the unlisted `orchrun-test-workspace-nofailfast.log.txt`.

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (F-001). `raw_alignment`/`flavour`/`font_name`/margins are stored for the "Phase-5 styling card" — a consumer that does not exist yet (tracked, not dead by accident).
- `#[cfg(test)] use crate::model::cue::CueText;` (:56-57) — test-only import, correctly gated.

## 8. Dependencies & flags
- std BTreeMap only + crate-internal model/tags types. No feature flags, no unsafe, no panics (all parsing via `strip_prefix`/`split`/typed damage). Allocation per line (split_fields, to_owned) — O(input), bounded; matches the fuzz-harness bound claim (ass/fuzz.rs:22-23).

## 9. Open questions for the next agent
- A10 (part2): verify `parse_ass_colour`/`parse_time`/`map_alignment` bodies against the record's exactness claims ("0:01:02.75 → 62 750 000 µs pinned"); check `parse_event_line`'s Text-comma absorption and the `layer_values_ignored`/`margin_overrides_ignored` counters fire as documented.
- A01 (part3): confirm whether any test covers duplicate styles / case-drifted records — my finding F-003/F-006 predict no.
