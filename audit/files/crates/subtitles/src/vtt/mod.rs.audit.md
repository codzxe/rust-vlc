# AUDIT: crates/subtitles/src/vtt/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Lines / functions | 423 / 4 (+8 tests) |
| File verdict | REAL (unreachable from shipped binaries — same crate-level verdict as dvb/mod.rs F-008) |
| Reachable from shipped entry point? | NO — no crate outside `crates/subtitles` and `fuzz/` depends on the subtitles crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b vtt::` -> 22 passed, 0 failed (exit 0); `sed -n` quote verification |

## 1. One-paragraph verdict
A genuine, spec-shaped WebVTT block parser with honest typed damage, correct µs timestamps
(saturating, verified by `vtt::timestamp::tests::saturates_at_i64_max`), and byte-asserting
tests that pass. The block dispatcher is prefix-based instead of word-boundary-based, so
`NOTES`/`STYLEZ`-style blocks and a glued `WEBVTTjunk` header are misclassified against the
spec, and auto-assigned cue ids silently collide with explicit numeric ids — all small-blast-
radius defects in a module nobody in the shipped product links.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b vtt::` -> `test result: ok. 22
passed; 0 failed` (covers header forms, BOM+CRLF, ids, NOTE/STYLE/REGION, six damage kinds
with exact line numbers, glued `-->`, empty cue text, multiline text, 75-minute hours-less
timestamps).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/vtt/mod.rs:171-181
Code:
```
    let first = block[0];
    let line_no = |i: usize| (block_base + i) as u32;
    if first.starts_with("NOTE") {
        return; // comment block: skipped whole
    }
    if first.starts_with("STYLE") {
```
What is wrong: WebVTT requires block keywords to be followed by a space, tab, or end of line.
`NOTES AND STUFF`, `STYLEZ`, `REGIONX` all match the prefixes and are misrouted: a malformed
cue block is silently swallowed as a NOTE (no damage event), a non-style block is stored as a
StyleBlock. What correct looks like: check the byte after the keyword (`first == "NOTE" ||
first.starts_with("NOTE ")` / `\t`).
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/subtitles/src/vtt/mod.rs:139-141
Code:
```
    if lines.is_empty() || !lines[0].starts_with("WEBVTT") {
        return Err(VttError::MissingHeader);
    }
```
What is wrong: The spec's signature is `WEBVTT` + (space|tab|EOL); `WEBVTTjunk` is accepted
here as a valid header. The parser's single fatal check is looser than documented ("does not
begin with the `WEBVTT` header line").
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/subtitles/src/vtt/mod.rs:229-251
Code:
```
    let cue_id = match id {
        Some(raw) => match raw.parse::<u64>() {
            Ok(n) => CueId::new(n),
            ...
            Err(_) => {
                // Legal non-numeric id: auto-assigned.
                *auto_id = auto_id.saturating_add(1);
                CueId::new(*auto_id)
            }
```
What is wrong: Auto ids start at 1 in document order and can collide with explicit numeric
ids (test `ids_numeric_nonnumeric_and_absent` produces cue ids 1, 1, 2). The doc admits ids
are opaque, but any consumer keying cues by id (dedup, wire encoding) silently merges two
distinct cues. What correct looks like: auto ids allocated outside the u64 numeric namespace
(e.g. offset) or a collision damage note.
Confidence: HIGH — Verification: read + executed test demonstrating the collision.

F-004 [S4]
Where: crates/subtitles/src/vtt/mod.rs (parse loop, no ledger cap) vs crates/subtitles/src/vtt/damage.rs:19-20
Code:
```
Setting/block names stored in events are truncated at a char
boundary to ≤16 bytes so hostile inputs cannot balloon the ledger.
```
What is wrong: The *name* length is capped but the *number* of events is not: a hostile VTT
of N orphan blocks yields N DamageEvents with no MAX_DAMAGE_EVENTS cap (DVB caps at 64, SRT
at 256 — the in-repo precedent). Memory stays linear in input size, so this is hygiene
inconsistency, not an amplification.
Confidence: HIGH — Verification: read (rg MAX_DAMAGE over vtt/ finds no cap).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| StyleBlock | 68-77 | REAL | — | collected-not-applied CSS + naive `::cue` substring count (comments count too) |
| VttDocument | 80-90 | REAL | — | plain data |
| VttError + Display + Error | 93-118 | REAL | — | single variant, honest message |
| parse | 127-164 | REAL | S4 | BOM strip, CRLF normalize; header-glued cues are spec-correct header text (not a bug); accepts glued `WEBVTTjunk` (F-002) |
| parse_block | 169-261 | REAL | S3 | prefix dispatch without word boundary (F-001); id collision (F-003); timing_idx>1 typed drop; IdOverflow typed |
| split_timing | 266-278 | REAL | — | split_once, greedy prefix parse, rest = settings |
| is_blank | 280-282 | REAL | — | space/tab only, empty=true |
| tests | 287-421 | REAL | — | 8 tests, byte-exact assertions incl. 6 damage kinds and line numbers |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_forms_and_empty_documents | YES | exact Err/len assertions, BOM+CRLF case |
| basic_cue_with_id_and_settings | YES | µs values, text, positioning all asserted |
| ids_numeric_nonnumeric_and_absent | YES | pins the (colliding) id sequence |
| note_style_region_blocks | YES | cue_rule_count=2, REGION damage |
| malformed_blocks_are_damages_not_panics | YES | 7 damage kinds with kinds + line numbers asserted |
| glued_separator_and_empty_cue_text | YES | µs values + empty text |
| multiline_text_and_line_numbers | YES | escape/tag stripping + 99-min form µs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A missing WEBVTT header is the one fatal, typed error" | mod.rs:33-35 | REAL | parse L139-141; test header_forms |
| "NOTE blocks skipped, STYLE blocks collected tolerantly (CSS is NOT applied)" | mod.rs:2-4 | REAL | parse_block; note_style_region_blocks |
| "vertical:/size: ... typed notes, never silent misrendering" | mod.rs:39-41 | REAL | malformed test asserts 2 UnsupportedSetting |
| "BOM is skipped, CRLF is normalized, whitespace-only lines count as blank" | mod.rs:42-43 | REAL | parse L133-137 + is_blank; BOM+CRLF test |
| "non-numeric ids ... get sequential auto ids starting at 1" | mod.rs:46-48 | REAL | test pins 1,1,2 sequence (collision documented as opaque) |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped binaries (crate-level, see dvb/mod.rs F-008). No intra-file dead code.

## 8. Dependencies & flags
Uses crate::model (cue/time), vtt submodules (damage/settings/text/timestamp), testkit (dev).
No feature gates. Sibling files audited by W09-A05/A06/A07/A09.

## 9. Open questions for the next agent
- W19-A10 (docs/formats/sub-vtt.md): does the doc claim spec-conformant NOTE/STYLE/REGION
  recognition? If yes it inherits F-001.
- W09-A06 (settings.rs): confirm percent parsing rejects >100 and negative values (not
  verified here — out of my file list).
