# AUDIT: crates/demux/tests/mkv_editions_tags.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 1048 / 34 (14 #[test] + 20 helpers) |
| File verdict | REAL (corpus differential portion: REAL-UNPROVEN — never ran here) |
| Reachable from shipped entry point? | N/A (test binary) — wired: runs under `cargo test -p demux`; verified live exit 0 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 5 |
| Commands run | `./target/debug/deps/mkv_editions_tags-35df1c0ffd2275fe --test-threads=1 --nocapture` -> 14 passed, exit 0; 2 corpus tests printed "SKIP: corpus-v4 absent"; ffprobe/ffmpeg present at /usr/bin; sed quote verifications (all byte-exact) |

## 1. One-paragraph verdict
Twelve hand-built EBML fixture tests are real, mutation-resistant, and pass
live: they pin exact UID lists, edition flags, typed truncation/structure
errors, depth guards at both sides of the boundary (tags), and bomb ceilings
against real source constants (`MAX_TAGS`=4096, `MAX_TAG_NESTING`=8,
`MAX_CHAPTER_ATOMS`=65536 — verified in tags.rs/editions.rs). The
ffmpeg-generated differential ran live and matched (global + 2 stream scopes,
2 chapter links). But the headline "real-corpus differential vs ffprobe"
claimed in the module header is two tests that silently SKIP (early `return`,
counted as "passed") whenever /tmp/my-project/corpus-v4 is absent — as it is
here and in the orchestrator baseline — so that portion of the suite's
advertised coverage is REAL-UNPROVEN, and `cargo test`'s "14 passed"
overstates what executed. Secondary defects: a mathematically false comment
on `size_vint` (8-octet vints hold 56 bits, not 64), a JSON mini-parser that
ignores `\uXXXX` escapes, and an exit-status-blind ffprobe invocation.

## 2. What actually works (verified)
— verified by live run `--nocapture`: 12 fixture tests + generated
differential pass; output lines "GENERATED TAGS DIFF: meta_title.mkv —
global + 2 stream scopes matched" and "GENERATED EDITIONS DIFF:
meta_chapters.mkv — 1 edition, 2 chapter links". Cross-checked baseline
audit/runs/baseline/cargo-test-workspace.txt:2318-2335 (14 passed, 0.43s —
note: baseline does not show the SKIP either; only --nocapture reveals it).
Bomb/depth guards referenced by tests exist in production code:
crates/demux/src/mkv/tags.rs:100,111 and editions.rs:76,81.

## 3. Findings
F-001 [S2]
Where: crates/demux/tests/mkv_editions_tags.rs:776-779 (same pattern 856-859)
Code:
```
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: the two corpus differential tests pass vacuously when the
corpus directory is absent — the skip is an eprintln invisible in default
`cargo test` output, and libtest counts them as "passed" (live-verified:
"test corpus_tags_match_ffprobe_format_and_stream_scopes ... ok" with a SKIP
line). The module header (lines 2-6) advertises "the real-corpus differential
vs ffprobe ... 0017-meta-bomb, 0014, 0003, 0007-chapter-bomb, 0063 and the
full mkv sweep" as this suite's content; none of that ran in this environment
or in the baseline. Why it matters: green counts overstate coverage; anyone
citing "14 passed" as corpus-parity evidence is citing tests that did
nothing. The repo does record the skip honestly in
docs/formats/container-matroska.md:251-254, but that is a doc, not a gate.
What correct looks like: `#[ignore]` with a reason (visible in the passed/
ignored split), or assert the corpus is present when an env var like
LUMEN_REQUIRE_CORPUS is set (fail-closed in CI), as e.g. corpus.rs's
`#[ignore = "randomized-corpus harness..."]` pattern does.
Confidence: HIGH — Verification: ran (live SKIP output captured).

F-002 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:56-68
Code:
```
fn size_vint(n: u64) -> Vec<u8> {
    for len in 1usize..=8 {
        let max_val = (1u64 << (7 * len)) - 1;
        if n <= max_val {
            let mut out = n.to_be_bytes()[8 - len..].to_vec();
            out[0] |= 1u8 << (8 - len);
            return out;
        }
    }
    // 8 octets always suffice for u64 (the marker eats one bit); the
    // loop's last arm returns first.
    unreachable!("u64 fits the 8-octet vint")
}
```
What is wrong: the comment is mathematically false. An 8-octet vint carries
7×8=56 payload bits (the marker eats one bit — which is exactly why 64 bits
do NOT fit); any n > 2^56−1 falls through the loop into `unreachable!()` and
panics. "The loop's last arm returns first" is wrong for those n. Never
triggered in this file (max n = 2 MiB bomb declaration), but the helper is a
landmine for future fixtures. What correct looks like: debug_assert or a
typed panic message, and delete the false comment.
Confidence: HIGH — Verification: read + arithmetic.

F-003 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:634-646
Code:
```
                let c = match bytes[*i] {
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    other => other,
                };
                out.push(c);
```
What is wrong: `\uXXXX` JSON escapes (ffprobe emits them for control
characters in tag values) are not decoded — the reader would emit the literal
bytes `u0001` etc., so the ffprobe truth side mis-parses any such value.
Failure mode is a false DIFF (test fails), not a false pass, and corpus tags
are expected UTF-8-clean; blast radius is a confusing failure, not wrong
green. What correct looks like: decode \uXXXX (with surrogate pairing) or use
a real JSON crate in dev-deps.
Confidence: MEDIUM — Verification: read (no corpus to construct a
control-char tag; UNVERIFIED end-to-end).

F-004 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:732-737 vs module header 16-23
Code:
```
fn is_recorded_gap(name: &str, scope: char) -> bool {
    match scope {
        'f' => name == "title", // Info.Title (5.D1 element, not retained)
        _ => name == "language" || name == "title", // TrackEntry elements
    }
}
```
What is wrong: the module header says "Two ffprobe-side names are excluded
with reasons" (format `title`, stream `language`) but the code excludes three
combinations — stream `title` (TrackEntry.Name) is also dropped. The helper's
own doc comment (lines 727-731) documents all three; the header undercounts.
A reader trusting the header would expect stream-scope title equality to be
pinned; it is not. What correct looks like: header lists all three recorded
gaps.
Confidence: HIGH — Verification: read (both locations).

F-005 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:615-622
Code:
```
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", entries, "-of", "json"])
        .arg(path)
        .output()
        .must();
    String::from_utf8_lossy(&out.stdout).to_string()
```
What is wrong: the exit status of ffprobe is never checked — a failing
ffprobe yields empty stdout, which the differential then treats as "no
tags". Combined with the `continue` at line 794 (both sides empty), a
systematically broken ffprobe would quietly shrink `checked` until the
`checked >= 25` floor trips (that floor is the only backstop, and it too is
inside the corpus-present branch). The ffmpeg invocations in the generated
test DO assert `status.success()` (lines 955-959, 978-982) — inconsistent.
What correct looks like: assert `out.status.success()` in ffprobe_json.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:477-500 (vs tags boundary test 353-376)
Code:
```
    let err = open_editions(buffer).err().must();
    assert!(
        matches!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { .. }
```
What is wrong: the editions nesting test pins only the ERROR side
(MAX_ATOM_NESTING+1 → typed refusal). The happy boundary — exactly
MAX_ATOM_NESTING levels parse — is not asserted anywhere in this suite, and
editions.rs's own unit tests (mod tests at editions.rs:328) do not cover it
either. An off-by-one guard (rejecting at MAX instead of MAX+1) would pass
all existing tests. The tags side has both boundaries pinned
(over_deep_simple_tag... lines 353-376), showing the authors know the
pattern. What correct looks like: mirror the tags ok-side chain-walk for
atoms at depth MAX_ATOM_NESTING.
Confidence: HIGH — Verification: read + rg of editions.rs tests (only
default_edition + budget_error unit tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| id_bytes | 49-53 | REAL | — | shortest-form id encoding, correct |
| size_vint | 56-68 | REAL | S4 (F-002) | correct for n ≤ 2^56−1; false comment; unreachable! for big u64 |
| el / el_uint / el_str / cat | 71-97 | REAL | — | correct minimal-width builders |
| open_tags / open_editions | 102-114 | REAL | — | parse_header_at + zone parsers; .must().must() (Result<Option>) |
| test global_and_track_scoped_tags_parse_with_languages | 120-181 | REAL | — | exact targets/languages/pairs; track_text_pairs(8) empty-check (negative case) |
| test uid_zero_targets_are_the_global_form | 183-213 | REAL | — | spec UID-0 rule pinned |
| test nested_simple_tags_and_binary_values_parse | 215-271 | REAL | — | nested child, binary passthrough, NUL strip, text-pairs count |
| test unknown_target_type_value_is_tolerantly_recorded | 273-299 | REAL | — | value 99 carried verbatim |
| test truncated_simple_tag_is_the_typed_truncation | 301-332 | REAL | — | cut mid-payload; asserts Corrupt::Truncated + context code |
| test over_deep_simple_tag_nesting_is_the_typed_structure_error | 334-377 | REAL | — | BOTH sides of MAX_TAG_NESTING boundary; depth walk + deepest pair value |
| test tag_bomb_ceilings_are_typed_resource_limits | 379-408 | REAL | — | count bomb + 2 MiB value bomb; ResourceLimit + budget code |
| test edition_flags_and_chapter_uid_links_parse | 414-454 | REAL | — | flags, nested atom UIDs in file order [101,202,203], default/ordered/by_uid |
| test edition_defaults_follow_the_spec_when_absent | 456-474 | REAL | — | absent flags = 0; first-visible selection |
| test atom_nesting_depth_guard_is_typed | 476-500 | REAL | S4 (F-006) | error side only; ok-boundary untested |
| test chapter_bomb_ceiling_is_the_typed_resource_limit | 502-521 | REAL | — | 65537 atoms → ResourceLimit + budget code |
| corpus_dir | 527-533 | REAL | — | env override; /tmp default absent in this sandbox |
| segment_children | 538-573 | REAL | — | test-side segment walker; typed errors on unknown-size top-levels |
| track_uids | 577-612 | REAL | S4 | size≤8 guard; `unwrap_or(0)` swallows read errors (test-side, low risk) |
| ffprobe_json | 615-622 | REAL | S4 (F-005) | exit status ignored |
| read_json_string | 626-656 | REAL | S4 (F-003) | no \uXXXX decode; \\ and \" handled |
| json_object_after | 659-700 | REAL | — | one-level object reader; first-"tags": match could be spoofed by a filename containing `"tags":` (theoretical) |
| ffprobe_format_tags / ffprobe_stream_tags | 703-718 | REAL | — | index+stream_tags split trick is sound |
| sorted | 721-725 | REAL | — | order-insensitive compare, correct (mkv order ≠ ffprobe order) |
| is_recorded_gap | 732-737 | REAL | S4 (F-004) | three gaps, header says two |
| parse_file_tags | 739-761 | REAL | — | collects Tags/Chapters/Tracks across whole segment |
| open_source | 767-772 | REAL | — | real input::FileAccess composition |
| test corpus_tags_match_ffprobe_format_and_stream_scopes | 774-852 | REAL-UNPROVEN | S2 (F-001) | SKIPPED here (corpus absent); logic read & sound (set equality, ≥25 floor) |
| test corpus_editions_match_ffprobe_chapter_counts | 854-920 | REAL-UNPROVEN | S2 (F-001) | SKIPPED here; hardcoded 2580/10 expectations + live ffprobe count check |
| test generated_metadata_files_match_ffprobe | 922-1048 | REAL | — | RAN live: ffmpeg 7.x generated, both differentials matched |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 12 hand-built fixture tests | YES | exact values, typed error kinds + context codes, both-sides depth boundary (tags), budget codes |
| corpus_tags_match... | UNVERIFIABLE HERE | skipped; when corpus present, set-equality asserts + ≥25 floor make it strong |
| corpus_editions... | UNVERIFIABLE HERE | skipped; live ffprobe chapter-count cross-check when present |
| generated_metadata_files... | YES (ran) | ffmpeg exit asserted, per-scope set equality vs live ffprobe, chapter count cross-check; ran live and passed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the real-corpus differential vs ffprobe ... 0017-meta-bomb, 0014, 0003, 0007-chapter-bomb, 0063 and the full mkv sweep" | module header lines 2-6 | REAL-UNPROVEN | live run: "SKIP: corpus-v4 absent" on both corpus tests; corpus dir missing in sandbox |
| "corpus-v4 is absent in this session's sandbox, so ... the 5.D2b tags sweep (mkv_editions_tags.rs) skip loudly" | docs/formats/container-matroska.md:251-254 | REAL (honest record) | sed verified; matches live behavior |
| "mkv_editions_tags 2/0 (live-ffprobe differential, LUMEN_CORPUS=...)" | docs/worklog/archive-2026-10.md:9970 | STALE | file now has 14 tests; worklog records an earlier 2-test state — historical, not a live claim |

## 7. Dead / duplicated / unreachable code
- `corpus_dir()` env default points outside the repo; unreachable asset in
  this environment (see F-001).
- No dead code found in the builders; `open_editions`/`open_tags` share shape
  (acceptable duplication).

## 8. Dependencies & flags
Test-only: demux (mkv zones), testkit (Must), input (FileAccess), plugin_api,
media_types. External tools: ffprobe + ffmpeg (present here). Env:
LUMEN_CORPUS (optional). No feature gates.

## 9. Open questions for the next agent
- Whoever audits crates/demux/src/mkv/tags.rs and editions.rs: confirm the
  depth/budget guards' arithmetic matches the boundary tests (tags ok-side is
  pinned here; editions ok-side is NOT — see F-006).
- mkv_corpus.rs shares the same vacuous-skip pattern (lines 105-106, 229-230)
  — same S2 applies there when that file is assigned.
