# AUDIT: crates/subtitles/src/vtt/text.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 179 / 8 (3 fns + decode_one, 4 tests) |
| File verdict | UNREACHABLE (real, library-tested code; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — crate-level proof (only `fuzz/Cargo.toml` depends on `subtitles`). In-crate caller: `vtt/mod.rs:252` (`text::decode_cue_text` on the joined cue lines); exercised by vtt unit tests + `tests/vtt_fuzz.rs`/`tests/vtt_acceptance.rs` (green in no-fail-fast baseline). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read; `rg -n "decode_cue_text" crates/subtitles/src/vtt` (sole caller vtt/mod.rs:252); read of vtt/mod.rs:175-260 (caller context) and vtt/damage.rs:1-80 (damage vocabulary — no text-loss kind exists); baseline `orchrun-test-workspace-nofailfast.log.txt:5269+` (267/0 incl. `vtt::text::tests`). |

## 1. One-paragraph verdict
A tight, well-tested tag-stripper/entity-decoder with the right
ordering (tags first, single-pass escapes second — pinned by tests)
and correct hostile-input handling on every numeric-reference edge
(overflow, surrogates, NUL, empty hex). Its one real defect is a
contract gap one level up: an unterminated `<` silently deletes the
rest of the cue text — potentially every remaining line — and the
vtt damage ledger, which promises "every skipped or degraded region
of the document is reported as a typed event" (damage.rs:4-6) and
which types every settings tolerance, records nothing. Two smaller
items: numeric character references are decoded although the WebVTT
grammar defines only six named escapes (documented tolerance,
browser-divergent), and the "tags cannot span lines" justification
comment does not match the spec tokenizer it invokes. Nothing shipped
runs this code.

## 2. What actually works (verified)
— Byte-safety of the slicing: `find('<')`/`find('>')` return
boundaries; `&rest[lt + 1..]` (38) is safe because `<` is one byte
(same for `>` at 40); no panic path exists in the file (traced).
— Escape semantics pinned by tests: `&amp;lt;` → `&lt;` exactly once
(152), `&lt;i&gt;` survives as text (155), the six WebVTT escapes
decode (141-144), `&#65;`/`&#x42;`/`&#x1F600;` decode (145-146),
`&#999999999999;`/`&#xD800;`/`&#0;` pass through literally (164-166) —
overflow→`u32` parse failure, surrogate→`char::from_u32` failure,
NUL→`is_control` rejection (102-107), all verified by trace.
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt:5269`
→ 267/0 subtitles unit tests incl. all 4 `vtt::text::tests`;
`tests/vtt_fuzz.rs` (7003-input corpus claim) ran green in the same
log.

## 3. Findings

### F-001 [S3] Unterminated-tag tail drop is silent — no damage event for cue-text loss
Where: crates/subtitles/src/vtt/text.rs:39-44 (site), vtt/mod.rs:252 (caller), vtt/damage.rs:4-6 (broken promise)
Code:
```rust
        match after.find('>') {
            Some(gt) => rest = &after[gt + 1..],
            None => {
                // Unterminated tag: drop the tail.
                return out;
            }
        }
```
with the module contract (vtt/damage.rs):
```rust
//! the parser never panics and never
//! aborts on a malformed region — every skipped or degraded region of
//! the document is reported as a typed event
```
What is wrong: when cue text contains a bare `<` with no closing `>`
(the dirty-world "x < y" case, or a truncated tag), everything from
the `<` to the end of the cue — including all subsequent LINES, since
vtt/mod.rs:252 joins the whole payload with `\n` before calling — is
discarded. `decode_cue_text` has no damage channel and no
`DamageKind` exists for text loss (vocabulary read: malformed /
unsupported / noted taxa only). The parser's own philosophy (typed
event for every degradation) is honored by settings.rs for every
tolerated setting but violated here for the largest possible loss.
Why it matters: a caption saying "if a < b then" renders as "if a "
with zero diagnostic; the user cannot distinguish clean files from
mangled ones, and the damage ledger — the module's observability
product — is blind to the worst degradation it produces.
What correct looks like: a `DamageKind::DroppedText { from_line }`
event pushed by the caller (decode_cue_text returning a flag, or
taking `&mut Vec<DamageEvent>` like `parse_settings` does at
settings.rs:232).
Confidence: HIGH — Verification: read (both files + damage
vocabulary).

### F-002 [S4] Numeric character references are decoded — beyond the WebVTT escape grammar
Where: crates/subtitles/src/vtt/text.rs:93-108
Code:
```rust
            if let Some(digits) = name.strip_prefix('#') {
                let value = if let Some(hex) = digits
                    .strip_prefix('x')
                    .or_else(|| digits.strip_prefix('X'))
                {
                    u32::from_str_radix(hex, 16).ok()?
```
What is wrong: the WebVTT grammar defines exactly six cue-text
escapes (`&amp; &lt; &gt; &lrm; &rlm; &nbsp;`); numeric references
(`&#65;`) are not part of it — spec-conformant renderers (browsers)
display them literally. This module decodes them to characters.
The module header documents the choice ("the five named escapes plus
numeric character references are decoded", 3-4), so it is a declared
tolerance, but it changes rendering vs the reference behavior for any
file that carries one.
Why it matters: silent divergence from the format's reference
renderers in the tolerant direction; a differential test against
browser output would fail.
What correct looks like: either spec-strict (numeric refs literal,
matching browsers) or the divergence recorded in the wave's
compatibility ledger with the header note as its evidence.
Confidence: MEDIUM — Verification: read for the behavior (HIGH); the
WebVTT escape-grammar claim is domain knowledge (spec text not in
sandbox — UNVERIFIED here).
### F-003 [S4] "Tags cannot span lines in WebVTT" — justification comment not backed by the spec it cites
Where: crates/subtitles/src/vtt/text.rs:27-29
Code:
```rust
/// Remove every `<...>` tag. Tags cannot span lines in WebVTT, but we
/// tolerate the whole document being one string: an unterminated `<`
/// truncates the remainder (documented tolerance, never a panic).
```
What is wrong: the WebVTT cue-text tokenizer's tag state terminates
on `>` only — it has no newline terminator — so per spec a tag CAN
span lines (that is precisely why the whole-payload truncation this
code implements is the spec-shaped outcome). The comment justifies
the tolerance with a spec claim that is at best unverified and at
worst backwards.
Why it matters: future maintainers will "fix" the truncation to be
per-line based on this comment and diverge from the tokenizer
semantics.
What correct looks like: cite the tokenizer state machine (tag state
ends at `>`), drop the "cannot span lines" assertion.
Confidence: MEDIUM — Verification: read for the comment; the spec
tokenizer behavior is domain knowledge (UNVERIFIED — no spec text in
sandbox).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode_cue_text` | 22-25 | REAL | — | composition: strip then decode (order pinned by test 150-156); two String allocs per cue (join + to_owned fast paths at 32/57 — Cow would elide) |
| `strip_tags` | 30-49 | REAL | F-001 | O(n); no-`<` fast path still copies (32); unterminated tail dropped silently |
| `decode_entities` | 55-77 | REAL | F-002 (via decode_one) | single pass, non-recursive; bare `&` preserved; O(n) |
| `decode_one` | 81-114 | REAL | — | six named escapes = the WebVTT set; numeric refs = extension (F-002); control/BOM rejection; no length cap on entity name but `find(';')` is linear and failure falls back to literal copy — no pathological cost |
| tests: `strips_common_tags` | 120-137 | REAL | — | v/speaker, c.class, karaoke stamp, ruby |
| `decodes_named_and_numeric_escapes` | 139-147 | REAL | — | |
| `escapes_decode_exactly_once_and_order_is_tags_first` | 149-156 | REAL (strong) | — | pins the two subtle invariants |
| `malformed_input_is_tolerated` | 158-177 | REAL (strong) | — | overflow/surrogate/NUL/literal-`&`/truncation/multi-line/empty |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| strips_common_tags | YES | exact strings incl. karaoke timestamp tag and ruby |
| decodes_named_and_numeric_escapes | YES | exact characters |
| escapes_decode_exactly_once_and_order_is_tags_first | YES | the two ordering/single-pass invariants |
| malformed_input_is_tolerated | YES | exact literal pass-throughs for all three hostile numeric forms + truncation |
| (gap) | — | no test for a multi-line cue where line 1 ends unterminated (asserting lines 2+ are dropped — the F-001 blast radius); no test that the truncation produces no damage event (it cannot — no channel exists) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parse WebVTT: cues, settings subset, NOTE/STYLE blocks" — implemented ×3 platforms; "tag-stripped plain cue text with entity decoding, and a damage-event tolerance ledger" | docs/feature-matrix.toml subtitle-vtt (918-928); crates/subtitles/src/lib.rs:99-101 | REAL (library, tested) / UNREACHABLE (shipped); ledger claim FALSE for cue-text loss (F-001) | code + tests green in no-fail-fast baseline; damage vocabulary lacks any text-loss kind |
| "3 real ffmpeg -c:s webvtt fixtures + settings_heavy handcrafted, 7003 fuzz inputs zero panics, 92/0 at card" | subtitle-vtt evidence string | REAL-UNPROVEN | fixtures/corpus not present in sandbox; the vtt test binaries ran green in baseline, the 7003-input figure is a card-session claim |
| "7003 fuzz inputs zero panics" | same | REAL-UNPROVEN (panic-freedom plausible: no panic path traced in text.rs; parser audited by another agent) | no run in baseline enumerates the 7003 inputs |

## 7. Dead / duplicated / unreachable code
- Nothing dead in-file. Whole module unreachable from shipped
  binaries (crate isolation, header proof).
- The styling deferral (`CueText::Styled` never produced by the vtt
  path — mod.rs:257 always `CueText::Plain`) is documented in this
  file's header (13-18) — honest, noted for the ASS wave question.

## 8. Dependencies & flags
- None: pure std string processing; no feature gates; visibility
  `pub(super)` correctly scoped.

## 9. Open questions for the next agent
- vtt/mod.rs / vtt/fuzz.rs owner: does the fuzz corpus include an
  unterminated-tag cue (exercising F-001's silent drop)? And does any
  acceptance test assert damage-event COUNTS on a text-truncation
  case (they cannot today — no event exists)?
- Wave synthesizer: the "damage-event tolerance ledger" phrase in
  lib.rs:99-101 and the subtitle-vtt row should be qualified — the
  ledger covers settings/blocks/timestamps, not cue-text loss.
