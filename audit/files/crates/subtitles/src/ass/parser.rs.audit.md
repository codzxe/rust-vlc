# AUDIT: crates/subtitles/src/ass/parser.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 1469 / 21 (+17 tests) |
| File verdict | UNREACHABLE (code quality: REAL — well-built, well-tested, but no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `subtitles` crate is a dependency of NO shipped app/crate; only `fuzz/Cargo.toml:50` depends on it. `apps/cli/src/dispatch.rs:197-198` refuses `subtitle.*` commands ("the subtitle path is a Phase-5 card"); `apps/cli/src/containers/mod.rs:50-76` (container registry) has no srt/ass/vtt arm (default arm = `unwired_refusal`); MKV subtitle tracks are minted but DESELECTED at open (`apps/cli/src/containers/mkv.rs:531-541` test `multitrack_fixture_deselects_subtitles_at_open`: "the subtitle kind deselects"); `playback-engine/src/pipeline.rs:1036-1038` "A provider that cannot build a subtitle decoder deselects that track". |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |
| Commands run | rg (reachability, exit 0); sed -n quotes (exit 0); rustc fresh rlib build of media-types+subtitles (exit 0); EXECUTED overflow probe -> panic exit 101 (parser.rs:722); baseline cite: audit/runs/baseline/cargo-test-workspace.txt:5735-5775 (all ass::parser tests ok) |

## 1. One-paragraph verdict
The parser itself is the real thing: a tolerant, infallible, typed-damage ASS/SSA core that parses sections, both style flavours (numpad + SSA legacy 1–11), any-order event formats, exact integer time/decimal/colour arithmetic with checked ops everywhere — except one unchecked `i64` add in `apply_position_override` that overflows on crafted `PlayResX` + `\pos` values (F-001). The doc ledger is *mostly* honest about the subset (G-2 tags verified against `tags.rs`), but two claims are contradicted by the code: style-line secondary/outline/shadow colours are NOT "parsed and counted" (silently skipped, F-002), and negative `\pos` coordinates ARE silently clamped to 0% ("never clamped" is false, F-003). The damning fact is shipping: nothing in any shipped binary calls this code — the whole `subtitles` crate is fuzz-only, so 1469 lines of quality parser are dead weight in the product today.

## 2. What actually works (verified)
- Baseline run `cargo test` (audit/runs/baseline/cargo-test-workspace.txt:5735+, PACKAGE subtitles, 267 tests): all 17 `ass::parser::tests` pass incl. `fuzz_lite_survives_junk_and_mutations` (2000 LCG runs, junk+mutation+truncation, invariants: dense ids, valid intervals, damage line numbers in range).
- Reachability scan (rg, exhaustive): `ass::parse` called only from `crates/subtitles/tests/*` and `crates/subtitles/src/ass/fuzz.rs` — no app, no demux route, no `HostDecoderProvider` arm.
- G-2 minimum verified in `ass/tags.rs:216-315` (`apply_tag`): `\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move` applied; `\2c\3c\4c\2a\3a\4a` validated→`unmapped_colour_tags`; 52-name `KNOWN_TAGS` table (longest-first) drops the rest by name (`\an`, `\r`, `\k/\kf/\ko/\K`, `\t`, `\fad/\fade`, `\frx/\fry/\frz`, `\p` drawing, `\clip/\iclip`, `\be/\blur`, `\bord/\shad`, `\fax/\fay`, `\org`, `\rnd`, `\pbo`, `\fe`, `\fsp`, `\fscx/\fscy`, `\q`, legacy `\a`) — matches the parser doc's ledger for TAGS.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/ass/parser.rs:715-724
Code:
```
    let to_percent = |milli: i64, extent: i64| -> Option<i32> {
        let num = milli.checked_mul(100)?;
        let den = extent.checked_mul(1000)?;
        if den == 0 {
            return None;
        }
        let q = (num + den / 2) / den;
```
What is wrong: `num + den / 2` is an UNCHECKED i64 add on values derived from parsed input (`\pos` x via `parse_decimal_milli`, `PlayResX` via `play_res`, both unbounded up to ~i64::MAX/100000 and i64::MAX/1000). Concrete overflow (EXECUTED): `PlayResX: 9223372036854775` + `\pos(92233720368547,10)` panics. Why it matters: crate lib.rs:70-72 promises, verbatim, "no unsafe (workspace lint: `deny`), no panics on any input — failure is / typed ([`model::ModelError`]/[`model::WireError`]), per MP §8"; debug builds (and the release lane of `fuzz/Cargo.toml`, which sets `overflow-checks = true`) PANIC here, release wraps to a garbage quotient. It is a direct violation of the crate's own headline contract, triggerable by a crafted subtitle file the moment any entry point is wired. What correct looks like: `num.checked_add(den / 2)?` — one fix preserving the round-half-up semantics. Confidence: HIGH — Verification: EXECUTED — probe (audit/runs/w09a01_probe.rs, rustc-built against freshly built rlibs) output, exit 101:
```
thread 'main' (10528) panicked at crates/subtitles/src/ass/parser.rs:722:17:
attempt to add with overflow
```

F-002 [S3]
Where: crates/subtitles/src/ass/parser.rs:15-20 (doc) vs 542-545 (code)
Code (doc):
```
//! [`AssStats::dropped_tags`]; the secondary/outline/shadow colour
//! slots are parsed and counted into [`AssStats::unmapped_colour_tags`]
//! (the model's single-colour subset). The G-2 minimum — `\b \i \u
```
Code (impl):
```
            // SecondaryColour/OutlineColour/BackColour, ScaleX/Y,
            // Spacing, Angle, BorderStyle, Outline, Shadow, Encoding:
            // full styling — deferred (§2.2).
            _ => {}
```
What is wrong: for STYLE lines, `SecondaryColour`/`OutlineColour`/`BackColour` fall into the `_ => {}` arm — neither parsed nor counted anywhere; `unmapped_colour_tags` is incremented only by the `\2c`-family TAGS (tags.rs:285-297). The doc's "colour slots are parsed and counted" is true only for inline tags, false for the style fields the sentence's context (styles paragraph) implies. Silent, uncounted information loss contradicts the file's "honest ledger" premise. What correct looks like: count these fields into a stat (or fix the doc to say tags-only). Confidence: HIGH — Verification: read + rg `unmapped_colour_tags` (only tags.rs writes it).

F-003 [S3]
Where: crates/subtitles/src/ass/parser.rs:702-708 (doc) vs 722-723 (code)
Code:
```
/// resolution (round-half-up — the model's lossy percent vocabulary,
/// `model::position` docs). Coordinates outside `0..=100 %` are
/// unmappable in that vocabulary: counted into
/// [`AssStats::position_overrides_unmappable`], never clamped (a
```
What is wrong: two doc violations in `to_percent`. (1) "round-half-up" — Rust `/` truncates toward zero, so for negative `num` this is round-half-toward-zero. (2) "never clamped" — a negative coordinate inside the window x ∈ (-3·extent/200, 0) rounds to q=0 which PASSES the `(0..=100)` filter, so the cue is silently placed at 0% (left/top edge) and counted as APPLIED. What correct looks like: floor division for the half-up rounding, and treat q from a negative milli as unmappable. Confidence: HIGH — Verification: EXECUTED — probe (audit/runs/w09a01_probe_neg.rs, fresh rlibs), `\pos(-5,10)` vs `\pos(-6,10)` with PlayResX 384:
```
pos=Some(0) line=Some(Percent(Percent(3)))
pos=None line=None
applied=1 unmappable=1
```

F-004 [S3]
Where: crates/subtitles/src/ass/parser.rs:314-339 + 394-397
Code:
```
    let names = match style_format.as_deref() {
        Some(custom) => Names::Custom(custom),
        None => Names::Default(DEFAULT_STYLE_FORMAT),
    };
```
What is wrong: a `[V4 Styles]` (SSA v4) section WITHOUT its own `Format:` line is validated against the 23-field V4+ default (`DEFAULT_STYLE_FORMAT`). SSA v4 style lines carry 17 fields (Name..AlphaLevel,Encoding) — every style in such a file hits `BadFieldCount{expected:23, seen:17}` and is SKIPPED, so all its cues degrade to UnknownStyle + default positioning. A `[V4 Styles]` section needs its own 17-field default (libass keys the default format on the script/section flavour). Files affected: SSA v4 without Format lines. Confidence: MEDIUM (libass default-table behavior not executed — UNVERIFIED: no libass in sandbox; the 17-field SSA v4 style layout is the published spec's). Verification: read.

F-005 [S4]
Where: crates/subtitles/src/ass/parser.rs:635-636
Code:
```
    let (found_style, positioning) = match doc.style(&style_name).cloned() {
        Some(s) => (Some(s.clone()), Positioning::new(s.alignment, None, None)),
```
What is wrong: double clone of `AssStyle` (7 owned Strings each) per dialogue line — `.cloned()` then `s.clone()`; the second could be a move. Needless per-event allocation (checklist 6.3); harmless at subtitle-file scale. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/subtitles/src/ass/parser.rs:692-700
Code:
```
    (read("PlayResX", 384), read("PlayResY", 288))
```
What is wrong: `PlayResX`/`PlayResY` lookup is case-sensitive via `AssDocument::info` (L268: "case-sensitive, as written in the file"); a file writing `Playresx:` silently falls back to 384×288, skewing every `\pos` percent. libass matches these keys case-insensitively (UNVERIFIED — no libass source in sandbox). Blast radius: nonstandard casing only. Confidence: MEDIUM — Verification: read.

F-007 [S4]
Where: crates/subtitles/src/ass/parser.rs:12-13 (doc) vs 588-592 (code)
Code:
```
    let joined = if text_idx + 1 == format.len() {
        fields[text_idx..].join(",")
    } else {
        fields[text_idx].to_owned()
    };
```
What is wrong: the header doc says "the `Text` field absorbs commas" without the when-last qualifier; the code (correctly) absorbs only when Text is the final declared field — the constraint lives in a code comment (L585-587), not the ledger. Doc imprecision only. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse` | 349-420 | REAL | — | infallible walk; BOM+CRLF; section-scoped format reset ✓ |
| `parse_format_line` | 423-428 | REAL | — | trims, drops empties |
| `section_name` | 440-444 | REAL | — | `[Name]` only |
| `parse_info_line` | 447-452 | REAL | — | split_once ':' |
| `parse_style_line` | 456-549 | REAL | F-002 | subset projection; BadFieldCount skip; defaults honest |
| `parse_event_line` | 553-687 | REAL | F-005 | comma absorption last-field-only ✓; times can skip cue ✓; unknown-style degrade ✓ |
| `play_res` | 692-700 | REAL | F-006 | >0 filter, 384×288 default |
| `apply_position_override` | 709-741 | REAL (buggy edge) | F-001, F-003 | unchecked add; negative-clamp |
| `split_fields` | 745-747 | REAL | — | |
| `parse_bool_field` | 751-764 | REAL | — | -1/1/0, damage otherwise |
| `parse_int_field` | 767-779 | REAL | — | damage+0 |
| `parse_time` | 785-816 | REAL | — | all-checked u64 math; lenient M/S; 1-6 digit frac ✓ doc |
| `map_alignment` | 825-863 | REAL | — | both flavours correct (tests pin all 9 numpad + legacy edges) |
| `parse_ass_colour` | 873-894 | REAL | — | &H/decimal/BGR/inverted-alpha; wrap for negatives = 32-bit field convention |
| `parse_decimal_milli` | 898-928 | REAL | — | no floats; checked; 0-3 frac digits |
| `AssDocument::{info,title,style}` | 266-288 | REAL | — | linear `style` scan per event (O(events×styles), fine at file scale) |
| `AssStats::count_tag` | 242-248 | REAL | — | pub(super), shared with tags.rs |
| `Names::{position,len}` | 298-312 | REAL | — | zero-alloc accessor |
| `plain_text` (cfg(test)) | 933-939 | REAL | — | test wrapper |
| `StyleFlavour`/`Section`/`Names` enums, consts | 65-344 | REAL | — | |
| 17 tests incl. `fuzz_lite` | 941-1468 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_script_info_and_style | YES | asserts exact font/colour/margins/alignment + empty damage |
| parses_dialogue_times_and_text | YES | pins µs times, text, hard-newline fold, stats counters |
| time_parsing_edges | YES | 14 cases incl. overflow `9999999999999:...` → None |
| colour_forms | YES | BGR/alpha/trailing-&/decimal/-1-wrap/9-hex-digit reject |
| alignment_conventions | YES | all 9 numpad + legacy 1..11 + invalid 0/10/4/8/12 |
| v4_legacy_section_uses_legacy_alignment | YES | end-to-end SSA flavour + inheritance |
| custom_event_format_orders_fields / text_commas_absorbed | YES | reorders Text to front; commas absorbed only-last behavior pinned |
| text_rules_override_blocks_and_escapes | YES | `{}` quirk, unterminated `{`, `\N\n\h`, karaoke dropped+counted, UTF-8 scalars |
| malformed_times_skip_the_cue_with_typed_damage | YES | exact damage vector (line numbers, variants) |
| zero_duration_cue_is_legal | YES | end==start allowed |
| tolerated_field_damage_and_counters | YES | BadField Layer/MarginL, counters, styled run shape |
| unknown_sections_and_bom_and_crlf | YES | BOM+CRLF+skipped sections |
| style_line_damage_paths | YES | field-count skip + kept-with-damage |
| decimal_milli_edges / non_text_events_counted_not_damaged | YES | edges; Picture/Command counted not damaged |
| fuzz_lite_survives_junk_and_mutations | PARTLY | invariants real (ids, intervals, line ranges, <200KB) but the alphabet/mutation space cannot reach F-001's magnitude (needs 16-digit PlayResX + 15-digit \pos) — verified by reasoning, not mutation |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| G-2 tags `\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move` interpreted | parser.rs:20-23 | REAL | tags.rs:216-315 `apply_tag` arms + tests pass (baseline:5735+) |
| Secondary/outline/shadow "colour slots ... parsed and counted" | parser.rs:18-20 | FALSE (for style fields) | F-002: `_ => {}` at 542-545; only tags counted |
| Karaoke/rotation/animation/fades/drawing "stripped, counted per tag name" | parser.rs:15-17 | REAL | tags.rs KNOWN_TAGS + dropped_tags; test `s2.dropped_tags.get("k") == Some(&2)` |
| `\pos`/`\move` → integer percents, unmappable counted "never clamped" | parser.rs:702-708 | FALSE (negative window) | F-003 |
| Times: lenient M/S, 1-6 digit fractions, i64::MAX cap | parser.rs:30-39 | REAL | parse_time + time_parsing_edges |
| `\N`/`\n`→`\n`, `\h`→space, other `\x` literal, `{}` preserved, unterminated `{` literal | parser.rs:41-51 | REAL | tags.rs:430-470 + text_rules test |
| "ANY field order (the Text field absorbs commas)" | parser.rs:12-13 | SHALLOW (doc) | F-007: only when Text last |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped entry points (see header): only fuzz + in-crate tests call it. No internal dead code found (all helpers called; `plain_text` is cfg(test)).

## 8. Dependencies & flags
No new deps (std only + crate-internal model/tags). No feature gates inside this file. Crate-level: `subtitles` is not a dependency of lumen-cli/desktop/web — only `fuzz/Cargo.toml:50`.

## 9. Open questions for the next agent
- tags.rs (ass/tags.rs) has its own audit owner? The `\fn` empty-reset → `font_name: None` diverges a run from the seed (Styled emission) — confirm the renderer interprets None as inherit.
- Whoever audits `ass/layout.rs` + `render.rs`: confirm margins (parsed here, `margin_l/r/v`) are actually consumed downstream, else they're parsed-dead.
