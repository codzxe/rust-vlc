# AUDIT: crates/playlists/src/probe.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 1234 / 15 prod fns + 14 tests |
| File verdict | REAL (code+tests genuine and executed) — but UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — `probe`/`probe_path`/`walk`/`DirSource` have zero callers outside this file (rg over repo: only self-references + doctest). lumen-cli does not depend on `playlists`; `apps/desktop/src/files.rs:49-55` uses its own hard-coded `PLAYLIST_SUFFIXES` extension list instead. The `playlists` crate ships in the binary only via `streaming` (Cargo.toml:42), which never touches the probe module. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/playlists-c262b72361161a1c probe:: --nocapture` → 14 passed, 2 of which printed "corpus not present — skipping" (exit 0); rg for `ProbedKind|WalkConfig|DirSource` (repo-wide); sed quote verification |

## 1. One-paragraph verdict
The probing/walking code is genuine, pure, bounds-enforced, and well-tested: I executed the prebuilt test binary and 12/14 tests actually exercise classification, depth/entry caps, cycle detection, truncation, and typed errors. However, none of it is wired anywhere: no shipped entry point (lumen CLI or desktop) calls `probe`, `probe_path`, `walk`, or implements `DirSource`; the desktop shell classifies playlists with its own hard-coded extension list. The feature matrix is honest about this (`protocol-directory` = not_started, empty evidence), so this is dead-but-real library code (card 5.O3 seam), not fraud. The two "real corpus" tests are environment-conditional and silently pass as no-ops where the corpus is absent — including this sandbox and any CI without `/tmp/my-project/corpus-v4`.

## 2. What actually works (verified)
— verified by `./target/debug/deps/playlists-c262b72361161a1c probe:: --noc` → `test result: ok. 14 passed; 0 failed` with my own eyes on the skip prints; the 12 synthetic tests assert exact file orders, cap boundaries (`files.len() == 10`), cycle counts, and typed errors, and would fail on mutation of `walk`'s accounting (`seen == config.max_entries`) or the depth gate (`depth + 1 < config.max_depth`).

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/probe.rs:756-766, 837-841
Code: (verbatim, 756-766 abridged to the skip arm)
```
        let media_dir = Path::new("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        let mut names: Vec<_> = match std::fs::read_dir(media_dir) {
            Ok(iter) => iter
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .filter_map(|n| n.into_string().ok())
                .collect(),
            Err(_) => {
                eprintln!("corpus not present — skipping (run on the dev machine)");
                return;
            }
```
What is wrong: `classification_table_over_real_corpus` and `real_hls_playlists_classify_as_m3u` return `Ok` without asserting anything when the private corpus directory is absent. I ran the test binary: both printed "corpus not present — skipping". The baseline log (audit/runs/baseline/cargo-test-workspace.txt:4659,4668) shows them "ok" — indistinguishable from a real pass. The "D-11: no synthetic echo chamber" comment (L752) is only true on one dev machine. Why it matters: the only tests that pin the triage table against real media/HLS files are no-ops everywhere else. What correct looks like: `#[ignore]` + explicit corpus-gated runner, so CI reports "ignored" not "ok".
Confidence: HIGH — Verification: ran

F-002 [S3]
Where: crates/playlists/src/probe.rs:111-120 (lib.rs export of the whole probe surface)
Code: (verbatim, lib.rs:115-118)
```
pub use probe::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES, DirEntryKind, DirSource, ListedEntry,
    PROBE_PREFIX_BYTES, PlaylistFormat, ProbedKind, WalkConfig, WalkError, WalkResult,
    WalkWarnings, probe, probe_path, walk,
};
```
What is wrong: the entire 5.O3 surface is dead code in the shipped build. Repo-wide rg for `ProbedKind|PlaylistFormat|WalkConfig|DirSource|WalkResult|WalkError` matches only this file, its own doc comment, plugin-api/src/kind.rs:14 (an unrelated `PluginKind` variant comment), and streaming's unrelated `RemotePlaylistFormat`. `apps/desktop/src/files.rs:49-55` re-implements playlist detection as a 5-string extension list and never calls `probe`. Why it matters: ~620 LOC of library API compiled into the release binary (via `streaming` → `playlists`) with zero call sites; the "R4 seam" has been open since card 5.O3 with no consumer. What correct looks like: wire the desktop/CLI open path to it or gate it behind a not-yet-active module.
Confidence: HIGH — Verification: read + rg

F-003 [S4]
Where: crates/playlists/src/probe.rs:229-231
Code:
```
        // First non-comment line: a location (URI / relative path).
        return true;
    }
    false
```
What is wrong: `confirm_m3u` confirms ANY `.m3u`/`.m3u8`-extension text file whose first non-comment, non-blank line exists — one line of prose in a file named `notes.m3u` classifies as `Playlist(M3u)`. The module doc (L10-12) discloses this ("the sniff can be no stronger"), and VLC's own loose M3U has the same shape, so this is an honest documented weakness, not a defect against a claim. Blast radius today is zero (F-002: unreachable).
Confidence: HIGH — Verification: read

F-004 [S4]
Where: crates/playlists/src/probe.rs:252-255
Code:
```
fn confirm_xspf(prefix: &[u8]) -> bool {
    let body = strip_bom(prefix);
    contains_sub(body, b"<?xml") && contains_sub(body, b"<playlist")
}
```
What is wrong: unlike `confirm_m3u`/`confirm_pls`/`confirm_cue`, `confirm_xspf` never calls `has_nul` (a binary blob with an `.xspf` name containing both substrings confirms), and it does not require `<?xml` to precede `<playlist` or anchor either to line starts. Same disclosure posture as F-003; unreachable today.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 128-151 | REAL | — | extension+content agreement; media magic first; executed |
| probe_path | 156-158 | REAL | — | thin wrapper over path.extension() |
| is_media_magic | 163-179 | REAL | — | RIFF/OggS/fLaC/EBML/ID3/ftyp@4; bounds-checked; tested incl. short-window case L731-734 |
| strip_bom | 182-188 | REAL | — | UTF-8 BOM only |
| has_nul | 192-194 | REAL | — | text disqualifier |
| next_line | 197-209 | REAL | — | CR+ASCII-trim; no panic on any bytes |
| confirm_m3u | 214-233 | REAL | S4 | F-003 looseness, documented |
| confirm_pls | 236-248 | REAL | — | first non-blank line == [playlist], case-insensitive |
| confirm_xspf | 252-255 | REAL | S4 | F-004 no NUL/order check |
| contains_sub | 257-259 | REAL | — | naive windows scan, O(n·m) on ≤4096 bytes — fine |
| confirm_cue | 265-284 | REAL | — | FILE + boundary + ≥2 quotes; `filename "x"` negative pinned L699 |
| DirSource (trait) | 339-354 | REAL | S3 | F-002: no impl outside tests/doctest |
| WalkConfig::new | 387-395 | REAL | — | zero bounds rejected typed; tested L1156-1167 |
| walk | 497-616 | REAL | — | iterative DFS, sort-for-determinism, caps, cycle set, truncation flush; accounting verified by depth/cap/cycle tests |
| 14 tests | 633-1233 | 12 REAL, 2 vacuous-here | S3 | F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_table_playlist_formats / negatives / media_magic_wins | YES | exact ProbedKind equality on both polarities |
| probe_path_uses_the_path_extension | YES | |
| classification_table_over_real_corpus | NO (here) | corpus absent → skip-return Ok (F-001); on the dev machine YES |
| real_hls_playlists_classify_as_m3u | NO (here) | same skip pattern (F-001) |
| nested_directories_in_deterministic_dfs_order | YES | exact 5-path order + reverse-listing invariance |
| depth_cap_is_enforced | YES | exact file set at depth 4/8/1 |
| symlink_cycle_is_detected_and_bounded | YES | cycles_skipped == 2 and exact files |
| entry_cap_truncates_the_walk | YES | len==budget, truncated==true, prefix order |
| root_unlistable_is_typed_nested_degrades_to_warning | YES | typed error match + warning count |
| config_zero_bounds_are_rejected_typed | YES | matches! on WalkError::InvalidConfig |
| unidentifiable_directory_is_skipped_and_counted | YES | ghost's file absent |
| walked_files_classify_through_probe | YES | combined walk+probe exact expectations |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "5.O3 probing + directory loading; feeds design library sidebar + Open Folder" | docs/feature-matrix.toml:1276 (row protocol-directory, status not_started, evidence "") | REAL-UNPROVEN (code exists, unclaimed) | matrix honestly claims nothing; code verified by test run |
| "no `std::fs` code ships in the library" | probe.rs:32 | REAL | rg: std::fs only inside #[cfg(test)] (L757, 838, 869) |
| "The function is pure — no I/O, no panics on any byte input" | probe.rs:125-126 | REAL | probe/probe_path take slices; no unwrap/expect/index beyond len-checked slices; 12 tests + fuzz crate target |

## 7. Dead / duplicated / unreachable code
The whole module is reachable only from its own tests/doctest (F-002). No duplication found inside the crate; apps/desktop/src/files.rs duplicates the *concept* (extension triage) with a weaker list instead of calling this.

## 8. Dependencies & flags
std + thiserror only (crate Cargo.toml); no features gate this module; compiled unconditionally into the `playlists` lib.

## 9. Open questions for the next agent
- When card 5.O3's consumer lands (library sidebar / Open Folder), check whether the desktop keeps its own PLAYLIST_SUFFIXES or wires `probe` — today both exist.
- Is `/tmp/my-project/corpus-v4` ever restored in CI? If not, F-001 stands permanently.
