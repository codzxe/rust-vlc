# AUDIT: crates/subtitles/src/model/style.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 219 / 14 (11 prod items, 3 tests) |
| File verdict | REAL (library) / UNREACHABLE (product) — clean data model, one false size comment |
| Reachable from shipped entry point? | NO — used by srt/tags.rs, ass/tags.rs, ass/parser.rs, model/cue.rs, model/wire.rs, srt/render.rs, ass/render.rs, all inside the `subtitles` crate no app depends on (only fuzz/Cargo.toml:50) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | prebuilt libtest `./target/debug/deps/subtitles-70f89de07410626f model::style` -> 3 passed / 0 failed (exit 0); rg users of from_hex/write_hex/TextStyle (exit 0); sed quote verification |

## 1. One-paragraph verdict
Pure-data style-token model, exactly what the crate's parsers need: `Color` (RGBA8 straight alpha, `Copy`, 4 bytes) with a lowercase `#rrggbbaa` writer/parser pair that round-trips and rejects malformed literals, and `TextStyle` (bold/italic/underline/strikeout + optional color, exact-milli font size, boxed font name) with const constructors and builders. Tests are few but exact (round-trip, 6 malformed vectors, default/builder consistency) and pass 3/3. Defects: the "keeps a run 40 bytes on 64-bit" comment is arithmetically false (48 bytes — `Option<i64>` has no niche), and the whole model is stranded in the unwired crate.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f model::style` -> `test result: ok. 3 passed; 0 failed` (exit 0): `#0abcdeff` write/parse round-trip, uppercase acceptance, rejection of ""/#fff/#fffffff/fffffff/#gggggggg/trailing-garbage, `Default == PLAIN`, G-2 builder chaining (strikeout + 32_500 milli + "Comic Sans").

## 3. Findings
F-001 [S2]
Where: this file + apps/cli/Cargo.toml (absence); fuzz/Cargo.toml:50
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50; no such row in apps/cli/Cargo.toml)
What is wrong: domain blocker (tags.rs F-001) — the shared style model underpins every subtitle format claim in the matrix, none reachable from a shipped binary. Confidence: HIGH — Verification: ran (rg).
F-002 [S4]
Where: crates/subtitles/src/model/style.rs:95-98
Code:
```
/// The style-token subset \[PROPOSAL\]: bold / italic / underline /
/// strikeout / optional RGBA color / optional exact font size and
/// family. Clone + `Eq` — `Option<Box<str>>` for the name keeps a run
/// 40 bytes on 64-bit without a `String` header.
```
What is wrong: layout arithmetic gives 48, not 40: 4 bools (4) + `Option<Color>` (5, no niche in 4×u8) = 9, pad 7 to 16; `Option<i64>` = 16 (i64 has no niche) -> 32; `Option<Box<str>>` = 16 (ptr niche) -> 48. The 40-byte figure only works if `Option<i64>` were 8 bytes, which it is not. The relative claim (Box<str> beats String's 24 bytes) is true. Confidence: MEDIUM — Verification: read (manual layout arithmetic; not executed — no size-print harness possible without building). Confidence limited by not running `size_of`; flag UNVERIFIED-exact-number, the 40-vs-48 discrepancy itself is HIGH confidence arithmetic.
F-003 [S4]
Where: crates/subtitles/src/model/style.rs:111-114
Code:
```
    /// Font size in thousandths of a unit (`16` → `16_000`), exact
    /// fixed point — no float near layout (MP §8); `None` = inherit.
    /// (ASS `\fs`, issue #17 G-2.)
    pub font_size_milli: Option<i64>,
```
What is wrong: no model-level invariant — any i64 (negative, > 8192·1000) can be stored; the only guard lives far away in font/pipeline.rs `MAX_FONT_SIZE_PX` (which nothing calls, per that file's F-002). A wire-format (`model/wire.rs`) value of e.g. 999999999999 milli parses into the model silently. Layering smell, not a crash. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Color (struct) | 16-25 | REAL | — | 4 bytes, Copy claim verified by field list |
| Color::new | 29-31 | REAL | — | const |
| Color::BLACK / WHITE | 34-47 | REAL | — | opaque constants, used by ass tests |
| Color::write_hex | 52-59 | REAL | — | no-alloc, lowercase, table-driven |
| Color::from_hex | 65-83 | REAL | — | strict 9-byte, upper+lower, tested rejections |
| hex_val | 86-93 | REAL | — | correct |
| TextStyle (struct) | 100-118 | REAL | S4 | 40-byte doc claim false (F-002); unbounded milli (F-003) |
| TextStyle::new | 124-134 | REAL | — | const; the 4-flag subset constructor |
| TextStyle::PLAIN | 137-145 | REAL | — | equals Default (pinned by test) |
| with_strikeout / with_font_size_milli / with_font_name | 148-164 | REAL | — | const builders (name one non-const, allocates) |
| tests (3) | 167-219 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| hex_roundtrip | YES | exact string + Ok equality both directions |
| hex_rejects_malformed | YES | 6 vectors incl. length and charset classes |
| plain_default_matches | YES | pins Default==PLAIN and all builder fields |
Gap: no test that from_hex rejects `#0abcdegf` style mixed-case-with-nonhex (covered by #gggggggg), no negative milli validation (none exists — F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keeps a run 40 bytes on 64-bit" | style.rs:98 | FALSE (48 by layout arithmetic) | F-002 |
| "Copy — 4 bytes" (Color) | style.rs:14 | REAL | 4×u8 fields, no padding |
| "`None` = inherit the track style" (color/size/name) | style.rs:109-116 | REAL | srt/ass parsers rely on it; ass {\fn} reset emits None (tags.rs audit F-002) |
| "[PROPOSAL]" markers on the subset | style.rs:1,95 | REAL (honest) | disclosed as proposal |

## 7. Dead / duplicated / unreachable code
BLACK/WHITE consts: used in ass tests + srt? (rg shows usage in parser tests at minimum — production usage of WHITE via ass/parser seed styles). No dead items found in-file. Whole crate unreachable from shipped builds (F-001).

## 8. Dependencies & flags
std-only; ModelError from crate::model::error. No features, no unsafe, no floats (i64 milli — MP §8 honored).

## 9. Open questions for the next agent
- model/cue.rs + model/wire.rs auditors: does the wire format validate font_size_milli anywhere before render? (F-003 chain.)
- If a real font stack is admitted, does TextStyle need a style-synthesis channel (faux-bold flags) as font/mod.rs `GlyphSynthesis` suggests? Model gap between the two seams.
