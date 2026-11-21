# AUDIT: crates/subtitles/src/srt/tags.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 415 / 5 (+2 test helpers, 6 tests) |
| File verdict | UNREACHABLE (real, careful, in-crate test-proven code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `parse_markup` called only by `srt/parser.rs:249` (in-crate) and this file's tests; crate-level unreachability proof in ass/tags.rs audit §2 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n 'parse_markup|srt::tags' crates/subtitles` (single caller) · `sed -n` quote verification · baseline test citation (below) |

## 1. One-paragraph verdict
A genuinely well-built SRT markup walker: case-insensitive tag matching
without allocation, run splitting driven by style-equality (including the
set-and-unset-with-nothing-between collapse back to `Plain`), a damage
ledger for every documented degrade path, and byte-boundary-safe slicing
throughout (every anchor is an ASCII `<`, `>`, `&`, `;` or quote). The six
tests assert exact segment vectors and exact damage sequences — they are
mutation-sensitive. One documented invariant is violated by a one-token
input (`"<b>"` alone emits an EMPTY styled run, contradicting "empty runs
are not emitted"); three smaller drifts: `&#0;` decodes to a NUL instead of
HTML5's U+FFFD, entities longer than the 12-byte lookup window degrade
silently without the documented `UnknownEntity` damage, and `find_color`'s
`=`-before leniency plus the no-`=` silent no-op accept attribute shapes the
doc does not describe. As with the whole crate: nothing shipped links this
code — the CLI answers "the subtitle path is a Phase-5 card"
(apps/cli/src/dispatch.rs:198).

## 2. What actually works (verified)
- All 6 in-file tests pass in the baseline no-fail-fast workspace run:
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` shows
  `test srt::tags::tests::bold_italic_underline_map_to_segments … ok` …
  `style_returning_to_plain_still_splits … ok` inside the 267/267 subtitles
  lib result. (The primary baseline test log never ran crate tests —
  fail-fast aborted at apps/desktop; see ass/tags.rs audit §2.)
- Run-splitting traced by hand on paper for: `<b>Bold</b> and <i>italic</i>`,
  `a<font ..></font>b` (collapse to Plain), `a<i><b>c` (style change on
  empty current correctly drops the intermediate style), unclosed trailing
  tag (`a<b>` → Plain("a"), `a<b>c` → styled tail), `<i>a</i>` (no trailing
  empty run). All correct.
- Entity decoder guards: bare `&` (no alphanumeric/# follow) literal without
  damage; `;` search capped at 12 bytes; whitespace inside the body
  rejected; surrogates and out-of-range code points rejected via
  `char::from_u32` (None → damaged literal).
- `find_color`: `to_ascii_lowercase` is byte-position-preserving (true), the
  before/after delimiters prevent `mycolor`/`colors` false matches
  (traced), quotes and unquoted values both handled, unterminated quote
  lenient.
- Zero panics: all `find`/slice offsets land on ASCII anchors in a valid
  `&str`; no unwrap on input data (`nib`'s `unwrap_or(0)` is dead-by-
  precondition, all digits pre-validated as hex).

## 3. Findings

### F-001 [S3] Input `"<b>"` emits an EMPTY styled run — documented invariant violated
Where: crates/subtitles/src/srt/tags.rs:85-90 (invariant doc at 21-24)
Code (doc, lines 21-24):
```
//! Output: [`CueText::Plain`] when the text has no effective styling
//! left (the SRT majority — one allocation, D-1),
//! [`CueText::Styled`] otherwise, with runs split at style
//! boundaries; empty runs are not emitted.
```
Code (violating path):
```
    if !current.is_empty() || segments.is_empty() {
        segments.push(StyledSegment {
            text: current,
            style,
        });
    }
```
What is wrong: trace `parse_markup("<b>", …)`: the tag sets `style.bold`,
`current` stays empty (no push at line 51 because `current.is_empty()`),
loop ends with `current == ""`, `segments == []`, `style != PLAIN`. Line 82's
early return is skipped (style not PLAIN); line 85's condition
`!current.is_empty() || segments.is_empty()` evaluates `false || true` → an
`StyledSegment { text: "", style: bold }` is pushed. Result:
`CueText::Styled([("", bold)])` — an empty run, exactly what line 24
promises never happens.
Why it matters: the invariant is a documented contract consumers rely on
(no zero-length runs to filter); a cue whose text is only an opening tag
(nowhere forbidden by the SRT parser) produces a phantom segment. Blast
radius small (renders as nothing), but it is a real behavior/doc
contradiction with no test pinning it.
What correct looks like: `if !current.is_empty() { push }` — and if
`segments` would end up empty, return `CueText::Plain(current)`; add the
`"<b>"`-only test case.
Confidence: HIGH — Verification: read + hand-trace of the exact code above
(both branches quoted verbatim); no test covers this input.

### F-002 [S4] `&#0;` decodes to a NUL character instead of U+FFFD
Where: crates/subtitles/src/srt/tags.rs:275-281
Code:
```
            if let Some(num) = body.strip_prefix('#') {
                if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                }
                .and_then(char::from_u32)
```
What is wrong: `char::from_u32(0)` is `Some('\0')`, so `&#0;` and `&#x0;`
push a NUL into the cue text. HTML5 (the reference for this entity
grammar) defines `&#0;` as a parse error that emits U+FFFD (REPLACEMENT
CHARACTER).
Why it matters: a NUL in subtitle text is a landmine for downstream
C-flavored rendering paths and terminal/log printing; the doc sells entity
decoding without noting the NUL divergence.
What correct looks like: `num.parse::<u32>().ok().filter(|&c| c != 0).map(|_| '\u{FFFD}')`-style handling, or at minimum documenting the divergence.
Confidence: HIGH on the behavior (trace), MEDIUM on severity (depends on
downstream consumers, which today are in-crate only).

### F-003 [S4] Entities longer than the 12-byte window degrade WITHOUT the documented damage
Where: crates/subtitles/src/srt/tags.rs:247-256 (doc claim at 13-15)
Code:
```
    let semi_rel = tail
        .bytes()
        .enumerate()
        .take(12)
        .find(|&(_, b)| b == b';')
        .map(|(i, _)| i);
    let Some(semi_rel) = semi_rel else {
        // Bare `&`: literal, no damage (extremely common in SRT).
        return EntityDecode::Literal(1);
    };
```
Doc (lines 13-15): "the named entities … and numeric forms (`&#65;`,
`&#x41;`) decode; unknown entities stay literal +
[`SrtDamageKind::UnknownEntity`]".
What is wrong: a well-formed but unknown entity whose `;` sits beyond the
12-byte window (e.g. HTML5's `&ClockwiseContourIntegral;`, 26 bytes) falls
into the `None` arm: the `&` is kept literal with NO damage event, and the
remainder of the name flows into the text. The doc promises UnknownEntity
damage for unknown entities; long ones get silent treatment.
Why it matters: the damage ledger is this crate's honesty artifact (the
differential tests assert on it); an input class that silently skips the
ledger under-reports degradation. Blast radius: real-world SRT files
essentially never carry >12-byte entities.
What correct looks like: on window exhaustion, record
`SrtDamageKind::UnknownEntity` (the `&`-run is clearly not a supported
entity) before returning the literal.
Confidence: HIGH — Verification: read + trace; window size and doc quote
verbatim above.

### F-004 [S4] find_color leniencies beyond the documented grammar
Where: crates/subtitles/src/srt/tags.rs:150-158, 168-172
Code:
```
                let before_ok = at == 0
                    || lower_attrs[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_whitespace() || c == '=');
```
and
```
    v = v.trim_start();
    if !v.starts_with('=') {
        return Some(None);
    }
```
What is wrong: (a) `before_ok` accepts `=` before the match, so in
`<font x=color="#f00000">` the `color` after the bogus `x=` is accepted as
THE color attribute; (b) `<font color red>` (attribute name with no `=`)
returns `Some(None)` — "no color attribute" — silently, though the doc
enumerates only value-level failures for `BadStyleValue`. Both accept
malformed input more permissively than documented.
Why it matters: minor spec-fidelity drift and under-reported damage on
garbage attributes; no test pins either shape.
What correct looks like: `before_ok` restricted to whitespace (an attribute
name is preceded by whitespace or nothing); a present-but-valueless
`color` recorded as `BadStyleValue`.
Confidence: HIGH on behavior (trace), LOW impact.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_markup` | 33-98 | REAL | S3 (F-001) | Early-exit fast path for markup-free text (one alloc, D-1 honored); ASCII-anchored slicing; correct split/collapse logic in all traced cases except the empty-run escape hatch at 85. |
| `apply_tag` | 101-134 | REAL | — | Case-insensitive name match via `eq_ignore_ascii_case` (no alloc); `<>` silently dropped (documented); unknown non-empty names → UnknownTag; `/font` resets color only (bold/italic/underline intentionally NOT reset by `</font>` — correct: font close only closes the font). |
| `find_color` | 139-186 | REAL | S4 (F-004) | Position-preserving lowercase trick is sound; delimiter checks block `mycolor`/`colors`; quoted/unquoted values; leniencies noted. |
| `parse_hex_color` | 189-225 | REAL | — | `#rgb` expansion, `#rrggbb`, `#rrggbbaa`; all-hexdigit precheck; `nib`'s `unwrap_or(0)` unreachable (harmless). |
| `EntityDecode` (enum) | 228-233 | REAL | — | Clean two-state contract. |
| `decode_entity` | 236-294 | REAL | S4 (F-002, F-003) | Named set amp/lt/gt/quot/apos/nbsp + decimal/hex numeric; bare-`&` no-damage path correct and common-case; NUL and >12-byte drifts filed. |
| tests `run`/`seg` | 303-315 | REAL | — | Honest helpers, drain the sink properly. |
| 6 tests | 317-414 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| plain_text_stays_plain | YES | Exact Plain string + zero damage. |
| bold_italic_underline_map_to_segments | YES | Exact 4-segment and 2-segment vectors with interleaved plain runs. |
| font_color_maps_and_bad_values_degrade | YES | Exact green color; `color=red` → BadStyleValue vector; `#0f0` short form ok; face/size silent; set+unset collapse to Plain("ab"). |
| unknown_and_unterminated_tags | YES | `sub` twice-counted damage + Plain("a junk b"); `1 < 2` → UnterminatedTag + literal preserved. |
| entities_decode_or_stay_literal | YES | Decoded set incl. `&#65;&#x42;` → "AB"; `&bogus;` → UnknownEntity + literal; bare `&` far from `;` no damage. |
| style_returning_to_plain_still_splits | YES | Pins the trailing plain run (would fail if the walker dropped it). |
Gap: no test input consisting of a lone opening tag (`"<b>"`) — F-001's
violation is unpinned; no numeric-entity edge tests (`&#0;`, >12-byte).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Supported … `<b>`, `<i>`, `<u>` + closing tags (case-insensitive), and `<font color=...>`" | tags.rs:4-6 | REAL | apply_tag:107-128 dispatches exactly these, case-insensitively. |
| "unknown tags → stripped + UnknownTag; `<` with no `>` → kept literally + UnterminatedTag" | tags.rs:8-12 | REAL | Lines 129-131, 59-65; pinned by tests. |
| "named entities … and numeric forms decode; unknown entities stay literal + UnknownEntity" | tags.rs:13-15 | SHALLOW | True within the 12-byte window; long entities silently literal (F-003); `&#0;` decodes to NUL (F-002). |
| "other `<font>` attributes (`face`, `size`) are ignored silently" | tags.rs:18-19 | REAL | find_color only searches `color`; pinned by test. |
| "empty runs are not emitted" | tags.rs:24 | FALSE for one input | F-001: `"<b>"` → `Styled([("", bold)])`. |
| matrix row `subtitle-srt`: "incl. bold/italic/underline styling runs" (description, line 895) | docs/feature-matrix.toml:892-902 | REAL (code) / UNREACHABLE (product) | Code real + tested in-crate; no shipped binary reaches it (crate proof in ass/tags.rs audit §2). |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (crate-level proof, see
  ass/tags.rs audit §2).
- No dead functions; `EntityDecode` fully consumed. The only internal
  duplication is mild (three near-identical `TextStyle::new(...)` test
  vectors — fine).

## 8. Dependencies & flags
- No `[features]`, no cfg gates beyond `#[cfg(test)]`, std-only, no unsafe,
  no unwraps on input data. `#[must_use]` on `parse_markup` — good.

## 9. Open questions for the next agent
- srt/parser.rs owner: what happens upstream when a block's text is only
  `"<b>"` — does the empty-run segment (F-001) survive to the renderer, or
  does the parser drop empty-text cues earlier?
- model/cue.rs owner: does `CueText::Styled` with an empty-text segment
  render as a no-op (assumed in my severity call for F-001)?
- If an HTML5 conformance target is ever claimed for entities, `&#0;` and
  long-entity handling need decisions (F-002/F-003).
