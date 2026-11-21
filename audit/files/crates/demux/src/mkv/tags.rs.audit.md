# AUDIT: crates/demux/src/mkv/tags.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-21 |
| Lines / functions | 774 / 16 (+6 tests, 24 consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/src/containers/mod.rs:57` `"matroska" => mkv::open` → `apps/cli/src/containers/mkv.rs:147-148` `MkvDemuxer::new` + `start()` → `crates/demux/src/mkv/mod.rs:423` `tags::ID_TAGS => self.collect_tags(&child)?` → `mod.rs:472` `tags::parse_tags`). Parsed output itself has NO shipped consumer (F-005). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 1 |
| Commands run | `rg -n "parse_tags\|ID_TAGS\|TagList\|global_text..."` (callers: mod.rs:65,126,253,398,423,472; tests only beyond); `rg -n "MkvDemuxer" crates apps` (CLI wiring at apps/cli/src/containers/{mod,mkv}.rs); `rg -n "\.title\(\)\|tags\(\)" --glob '!crates/demux'` (no shipped consumer); `sed -n` on every quoted range; baseline: `rg -n "mkv::tags\|mkv_editions_tags\|mkv_extras_wiring" audit/runs/baseline/test-workspace.log.txt` → all pass. |

## 1. One-paragraph verdict
Real, reachable, tested Matroska `Tags` parser with a genuinely defended structure (count caps, nesting guard, per-value caps, typed errors) — but its headline memory guarantee is only half-enforced: the "cumulative value budget" counts `TagString`/`TagBinary` bytes only, so `TagName`/`TagLanguage`/`TagLanguageIETF`/`TargetType` text (each up to 1 MiB × 16,384 simple tags) bypasses the documented 8 MiB ceiling entirely, and UID lists have no count ceiling at all. Two dead `unwrap_or` fallbacks advertise an EBML-default rule the code does not implement (zero-length `TargetTypeValue`/`TagDefault` read as 0). And the parsed metadata is retained on the demuxer with no shipped consumer — parsed on every MKV open, then dropped.

## 2. What actually works (verified)
— `parse_tags` wired into the demuxer's `open()` walk: `mod.rs:471-475` `collect_tags` calls `tags::parse_tags` and extends the retained list (verified by read).
— Unit + integration suites executed and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1799-1804` (6 `mkv::tags::tests` … ok), `:2146-2164` (`tests/mkv_editions_tags.rs` … "14 passed", incl. `tag_bomb_ceilings_are_typed_resource_limits`, `over_deep_simple_tag_nesting_is_the_typed_structure_error`, `truncated_simple_tag_is_the_typed_truncation`, `corpus_tags_match_ffprobe_format_and_stream_scopes`); same suites in `orchrun-test-workspace-nofailfast.log.txt:1761` — parse paths exercised on real bytes, not just the model.
— `mkv_extras_wiring.rs` drives the full demuxer (`MkvDemuxer::new` + `start`) on hand-built bytes and asserts `demuxer.tags().global_text_pairs()` (lines 331-339) — a mutation breaking `parse_tags` fails it.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mkv/tags.rs:540-551 vs 588-600 (claim at 35-38, 123-126)
Code:
```rust
            ID_TAG_STRING => {
                if child.size > MAX_TAG_TEXT_BYTES {
                    return Err(tag_budget(MAX_TAG_TEXT_BYTES, child.size));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                budget.value_bytes = budget
```
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
What is wrong: `budget.value_bytes` is incremented only in the `ID_TAG_STRING` (540-551) and `ID_TAG_BINARY` (554-565) arms. `read_text` — used for `TagName`, `TagLanguage`, `TagLanguageIETF` (522-532) and `TargetType` (459-461) — enforces only the 1 MiB per-value cap and never touches the cumulative budget.
Why it matters: the module doc (35-38) claims "Memory is bounded (MP §12): tag count, simple-tag count, per-value size and the cumulative value budget all carry typed ResourceLimit refusals", and `MAX_TAG_LIST_BYTES`'s own doc (123-126) says the 8 MiB budget "bounds a many-medium-values bomb the per-value ceiling alone cannot". A hostile-but-well-formed file with 16,384 `SimpleTag`s each carrying ≤1 MiB `TagName` (+1 MiB language +1 MiB IETF) holds up to ~48 GiB of retained `String`s without ever tripping the 8 MiB guard (allocation stays file-proportional, so this is amplification-defect, not unbounded-from-nothing). The stated bomb defense is defeated by writing the bomb into names instead of values.
What correct looks like: `read_text` takes `&mut TagBudget` and adds `child.size` to `value_bytes` (with the same checked_add + refusal), or the doc downgrades its claim to "TagString/TagBinary bytes only".
Confidence: HIGH — Verification: read (all four call sites of `read_text` and both increment sites traced).

### F-002 [S3]
Where: crates/demux/src/mkv/tags.rs:462-473
Code:
```rust
            ID_TAG_TRACK_UID | ID_TAG_EDITION_UID | ID_TAG_CHAPTER_UID | ID_TAG_ATTACHMENT_UID => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                let uid = ebml::uint_from_be(&bytes).unwrap_or(0);
                match child.id {
                    ID_TAG_TRACK_UID => out.track_uids.push(uid),
```
What is wrong: the four UID lists grow one `push` per element with no count ceiling; none of the documented guards (tag count, simple-tag count, value budget) covers them.
Why it matters: contradicts the "Memory is bounded" house claim (tags.rs:35-38; crate rule `lib.rs:39-41` "bounded memory … declared sizes are ceiling-checked before they are trusted into an allocation"). A `Targets` master padded with millions of 1-byte `TagTrackUID` entries grows the `Vec<u64>` ~8× the payload bytes with no typed refusal. Blast radius limited (stream-proportional, ~2× file size), hence S3 not S2.
What correct looks like: a `MAX_TARGET_UIDS`-style ceiling per list with the typed `tag_budget` refusal, matching the cue-table pattern the doc cites.
Confidence: HIGH — Verification: read.

### F-003 [S3]
Where: crates/demux/src/mkv/tags.rs:448-457 and 533-538 (helper at crates/demux/src/mkv/ebml.rs:492-501)
Code:
```rust
                // Tolerant: any value (or a degenerate zero-length
                // field, which the EBML default rule reads as the
                // default) is recorded.
                out.target_type_value =
                    ebml::uint_from_be(&bytes).unwrap_or(DEFAULT_TARGET_TYPE_VALUE);
```
```rust
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                out.default = ebml::uint_from_be(&bytes).unwrap_or(1) != 0;
```
What is wrong: `uint_from_be(&[])` returns `Some(0)`, not `None` (ebml.rs:492-501 — `None` only when `len > 8`, which both call sites pre-reject at 449/534). So a zero-length `TargetTypeValue` parses as 0, not the spec default 50, and a zero-length `TagDefault` parses as `false`, not the spec default `true` (Matroska: absent/empty element = default value). The comment's claim ("the EBML default rule reads as the default") is false, and both `unwrap_or(...)` fallbacks are dead code.
Why it matters: comment contradicts code on a spec-compliance path; the fallback constants advertise behavior that cannot execute. Degenerate input is rare in the wild, so limited blast radius.
What correct looks like: `if bytes.is_empty() { DEFAULT_TARGET_TYPE_VALUE } else { uint_from_be(&bytes).unwrap_or(...) }` (same for `TagDefault` → `true`), or fix the comment and delete the dead fallbacks.
Confidence: HIGH — Verification: read of both call sites + `uint_from_be` body.
Mutation check: no test feeds a zero-length `TargetTypeValue`/`TagDefault` (the integration suite's `unknown_target_type_value_is_tolerantly_recorded` uses non-empty payloads — mkv_editions_tags.rs, baseline log :2158) — the bug is uncaught.

### F-004 [S4]
Where: crates/demux/src/mkv/tags.rs:605-608 (policy contrast at 585-587)
Code:
```rust
fn text_from_padded(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    ebml::utf8_from(&bytes[..end]).unwrap_or_default()
}
```
What is wrong: invalid-UTF-8 `TagString` becomes `TagValue::Text("")` — a fabricated empty value that still counts as "has value" (579-582) and surfaces as `NAME=""` in `global_text_pairs`/`track_text_pairs`. `read_text` (585-587: "kept as absent, never fabricated") does the opposite for names/languages. Inconsistent tolerance policy; also a `SimpleTag` carrying both `TagString` and `TagBinary` silently keeps the last one.
Why it matters: doc says "never fabricated" for one arm while its sibling fabricates; `NAME=""` pairs leak into the ffprobe-differential shape. Cosmetic-to-minor.
What correct looks like: `TagValue::None` when the value bytes are not valid UTF-8 (drop to the absent form), matching `read_text`.
Confidence: HIGH — Verification: read; unit test at 638-647 pins the current `""` behavior (`text_from_padded(&[0xFF, 0xFE])` → `""`), so it is deliberate but undocumented as a fabrication.

### F-005 [S3]
Where: crates/demux/src/mkv/mod.rs:471-475 (seam) + apps/cli/src/containers/mkv.rs:147-149 (open) / 195-233 (harvest, no tags)
Code:
```rust
    fn collect_tags(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = tags::parse_tags(&mut self.reader, child, self.segment_end)?;
        self.tags.tags.extend(more.tags);
        Ok(())
    }
```
```rust
    let mut demuxer = MkvDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
```
What is wrong: `MkvDemuxer::tags()`/`title()` (title uses `tags.global_text("TITLE")`, mod.rs:253) and `TagList::track_text_pairs`/`tags_for_track` have zero callers outside `crates/demux/tests/*` (`rg "\.title\(\)|tags\(\)"` over apps/ + crates/ minus demux → docs and demux tests only). The CLI's `harvest_facts` (apps/cli/src/containers/mkv.rs:195-233) reads `track_entries` only; no title/tag fact is minted, `MediaFacts` carries none.
Why it matters: tags.rs's own seam doc (44-50) says "the 5.D2b library-only seam are closed", but the closure stops at an accessor no shipped build reads — every MKV open pays the parse cost and drops the result. Matrix row `container-matroska-webm` evidence leans on "issue #34 open() wiring/TrackUID/Title retention" — retention is real, surfacing is not.
What correct looks like: either a `MediaFacts` title/metadata field populated in `harvest_facts` (CLI `open` output), or docs stating the consumer is future work (the chapter-nav row does this honestly for chapters).
Confidence: HIGH — Verification: rg caller sweep + read of harvest_facts. Parse itself still runs in shipped opens (real code), so this is dead-end-data, not fake code.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tag_budget` | 140-150 | REAL | — | typed ResourceLimit constructor; verified by unit test :708-722 |
| `TagValue::as_text` | 170-175 | REAL | — | trivial, correct |
| `SimpleTag::text_pair` | 200-202 | REAL | — | trivial, correct |
| `TagTargets::default` | 224-234 | REAL | — | correct (50); zero-length-element default NOT honored (F-003) |
| `TagTargets::is_global` | 241-246 | REAL | — | all-zero/empty = global; unit-tested :615-635 |
| `TagTargets::targets_track` | 251-253 | REAL | — | 0-wildcard; correct |
| `TagList::global_text` | 279-290 | REAL | — | case-insensitive; only caller mod.rs:253 (+tests) |
| `TagList::global_text_pairs` | 296-309 | REAL | — | top-level pairs only (nested children not flattened — matches ffprobe shape per tests) |
| `TagList::track_text_pairs` | 314-327 | REAL | — | consumers: tests only (F-005) |
| `TagList::tags_for_track` | 331-336 | REAL | — | consumers: tests only (F-005) |
| `parse_tags` | 357-391 | REAL | — | `declared` counts degenerate `Tag` records (bomb-safe, :366-381); unknown-size child → typed bad_structure; loop strictly advances (child_end > pos); merge seam mod.rs:472 |
| `parse_tag` | 396-427 | REAL | — | repeated `Targets` masters: last wins silently (spec: one) — S4-grade, not separately filed |
| `parse_targets` | 432-480 | REAL | F-002, F-003 | UID lists uncounted; zero-length value reads as 0 |
| `parse_simple_tag` | 485-583 | REAL | F-001 | nesting guard 492-496 correct and integration-tested; budget check 497-503 correct for counts; value_bytes gap F-001; degenerate `Ok(None)` skip 579-582 |
| `read_text` | 588-600 | REAL | F-001 | per-value cap only; invalid UTF-8 → None (honest) |
| `text_from_padded` | 605-608 | REAL | F-004 | NUL-strip matches ffmpeg reader (unit-tested :638-647); invalid UTF-8 → "" |
| consts (17 IDs + 7 ceilings) | 62-130 | REAL | — | element IDs cross-checked against Matroska spec table (Tags 0x1254C367, Tag 0x7373, Targets 0x63C0, TargetTypeValue 0x68CA, TargetType 0x63CA, Tag*UID 0x63C5/0x63C9/0x63C4/0x63C6, SimpleTag 0x67C8, TagName 0x45A3, TagLanguage 0x447A, IETF 0x447B, TagDefault 0x4484, TagString 0x4487, TagBinary 0x4485) — all correct |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `targets_globally_scope_the_uid_zero_form` (615) | YES | pure-model assertions on wildcard/global logic; mutation `any(u==uid)` breaks :629/:634 |
| `text_values_strip_nul_padding_like_ffmpeg` (638) | YES | pins NUL-strip + invalid-UTF-8→"" (the "" is the F-004 behavior, deliberately pinned) |
| `default_targets_are_track_level` (650) | YES (weakly) | asserts 50 only |
| `global_text_finds_names_case_insensitively` (655) | YES | case-insensitive match + scope exclusion asserted |
| `budget_error_is_typed_resource_limit` (708) | YES | pins stage/kind/code/`[demux]` prefix |
| `global_and_track_pairs_shape_the_differential` (725) | YES | binary-not-text, scope filter, empty-uid miss all asserted |
| (integration, this file's parser) `mkv_editions_tags.rs` 14 tests | YES | real EBML bytes through `parse_tags` (helper :106); bomb ceilings, nesting guard, truncation, corpus+ffmpeg differentials (baseline test-workspace.log.txt:2146-2164 all ok) |
| (integration) `mkv_extras_wiring.rs` 9 tests | YES | full demuxer `start()` then `tags()`/`title()` assertions (:336, :368, :475, :696, :906) |
Gap: no test feeds zero-length `TargetTypeValue`/`TagDefault` (F-003 uncaught) or a many-medium-`TagName` bomb (F-001 uncaught).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "full parse, wired through `open()` (issue #34: retained on the demuxer, `tags()`)" | docs/formats/container-matroska.md:28 | REAL (parse+retention) / consumer-less | mod.rs:423→472; F-005 for the missing consumer |
| "Memory is bounded (MP §12): tag count, simple-tag count, per-value size and the cumulative value budget all carry typed ResourceLimit refusals" | tags.rs:35-38 (module doc) | SHALLOW (counts + values yes; names/languages/UID lists no) | F-001, F-002 |
| "Bomb ceilings typed ResourceLimit (demux.mkv.tags.budget): 4096 tags, 16384 simple tags, 1 MiB per value, 8 MiB cumulative" | docs/worklog/archive-2026-10.md:8825 | REAL for counts/values; 8 MiB cumulative only covers TagString/TagBinary | F-001 |
| "the 5.D1 parse-and-skip tolerance and the 5.D2b library-only seam are closed" | tags.rs:44-50 | REAL for parsing; MISLEADING for the seam (no shipped consumer) | F-005 |
| matrix row `container-matroska-webm` status implemented (win/linux/macos) | docs/feature-matrix.toml:129-139 | REAL-UNPROVEN at shipped-binary level (own note: "Linux-only evidence"; no MKV `lumen open` smoke in audit/runs/baseline/INDEX.md — only AVI/MP4 lanes ran) | baseline INDEX #20 (rc-smoke "AVI first-vertical only, by design"); demux+tests proven per §2 |

## 7. Dead / duplicated / unreachable code
- Dead fallbacks: `unwrap_or(DEFAULT_TARGET_TYPE_VALUE)` (456-457) and `unwrap_or(1)` (538) unreachable (F-003).
- Consumer-less surface: `TagList::track_text_pairs`, `tags_for_track`, `MkvDemuxer::tags`, `MkvDemuxer::title` — test-only consumers (F-005).
- No `unsafe`, no `unwrap`/`expect`/`panic!`/`todo!` in the file (house rule kept; verified by read).

## 8. Dependencies & flags
- `media_types` (EngineError/ErrorKind/PipelineStage/LimitKind), `crate::mkv::ebml`, `crate::mkv::error` — std-only, no external crates (crate rule C5 held). No feature gates: module compiled unconditionally (`pub mod mkv` lib.rs:50), so every shipped build (incl. the `lumen engine-info` build with `first-vertical:avi-mjpeg-pcm`) contains and reaches this code on MKV open. No cfg(test) leakage into prod paths.

## 9. Open questions for the next agent
- mkv/mod.rs (owner: another W05 agent): does the `Segment` walk bound `Tags` masters seen before `first_cluster` (a Tags-only file → `no_clusters` refusal)? Interaction of `collect_tags` merge order with repeated masters.
- Who owns the F-005 consumer gap — CLI `MediaFacts` has no metadata field at all (apps/cli/src/demux_host.rs, unaudited by me).
- `MAX_TAG_LIST_BYTES` accounting fix should ride with the cues.rs ceiling review (same pattern family).
