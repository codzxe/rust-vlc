# AUDIT: crates/subtitles/src/model/style.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Lines / functions | 219 / 16 (1 struct Color + 5 Color items + hex_val + 1 struct TextStyle + 5 TextStyle items + 3 tests) |
| File verdict | REAL-UNPROVEN (in-crate test-proven; crate UNREACHABLE from shipped entry points; one module-doc claim about WebVTT color is not wired) |
| Reachable from shipped entry point? | NO (crate-level, see W09-A10 wave context). In-crate users of `TextStyle`/`Color`: model/cue.rs (StyledSegment), model/wire.rs (:141 write_hex, :458 from_hex — the only `from_hex` caller), ass/parser.rs, ass/tags.rs, ass/render.rs, srt/tags.rs, srt/render.rs, vtt/text.rs, vtt/mod.rs, lib.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "from_hex\|write_hex" src tests` (wire.rs-only consumer); `rg -ni "color" src/vtt/` (only a test string + "styling deferred" docs); `rg -n "fn \|TextStyle\|Color" src/vtt/settings.rs` (positioning-only API, no color); `sed -n 185,215p src/srt/tags.rs` (the OTHER hex parser); `rg -rn "size_of\|40 bytes" tests/ src/model/` (no size pin); `rg -n "TextStyle::new\|TextStyle::PLAIN\|with_font" src` (usage map); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269 (267 unit green incl. this file's 3) |

## 1. One-paragraph verdict
A small, clean, exact data-model file: `Color` (RGBA8, straight alpha) with an
allocation-free hex writer and a strict 9-byte `#rrggbbaa` reader, and the
G-2-extended `TextStyle` token (four flags, optional color, thousandths font
size, boxed font name) with const constructors/builders. All three tests assert
real round-trips and rejections and would fail under mutation; the hex pair is
internally consistent (wire.rs writes `#rrggbbaa`, wire.rs reads it back — the
only reader). Two defects, both documentation-level: the module doc claims the
subset includes "WebVTT's `color`", but no VTT code path produces a `Color` —
VTT styling is explicitly collected-and-deferred (vtt/mod.rs "STYLE blocks …
collected but not applied") — so the claim describes an unwired aspiration; and
the "keeps a run 40 bytes on 64-bit" comment does not survive layout arithmetic
(48 bytes by hand computation). A third hygiene note: the crate carries two
hex-color parsers with different vocabularies (`Color::from_hex`: 9 bytes only;
`srt/tags.rs::parse_hex_color`: `#rgb`/`#rrggbb`/`#rrggbbaa`).

## 2. What actually works (verified)
- Tests green in baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5271
  ("Running unittests src/lib.rs … subtitles … 267 passed; 0 failed") — the
  3 in-file tests are inside that count.
- `write_hex`/`from_hex` round-trip verified by hand: `Color::new(0x0a,0xbc,0xde,0xff)`
  → "#0abcdeff" → same Color; uppercase accepted (`hex_val` covers `A..=F`);
  `bytes[1 + i*2]` indexing is gated by the `len() == 9` check — panic-free.
- `TextStyle::default() == PLAIN` and the builder chain are structurally
  identical (`#[derive(Default)]` on all-Option/false fields) — pinned by
  `plain_default_matches`.

## 3. Findings
### F-001 [S3]
Where: crates/subtitles/src/model/style.rs:1-3
Code:
```
//! The style-token subset \[PROPOSAL\] (card 5.L2): bold, italic,
//! underline, strikeout, color, font size and font name. The SRT-
//! trivial run set (doc 04 §5 row 1) plus WebVTT's `color`; the ASS
```
What is wrong: The sentence sells the token's provenance as "SRT-trivial …
plus WebVTT's `color`". The SRT half is real (srt/tags.rs maps `<font
color=…>` into `TextStyle.color` — via its OWN parser, see F-003). The WebVTT
half is wired NOWHERE: `rg -ni color` over src/vtt/ finds only a test fixture
string and the module docs that say STYLE blocks are "collected but not
applied" (vtt/mod.rs:81-88, :176-178 — raw CSS stored, `::cue` counted);
vtt/settings.rs parses positioning only. No VTT path constructs a `Color` or
`TextStyle`. Why it matters: a reader of the model's entry doc believes VTT
color cues round-trip into the style token; they do not — the claim is
PAPER-ONLY for the VTT half. What correct looks like: "…plus (planned)
WebVTT's `color`" or drop the WebVTT mention until vtt styling lands.
Confidence: HIGH — both sides read (style.rs + vtt/mod.rs + vtt/settings.rs).

### F-002 [S4]
Where: crates/subtitles/src/model/style.rs:95-98
Code:
```
/// The style-token subset \[PROPOSAL\]: bold / italic / underline /
/// strikeout / optional RGBA color / optional exact font size and
/// family. Clone + `Eq` — `Option<Box<str>>` for the name keeps a run
/// 40 bytes on 64-bit without a `String` header.
```
What is wrong: The size claim does not survive arithmetic: `Option<i64>` (no
niche) = 16 bytes, `Option<Box<str>>` (niche) = 16, `Option<Color>` = 5, four
bools = 4 → 41 bytes, aligned to 8 → **48 bytes**, not 40 (rustc may reorder
fields but cannot go below 41). Nothing in the crate pins the size (no
`size_of` assertion anywhere — verified by rg). The comment is stale
(plausibly written before `font_size_milli` was added). Why it matters: a
performance claim in the model's own docs that is off by 20%. What correct
looks like: `size_of::<TextStyle>() == 48` test + corrected comment, or
delete the number. Confidence: MEDIUM — layout computed by hand from the
language rules; NOT verified by execution (builds forbidden; see
REQUEST-ORCH-RUN in the log).

### F-003 [S4]
Where: crates/subtitles/src/model/style.rs:65-83 vs crates/subtitles/src/srt/tags.rs:188-215
Code (style.rs):
```
    pub fn from_hex(s: &str) -> Result<Self, ModelError> {
        let bad = || ModelError::BadColor;
        let bytes = s.as_bytes();
        if bytes.len() != 9 || bytes[0] != b'#' {
            return Err(bad());
        }
```
What is wrong: The crate contains TWO hex-color parsers with divergent
vocabularies: `Color::from_hex` accepts exactly `#rrggbbaa` (9 bytes) — its
only caller is the crate's own wire format (model/wire.rs:458), which always
feeds it `write_hex` output; `srt/tags.rs::parse_hex_color` independently
accepts `#rgb`, `#rrggbb`, AND `#rrggbbaa` for SRT `<font color>` markup. A
future caller reusing the model's `from_hex` for user-facing subtitle input
will type-reject `#rrggbb` — the most common SRT spelling. Why it matters:
duplication with different behavior invites exactly that wrong reuse; the
narrow parser's name advertises generality it does not have. What correct
looks like: one parser with a documented vocabulary (widened or split), the
wire format pinning its own strictness separately. Confidence: HIGH — both
bodies read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Color` (struct) | 15-25 | REAL | — | Copy/Hash, 4×u8, straight alpha documented |
| `Color::new` | 29-31 | REAL | — | const |
| `Color::BLACK` / `WHITE` | 34-47 | REAL | — | const, opaque |
| `Color::write_hex` | 52-59 | REAL | — | lowercase `#rrggbbaa`, zero intermediate alloc (claim TRUE — pushes into `out`) |
| `Color::from_hex` | 65-83 | REAL | S4 (F-003) | strict 9-byte form; len-gated indexing; wire-only consumer |
| `hex_val` | 86-93 | REAL | — | correct ranges, no unwrap |
| `TextStyle` (struct) | 99-118 | REAL | S4 (F-002) | all-Option+bool model; doc size claim stale |
| `TextStyle::new` | 124-134 | REAL | — | const; G-2 fields default to inherit |
| `TextStyle::PLAIN` | 137-145 | REAL | — | const; equals Default (test-pinned) |
| `TextStyle::with_strikeout` | 148-151 | REAL | — | const builder |
| `TextStyle::with_font_size_milli` | 154-157 | REAL | — | const builder |
| `TextStyle::with_font_name` | 161-164 | REAL | — | non-const (Box from &str) — fine |
| tests: `hex_roundtrip` | 171-182 | REAL test | — | exact round-trip + uppercase |
| tests: `hex_rejects_malformed` | 184-196 | REAL test | — | 6 bad vectors incl. length/trailing/glyph errors |
| tests: `plain_default_matches` | 198-218 | REAL test | — | Default==PLAIN + full builder chain |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hex_roundtrip | YES | exact channel values both directions + case-insensitivity |
| hex_rejects_malformed | YES | each vector targets a distinct guard (len≠9, non-hex, missing #, trailing junk) |
| plain_default_matches | YES | field-wise equality + builder values |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The SRT-trivial run set … plus WebVTT's `color`" | style.rs:2-3 | SRT half REAL / VTT half PAPER-ONLY (F-001) | srt/tags.rs:122-124 populates color; zero VTT producers (vtt styling deferred) |
| "no intermediate allocation (D-2: the serializer hot path…)" | style.rs:49-51 | REAL | write_hex pushes into `&mut String`, no alloc |
| "keeps a run 40 bytes on 64-bit" | style.rs:97-98 | FALSE by arithmetic (48) — UNVERIFIED by execution (F-002) | hand layout computation; no pinning test |
| "exact fixed point — no float near layout (MP §8)" | style.rs:111-113 | REAL | `font_size_milli: Option<i64>`; parser part2 verified `parse_decimal_milli` |

## 7. Dead / duplicated / unreachable code
- Duplicated: the two hex parsers (F-003).
- `Color::BLACK` — grep shows no non-test user in-crate (WHITE is used by
  parser tests/tags); a library-crate API surface, acceptable but noted.
- Crate-level unreachability: see header.

## 8. Dependencies & flags
Depends only on `crate::model::error::ModelError` (one variant used). No
external deps, no unsafe, no feature gates, no cfg beyond `#[cfg(test)]`.
Note: the test file uses the repo's `lumen-allow-literal` escape hatch for
the `#fff` validation vector (style.rs:188) — consistent with the
lint-literals gate (baseline #13), not a violation.

## 9. Open questions for the next agent
- model/wire.rs owner: does the wire round-trip test suite cover a color
  round-trip through write_hex/from_hex (pinning the 9-byte pair)?
- Whoever audits vtt/mod.rs: confirm the STYLE-block deferral surfaces a
  stats/damage counter (the F-001 mitigation) — my read of vtt/mod.rs:81-88
  says `::cue` rules are counted.
- REQUEST-ORCH-RUN (non-essential, would firm F-002 to HIGH):
  `cargo test -p subtitles --lib style -- --nocapture` after adding a
  `size_of::<TextStyle>()` print — or any one-off `rustc` size check.
