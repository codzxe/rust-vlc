# AUDIT: crates/demux/src/mkv/tags.rs

| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-13 |
| Lines / functions | 774 / 18 (incl. 6 tests) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mkv.rs → demux::mkv::MkvDemuxer `open()` (crates/demux/src/mkv/mod.rs:340) → `collect_tags` (mod.rs:471-475, call sites mod.rs:398 & 423) → `tags::parse_tags` (this file, line 357). `crates/demux/src/lib.rs:50` (`pub mod mkv`) exports the module. |
| Findings | S0:0 S1:0 S2:2 S3:2 S4:3 |
| Commands run | `Read` full file; `sed -n '20,39p;440,460p;520,532p;594,600p' tags.rs` (quote verification, exit 0); `rg` for `parse_tags\|tags_for_track\|global_text\|track_text_pairs` across repo (exit 0); `sed -n '380,414p;95,120p' tests/mkv_editions_tags.rs` (exit 0). No cargo (adaptation: run-dependent claims cite W00 baseline or UNVERIFIED). |

## 1. One-paragraph verdict

This file implements Matroska `Tags` parsing (Tag/Targets/SimpleTag, nesting, UID scoping, global/track queries) and the implementation is genuine: typed errors, depth guard, count/value ceilings, NUL-stripping, UTF-8 tolerance. It is reachable from the CLI's mkv host and its parse path is exercised by hand-built byte fixtures in `tests/mkv_editions_tags.rs`. But two of its three documented hardening invariants are not what the comments claim: (1) the cumulative memory budget counts only `TagString`/`TagBinary` bytes — `TagName`/`TagLanguage`/`TagLanguageIETF`/`TargetType` bytes are never budgeted, so retention is file-size-proportional, not budget-bounded, and repeated `Tags` masters accumulate across the segment with no total cap; (2) child reads and child walks are bounded by each child's *own* declared end, never by the parent's effective end — the doc's "every child read is clamped to its parent's effective end" is false, letting a lying child drag the walk to stream end (seek+12-byte-read per element) and source tag values from bytes beyond the `Tags` element. Verdict REAL-UNPROVEN: not executed by me (no cargo); the W00 baseline (`tests 1751 pass / 2 fail / 25 ignored`) covers the crate but I cannot attribute specific passes to this file.

## 2. What actually works (verified)

- Vint/element walk arithmetic verified by reading `ebml.rs`: `declared_end()` uses `checked_add` (ebml.rs:422), `pos = child_end` strictly increases (header ≥ 2 bytes), so the walk terminates. Panicking-call census of the parse path (lines 350-608, read line by line): zero `unwrap()`/`expect()`/direct indexing on untrusted data; the only fallible-form calls are non-panicking (`unwrap_or`, `unwrap_or_default`, `ok_or_else`). Verified: zero panicking calls in the parse path.
- The count guards are real and tested at integration level: `tests/mkv_editions_tags.rs:380-413` (`tag_bomb_ceilings_are_typed_resource_limits`) builds 4097 empty `Tag` records and a 2 MiB `TagString` and asserts the typed `ResourceLimit` — verified by `sed -n '380,414p'` (exit 0). These tests would fail if the guards were removed (the error would become `Ok`).
- Reachability chain verified by grep: mod.rs:398/423/472 call `collect_tags`/`parse_tags`; `global_text` is consumed in production at mod.rs:253 (the TITLE fallback).

## 3. Findings

### F-001 [S2] Cumulative budget omits all name/language bytes — "Memory is bounded" is false as stated
Where: crates/demux/src/mkv/tags.rs:588-600 (and budget adds at 545-551, 559-565)
Code:
```rust
fn read_text(
    reader: &mut EbmlReader,
    child: &ElementHeader,
    end: u64,
    cap: u64,
) -> Result<Option<String>, EngineError> {
    if child.size > cap {
        return Err(tag_budget(cap, child.size));
    }
```
What is wrong: `budget.value_bytes` is incremented only in the `ID_TAG_STRING` (545-551) and `ID_TAG_BINARY` (559-565) arms. `read_text` — used for `TagName`, `TagLanguage`, `TagLanguageIETF` (lines 522-532) and `TargetType` (460) — never touches the budget. The header claims (lines 34-36): "Memory is bounded (MP §12): tag count, simple-tag count, per-value size and the cumulative value budget all carry typed `ResourceLimit` refusals".
Why it matters: within one `Tags` element, up to 16,384 `SimpleTag`s can each retain a ~1 MiB name plus ~1 MiB language plus ~1 MiB IETF language (all individually under `MAX_TAG_TEXT_BYTES`), i.e. tens of GiB of retained `String`s with `value_bytes == 0` — the bomb guard never fires. There is no multiplication of file size (the bytes must physically exist in the stream), but retention is proportional to the tags region with no ceiling, and `MkvDemuxer::collect_tags` (mod.rs:473 `self.tags.tags.extend(more.tags)`) accumulates across repeated `Tags` masters with no segment-level total, so the per-element caps do not compose. A hostile multi-GiB (or simply gigantic legitimate) tags section is fully materialized in RAM before playback starts.
What correct looks like: `read_text` takes `&mut TagBudget` and adds `bytes.len()` to `value_bytes` against `MAX_TAG_LIST_BYTES`; `collect_tags` enforces a segment-wide tag/byte total.
Confidence: HIGH — Verification: read (budget flow traced through every arm; `rg budget.value_bytes` shows only two increment sites).

### F-002 [S2] Child reads are never clamped to the parent's end — the documented clamp discipline does not exist below the top level
Where: crates/demux/src/mkv/tags.rs:382 (same pattern 413, 415, 452, 460, 466, 524, 527, 531, 544, 558)
Code:
```rust
        if child.id == ID_TAG {
            declared += 1;
            if declared > MAX_TAGS {
                return Err(tag_budget(MAX_TAGS as u64, declared as u64));
            }
            if let Some(tag) = parse_tag(reader, &child, child_end, &mut budget)? {
```
What is wrong: the header states (lines 21-26): "Declared sizes are hints: every child read is clamped to its parent's effective end and the stream's physical end; ... a child crossing its parent's declared end is the parent's contained lie — tolerated, the walk stops at the parent bound." The code passes `child_end` — the child's *own* declared end, unclamped — into `parse_tag`, `parse_targets`, `parse_simple_tag`, `read_text`, and every `read_exact_at` (e.g. line 544 `reader.read_exact_at(len, child.data_pos, child_end)?`). Only the `while pos < end` loop conditions and `parse_header_at`'s window see the parent bound.
Why it matters: (a) a `Tag`/`SimpleTag` declaring a huge (non-all-ones) size makes its child walk continue to the stream end — every iteration is a `seek` + ≤12-byte read (`parse_header_at`, ebml.rs:450-451), so a lying metadata element converts the rest of a multi-GiB file into millions of tiny positional reads before the first frame is served (count ceilings don't bound this: unknown children are skipped uncounted). (b) a `TagString`/`TagName` child whose declared end crosses the `Tags` element is read from bytes *after* the metadata section — cluster/block data can be absorbed as a tag value (bounded by the 1 MiB per-value and 8 MiB list caps), misattributing payload as metadata. The stream-length clamp in `read_exact_at` (ebml.rs:385) keeps this memory-safe, but the file's own invariant is false.
What correct looks like: `child_end.min(end)` at every descent (the clamp `parse_tags` already applies to itself at line 362).
Confidence: HIGH — Verification: read (every call site traced; quotes verified with `sed -n '520,532p'`).

### F-003 [S3] One oversized known field aborts the whole file open — inconsistent with the adjacent tolerance
Where: crates/demux/src/mkv/tags.rs:594-596 (fatal) vs 599 (tolerant), consumed at mod.rs:472
Code:
```rust
    if child.size > cap {
        return Err(tag_budget(cap, child.size));
    }
    let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
    let bytes = reader.read_exact_at(len, child.data_pos, end)?;
    Ok(ebml::utf8_from(&bytes))
```
What is wrong: an invalid-UTF-8 name is skipped (`Ok(None)` → field keeps default, tag survives), but the very same field one byte over `MAX_TAG_TEXT_BYTES` returns `Err`, which `collect_tags` (mod.rs:472 `tags::parse_tags(...)?`) propagates through `open()` — the entire MKV, including perfectly valid tracks, becomes unopenable because of one metadata field.
Why it matters: the header advertises "Unknown children are skipped by declared size (never fatal)" (line 27) — true only for unknown IDs; known children are fatal. VLC-class behavior is to drop the metadata, not the file. Blast radius is every playback of that file.
What correct looks like: cap overflow → skip the field (return `Ok(None)`), keep the walk; reserve `Err` for structural lies.
Confidence: HIGH — Verification: read (mod.rs:471-475 `?` chain traced).

### F-004 [S3] The file's own tests never touch the byte-level parse path
Where: crates/demux/src/mkv/tags.rs:610-773
Code:
```rust
    #[test]
    fn text_values_strip_nul_padding_like_ffmpeg() {
        assert_eq!(
            text_from_padded(b"00:00:02.104000000\0"),
            "00:00:02.104000000"
        );
```
What is wrong: all six in-file tests exercise the model (`TagList`/`TagTargets` queries), `text_from_padded`, and the error constructor. None calls `parse_tags`/`parse_tag`/`parse_targets`/`parse_simple_tag` with bytes. Deleting lines 350-608 (the entire parser) leaves every in-file test green.
Why it matters: the in-file suite proves nothing about the parser; the real coverage lives in `tests/mkv_editions_tags.rs` (hand-built fixtures, `open_tags` at line 102) — synthetic blobs, not real media (see that file's audit by its owner).
What correct looks like: at least one in-file round-trip `bytes → parse_tags → TagList` test.
Confidence: HIGH — Verification: read (all 6 test bodies).
Mitigation verified: `tests/mkv_editions_tags.rs:380-413` does drive `parse_tags` over crafted bytes and would fail if the guards/walk regressed (`sed -n '380,414p'`, exit 0).

### F-005 [S4] Unknown-size policy is inconsistent between levels
Where: crates/demux/src/mkv/tags.rs:373-375 (refusal) vs mod.rs:398 (acceptance)
Code:
```rust
        let Some(child_end) = child.declared_end() else {
            // Unknown-size inside Tags: not the streaming form's scope.
            return Err(error::bad_structure(child.data_pos));
        };
```
What is wrong: an unknown-size `Tags` master inside `Segment` is accepted (mod.rs:398 routes it to `collect_tags`), but an unknown-size `Tag` — also a master element — inside `Tags` is refused. ebml.rs's own doc (lines 15-17) says unknown sizes are "legal **only for master elements**", implying masters may carry them.
Why it matters: the tolerance rule is scope-dependent without a spec basis stated; a streaming-muxed file that writes unknown-size `Tag` masters (legal per that doc) fails to open.
What correct looks like: either clamp unknown-size children to the parent bound everywhere, or refuse everywhere; document the choice.
Confidence: MEDIUM — Verification: read (both call sites); spec-conformance of each choice UNVERIFIED (no spec text in repo beyond these comments).

### F-006 [S4] Corpus claims in the header are unverifiable from this file
Where: crates/demux/src/mkv/tags.rs:37-39
Code:
```rust
//!   value size and the cumulative value budget all carry typed
//!   `ResourceLimit` refusals — the tag-bomb twin of the cue-table
//!   ceiling. The corpus's own meta-bomb (0017) carries 5 simple tags
//!   totalling ~47 KiB (a 32 KiB `LYRICS`); the budget is set with
//!   an order of headroom above that.
```
What is wrong / why it matters: "corpus 0017" facts (5 tags, ~47 KiB, 32 KiB LYRICS) are asserted as calibration evidence for the ceilings; I could not run the corpus harness (no cargo) and did not locate the 0017 fixture bytes in this audit (the corpus lives under a generated directory; see `tools/corpus/`). UNVERIFIED — if the numbers are wrong the "order of headroom" justification is fiction. Also note the claimed budget headroom is exactly what F-001 shows to be leaky.
What correct looks like: a pointer to the fixture path + a test asserting the claimed counts.
Confidence: LOW — Verification: UNVERIFIED (tool missing: cargo/corpus runner).

### F-007 [S4] Production consumers: only `global_text`; the rest of the parsed metadata is retained for nobody
Where: crates/demux/src/mkv/tags.rs:296-336; consumer grep result
Code:
```rust
    pub fn global_text_pairs(&self) -> Vec<(String, String)> {
```
What is wrong: repo-wide grep (exit 0) shows `global_text` consumed once in production (mod.rs:253, TITLE fallback); `global_text_pairs`, `track_text_pairs`, `tags_for_track` are called only from test files (`tests/mkv_editions_tags.rs`, `tests/mkv_extras_wiring.rs`). The demuxer pays the full parse+retention cost (F-001) for track-scoped metadata no shipped entry point ever surfaces.
Why it matters: dead-ish public API on a paying path; the "wired, issue #34" seam claim (lines 44-50) is true for the title only.
What correct looks like: either surface stream tags in the CLI/UI or don't retain them.
Confidence: HIGH — Verification: read + repo grep.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tag_budget` | 140-150 | REAL | — | Typed error constructor; correct; tested (708-722). |
| `TagValue::as_text` | 170-175 | REAL | — | Trivial match; correct. |
| `SimpleTag::text_pair` | 200-202 | REAL | — | Trivial; correct. |
| `TagTargets::default` (impl Default) | 223-234 | REAL | — | target_type_value 50 per spec; matches test 650-652. |
| `TagTargets::is_global` | 241-246 | REAL | S4 | Empty lists = global; ffmpeg-rule equivalence UNVERIFIED. O(uids) per call, called per query — fine. |
| `TagTargets::targets_track` | 251-253 | REAL | — | UID-0 wildcard; correct; tested. |
| `TagList::global_text` | 279-290 | REAL | — | Case-insensitive; top-level pairs only (nested children not surfaced — matches its doc). Production consumer mod.rs:253. |
| `TagList::global_text_pairs` | 296-309 | REAL | S4 | Test-only consumer (F-007). |
| `TagList::track_text_pairs` | 314-327 | REAL | S4 | Test-only consumer (F-007). |
| `TagList::tags_for_track` | 331-336 | REAL | S4 | Test-only consumer (F-007). |
| `parse_tags` | 357-391 | REAL-UNPROVEN | S2 | Top-level walk; clamps its own end (362) but not children's (F-002); unknown-size child refused (F-005); count guard works (integration-tested). |
| `parse_tag` | 396-427 | REAL-UNPROVEN | S2 | Passes unclamped `child_end` down (F-002); degenerate records skipped but counted. |
| `parse_targets` | 432-480 | REAL-UNPROVEN | S2 | `len > 8` guards before `read_exact_at` — good; reads bounded by child's own end (F-002); `uint_from_be(&[]).unwrap_or(DEFAULT)` handles zero-length. |
| `parse_simple_tag` | 485-583 | REAL-UNPROVEN | S2 | Depth guard 492-496 and count guard 497-503 real and tested; name/language bytes unbudgeted (F-001); over-cap known field fatal (F-003). |
| `read_text` | 588-600 | REAL-UNPROVEN | S2 | Per-value cap only; no cumulative accounting (F-001); UTF-8 failure tolerant. |
| `text_from_padded` | 605-608 | REAL | — | NUL-strip + invalid-UTF-8→""; matches ffmpeg reader claim UNVERIFIED; tested in-file. |
| `tests::targets_globally_scope_the_uid_zero_form` | 615-635 | REAL | — | Would fail on regression; model-only. |
| `tests::text_values_strip_nul_padding_like_ffmpeg` | 638-647 | REAL | — | Would fail on regression. |
| `tests::default_targets_are_track_level` | 650-652 | REAL | — | Asserts constant 50. |
| `tests::global_text_finds_names_case_insensitively` | 655-705 | REAL | — | Would fail on regression; model-only. |
| `tests::budget_error_is_typed_resource_limit` | 708-722 | REAL | — | Asserts error shape only. |
| `tests::global_and_track_pairs_shape_the_differential` | 725-772 | REAL | — | Model-only; parser untouched (F-004). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| targets_globally_scope_the_uid_zero_form (615) | YES for `is_global`/`targets_track` | Direct assertions on scoping logic. |
| text_values_strip_nul_padding_like_ffmpeg (638) | YES | Byte-level assertions on `text_from_padded`. |
| default_targets_are_track_level (650) | YES | Constant check. |
| global_text_finds_names_case_insensitively (655) | YES | Lookup logic. |
| budget_error_is_typed_resource_limit (708) | NO (for the parser) | Only the error constructor is exercised. |
| global_and_track_pairs_shape_the_differential (725) | YES for queries, NO for parser | Model-only. |
| Whole in-file suite vs a deleted parser | NO | No test calls `parse_tags` et al. (F-004). The byte-level coverage is in tests/mkv_editions_tags.rs (synthetic hand-built buffers, not real media). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every child read is clamped to its parent's effective end" | tags.rs:21-23 | FALSE | Child walks/reads use `child_end` unclamped (382, 413, 415, 452, 460, 466, 524-531, 544, 558); only loop conditions see the parent end. F-002. |
| "Memory is bounded (MP §12) ... the cumulative value budget" | tags.rs:34-36 | FALSE (as an end-to-end bound) | Name/language/IETF/TargetType bytes never enter `value_bytes` (588-600); repeated `Tags` masters accumulate uncapped (mod.rs:473). F-001. |
| "Unknown children are skipped by declared size (never fatal)" | tags.rs:27 | SHALLOW | True only for unknown IDs; known children over cap are fatal to the whole open (594-596 + mod.rs:472). F-003. |
| "The demuxer's open() walk parses every Tags element and retains the merged list ... the seam is closed" | tags.rs:44-50 | REAL-UNPROVEN | Call chain verified by grep (mod.rs:398/423/472); not executed. |
| "corpus 0017 carries 5 simple tags totalling ~47 KiB" | tags.rs:37-39 | UNVERIFIED | No cargo/corpus run possible (adaptation); fixture not read. F-006. |
| "the value ends at the first NUL, matching ffmpeg's reader exactly" | tags.rs:40-42, 602-604 | REAL-UNPROVEN | Behavior verified in code+test; ffmpeg equivalence not run. |

## 7. Dead / duplicated / unreachable code

- `global_text_pairs` / `track_text_pairs` / `tags_for_track`: no production caller (grep, exit 0) — test-only public API (F-007).
- No duplication found within the file; the three walk loops (`parse_tags`/`parse_tag`/`parse_simple_tag`/`parse_targets`) are near-identical four times over — a candidate for one shared bounded-walk helper (S4 hygiene; the copy-paste is why the F-002 clamp bug is repeated at four levels).

## 8. Dependencies & flags

- Depends on `media_types` (EngineError taxonomy), `plugin_api::SeekOrigin` (via ebml), `crate::mkv::ebml`, `crate::mkv::error`. No feature flags gate this file; `crates/demux/src/lib.rs:50` unconditionally exports `mkv`. No cfg-gated dead paths. No unsafe.

## 9. Open questions for the next agent

- `tests/mkv_editions_tags.rs` and `tests/mkv_extras_wiring.rs` (owners: whoever holds them): do the corpus-backed tests there feed *real* media files through `parse_tags`, or only hand-built buffers? (I saw only hand-built helpers at lines 44-118.)
- `mkv/mod.rs` owner: is there any segment-level cap on `self.tags.tags` growth across repeated `Tags` masters? I found none (471-475).
- Corpus owner: verify fixture 0017's tag counts (F-006).
