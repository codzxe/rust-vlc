# AUDIT: crates/demux/tests/corpus_mp4.rs

| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-13 |
| Lines / functions | 346 / 5 units (2 structs/consts, 1 helper, 2 tests) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | N/A (integration test). Runs under `cargo test -p demux`. In CI it compiles, runs, and **passes vacuously** — verified: no workflow sets `LUMEN_CORPUS` (grep of `.github/workflows/*.yml` + `scripts/` → zero hits), the default dir `/tmp/my-project/corpus-v4/media` is absent in this audit sandbox (`ls` verified), and the skip path `return`s `Ok` on a plain `#[test]` (no `#[ignore]`). The W00 baseline "1751 pass" therefore includes these as passes that compared nothing. |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:1 |
| Commands run | `Read` full file; `sed -n '9,15p;211,227p;326,343p'` (quote verification, exit 0); `test -d /tmp/my-project/corpus-v4` → ABSENT; `rg 'LUMEN_CORPUS' .github/ scripts/` → no hits; `rg 'corpus_mp4\|0074-bframe-storm\|0105-multi-av\|0060-drift-av' docs/worklog/` → aggregate lines only, no per-file table; `sed -n '1,50p' docs/perf/corpus-v4-sweep-20261024.md` (exit 0). No cargo (adaptation). |

## 1. One-paragraph verdict

This is the project's flagship "real corpus MP4/MOV/M4A differential" — 30 pinned lanes with ffmpeg-measured codec identities, per-stream packet counts, and keyframe counts, opened through the real `input::FileAccess` stack (tests/common/mod.rs:546-550, not a memory mock), plus a 5-file seek probe. The test *logic* is strong: exact-equality assertions that would catch any demux miscount. But as shipped it is a test that runs nowhere: CI never sets `LUMEN_CORPUS`, no workflow mounts a corpus (the only corpus-running workflow, `compat-matrix.yml`, is manual-dispatch-only and drives a *different* runner, `tests/compat`), the default corpus path is a sandbox-local `/tmp` directory that does not exist in a fresh clone — and I verified it does not exist in this audit environment either. So both tests take the `eprintln!` + `return` skip path, report "pass", and the module doc's promise that "the per-file PASS/FAIL table for this instance lives in the worklog" points at a table that does not exist (grep of docs/worklog for the lane filenames: zero hits; only aggregate "corpus_mp4 2/2" / "30/30 lanes" prose at archive-2026-10.md:8753/8760). The differential is real work pinned to one dead machine's `/tmp` — SHALLOW: strong code, vacuous everywhere it is counted.

## 2. What actually works (verified)

- When a corpus IS present, the assertions are real and would fail on regression: exact track counts (237-242), exact codec-name equality per track (243-245), exact per-stream packet counts (271-273), exact keyframe counts (274-278), with per-file context strings. Mutation reasoning (UNVERIFIED by run): delete the stsz parser and counts diverge → assert fires.
- Files open through the real access stack: `open_via_input` → `media_types::ids::SourceUri::from_file_path` → `input::FileAccess::new().open(&uri)` (common/mod.rs:546-550) — no in-memory stand-in on the tested path. Verified by reading common/mod.rs.
- The drain loop carries an event budget (`assert!(events < 2_000_000)`, line 256) so a demuxer infinite-stream bug fails rather than hangs.
- The module doc honestly records the one out-of-vocabulary lane (0128 mpeg2video → typed `no-tracks` refusal, lines 19-22 & 162-163) and the cover-art lane's semantics (23-24). The worklog's own scope note ("NOT measured? — ... the decoders themselves", archive-2026-10.md:8760) matches the W00 baseline reality (h264+aac MP4 opens, 0 frames decoded): this suite measures demux counts only and claims nothing about decoding — honest.

## 3. Findings

### C1 [S2] The differential passes vacuously in every reproducible environment (CI and fresh clones)
Where: crates/demux/tests/corpus_mp4.rs:211-227
Code:
```rust
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

#[test]
fn corpus_mp4_family_matches_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(mp4): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
```
What is wrong: the suite's entire value is gated on a directory that exists only on the author's sandbox. Verified in this environment: `/tmp/my-project/corpus-v4` → ABSENT (`test -d`, exit ≠ 0). Verified in CI config: no `.github/workflows/*.yml` and no script under `scripts/` references `LUMEN_CORPUS` or provisions a corpus (grep, zero hits; the `compat-matrix.yml` corpus sweep is manual-dispatch-only by its own header and runs `tests/compat`, not this suite). The skip is a plain `return` on an un-`#[ignore]`d test → reports `ok`.
Why it matters: the module's opening claim — "every ISO-BMFF file in the randomized real corpus ... demuxed through the real access stack and compared against the measured ffmpeg truth" (lines 1-7) — is false in every environment a fresh clone or CI runner can produce. The W00 baseline's "tests 1751 pass / 2 fail / 25 ignored" counts these two as passes while they compared nothing. Anyone reading the green CI log believes a 30-file ffmpeg differential ran. The `eprintln!` mitigation is invisible in test results.
What correct looks like: commit small corpus fixtures into the repo (the crate already does this for `fixtures/adts/`, `fixtures/ps/`) and always run the differential against them; or mark the suite `#[ignore]` so it cannot inflate the pass count.
Confidence: HIGH — Verification: read + `test -d` + repo-wide grep (all exit codes observed).

### C2 [S3] "The corpus regenerates with fresh seeds" contradicts the pinned exact-count truth table
Where: crates/demux/tests/corpus_mp4.rs:9-15 vs 47-207
Code:
```rust
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffprobe
//! 7.1.5 / Lavf61.7.103, the same build that authored the corpus).
//! The corpus regenerates with fresh seeds: when it is absent (and no
```
What is wrong: TRUTHS pins exact packet/keyframe counts (e.g. `("h264", 1412), ("aac", 2029)` for 0060, lines 97-99). If the corpus "regenerates with fresh seeds" (tools/corpus/gen_corpus.py), the regenerated files cannot be guaranteed to reproduce those counts — the suite is pinned to the measurements of one ephemeral instance. Anyone who regenerates the corpus at the default path and runs the suite gets either a skip (names/paths differ) or failures that are not demux bugs.
Why it matters: the differential is unreproducible *by design* — the truth table and the corpus generator are not coupled to a committed seed or committed bytes. The only reproducible corpus data in-repo is the file list (`docs/corpus/corpus-v4-smoke-manifest-20261023.json`), which carries no packet counts.
What correct looks like: commit the corpus (or a deterministic seed + generator pin) with the truths, like the adts/ps fixtures do.
Confidence: MEDIUM — Verification: read; generator behavior UNVERIFIED (no cargo/python run; gen_corpus.py not executed).

### C3 [S3] The seek test never verifies "resumes" and accepts seek-to-start-always
Where: crates/demux/tests/corpus_mp4.rs:326-343
Code:
```rust
        assert!(
            nanos <= 250_000_000,
            "{probe}: landed after target ({nanos} ns)"
        );
        // The walk resumes and finishes.
        let mut seen = 0u64;
        loop {
            match mp4
                .next_event()
                .unwrap_or_else(|e| unreachable!("{probe}: event: {e}"))
            {
                DemuxEvent::EndOfStream => break,
                DemuxEvent::Packet(_) => seen += 1,
                _ => {}
            }
            seen += 1;
```
What is wrong: (a) the landing check is an upper bound only — a demuxer whose seek always rewinds to 0 ns passes every probe; landing accuracy (e.g. within one frame of the target) is unasserted. (b) the comment says "The walk resumes and finishes", but nothing asserts any packet was delivered post-seek: if seek lands at EOS and `next_event()` returns `EndOfStream` immediately, `seen == 0` and the test is green. The "resumes" half of the test's name (`corpus_mp4_seek_lands_and_resumes`) is unverified. (c) `seen` is incremented both in the `Packet` arm (338) and unconditionally after the match (341), double-counting packets — harmless for a pure budget counter, but the counter is the only post-seek assertion's variable, and it checks nothing but the budget.
Why it matters: this is the only seek AC in the mp4 corpus suite; a seek implementation that truncates the stream at the target would pass. Test names promising more than the body checks is exactly §3.4's target.
What correct looks like: assert `seen > 0` (and ideally `>= expected remaining packets` from the same ffprobe truth), and assert a lower bound on the landing (e.g. ≥ target − one frame duration).
Confidence: HIGH — Verification: read (quotes verified `sed -n '326,343p'`).

### C4 [S4] The worklog "per-file PASS/FAIL table" the skip message points to does not exist
Where: crates/demux/tests/corpus_mp4.rs:13-15 (pointer) — evidence absence in docs/worklog/
Code:
```rust
//! `LUMEN_CORPUS` override is set) this suite **skips loudly** — the
//! per-file PASS/FAIL table for this instance lives in the worklog
//! entry, exactly as measured.
```
What is wrong: grep of `docs/worklog/` (README, archive-2026-10.md, w24-*.md) for the 30 lane filenames (`0074-bframe-storm`, `0105-multi-av`, `0060-drift-av` — zero hits; `bframe-storm` matches only the *TS* lane 0091 at archive-2026-10.md:9110/9144). The only corpus_mp4 references are aggregate prose lines (archive-2026-10.md:8753 "corpus_mp4 2/2 (seek + full 30-file differential incl. 0029 ...)", :8760 "corpus differential 30/30 mp4-family lanes"). No per-file table exists anywhere in the repo (the `docs/perf/corpus-v4-sweep-20261024.md` file is a different artifact — a `lumen-sweep` outcome census, not this differential).
Why it matters: the skip's stated mitigation — "the evidence lives in the worklog, exactly as measured" — is a dangling pointer. The entire differential's evidence reduces to two unsourced summary sentences. This is the project's recurring pattern: measured-somewhere-once, verifiable-nowhere.
What correct looks like: commit the per-file table (30 rows: file, tracks, codecs, counts, keyframes) next to the truths it verifies.
Confidence: HIGH — Verification: grep with zero hits (exit codes observed); aggregate lines quoted above.
(Severity note: filed as S4 because the pointer is documentation, but combined with C1 it materially weakens the suite's evidentiary claim.)

## 4. Per-unit table

| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-27 | SHALLOW | — | Claims "every ISO-BMFF file ... compared" — true only on one dead sandbox; honestly records the 0128 gap. |
| `CorpusTruth` (struct) | 38-45 | REAL | — | Plain data carrier. |
| `TRUTHS` (const) | 47-207 | REAL-UNPROVEN | S3 | 30 lanes (23 mp4 + 5 mov + 2 m4a; 24th mp4 excluded honestly); values UNVERIFIED (C2/C4) — no corpus, no ffprobe run by me. |
| `corpus_dir` | 211-217 | REAL | S2 | Env override else `/tmp/...` sandbox default; absent → None (C1). |
| `corpus_mp4_family_matches_the_ffmpeg_truth` | 219-280 | SHALLOW | S2 | Strong assertions (exact counts) but environment-gated to zero coverage in CI (C1); `counts[handle as usize]` unchecked index → panic-not-assert on a rogue handle (acceptable in test); `unreachable!` error formatting panics loudly on any open/start/event error (good). |
| `corpus_mp4_seek_lands_and_resumes` | 284-345 | SHALLOW | S3 | Upper-bound landing only; "resumes" unasserted; `seen` double-counted (C3); same env gating (C1). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| corpus_mp4_family_matches_the_ffmpeg_truth (219) | YES **iff the corpus is present** — exact packet/keyframe/codec assertions. NO in CI/fresh clone (skip → vacuous pass, C1). | Environmental gating. |
| corpus_mp4_seek_lands_and_resumes (284) | PARTIAL even with corpus: a seek-to-0 regression passes (upper-bound only); a stream-truncating seek passes (no resume assertion, C3). | Missing assertions. |
| Both | Attribution to W00 baseline's "1751 pass" UNVERIFIED — I cannot run cargo; the skip mechanism guarantees they *pass* wherever the corpus is absent, which I verified is the case here. | — |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every ISO-BMFF file in the randomized real corpus ... compared against the measured ffmpeg truth" | corpus_mp4.rs:1-7 | SHALLOW | True only against a sandbox-local `/tmp` corpus absent from CI and this environment (C1). |
| "the per-file PASS/FAIL table for this instance lives in the worklog entry, exactly as measured" | corpus_mp4.rs:13-15 | FALSE | No per-file table in docs/worklog/ (grep zero hits on lane names; only aggregate lines archive-2026-10.md:8753/8760). C4. |
| "Every pinned value below was measured this session from the corpus-v4 instance" | corpus_mp4.rs:9-11 | UNVERIFIED | No corpus, no ffprobe run possible; rests on worklog prose. |
| "0128-param-midchange.mp4 ... refuses the file typed (no-tracks) — recorded [PENDING]" | corpus_mp4.rs:19-22 | REAL-UNPROVEN | The typed refusal path exists in the demuxer (mp4 stsd vocabulary, per mod.rs owner's file); not executed by me. |
| (worklog) "corpus differential 30/30 mp4-family lanes vs live ffprobe ... NOT measured? — ... the decoders themselves" | archive-2026-10.md:8760 | REAL-UNPROVEN | The demux-only scoping is honest and matches W00 (h264 MP4 opens, 0 frames decoded); the 30/30 number itself is unrepeatable (C1/C2). |

## 7. Dead / duplicated / unreachable code

- No dead code in this file. `TRUTHS` is fully consumed by test 1; 5 of its 30 lanes are re-used by test 2.
- The suite duplicates the skip-convention of the sibling corpus suites (corpus.rs / corpus_wav.rs per the worklog) — pattern duplication, each with its own copy of the skip logic.

## 8. Dependencies & flags

- `plugin_api` (DemuxEvent, Demuxer), `media_types` (MediaKind, time types), `demux::mp4::Mp4Demuxer`, `common::open_via_input` → `input` crate (real FileAccess), `testkit::Must` (via common). No feature flags. Dev-dependency on the `input` crate makes the "real access stack" claim true at the code level.

## 9. Open questions for the next agent

- Wave 18/19 claim-ledger agents: do any feature-matrix rows cite `corpus_mp4.rs` as evidence? If yes, those rows inherit C1 (a test that proves nothing in CI used as evidence — §4 S0 territory; I did not audit the matrix).
- tools/corpus owner: does `gen_corpus.py --replay` from the committed manifest reproduce byte-identical files (deterministic seeds) or fresh-seed files? C2's severity depends on it. UNVERIFIED.
- Who deleted /tmp corpus instances between sessions? The suite's design assumes a persistent sandbox path across sessions — fragile by construction.
