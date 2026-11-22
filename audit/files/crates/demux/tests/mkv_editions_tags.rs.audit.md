# AUDIT: crates/demux/tests/mkv_editions_tags.rs

| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-22 |
| Lines / functions | 1048 / 36 (14 `#[test]` fns, 12 builders/helpers, 10 differential infra fns) |
| File verdict | SPLIT: hand-built + generated tests REAL; the two advertised corpus differentials are conditional no-ops that passed the recorded baseline without executing (timing-proven) |
| Reachable from shipped entry point? | N/A (test file). Runs under `cargo test -p demux --test mkv_editions_tags`. W00 baseline: `14 passed; 0 failed; 0 ignored; 0.43s` (audit/runs/baseline/cargo-test.log:2104-2122). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:3 |
| Commands run | Read (full); sed -n quote verification; `ls /tmp/my-project/corpus-v4` → **No such file or directory**; `which ffprobe ffmpeg` → both present in this sandbox; forensics on audit/runs/baseline/cargo-test.log (suite timings); rg into docs/formats/container-matroska.md + docs/feature-matrix.toml for who cites this suite. No cargo (adaptation). |

## 1. One-paragraph verdict

The ten hand-built fixtures tests are honest: real EBML bytes, exact-value assertions, typed error kinds and budget codes, boundary tests on both sides of every ceiling — they would fail if `tags.rs`/`editions.rs` regressed, and they passed in the W00 baseline. The ffmpeg-generated lane (`generated_metadata_files_match_ffprobe`) is a real end-to-end differential (mux with ffmpeg → parse with demux → compare with ffprobe) and evidently ran (suite timing is consistent only with the two ffmpeg invocations executing). But the suite's advertised headline — "the **real-corpus differential vs ffprobe**" over the 31 corpus-v4 lanes — is a conditional no-op: when `/tmp/my-project/corpus-v4` is absent the test prints a captured `eprintln!` and **returns Ok**, counting as "passed". Timing forensics on the recorded W00 baseline prove both corpus tests executed zero comparisons (0.43 s for 14 tests including 62+ ffprobe subprocess spawns is impossible; the twin suite mkv_corpus.rs did the same in 0.01 s), and the corpus directory does not exist in this sandbox. So the baseline's "1751 pass" includes this suite's corpus differentials as verified-while-unexecuted, and the record doc's claim that they "skip loudly" is mechanically false — cargo captures the message, and the baseline log contains no SKIP output at all.

## 2. What actually works (verified)

— The 10 hand-built tests assert exact parsed values (uids, flags, languages, NUL-stripping, nested children), exact error kinds (`Corrupt::Truncated`, `Corrupt::InvalidStructure`, `ResourceLimit`) and exact codes (`demux.mkv.truncated`, `demux.mkv.tags.budget`, `demux.mkv.editions.budget`) — read-verified, and W00 baseline shows all 14 green (cargo-test.log:2106-2121).
— Boundary discipline: `over_deep_simple_tag_nesting…` tests both MAX (parses, deepest pair carries its value via a depth walk) and MAX+1 (typed refusal); `tag_bomb_ceilings…` value-bomb declares a 2 MiB `TagString` **without payload bytes**, proving the ceiling refuses on the declared size before any read (lines 394-407).
— `generated_metadata_files_match_ffprobe` is a genuine oracle differential: real ffmpeg mux with `-metadata`, real ffmetadata chapter injection, then ffprobe cross-checks with recorded-gap exclusions documented in-code (`is_recorded_gap`, lines 732-737). Ran in baseline (timing analysis, §3 F-001).
— The differential exclusions (`title` both scopes, stream `language`) are pinned by name with reasons in both the module doc (16-23) and `is_recorded_gap` — recorded gaps, not silent drops. REAL.

## 3. Findings

### F-001 [S2] — The advertised corpus differential passed the recorded baseline without executing a single comparison
Where: crates/demux/tests/mkv_editions_tags.rs:774-779 (and 856-859, 924-927) + audit/runs/baseline/cargo-test.log:2104-2122
Code (verbatim, 774-779):
```
#[test]
fn corpus_tags_match_ffprobe_format_and_stream_scopes() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
Evidence chain:
1. `/tmp/my-project/corpus-v4` does not exist in this sandbox (`ls` → "No such file or directory"); `LUMEN_CORPUS` is not recorded as set in any baseline file.
2. W00 baseline: this suite `14 passed … finished in 0.43s` (cargo-test.log:2122). If the corpus were present, `corpus_tags_match…` alone spawns ffprobe twice per corpus mkv (~31 files ⇒ 62+ subprocesses, seconds of wall time), and `corpus_editions_match…` spawns more; 0.43 s is only consistent with both corpus tests taking the skip branch while the generated test's two ffmpeg runs execute in parallel.
3. The twin suite `tests/mkv_corpus.rs` — same skip shape — reports `2 passed … finished in 0.01s` (cargo-test.log:2096-2102), which is a no-op signature, not a 31-lane seek/demux sweep.
What is wrong: the module doc's headline (lines 2-6: "the **real-corpus differential vs ffprobe** … 0017-meta-bomb, 0014, 0003, 0007-chapter-bomb, 0063 and the full mkv sweep") describes verification that did not happen in the only recorded run. A pass that executed zero assertions inflated the baseline's "1751 pass".
Why it matters: this suite is the demux crate's tags/editions conformance story; matrix row `container-matroska-webm` (status `implemented` ×3 platforms, docs/feature-matrix.toml:138) leans on the record doc that leans on this differential. "Verified" and "skipped" are indistinguishable in every count the project publishes.
What correct looks like: `#[ignore = "requires corpus-v4 (set LUMEN_CORPUS)"]` so absence shows as `ignored`, plus a CI lane that runs with the corpus present; or an explicit env-gated hard fail.
Confidence: HIGH — Verification: read + baseline-log forensics + filesystem check (per-test cargo attribution UNVERIFIED — no toolchain; the timing argument is arithmetic, not execution)

### F-002 [S3] — The skip is invisible by mechanism, and the record doc's "skip loudly" claim is false
Where: crates/demux/tests/mkv_editions_tags.rs:527-533 + 776-778 + 924-927; docs/formats/container-matroska.md:251-253
Code (verbatim, corpus_dir, 527-533):
```
fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
    let media = dir.join("media");
    media.is_dir().then_some(media)
}
```
Code (verbatim, the doc's characterization, container-matroska.md:251-253):
```
Honest exclusions: corpus-v4 is absent in this session's sandbox, so
the 31-lane corpus differential (mkv_corpus.rs) and the 5.D2b tags
sweep (mkv_editions_tags.rs) skip loudly — their recorded results
stand as recorded, not re-run here.
```
What is wrong: `eprintln!` inside a passing test is captured by the test harness and shown only with `--nocapture` or on failure — the W00 baseline log contains **zero** "SKIP" lines (grep over cargo-test.log). "Skip loudly" is false; the skip is silent in every published count. The same pattern hides the ffmpeg-absent branch of the generated test (line 924-927). The record doc's own gate line ("587 passed / 0 failed / **4 ignored** … the 4 ignores are the absent-corpus skips") describes a mechanism the current tree does not have — this file carries no `#[ignore]`, and the W00 baseline reports `0 ignored` for this suite: the skips count as **passed**, not ignored.
Confidence: HIGH — Verification: read + grep of baseline log + doc cross-read

### F-003 [S4] — Hand-rolled ffprobe-JSON parsing is fragile (loud failure modes)
Where: crates/demux/tests/mkv_editions_tags.rs:626-656 (read_json_string), 713-717 (stream split)
Code (verbatim, 639-645):
```
                let c = match bytes[*i] {
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    other => other,
                };
                out.push(c);
```
Code (verbatim, 713-717):
```
    let mut streams = Vec::new();
    for chunk in json.split("\"index\":").skip(1) {
        streams.push(json_object_after(chunk, "tags"));
```
What is wrong: (a) `\uXXXX` escapes are not decoded — ffprobe emits them for control characters, and they would surface as the literal 4 hex characters, so a corpus tag containing one would make the differential compare wrong strings; (b) the stream split is a raw-text split on `"index":` — a tag *value* containing that literal would mis-split the stream list; (c) `json_object_after` matches the first `"tags":` anywhere in the document. All three fail loudly (assertion mismatch), not silently — hygiene, not a false-pass.
Confidence: HIGH — Verification: read

### F-004 [S4] — `size_vint`'s `unreachable!` and its comment are wrong for the top of the u64 range
Where: crates/demux/tests/mkv_editions_tags.rs:64-68
Code (verbatim):
```
    // 8 octets always suffice for u64 (the marker eats one bit); the
    // loop's last arm returns first.
    unreachable!("u64 fits the 8-octet vint")
```
What is wrong: an 8-octet EBML size vint carries 56 bits (max `0x00FF_FFFF_FFFF_FFFF`); u64 values above that do NOT fit, so the loop can fall through and the `unreachable!` is reachable for `n > 2^56-1`. The comment asserts the opposite. Unreachable in practice (payloads of 2^56 bytes), but the "always suffice" claim is false and the panic is a landmine for future fixture builders.
Confidence: HIGH — Verification: read + arithmetic

### F-005 [S4] — Corpus-internal pins with an unverifiable provenance claim
Where: crates/demux/tests/mkv_editions_tags.rs:860-869, 919
Code (verbatim, 860-862):
```
    // The two known chapter carriers (verified structurally before
    // this suite ran: one default edition each, flat atoms).
    let expectations = [
```
What is wrong: the suite pins corpus filenames, exact UID counts (2580/10) and `assert_eq!(with_chapters, 2, "exactly the two known chapter lanes")` — brittle-but-loud pins, fine in themselves — but the justification "verified structurally before this suite ran" cites a verification event no artifact in this tree records, and when the corpus is absent none of these pins execute at all (F-001).
Confidence: HIGH — Verification: read

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts ID_TRACK_UID / ID_TRACK_ENTRY | 39-41 | REAL | — | correct ids (0x73C5 / 0xAE) |
| id_bytes | 49-53 | REAL | — | shortest-marker form, correct |
| size_vint | 56-68 | REAL | S4 | F-004: 56-bit ceiling + wrong comment |
| el / el_uint / el_str / cat | 71-97 | REAL | — | correct minimal encodings; el_uint(0) → [0x00] |
| open_tags / open_editions | 102-114 | REAL | — | fixture-is-the-element harness; `.must().must()` panics on malformed fixture (test failure, correct) |
| global_and_track_scoped_tags_parse_with_languages | 120-181 | REAL | — | exact values incl. IETF language, defaults, join accessors |
| uid_zero_targets_are_the_global_form | 183-213 | REAL | — | the ffmpeg format-scope rule |
| nested_simple_tags_and_binary_values_parse | 215-271 | REAL | — | nesting, TagBinary passthrough, NUL-strip |
| unknown_target_type_value_is_tolerantly_recorded | 273-299 | REAL | — | tolerant carry, no guess |
| truncated_simple_tag_is_the_typed_truncation | 301-332 | REAL | — | typed kind + code asserted |
| over_deep_simple_tag_nesting… | 334-377 | REAL | — | both sides of the boundary; depth walk |
| tag_bomb_ceilings_are_typed_resource_limits | 379-408 | REAL | — | count bomb + header-only value bomb (pre-read refusal) |
| edition_flags_and_chapter_uid_links_parse | 414-454 | REAL | — | flags, nested UIDs, link seam, default_edition |
| edition_defaults_follow_the_spec_when_absent | 456-474 | REAL | — | spec defaults |
| atom_nesting_depth_guard_is_typed | 476-500 | REAL | — | typed refusal at MAX+1 |
| chapter_bomb_ceiling_is_the_typed_resource_limit | 502-521 | REAL | — | editions.budget code asserted |
| corpus_dir | 527-533 | REAL | S3 | the conditional no-op gate (F-001/F-002) |
| segment_children | 538-573 | REAL | — | tolerant walk; unknown-size → break (test-side OK) |
| track_uids | 577-612 | REAL | — | ≤8 guard, read_exact_at, unwrap_or(0) |
| ffprobe_json | 615-622 | REAL | — | real subprocess; `.must()` panics if ffprobe absent (loud) |
| read_json_string | 626-656 | REAL | S4 | F-003: no \uXXXX decoding |
| json_object_after | 659-700 | REAL | S4 | F-003: first-match key scan |
| ffprobe_format_tags / ffprobe_stream_tags | 703-718 | REAL | S4 | F-003: `"index":` text split |
| sorted | 721-725 | REAL | — | order-insensitive compare |
| is_recorded_gap | 732-737 | REAL | — | documented exclusions, pinned by name |
| parse_file_tags | 739-761 | REAL | — | re-walks via public parsers; real FileAccess I/O |
| open_source | 767-772 | REAL | — | input::FileAccess composition, not mem_stream |
| corpus_tags_match… | 774-852 | NO-OP-IN-BASELINE | S2 | F-001; `checked >= 25` guard never ran |
| corpus_editions_match… | 854-920 | NO-OP-IN-BASELINE | S2 | F-001/F-005 |
| generated_metadata_files_match_ffprobe | 922-1048 | REAL | S3 | ran in baseline (timing); same invisible-skip branch if ffmpeg absent (F-002) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 10 hand-built tests (above) | YES | exact-value / exact-error-kind / exact-budget-code assertions; boundary pairs |
| corpus_tags_match_ffprobe_format_and_stream_scopes | NO — when corpus absent (proven for the W00 baseline) | returns Ok before any assertion; the `checked >= 25` floor is inside the skipped region |
| corpus_editions_match_ffprobe_chapter_counts | NO — when corpus absent | same; also skips per-file if the two named files are missing (lines 872-875) |
| generated_metadata_files_match_ffprobe | YES (when ffmpeg present — it was, per timing) | asserts exact ARTIST/COMMENT pairs, set equality vs ffprobe, chapter counts vs ffprobe; silently passes only if ffmpeg is absent |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the **real-corpus differential vs ffprobe** … and the full mkv sweep" | module doc lines 2-6 | SHALLOW (in the recorded baseline: FALSE as evidence) | F-001: skip branch taken; 0 comparisons executed; timing + absent dir |
| "skip loudly" for the absent corpus | container-matroska.md:251-253 | FALSE | F-002: eprintln is captured; zero SKIP lines in baseline log; counts as "passed", not ignored |
| "the 4 ignores are the absent-corpus skips" (gate record) | container-matroska.md:262-264 | FALSE for the current tree | no `#[ignore]` in this file; baseline shows `0 ignored` for this suite — skips are passes |
| Corpus table (0017: 47 908 B Tags, 2580 chapter links, "all match ffprobe") | container-matroska.md:76-90 | REAL-UNPROVEN | numbers are consistent with the pins in this suite but were recorded in an unrecorded prior session; not reproducible in this sandbox (corpus absent) |
| Excluded ffprobe names "recorded gaps, not silent skips" | module doc 16-23 + is_recorded_gap | REAL | code verified |
| matrix row container-matroska-webm `implemented` ×3 with this suite in the evidence chain | docs/feature-matrix.toml:130-139 | Wave 19's call — inputs: F-001/F-002 (differential unexecuted in baseline; hand-builts real) | filed for the ledger agents |

## 7. Dead / duplicated / unreachable code

- Nothing dead in this file. Duplication note for the crate: this suite exercises `editions.rs` — the repo's SECOND Chapters parser alongside `chapters.rs` (cross-ref my chapters.rs audit F-002 note): two parsers for the same `Chapters` element with different models (UID link list vs full atom tree), both reached from `open()`.
- The skip branches (corpus ×2, ffmpeg ×1) are "reachable" dead code in every environment lacking the external resource.

## 8. Dependencies & flags

Integration test (`tests/`), compiled only for `cargo test`. Uses `demux::avi::test_support::mem_stream` (line 28) — the ungated test-support import that motivates the pattern flagged in my mp4/test_support.rs audit F-001. Dev-deps in play: `testkit` (Must), `input` (FileAccess), `plugin-api`, `media-types`. External tools: ffprobe (hard-required by corpus tests when corpus present — `.must()` panics if missing), ffmpeg (soft-required by the generated test). External data: corpus-v4 (absent).

## 9. Open questions for the next agent

- Wave 16 (test-quality): how many suites tree-wide use the `corpus_dir()`-or-skip pattern, and how many of the W00 baseline's "1751 pass" are resource-absent no-ops? This file proves at least 3 (its own two + mkv_corpus's two); the timing column of cargo-test.log is the cheap detector (sub-0.05s "differential" suites).
- Wave 19: does the container-matroska-webm row's evidence chain survive F-001 (the record doc's corpus table is unreproducible in-tree)?
- mkv_corpus.rs / mkv_attachments_chapters.rs agents: same F-001/F-002 pattern — 0.01 s / 0.18 s suite times in the baseline corroborate the skips.
- Where is corpus-v4 supposed to come from (no fetch script found in-tree by me — UNVERIFIED whether one exists under tools/ or scripts/)? If it is not reproducible from the repo, the corpus table in container-matroska.md is unfalsifiable by any fresh contributor.
