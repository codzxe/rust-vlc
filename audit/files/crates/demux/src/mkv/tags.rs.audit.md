# AUDIT: crates/demux/src/mkv/tags.rs
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2025-06-27 |
| Lines / functions | 773 / 16 (+6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli -> demux -> `MkvDemuxer::open` (mkv/mod.rs:398,423 `tags::ID_TAGS => self.collect_tags(&child)`) -> `parse_tags` (mkv/mod.rs:472) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `cargo test -p demux --test mkv_editions_tags` (exit 0, 14 passed); `rg` for callers; `sed -n` quote verification |

## 1. One-paragraph verdict
Real MKV `Tags` parser with a genuinely bounded walk (strictly-advancing
`pos`, typed truncation/structure/ResourceLimit errors, depth guard). The
robustness model in the header doc is 90% true — but the two memory guards
that justify the module's existence have holes: the cumulative value-byte
budget never counts `TagName`/`TagLanguage`/`TagLanguageIETF`/`TargetType`
bytes, and every budget is per-`Tags`-element while `collect_tags` merges an
unbounded number of elements. The parse paths are exercised by real
hand-built byte fixtures (integration test), not by the model-only unit tests
in this file.

## 2. What actually works (verified)
— verified by `cargo test -p demux --test mkv_editions_tags` -> 14 passed,
0 failed (byte-level fixtures: global/track scoping, UID-0 rule, nesting
guard at exactly MAX_TAG_NESTING, MAX_TAGS+1 bomb, 2 MiB TagString bomb,
mid-value truncation -> `demux.mkv.truncated`). Unit tests
(mkv::tags::tests, 6) pass per baseline cargo-test-workspace.txt:1971-1976.
Loop termination: `child_end = data_pos + size >= data_pos > pos` (header ≥
2 bytes), so `pos` strictly increases; `declared_end` is
`checked_add` (ebml.rs:418-424); `read_exact_at`/`prefix_len` clamp to
`until.min(self.length)` (ebml.rs:379-394, 306-309).

## 3. Findings
F-001 [S2]
Where: crates/demux/src/mkv/tags.rs:522-532 vs 545-551
Code:
```
            ID_TAG_NAME => {
                out.name =
                    read_text(reader, &child, child_end, MAX_TAG_TEXT_BYTES)?.unwrap_or_default();
```
vs
```
                budget.value_bytes = budget
                    .value_bytes
                    .checked_add(bytes.len() as u64)
                    .ok_or_else(|| error::bad_structure(child.data_pos))?;
                if budget.value_bytes > MAX_TAG_LIST_BYTES {
                    return Err(tag_budget(MAX_TAG_LIST_BYTES, budget.value_bytes));
                }
```
What is wrong: only `ID_TAG_STRING`/`ID_TAG_BINARY` feed `budget.value_bytes`;
`TagName`, `TagLanguage`, `TagLanguageIETF` (all capped at 1 MiB each via
`read_text`) and `Targets`' `TargetType` are retained as `String`s in the
`SimpleTag`/`TagTargets` structs but never counted. Why it matters: the
documented purpose of `MAX_TAG_LIST_BYTES` is "bounds a many-medium-values
bomb the per-value ceiling alone cannot" (tags.rs:123-126) — a bomb of 16384
simple tags each carrying ~1 MiB name + language + IETF retains ~48 GiB with
`budget.value_bytes == 0`; the 8 MiB ceiling never fires. What correct looks
like: `read_text` calls also add to `value_bytes` (or a separate retained-bytes
counter covers all string fields).
Confidence: HIGH — Verification: read (accounting arms at 545-551/559-565 are
the only writers/readers of `value_bytes`; `rg "value_bytes"` in tags.rs =
9 hits: the field decl at 347 plus the two ID_TAG_STRING/ID_TAG_BINARY arms;
no other site updates or checks it).

F-002 [S2]
Where: crates/demux/src/mkv/tags.rs:357-364 + crates/demux/src/mkv/mod.rs:471-475
Code:
```
    let mut list = TagList::default();
    let mut budget = TagBudget::default();
```
+ (mod.rs)
```
    fn collect_tags(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = tags::parse_tags(&mut self.reader, child, self.segment_end)?;
        self.tags.tags.extend(more.tags);
```
What is wrong: every `Tags` master gets a fresh budget; the walk merges all of
them with no cross-element ceiling. Why it matters: the "Memory is bounded
(MP §12)" claim (tags.rs:34-36) holds per element only — a file with N `Tags`
masters retains N x 16384 simple tags and N x 8 MiB of counted value bytes
(plus the F-001 uncounted names) with no global refusal; a sparse multi-GiB
file converts directly into retained heap. What correct looks like: a
demuxer-level cumulative budget threaded through `collect_tags`.
Confidence: HIGH — Verification: read (`TagBudget::default()` is per-call;
`collect_tags` extends unconditionally; `MAX_TAGS`/`MAX_SIMPLE_TAGS` docs say
"per `Tags` element", so the per-element scoping is documented but the merged
list is genuinely unbounded).

F-003 [S3]
Where: crates/demux/src/mkv/tags.rs:602-608 and 643-647
Code:
```
fn text_from_padded(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    ebml::utf8_from(&bytes[..end]).unwrap_or_default()
}
```
and the test:
```
        // Invalid UTF-8 never fabricates a value.
        assert_eq!(text_from_padded(&[0xFF, 0xFE]), "");
```
What is wrong: an invalid-UTF-8 `TagString` becomes `TagValue::Text("")` —
a fabricated empty value that also *retains* the pair (the degenerate check
at 579-581 treats `Text("")` as content). The test comment says "never
fabricates a value" while the assertion returns a fabricated empty string.
Why it matters: `read_text` (names/languages) returns `None` on invalid UTF-8
("kept as absent, never fabricated", tags.rs:585-587) — the two paths
contradict each other, and ffmpeg keeps the decodable prefix / substitutes
U+FFFD rather than discarding the whole value. What correct looks like: same
rule as `read_text` (skip to `TagValue::None`) or lossy decode.
Confidence: HIGH — Verification: read + ran the asserting test (passes in the
run above).

F-004 [S4]
Where: crates/demux/src/mkv/tags.rs:412-413
Code:
```
        match child.id {
            ID_TARGETS => targets = parse_targets(reader, &child, child_end)?,
```
What is wrong: multiple `Targets` masters inside one `Tag` — the last
silently replaces the first (spec allows one). No damage counter, unlike the
chapters/attachments zones. Confidence: MEDIUM — Verification: read.
What correct looks like: keep first + damage row, or structural error.

F-005 [S4]
Where: crates/demux/src/mkv/tags.rs:446,520
Code:
```
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
```
What is wrong: computed for *every* child including unknown ones that are
skipped by declared size; on the 64-bit release lane `u64 -> usize` cannot
fail, so the error arm is dead code, and on 32-bit an unknown child with a
>4 GiB declared size would abort the walk instead of being skipped.
Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tag_budget | 140-150 | REAL | - | typed ResourceLimit ctor; unit-tested (708-722) |
| TagValue::as_text | 167-176 | REAL | - | trivial, correct |
| SimpleTag::text_pair | 196-203 | REAL | - | trivial, correct |
| TagTargets::default | 223-234 | REAL | - | spec default 50; tested |
| TagTargets::is_global | 236-246 | REAL | - | empty lists = global; tested (615-635) |
| TagTargets::targets_track | 236-253 | REAL | - | UID-0 wildcard; tested |
| TagList::global_text | 273-290 | REAL | - | case-insensitive; tested (655-705) |
| TagList::global_text_pairs | 296-309 | REAL | - | text-only; tested |
| TagList::track_text_pairs | 314-327 | REAL | - | tested |
| TagList::tags_for_track | 331-336 | REAL | - | tested (771) |
| parse_tags | 357-391 | REAL | - | walk advances strictly; unknown-size child refused; count guard on declared records |
| parse_tag | 396-427 | REAL | S4 | F-004 multi-Targets last-wins; degenerate drop documented |
| parse_targets | 432-480 | REAL | S4 | F-005 try_from; len>8 refused; tolerant TargetTypeValue |
| parse_simple_tag | 485-583 | REAL | S2 | F-001 name/language bytes uncounted; depth+count guards correct |
| read_text | 588-600 | REAL | - | per-value cap; None on invalid UTF-8 |
| text_from_padded | 605-608 | REAL | S3 | F-003 fabricated "" on invalid UTF-8 |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| targets_globally_scope_the_uid_zero_form | YES | asserts global/wildcard/scoped discrimination |
| text_values_strip_nul_padding_like_ffmpeg | YES | but codifies F-003's "" (assertion contradicts its own comment) |
| default_targets_are_track_level | YES | trivially |
| global_text_finds_names_case_insensitively | YES | mixed-case + non-global exclusion asserted |
| budget_error_is_typed_resource_limit | YES | checks stage/code/kind fields |
| global_and_track_pairs_shape_the_differential | YES | binary exclusion + scoping asserted |
| (integration) tag_bomb_ceilings_are_typed_resource_limits | YES | MAX_TAGS+1 and 2 MiB TagString actually refused — real byte fixtures |
| (integration) over_deep_simple_tag_nesting... | YES | guard at exactly MAX_TAG_NESTING, both error and ok side |
Note: the two S2 gaps (F-001/F-002) have NO test — no fixture exercises
MAX_TAG_LIST_BYTES at all, nor repeated Tags masters.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The demuxer's open() walk parses every Tags element and retains the merged list" | tags.rs:44-50 | REAL | mkv/mod.rs:398,423,472-473 |
| "Memory is bounded (MP §12): tag count, simple-tag count, per-value ceilings, cumulative byte budget" | tags.rs:34-39 | FALSE per-element only | F-001/F-002: names uncounted; budgets reset per element |
| "Invalid UTF-8 never fabricates a value" (comment) | tags.rs:645-646 | FALSE | returns `""` (a fabricated empty value), F-003 |
| "Unknown TargetTypeValue numbers are tolerantly recorded" | tags.rs:31-33 | REAL | parse_targets:448-457 + integration test unknown_target_type_value_is_tolerantly_recorded |

## 7. Dead / duplicated / unreachable code
- `usize::try_from` error arms (446, 520): dead on 64-bit release lane (F-005).
- No unreachable!/todo!/panic! in this file (rg-verified: no matches).

## 8. Dependencies & flags
Depends on `media_types` (errors) and sibling `mkv::ebml`/`mkv::error`. No
feature gates; module compiled in the plain `demux` crate used by apps/cli
(apps/cli/Cargo.toml:53). No #[allow] in file.

## 9. Open questions for the next agent
- Does the MKV fuzz zone (mkv/fuzz.rs) ever exercise repeated `Tags` masters
  or >8 MiB of names? (F-002 multiplication — see my audit of fuzz.rs.)
- Do chapters/attachments share the same per-element-budget reset pattern in
  their collect_* twins (mkv/mod.rs:480-494)?
