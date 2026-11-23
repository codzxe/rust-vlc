# AUDIT: crates/subtitles/src/srt/tags.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 415 / 13 (6 prod fns + 1 enum, 6 tests + 2 helpers) |
| File verdict | REAL (library) / UNREACHABLE (product) — solid honest-degrade markup parser with real bugs at the edges |
| Reachable from shipped entry point? | NO — parse_markup <- srt/parser.rs:249 <- `subtitles` crate root <- no app depends on it (only fuzz/Cargo.toml:50) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | prebuilt libtest `./target/debug/deps/subtitles-70f89de07410626f srt::tags` -> 6 passed / 0 failed (exit 0); rg callers + 8-digit-hex coverage + strikeout-in-srt searches (exit 0); sed quote verification |

## 1. One-paragraph verdict
A working, infallible, honestly-degrading SRT markup parser: `<b>/<i>/<u>` (case-insensitive, with closers) and `<font color=#rgb|#rrggbb|#rrggbbaa>` map to styled runs; unknown tags, unterminated `<`, and bad colors are counted into typed `SrtDamageKind`s, never fatal; entity decoding (`&amp;` family + `&#65;`/`&#x42;`) is correct with sane bare-`&` handling ("fish & chips" stays literal, "AT&T &bogus;" splits correctly). Tests are exact-vector and pass 6/6. Real defects: `</font>` clears color instead of restoring the enclosing `<font>` (no stack), `find_color` can anchor on the literal word "color" inside a quoted attribute value and then silently drop the real color attribute, `<s>` strikeout is unsupported even though the model has the field and the ASS engine honors `\s`, and the documented 8-digit `#rrggbbaa` form has zero test coverage anywhere in-tree. And the standing domain fact: none of it is reachable from any shipped binary.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f srt::tags` -> `test result: ok. 6 passed; 0 failed` (exit 0): plain fast-path, b/i/u run splitting with exact 4-segment vectors, `#00ff00`/`#0f0` colors, `color=red` -> BadStyleValue, face/size ignored no-damage, set-then-unset collapse to Plain, `<sub>` x2 -> UnknownTag, `1 < 2` -> UnterminatedTag + literal, entity set incl. `&#65;&#x42;`, `&nbsp;` -> U+00A0, bare `&` no damage.

## 3. Findings
F-001 [S2]
Where: this file + apps/cli/Cargo.toml (absence); fuzz/Cargo.toml:50
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50; no such row in apps/cli/Cargo.toml)
What is wrong: same domain blocker as tags.rs F-001 — the SRT parser chain is wired to nothing shipped; matrix row subtitle-srt claims `status = { windows = "implemented", linux = "implemented", macos = "implemented" }` (feature-matrix.toml:899) for unreachable code. What correct looks like: dependency + sidecar-load path from playback. Confidence: HIGH — Verification: ran (rg).
F-002 [S3]
Where: crates/subtitles/src/srt/tags.rs:119-120
Code:
```
    } else if name.eq_ignore_ascii_case("/font") {
        style.color = None;
    } else if name.eq_ignore_ascii_case("font") {
```
What is wrong: no tag stack. `<font color="#ff0000">a<font color="#00ff00">b</font>c</font>` renders "c" with NO color instead of red — the inner close wipes the outer setting. Real HTML/SRT semantics restore the enclosing scope. Rare input, wrong output. What correct looks like: a small Vec<Color> stack or a style-snapshot stack. Confidence: HIGH — Verification: read (traced); not covered by any test (nested fonts untested).
F-003 [S3]
Where: crates/subtitles/src/srt/tags.rs:143-172
Code:
```
    let lower_attrs = attrs.to_ascii_lowercase();
    let mut from = 0usize;
    let anchor = loop {
        match lower_attrs[from..].find("color") {
            Some(rel) => {
                let at = from + rel;
                let after = at + "color".len();
                let before_ok = at == 0
                    || lower_attrs[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_whitespace() || c == '=');
```
What is wrong: the anchor scan runs over the RAW attribute string, ignoring quoted values. A value like `face="my color choice"` contains " color " with valid boundaries -> the loop breaks on it, the following text is not `=`, and `find_color` returns `Some(None)` (declared "no color attribute") — the real `color="#00ff00"` later in the same tag is never found. Worse, a value like `face="a color=x"` makes the unquoted path parse `x"` as the color -> spurious BadStyleValue. What correct looks like: tokenize attributes respecting quotes before matching names. Confidence: HIGH — Verification: read (traced both paths); no test covers it.
F-004 [S3]
Where: crates/subtitles/src/srt/tags.rs:101-134 (absence)
Code:
```
    if name.eq_ignore_ascii_case("b") {
        style.bold = true;
    } else if name.eq_ignore_ascii_case("i") {
        style.italic = true;
    } else if name.eq_ignore_ascii_case("u") {
        style.underline = true;
```
What is wrong: `<s>`/`</s>` (strikethrough) is degraded as UnknownTag although `TextStyle` HAS a `strikeout` field (set by the ASS engine's `\s`, tags.rs:230) — the two engines disagree on the model's capability for the same concept. What correct looks like: map `<s>` like `<u>`. Confidence: HIGH — Verification: read + rg (zero `strikeout` hits under src/srt/).
F-005 [S4]
Where: crates/subtitles/src/srt/tags.rs:24 vs 85-90
Code:
```
//! [`CueText::Styled`] otherwise, with runs split at style
//! boundaries; empty runs are not emitted.
```
vs
```
    if !current.is_empty() || segments.is_empty() {
        segments.push(StyledSegment {
            text: current,
            style,
        });
    }
```
What is wrong: text `"<b>"` (tag-only cue text) ends with current="" and segments empty -> the `|| segments.is_empty()` arm pushes an EMPTY-TEXT segment, contradicting "empty runs are not emitted". Harmless downstream but the doc claim is false. Confidence: HIGH — Verification: read (traced); untested path.
F-006 [S4]
Where: crates/subtitles/src/srt/tags.rs:6 vs 214-222
Code:
```
//! `<font color="#rrggbb" | "#rgb" | "#rrggbbaa">` / `</font>`.
```
What is wrong: the 8-digit branch is documented and implemented but rg finds NO 8-digit color test anywhere under crates/subtitles (only model/style.rs's separate `Color::from_hex`). Untested documented feature. The arithmetic itself (alpha last, CSS order) reads correct. Confidence: HIGH — Verification: ran (rg).
F-007 [S4]
Where: crates/subtitles/src/srt/tags.rs:275-281
Code:
```
                if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                }
                .and_then(char::from_u32)
```
What is wrong: numeric entities decode control characters into cue text (`&#0;` -> NUL, `&#7;` -> BEL) — HTML replaces these; nothing filters them. Placeholder renderer tolerates any char, so impact is cosmetic today. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_markup | 33-98 | REAL | S4 | fast path; run splitting correct (traced 4 cases); empty-segment edge F-005; Plain-collapse correct |
| apply_tag | 101-134 | REAL | S3 | case-insensitive names; `</font>` stack-less (F-002); `<s>` missing (F-004); `<>` silently dropped (documented) |
| find_color | 139-186 | REAL | S3 | quote-blind anchor scan (F-003); value extraction (quoted/unquoted) itself sound |
| parse_hex_color | 189-225 | REAL | S4 | 3/6/8-digit correct order; 8-digit untested (F-006) |
| EntityDecode enum | 228-233 | REAL | — | data |
| decode_entity | 236-294 | REAL | S4 | 12-byte window sane; whitespace-in-body guard correct; control-char pass-through F-007 |
| tests (6) | 296-415 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plain_text_stays_plain | YES | exact string + zero damage |
| bold_italic_underline_map_to_segments | YES | exact 4- and 2-segment vectors |
| font_color_maps_and_bad_values_degrade | YES | exact color values + damage kinds + collapse case |
| unknown_and_unterminated_tags | YES | exact damage vector + literal `<` |
| entities_decode_or_stay_literal | YES | exact decoded strings + damage count |
| style_returning_to_plain_still_splits | YES | exact 2-segment vector |
Gaps: no nested-font test (F-002), no quoted-value-with-"color" test (F-003), no `<s>` test, no 8-digit color test (F-006).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bold/italic/underline styling runs" (subtitle-srt row) | feature-matrix.toml:895 | REAL (library) / UNREACHABLE (product) | test run above; F-001 |
| "`<font color="#rrggbb" | "#rgb" | "#rrggbbaa">`" | tags.rs:5-6 | REAL-UNPROVEN | 8-digit branch untested (F-006); 3/6-digit verified |
| "empty runs are not emitted" | tags.rs:24 | FALSE (edge) | F-005 trace |
| "unknown entities stay literal + UnknownEntity" | tags.rs:14-15 | REAL | entities test passes |
| "other `<font>` attributes (`face`, `size`) are ignored silently" | tags.rs:18-19 | REAL | face/size test, dmg empty |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Whole chain unreachable from shipped builds (F-001).

## 8. Dependencies & flags
std-only within the crate (media-types is the sole dep); no features; no unsafe; DamageSink is srt::damage's (another agent's file).

## 9. Open questions for the next agent
- srt/parser.rs:253 `CueInterval::new(...).unwrap_or(CueInterval::EMPTY)` — "Infallible by construction" comment; verify the rejection logic actually covers all reversals (that file's auditor).
- Should SRT `<s>` mapping (F-004) be a one-line fix card? Flag to Lead.
