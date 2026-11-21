# AUDIT: crates/subtitles/src/ass/parser.rs#part2 (lines 454-939)
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 454-939: `parse_style_line` through `plain_text` (all field/token parsers + the event-line walk + alignment map + colour/milli/time parsers). Lines 1-453 (module docs, types, `parse()` section walker) = part1 (W09-A09); lines 940-1469 (`#[cfg(test)] mod tests`, 20 tests) = part3 (W09-A01). Boundary chosen at function starts; part1/part3 owners should align to these edges. |
| Lines / functions | 486 / 12 (11 production fns + 1 cfg(test) helper) |
| File (chunk) verdict | REAL-UNPROVEN (in-crate test-proven; crate UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO (crate-level, see W09-A10 wave context). In-crate: every part2 function is called (directly or transitively) by `pub fn parse` (part1, re-exported ass/mod.rs:25); `parse_ass_colour`/`parse_decimal_milli` are `pub(super)` and also consumed by ass/tags.rs:59,242,288,343,385-401 (A02's file); tests: ass/parser.rs#part3 + tests/ass_ffmpeg_ac.rs (4/0 green in baseline) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `rg -n "fn [a-z0-9_]+\(" parser.rs` (function map); `rg -n "parser::\|AssDocument\|parse\(" src/ass/*.rs` (caller map); Read of full 1469-line file (part1+part2+part3 context); `sed -n` quote verification (7 ranges); `rg -n "DEFAULT_PLAY_RES" layout.rs` (:60 = (384,288)); `rg -n "impl Percent" -A 14 model/position.rs` (0..=100 gate); `rg -n "subtitle-ass-ssa" -A 11 docs/feature-matrix.toml`; `rg -n "Marked\|H:MM:SS\|colour\|384" docs/formats/sub-ass-ssa.md`; baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269 (267 unit green incl. this module's tests), :5542 (ass_ffmpeg_ac 4/0) |

## 1. One-paragraph verdict
The core field/token machinery of the ASS parser is real, exact-integer (no floats
anywhere), overflow-checked on every arithmetic path (`parse_time`,
`parse_decimal_milli`, `apply_position_override` all use checked ops), and
panic-free (every index is behind a `fields.len() < format.len()` gate or a
`.get()`). The honest-degradation claims that docs/formats/sub-ass-ssa.md makes
for this range are TRUE in the code: thousandths Fontsize, all four colour
literal forms with ASS alpha inversion, both alignment conventions (numpad 1-9
and legacy 1-11, bare 4/8 rejected), 1-6-digit time fractions with lenient
M/S>59, comma absorption only for a final Text field — each pinned by part3
tests that assert exact values. The defects are an SSA-v4 gap — the default
event format is the ASS `Layer`-first order and the v4 `Marked` field is not
handled, so a Format-less SSA v4 file produces one false `BadField{Layer}`
damage event per dialogue line — plus four hygiene-level drifts (duplicated
PlayRes default literals with first-wins duplicate-key semantics, a
half-percent boundary clamp that contradicts the "never clamped" comment,
positive colour overflow silently truncating via `as u32`, unknown line types
vanishing without any counter). Nothing here is reachable from a shipped
binary.

## 2. What actually works (verified)
- Baseline executed the module's tests green:
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5271
  (subtitles unittests "267 passed; 0 failed") and :5542 (tests/ass_ffmpeg_ac.rs
  "4 passed; 0 failed" — real-ffmpeg ASS documents, times and text exact).
- Part3 tests pin part2 functions with exact values (would fail under mutation):
  `time_parsing_edges` (16 cases incl. `0:01:02.75 → 62_750_000`, i64::MAX
  overflow → None, 7-digit fraction → None); `colour_forms` (14 cases incl.
  `-1` wrap → transparent white, 9-hex-digit → None); `alignment_conventions`
  (both conventions exhaustively incl. legacy 4/8 → None, 12 → None);
  `decimal_milli_edges` (9 cases incl. `.5` → 500, `1.2345` → None);
  `v4_legacy_section_uses_legacy_alignment` (11 → Middle/Right end-to-end);
  `custom_event_format_orders_fields` (Text FIRST, non-final → single field);
  `fuzz_lite_survives_junk_and_mutations` (2000 runs, asserts dense ids, valid
  intervals, damage line numbers in range).
- Hand-verified math: `parse_ass_colour("&H0000FF")` → value 0x0000FF → r=255,
  g=0, b=0, alpha 255 (test asserts red) — BGR ordering correct;
  `parse_decimal_milli("20.5")` → 20·1000 + 5·100 = 20 500 ✓;
  `parse_time("0:99:00.00")` → 99·60·10⁶ = 5_940_000_000 µs (linear M/S) ✓.
- `Percent::new` gate confirmed at model/position.rs:41-47 (0..=100) — the
  "unreachable" arm in `apply_position_override` (:735-736) is indeed
  unreachable after the `filter`.

## 3. Findings
### F-001 [S3]
Where: crates/subtitles/src/ass/parser.rs:341-344 (part1 const, cross-cited) consumed at :553-687 (`parse_event_line`, part2)
Code:
```
/// Default `[Events]` field order (the spec's).
const DEFAULT_EVENT_FORMAT: &[&str] = &[
    "Layer", "Start", "End", "Style", "Name", "MarginL", "MarginR", "MarginV", "Effect", "Text",
];
```
What is wrong: "the spec's" is only the **ASS (V4+)** default. The SSA V4 spec's
default event order is `Marked, Start, End, Style, Name, MarginL, MarginR,
MarginV, Effect, Text` — `Marked=0` occupies the slot this parser reads as
`Layer`. `parse_event_line` receives no `flavour` (the style section tracks
V4Legacy vs V4Plus at :368-375, the events section does not), and no code path
recognizes `Marked`. For an SSA v4 file that omits its `Format:` line, every
`Dialogue: Marked=0,…` line runs `"Marked=0".parse::<i32>()` → fails → one
`AssDamage::BadField { field: "Layer" }` per dialogue line, and the cue
survives only by positional coincidence (Marked and Layer share slot 0). The
damage ledger — the crate's core honesty artifact — systematically reports
false damage on valid legacy input. (With a v4 `Format:` line that declares
`Marked`, `get("Layer")` is `None` → "0" → no damage; `Marked` itself is
silently ignored without any counter.) What correct looks like: thread the
flavour into `parse_event_line`, default to `Marked`-first for `V4Legacy`, and
accept/ignore-count `Marked=0`. Why it matters: SSA v4 is half of the
"ASS/SSA" claim in the matrix row. Confidence: HIGH on the code behavior
(read + traced), MEDIUM on the SSA default-format claim (recalled from the SSA
v4 spec; no spec copy in tree — UNVERIFIED offline). Severity note: every
in-tree fixture declares a `Format:` line, so the suite cannot see this.

### F-002 [S4]
Where: crates/subtitles/src/ass/parser.rs:689-700
Code:
```
/// The effective script resolution: `PlayResX`/`PlayResY` when
/// present and nonzero, else the 384×288 VSFilter-heritage default
/// (`layout::DEFAULT_PLAY_RES`, the libass behaviour).
fn play_res(doc: &AssDocument) -> (i64, i64) {
    let read = |key: &str, default: i64| {
        doc.info(key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(default)
    };
    (read("PlayResX", 384), read("PlayResY", 288))
}
```
What is wrong: Two drifts. (1) The doc points at `layout::DEFAULT_PLAY_RES`
(= `(384, 288)`, layout.rs:60) but the code re-hard-codes `384`/`288` — the
constant and the literals can diverge silently. (2) `doc.info()` (part1:270-275)
returns the FIRST occurrence of a key, while the `script_info` field doc
(part1:253-254) says "duplicates kept — the later key usually wins in
consumers"; for a file with two `PlayResX` lines this parser uses the first,
libass the last. What correct looks like: `read("PlayResX",
i64::from(layout::DEFAULT_PLAY_RES.0))` and a documented first-vs-last choice.
Confidence: HIGH — read; layout.rs:60 verified.

### F-003 [S4]
Where: crates/subtitles/src/ass/parser.rs:715-724 (promise at :702-708)
Code:
```
    // percent = milli / (extent × 1000) × 100, round-half-up.
    let to_percent = |milli: i64, extent: i64| -> Option<i32> {
        let num = milli.checked_mul(100)?;
        let den = extent.checked_mul(1000)?;
        if den == 0 {
            return None;
        }
        let q = (num + den / 2) / den;
        i32::try_from(q).ok().filter(|v| (0..=100).contains(v))
    };
```
What is wrong: The doc above it (:705-708) promises "Coordinates outside
`0..=100 %` … never clamped (a clamp would move the cue silently)". The
round-half-up makes that false inside a half-percent band: any x in
`(-extent/200 px, 0)` rounds to q=0 and is APPLIED at 0% (e.g. `\pos(-1000,…)`
on PlayResX 384: num=-100 000, q=(−100 000+192 000)/384 000=0 → 0%), and any
x within `+extent/200 px` past 100% rounds to q=100 and is applied at 100%.
Small silent clamps — exactly what the comment says cannot happen. What
correct looks like: pre-check `milli` against `[0, extent·1000]` before
rounding (reject the band), or fix the comment. Confidence: HIGH — arithmetic
recomputed by hand.

### F-004 [S4]
Where: crates/subtitles/src/ass/parser.rs:884-886
Code:
```
    } else if let Ok(dec) = raw.parse::<i64>() {
        u32::try_from(dec).unwrap_or(dec as u32)
    } else {
```
What is wrong: For decimal literals above `u32::MAX` (e.g. `4294967296`),
`try_from` fails and `dec as u32` silently truncates to the low 32 bits
(2³² → 0 → opaque black). The doc comment (:869-871) justifies the wrap only
for NEGATIVE values ("negative wraps as the two's-complement 32-bit value");
positive overflow wraps undocumented — the classic `as` truncation the
audit checklist targets. What correct looks like: reject > u32::MAX (None +
`BadColour`) or document the full wrap. Confidence: HIGH — read; semantics of
`as` truncation are language-defined.

### F-005 [S4]
Where: crates/subtitles/src/ass/parser.rs:463-465 and :568-570
Code:
```
    let Some(rest) = line.strip_prefix("Style:") else {
        return; // unknown line type in a styles section: skipped
    };
```
and
```
    let Some(rest) = line.strip_prefix("Dialogue:") else {
        return; // unrecognized line type in Events: skipped
    };
```
What is wrong: Unknown line types inside recognized sections vanish with no
`AssStats` counter and no damage event — inconsistent with the ledger
philosophy that counts skipped SECTIONS (`skipped_sections`), non-text events,
ignored Layers/margins/Names/Effects. A `[Events]` line of `Garbage: …` is
indistinguishable (in the output) from a line that never existed. What correct
looks like: an `unknown_lines: u64` (or per-section counter). Confidence:
HIGH — read; no counter exists anywhere in `AssStats` (part1:193-240).

## 4. Per-function table (part2: lines 454-939)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_style_line` | 456-549 | REAL | S4 (F-005) | field-count gate before any index; unknown fields `_ => {}` (documented deferral); fabricated `raw_alignment: 2` default when the Format lacks Alignment |
| `parse_event_line` | 553-687 | REAL | S3 (F-001) | event-type prefixes counted; Text comma-absorption only when final; unknown style → damage + default positioning; `CueId` dense 1-based; O(styles) lookup per cue (fine at file scale) |
| `play_res` | 692-700 | REAL | S4 (F-002) | first-wins duplicates; duplicated literals |
| `apply_position_override` | 709-741 | REAL | S4 (F-003) | checked math; den==0 guarded; negative/half-band clamp vs "never clamped" doc |
| `split_fields` | 745-747 | REAL | — | trims (style fields only) |
| `parse_bool_field` | 751-764 | REAL | — | accepts `-1`/`1`/`0`; doc says "-1/0" (superset, harmless) |
| `parse_int_field` | 767-779 | REAL | — | i32 or damage+0 |
| `parse_time` | 785-816 | REAL | — | exact u64 math, checked at every step; 1-6 digit fractions; linear M/S; i64::MAX gate; single-dot split (double dot → frac non-digit → None) |
| `map_alignment` | 825-863 | REAL | — | numpad 1-9 ✓; legacy 1-11 with bare 4/8 rejected ✓ (`v & 3 == 0`); exhaustive |
| `parse_ass_colour` | 873-894 | REAL | S4 (F-004) | `&H`/`&h`, trailing `&`, ≤8 hex digits, decimal ±, BGR + inverted alpha — all correct; positive-overflow wrap |
| `parse_decimal_milli` | 898-928 | REAL | — | sign, 0-3 fraction digits, checked mul/add; `.5` → 500; rejects `1.2345` |
| `plain_text` (cfg(test)) | 933-939 | REAL | — | test-only wrapper over tags engine |

## 5. Test honesty (tests live in part3 — external evidence for this range)
| Test (part3) | Would it fail if part2 code were broken? | Reason |
|---|---|---|
| time_parsing_edges | YES | 16 exact Some/None values incl. overflow |
| colour_forms | YES | 14 exact Color values incl. -1 wrap, 9-digit None |
| alignment_conventions | YES | every numpad + legacy cell, invalid inputs |
| v4_legacy_section_uses_legacy_alignment | YES | end-to-end legacy 11 → Middle/Right |
| custom_event_format_orders_fields | YES | Text-first format parsed positionally |
| text_commas_absorbed_into_text_field | YES | "One, two, three" exact |
| malformed_times_skip_the_cue_with_typed_damage | YES | exact damage vector (3 events, line numbers) |
| zero_duration_cue_is_legal | YES | end==start kept, no damage |
| tolerated_field_damage_and_counters | YES | per-field damage kinds + all counters |
| style_line_damage_paths | YES | BadFieldCount/BadField/BadColour/BadAlignment path each hit |
| decimal_milli_edges | YES | 9 exact values |
| non_text_events_counted_not_damaged | YES | 2 counters, empty damage |
| fuzz_lite_survives_junk_and_mutations | YES | 2000 runs, structural invariants asserted |
Deep test-body audit belongs to part3 (W09-A01); listed here only as execution
evidence for part2 functions (baseline green, cite in header).

## 6. Doc claims touching this chunk
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Fontsize (exact i64 thousandths, no floats)" | docs/formats/sub-ass-ssa.md:26 | REAL | parse_decimal_milli:898-928, pinned by decimal_milli_edges |
| "PrimaryColour (&HAABBGGRR/short-hex/decimal/negative-wrap → straight RGBA8, ASS alpha inverted)" | sub-ass-ssa.md:26-28 | REAL | parse_ass_colour:873-894; colour_forms pins all forms (positive-overflow wrap is an undocumented superset, F-004) |
| "Alignment BOTH conventions (ASS numpad 1–9; legacy SSA 1–11)" | sub-ass-ssa.md:29 | REAL | map_alignment:825-863 + alignment_conventions |
| "Times: H:MM:SS.CC → µs exact; fractions 1–6 digits; lenient M/S>59; overflow/garbage → typed BadTime skip" | sub-ass-ssa.md:32-34 | REAL | parse_time:785-816 + time_parsing_edges |
| "Text absorbs commas only as last declared field" | sub-ass-ssa.md:30-31 | REAL | :588-592 + custom_event_format_orders_fields |
| matrix row `subtitle-ass-ssa` status `partial` (win/linux/macos) | docs/feature-matrix.toml:913 | REAL-as-tree-code, UNREACHABLE-in-product | nothing shipped links the crate (wave context); "partial" is the honest in-tree status |
| "Coordinates outside 0..=100 % are unmappable … never clamped" | parser.rs:705-708 (in-chunk doc) | FALSE (boundary band) | F-003 math |
| "(the spec's)" on DEFAULT_EVENT_FORMAT | parser.rs:341 (part1, cross-cited) | FALSE-for-SSA-v4 | F-001 |

## 7. Dead / duplicated / unreachable code
No dead functions in part2 (all called by `parse` or tags.rs). Duplicated: the
384/288 PlayRes default vs `layout::DEFAULT_PLAY_RES` (F-002). Crate-level
unreachability: see header.

## 8. Dependencies & flags
Uses crate::ass::tags (A02), crate::model::{cue, position, style, time} — all
in-crate. No external deps, no unsafe, no feature gates, no cfg beyond
`#[cfg(test)]` on plain_text. Workspace lints (deny unwrap/expect/panic)
satisfied — baseline clippy CLEAN (#5/#6).

## 9. Open questions for the next agent
- part3 (W09-A01): does any test exercise a `[Events]` section with NO
  `Format:` line at all (the F-001 path)? My read says no — every fixture
  declares one.
- part1 (W09-A09): `parse()` resets both formats on EVERY section header
  (:382-384) — confirm no test covers a `[Events]` Format surviving a
  same-section re-entry (it cannot) and check the `Names`-borrow code path.
- tags.rs (W09-A02): `PositionOverride` milli-units are assumed positive-free
  (my F-003 math assumes negative milli reaches `to_percent` — confirm tags.rs
  can emit negative x/y from `\pos(-1000,100)`).
- Whoever holds SSA v4 spec access: confirm the default `Marked,` order (F-001)
  and whether libass keeps the LAST duplicate Script Info key (F-002).
