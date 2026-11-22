# AUDIT: crates/demux/tests/mkv_extras_wiring.rs

| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-17 |
| Lines / functions | 961 / 9 tests + 17 helpers |
| File verdict | REAL-UNPROVEN (strong, mutation-hostile wiring suite; ran inside the W00 baseline workspace suite — per-test results not isolable under the no-cargo adaptation) |
| Reachable from shipped entry point? | N/A (test binary) — reachable from `cargo test -p demux --test mkv_extras_wiring`; every hand-built vector enters the demuxer through the real `MkvDemuxer::new` + `start()` path (the same code the `lumen` CLI reaches via `containers/mod.rs:57`) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read (full file, 4 chunks); Grep for `fn id_bytes|size_vint|fn el(` across tests (duplication evidence); `ls /usr/share/fonts/truetype/dejavu/` (fonts present), `which ffmpeg ffprobe` (both present, exit 0); `sed -n` verification of all quotes below (exit 0). No cargo (adaptation); W00 baseline (1751 pass / 2 fail / 25 ignored) contains this suite |

## 1. One-paragraph verdict

This is the best test file I audited in this assignment: eight of nine tests build complete hand-crafted MKV files through real EBML byte builders, drive the actual `MkvDemuxer` open/demux/seek paths, and assert dozens of exact values (uids, scale ticks, byte-exact payloads, damage counts, one `Recovered` for a malformed laced block — a pinned livelock regression). A regression to the 5.D1 "stop at first cluster" walk, to parse-and-skip extras, to damage-miscounting, or to a bad seek landing **fails these tests by design**. The one real defect is the ninth test, the ffmpeg "differential lane": when the sandbox lacks a system TTF or ffmpeg it `return`s early with only an `eprintln!("SKIP: …")` — cargo records it as **passed** having asserted nothing (F-001), and it caches its generated fixture in a shared temp dir with an `if !out.exists()` reuse that can poison later runs with stale artifacts (F-002). The environment where W00 ran does have the fonts and ffmpeg (verified below), so the lane almost certainly ran for real there — but nothing in the test forces that, and on a bare CI image this "differential" lane is a vacuous green check.

## 2. What actually works (verified)

- `tags_attachments_chapters_wire_through_start_retained` (`:211-453`) asserts 30+ concrete facts across tags (structure, nesting, TrackUID-77 scoping), title precedence, attachments (count, mime, uid, byte-exact `Inline` payloads, `cover_art` pick), chapters (edition flags, ns times, `beyond_duration == 0`), ns→tick conversion (1500/3900 at 1 ms scale, `:421-448`), a 2-packet drain and `damage_bytes() == 0` (`:451-452`). Breaking any retention seam fails it.
- `damaged_block_recovers_once_and_still_reaches_eos` (`:709-749`) pins a real past defect (the window-leak livelock) with exact counts: 1 packet, **1** `Recovered`, `damage_bytes() > 0`, trailing tags still retained (`:741-748`). This is what a regression test should look like.
- `chapter_targets_seek_lands_at_or_before` (`:608-671`) actually calls `demuxer.seek(...)` and asserts the exact cue-indexed landing (1000 ticks for a 1500-tick chapter, never past, `:658-660`) — it exercises the seek machinery, not just the model.
- The cue-position placeholder dance is self-policing: `assert!(cue_pos < 0x80, "fixture contract: one-byte cue position")` (`:319`) guarantees the two-pass Cues build (placeholder then real position, `:317-323`) cannot silently change byte width.
- `unknown_size_tags_master_parses_and_playback_reaches_eos` (`:676-701`) covers the streaming EBML form end to end (parsed at open AND EOS reached).
- The fixture builders are correct EBML: `size_vint` computes the shortest legal encoding with the marker bit (`:47-57`); `simple_block` uses the spec's 16-bit signed relative timecode (`:105-113`).
- Environment fact (verified by me): this sandbox has `/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Oblique.ttf` (the first candidate `:794`), `/usr/bin/ffmpeg` and `/usr/bin/ffprobe` — so the generated lane's skip conditions do not fire here, and the W00 baseline (which ran in this tree) almost certainly executed it for real. UNVERIFIED: per-test pass output (not captured per-test in the baseline).

## 3. Findings

### F-001 [S3] The ffmpeg differential lane silently passes when its preconditions are absent — a vacuous green check counted as a run
Where: crates/demux/tests/mkv_extras_wiring.rs:792-815
Code:
```rust
fn system_font() -> Option<PathBuf> {
    for candidate in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Oblique.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    eprintln!("SKIP: no system TTF for the font-attachment lane");
    None
}

#[test]
fn generated_lane_wires_title_font_cover_and_chapters() {
    let Some(font_path) = system_font() else {
        return;
    };
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("SKIP: ffmpeg absent (generated wiring lane)");
        return;
    }
```
What is wrong: the test's body is entirely conditional. No system TTF (e.g. a minimal CI image, a Windows runner — the candidate list `:793-797` is Linux-only paths) or no ffmpeg → the function returns before a single assertion, and cargo reports the test as **passed**. The only trace is an `eprintln` on stderr that no test harness parses. This is a runtime-conditional "test that never runs" (§3.4) that still lands in the pass count — the exact shape that lets an evidence ledger claim "the ffmpeg-generated lane passes" on a machine where it never executed.
Why it matters: the file's own module doc sells this as "an **ffmpeg-generated differential lane** … asserted through the wired getters" (`:5-7`); on an environment without those binaries the claim is carried by a no-op. (In the W00 sandbox the binaries exist — verified — so the baseline number is probably honest; the test's *shape* is the defect, not the recorded result.)
What correct looks like: `#[ignore]` + an explicit opt-in env var, or assert the precondition failure loudly (`panic!` when an opt-in env var requested the lane), or generate the font bytes in-test instead of hunting for system fonts.
Confidence: HIGH — Verification: read (control flow is unconditional-early-return; runtime behavior of the skip in other environments UNVERIFIED).

### F-002 [S4] Generated-fixture caching in a shared temp dir with no freshness check
Where: crates/demux/tests/mkv_extras_wiring.rs:816-822, 823-832, 840-871
Code:
```rust
    let dir = std::env::temp_dir().join("lumen-w3i-mkv-wiring");
    std::fs::create_dir_all(&dir).must();
    let cover = dir.join("w3i-cover.jpg");
    let font_copy = dir.join("w3i-font.ttf");
    let meta = dir.join("w3i-chapters.ffmeta");
    let out = dir.join("w3i-wiring.mkv");
```
Code (`:823-824` and `:840-841`):
```rust
    if !cover.exists() {
        let status = Command::new("ffmpeg")
```
```rust
    if !out.exists() {
        let status = Command::new("ffmpeg")
```
What is wrong: the lane reuses any `w3i-wiring.mkv` / `w3i-cover.jpg` already present in the shared temp dir from a previous run — including one generated by older code, or a partial file left by an interrupted run. There is no cleanup and no freshness check; the assertions then run against a stale artifact (a circular-evidence shape: fixtures generated by a previous incarnation of the pipeline).
Why it matters: a future change to the generation command (codecs, metadata flags) will not be reflected if the stale file survives; a truncated leftover file makes the test fail (or worse, pass against a coincidentally-valid prefix) for reasons unrelated to the change under test.
What correct looks like: write into a fresh `tempfile::TempDir` per run (auto-cleaned), or include a content hash/generation stamp in the filename.
Confidence: HIGH — Verification: read.

### F-003 [S4] Fourth copy of the EBML test builders (cross-file)
Where: crates/demux/tests/mkv_extras_wiring.rs:40-65
Code (`:40-44`):
```rust
fn id_bytes(id: u32) -> Vec<u8> {
    let be = id.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(3);
    be[first..].to_vec()
}
```
What is wrong / why it matters / fix: see attachments.rs audit F-005 — this file is one of four divergent copies of `id_bytes`/size-vint/`el` in the mkv surface (header.rs:860-892, attachments.rs:287-317, mkv_editions_tags.rs:49-71, this file).
Confidence: HIGH — Verification: read of all four sites.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `id_bytes` / `size_vint` / `el` / `el_uint` / `el_str` / `cat` | 40-86 | REAL | S4 | Correct builders; **F-003** duplication (4th copy) |
| `ebml_header` | 89-99 | REAL | — | Full valid header incl. MaxIDLength/MaxSizeLength — every test therefore exercises `parse_ebml_header` for real |
| `simple_block` | 105-113 | REAL | — | 16-bit signed rel timecode per spec; `unwrap_or` on the i16 cast is a fixture contract (fine) |
| `cluster` | 116-122 | REAL | — | `u64::try_from(timecode).unwrap_or(0)` swallows a fixture bug silently (test-only, S4 note) |
| `track_entry` | 126-134 | REAL | — | Always writes CodecID `A_OPUS` — the missing-CodecID degenerate path (header.rs F-001) is never built here |
| `cues` | 138-154 | REAL | — | Correct relative-to-segment-payload positions |
| `mkv_file` / `info` / `F64Bytes` | 156-189 | REAL | — | Correct; Duration encoded as 8-byte f64 |
| `drain` | 192-202 | REAL | — | Counts packets to EOS; `.must()` panics on error events (acceptable in tests) |
| test `tags_attachments_chapters_wire_through_start_retained` | 211-453 | REAL | — | 30+ exact assertions; the namesake vector (extras AFTER clusters, Cues BEFORE) — any early-break regression fails |
| test `title_falls_back_to_the_global_title_tag` | 457-486 | REAL | — | Fallback + honest-absence lanes |
| test `track_uids_are_stable_across_reopens` | 490-523 | REAL | — | Reopen stability + out-of-range None |
| test `multi_edition_chapters_retain_all_and_target_the_default` | 528-604 | REAL | — | Hidden edition/atom semantics; flattened=5 vs targets=2 |
| test `chapter_targets_seek_lands_at_or_before` | 608-671 | REAL | — | Real `seek()`; exact landing 1000 ≤ 1500; Zero lands 0 |
| test `unknown_size_tags_master_parses_and_playback_reaches_eos` | 676-701 | REAL | — | Streaming-form master + EOS |
| test `damaged_block_recovers_once_and_still_reaches_eos` | 709-749 | REAL | — | Livelock regression pin; exact counts |
| test `degenerate_attachment_damage_folds_into_the_counter` | 753-784 | REAL | — | Damage row + `damage_bytes() > 0` + survivor retained |
| `system_font` | 792-805 | REAL | S3 | **F-001**; Linux-only candidate paths |
| test `generated_lane_wires_title_font_cover_and_chapters` | 807-961 | REAL (when env allows) | S3/S4 | **F-001** (vacuous skip), **F-002** (stale cache); when it runs: ffprobe title cross-check, byte-exact font/cover, uid joins, 0/250-tick chapters, drain > 0 — strong |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `tags_attachments_chapters_wire_through_start_retained` | YES | 30+ exact assertions over every retained extra + the packet walk + the honesty counter |
| `title_falls_back_to_the_global_title_tag` | YES | Exact `Some("Tag-Scope Title")` / `None` outcomes |
| `track_uids_are_stable_across_reopens` | YES | Exact uid tables across two opens + out-of-range `None` |
| `multi_edition_chapters_retain_all_and_target_the_default` | YES | Edition flags, atom counts, flattened vs targets |
| `chapter_targets_seek_lands_at_or_before` | YES | Exact landing tick (1000) and bounds — a seek regression or off-by-one fails |
| `unknown_size_tags_master_parses_and_playback_reaches_eos` | YES | Retention + 2-packet drain past the streaming master |
| `damaged_block_recovers_once_and_still_reaches_eos` | YES | Exact packets=1 / recoveries=1 / damage>0 — a livelock or double-recovery fails |
| `degenerate_attachment_damage_folds_into_the_counter` | YES | Exact damage reason + counter > 0 + survivor |
| `generated_lane_wires_title_font_cover_and_chapters` | ONLY IF the environment has fonts+ffmpeg | **F-001**: without them it passes having asserted nothing; with them (this sandbox: verified present) it is a strong differential lane |

Coverage gaps (no test here): missing-`CodecID` degenerate track (header.rs F-001 untested repo-wide by this suite), over-ceiling `CodecID`/`CodecPrivate`/`MAX_TRACKS`, overrunning attachment record (attachments.rs F-001), multiple `Attachments` masters (attachments.rs F-002), aggregate attachment memory (F-003), zero `TimestampScale` + `chapter_targets` (the latent panic logged by the mod.rs auditor).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "any regression to the early break or the parse-and-skip arm fails here" | module doc `:11-15` | REAL | Extras placed after clusters + asserted retained (`:326-328`, `:336-410`) |
| "an **ffmpeg-generated differential lane** (real `-metadata title=` …) asserted through the wired getters" | module doc `:5-7` | SHALLOW (environment-conditional) | **F-001**: the lane is a vacuous pass without fonts/ffmpeg; real in this sandbox (binaries verified present) |
| "The unknown-size `Tags` vector pins the streaming shape end to end" | module doc `:15-16` | REAL | Test `:676-701` |
| (implied) the suite pins the mid-cluster damage livelock | test doc `:703-708` | REAL | Test `:709-749` with exact counts |

## 7. Dead / duplicated / unreachable code

- No dead tests. Duplication: the builder set (F-003). `F64Bytes` trait (`:181-189`) exists only to spell `secs.to_f64_bytes()` — a one-use trait where `secs.to_be_bytes().to_vec()` inline would do (S4 note, folded here).

## 8. Dependencies & flags

- Integration test (`tests/`), compiled by `cargo test` only — no feature flags.
- Imports: `demux::avi::test_support::mem_stream` (AVI-zone test support again), `demux::mkv::*`, `common::open_via_input` (`tests/common/mod.rs`), `testkit::Must`, `std::process::Command` (ffmpeg/ffprobe), `std::path::PathBuf`.
- External tools at runtime: `ffmpeg`, `ffprobe`, a system TTF — all present in this sandbox (verified with `which`/`ls`, exit 0).

## 9. Open questions for the next agent

- Did the W00 baseline capture per-test output showing the generated lane actually ran (the `println!("PASS w3i-wiring.mkv: …")` at `:960`)? If not, its "1751 pass" cannot prove this lane executed even once — worth a one-line rerun capture when cargo is available.
- The mod.rs auditor's zero-`TimestampScale` panic in `chapter_targets` (`mod.rs` audit F-002) is reachable from this suite's 4 call sites only after a successful `start()`, which rejects the zero scale — confirmed consistent; no action for this file.
- Wave 16/17: whether CI runners that execute this suite carry ffmpeg/fonts (the release workflow's test job definition decides whether F-001's vacuous pass can fire in CI).
