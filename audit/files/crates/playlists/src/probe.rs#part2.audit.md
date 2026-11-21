# AUDIT: crates/playlists/src/probe.rs#part2 (lines 618-1234)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 618-1234, per wave-00 assignments.tsv LOC split (part1=617 → A09: 1-617 = probe/confirm/classification fns, `DirSource`, `WalkConfig`, `walk`; part2=617 → this audit: the `// Tests` banner + the entire `#[cfg(test)] mod tests`, 622-1234). |
| Lines / functions | 617 / 23 (1 file-read helper + 7 FakeFs impl items + 15 test fns) |
| File verdict | UNREACHABLE (real, tested library API with zero shipped callers; tests themselves executed green in baseline) |
| Reachable from shipped entry point? | NO — `rg -n "playlists::" apps/ tools/` → zero production call sites (apps/desktop/src/files.rs:48 and dispatch.rs:234 mention the crate in comments only). The crate's only consumer is `crates/streaming` (Cargo.toml:42), whose sole use is `remote_playlist.rs:178` `playlists::parse_bytes` — and `open_remote_playlist` itself has no production caller (test-only, rg). Nothing in `lumen` CLI or `lumen-tauri` reaches `probe`/`probe_path`/`walk`/`DirSource`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "playlists::" apps/ tools/ --glob '*.rs'` (no production callers) · `rg -n "probe::(walk\|probe\|DirSource)"` (only probe.rs's own doctest, line 467) · `rg -n "remote_playlist\|open_remote_playlist"` (test-only callers) · `ls /tmp/my-project/corpus-v4/` → **No such file or directory** · baseline `orchrun-test-workspace-nofailfast.log.txt:4259-4307` (all 15 probe::tests ok, incl. the two silently-skipped corpus tests), `:6484` (probe::walk doctest ok) · `sed -n` quote verification |

## 1. One-paragraph verdict
This is a well-built test module for a library nobody ships: 13 of the 15 tests are genuinely discriminating (exact DFS file orders, exact depth-cap file lists, cycle counts, budget-exact truncation, byte-exact classification tables), they all ran green in the baseline no-fail-fast run, and I hand-re-derived the walk algorithm's ordering against `nested_directories_in_deterministic_dfs_order` and `depth_cap_is_enforced` and found the expectations arithmetically correct. The two "real corpus" tests, however, are the classic pass-while-testing-nothing pattern: they key off a hard-coded `/tmp/my-project/corpus-v4/…` path that does not exist in the baseline sandbox, print a skip note, and return `ok` — so the D-11 "no synthetic echo chamber" claim is exercised by exactly zero bytes in the recorded evidence, and a comment claiming the path is "env-overridable" is false (no env var is read). The entire `probe`/`walk` API is unreachable from both shipped entry points.

## 2. What actually works (verified)
— All 15 `probe::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:4293-4307` and the `probe::walk` doctest green at `:6484` (release profile).
— Hand-traced `walk`'s stack discipline against the tests' exact expectations: reverse-push/smallest-pop yields pre-order DFS in byte-wise name order (`/r/a/k/x.txt` before `/r/a/m.txt` before `/r/b/z1.txt`, test 970-979); depth semantics "files at exactly max_depth collected, directories at max_depth not descended" (`depth+1 < config.max_depth`, walk:573) match the f4-in/f5-out and f8-in/f9-out expectations (1043-1072); entry-cap accounting counts listed children in sorted order so the truncated prefix is the sorted prefix (1122-1125).
— Cycle guard verified against the FakeFs link table: both `/r/loop` and `/r/sub/back` canonicalize to the root's visited identity → `cycles_skipped == 2` with the subtree intact (1100-1107).
— `ls /tmp/my-project/corpus-v4/` → does not exist: proves the two corpus tests took their skip path in the baseline run (their `ok` at 4293/4307 is a skip, not a pass over data).

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/probe.rs:754-767 and 836-841 (baseline evidence: orchrun-test-workspace-nofailfast.log.txt:4293,4307)
Code:
```
        let media_dir = Path::new("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        let mut names: Vec<_> = match std::fs::read_dir(media_dir) {
            Ok(iter) => iter
```
What is wrong: the two "real corpus" tests (`classification_table_over_real_corpus`, `real_hls_playlists_classify_as_m3u`) silently skip — `eprintln!(...)` then `return` — when the hard-coded `/tmp/my-project/corpus-v4/…` directory is absent. It is absent: I verified `ls /tmp/my-project/corpus-v4/` → No such file or directory. Both tests report `ok` in the baseline run, so the recorded evidence contains zero executions of the corpus triage table and zero real-HLS classifications. The doc claim these tests carry ("D-11: no synthetic echo chamber", comment at 752-753) is untested wherever the corpus is missing — which includes every machine but the dev box.
Why it matters: a test that returns `ok` on a missing fixture can never catch a triage regression on real-world files; the gate is invisible (no `#[ignore]`, no failure, stdout swallowed unless `--nocapture`). This is the "cfg-gated tests that never run" pattern implemented via filesystem-presence gating.
What correct looks like: gate the corpus behind an explicit `#[ignore = "corpus"]` + a `LM_CORPUS_DIR` env override, or fail (not skip) when a `LM_REQUIRE_CORPUS=1` marker is set for the dev-machine gate — anything that makes absence visible instead of green.
Confidence: HIGH — Verification: ran `ls /tmp/my-project/corpus-v4/` (absent) + read baseline log lines 4293/4307 (`ok`) — the skip path is the only possible execution in the baseline environment.

### F-002 [S4]
Where: crates/playlists/src/probe.rs:756 and 837 (comment text)
Code:
```
        let media_dir = Path::new("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
```
What is wrong: the comment asserts the path is "env-overridable"; no environment variable is read anywhere in the file (`rg -n "env::var" crates/playlists/src/probe.rs` → nothing). The path is a compile-time constant.
Why it matters: a maintainer following the comment would look for a knob that does not exist; the comment is the only documentation of how to point the tests at a corpus.
What correct looks like: actually read `std::env::var("LM_CORPUS_DIR")` with this literal as the default, or delete the "env-overridable" claim.
Confidence: HIGH — Verification: read + rg (zero env reads in the file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `read_prefix` | 868-874 | REAL | — | PROBE_PREFIX_BYTES-bounded read; `unreachable!` on IO error acceptable in tests. |
| `FakeFs` (struct + `dir`/`link`/`fail`/`fail_canon`/`resolve`) | 883-925 | REAL | — | in-memory fs with scripted failures; `reverse` flag drives the determinism test. |
| `FakeFs::list` / `FakeFs::canonical` (DirSource impl) | 927-946 | REAL | — | models symlink resolution via the link table; unknown dir = NotFound (so an unregistered root fails typed — pinned by 1147-1151). |
| `probe_table_playlist_formats` | 633-674 | REAL | — | M3U/M3U8 case-tolerance, plain-M3U, PLS, XSPF, CUE incl. BOM + lowercase `file` keyword. |
| `probe_table_playlist_negatives` | 676-707 | REAL | — | extension+content disagreement, `filename "x"` boundary, `.txt` extension gate. |
| `probe_table_media_magic_wins` | 709-736 | REAL | — | 7 magic families × {right-ext, wrong-ext, no-ext}; short ftyp window = Unknown. |
| `probe_path_uses_the_path_extension` | 738-749 | REAL | — | `.M3U8` path case tolerance. |
| `classification_table_over_real_corpus` | 754-831 | SHALLOW (in baseline) | S3 (F-001) | skip-path taken; `Box::leak` at 814-816 (test-only, harmless); `unreachable!` on un-tabled corpus extension (805) is brittle-but-visible. |
| `real_hls_playlists_classify_as_m3u` | 836-866 | SHALLOW (in baseline) | S3 (F-001) | skip-path taken. |
| `nested_directories_in_deterministic_dfs_order` | 948-1010 | REAL | — | exact 5-path order + reverse-listing invariance. |
| `depth_cap_is_enforced` | 1012-1078 | REAL | — | depth 4/8/1 exact lists; hand-verified against `walk`'s `depth+1 < max_depth` rule; `Box::leak` loop labels (1027-1033) test-only. |
| `symlink_cycle_is_detected_and_bounded` | 1080-1109 | REAL | — | `cycles_skipped == 2`, subtree files exact. |
| `entry_cap_truncates_the_walk` | 1111-1129 | REAL | — | exactly 10 files f00..f09, `truncated`, default caps pinned. |
| `root_unlistable_is_typed_nested_degrades_to_warning` | 1131-1153 | REAL | — | RootUnlistable vs nested warning; Display contains "root directory". |
| `config_zero_bounds_are_rejected_typed` | 1155-1167 | REAL | — | InvalidConfig on zero depth/entries. |
| `unidentifiable_directory_is_skipped_and_counted` | 1173-1196 | REAL | — | ghost skipped+counted; empty dir no warning. |
| `walked_files_classify_through_probe` | 1199-1233 | REAL | — | walk order + probe classification integration (jpg Unknown, m3u Playlist, flac/mkv Media). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| probe_table_playlist_formats / negatives / media_magic_wins | YES | exact `ProbedKind` equality on both sides of every boundary (content-vs-extension precedence, BOM, case, short windows). |
| probe_path_uses_the_path_extension | YES | extension extraction is the only variable. |
| classification_table_over_real_corpus | NO (in baseline) | corpus absent → skip path returns `ok` unconditionally; on a corpus machine, YES (exact per-file expected table + family-coverage asserts 819-824 + counts 829-830). |
| real_hls_playlists_classify_as_m3u | NO (in baseline) | same skip path; on a corpus machine, YES (`paths.len() >= 5` + per-file M3U equality). |
| nested_directories_in_deterministic_dfs_order | YES | exact 5-element path-vector equality; any ordering/emit bug moves an element. |
| depth_cap_is_enforced | YES | exact file lists at three depths; off-by-one in the depth rule adds/removes f4/f5/f8/f9. |
| symlink_cycle_is_detected_and_bounded | YES | cycle count + exact file set; a broken guard either loops (timeout) or misses a count. |
| entry_cap_truncates_the_walk | YES | `files.len() == 10` exactly + first/last element + `truncated`; an off-by-one in `seen` accounting trips it. |
| root_unlistable_is_typed_nested_degrades_to_warning | YES | error variant + warning count both pinned. |
| config_zero_bounds_are_rejected_typed | YES | variant matches on both zero shapes. |
| unidentifiable_directory_is_skipped_and_counted | YES | ghost's file must be absent + counter exact. |
| walked_files_classify_through_probe | YES | index-exact file order + classification per file. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Real corpus … (D-11: no synthetic echo chamber)" | probe.rs:751-753 test doc | SHALLOW | corpus path absent in baseline sandbox (`ls`), tests skipped green — F-001. |
| "env-overridable" corpus path | probe.rs:756, 837 comments | FALSE | no `env::var` read in the file — F-002. |
| "Runs when the corpus is present; prints a skip note otherwise (CI portability)" | probe.rs:753-754 | REAL | the skip mechanism itself works as described (baseline shows `ok` with corpus absent) — the honesty problem is that `ok` is indistinguishable from a pass. |
| (part1's doc claims — probe/walk contracts) | probe.rs:1-70, 443-458 | audited by A09 | out of my chunk; my tests corroborate the depth/cycle/order/budget contracts. |

## 7. Dead / duplicated / unreachable code
The whole audited surface is production-dead from the shipped binaries: `probe`, `probe_path`, `walk`, `DirSource`, `WalkConfig`, `WalkError`, `WalkResult` have zero callers in `apps/` or `tools/` (rg). The crate's single production consumer (`crates/streaming` → `remote_playlist.rs`) uses `parse_bytes`/`resolve_uri`, not `probe`. The tests are the only live users. `FakeFs.reverse` is exercised (989-1008); no dead test helpers found.

## 8. Dependencies & flags
Test module only: `std` (fs/io/collections), `super::*`. No feature gates, no unsafe, no new deps. Crate built and tests run in baseline (4259: playlists lib target, 15 tests + suites; doc-test 6484).

## 9. Open questions for the next agent
- A09 (part1): the `is_media_magic` table and `confirm_*` heuristics are only exercised by the synthetic table + (absent) corpus — consider whether the recorded transport/raw-ES seam (`.ts`, `.aac`, raw H.264 → Unknown) is documented in the feature matrix as such.
- Whoever audits `crates/streaming/src/remote_playlist.rs` (A01): it is the playlists crate's only production consumer and itself has no shipped caller — the entire playlist lane (parse + probe + walk) may be dead from the binary's perspective; worth a cross-wave note.
